//! Offline-deterministic bake workers for fw-content-baker.
//!
//! T2-3 ships only `BakeNamesOffline` — the first end-to-end bake path.
//! Real Claude API call wiring is deferred until budget approval +
//! `ANTHROPIC_API_KEY` threading (T2-4+). `model_id` in the manifest is
//! the literal string `"offline-v1"` for this path.
//!
//! **Determinism contract:** `BakeNamesOffline::run(culture, count, seed)` is
//! pure: same inputs → same RON bytes → same BLAKE3 → same manifest. No
//! wall-clock reads, no system RNG, no HashMap iteration.

use std::path::{Path, PathBuf};

use blake3::Hasher;
use fw_content::Culture;
use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// BakeManifest — audit-trail sidecar
// ---------------------------------------------------------------------------

/// Manifest sidecar emitted alongside each baked artifact.
///
/// Committed alongside the RON so reviewers can verify which model + seed
/// produced the output, and re-bakes can confirm byte-identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BakeManifest {
    /// `"offline-v1"` for the deterministic sampling path; a Claude model ID
    /// string when the real API path lands at T2-4.
    pub model_id: String,
    /// BLAKE3 hex of the prompt template that drove this bake.
    pub prompt_hash: String,
    /// Seed passed to `ChaCha8Rng::seed_from_u64`. Pinned in the manifest so
    /// a re-bake with the same seed reproduces the same output.
    pub seed: u64,
    /// Filename of the emitted RON file (NOT an absolute path). The manifest
    /// always sits in the same directory as the RON it describes, so a
    /// relative reference is unambiguous AND keeps the manifest JSON
    /// byte-identical across different output directories / machines.
    /// Post-T2-3 code-reviewer P1 fix.
    pub output_path: String,
    /// BLAKE3 hex of the RON file bytes.
    pub output_blake3: String,
    /// Number of name entries in the output.
    pub count: usize,
}

// ---------------------------------------------------------------------------
// BakeNamesOffline — deterministic sampler
// ---------------------------------------------------------------------------

/// Offline name baker — composes full-name strings from
/// `Culture.first_name_bank` × `Culture.last_name_bank` via a seeded
/// `ChaCha8Rng`.
///
/// Determinism contract: same `(culture, count, seed)` triple →
/// same RON bytes → same `output_blake3`. Verified by the integration test
/// `bake_names_is_deterministic_same_seed_same_bytes`.
pub struct BakeNamesOffline<'a> {
    /// The culture to sample names from.
    pub culture: &'a Culture,
    /// How many full-name strings to generate.
    pub count: usize,
    /// Seed for `ChaCha8Rng`. Stamped into the manifest for audit.
    pub seed: u64,
}

impl<'a> BakeNamesOffline<'a> {
    /// Run the bake; write `names_<slug>.ron` + `names_<slug>.manifest.json`
    /// to `output_dir`. Returns `(ron_path, manifest_path)`.
    ///
    /// Both files are written atomically (write-then-rename is not attempted;
    /// this is a dev-only tool). Any I/O failure is surfaced as `std::io::Error`.
    pub fn run(&self, output_dir: &Path) -> std::io::Result<(PathBuf, PathBuf)> {
        // 1. Generate `count` full-name strings via seeded RNG.
        let mut rng = ChaCha8Rng::seed_from_u64(self.seed);
        let mut names: Vec<String> = Vec::with_capacity(self.count);

        // Guard empty banks: propagate an I/O error rather than panicking, so
        // the caller gets an actionable message. Empty banks are a content
        // error caught by CultureValidator before bake-names is ever called in
        // practice; this guard is belt-and-braces.
        if self.culture.first_name_bank.is_empty() || self.culture.last_name_bank.is_empty() {
            return Err(std::io::Error::other(format!(
                "culture {:?} has empty name bank; run validate-structural first",
                self.culture.id
            )));
        }

        // T2-3 scope: ONLY `{first} {last}` substitution is supported. Cultures
        // declaring a multi-component pattern (e.g. `{first} {patronymic} {last}`)
        // FAIL LOUD here rather than silently degrading to the simple form +
        // emitting a manifest that hashes the degraded output as if it were
        // correct. Multi-component pattern support lands at T2-4 alongside the
        // first patronymic-bearing culture.
        //
        // Post-T2-3 silent-failure-hunter P0 fix: prior code silently rewrote
        // `{first} {patronymic} {last}` to `{first} {last}` with no warning,
        // no error, and no manifest field recording the substitution — exactly
        // the silent-degradation pattern banned by the project rules.
        if self.culture.naming_pattern.contains("{patronymic}") {
            return Err(std::io::Error::other(format!(
                "culture {:?}: naming_pattern {:?} contains {{patronymic}}; \
                 multi-component pattern support is deferred to T2-4. \
                 Either simplify the pattern to '{{first}} {{last}}' or wait for T2-4.",
                self.culture.id, self.culture.naming_pattern
            )));
        }

        for _ in 0..self.count {
            let first_idx = rng.gen_range(0..self.culture.first_name_bank.len());
            let last_idx = rng.gen_range(0..self.culture.last_name_bank.len());

            let full = self
                .culture
                .naming_pattern
                .replace("{first}", &self.culture.first_name_bank[first_idx])
                .replace("{last}", &self.culture.last_name_bank[last_idx]);
            names.push(full);
        }

        // 2. Serialize to RON.
        // Use PrettyConfig with explicit LF newlines so output is byte-identical
        // across platforms (Windows would otherwise inject CR+LF).
        let ron_text =
            ron::ser::to_string_pretty(&names, ron::ser::PrettyConfig::new().new_line("\n".into()))
                .expect("RON serialization of Vec<String> is infallible");

        // 3. Slug + paths.
        let slug = slug_for_culture_id(&self.culture.id);
        let ron_path = output_dir.join(format!("names_{slug}.ron"));
        let manifest_path = output_dir.join(format!("names_{slug}.manifest.json"));

        // 4. Refuse to silently clobber an existing artifact.
        //
        // Post-T2-3 silent-failure-hunter P1 fix: `slug_for_culture_id` takes
        // the segment after the last `.`, which collides across pack-id
        // namespaces (`fwh.core:culture.anglo` and
        // `mod.community.somerset:culture.anglo` both produce slug `"anglo"`).
        // Per `Content/RULES.md §6`, mod overlays in their own pack-id are
        // first-class. Without this guard, the second bake into the same
        // `--output` directory would silently overwrite the first — including
        // the manifest sidecar that's supposed to be the audit trail.
        //
        // Failing loudly forces the user to pick distinct `--output` dirs OR
        // delete the prior artifact, which is the right UX for a dev tool.
        // A future "merge" mode that intentionally combines pack-id namespaces
        // would live behind a `--force-overwrite` flag, not silent default.
        if ron_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "output RON {ron_path:?} already exists for culture {:?}; \
                     refusing to overwrite. Delete the prior file or pick a \
                     different --output directory.",
                    self.culture.id
                ),
            ));
        }
        if manifest_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::AlreadyExists,
                format!(
                    "output manifest {manifest_path:?} already exists for culture {:?}; \
                     refusing to overwrite.",
                    self.culture.id
                ),
            ));
        }

        // 5. Write RON.
        std::fs::write(&ron_path, ron_text.as_bytes())?;

        // 6. Hash output bytes + build manifest.
        let output_blake3 = {
            let mut hasher = Hasher::new();
            hasher.update(ron_text.as_bytes());
            hasher.finalize().to_hex().to_string()
        };

        let prompt_hash = blake3::hash(OFFLINE_PROMPT_TEMPLATE.as_bytes())
            .to_hex()
            .to_string();

        // Manifest stores the RON FILENAME, not the absolute path. Storing
        // the absolute path makes the manifest JSON non-deterministic across
        // machines/tempdirs (post-T2-3 code-reviewer P1 fix) — same
        // `(culture, count, seed)` triple would produce different manifest
        // bytes on different systems, undermining the "audit + reproducibility"
        // contract documented on the `BakeManifest` type. The filename is
        // sufficient: the manifest sidecar lives next to the RON in the same
        // directory, so the relative reference is unambiguous.
        let output_filename = ron_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("names_{slug}.ron"));

        let manifest = BakeManifest {
            model_id: "offline-v1".to_string(),
            prompt_hash,
            seed: self.seed,
            output_path: output_filename,
            output_blake3,
            count: self.count,
        };

        let manifest_json = serde_json::to_string_pretty(&manifest)
            .expect("BakeManifest JSON serialization is infallible");
        std::fs::write(&manifest_path, manifest_json.as_bytes())?;

        Ok((ron_path, manifest_path))
    }
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Convert a content-pack-qualified culture ID into a filesystem-safe slug.
///
/// `"fwh.core:culture.anglo"` → `"anglo"`
/// `"fwh.core:culture.fantasy-elvish"` → `"fantasy-elvish"`
///
/// Takes the segment after the last `.` in the whole ID string. This is
/// unambiguous because culture IDs always end with `.{slug}` (per the ID
/// format spec in `Content/RULES.md §2`).
fn slug_for_culture_id(id: &str) -> String {
    id.rsplit('.').next().unwrap_or(id).to_string()
}

/// The "prompt" for the OFFLINE bake path — included in the manifest's
/// `prompt_hash` so the offline path has a stable audit identifier even though
/// no LLM is called. When the real `--api` path lands at T2-4, this constant
/// is replaced by the loaded `src/prompts/names.md` content.
const OFFLINE_PROMPT_TEMPLATE: &str = "OFFLINE bake: sample N distinct full-name strings from \
     culture.first_name_bank × culture.last_name_bank via seeded ChaCha8Rng \
     + culture.naming_pattern substitution. model_id=offline-v1.";

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_content::{Culture, CultureWeights};
    use tempfile::TempDir;

    fn minimal_culture() -> Culture {
        Culture {
            id: "fwh.core:culture.test".to_string(),
            name: "Test".to_string(),
            first_name_bank: vec!["Alice".into(), "Bob".into(), "Carol".into()],
            last_name_bank: vec!["Smith".into(), "Jones".into()],
            team_name_bank: vec![],
            naming_pattern: "{first} {last}".to_string(),
            weights: CultureWeights::default(),
        }
    }

    #[test]
    fn slug_for_culture_id_extracts_final_segment() {
        assert_eq!(slug_for_culture_id("fwh.core:culture.anglo"), "anglo");
        assert_eq!(
            slug_for_culture_id("fwh.core:culture.fantasy-elvish"),
            "fantasy-elvish"
        );
        // Fallback: no dot → return the whole string unchanged.
        assert_eq!(slug_for_culture_id("nodot"), "nodot");
    }

    #[test]
    fn bake_names_offline_writes_ron_and_manifest() {
        let tmp = TempDir::new().expect("tempdir");
        let culture = minimal_culture();
        let baker = BakeNamesOffline {
            culture: &culture,
            count: 5,
            seed: 42,
        };
        let (ron_path, manifest_path) = baker.run(tmp.path()).expect("bake must succeed");
        assert!(ron_path.exists(), "RON file must be written");
        assert!(manifest_path.exists(), "manifest file must be written");

        let names: Vec<String> =
            ron::de::from_str(&std::fs::read_to_string(&ron_path).unwrap()).unwrap();
        assert_eq!(names.len(), 5);

        let manifest: BakeManifest =
            serde_json::from_str(&std::fs::read_to_string(&manifest_path).unwrap()).unwrap();
        assert_eq!(manifest.count, 5);
        assert_eq!(manifest.model_id, "offline-v1");
        assert_eq!(manifest.seed, 42);
    }

    #[test]
    fn bake_names_offline_is_deterministic() {
        let culture = minimal_culture();
        let tmp1 = TempDir::new().expect("tempdir 1");
        let tmp2 = TempDir::new().expect("tempdir 2");

        let baker1 = BakeNamesOffline {
            culture: &culture,
            count: 10,
            seed: 0xdeadbeef,
        };
        let baker2 = BakeNamesOffline {
            culture: &culture,
            count: 10,
            seed: 0xdeadbeef,
        };

        let (ron1, _) = baker1.run(tmp1.path()).expect("bake 1");
        let (ron2, _) = baker2.run(tmp2.path()).expect("bake 2");

        let bytes1 = std::fs::read(&ron1).unwrap();
        let bytes2 = std::fs::read(&ron2).unwrap();
        assert_eq!(bytes1, bytes2, "same seed must produce byte-identical RON");
    }

    #[test]
    fn bake_names_offline_empty_first_bank_returns_error() {
        let mut culture = minimal_culture();
        culture.first_name_bank.clear();
        let tmp = TempDir::new().expect("tempdir");
        let baker = BakeNamesOffline {
            culture: &culture,
            count: 1,
            seed: 0,
        };
        let result = baker.run(tmp.path());
        assert!(result.is_err(), "empty first_name_bank must return Err");
    }
}

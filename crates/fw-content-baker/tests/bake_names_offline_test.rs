//! Integration tests for `BakeNamesOffline` — AC3, AC4, AC5 of T2-3.
//!
//! These tests exercise the real `content/sources/cultures/` fixtures via
//! `ContentStore::load_sources`, so they're integration tests rather than
//! unit tests.

use std::path::PathBuf;

use fw_content::ContentStore;
use fw_content_baker::bake::{BakeManifest, BakeNamesOffline};
use tempfile::TempDir;

/// Return the workspace `content/` directory, located relative to
/// `CARGO_MANIFEST_DIR` (`crates/fw-content-baker`).
fn content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

// ---------------------------------------------------------------------------
// AC3: bake-names writes N entries + manifest to a tempdir
// ---------------------------------------------------------------------------

#[test]
fn bake_names_writes_100_entries_and_manifest_to_tempdir() {
    let content_root = content_root();
    let store =
        ContentStore::load_sources(&content_root).expect("content/sources must load successfully");

    // Use the first culture in the store (sorted BTreeMap, deterministic).
    let (culture_id, culture) = store
        .cultures
        .iter()
        .next()
        .expect("at least one culture must be present in content/sources/");

    let tmp = TempDir::new().expect("tempdir");
    let baker = BakeNamesOffline {
        culture,
        count: 100,
        seed: 0xfeed_beef_cafe_fade,
    };

    let (ron_path, manifest_path) = baker.run(tmp.path()).expect("bake-names must succeed");

    // RON file assertions.
    assert!(ron_path.exists(), "RON file must exist at {ron_path:?}");
    let ron_text = std::fs::read_to_string(&ron_path).expect("RON must be readable");
    let names: Vec<String> = ron::de::from_str(&ron_text).expect("RON must parse as Vec<String>");
    assert_eq!(
        names.len(),
        100,
        "RON must contain exactly 100 name entries"
    );

    // Every entry must be non-empty.
    for name in &names {
        assert!(!name.is_empty(), "name entry must be non-empty: {name:?}");
    }

    // Manifest assertions.
    assert!(
        manifest_path.exists(),
        "manifest JSON must exist at {manifest_path:?}"
    );
    let manifest_text = std::fs::read_to_string(&manifest_path).expect("manifest must be readable");
    let manifest: BakeManifest =
        serde_json::from_str(&manifest_text).expect("manifest must parse as BakeManifest");

    assert_eq!(manifest.count, 100);
    assert_eq!(manifest.model_id, "offline-v1");
    assert_eq!(manifest.seed, 0xfeed_beef_cafe_fade);
    assert!(
        !manifest.output_blake3.is_empty(),
        "output_blake3 must be set"
    );
    assert!(!manifest.prompt_hash.is_empty(), "prompt_hash must be set");

    // Post-T2-3 code-reviewer P1 fix: manifest.output_path stores the RON
    // FILENAME (not the absolute path) so the manifest JSON is byte-identical
    // across machines / tempdirs. The filename starts with `names_` and ends
    // with `.ron`; nothing else should appear in it.
    assert!(
        manifest.output_path.starts_with("names_"),
        "output_path must be a filename starting with 'names_': {}",
        manifest.output_path
    );
    assert!(
        manifest.output_path.ends_with(".ron"),
        "output_path must be a filename ending in '.ron': {}",
        manifest.output_path
    );
    assert!(
        !manifest.output_path.contains('/') && !manifest.output_path.contains('\\'),
        "output_path must be a bare filename (no path separators): {}",
        manifest.output_path
    );

    // Sanity: culture_id appears in the ron filename slug.
    let slug = culture_id.rsplit('.').next().unwrap_or(culture_id.as_str());
    assert!(
        ron_path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains(slug),
        "RON filename must contain culture slug {slug:?}; got {:?}",
        ron_path.file_name()
    );
}

// ---------------------------------------------------------------------------
// AC4: same seed → byte-identical RON + same output_blake3
// ---------------------------------------------------------------------------

#[test]
fn bake_names_is_deterministic_same_seed_same_bytes() {
    let content_root = content_root();
    let store = ContentStore::load_sources(&content_root).expect("content/sources must load");

    let (_, culture) = store.cultures.iter().next().expect("at least one culture");

    let seed = 0x1234_5678_9abc_def0_u64;

    let tmp1 = TempDir::new().expect("tempdir 1");
    let tmp2 = TempDir::new().expect("tempdir 2");

    let (ron1, manifest1) = BakeNamesOffline {
        culture,
        count: 50,
        seed,
    }
    .run(tmp1.path())
    .expect("bake 1 must succeed");

    let (ron2, manifest2) = BakeNamesOffline {
        culture,
        count: 50,
        seed,
    }
    .run(tmp2.path())
    .expect("bake 2 must succeed");

    let bytes1 = std::fs::read(&ron1).expect("read bake 1 RON");
    let bytes2 = std::fs::read(&ron2).expect("read bake 2 RON");
    assert_eq!(
        bytes1, bytes2,
        "same seed must produce byte-identical RON output (determinism contract)"
    );

    let m1: BakeManifest =
        serde_json::from_str(&std::fs::read_to_string(manifest1).unwrap()).unwrap();
    let m2: BakeManifest =
        serde_json::from_str(&std::fs::read_to_string(manifest2).unwrap()).unwrap();

    assert_eq!(
        m1.output_blake3, m2.output_blake3,
        "output_blake3 must be identical across runs with the same seed"
    );
    assert_eq!(
        m1.prompt_hash, m2.prompt_hash,
        "prompt_hash must be stable across runs"
    );

    // Post-T2-3 code-reviewer P1 fix: the WHOLE manifest JSON must be
    // byte-identical across runs (not just the hashes). Previously
    // `manifest.output_path` stored an absolute tempdir path → manifest JSON
    // diverged across runs even when RON bytes matched. Now `output_path` is
    // the bare filename, so the manifest JSON is genuinely reproducible.
    let manifest_bytes_1 =
        std::fs::read(ron1.with_extension("manifest.json")).expect("read bake 1 manifest bytes");
    let manifest_bytes_2 =
        std::fs::read(ron2.with_extension("manifest.json")).expect("read bake 2 manifest bytes");
    assert_eq!(
        manifest_bytes_1, manifest_bytes_2,
        "same seed must produce byte-identical manifest JSON (reproducibility contract)"
    );
}

// ---------------------------------------------------------------------------
// AC4 (sanity): different seeds → different RON
// ---------------------------------------------------------------------------

#[test]
fn bake_names_differs_across_seeds() {
    let content_root = content_root();
    let store = ContentStore::load_sources(&content_root).expect("content/sources must load");

    let (_, culture) = store.cultures.iter().next().expect("at least one culture");

    let tmp1 = TempDir::new().expect("tempdir 1");
    let tmp2 = TempDir::new().expect("tempdir 2");

    let (ron1, _) = BakeNamesOffline {
        culture,
        count: 50,
        seed: 0,
    }
    .run(tmp1.path())
    .expect("bake seed=0");

    let (ron2, _) = BakeNamesOffline {
        culture,
        count: 50,
        seed: u64::MAX,
    }
    .run(tmp2.path())
    .expect("bake seed=MAX");

    let bytes1 = std::fs::read(&ron1).unwrap();
    let bytes2 = std::fs::read(&ron2).unwrap();

    // The probability of a collision with different seeds over 50 samples is
    // negligible; this test would only fail via a bug in the RNG wiring.
    assert_ne!(
        bytes1, bytes2,
        "different seeds must (almost certainly) produce different RON output"
    );
}

// ---------------------------------------------------------------------------
// AC5: validate-structural surface preserved — existing cultures pass
// ---------------------------------------------------------------------------

#[test]
fn committed_cultures_pass_culture_validator() {
    use fw_content_baker::validators::CultureValidator;

    let content_root = content_root();
    let store = ContentStore::load_sources(&content_root).expect("content/sources must load");
    let validator = CultureValidator::new();

    for (id, culture) in &store.cultures {
        validator
            .validate(culture)
            .unwrap_or_else(|e| panic!("culture {id:?} must pass structural validation: {e}"));
    }
}

#[test]
fn committed_archetypes_pass_tactical_archetype_validator() {
    use fw_content_baker::validators::TacticalArchetypeValidator;

    let content_root = content_root();
    let store = ContentStore::load_sources(&content_root).expect("content/sources must load");
    let validator = TacticalArchetypeValidator::new();

    for (id, archetype) in &store.tactical_archetypes {
        validator
            .validate(archetype)
            .unwrap_or_else(|e| panic!("archetype {id:?} must pass structural validation: {e}"));
    }
}

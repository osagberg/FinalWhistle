//! fw-content-baker — bake-time LLM content compiler (dev-only).
//!
//! This binary is **never linked into the shipping runtime.** It exists to
//! compile procedural-fantasy content (names, biographies, scout phrases,
//! news headlines, manager quotes, fan reactions, commentary lines) from
//! LLM output into deterministically-sampleable RON corpus files under
//! `content/baked/`.
//!
//! The committed RON is the source of truth. Regeneration produces a delta
//! pack with a bumped corpus_version. Runtime (fw-content::runtime) loads the
//! corpus once at startup and samples deterministically via ChaCha8Rng.
//!
//! See `docs/CONTENT_PIPELINE.md` for the contract this implements.

use std::path::PathBuf;

use clap::{Parser, Subcommand};

mod prompts;
mod schemas;

// Re-use the library surface (exposed via lib.rs) so the bin and integration
// tests share the same module tree.
use fw_content_baker::bake::BakeNamesOffline;
use fw_content_baker::validators::{
    CultureValidator, PlayerTemplateValidator, RoleAffinityTableValidator,
    TacticalArchetypeValidator,
};

#[derive(Parser, Debug)]
#[command(
    name = "fw-content-baker",
    about = "Bake LLM-generated procedural content for Final Whistle.",
    long_about = "Dev-only CLI. Compiles Claude API output into RON corpus files \
                  under content/baked/. Output is reviewed + committed; runtime never \
                  re-bakes. See docs/CONTENT_PIPELINE.md."
)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Path to the workspace root (where content/ lives).
    #[arg(long, default_value = ".")]
    workspace: String,

    /// Dry-run: print the prompts + estimated tokens; do NOT call the API.
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Seed for any baker-side RNG (e.g. shuffling prompt variants). Pinned
    /// to the manifest so reruns are reproducible.
    #[arg(long, default_value_t = 0xfeed_beef_cafe_fade_u64)]
    seed: u64,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Bake per-culture player-name banks (first + last + naming-pattern grammar).
    ///
    /// T2-3: offline-deterministic path — samples from the culture's existing
    /// `first_name_bank` × `last_name_bank` via seeded `ChaCha8Rng`.
    /// Real Claude API call wiring lands at T2-4.
    BakeNames {
        /// Culture ID to bake (content-pack-qualified, e.g.
        /// `fwh.core:culture.anglo`). Required — must match a culture loaded
        /// from `content/sources/cultures/`. Post-T2-3 silent-failure-hunter
        /// P1 fix: `required = true` so clap surfaces the missing-arg error
        /// at parse time rather than inside the handler after env_logger init.
        #[arg(long, required = true)]
        culture: String,

        /// Number of full-name entries to generate.
        #[arg(long, default_value_t = 50)]
        count_per_bank: usize,

        /// Output directory. Receives `names_<slug>.ron` +
        /// `names_<slug>.manifest.json`. Defaults to `content/baked/`.
        #[arg(long, default_value = "content/baked")]
        output: String,
    },

    /// Bake player biographies per culture × role archetype.
    BakeBios {
        #[arg(long)]
        culture: Option<String>,
        #[arg(long)]
        archetype: Option<String>,
        /// Templates per (culture, archetype) cell. Cell-cardinality target ~200.
        #[arg(long, default_value_t = 200)]
        templates_per_cell: usize,
    },

    /// Bake news-headline Tracery grammars per event class.
    BakeHeadlines {
        /// Event class to bake (breakthrough-goal, sacking, derby-result,
        /// upset, contract-drama). Omit for all.
        #[arg(long)]
        event_class: Option<String>,
    },

    /// Bake scout-report phrase templates with positive/neutral/negative variants.
    BakeScoutPhrases {
        /// Scout archetype (physical_profiler, technical_purist, regional_expert).
        #[arg(long)]
        archetype: Option<String>,
    },

    /// Bake manager-quote Tracery grammars keyed by archetype + outcome class.
    BakeManagerQuotes {
        #[arg(long)]
        archetype: Option<String>,
    },

    /// Bake fan-reaction Tracery grammars per fan-base mood + recent result.
    BakeFanReactions,

    /// Bake match-commentary phrase banks per event type.
    BakeCommentary {
        /// Event type: goal | save | miss | foul | card | sub | kick-off | full-time.
        #[arg(long)]
        event_type: Option<String>,
        /// Templates per event type (target: 50 per type, ~140 total MVP).
        #[arg(long, default_value_t = 50)]
        templates_per_type: usize,
    },

    /// Bake everything in dependency order (cultures → names → bios → phrases
    /// → headlines → manager-quotes → fan-reactions → commentary). The big red
    /// button. Honors `--dry-run`. Output goes to a delta pack with a bumped
    /// corpus_version.
    BakeAll,

    /// Inspect the current baked manifest (corpus_version, per-file
    /// model_id / prompt_hash / seed audit trail).
    Manifest,

    /// Run the **structural** validators over loaded content/sources/**
    /// (role-affinity weight sums, player-template attribute Q32 ranges,
    /// ability-ceiling bounds, culture name-bank minimums,
    /// tactical-archetype formation correctness).
    ///
    /// **NOT** a full content-pack validation — `banned_terms`,
    /// `licensed_data`, and `cliche` validators return
    /// `ValidationError::NotImplemented` per T1-12 hardening. The future
    /// `validate-semantic` + `validate-content-pack` subcommands ship at T2-4
    /// alongside the real bake pipeline (rolled forward from T2 per Codex
    /// Tier-3 verdict; see `docs/MASTER_PLAN.md` T2-4).
    ///
    /// **STRUCTURAL ONLY — does NOT prove the content pack is safe to ship.**
    /// Composed-name output (e.g. `first_name × last_name` concatenation) is
    /// NOT sampled or linted. A `Culture` whose banks deterministically
    /// concatenate into a banned place-name (Codex Track E-2 "Manchester"
    /// exploit) PASSES this subcommand. Treat green output here as a
    /// necessary but NOT sufficient gate before publishing.
    ///
    /// T1-20 (post-T1-close ultimate-review Track E #1): renamed from
    /// `validate` so the CLI surface stops promising "all validators passed"
    /// when only structural validators actually run.
    ///
    /// T2-3: now delegates to the four dedicated `*Validator` structs
    /// (`RoleAffinityTableValidator`, `PlayerTemplateValidator`,
    /// `CultureValidator`, `TacticalArchetypeValidator`) rather than
    /// inline rule lists.
    ValidateStructural,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    log::info!("fw-content-baker");
    log::info!("workspace: {}", cli.workspace);
    log::info!("dry_run:   {}", cli.dry_run);
    log::info!("seed:      0x{:016x}", cli.seed);

    match cli.command {
        Command::BakeNames {
            culture,
            count_per_bank,
            output,
        } => {
            let workspace = cli.workspace.clone();
            let seed = cli.seed;
            run_bake_names(&workspace, seed, culture, count_per_bank, &output)
        }

        Command::BakeBios {
            culture,
            archetype,
            templates_per_cell,
        } => {
            log::info!(
                "bake-bios: culture={:?} archetype={:?} templates_per_cell={}",
                culture,
                archetype,
                templates_per_cell
            );
            stub_unimplemented("bake-bios", "T2-4")
        }
        Command::BakeHeadlines { event_class } => {
            log::info!("bake-headlines: event_class={:?}", event_class);
            stub_unimplemented("bake-headlines", "T3-3")
        }
        Command::BakeScoutPhrases { archetype } => {
            log::info!("bake-scout-phrases: archetype={:?}", archetype);
            stub_unimplemented("bake-scout-phrases", "T3-5")
        }
        Command::BakeManagerQuotes { archetype } => {
            log::info!("bake-manager-quotes: archetype={:?}", archetype);
            stub_unimplemented("bake-manager-quotes", "T3-3")
        }
        Command::BakeFanReactions => {
            log::info!("bake-fan-reactions");
            stub_unimplemented("bake-fan-reactions", "T3+ (Stretch)")
        }
        Command::BakeCommentary {
            event_type,
            templates_per_type,
        } => {
            log::info!(
                "bake-commentary: event_type={:?} templates_per_type={}",
                event_type,
                templates_per_type
            );
            stub_unimplemented("bake-commentary", "T3-3")
        }
        Command::BakeAll => {
            log::info!("bake-all");
            stub_unimplemented("bake-all", "T2-4 (full pipeline orchestration)")
        }
        Command::Manifest => {
            log::info!("manifest");
            // Post-T2-3 code-reviewer P1 fix: milestone string was "T2-3" but
            // `Manifest` (a read-and-print-the-baked-manifest command) was
            // never in T2-3 scope — `BakeNamesOffline` writes manifests in
            // this row but a separate read-side surface lands at T2-4 when
            // the bake-time pipeline can produce more than one bake artifact.
            stub_unimplemented("manifest", "T2-4")
        }
        Command::ValidateStructural => {
            log::info!("validate-structural");
            run_validate_structural(&cli.workspace)
        }
    }
}

// ---------------------------------------------------------------------------
// bake-names subcommand
// ---------------------------------------------------------------------------

fn run_bake_names(
    workspace: &str,
    seed: u64,
    culture_id: String,
    count_per_bank: usize,
    output: &str,
) -> anyhow::Result<()> {
    use fw_content::ContentStore;

    // Post-T2-3 silent-failure-hunter P1 fix: `--culture` is now
    // `#[arg(long, required = true)]` so clap rejects missing flags at parse
    // time. The handler no longer needs the `Option::ok_or_else` shim.

    let content_root = PathBuf::from(workspace).join("content");
    let store = ContentStore::load_sources(&content_root)
        .map_err(|e| anyhow::anyhow!("content load failed: {e}"))?;

    let cult = store.cultures.get(&culture_id).ok_or_else(|| {
        let available: Vec<&str> = store.cultures.keys().map(String::as_str).collect();
        anyhow::anyhow!(
            "culture {culture_id:?} not found in content/sources/cultures/; \
             available: {available:?}"
        )
    })?;

    let output_dir = PathBuf::from(output);
    std::fs::create_dir_all(&output_dir)
        .map_err(|e| anyhow::anyhow!("could not create output dir {output_dir:?}: {e}"))?;

    let baker = BakeNamesOffline {
        culture: cult,
        count: count_per_bank,
        seed,
    };
    let (ron_path, manifest_path) = baker
        .run(&output_dir)
        .map_err(|e| anyhow::anyhow!("bake failed: {e}"))?;

    println!(
        "bake-names: wrote {} entries to {} (manifest: {})",
        count_per_bank,
        ron_path.display(),
        manifest_path.display(),
    );
    Ok(())
}

// ---------------------------------------------------------------------------
// validate-structural subcommand
// ---------------------------------------------------------------------------

/// `validate-structural` subcommand entry point.
///
/// T2-3 refactor: delegates all validation to the four dedicated `*Validator`
/// structs in `fw_content_baker::validators`. Prior implementation had inline
/// rule lists; the Validator structs are the single audit surface from now on.
///
/// What's implemented today (real, fail-closed):
/// - Load every committed content fixture under `content/sources/*` via
///   `fw_content::ContentStore::load_sources`. The loader itself enforces
///   manager → tactical_archetype and player → signature_definition
///   cross-refs (T1-7 + T1-20).
/// - `RoleAffinityTableValidator`: weight sums + unknown attribute keys.
/// - `PlayerTemplateValidator`: attribute unit-range + ability-ceiling.
/// - `CultureValidator`: first/last name bank minimums.
/// - `TacticalArchetypeValidator`: formation size + buildup-speed range +
///   roster-slot permutation.
///
/// What's NOT in scope (deferred to T3+ `validate-semantic` +
/// `validate-content-pack`):
/// - `check_banned_terms` / `check_licensed_data` / `check_cliche` (still
///   return `ValidationError::NotImplemented` per T1-12 honesty contract).
/// - Content-pack manifest cohesion, mod overlay ordering.
/// - **Composed-output sampling.** Structural bank-size checks pass for a
///   `Culture` whose `first_name_bank` × `last_name_bank` deterministically
///   composes into a banned place-name (Codex Track E-2 "Manchester" exploit:
///   20× "Man" + 20× "chester" + `naming_pattern: "{first}{last}"`). The
///   semantic validator that samples generated names + lints them against
///   `scripts/lint-banned-terms.py` lands at T2-4 alongside the real
///   `BakeNames` consumer (rolled forward from T2 close per Codex Tier-3
///   verdict; see `docs/MASTER_PLAN.md` T2-4 row).
fn run_validate_structural(workspace: &str) -> anyhow::Result<()> {
    use fw_content::ContentStore;

    let content_root: PathBuf = PathBuf::from(workspace).join("content");
    println!(
        "fw-content-baker: running STRUCTURAL validation at {}",
        content_root.display()
    );

    let store = ContentStore::load_sources(&content_root)
        .map_err(|e| anyhow::anyhow!("content load failed: {e}"))?;

    // T2-R-C4 (post-T2 ultimate-review Track C-4): fail-loud on an
    // empty corpus. `ContentStore::load_sources` silently skips missing
    // directories, so a freshly-cloned repo (or a malformed pack with
    // no archetypes) would otherwise iterate zero entities through
    // each validator + print "STRUCTURAL validation passed" — a false-
    // positive that the operator only discovers far away at
    // `AppState::new`'s `generate_league` panic.
    //
    // Today the production pack ships ≥1 in every required category;
    // this guard catches "operator deleted content/sources/cultures/"
    // and similar corruption at the validation site, where the
    // diagnostic still names the missing category.
    let mut empty_categories: Vec<&'static str> = Vec::new();
    if store.cultures.is_empty() {
        empty_categories.push("cultures");
    }
    if store.tactical_archetypes.is_empty() {
        empty_categories.push("tactical_archetypes");
    }
    if store.player_templates.is_empty() {
        empty_categories.push("player_templates");
    }
    if store.managers.is_empty() {
        empty_categories.push("managers");
    }
    if store.role_affinity_tables.is_empty() {
        empty_categories.push("role_affinity_tables");
    }
    if !empty_categories.is_empty() {
        anyhow::bail!(
            "content corpus is empty in one or more required categories: {:?}. \
             content/sources/{{cultures,archetypes,players,managers,role-affinities}}/ \
             must each contain >=1 entity. Did content/sources/ get partially deleted \
             or was the workspace cloned without LFS-fetched RON files?",
            empty_categories
        );
    }

    let mut errors = Vec::<String>::new();

    // Role-affinity validation.
    let role_validator = RoleAffinityTableValidator::new();
    for (id, table) in &store.role_affinity_tables {
        if let Err(e) = role_validator.validate(table) {
            errors.push(format!("role-affinity {id:?}: {e}"));
        }
    }

    // Player-template validation.
    let player_validator = PlayerTemplateValidator::new();
    for (qid, template) in &store.player_templates {
        if let Err(e) = player_validator.validate(template) {
            errors.push(format!("player {qid:?}: {e}"));
        }
    }

    // Culture validation.
    let culture_validator = CultureValidator::new();
    for (id, culture) in &store.cultures {
        if let Err(e) = culture_validator.validate(culture) {
            errors.push(format!("culture {id:?}: {e}"));
        }
    }

    // Tactical-archetype validation.
    let archetype_validator = TacticalArchetypeValidator::new();
    for (id, archetype) in &store.tactical_archetypes {
        if let Err(e) = archetype_validator.validate(archetype) {
            errors.push(format!("archetype {id:?}: {e}"));
        }
    }

    println!(
        "fw-content-baker: structurally validated {} cultures, {} archetypes, \
         {} role-affinity tables, {} player templates, {} signatures, {} managers",
        store.cultures.len(),
        store.tactical_archetypes.len(),
        store.role_affinity_tables.len(),
        store.player_templates.len(),
        store.signature_definitions.len(),
        store.managers.len(),
    );

    if !errors.is_empty() {
        for e in &errors {
            eprintln!("validation error: {e}");
        }
        anyhow::bail!("{} validation error(s); see stderr above", errors.len());
    }

    println!(
        "fw-content-baker: STRUCTURAL validation passed. \
         NOTE: structural != semantic. Bank-size + range checks succeeded \
         but composed-name output (e.g. first×last concatenation) was NOT \
         linted for banned terms / licensed-data collisions. Semantic \
         validator lands at T2-4 alongside the real bake pipeline. \
         Do NOT publish a content pack on the basis of this exit code alone."
    );
    Ok(())
}

fn stub_unimplemented(cmd: &str, milestone: &str) -> anyhow::Result<()> {
    println!(
        "fw-content-baker: `{}` not implemented yet (lands at MASTER_PLAN {}). \
         See docs/CONTENT_PIPELINE.md §6 for the milestone table.",
        cmd, milestone
    );
    Ok(())
}

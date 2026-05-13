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
//!
//! Real implementation lands at MASTER_PLAN T2-3 + T3-3. This stub establishes
//! the CLI surface so downstream wiring (Justfile, CI, `scripts/fw bake`) can
//! be authored before the LLM-calling guts exist.

use clap::{Parser, Subcommand};

mod prompts;
mod schemas;
mod validators;

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
    BakeNames {
        /// Culture archetype to bake (e.g. anglo, germanic, fantasy-elvish).
        /// Omit to bake all cultures defined in content/sources/cultures/.
        #[arg(long)]
        culture: Option<String>,
        /// Number of first-name + last-name entries per culture.
        #[arg(long, default_value_t = 50)]
        count_per_bank: usize,
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

    /// Run the validators (banned-terms lint + JSON-schema check + cliché
    /// detector) over the existing content/baked/** without re-baking.
    /// Useful pre-commit.
    Validate,
}

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cli = Cli::parse();

    log::info!("fw-content-baker stub — implementation lands at MASTER_PLAN T2-3");
    log::info!("workspace: {}", cli.workspace);
    log::info!("dry_run:   {}", cli.dry_run);
    log::info!("seed:      0x{:016x}", cli.seed);

    match cli.command {
        Command::BakeNames {
            culture,
            count_per_bank,
        } => {
            log::info!(
                "bake-names: culture={:?} count_per_bank={}",
                culture,
                count_per_bank
            );
            stub_unimplemented("bake-names", "T2-3")
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
            stub_unimplemented("bake-all", "T2-3 (names) → T3-3 (rest)")
        }
        Command::Manifest => {
            log::info!("manifest");
            stub_unimplemented("manifest", "T2-3")
        }
        Command::Validate => {
            log::info!("validate");
            stub_unimplemented("validate", "T2-3")
        }
    }
}

fn stub_unimplemented(cmd: &str, milestone: &str) -> anyhow::Result<()> {
    println!(
        "fw-content-baker: `{}` not implemented yet (lands at MASTER_PLAN {}). \
         See docs/CONTENT_PIPELINE.md §6 for the milestone table.",
        cmd, milestone
    );
    Ok(())
}

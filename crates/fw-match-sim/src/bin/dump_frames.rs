//! `dump_frames` — deterministic match-frame JSON writer.
//!
//! Produces the same `Vec<MatchFrameDto>` JSON shape that the Tauri
//! `match_frames` IPC command returns, but writes it to stdout for the
//! browser-dev mode (T1-2a per ADR-0008). The frontend's
//! `HttpFrameSource` fetches the JSON fixture from a URL or local path
//! and renders it on the dev-tier 2D tactical board WITHOUT requiring
//! a running Tauri shell — which is what unlocks Claude-Preview-driven
//! visual verification.
//!
//! ## Usage
//!
//! ```sh
//! # Minimal (no content — empty sig_definitions):
//! cargo run --bin dump_frames -- --seed 0xdeadbeef --ticks 60 > /tmp/smoke.json
//!
//! # With content (real signatures wired into slot 7 AM):
//! cargo run --bin dump_frames -- --seed 0xdeadbeef --ticks 600 \
//!     --content content > /tmp/smoke-content.json
//! ```
//!
//! Output: pretty-printed JSON array of `MatchFrameDto`. Length
//! `tick_count + 1` (index 0 is the initial state; index `tick_count`
//! is the state after `tick_count` advances). Matches the
//! `fw_tauri::commands::match_frames` shape verbatim — same struct,
//! same camelCase serde.
//!
//! ## Determinism
//!
//! Running the binary twice with the same `--seed` + `--ticks` + `--content`
//! produces byte-identical stdout. The path through
//! `MatchState::initial[_with_content]` + `tick_match` is fully deterministic
//! (per the canonical-state contract in `docs/specs/determinism-gate.md`); the
//! `serde_json::to_string_pretty` projection is also deterministic given the
//! camelCase ordering pinned in `MatchFrameDto`.
//!
//! ## Not a content-pack tool
//!
//! This binary is a dev / CI tool that emits ephemeral fixtures for
//! the browser-dev workflow. It is NEVER linked into the shipping
//! game. Distinct from `fw-content-baker` (which compiles LLM output
//! into committed RON corpus files).

use std::collections::BTreeMap;
use std::path::PathBuf;

use clap::Parser;
use fw_content::ContentStore;
use fw_core::Seed;
use fw_match_sim::{MatchFrameDto, MatchState, SignatureDefinition, tick_match};

#[derive(Parser, Debug)]
#[command(
    name = "dump_frames",
    about = "Dump a deterministic Vec<MatchFrameDto> JSON sequence for the dev-tier 2D tactical board (T1-2a browser-dev path)."
)]
struct Cli {
    /// Match seed (hex). `0x`-prefixed or bare.
    #[arg(long, default_value = "0xdeadbeefdeadbeef")]
    seed: String,

    /// Number of ticks to advance. Output array length is `ticks + 1`
    /// (index 0 is the initial state).
    #[arg(long, default_value_t = 60)]
    ticks: u32,

    /// Compact JSON output (no indentation). Useful if the consumer is
    /// machine-only. Default is pretty-printed for human-readable
    /// fixtures.
    #[arg(long, default_value_t = false)]
    compact: bool,

    /// Path to the content root directory (e.g. `content`). When provided,
    /// loads `ContentStore::load_sources(&path)` and uses
    /// `MatchState::initial_with_content` so that slot-7 AM signature
    /// candidates are wired in and real `SignatureDefinition` objects are
    /// passed to `tick_match`. Without this flag, `MatchState::initial`
    /// is used and `sig_definitions` is empty (no signatures fire).
    #[arg(long)]
    content: Option<PathBuf>,
}

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("dump_frames: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    let trimmed = cli.seed.trim_start_matches("0x");
    let raw = u64::from_str_radix(trimmed, 16)
        .map_err(|e| format!("invalid --seed {:?}: {e}", cli.seed))?;
    let seed = Seed::from_u64(raw);

    // Load content store if --content was given; otherwise use empty maps.
    let (initial_state, sig_definitions): (MatchState, BTreeMap<String, SignatureDefinition>) =
        if let Some(content_path) = &cli.content {
            let store = ContentStore::load_sources(content_path)
                .map_err(|e| format!("ContentStore::load_sources({content_path:?}): {e}"))?;
            // T2-1a: default both teams to DEFAULT_ARCHETYPE_ID; per-team
            // variation can be added as future dump_frames CLI flags.
            let state = MatchState::initial_with_content(
                seed,
                &store,
                fw_match_sim::DEFAULT_ARCHETYPE_ID,
                fw_match_sim::DEFAULT_ARCHETYPE_ID,
            )
            .map_err(|e| format!("initial_with_content: {e}"))?;
            let sigs = store.signature_definitions.clone();
            (state, sigs)
        } else {
            (MatchState::initial(seed), BTreeMap::new())
        };

    let mut state = initial_state;
    let total = (cli.ticks as usize).saturating_add(1);
    let mut frames: Vec<MatchFrameDto> = Vec::with_capacity(total);
    frames.push(MatchFrameDto::from_state(&state));
    for _ in 0..cli.ticks {
        state = tick_match(state, &sig_definitions);
        frames.push(MatchFrameDto::from_state(&state));
    }

    let json = if cli.compact {
        serde_json::to_string(&frames).map_err(|e| format!("JSON encode: {e}"))?
    } else {
        serde_json::to_string_pretty(&frames).map_err(|e| format!("JSON encode: {e}"))?
    };
    println!("{json}");
    Ok(())
}

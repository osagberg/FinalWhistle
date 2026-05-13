//! Post-LLM validation lints.
//!
//! Every baker subcommand pipes generated fragments through these validators
//! before writing RON. A Category-A hit rejects the fragment outright. A
//! Category-B / cliché hit is logged and surfaces in the bake review report.

// Validators authored ahead of the bake subcommands that pipe through
// them. T2-3 wires bake-names through `validate_fragment`; the inner
// `check_*` helpers land their consumers as each baker subcommand is
// implemented. Same staging strategy as prompts.rs / schemas.rs.
#![allow(dead_code)]
//!
//! Three layers:
//!   1. `banned_terms` — shells out to `scripts/lint-banned-terms.py` with
//!      `--scope <generated-tmpfile>`. The Python is the single source of
//!      truth; we do not reimplement the catalog in Rust.
//!   2. `licensed_data` — regex match against a curated list of real-world
//!      Premier League / La Liga / Bundesliga / Serie A clubs + canonical
//!      player surnames. Reject on any hit (Category A — no exemption).
//!   3. `cliche_detector` — reject bio sentences that match common LLM tells.
//!      Override per-fragment with a sentinel comment.
//!
//! Stub at T0; real implementation lands at MASTER_PLAN T2-3 alongside the
//! first `bake-names` call.

use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("banned-term violation (Category A): {0}")]
    BannedTerm(String),
    #[error("licensed-data hit: {0}")]
    LicensedData(String),
    #[error("cliché detected: {0}")]
    Cliche(String),
    #[error("validator I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Run the banned-terms lint against a generated content fragment.
///
/// Stub: real implementation spawns
/// `scripts/lint-banned-terms.py <tmpfile>` and parses exit code + stderr.
pub fn check_banned_terms(_fragment_path: &Path) -> Result<(), ValidationError> {
    // TODO(T2-3): wire to scripts/lint-banned-terms.py via std::process::Command.
    Ok(())
}

/// Reject any text containing a real-world licensed club / surname.
///
/// Curated list lives in `data/licensed-blocklist.txt` (gitignored from
/// shipped build; lives alongside this crate for dev-only use). Stub.
pub fn check_licensed_data(_text: &str) -> Result<(), ValidationError> {
    // TODO(T2-3): load blocklist; case-insensitive whole-word match.
    Ok(())
}

/// Cliché detector — reject obvious LLM tells.
///
/// Default patterns (override per-fragment via sentinel):
/// - "passionate about"
/// - "exceptional ability to"
/// - "rising star with bright future"
/// - "the world of football"
/// - "wears his heart on his sleeve" (acceptable as football vernacular but
///   over-used by LLMs; soft-rejected and devs decide)
///
/// Devs override by appending `// bake-baker:allow-cliche reason="..."` to
/// the bake log entry for that fragment.
pub fn check_cliche(_text: &str) -> Result<(), ValidationError> {
    // TODO(T2-3): regex catalog + fuzzy match against bio templates.
    Ok(())
}

/// Run all three validators in order. Returns the first error encountered.
pub fn validate_fragment(text: &str, fragment_path: &Path) -> Result<(), ValidationError> {
    check_banned_terms(fragment_path)?;
    check_licensed_data(text)?;
    check_cliche(text)?;
    Ok(())
}

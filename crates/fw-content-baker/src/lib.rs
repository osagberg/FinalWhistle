//! fw-content-baker library surface.
//!
//! Exposes `bake` and `validators` as `pub mod` so integration tests under
//! `crates/fw-content-baker/tests/` can import them. The binary target
//! (`src/main.rs`) also imports these modules.
//!
//! `prompts` and `schemas` are **not** re-exported here — they are staging
//! modules left untouched per the T2-3 scope constraint.

pub mod bake;
pub mod validators;

/// Return `Err` for a subcommand that is not yet implemented.
///
/// BK-E-3 fix: stubs must exit non-zero so CI and operators cannot mistake
/// "not implemented" for "ran successfully". Every unimplemented subcommand
/// in `main.rs` calls this function; the process exits with a non-zero status
/// because `main()` propagates the `Err` via `anyhow::Result`.
pub fn stub_unimplemented(cmd: &str, milestone: &str) -> anyhow::Result<()> {
    anyhow::bail!(
        "bake subcommand `{}` is not yet implemented (deferred to MASTER_PLAN {}). \
         See docs/CONTENT_PIPELINE.md §6 for the milestone table.",
        cmd,
        milestone
    )
}

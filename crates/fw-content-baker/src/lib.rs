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

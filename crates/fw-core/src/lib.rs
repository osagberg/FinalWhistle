//! `fw-core` — determinism primitives.
//!
//! Zero gameplay logic lives here. Only the type vocabulary every other
//! Final Whistle crate consumes: the Q32.32 fixed-point numeric type, the
//! `Seed` and `Tick` newtypes, and the entity-ID types.
//!
//! This crate is the *only* crate every other crate may depend on
//! unconditionally; downstream-only-on-it makes circular-dependency
//! impossible by construction. See `docs/architecture.md` for the full
//! crate graph.
//!
//! ## Determinism contract
//!
//! - No floats. Q32.32 is the numeric primitive; `f32`/`f64` are not
//!   surfaced from this crate at all. The `clippy::float_arithmetic = deny`
//!   lint at crate root enforces this for the crate's own code.
//! - No `HashMap` / `HashSet` in canonical state. Downstream crates use
//!   `BTreeMap` / `BTreeSet` for sorted, reproducible iteration order.
//! - No clocks. `Tick` is a monotonic in-sim counter — never `Instant::now()`
//!   or `SystemTime::now()`. Wall-clock time is a renderer concern.
//!
//! See `docs/specs/determinism-gate.md` for the full contract.

pub mod ids;
pub mod q32;
pub mod seed;
pub mod tick;

// -------------------------------------------------------------------------
// Public re-exports — the canonical surface every other crate imports.
// -------------------------------------------------------------------------

pub use ids::{ClubId, MatchId, PlayerId};
pub use q32::{Q32, Q32Inner};
pub use seed::Seed;
pub use tick::Tick;

// -------------------------------------------------------------------------
// Smoke
// -------------------------------------------------------------------------

#[cfg(test)]
mod smoke {
    #[test]
    fn smoke() {
        assert_eq!(2 + 2, 4);
    }
}

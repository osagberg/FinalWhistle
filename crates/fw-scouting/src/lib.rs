//! `fw-scouting` — scout uncertainty model (Path B MVP, T3-5).
//!
//! Implements the `design/scouting.md` type contract + Path-B report generator.
//! A scout is a biased observer with a model; the player never sees the canonical
//! `GeneSnapshot` — truth emerges from scout reports over seasons (DESIGN_DOC §3 Pillar 4).
//!
//! ## Module layout
//! - `band` — `UncertaintyBand` + threshold constants + `GeneCategoryEstimate::band`
//! - `report` — `GeneCategory`, `GeneCategoryEstimate`, `LabelEstimate`, `ScoutReport`
//! - `scout` — `ScoutArchetypeKind`, `CategoryBiases`, `Scout` + tuning constants
//! - `observe` — `observe_player` Path-B generator
//!
//! ## Determinism contract
//! `float_arithmetic = "deny"` (clippy). All numerics are `Q32`. `BTreeMap`/`BTreeSet`/`Vec` only.
//! No `HashMap`, no clocks, no `thread_rng`. One `ChaCha8Rng` per `observe_player` call,
//! seeded via `seed_fn(.., SeedLayer::ScoutObservation, 0)` per ADR-0009.

pub mod band;
pub mod observe;
pub mod report;
pub mod scout;

// ---------------------------------------------------------------------------
// Public surface re-exports
// ---------------------------------------------------------------------------

pub use band::UncertaintyBand;
pub use observe::observe_player;
pub use report::{
    GeneCategory, GeneCategoryEstimate, GeneCategoryEstimateError, LabelEstimate, ScoutReport,
};
pub use scout::{
    BASIC_SCOUT_BAND_HALF_WIDTH, BASIC_SCOUT_OBSERVATION_NOISE, CategoryBiases,
    CategoryBiasesError, LABEL_CONFIDENCE_MAX, LABEL_CONFIDENCE_MIN, NO_LABEL_DEFAULT_CONFIDENCE,
    Scout, ScoutArchetypeKind,
};

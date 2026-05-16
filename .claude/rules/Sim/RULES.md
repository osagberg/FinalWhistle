---
description: Determinism non-negotiables for sim crates. Reject-on-sight rules — Q32.32 only, BTreeMap only, no f32/f64, no async, no clocks, no HashMap.
applies_to:
  - crates/fw-core/**
  - crates/fw-match-sim/**
  - crates/fw-memory/**
  - crates/fw-replay/**
  - crates/fw-save/**
  - crates/fw-content/**
  - crates/fw-scouting/**
auto_load: when_editing_matching_path
---

# Sim determinism rules

These rules are **reject-on-sight** for `lead-programmer` review. Clippy + the canonical-hash regression test enforce most of them, but humans are the last line. Quote this file by section number when flagging a violation.

## §1. No floats in canonical state

- **BANNED:** `f32` / `f64` in `MatchEvent`, `MemoryEvent`, any canonical-state struct field, any function returning canonical-state-shaped data.
- **REQUIRED:** `Q32` newtype (`fixed::FixedI64<U32>`) from `fw-core`.
- `#[deny(clippy::float_arithmetic)]` is set at the crate root. If clippy lets a float through, file a bug — don't `#[allow]`.
- Trigonometry / sqrt: use the `cordic` crate on `Q32`. Never call `f64::sqrt`.

## §2. No HashMap / HashSet

- **BANNED:** `std::collections::HashMap`, `std::collections::HashSet`, `ahash::AHashMap`, any hash-randomized map.
- **REQUIRED:** `BTreeMap` / `BTreeSet` (`std::collections`) for deterministic iteration order. `IndexMap` (from `indexmap` crate) when insertion-order is the semantic.
- Why: hash maps iterate in randomized order; canonical state serialization depends on stable order across platforms + builds.

## §3. No clocks

- **BANNED:** `std::time::Instant::now()`, `SystemTime::now()`, `std::time::UNIX_EPOCH` reads, OS-time reads of any kind in sim path.
- **REQUIRED:** Sim time is `Tick(u32)` from `fw-core`. Externally-driven (test fixtures supply tick budgets; Tauri command handler supplies wall-clock when needed for UI).

## §4. No system RNG

- **BANNED:** `rand::thread_rng()`, `rand::rngs::StdRng::from_entropy()`, OsRng, `getrandom`.
- **REQUIRED:** `ChaCha8Rng::seed_from_u64(seed_fn(match_seed, tick, layer, site))` per ADR-0009. `seed_fn` lives in `fw-core::seed`; `SeedLayer` enumerates 8 non-overlapping discriminants (`Decision` / `UtilityTieBreak` / `ReactiveInterrupt` / `BallPhysics` / `SignatureTrigger` / `MemoryEvent` / `ScoutObservation` / `ContentBake`). `site` is the per-layer disambiguator (e.g. `(player_id << 16) | slot` for `Decision`).
- Same `(match_seed, tick, layer, site)` → same `u64` → same output. Always. Across platforms. BLAKE3 over a 17-byte fixed-order buffer, truncated to u64 LE.

## §5. No async / no tokio

- **BANNED in `fw-match-sim` and `fw-memory`:** `tokio`, `async fn`, `.await`, `futures::*`.
- The sim is synchronous. A match advances deterministically tick-by-tick on the calling thread.
- Tauri command handlers (in `fw-tauri`) MAY be async — they sit outside the sim ring. See `Tauri/RULES.md`.

## §6. Iteration order

- When writing canonical state, the iteration order must be stable.
- `BTreeMap::iter()` is already ordered by key.
- `Vec::iter()` is insertion-ordered — fine.
- `IndexMap::iter()` is insertion-ordered — fine.
- If you must build a temporary from a non-ordered source, **sort explicitly** before consuming.

## §7. Canonical-state hashing

- BLAKE3 via `blake3` crate. Not SHA-256. Not xxHash.
- The pinned regression test in `crates/fw-replay/tests/canonical_hash.rs` runs a fixed-seed scenario for N ticks, BLAKE3-hashes the final state, asserts == pinned hash.
- Drift on macOS / Windows / Linux CI matrix on a single platform = REGRESSION. Investigate before re-pinning.
- Re-pinning is allowed only when the task spec explicitly authorizes it; document the reason in the commit body.

## §8. Tests are mandatory for new behavior

- Any change to canonical-state-emitting code requires:
  - `insta` snapshot test of the new behavior's canonical state at a known tick.
  - `proptest` invariant covering the property the behavior preserves.
- Run `scripts/fw verify` before commit.

## §9. Serde for canonical types

- `#[derive(serde::Serialize, serde::Deserialize)]` on canonical types.
- Field order in `Serialize` impl: stable. Don't rearrange struct fields casually; it changes Bincode output bytes.
- Save migrations live in `fw-save` with versioned enum. Forward migration only.

## §10. Banned dependencies

- No `rayon` in sim crates (introduces non-determinism via thread pool).
- No `dashmap` (hash-based concurrent map).
- No `parking_lot` Mutex/RwLock in canonical-state paths (sync sim → no need).

## §11. Invariants fail in release, not just debug (post-2026-05-16 ultimate-review hardening)

Per Codex's 2026-05-16 ultimate-review verdict on T1 close: canonical and gameplay invariants MUST fail in release builds, not silently degrade. Two specific bans:

- **BANNED:** `debug_assert!` / `debug_assert_eq!` / `debug_assert_ne!` for canonical-state invariants OR gameplay-truth invariants. Release builds skip `debug_assert!` macros, which means a release-build bug that violates the invariant silently corrupts state (the documented intent of the assert never fires when it matters).
- **BANNED:** `saturating_add` / `saturating_sub` / `saturating_mul` (or any `saturating_*` arithmetic) on `Tick`, `Q32`, `PlayerSlot`, or any other sim-bearing newtype WITHOUT an explicit `// SAFETY:`-style comment justifying why saturation (vs panic-on-overflow) is the right semantic. Default discipline is checked-then-panic, matching `Q32`'s overflow behavior; saturation is opt-in via explicit `*_clamping_*` named methods.

**REQUIRED for canonical/gameplay invariants:** `assert!` / `assert_eq!` / `assert_ne!` / `panic!` — these fire in both debug + release. If the cost of running the assert in release is unacceptable for a hot-path invariant, write a unit test that exercises the invariant explicitly in test builds + skip the assert in production; do NOT use `debug_assert!` as a release-build silencer for a real invariant.

**Allowed:** `debug_assert!` for performance-sensitive sanity checks that are NOT load-bearing for canonical state OR gameplay correctness — e.g. internal helper-function preconditions that are guaranteed by the caller in well-formed code. Document the rationale inline with `// debug_assert OK here because:` comment.

This rule was added 2026-05-16 after the ultimate-review surfaced two distinct silent-failure classes:
1. `Tick` newtype saturates on overflow vs `Q32` panics — divergent policy that would silently fail-open on cooldown math invariant violations (Track C P1, Surface 3 in `docs/audits/post-t1-ultimate-review-2026-05-16.md`).
2. `ball_physics.rs` has `debug_assert!` on `is_well_formed` with an inline comment that explicitly names the silent-failure risk in release builds — exactly the pattern this rule forbids (Track C P2).

Cross-reference: T1-21 row in `docs/MASTER_PLAN.md` is the cleanup row that aligns existing `Tick` arithmetic to this policy + audits remaining `debug_assert!` sites in sim crates.

## Cross-references

- `CLAUDE.md` §3 (determinism stack), §7 (code style), §10 (pitfalls)
- `Rust/RULES.md` — general Rust style (this file overrides where stricter)
- `docs/specs/determinism-gate.md` — the canonical-hash regression contract

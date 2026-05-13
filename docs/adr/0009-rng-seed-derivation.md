# ADR-0009 — RNG seed derivation

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** Claude (Codex full-project audit P1 driver) + Codex (pending pre-T1-2b re-audit)

---

## Context

The 2026-05-13 Codex full-project audit Lane B finding P1 — "RNG seed contract is inconsistent" — surfaced that four different documents cite four different seed tuples for sim-side `ChaCha8Rng` derivation:

- `docs/DESIGN_DOC.md` line 134 — `(match_seed, tick, event_id)`
- `docs/adr/0001-match-engine-architecture.md` line 51 — `(match_seed, tick, layer_tag, decision_id)`
- `docs/adr/0003-decision-utility-math.md` line 25 — implicit `seed_fn(match_seed, tick, decision_id)`
- `docs/adr/0006-bt-vs-fsm-decision-layer.md` line 41 — `(match_seed, tick, decision_id)`

Implementing any subset of these ad-hoc would fork replay behavior the first time two layers reach for randomness in the same tick. The pinned canonical-state hash + cross-OS reproducibility floor (`docs/specs/determinism-gate.md`) depend on every random draw producing identical bits across platforms + builds. That requires every random draw resolving its seed through a single canonical function.

The pre-T1-2b deadline matters: T1-2b is the first row that actually generates randomness from the sim layer (BT-runner tie-breaking softmax via top-3 sample, archetype-driven probabilistic decorators, signature-trigger jitter). Settling the seed shape before code lands is cheaper than retrofitting.

## Decision

A single canonical `seed_fn` in `fw_core::seed` derives every sim-side RNG seed. Sites that need an `RngCore` instantiate `ChaCha8Rng::seed_from_u64(seed_fn(...))`.

### Canonical signature

```rust
// crates/fw-core/src/seed.rs (sketch — actual code in T1-2b alongside the
// first real consumer).
pub fn seed_fn(
    match_seed: u64,
    tick: u32,
    layer: SeedLayer,
    site: u32,
) -> u64;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SeedLayer {
    /// Per-player BT decision runner (ADR-0001 layer 2).
    Decision = 0x10,
    /// On-ball utility-selector tie-break softmax (ADR-0003 §6).
    UtilityTieBreak = 0x11,
    /// Reactive interrupt resolution (ADR-0001 layer 6) — coin-flip
    /// when two interrupts fire on the same tick.
    ReactiveInterrupt = 0x12,
    /// Ball-physics stochastic micro-events (deflection angle jitter,
    /// post-bounce dribble exit, ricochet outcome). Used sparingly.
    BallPhysics = 0x13,
    /// Signature-move trigger jitter (ADR-0001 layer 1 → signature
    /// dispatch) — softmax across eligible signatures.
    SignatureTrigger = 0x14,
    /// Memory event derivation (ledger writes that need a stochastic
    /// component, e.g. derby-controversy outcome). T3+.
    MemoryEvent = 0x20,
    /// Scout observation noise (T2-7+).
    ScoutObservation = 0x30,
    /// Content-bake-time procedural generation. Different `match_seed`
    /// space; lives in fw-content-baker but uses the same seed_fn for
    /// uniformity.
    ContentBake = 0x40,
}
```

### Derivation

`seed_fn` is BLAKE3 over a 17-byte fixed-order buffer, truncated to u64 LE:

```rust
pub fn seed_fn(match_seed: u64, tick: u32, layer: SeedLayer, site: u32) -> u64 {
    let mut buf = [0u8; 17];
    buf[0..8].copy_from_slice(&match_seed.to_le_bytes());
    buf[8..12].copy_from_slice(&tick.to_le_bytes());
    buf[12] = layer as u8;
    buf[13..17].copy_from_slice(&site.to_le_bytes());
    let hash = blake3::hash(&buf);
    u64::from_le_bytes(hash.as_bytes()[0..8].try_into().expect("hash ≥ 8 bytes"))
}
```

Identical to the pattern already in `fw_content::runtime::derive_seed` (which lives in the content layer for content-bake derivation); the sim-side seed_fn promotes that pattern to a `fw-core::seed` resident so EVERY random draw shares it.

### Field semantics

- `match_seed: u64` — the per-match seed published by the caller of `MatchState::initial`. For content-bake derivation, this is the per-career or per-corpus seed.
- `tick: u32` — the current `Tick::raw()` value. Caller threads `state.tick` into the call.
- `layer: SeedLayer` — the layer the draw belongs to. **Layers are non-overlapping** — every draw site picks exactly one layer; a layer never shares its space with another layer at the same `(match_seed, tick)`.
- `site: u32` — a per-layer site identifier. Examples:
  - For `Decision`: pack `(player_id as u16 as u32) | (slot_index_within_player as u16 << 16)`. Each player at most one decision-RNG instance per tick; the slot index disambiguates if a player's BT needs more than one draw.
  - For `UtilityTieBreak`: `decision_id` (a monotonic per-tick counter from the on-ball event queue).
  - For `BallPhysics`: pack `(ball_event_kind as u8) | (sub_event_counter << 8)`.
  - For `MemoryEvent`: `event_id` (the canonical ID emitted by `fw-memory`).
  - For `ScoutObservation`: pack `(scout_id as u16) | (attribute_index << 16)`.

The `site` field is the **disambiguator within a layer**, not the layer itself. Per-layer `site` schemes are documented in `docs/specs/rng-seed-sites.md` (a Tranche 4 follow-up — site catalogue grows as consumers land).

### Contract

- **Determinism floor:** identical `(match_seed, tick, layer, site)` MUST produce identical `u64` across macOS / Windows / Linux / aarch64 / x86_64. BLAKE3 + integer-only inputs guarantee this; the canonical-hash regression test enforces it transitively.
- **Non-overlapping sites:** any two draws within a `(match_seed, tick)` MUST differ in either `layer` or `site`. Violation = silent correlated draws (the two sites see the same `u64`). Reviewers should reject any new RNG call that doesn't go through `seed_fn`.
- **No system RNG.** `rand::thread_rng()` / `OsRng` / `SystemTime::now()` are banned in sim crates per `.claude/rules/Sim/RULES.md` §4. This stays banned; `seed_fn` is the only seed source.
- **One RNG per draw.** `ChaCha8Rng::seed_from_u64(seed_fn(...))` produces a fresh RNG; the call site uses it for one logical draw, then drops it. Re-using an RNG across logical sites couples them — avoid.

## Consequences

**Positive:**
- Replay reproduces across builds + platforms by construction.
- The seed surface is auditable: `grep -r "seed_fn(" crates/` enumerates every random site.
- New layers added by future ADRs reserve a `SeedLayer` discriminant + a doc entry in `rng-seed-sites.md`; no inter-layer collision risk.
- The same function powers content-bake-time derivation (replacing the existing `fw_content::runtime::derive_seed`), so bake-time and runtime share one canonical seed pipeline.

**Negative:**
- 17-byte BLAKE3 per draw is heavier than the dirt-cheap `hash(match_seed ^ tick ^ id)` form. We measured FW v1's BLAKE3-based `derive_seed` at ~150 ns on a warm cache — negligible at our draw rates (a few hundred per second at peak).
- The `SeedLayer` enum grows over time. Every addition must NOT renumber existing variants (the discriminant values are load-bearing per the same rationale as `fw_content::ContentKind` in `runtime.rs`).
- The `site: u32` packing schemes per layer are ad-hoc; they live in `docs/specs/rng-seed-sites.md` as the single source of truth. Future drift risk between code + spec.

**Neutral:**
- `fw_content::runtime::derive_seed` is functionally identical to `seed_fn(SeedLayer::ContentBake, ...)`. We keep the old function as a typed shim for the bake layer (the `ContentKind` enum there is the content-specific `site` packing scheme) and have it delegate to `seed_fn`. No breaking change to existing fw-content tests.

**Rollback path:**
- If a real seed-derivation bug surfaces post-implementation, we change the BLAKE3 truncation strategy or the buffer layout under a `seed_fn_v2` parallel function — keep v1 around until the canonical corpus rebakes against v2. The `SeedLayer` enum carries the v1 spec; v2 lives in a sibling module.

## Alternatives considered

- **`SipHash` instead of BLAKE3.** Faster (~50 ns) but `std::collections::hash_map::DefaultHasher` shapes are not guaranteed stable across Rust versions, and the explicit `siphasher` crate is fine but offers no real win over BLAKE3 which we already depend on for the canonical hash. Rejected on dep-minimization grounds.
- **`splitmix64` from a fixed seed.** Even faster (~5 ns) but the avalanche properties are worse for short inputs; we'd risk correlated draws at adjacent ticks within the same layer. Rejected on quality grounds.
- **One global `RngCore` per match, drawn from at every random site.** Simpler in code but couples every draw to the order in which they happen — a bug in one decision site silently shifts every subsequent draw. Replay debugging becomes "which draw shifted?" instead of "which draw differs from the canonical?". Rejected on auditability grounds.
- **Per-layer `RngCore` instances persisted across ticks.** Same coupling problem as above, but scoped per layer. Slight improvement; still rejected on the same grounds.
- **Tuple shape `(match_seed, tick, event_id)` (DESIGN_DOC's claim).** Three fields, no layer discriminator. The "event_id" field is a one-namespace catch-all; works fine until two layers want to draw at the same tick without sharing an event-id space. Rejected because BT-runner decisions and on-ball utility-selector decisions live in different namespaces.

## References

- `docs/specs/determinism-gate.md` §1–§3 (the contract this ADR services)
- `.claude/rules/Sim/RULES.md` §4 (RNG ban list)
- `crates/fw-content/src/runtime.rs::derive_seed` (the pattern this ADR generalizes; will delegate to `seed_fn` post-T1-2b)
- ADR-0001 §"Cadence rationale" — decisions @ 4 Hz, reactive interrupts @ 60 Hz; each fires through `SeedLayer::Decision` / `SeedLayer::ReactiveInterrupt` respectively
- ADR-0003 §6 "Tie-breaking — top-N softmax" — first real `UtilityTieBreak` consumer
- ADR-0006 §"Universal pre-emption hooks" — `ReactiveInterrupt` consumer
- Codex full-project audit Lane B finding P1 (the 4-doc tuple drift)
- `docs/specs/rng-seed-sites.md` — site-packing catalogue per layer (Tranche 4 deliverable)

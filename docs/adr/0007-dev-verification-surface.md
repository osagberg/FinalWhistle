# ADR-0007 — Dev verification surface (three layers)

**Status:** Accepted

**Date:** 2026-05-13

**Decider:** Claude (formalization of pre-existing design doc) + qa-lead + gameplay-programmer + ui-programmer (owning agents). Ratified by being committed to `docs/MASTER_PLAN.md` rows T1-2a / T1-4 / T1-9.

---

## Context

`docs/DESIGN_DOC.md` commits the shipped game to a text-first surface: 2D tactical board + commentary, no 3D viewer. That is the product. It is not the development surface.

A football sim is emergent across 22 agents plus the ball. Small attribute changes cascade into wildly different match outcomes, and the only way to tell whether a new outcome is "better football" or "different random walk" is to watch a match — or to encode what "watching" tells you into automation. FW v1 surfaced its worst behavioral bugs (static-ball convergence, brain-dead pressing, goalkeeper-wanders-to-midfield) only when a dots viewer made them visible. Phase T1's exit gate ("two teams play a match and it makes sense") cannot be defended without an answer to: how do we know it makes sense?

The pinned canonical-state BLAKE3 hash test (`crates/fw-replay/tests/canonical_hash.rs`) owns a different question — "did the simulation drift bit-for-bit across platforms or commits?" — and is bedrock for this ADR but is not in scope of it. The three layers below answer the orthogonal question: is the behavior football-shaped?

This ADR formalizes the strategy that was first written up at `docs/design/dev-verification.md` and has since been ratified by appearing as concrete rows in `docs/MASTER_PLAN.md` Phase T1 (T1-2a board, T1-4 commentary, T1-9 behavioral assertions). Research inputs:

- `docs/research/sports-sims/06-verification-qa-in-sims.md` — FM / OOTP / EHM / FOF / EA QA practices, including the 9 behavioral assertions adopted into T1-9, plus the OOTP stat-distribution and EHM two-engine techniques noted as T2 candidates.
- `docs/research/existing-rust-sims/03-openfootmanager-tests.md` — OFM's ~816 hand-rolled `#[test]`s, in particular the pair-seed knob-isolation pattern (`home_advantage_helps` runs the same seeds twice with one config flipped) we adopt as a 10th Layer-3 technique.

## Decision

We will run a **three-layer developer-tier verification surface** alongside (not as part of) the canonical-hash regression gate. The shipped game remains text-first per pillar; these layers exist for development and CI only.

### Layer 1 — Diagnostic commentary

Rich event-by-event log; football-position-aware text dense enough to spot brain-dead behavior without a visual. Lives in `fw-memory` event readers + a commentary template bank owned by `narrative-director`. Lands in **T1-4** (`fw-match-sim` event emission), authored alongside the `MatchEvent` enum.

**Cadence:** fires per significant event, every match, always on in dev builds.

### Layer 2 — 2D tactical board, dev-tier

The same PixiJS canvas the shipped UI will eventually use in T4, built minimally in T1-2a: dots, ball, tick scrubber. Top-down pitch. Lives at `frontend/src/routes/Dev/TacticalBoard.tsx`, consuming the `MatchFrameDTO` stream over Tauri IPC. **This is the dev-tier viewer, distinct from the shipped surface** — the shipped 2D board (T4) reuses the component but adds trails, role colours, signature-move highlights, and the rest of the polish.

**Cadence:** always available in dev. Toggleable from the Match page. Lands in **T1-2a** before any BT-runner work so the rendering pipeline is verified on T0's stationary fixture first.

### Layer 3 — Behavioral assertions in property tests

Things you would notice visually, encoded as invariants. Lives in `crates/fw-match-sim/tests/behavior_proptest.rs`. Authored in **T1-9** after T1-2b has been watched enough that "normal" is intuitive.

The seed set authored in T1-9:

- GK within 30 m of own goal in 95%+ of ticks across a 90-minute match.
- Team width during in-possession phases 35-65 m in 90%+ of windows.
- No player sustains >12 m/s for >4 consecutive seconds.
- Average defender depth during opponent in-possession within 8 m of the line height set by tactical archetype.
- The nine behavioral assertions from `docs/research/sports-sims/06-verification-qa-in-sims.md` (seasonal goal rate, shots/match, pass completion, top-scorer concentration, card distribution, home advantage, signature-move diversity, breakthrough trigger rate, scout-disagreement spread).
- **Pair-seed knob-isolation tests** (adopted from openfootmanager's `home_advantage_helps` pattern): run N seeds twice with a single config flag flipped, assert the directional delta. Cheap to write, hard to fake, isolates one knob at a time.

**Cadence:** every push, via the cross-OS CI matrix. The proptest suite is part of `scripts/fw verify` and gates merges.

### Sequencing rule

**Layer 2 lands before Layer 3, by design.** The board comes up in T1-2a on stationary T0 fixtures (proves rendering); the BT runner lands in T1-2b and is verified by the board + eyeball; commentary lands in T1-4 alongside event emission; behavioral invariants land last in T1-9, authored from a position of having watched enough matches to know what to encode. The order is load-bearing: writing Layer-3 assertions before having Layer 2 produces brittle invariants that pin random walks rather than "good football."

### T2 candidates surfaced but not yet rowed

Two stronger verification techniques are deferred to Phase T2 (which adds league + season). Both are documented here so the option is preserved; both will be added as explicit T2 rows in the next MASTER_PLAN update touching that phase.

- **Stat-distribution CI gate (OOTP-inspired).** Run an N-season simulation at fixed seed and KS-test the resulting goals/match, shots/match, pass-completion %, possession %, and card distribution against a pinned `docs/design/reference-distributions.ron` of real-world envelopes. Stronger than scalar invariants: catches systemic drift that scalar bounds (the 0.5-8.0-goal trap OFM falls into) cannot see.
- **Two-engine cross-check (EHM-inspired).** A lean Dixon-Coles closed-form stat sim, calibrated against real-world totals, used as a reference distribution: the full match engine's aggregate output over many seasons must match the closed-form sim's distributions within a designed band. Catches the failure mode where the pinned hash is stable but the sim has quietly drifted away from football.

Neither lands in T1 — both require a league loop to be meaningful. Both are mentioned in this ADR so they are not lost.

## Consequences

### Positive

- **Phase T1's exit gate becomes defensible.** "Matches play, the board renders them, and behavioral invariants hold over 100 random seeds" is a real bar, not a vibe.
- **Three independent failure modes covered.** Bit-for-bit regression (canonical hash), human-legible regression (commentary + board), and statistical regression (proptest invariants + future T2 stat-gate) catch different bugs.
- **The board pays for itself twice.** Built minimally in T1-2a as the dev-tier viewer; the same component is polished into the shipped 2D viewer in T4. No wasted work.
- **The 9-assertion catalog from research lands as code.** Most shipped sports sims (FM, OOTP, EHM, FOF) have nothing equivalent committed; Final Whistle gets this for the cost of authoring the assertions once.
- **Pair-seed knob-isolation tests** (the OFM technique) extend Layer 3 with cheap directional checks ("this knob actually does something") that complement the absolute invariants.

### Negative

- **Authoring cost is front-loaded into Phase T1.** Approximately a week's work pulled forward from T4 / added to T1. Already accounted for in MASTER_PLAN rows.
- **The dev-tier board is not the shipped board.** Care needed in T4 to avoid the dev affordances (debug overlays, raw numeric readouts) leaking into the shipped surface.
- **Stat-distribution and two-engine techniques are deferred.** T1 ships without them. Acceptable because both need a league loop, but worth re-checking at T2 kickoff.

### Neutral

- Layer 3's invariant bands will need tuning as the sim matures. Initial bounds are designed to catch catastrophic regressions; tightening them is a phase-gate activity, not a per-task one.
- The commentary template bank in Layer 1 is owned by `narrative-director` — it is both a dev-verification surface and a content asset, which is intentional: the templates that surface bugs textually are the same ones that ship.

## Alternatives considered

- **Skip dev-tier viewer; rely on canonical-hash + invariants only.** Rejected: FW v1's worst bugs were invisible in scalar checks but obvious on a dots viewer within seconds. Authoring invariants without first watching matches produces invariants that pin random walks. The hash gate confirms determinism, not football-shape.
- **Build the shipped 2D viewer first (skip the dev-tier variant).** Rejected: couples T1's verification needs to T4's polish requirements. The dev-tier viewer's minimal scope (dots + ball + scrubber) ships in T1-2a; the polish lands in T4 reusing the same component.
- **Adopt OOTP-style stat-distribution gating in T1.** Rejected for sequencing: the closed-loop stat gate is meaningful only over many simulated seasons, and T1 has no league loop. Documented as a T2 candidate above so the technique is preserved.
- **Author Layer-3 invariants before Layer-2 board.** Rejected: invariants written without visual reference encode "what the sim does today" rather than "what good football looks like." The board-first order ensures the invariants describe behavior, not drift.

## References

- `docs/DESIGN_DOC.md` §3 (pillars) and §6 (presentation rules — text-first for shipped game)
- `docs/design/dev-verification.md` (the original design doc this ADR formalizes)
- `docs/MASTER_PLAN.md` Phase T1 rows T1-2a, T1-2b, T1-4, T1-9, plus T1 sequencing note
- `docs/research/sports-sims/06-verification-qa-in-sims.md` (9 behavioral assertions + OOTP stat-gate + EHM two-engine techniques)
- `docs/research/existing-rust-sims/03-openfootmanager-tests.md` (pair-seed knob-isolation pattern; cautionary `f64` + `HashMap` + 0.5-8.0-goal band counter-examples)
- `docs/postmortems/phase-T0.md` (the FW v1 "we couldn't verify behavior" lesson)
- `.claude/rules/Sim/RULES.md` §7 (canonical-hash regression — the bedrock this ADR sits beside, not inside)
- `.claude/agents/qa-lead.md`, `.claude/agents/ui-programmer.md`, `.claude/agents/narrative-director.md` (owning agents)
- Prior ADRs: ADR-0001 (match engine architecture — produces the canonical state these layers verify)

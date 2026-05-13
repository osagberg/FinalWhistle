# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T1 — First Match** (active; T1-1 + T1-2a + T1-2b-i + T1-2b-ii + T1-2b-iii-a + **T1-2b-iii-b** closed; next: T1-2b-iii-c — BT site bindings + personality bias + utility-scored leaves)

## Active task

(none — T1-2b-iii-b closed. `/next` picks T1-2b-iii-c.)

## Phase pointer

- **Just closed:** **T1-2b-iii-b utility math primitives + PlayerAttributes baseline.** Five pure-function Q32 utility modules per ADR-0003 §1-§6 (xG / xT / pitch-control / pressing / softmax). New `fw-core::math` LUT module (sigmoid_q32 + exp_q32) with pure-Q32 lut_eval — f64 confined to one-shot LazyLock bake. PlayerState gained `attributes: PlayerAttributes` (mid_range_baseline; `pub(crate)` + accessor). Canonical encoder VERSION 3→4; hash rebaselined to `blake3:b3b0e64f…d4da1169`. **Fourth row under the superpowers TDD mandate.** Self-review triple BLOCKED initially (silent-failure-hunter found f64 in per-tick canonical path via `lut_eval` — broke ADR-0003's bit-exact-across-platforms promise). Fix pass closed 4 P0 + 9 P1 + 3 P2 in-place: `lut_eval` rewritten in pure Q32; `checked_*().unwrap_or()` patterns replaced workspace-wide with bare panic-on-overflow operators; `Q32::acos` debug_assert → assert!; ShotContext got try_new validator; XT_GRID + from_f64_clamped tightened to pub(crate); softmax returns Option; PitchControlOutcome gained neutral_control so sum-to-1 holds by construction; 7-invariant utility_proptest.rs added.
- **Now:** Phase T1 critical path: T1-2b-iii-b → **T1-2b-iii-c (BT site bindings + personality bias + utility-scored leaves)** → T1-2b-iii-d (PlayerSeparation + manual eyeball gate) → T1-2b-iv (signature dispatcher).
- **Next:** `T1-2b-iii-c` — wires the math primitives from iii-b into the BT runner from iii-a. 21 BT sites per `docs/specs/bt-attribute-binding.md` consume PlayerAttributes; 14-dim multiplicative personality bias matrix per `docs/design/personality-bias-weights.md` applied at every documented decision site (ADR-0003 §5); expanded PlayerIntent variants (AttemptShot / AttemptPass / Dribble / Press / etc.); stub MoveToFormationPosition leaves replaced with utility-scored picks via top-N softmax. Hash REBASELINE likely (real utility outputs change behavior).

## Blockers

None. T1-2b-iii-b shipped clean with `scripts/fw verify` green; 255 unit tests + 26 proptest integrations.

## Last green verify

2026-05-13 — `scripts/fw verify` clean post-self-review-fixes: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `b3b0e64f…d4da1169` + banned-terms + determinism-audit (q32.rs `EXEMPT_FILES` narrowly scoped post-P1-6 `from_f64_clamped` lockdown) + `fw-content-baker validate`. Cross-OS matrix verification happens on the post-commit CI run.

## Last canonical hash

`blake3:b3b0e64fbf6d5f1e1c1f54434e4c5aa277ebdcdc2815ac06e41d82d2d4da1169` (60-tick smoke seed; rebaselined T1-2b-iii-b per ADR-0012 trigger #1 — PlayerState gained 55-field `PlayerAttributes`; encoder VERSION 3→4; +9680 bytes per match-state encoding; prior pin `c0b5e395…c1430ff` was T1-2b-iii-a baseline). The hash stayed bit-identical through the P0-1 self-review fix (pure-Q32 `lut_eval` produces same bytes as the f64-tainted path because LUT entries are baked once in f64 then stored as Q32, and Q32 index arithmetic `(x+8)*16` is exact). Another rebaseline expected at T1-2b-iii-c (real utility outputs flow into BT-selected actions which mutate player vel non-trivially).

## Recent commits

- `<this commit>` feat(sim,core): T1-2b-iii-b utility math + PlayerAttributes baseline (ADR-0012 #1 rebaseline)
- `7786db0` docs(plan): further-split T1-2b-iii into iii-b/c/d
- `abebdf0` feat(sim): T1-2b-iii-a BT runner + per-role skeletons
- T1-2b-i + T1-2b-ii — see CHANGELOG.

## Next up

`/next` will pick **T1-2b-iii-c** — make the BTs actually decide. 21 BT sites consume `PlayerAttributes`; 7-consideration × 14-element personality bias matrix multiplies into every decision site; expanded `PlayerIntent` enum; utility-scored leaves replace `MoveToFormationPosition` across all role-state subtrees. TDD mandate continues. After this row, the only thing left in T1-2b is iii-d (PlayerSeparation + the manual eyeball gate where you watch a 600-tick fixture and sign off that it looks like football).

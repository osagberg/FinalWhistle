# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T1 — First Match** (active; T1-1 + T1-2a + T1-2b-i + T1-2b-ii + T1-2b-iii-a + T1-2b-iii-b + **T1-2b-iii-c** closed; next: T1-2b-iii-d — PlayerSeparation + manual eyeball acceptance gate; FINAL T1-2b row before signature dispatcher)

## Active task

(none — T1-2b-iii-c closed. `/next` picks T1-2b-iii-d.)

## Phase pointer

- **Just closed:** **T1-2b-iii-c BT site bindings + personality bias + utility-scored leaves.** The BTs actually decide things now. New `bt/` directory module with 4 submodules (personality_bias / on_ball / off_ball / reactive). 21 BT sites consume `PlayerAttributes` exactly per spec — 12 outfield utility sites + 4 reactive predicates (defined-not-wired; T1-4) + 5 GK utility-bearing states. 14-dim multiplicative personality bias matrix applied at every documented site. PlayerIntent expanded 2→19 variants. `subtree_library::select_outfield_intent()` picks via top-N softmax seeded with `SeedLayer::UtilityTieBreak`. GK FSM has real ball-position-based transitions (4/5 utility-bearing states reachable). Canonical hash rebaselined to `235f6c5e…181288d`. **Self-review triple BLOCKED initially** — silent-failure-hunter found binding-drift across ~10/12 utility sites (the agent's `*_ATTRS` consts were decorative; no test consumed them; `utility_hold_formation` violated the spec's bias-path-only caveat by reading `personality.work_rate` directly) PLUS GK frozen at InBoxPositioning (4/5 new Gk* variants unreachable). Fix pass closed 4 P0 + 4 P1 + 1 P2 in-place — walked every site against the spec, shipped the binding-correctness test suite the agent had originally punted (24 tests), implemented real GK transitions (10 predicate tests), replaced `checked_*().unwrap_or()` recidivism in lib.rs position integration with bare operators, fixed iii-b carryover bug in `xt_delta` (silently zeroing negative deltas).
- **Now:** Phase T1 critical path: T1-2b-iii-c → **T1-2b-iii-d (PlayerSeparation + manual eyeball acceptance gate)** → T1-2b-iv (signature dispatcher).
- **Next:** `T1-2b-iii-d` — final T1-2b row. PlayerSeparation pass per the FW v1 carry-forward with 6 falsifiable invariants (min-distance, deterministic pair-iteration, ball-unchanged, velocity-preservation, zero-distance fallback, runner-order regression). **Manual eyeball acceptance** on the T1-2a tactical board — you watch a 600-tick smoke fixture in `/dev/board` with two `direct-pressing` archetypes and sign off that the movement "visually resembles football." Final T1-2b hash rebaseline.

## Blockers

None. T1-2b-iii-c shipped clean with `scripts/fw verify` green; 262+ unit tests + 26 proptest integrations.

## Last green verify

2026-05-13 — `scripts/fw verify` clean post-self-review-fixes: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `235f6c5e…181288d` + banned-terms + determinism-audit + `fw-content-baker validate`. Cross-OS matrix verification happens on the post-commit CI run.

## Last canonical hash

`blake3:235f6c5e841c7b529104b5f3fa57b69315aebe439677fbbc7549c62bc181288d` (60-tick smoke seed; rebaselined T1-2b-iii-c per ADR-0012 trigger #1 — utility outputs now drive BT-selected actions which mutate player vel; PlayerIntent expanded 2→19 variants; GK FSM transitions reach utility-bearing states; RNG seed-layer corrected from `Decision` to `UtilityTieBreak` per ADR-0009; prior pin `b3b0e64f…d4da1169` was T1-2b-iii-b baseline). The self-review fix pass rebaselined twice within this row (intermediate `c392bac5…` after initial implementation, final `235f6c5e…` after the P0/P1 fix pass closed binding-drift + GK frozen-FSM + RNG layer + xt_delta sign bug). Another rebaseline expected at T1-2b-iii-d (PlayerSeparation pass adds a documented step in tick_match captured in canonical state).

## Recent commits

- `<this commit>` feat(sim): T1-2b-iii-c BT site bindings + personality bias + utility-scored leaves (ADR-0012 #1 rebaseline)
- `d471892` feat(sim,core): T1-2b-iii-b utility math primitives + PlayerAttributes baseline
- `7786db0` docs(plan): further-split T1-2b-iii into iii-b/c/d
- `abebdf0` feat(sim): T1-2b-iii-a BT runner + per-role skeletons
- earlier — see CHANGELOG.

## Next up

`/next` will pick **T1-2b-iii-d** — the final T1-2b row. PlayerSeparation pass (6 invariants) + you watching a 600-tick fixture on the tactical board and signing off it looks like football. After that, only T1-2b-iv (signature dispatcher + 3 signatures) closes T1-2b, and the rest of T1 (T1-3 through T1-9) follows.

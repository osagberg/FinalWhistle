# STATUS — Final Whistle

**Last updated**: 2026-05-17

## Phase

**T2 in progress. T2-1a + T2-1b both closed 2026-05-17.** Per-team manager archetypes now drive sim behavior end-to-end: 8 archetype RONs (2 T1 + 6 T2-1b) + 8 manager RONs (1 T1 + 7 T2-1b), per-team `archetype_params` sidecars cached on `MatchState`, `PossessionLost` + `BallRecovered` `TacticEvent` emissions in `tick_match` actually consult those sidecars per the affected team. Per-team behavioral divergence is empirically observable: `per_team_archetypes_diverge_canonical_state_after_60_ticks` proptest demonstrates default+default vs default+park-the-bus produce different canonical bytes after 60 sim ticks (proves apply_event arms use ITS OWN team's params, not a shared one).

## Active task

(none — T2-1b closed at this commit; `scripts/fw verify` exit 0; BOTH canonical hashes REBASELINED per ADR-0012 trigger #3 (sim behavior change). 5-seed envelope re-verified pre-rebaseline. Next `/next` picks **T2-1c** (final 7-12 manager archetypes — CREATIVE JUDGMENT required for archetype names + tactical-shape params; will need user ambiguity-gate confirmation per declared order + skip-DEFERRED rule).)

## Phase pointer

- **Just landed:** **T2-1b** — 6 football-canonical archetype RON files (high-press-possession / wing-overload / gegen-press / park-the-bus / tiki-taka / route-one) + 7 matching manager RON files. New `lib.rs` helpers (`team_of`, `team_arch_params`, `compute_opponent_shape_broken`, `emit_possession_transition_events`). 4-class transition taxonomy in `tick_match` snapshot-and-compare: Some→None / Some→Some same-team / Some→Some cross-team / None→Some. Each apply_event call consults the AFFECTED team's archetype_params sidecar. Bridge table-test pins all 6 new archetypes' expected `(press_intensity, counter_intent, default_in_defence_state)` tuples. Per-team divergence proptest UPGRADED from T2-1a's tick-0 schema-only to T2-1b's 60-tick behavioral. Silent-failure-hunter ACCEPT-with-P3 (no P0/P1 in-place; 2 P2 + 1 P3 deferred to follow-up). Both canonical pins rebaselined: 60-tick → `eaf842ac…ad46`; 600-tick → `5716e868…19e3`.
- **Next:** **T2-1c** — final 7-12 new manager archetypes covering broader strategic space (asymmetric setups, specialist response archetypes, edge cases). CREATIVE JUDGMENT required for archetype names + tactical-shape parameters; will need user ambiguity-gate confirmation. Total archetype count reaches the "20-30" design target from the original T2-1 row. T2-1c MAY also wire `BallInPlay` + `BallOutOfPlay` `TacticEvent` emissions to close the set-piece restart loop (deferred from T2-1b — see Known follow-ups). **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17 (friction-test rewrite); T1-25 (sig fit-score 2-candidate test); T1-26 (AC-4 via dispatch_tick path); T1-27 (BT attribute-binding table-walk proptest); T1-28 (separation EPSILON vs MIN_PLAYER_DISTANCE); T4-9 (Stretch 2D viewer). Plus T2-1b commit-body known follow-ups: `compute_opponent_shape_broken` wrapping_add→checked_add §11 hardening; goal+dispatch same-tick semantic documentation; direct unit-test matrix for `emit_possession_transition_events`'s 4 transition classes.

## Blockers

None.

## Last green verify

2026-05-17 (T2-1b close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace + pnpm test 56 frontend + banned-terms + canonical-hash regression on BOTH rebaselined pins + content-pack validate-structural + new hash-pins atomicity test + cargo audit + cargo deny).

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; REBASELINED at T2-1b from `e0312069…3696` per ADR-0012 trigger #3 — sim BEHAVIORAL change: PossessionLost + BallRecovered emissions now fire in tick_match + drive team_tactic_states[0/1] evolution within the 60-tick window).

**Second corpus pin:** `blake3:5716e86877c2d9973a713be0a49ab400fa1d4d8356bfebe9985bf5758aa619e3` (600-tick extended seed `0xfeedbeefcafefade`; REBASELINED at T2-1b from `81098579…d999` per ADR-0012 trigger #3 — same sim behavior change; home=attacking-fullback vs away=low-block-counter now actually diverge in TeamTacticState evolution across 600 ticks).

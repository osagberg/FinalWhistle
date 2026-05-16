# STATUS — Final Whistle

**Last updated**: 2026-05-17

## Phase

**T1 CLOSED 2026-05-16 at `v0.1.0-first-match`. T2 STARTED 2026-05-17 with T2-1a — per-team archetype loading + canonical schema bump (VERSION 8→9).** Foundation row of the T2-1 split is DONE. T2-1a also doubled as the Codex whole-codebase audit reconciliation: 8 of 9 findings were STALE against current HEAD (Codex audited pre-T1-close `537148f3` before T1-3.5/T1-10/T1-11/T1-15/T1-20/T1-22/T1-23/T1-24 landed); 4 still-valid findings authored as DEFERRED rows T1-25..T1-28; 2 in-place reconciliations applied (T1-3.6 status column drift + `protect-decisions.sh` MultiEdit coverage gap).

## Active task

(none — T2-1a closed at this commit; `scripts/fw verify` exit 0; **BOTH canonical hashes REBASELINED per ADR-0012 trigger #1 ONLY** — schema bump: 2 new `home_archetype_id` + `away_archetype_id` String fields appended after `last_touched_by`. Per the T2-1a silent-failure-hunter CRITICAL-1 correction, drift on BOTH pins is **SCHEMA-ONLY** — the only production `TacticEvent` consumer is `Goal` which hardcodes `MidBlock` independent of archetype; per-team behavioral divergence + trigger #3 ships at T2-1b/c when `BallInPlay`/`PossessionLost`/`BallRecovered` emissions land + consult per-team `archetype_params`. Next `/next` picks **T2-1b** (5-8 new manager archetypes — CREATIVE JUDGMENT required for archetype names + tactical-shape parameters; will need user ambiguity-gate confirmation) per declared order + skip-DEFERRED rule.)

## Phase pointer

- **Just landed:** **T2-1a** — per-team archetype loading + canonical schema bump VERSION 8→9. `MatchState` gained `home_archetype_id` + `away_archetype_id: String` canonical fields + `home_archetype_params` + `away_archetype_params: ArchetypeParams` sidecars (`pub(crate)` per CRITICAL-2; accessors added). New `tactic_fsm::archetype_params_for(&TacticalArchetype) -> ArchetypeParams` bridge with PRESERVE-CURRENT-BEHAVIOR thresholds. `MatchState::initial_with_content` widened 2-arg → 4-arg, validates both ids against `content.tactical_archetypes`. Goal-event `apply_event` site at `lib.rs:781` consumes per-team sidecar. Canonical encoder bumped + appends `[u16 LE len] [UTF-8 bytes]` per id. Self-review silent-failure-hunter REVISE with 5 findings (3 CRITICAL + 1 HIGH + 1 MEDIUM) ALL fixed in-place: CRITICAL-1 schema-only re-framing across 3 history-comment files; CRITICAL-2 pub→pub(crate) + accessors; CRITICAL-3 3-way DEFAULT_ARCHETYPE_ID/sidecar/bridge coherence test; HIGH-1 5-seed envelope sweep (strict [2,5] pinned + broader [0,7] sanity); MEDIUM-1 renamed misleading defender-depth proptest → honest schema-bump-observable round-trip. Plus 2 audit reconciliations in the same commit (T1-3.6 status column DONE + `protect-decisions.sh` MultiEdit).
- **Next:** **T2-1b** — 5-8 new manager archetype RON files under `content/sources/archetypes/` + matching `content/sources/managers/`. Creative judgment required: archetype names, tactical-shape parameters (`press_radius_metres`, `buildup_speed_factor_bps`), formation slot positions, personality biases. Will need user ambiguity-gate confirmation. Required subagent: `systems-designer` (balance) + `narrative-director` (naming). T2-1b WILL drift canonical hash per ADR-0012 trigger #3 IF + ONLY IF it also wires the `BallInPlay`/`PossessionLost`/`BallRecovered` `TacticEvent` emissions + their archetype-consuming `apply_event` arms (currently the gap that keeps T2-1a as trigger-#1-only). **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17 (friction-test rewrite, test-quality only); T1-25 (sig fit-score 2-candidate test); T1-26 (AC-4 via dispatch_tick path); T1-27 (BT attribute-binding table-walk proptest); T1-28 (separation EPSILON vs MIN_PLAYER_DISTANCE); T4-9 (Stretch 2D viewer).

## Blockers

None.

## Last green verify

2026-05-17 (T2-1a close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace + pnpm test 56 frontend + banned-terms + canonical-hash regression on BOTH rebaselined pins + content-pack validate-structural + new hash-pins atomicity test + cargo audit + cargo deny).

## Last canonical hash

`blake3:e0312069b901e16cd6caf190a7ca21401ffdd8be9d0bd18cc80280a2612f3696` (60-tick smoke seed; REBASELINED at T2-1a from `fcccb840…a751` per ADR-0012 trigger #1 — schema-only, 2 new String fields appended).

**Second corpus pin:** `blake3:8109857942e1ee2a8c429a43e89bfa5eac4582fb70ef59f1a3a04f26765ad999` (600-tick extended seed `0xfeedbeefcafefade`; REBASELINED at T2-1a from `9353bd25…947eb` per ADR-0012 trigger #1 ONLY — same schema-only character as the smoke pin per the CRITICAL-1 re-framing; not trigger #3 because the `Goal` TacticEvent arm hardcodes `MidBlock` independent of archetype).

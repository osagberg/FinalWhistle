# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Pillar Wiring + Polish — IN PROGRESS.** T4-1/2/3/4/5a/6a DONE. Career-roster layer underway (blueprint `docs/design/career-roster-layer.md`). **T4-2.5a DONE** — `AttributeFamily` moved to `fw-core` + the gene-sourced `fw-content::breakthrough_input` PA/CA bridge (`FamilyPaCa`); formula pinned in `progression.md`. T4.5 (World Scale + Content Bake) is the new EA-critical phase between T4 and T5 per re-baseline 2026-05-29 — see `docs/MASTER_PLAN.md ## Tier 4.5`.

## Active task

**T4-2.5b next** — roster data model: `CareerState.roster: BTreeMap<ClubId, Vec<PlayerInstance>>` + career-start generation (assign the 22-`PlayerTemplate` pool across 20 clubs, distinct `PlayerId`s; `generate_league` returns per-club `ProcGenTeam`) + `get_roster_for_club` IPC + `PlayerRosterDto`. Deps: T4-2.5a (DONE). Subagent: lead-programmer.

## Blockers

None live. Pre-task requirement for T4-2.5b: log Decision 5 (forward-compat clause) via `/log-decision` before implementation starts.

Previously: T2-1d2 (xg_utility honesty / cross-band oscillation) re-anchored to Deferred section of MASTER_PLAN. Open EA-scope question resolved 2026-05-29 per DECISIONS.md — DESIGN_DOC `§MVP-scope` anchor added at §8 `### MVP scope`.

## Last green verify

2026-05-29 (T4-2.5a close): `scripts/fw verify` exit 0; +19 tests (15 fw-content breakthrough_input + 4 fw-core attribute_family); clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean.

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick) + `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick). UNCHANGED through T4-2.5a (no match-state touch). **T4-2.5c will REBASELINE both** (authorized — signature candidates onto all 22 slots is a behavior change). Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

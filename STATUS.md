# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Beautiful UI + Tactical Viewer — IN PROGRESS.** T4-1/2/3/4/5a/6a DONE. **Career-roster layer underway** (resequenced 2026-05-29 ahead of T4-7/T4-9 per `docs/DECISIONS.md` 2026-05-29 + the mid-T4 fresh-eyes review `docs/audits/mid-t4-fresh-eyes-review-2026-05-29.md`; blueprint `docs/design/career-roster-layer.md`). **T4-2.5a DONE** — `AttributeFamily` moved to `fw-core` + the gene-sourced `fw-content::breakthrough_input` PA/CA bridge (`FamilyPaCa`); formula pinned in `progression.md`.

## Active task

**T4-2.5b next** — roster data model: `CareerState.roster: BTreeMap<ClubId, Vec<PlayerInstance>>` + career-start generation (assign the 22-`PlayerTemplate` pool across 20 clubs, distinct `PlayerId`s; `generate_league` returns per-club `ProcGenTeam`) + `get_roster_for_club` IPC + `PlayerRosterDto`. Deps: T4-2.5a (DONE). Subagent: lead-programmer.

## Blockers

- T2-1d2 still DEFERRED-ROLLED — end-of-T-phase rebalance pass per `personality-bias-weights.md §Re-tuning cadence`.
- Open product decision (separate `/log-decision`, not blocking): reconcile DESIGN_DOC §MVP-scope's 6-tier ~96-club LLM-baked pyramid with the shipped single-20-club-league reality.

## Last green verify

2026-05-29 (T4-2.5a close): `scripts/fw verify` exit 0; +19 tests (15 fw-content breakthrough_input + 4 fw-core attribute_family); clippy + fmt + determinism-audit + banned-terms + cargo audit/deny clean.

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick) + `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick). UNCHANGED through T4-2.5a (no match-state touch). **T4-2.5c will REBASELINE both** (authorized — signature candidates onto all 22 slots is a behavior change). Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

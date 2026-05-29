# STATUS — Final Whistle

**Last updated**: 2026-05-29

## Phase

**Phase T4 — Beautiful UI + Tactical Viewer — IN PROGRESS.** T4-1/2/3/4/5a/6a DONE. **Resequenced 2026-05-29** (`docs/DECISIONS.md` 2026-05-29 + mid-T4 fresh-eyes review `docs/audits/mid-t4-fresh-eyes-review-2026-05-29.md`): the **career-roster layer** (sub-rows T4-2.5a–h; blueprint `docs/design/career-roster-layer.md`) is inserted ahead of T4-7/T4-9 — it's the unscheduled foundation that wires pillars 2/3/4/5 into a played career + unblocks deferred T4-2b/T4-5b. Review found zero P0/P1 correctness bugs.

## Active task

**T4-2.5a next** — `fw-core::attribute_family_bridge`: map the 55 `PlayerAttributes` → the 10 `AttributeFamily` PA/CA buckets on the 1..=200 scale (`attrs_to_family_pa_ca`) + the inverse `apply_family_delta_to_ceiling`. The bridge `BreakthroughContext` requires; no current impl. Pure `fw-core`, no canonical-hash drift.

## Blockers

- T2-1d2 still DEFERRED-ROLLED — end-of-T-phase rebalance pass per `personality-bias-weights.md §Re-tuning cadence`.
- Open product decision (separate `/log-decision`, not blocking): reconcile DESIGN_DOC §MVP-scope's 6-tier ~96-club LLM-baked pyramid with the shipped single-20-club-league reality.

## Side-tracks available (independent of the roster layer)

- 4 P2 correctness fixes (2 panic-in-handler → `IpcError`; broken `just bake-content`; Transfers raw-error leak).
- Cheap pillar-visibility wins (per-signature commentary routing; career-overview blank-name render fix).

## Last green verify

2026-05-22 (T4-6a). The 2026-05-29 resequence is a docs-only planning commit.

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick) + `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick). UNCHANGED through T4 to date. **T4-2.5c will REBASELINE both** (authorized — signature candidates onto all 22 slots is a behavior change). Save wire bytes: SaveEnvelope V0–V3; SettingsEnvelope V0. T4-2.5g adds SaveV4 (`0x04`).

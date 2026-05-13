# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T1 — First Match** (active; T1-1 closed; remediation in progress per Codex full-project audit)

## Active task

(none — Codex audit remediation queued. Tranche 1 in progress this session; Tranches 2-7 queued via `/next` cycles. T1-2a starts only after Tranche 1 lands + design-tranche specs settle.)

## Phase pointer

- **Just closed:** T1-1 — `fw-content` schema lock at commit `69f900b9`. 55-field player model (ADR-0002), encapsulated `AbilityCeiling`, `RoleId` newtype, `TacticalArchetype.buildup_speed_factor: f32 → u16 bps` (Codex Imp #3 from T0), `schema_version: 1` on new content types, first RON fixtures.
- **Just landed:** Codex full-project audit (`c3945227`), P0 three-layer bedrock-`#[ignore]` guard (`eb0b952e`). Carry-forward debts logged at `821d3875`.
- **Now:** Codex audit remediation. ~50 findings triaged into 7 tranches at `docs/audits/codex-full-audit-2026-05-13.md`.
- **Next:** Tranche 1 doc-drift cleanup (this commit); then Tranche 2 (T1-1 schema follow-ups: try_new on AbilityCeiling, VISIBLE vs KNOWN attribute names split, Q32Inner re-export removal). Then Tranche 3 (ADRs for RNG seed, cadence reconciliation, save format, signatures, hash rebaseline, licensed-data, runtime-AI). Then Tranche 4 (T1-2b companion specs). THEN T1-2a.

## Blockers

None. Codex re-audit before T1-2b code is queued, not blocking T1-2a.

## Last green verify

2026-05-13 — `scripts/fw verify` green at commit `eb0b952e`: `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace` + `banned-terms` + `determinism-audit` + canonical-hash regression all clean. **Note:** local `main` is currently ahead of `origin/main`; CI on HEAD will run on next push (Tranche 1 final step).

## Last canonical hash

`blake3:d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49` (60-tick smoke seed; pinned T0-7; UNCHANGED through T1-1 + P0 fix — `MatchState` does not yet reference `PlayerAttributes`).

## Recent commits

- `eb0b952e` fix(determinism): three-layer guard against bedrock-test #[ignore] disable (Codex audit P0)
- `c3945227` docs(audit): land Codex full-project audit verbatim + triage tranches
- `821d3875` docs(refs): log FW v1 → v2 carry-forward debts from T1-1 comparison
- `69f900b9` feat(content): fw-content schema lock — ADR-0002 55-field player model [T1-1]
- `20314655` docs(adr): ADR-0008 browser-dev mode + superpowers TDD skill mandate

## Next up

Tranche 1 of audit remediation finishes this session (this STATUS commit + `.claude/launch.json` track + push to origin). Tranche 2 starts in the next session via `/next`-style cycles. T1-2a's "browser-dev wiring confirmed" claim is dialed back to "MCP tools loaded, end-to-end against a real fixture pending T1-2a" — the workflow runs but the deterministic frame JSON consumer doesn't exist yet (`crates/fw-match-sim/src/bin/dump_frames.rs` is in T1-2a's scope, not landed).

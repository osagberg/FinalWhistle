# Changelog

Append-only human-readable ship log. One line per shipped MASTER_PLAN task. Phase-summary block on `/done`. Reverse-chronological within each phase section.

---

## Phase T1: First Match — IN PROGRESS

- 2026-05-13 — Codex audit Tranche 7 (workflow + rules cleanup): retired-command refs (`/phase-gate`, `/duo-debate`) removed from SKILL.md + escalation triggers; Frontend/RULES.md §2 `@apply` ban clarified (targets shared-utility-class CSS, not @layer base); Content/RULES.md §3 + §2 carve-outs (legacy fixtures + hand-authored ID form); MEMORY.md "Queued user actions" section for Unity-MCP removal + Claude Preview MCP install. (commit `27920de6`)
- 2026-05-13 — Codex audit Tranche 6 (real ContentStore loader + real FW-VAL): `ContentStore::load_sources` walks `content/sources/{cultures,archetypes,role-affinities,players}/*.ron`; `load_baked` delegates until T2-3. `fw-content-baker validate` is no longer a stub — runs RoleAffinityTable + PlayerAttributes range validators end-to-end. Justfile `verify-content` invokes the real command (was `validate-content || echo`); now wired into `ci-local`. (commit `e79adb0`)
- 2026-05-13 — Codex audit Tranche 5 (MASTER_PLAN restructure): T1-2b split into 4 sub-rows (i ball physics / ii tactic FSM + cadence stagger / iii FSM-of-BTs + utility selector + PlayerSeparation / iv signature dispatcher). T1-9 done-criteria expanded with PlayerSeparation invariants. CI-R1 + CI-R2 (cargo audit + cargo deny) pulled forward to T1-2b-i. T2-1 split-candidate noted. (commit `1dc2fd0`)
- 2026-05-13 — Codex audit Tranche 4 (T1-2b companion specs, ~840 LoC): `tactic-fsm.md` (5 states + 2 Hz heartbeat + archetype params), `bt-attribute-binding.md` (every BT decision site + primary/secondary attribute reads + bias inputs across 21 sites), `decision-cadence-stagger.md` (4 Hz × 22-player stagger with decision_slots in canonical state), `xg-coefficients.md` (Phase-1 placeholder β₀..β₆; re-fit T2-1), `personality-bias-weights.md` (full 7×8 k₁..k₁₄ matrix). (commit `e54d59a`)
- 2026-05-13 — Codex audit Tranche 3 (4 ADR amendments + 7 new ADRs): ADR-0001 cadence 8 Hz → 4 Hz + personality vector 8 → 14; ADR-0003 personality reconciled with ADR-0002; ADR-0005 event-count 28/29 → 30 + Compaction variant added; ADR-0006 RNG ref updated. New: ADR-0009 (RNG seed derivation), ADR-0010 (save format), ADR-0011 (signature system), ADR-0012 (hash rebaseline policy), ADR-0013 (licensed-data policy), ADR-0014 (runtime AI/content boundary), ADR-0015 (phase-gate review policy). DESIGN_DOC §11 + DECISIONS.md updated. (commit `fba546b`)
- 2026-05-13 — Codex audit Tranche 2 (T1-1 schema follow-ups): `AbilityCeiling::try_new` validates ranges + invariant; `VISIBLE_ATTRIBUTE_NAMES` split from `KNOWN_ATTRIBUTE_NAMES` (CA-weights validate visible-only per ADR-0002 §"Choices" item 6); `RoleId::try_new` replaces release-build silent acceptance; `Q32Inner` re-export removed + field tightened to `pub(crate)`; `PlayerAttributes::validate_unit_range` (collect-all). 75 new tests. (commit `bf439f7`)
- 2026-05-13 — Tranche 1 of Codex audit remediation: STATUS / CHANGELOG / MEMORY / MASTER_PLAN doc-drift cleanup; `.claude/launch.json` tracked; pushed to origin. (commit `ccd4d20b`)
- 2026-05-13 — Codex full-project audit P0 fix — three-layer guard against bedrock-test `#[ignore]` disable: (1) in-process meta-test `bedrock_pinned_test_is_not_ignored` reads canonical_hash.rs source via `include_str!`, (2) CI step `Bedrock-test ignore-attr guard` in determinism-gate.yml, (3) commit hook expanded to watch `crates/fw-replay/tests/canonical_hash.rs` + `crates/fw-replay/fixtures/**`. (commit `eb0b952e`)
- 2026-05-13 — Codex full-project audit landed at `docs/audits/codex-full-audit-2026-05-13.md` — ~50 findings (1 P0, 11 headline P1, ~30 lower-severity) across 10 lanes; triaged into 7 remediation tranches. (commit `c3945227`)
- 2026-05-13 — T1-1 → v1 carry-forward debts logged: `PlayerTemplate.signature_candidates` owed at T1-3; `<Kind>Validator` pattern owed at T2-3; 46-label phenotype catalog owed at T2-4. MASTER_PLAN done-criteria amended accordingly. (commit `821d3875`)
- 2026-05-13 — T1-1 `fw-content` schema lock — ADR-0002 55-field player model (`PlayerAttributes` in `fw-core`: 14/10/8/6 visible + 14/3 hidden), `AbilityCeiling` with `pub(crate)` fields + breakthrough-only `redraw_ceiling` mutator (Pillar 3 contract), `RoleId` newtype + `RoleAffinityTable` (collect-all `invalid_roles` + `unknown_attribute_keys` validators), `TacticalArchetype.buildup_speed_factor: f32 → u16 bps` (Codex Imp #3 from T0; `BUILDUP_SPEED_BASELINE_BPS = 10_000` reference constant), `schema_version: 1` on new content types, `KNOWN_ATTRIBUTE_NAMES` const + size-of static asserts pin schema shape. First RON fixtures under `content/sources/players/` + `content/sources/role-affinities/`. 65 new tests; canonical hash UNCHANGED. Self-review triple twice → Accept. (commit `69f900b9`)

---

## Phase T0: Scaffold — APPROVED + MERGED 2026-05-13

**Codex verdict 2026-05-13:** APPROVE at `4721fee6` + hardening `bad1a400`. All workflow runs green on macOS-14 + windows-latest + ubuntu-22.04. T0-7b cross-OS canonical-hash agreement verified. Postmortem at `docs/postmortems/phase-T0.md`.

Additional commits post-`/done`:
- 2026-05-13 — **CI matrix unblock + Codex pre-merge audit** (`89479063`, `a0b2e084`, `a612e585`, `4721fee6`, `bad1a400`) — 5 commits closing the gap between dev-box green and CI matrix green. MSRV bump (fixed@1.31 → rustc 1.95); workflow path fixes; icon-tracking; Linux Tauri deps; SKILL.md atomic ordering; frontend strict-typecheck cleanup; Swatinem bin-cache hardening.

---

## Phase T0: Scaffold — closed 2026-05-13 (preliminary; superseded by APPROVED block above)

**Shipped:** an empty repo became a workspace-verifying Rust+Tauri+SolidJS scaffold with deterministic match-sim primitives, a pinned BLAKE3 canonical-state hash, a reconciled blueprint, a 9-step `/next` workflow, and Codex pre-T0 audit findings landed.

- **Crates (8 + Tauri shell)**: `fw-core` (Q32.32 panic-on-overflow + Seed + Tick + durable u32 IDs), `fw-match-sim` (22-player struct + canonical encoder + tick reducer), `fw-content` (schema + integer-bps weights), `fw-content-baker` (CLI scaffold with staged prompts/schemas/validators), `fw-scouting` (stub), `fw-memory` (MemoryEvent stub with Q32 stakes/salience), `fw-replay` (pinned-hash regression test + RON fixture + sanity-non-zero guard + corpus-agreement test), `fw-save` (SaveV1 enum stub), `fw-tauri` (DTOs + IPC handlers in sibling commands module), `src-tauri` (Tauri shell + frontend/dist stub + icon stubs).
- **Frontend**: Vite + SolidJS + Tailwind v3 + TanStack v8 + PixiJS v8 + ECharts scaffolded with 6 placeholder routes.
- **Workflow**: 7-agent roster, 6 slash commands, 5 path-scoped rule modules, 5 hooks, 3 context scopes, 8 design templates, 9 patterns.
- **Determinism gate**: pinned BLAKE3 = `d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49` on the smoke seed at tick 60. Bit-identical across 100 fresh runs intra-process on macOS-14. Cross-OS matrix agreement (T0-7b) verified by the phase-gate PR's CI.
- **Codex pre-T0 audit**: 14 of 16 findings fixed in-tree; 2 intentionally deferred (src-tauri command consolidation → T1-5; bincode dev-dep alignment → T2-9). 7 of 7 open questions resolved.
- **Canonical-state hash:** `blake3:d6258107…` (60-tick smoke seed).
- **Tests:** 19 test-runs across 13 crates + integration; all green on macOS-14 dev box. Cross-OS verification pending T0-7b.
- **Decisions logged:** 0 in `docs/DECISIONS.md` this phase (architectural decisions were made via STATUS open-questions resolution + Codex audit follow-ups, all documented in the commit bodies). First DECISIONS entry expected at T1 (BT-runner contract).

### Commit chain in T0

- `81fdeff` — Rust pivot scaffold (109 files; clean-slate rewrite from Unity+C#)
- `bc5c683` — pivot cleanup (drop Unity AI session leftovers)
- `26f1ba0` — blueprint reconciliation (51 files; 14→7 agents, 35→6 commands, 14→5 hooks)
- `7dc510d` — Codex audit followup quick wins (9 of 16; STATUS bootstrap; sanity hash test)
- `9eb184e` — Codex Q1+Q2 (Q32 panic-on-overflow + durable u32 IDs)
- `239594e` — T0-7 pin BLAKE3 canonical hash on macOS-14 dev box
- `73c1da0` — T0-12 unblock workspace verify (Tauri lib.rs `pub` bug + 4 other scaffold gaps)

### Per-task summary

- 2026-05-13 — **T0-12** Fix pre-existing scaffold build failures — fw-tauri commands moved to sibling module (Tauri 2 `pub` + `#[command]` bug, ref tauri #4665); fw-content-baker dead-code suppressed at staging-module roots (T2-3+ wiring deferred); src-tauri build.rs stubs frontend/dist + icons generated; ui-vocabulary.md meta-references wrapped in sentinels. `cargo {build,clippy,test,fmt} --workspace` ALL green.
- 2026-05-13 — **T0-7** Pin BLAKE3 canonical hash on macOS-14 dev box — `PINNED_60_TICK = d6258107…` recorded; `#[ignore]` removed; `cargo test -p fw-replay` 4/4 green. Cross-OS matrix verification deferred to T0-7b via the phase-gate PR.
- 2026-05-13 — **Codex audit followup** (`9eb184e`) — Q32 operators panic-on-overflow; IDs are durable `u32` newtypes (slotmap dropped). 7 of 7 Q1–Q7 audit decisions resolved.
- 2026-05-13 — **Codex audit quick wins** (`7dc510d`) — STATUS.md created; hash-non-zero sanity test; DESIGN_DOC float fields → Q32 basis points; fw-content weights → integer bps; determinism-audit script (comment-aware); 9 of 16 Codex findings fixed.
- 2026-05-13 — **Blueprint reconciliation** (`26f1ba0`) — 51 files; 7 agents (down from 14), 6 commands (down from 35), 5 hooks (down from 14), 3 context scopes (down from 5), Rust-flavored path-scoped RULES, 8 design templates, 9 patterns.
- 2026-05-13 — **Rust pivot scaffold** (`81fdeff`) — Clean-slate rewrite from Unity+C# to Rust+Tauri+SolidJS. 109 scaffold files across 8 crates + frontend + Tauri shell. Pre-pivot state preserved at git tag `v0-pre-pivot-2026-05-13` + sibling `/Users/vibelogic/dev/football-archive/`.

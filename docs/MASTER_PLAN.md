---
description: Tiered execution roadmap for Final Whistle (Rust + Tauri pivot). Pairs with DESIGN_DOC.md for the contract.
last_verified: 2026-05-13
---

# Final Whistle — Master Plan (Rust pivot)

> Execution source of truth. Keep synced with every `src/` change.
> Canonical pair: `docs/DESIGN_DOC.md` + `docs/MASTER_PLAN.md`.
>
> `DESIGN_DOC.md` defines the game. This file defines delivery order.

---

## Snapshot (2026-05-13)

- **Phase:** `T0 — Scaffold` (in progress; 10 of 11 rows DONE, T0-7 is critical path).
- **Current track:** awaiting first `/next` invocation on T0-7 (CI matrix green + pinned BLAKE3 hash).
- **Build health:** scaffold compiles; `scripts/fw verify` not yet exercised on the matrix.
- **Last commit:** `26f1ba0` (blueprint reconciliation).
- **Carry-forward set:** ~50 files queued from `/Users/vibelogic/dev/football-archive/` per `MIGRATION_AUDIT.md` §4.
- **Frozen Unity snapshot:** `/Users/vibelogic/dev/football-archive/`.

---

## Status Legend

| Status | Meaning |
|---|---|
| `TODO` | Not started |
| `IN_PROGRESS` | Actively being worked on |
| `DONE` | Complete with commit SHA + test evidence |
| `BLOCKED` | Waiting on dependency |
| `CUT` | Intentionally removed from scope |

Stale rule: any `IN_PROGRESS` item older than 14 days must be reviewed at next `/status` — either re-scoped, escalated, or marked `BLOCKED` with the blocker named.

---

## Now / Next / Blocked

- **Now:** none — T0-12 DONE. T0 phase essentially complete on dev box; T0-7b cross-OS matrix verification is the last gate.
- **Next:** `/done` opens the T0 phase-gate PR for Codex review; CI matrix exercises T0-7b.
- **Blocked:** none.

---

## Locked Design Constraints

Already decided in `docs/DESIGN_DOC.md` §2. Do not relitigate during normal implementation:

1. No real-world licensed players, clubs, leagues, kits, or competitions — procedural fantasy world only.
2. Deterministic Q32.32 fixed-point canonical state; pinned-hash replay corpus across Mac + Windows + Linux on every commit.
3. No runtime LLM calls. All generated text baked at content-pack build time.
4. Text-first match-day surface: 2D tactical board + dense football-native commentary. No 3D viewer, no manga-broadcast cinematic mode.
5. Event-sourced career-memory ledger is append-only; supersession via new events that cite priors.
6. Football-native UI vocabulary — no capitalized mystical state-nouns. Banned-terms lint enforced from Phase 1.
7. Mod-friendly data layer from day one: RON content packs, stable content-pack-qualified IDs, schema versioning.
8. Single-player premium product: no multiplayer, no server-side anything, no live ops.
9. Solo-dev scope discipline: every feature lives in exactly one bucket (Product MVP / Architecture-from-day-one / Dev pipeline / Deferred). Out-of-bucket features get cut, not parked.

---

## Tier Overview

| Tier | Goal | Items | Exit Gate |
|---|---|---|---|
| T0 | Scaffold | 11 | Empty repo compiles cleanly; 60-tick dummy sim canonical hash pins across CI matrix. |
| T1 | First Match | 8 | Two procedural teams play one match end-to-end; text recap surfaces with goals + score + key events. |
| T2 | League + Season | 10 | A full season cycles; league table updates; transfer-window stub UI exists; first save survives a schema-version bump. |
| T3 | Career + Memory | 8 | Multi-season careers run; memory ledger surfaces callbacks in player-facing surfaces; breakthrough events fire. |
| T4 | Beautiful UI + Tactical Viewer | 8 | Match-day live mode reads as a finished product on a stranger's screen; visual identity locked. |
| T5 | Ship to Steam | 8 | Public EA release on Steam; itch.io demo validated update pipeline first. |

**Policy:** T3-T5 items are classified MVP or Stretch. Cut Stretch before adding new scope. Total budget capped at ~55 items unless something is cut or completed.

---

## Critical Path to First Playable Match

1. `T0-1` workspace skeleton — empty repo compiles.
2. `T0-3` `fw-core` Q32.32 + Seed + Tick types — locked primitive contract.
3. `T0-4` `fw-match-sim` stub — 22-player struct + deterministic tick reducer.
4. `T0-5` canonical state encoder + BLAKE3 hash.
5. `T0-6` first insta snapshot — 60-tick canonical hash pinned.
6. `T0-7` CI matrix green on Mac/Win/Linux.
7. `T1-2` real BT runner + ball physics ported from the C# design.
8. `T1-4` event emission + ledger output.
9. `T1-5` `play_match` Tauri command + text recap rendering.
10. Execute T1 exit gate and decide go/no-go to T2.

Do not block this on UI polish, signature presentation banks, breakthrough triggers, save migration, or scouting uncertainty — all are explicitly downstream.

---

## Tier 0 — Scaffold (11 items)

**Goal:** empty repo compiles, Tauri opens, sim ticks deterministically, pinned hash matches across CI matrix, `/next` works.

**Acceptance gate:** a 22-player dummy sim runs 60 ticks and the canonical BLAKE3 hash pins identically on `macos-14`, `windows-latest`, and `ubuntu-22.04` CI.

| ID | Item | Status | Effort | Dependencies | Done Criteria |
|---|---|---|---|---|---|
| T0-1 | Cargo workspace skeleton — 9 crates (`fw-core`, `fw-match-sim`, `fw-content`, `fw-content-baker`, `fw-memory`, `fw-replay`, `fw-scouting`, `fw-save`, `fw-tauri`) all compile clean | DONE | M (1d) | — | `cargo build --workspace` green (commit 81fdeff). Naming note: `fw-content-baker` replaces the originally-planned `fw-cli` name. |
| T0-2 | Tauri 2 + SolidJS + Tailwind frontend shell — 6 placeholder routes (Home / Squad / Tactics / Transfers / League / Match) | DONE | M (2d) | T0-1 | Tauri shell opens at 81fdeff. |
| T0-3 | `fw-core`: `Q32` newtype (i64-backed Q32.32) + `Seed` + `Tick` + `MatchId` types with derive-locked PartialOrd/Ord/Hash | DONE | M (2d) | T0-1 | Landed at 81fdeff. Open follow-up per Codex audit: bare `+ - * /` operators on Q32 wrap silently in release — decision pending (remove operator impls vs add `clippy::arithmetic_side_effects` deny). |
| T0-4 | `fw-match-sim`: 22-player struct + deterministic tick reducer (no behavior yet — stationary players, no ball) | DONE | M (1d) | T0-3 | Landed at 81fdeff. `MatchState::initial` + `tick_match` reducer in place. |
| T0-5 | Canonical state encoder + BLAKE3 hash function in `fw-core` | DONE | M (1d) | T0-3, T0-4 | Hand-rolled little-endian encoder at `crates/fw-match-sim/src/canonical.rs` (81fdeff). Switched to BLAKE3 (was SHA-256 in earlier plan). |
| T0-6 | Canonical-hash regression test wiring (pinned hash constant, RON fixture, three-test surface in `crates/fw-replay/tests/canonical_hash.rs`) | DONE | S (0.5d) | T0-5 | Test wired, fixture exists (81fdeff). Sanity test `smoke_seed_canonical_hash_is_nonzero` added (7dc510d) prevents all-zero footgun. Pinning the actual hash is T0-7's job. |
| T0-7 | Pin the BLAKE3 canonical hash on the macOS-14 dev box. Update `crates/fw-replay/tests/canonical_hash.rs::PINNED_60_TICK` + `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron::expected_hash` to the real value; remove `#[ignore]` from `smoke_seed_60_tick_canonical_hash_pinned`. CROSS-OS matrix verification (Win + Linux agreement) is deferred to the `/done` phase-gate workflow. | DONE | M (2d) | T0-6 | Landed at 239594e. Pinned hash `blake3:d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49`. `cargo test --release -p fw-replay`: 4 passed / 1 ignored (insta baseline) / 0 failed. |
| T0-7b | Cross-OS canonical-hash agreement — GitHub Actions matrix `[macos-14, windows-latest, ubuntu-22.04]` runs the un-ignored `smoke_seed_60_tick_canonical_hash_pinned` test and all three platforms produce the same BLAKE3 hash. Drift on any platform = real determinism leak; investigate + fix. | TODO | M (2d) | T0-7 | All three CI jobs green on the phase PR opened by `/done`; total wall-clock ≤6 min; no `--ignored` flag needed. |
| T0-8 | `Justfile` (or `cargo make`) with dev / test / build / lint / ci-local commands | DONE | S (0.5d) | T0-1 | Justfile + `scripts/fw` bash front-door at 81fdeff. Reconciliation (26f1ba0) added `banned-terms` + `verify-content` + `determinism-audit` recipes. |
| T0-9 | `/next` slash-command implementation + auto-self-review hook | DONE | M (1d) | T0-1 | Full workflow reconciled at 26f1ba0. 6 commands, 7 agents, 5 hooks, path-scoped rules. |
| T0-10 | `docs/DECISIONS.md` + `protect-decisions.sh` hook | DONE | S (0.5d) | T0-1 | Hook live at 81fdeff; verified in reconciliation. |
| T0-11 | `README.md` + `REFERENCES.md` | DONE | S (0.5d) | T0-1 | Both at 81fdeff; REFERENCES.md updated at this audit-followup commit (15→7 agents). |
| T0-12 | Fix pre-existing scaffold build failures. (a) fw-tauri `#[tauri::command]` E0255 — root cause: known Tauri 2 bug when `pub` + `#[tauri::command]` are applied inside `lib.rs`. Fix: moved both commands to a sibling `commands.rs` module and re-exported. Ref: tauri-apps/tauri discussion #4665. (b) fw-content-baker dead-code — root cause: 10 consts + 4 fns authored ahead of T2-3 wiring. Fix: `#![allow(dead_code)]` at the three staging module roots with TODO(T2-3/T2-4/T3-3/T3-5) comments. (c) src-tauri frontend/dist requirement — root cause: `tauri::generate_context!` validates `frontendDist` at compile time. Fix: `build.rs` stubs `frontend/dist/index.html` on fresh clones; Vite overwrites on real build. (d) src-tauri icons missing/non-RGBA — root cause: scaffold left `icons/` empty. Fix: solid-green stub PNGs (RGBA) + icon.icns / icon.ico via magick + sips (gitignored — real art lands at T4). (e) ui-vocabulary.md meta-references — root cause: catalog file mentions banned terms it bans, tripping the lint. Fix: `<!-- ui-lint:ignore-start/end -->` blocks around the meta-references. | DONE | S (1d) | T0-1 | `cargo build --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace --release` + `cargo fmt --check` + `determinism-audit` + `banned-terms` ALL CLEAN. 19 test-runs across all crates, all green. |

### T0 Exit Gate (locked)

- `cargo build --workspace` + `cargo test --workspace` + `cargo clippy -- -D warnings` + `cargo fmt --check` all green on Mac/Win/Linux CI.
- Canonical-hash regression test pins the same BLAKE3 across all three OSes.
- `pnpm tauri dev` opens the shell on the dev machine.
- `/next` picks `T1-1`.
- Vertical-slice tag: `v0.1.0-scaffold`.

---

## Tier 1 — First Match (8 items)

**Goal:** the user can click "Play Match" between two procedural teams and get a text recap with goals, score, and key events.

| ID | Item | Status | Effort | Dependencies | Done Criteria |
|---|---|---|---|---|---|
| T1-1 | `fw-content` schema: `TeamTemplate` + `PlayerTemplate` + `BehaviorArchetype` (serde + RON files under `content/base/`) | TODO | M (2d) | T0-1 | Round-trip serde test; RON files load via `cargo test` |
| T1-2 | `fw-match-sim`: ball physics + 22-player BT runner (port from C# design — gravity, drag, Magnus, bounce, friction; semi-implicit Euler in Q32) | TODO | XL (1w) | T0-4, T1-1 | 600-tick run with two `direct-pressing` archetypes produces ball trajectory + player positions; canonical hash pinned |
| T1-3 | `fw-match-sim`: signatures stub — type system only, no triggers yet (`SignatureId` + `SimBiasSnapshot` + stacking policy types) | TODO | M (2d) | T1-2 | Types compile; one no-op signature applies in the BT runner without affecting hashes |
| T1-4 | `fw-match-sim`: event emission — `MatchEvent` enum (Goal / Shot / Pass / KickOff / FullTime) + ledger output struct | TODO | M (2d) | T1-2 | Events emit in tick-order; hash includes event stream; replay reconstructs events identically |
| T1-5 | `fw-tauri`: `play_match` command returning serialized `MatchResult` (final score + event list + canonical hash) | TODO | M (2d) | T1-4 | `pnpm tauri dev` → click Play → console shows scoreline; round-trip via Tauri IPC preserves canonical hash |
| T1-6 | Frontend: Match page with "Play" button, text recap rendering (goals + minute markers), simple event-list view | TODO | M (3d) | T1-5 | Stranger reads the recap and understands what happened in <60s; no broken states |
| T1-7 | Procedural content stub — 22 player names (Markov chain seeded by region prior) + 2 team names + 1 manager archetype YAML port | TODO | M (2d) | T1-1 | Two distinct teams generated from one seed; same seed → identical names |
| T1-8 | Replay corpus fixture #1 — smoke seed, 600 ticks, two-archetype matchup, pinned canonical hash on CI matrix | TODO | S (1d) | T1-2, T1-4, T0-7 | `fixtures/replay-corpus/0xfeedbeefcafefade.json` exists; CI matrix green on all three OSes |

### T1 Exit Gate (locked)

- "Play Match" produces a sensible text recap (2-5 goals total across the 600 ticks, no NaN-tier weirdness).
- Replay corpus has ≥2 fixtures, both pin across CI matrix.
- `cargo test --workspace` green; clippy + fmt clean.
- No `unwrap()` calls in `fw-match-sim` non-test code.
- Vertical-slice tag: `v0.1.0-first-match`.

---

## Tier 2 — League + Season (10 items)

**Goal:** a full season plays through. Many matches, league table updates, basic transfer window UI surfaces.

| ID | Class | Item | Status | Effort | Dependencies | Done Criteria |
|---|---|---|---|---|---|---|
| T2-1 | MVP | `fw-match-sim`: full BT runner with all 20-30 manager archetypes (port YAML from `MatchSim/Content/archetypes/*.yaml`) | TODO | L (1w) | T1-2 | Each archetype produces visibly distinct match flow; canonical hash pinned per archetype-pair |
| T2-2 | MVP | `fw-content`: 20 procedural clubs in a fantasy second-tier league (one-nation pyramid slice for season-loop testing) | TODO | M (3d) | T1-1, T1-7 | 20 clubs distinct; 380-fixture season schedule generates deterministically from a seed |
| T2-3 | MVP | `fw-content`: bake-time content-baker binary stub (`fw-cli bake`) — Claude API call → RON corpus → manifest with model-id + prompt-hash + seed | TODO | L (1w) | T1-1 | One end-to-end bake produces 100 player names + manifest; offline runtime sample reproduces identically |
| T2-4 | MVP | `fw-content`: `PlayerBio` generation — names per cultural region, basic personalities, 22-field gene model (port from `design/player-generation.md`) | TODO | L (1w) | T2-3 | 500 players generated; phenotype-label catalog covers all; ID-stability test passes round-trip |
| T2-5 | MVP | `fw-tauri`: season-controller commands — `advance_week`, `play_fixtures`, `get_standings`, `get_fixtures(club_id)` | TODO | M (3d) | T2-1, T2-2 | All four commands serialize to/from frontend; advancing a full season takes <30s on dev machine |
| T2-6 | MVP | Frontend: League page — standings table (TanStack Table with sortable columns: P / W / D / L / GF / GA / GD / Pts) | TODO | M (3d) | T2-5 | Sort by any column; dark-mode + light-mode both readable |
| T2-7 | MVP | Frontend: Squad page — player list per club (TanStack Table; columns: name / age / role / phenotype labels / contract end) | TODO | M (3d) | T2-4, T2-5 | All 20 clubs' squads viewable; phenotype labels render as text (NOT raw gene numbers — banned-terms lint catches if leaked) |
| T2-8 | MVP | Frontend: Transfer-window stub — UI shell only, no transfer mechanics; "window opens / window closes" state visible | TODO | M (2d) | T2-6 | UI shows window state per game-date; mechanic implementation deferred to T3 |
| T2-9 | MVP | `fw-save`: bincode-based save format + version-migration enum chain — first schema version locked at `1` | TODO | L (1w) | T2-5 | Save → load → byte-identical state; version `0` → `1` migration test passes |
| T2-10 | MVP | Phase-gate Codex review #1 — full T0+T1+T2 architecture review on the agent-bus (optional carry-forward; ports `scripts/agent-bus` only if review-loop is structurally in scope) | TODO | M (2d) | T2-9 | Codex posts ack on a `phase-gate-T2` topic; any blocking counters resolved or logged as deferred |

### T2 Exit Gate (locked)

- A full season plays through end-to-end on the dev machine with no panics.
- League table updates after each match-day batch.
- Save → quit → load reproduces canonical state byte-identically.
- Banned-terms lint passes on every UI surface that touched Squad / League pages.
- Vertical-slice tag: `v0.2.0-season`.

---

## Tier 3 — Career + Memory (8 items)

**Goal:** multi-season careers with event-sourced memory surfacing in player-facing copy; breakthroughs fire.

| ID | Class | Item | Status | Effort | Dependencies | Done Criteria |
|---|---|---|---|---|---|---|
| T3-1 | MVP | `fw-memory`: ledger storage + persistence — append-only `MemoryEvent` records keyed by `event_id` (port schema from `adr-0004-memory-event-schema.md`) | TODO | L (1w) | T2-9 | Append-only invariant tested; load-time migration framework in place; 1000-event ledger round-trips in <100ms |
| T3-2 | MVP | `fw-memory`: 5 readers — alumni-DB / rival-recall / promise-tracking / big-match-scars / press-fan-callbacks | TODO | XL (2w) | T3-1 | Each reader has ≥3 unit tests + one integration test against a seeded multi-season ledger |
| T3-3 | MVP | `fw-content`: news headlines + manager-quote templates via Tracery-style grammars; phrase banks loaded from RON | TODO | M (3d) | T3-1, T2-3 | Slot-filling deterministic on `(career_id, event_id)` seed; banned-terms lint green |
| T3-4 | MVP | `fw-content`: breakthrough events — signature awakening + latent-flag unlock + regressive collapse triggers (port from `design/breakthrough-moments.md`) | TODO | L (1w) | T3-1 | Across a 5-season career, 1-3 breakthroughs fire per player on average; structured text recap surfaces |
| T3-5 | MVP | `fw-scouting`: scout-uncertainty model — single-scout-report variant first (Path B fallback); 3 archetypes if Phase-3 feel-prototype passes | TODO | XL (2w) | T2-4 | Report data shape locked; uncertainty bands display as text labels, not numbers |
| T3-6 | MVP | Frontend: Player detail page with memory callbacks — surfaces top-5 ledger events for the player + readable phenotype block + contract block | TODO | M (3d) | T3-2, T2-7 | Callback text renders football-grade ("Scored the winner in the '98 cup final"); no template seams visible |
| T3-7 | MVP | Save migration tests — four-test-per-bump discipline (forward / callback-preservation / forward-incompat / round-trip) for version `1` → `2` (a deliberate field add) | TODO | M (3d) | T2-9, T3-1 | All four tests green; per-version fixtures committed under `fixtures/save-migration/v0001-to-v0002/` |
| T3-8 | MVP | Phase-gate Codex review #2 — multi-season-determinism + memory-ledger-integrity review | TODO | M (2d) | T3-7 | Codex posts ack on `phase-gate-T3` topic; any blocking counters resolved |

### T3 Exit Gate (locked)

- A 5-season career runs end-to-end; ledger compaction fires at the 5-season boundary.
- At least one cross-season callback surfaces in a press-conference or pre-match overlay screen.
- Save format survives one deliberate schema bump.
- Vertical-slice tag: `v0.3.0-career`.

---

## Tier 4 — Beautiful UI + Tactical Viewer (8 items)

**Goal:** polish pass. Match-day live mode reads as a finished product. Visual identity locked.

| ID | Class | Item | Status | Effort | Dependencies | Done Criteria |
|---|---|---|---|---|---|---|
| T4-1 | MVP | PixiJS tactical board — pitch + 22 dots + ball; smooth interpolation at 30Hz from the 60Hz canonical state | TODO | L (1w) | T1-2 | Visual replay of a 600-tick fixture reads as football; no jitter, no determinism leak |
| T4-2 | MVP | ECharts stat dashboards — per-player + per-team + per-season views; sortable, filterable | TODO | L (1w) | T2-6, T2-7 | Three stat views ship; data binding via Tauri IPC; no perf regressions on 5-season careers |
| T4-3 | MVP | Tailwind theming pass — lock visual identity (Anton / JetBrains Mono / Inter; muted-pitch-green + accent-yellow/red; dark-mode first-class) | TODO | M (3d) | T2-6 | Style guide doc + 3 reference screenshots committed under `docs/visual/` |
| T4-4 | MVP | Loading / empty / error states for every screen — no white-flash, no raw-error-text leaks | TODO | M (3d) | T4-2 | Manual QA pass: every route survives a Tauri-backend kill |
| T4-5 | MVP | Match-day live mode — tick-by-tick board update + commentary feed in the right rail | TODO | L (1w) | T4-1, T3-3 | Pause / play / 1× / 4× / 16× speeds; auto-sim-to-next-salience-band-N event mode works |
| T4-6 | MVP | Settings screen — accessibility (text-scale, colorblind palette toggle, reduce-motion), key rebinds, save-folder location | TODO | M (3d) | T0-2 | All settings persist via the save layer's version-migration chain |
| T4-7 | MVP | Game-shell polish — window chrome (Tauri title bar customization), main-menu, splash screen, app-icon | TODO | M (3d) | T4-3 | Looks like a finished product on a stranger's machine; app-icon ships per-OS |
| T4-8 | MVP | Phase-gate Codex review #3 — UI / accessibility / performance review | TODO | M (2d) | T4-7 | Codex posts ack on `phase-gate-T4` topic |

### T4 Exit Gate (locked)

- A stranger watching a match-day live-mode session for 3 minutes reports they understand drama + momentum + player identity without reading a design doc.
- All accessibility settings actually work, not just exist in UI.
- Vertical-slice tag: `v0.4.0-polish`.

---

## Tier 5 — Ship to Steam (8 items)

**Goal:** public Steam EA release. itch.io demo first validates the update pipeline.

| ID | Class | Item | Status | Effort | Dependencies | Done Criteria |
|---|---|---|---|---|---|---|
| T5-1 | MVP | Apple Developer enrollment ($99) + Mac code-signing via `apple-codesign` in CI | TODO | M (3d) | T4-7 | Mac DMG signs + notarizes + Gatekeeper-passes on a clean machine |
| T5-2 | MVP | Steam Direct ($100) + `steamworks-rs` integration — achievements + cloud saves + rich presence | TODO | L (1w) | T5-1 | Three test achievements unlock + cloud-save round-trip works on a second machine |
| T5-3 | MVP | Steam Deck Verified prep — 1280×800 UI sweep, controller-mappable inputs, suspend/resume safety | TODO | L (1w) | T4-6 | Steam Deck dev hardware (borrowed if needed) survives 30-minute suspend/resume; rating: Verified or Playable |
| T5-4 | MVP | Localization pipeline — i18n string-extraction + `fluent` runtime + English-only at EA but pipeline ready for translator handoff | TODO | M (3d) | T4-3 | One non-English test locale loads + renders; lint catches hardcoded strings |
| T5-5 | MVP | Performance pass — profiling with `samply` + Tracy; optimize hot paths in `fw-match-sim` + `fw-memory` readers | TODO | L (1w) | T3-2 | A full 10-season career runs in <60s on the dev machine; tactical-board live mode p95 frame time ≤16.6ms |
| T5-6 | MVP | itch.io demo release first — validate update pipeline before Steam | TODO | M (3d) | T5-4 | Two consecutive demo updates ship cleanly to itch.io; <50 external testers report no install issues |
| T5-7 | MVP | Steam EA release — store page, EA roadmap public, launch trailer cut from in-game footage | TODO | L (1w) | T5-2, T5-6 | Public EA available; first 24h crash-free rate ≥98% |
| T5-8 | MVP | Phase-gate Codex review #4 (pre-EA) — release-readiness audit, security review, content-policy + AI-disclosure audit | TODO | M (2d) | T5-7 | Codex posts ack on `phase-gate-T5` topic; no `p0` or `p1` counter remaining |

### T5 Exit Gate (locked)

- Public Steam EA live.
- Steam Deck Verified or Playable rating granted.
- First-week crash-free rate ≥98%.
- itch.io demo build remains live as a fallback distribution channel.
- Vertical-slice tag: `v1.0.0-ea`.

---

## Dependencies

```
T0 ─┬─> T1 ─┬─> T2 ─┬─> T3 ─┬─> T4 ─┬─> T5
    │       │       │       │       │
    │       │       │       │       └─ T4 depends on T1-2 ball physics + T3-3 prose templates
    │       │       │       └─ T3 depends on T2-3 baker + T2-4 player generation + T2-9 save layer
    │       │       └─ T2 depends on T1-2 BT runner + T1-1 RON content schema
    │       └─ T1 depends on T0-3 fixed-point types + T0-5 canonical encoder + T0-6 corpus harness
    └─ T0 blocks everything; nothing parallelizes around it
```

**Hard blocks:** T0-3 (fixed-point) blocks everything sim-side. T0-5 (canonical encoder) blocks corpus + replay + cross-OS CI. T2-9 (save layer) blocks T3 multi-season testing because there's no way to round-trip a long career.

**Parallelizable within a tier:** T1-1 (content schema) + T1-7 (procedural content stub) can land before T1-2 (full sim) closes. T2-6/T2-7/T2-8 (UI pages) can land in any order once T2-5 (Tauri commands) lands. T4-1/T4-2/T4-3 run in parallel.

---

## Acceptance Evidence per Task

Each task's `DONE` row carries: **commit SHA + test names + evidence link**. Pattern (taken from Crumble Arena's MASTER_PLAN, adapted):

```
T1-2 | DONE | `<sha>` | `cargo test -p fw-match-sim::ball_physics` (24 passing), `fixtures/replay-corpus/0xfeedbeefcafefade.json` pinned | 2026-MM-DD
```

Evidence lives in `docs/evidence/<tier>-<id>/` for screenshot artifacts; pinned fixture hashes live in `fixtures/replay-corpus/*.json` (append-only).

---

## Deferred Scaffolding Trackers

| ID | Item | Trigger | Status | Notes |
|---|---|---|---|---|
| CI-R1 | `cargo audit` | Start of T1 | TODO | Wire as `fw verify audit` subcommand |
| CI-R2 | `cargo deny` | T1 mid | TODO | Add `deny.toml`; ban GPL-class licenses in production deps |
| CI-R3 | Coverage gate (`cargo llvm-cov`) | After T2-9 | TODO | Floors: `fw-core` ≥80, `fw-match-sim` ≥75, `fw-memory` ≥70 |
| CI-R4 | Cross-platform reproducibility test (build → SHA hash artifacts) | Start of T3 | TODO | Build-time reproducibility separate from runtime determinism |
| CI-R5 | Banned-terms lint Python port (`scripts/lint-banned-terms.py`) wired to `fw verify banned-terms` | T0-9 | TODO | Carry-forward verbatim from FW archive per MIGRATION_AUDIT §3.5 |

---

## Risk Register

| Tier | Risk | Why it matters | Mitigation |
|---|---|---|---|
| T0 | Q32.32 implementation chooses wrong primitive (i64 vs i128 vs fixed-point crate) and either overflows under realistic match-sim ranges or is too slow | Late refactor would break the canonical-hash corpus mid-stream | Bench round 1 with i64-backed Q32.32; if any single matrix op overflows in a 600-tick simulation, escalate to i128. Decision recorded in DECISIONS.md |
| T1 | Rust port of BT runner introduces non-determinism vs the C# version (e.g. HashMap iteration, floating-point sneaking in) | Determinism is the foundational contract; finding drift at T3 is catastrophic | Port one archetype first; compare tick-by-tick against an archived C# corpus output before porting the rest. Forbidden-primitive lint via clippy custom rule (no `HashMap` in `fw-match-sim/sim/**`) |
| T2 | Content baker drifts unsupervised — model output changes between bakes; same prompt → different RON → silent breakage | Mid-EA "rebake" producing different player names than shipped saves expect breaks live careers | Manifest pins `model_id + prompt_hash + seed`; baker output is reviewed before commit; runtime never re-bakes |
| T3 | Memory-ledger compaction loses callback-eligibility for events it shouldn't | Cross-decade callbacks are Pillar 2 — silent loss = pillar betrayal | Compaction test corpus: 100 ledgers across 10 seasons, assert ≥95% of `recall_after_seasons ≤ 8` events survive compaction with their callback-tags intact |
| T4 | Tactical-board renders the canonical state at 30Hz but introduces drift between board-frame and canonical hash | Determinism gate fails because viewer interpolation leaks into canonical state | Strict layering: viewer reads canonical state, never writes. Hash check runs on `fw-match-sim` output independently of any viewer code |
| T5 | Apple notarization fails for Tauri 2 + custom signing pipeline on first attempt | Could delay EA by 2-4 weeks | Notarize a hello-world Tauri 2 app during T4 as a dry-run before T5-1 starts |

---

## Changelog

| Date | Change |
|---|---|
| 2026-05-13 | MASTER_PLAN authored. T0-T5 tier structure + 53 items + dependency graph + risk register defined. Pairs with DESIGN_DOC.md. Pulls carry-forward set from MIGRATION_AUDIT.md §4. |

---

*Authored 2026-05-13. Revise tier exit gates and risk-register entries at every phase transition. Decision moves go through `docs/DECISIONS.md` (append-only).*

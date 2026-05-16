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

- **Phase:** T1 (First Match) active. T1-1 (fw-content schema lock) closed at `69f900b9`. Codex full-project audit Tranches 1-7 closed (`eb0b952e..27920de6`). Codex pre-T1-2b re-audit returned YELLOW (0 new P0, 7 residual P1) — those P1s being fixed in the current commit set. **Next: clear residual P1s, re-run re-audit, then `/next` picks T1-2a.**
- **Build health:** local `main` synced with `origin/main` through `af7df8fa`. Determinism Gate green on HEAD; full CI matrix runs on each push. Pinned BLAKE3 canonical hash matches cross-platform.
- **Codex audits on disk:** `docs/audits/codex-full-audit-2026-05-13.md` (full-project audit) + `docs/audits/codex-pre-t1-2b-prompt.md` (re-audit Tier-2 prompt). Pre-T1-2b re-audit findings being remediated in-flight.
- **Carry-forward set:** ~50 files queued from `/Users/vibelogic/dev/football-archive/` per `MIGRATION_AUDIT.md` §4 (mostly consumed during pivot + reconciliation; remainder lands during T1). v1→v2 carry-forward debts logged at `REFERENCES.md` "Carry-forward debts" table.
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

- **Now:** none — T1-2a closed. Dev-tier 2D tactical board shipped: `MatchFrameDto` in `fw-match-sim::dto`, `match_frames` IPC command, `dump_frames` binary, SolidJS `TacticalBoard.tsx` with PixiJS rendering, FrameSource trait + dual impls, `/dev/board` route, `window.fwDev` DEV-only debug surface. E2E verified via Claude Preview MCP — pitch + 22 dots + ball + scrubber render from a real `dump_frames` fixture; `window.fwDev.scrubTo(N)` drives the scrubber.
- **Next:** `T1-2b-i` ball physics (per Tranche-5 T1-2b split). The TDD mandate fires here (per `docs/DECISIONS.md` 2026-05-13 superpowers TDD mandate) — first real behavior-code row in fw-match-sim.
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
| T1 | First Match | 23 (was 13; +1 T1-2b-fix mid-phase Codex audit, +1 T1-3.5 ball-mutation Codex 2026-05-16 audit P0, +1 T1-4 split into T1-4a + T1-4b, +4 hardening rows T1-10..T1-13 Codex 2026-05-16 audit P1s) | Two procedural teams play one match end-to-end; 2D tactical board renders the match readably; behavioral proptest invariants hold; text recap surfaces with goals + score + key events. |
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
7. `T1-2a` dev-tier 2D tactical board for in-loop verification, then `T1-2b-i` (ball physics) → `T1-2b-ii` (tactic FSM + decision cadence stagger) → `T1-2b-iii-a` (BT runner + per-role BT skeletons) → `T1-2b-iii-b` (utility math primitives + PlayerAttributes baseline) → `T1-2b-iii-c` (BT site bindings + personality bias + utility-scored leaves) → `T1-2b-iii-d` (PlayerSeparation + visual playtest gate) → `T1-2b-iv` (signature dispatcher + 3 signatures end-to-end). The T1-2b split was applied via Codex audit Tranche 5 remediation 2026-05-13. The T1-2b-iii three-way split was applied 2026-05-13 (post-T1-2b-ii) when chunk-count exceeded `/next`'s 7-chunk ceiling. T1-2b-iii was further split 2026-05-13 (post-T1-2b-iii-a) when the original iii-b was found to bundle math primitives + attribute plumbing + BT wiring + personality bias into ~11 chunks; iii-b now covers the deterministic math + attribute shape; iii-c covers BT integration + personality bias; iii-d (previously iii-c) covers PlayerSeparation + the manual eyeball acceptance gate.
8. `T1-4a` MatchEvent enum + emission + canonical encoding → `T1-4b` Tracery commentary template bank (split 2026-05-16 per scope discipline; T1-4b owns the narrative-director-authored ≥3-variants-per-event template bank).
9. `T1-5` `play_match` Tauri command + text recap rendering.
10. Execute T1 exit gate and decide go/no-go to T2.

Do not block this on UI polish, signature presentation banks, breakthrough triggers, save migration, or scouting uncertainty — all are explicitly downstream.

---

## Tier 0 — Scaffold (11 items)

**Goal:** empty repo compiles, Tauri opens, sim ticks deterministically, pinned hash matches across CI matrix, `/next` works.

**Acceptance gate:** a 22-player dummy sim runs 60 ticks and the canonical BLAKE3 hash pins identically on `macos-14`, `windows-latest`, and `ubuntu-22.04` CI.

| ID | Item | Status | ~~Effort~~ (deprecated) | Dependencies | Done Criteria |
|---|---|---|---|---|---|
| T0-1 | Cargo workspace skeleton — 9 crates (`fw-core`, `fw-match-sim`, `fw-content`, `fw-content-baker`, `fw-memory`, `fw-replay`, `fw-scouting`, `fw-save`, `fw-tauri`) all compile clean | DONE | — | — | `cargo build --workspace` green (commit 81fdeff). Naming note: `fw-content-baker` replaces the originally-planned `fw-cli` name. |
| T0-2 | Tauri 2 + SolidJS + Tailwind frontend shell — 6 placeholder routes (Home / Squad / Tactics / Transfers / League / Match) | DONE | — | T0-1 | Tauri shell opens at 81fdeff. |
| T0-3 | `fw-core`: `Q32` newtype (i64-backed Q32.32) + `Seed` + `Tick` + `MatchId` types with derive-locked PartialOrd/Ord/Hash | DONE | — | T0-1 | Landed at 81fdeff. Open follow-up per Codex audit: bare `+ - * /` operators on Q32 wrap silently in release — decision pending (remove operator impls vs add `clippy::arithmetic_side_effects` deny). |
| T0-4 | `fw-match-sim`: 22-player struct + deterministic tick reducer (no behavior yet — stationary players, no ball) | DONE | — | T0-3 | Landed at 81fdeff. `MatchState::initial` + `tick_match` reducer in place. |
| T0-5 | Canonical state encoder + BLAKE3 hash function in `fw-core` | DONE | — | T0-3, T0-4 | Hand-rolled little-endian encoder at `crates/fw-match-sim/src/canonical.rs` (81fdeff). Switched to BLAKE3 (was SHA-256 in earlier plan). |
| T0-6 | Canonical-hash regression test wiring (pinned hash constant, RON fixture, three-test surface in `crates/fw-replay/tests/canonical_hash.rs`) | DONE | — | T0-5 | Test wired, fixture exists (81fdeff). Sanity test `smoke_seed_canonical_hash_is_nonzero` added (7dc510d) prevents all-zero footgun. Pinning the actual hash is T0-7's job. |
| T0-7 | Pin the BLAKE3 canonical hash on the macOS-14 dev box. Update `crates/fw-replay/tests/canonical_hash.rs::PINNED_60_TICK` + `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron::expected_hash` to the real value; remove `#[ignore]` from `smoke_seed_60_tick_canonical_hash_pinned`. CROSS-OS matrix verification (Win + Linux agreement) is deferred to the `/done` phase-gate workflow. | DONE | — | T0-6 | Landed at 239594e. Pinned hash `blake3:d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49`. `cargo test --release -p fw-replay`: 4 passed / 1 ignored (insta baseline) / 0 failed. |
| T0-7b | Cross-OS canonical-hash agreement — GitHub Actions matrix `[macos-14, windows-latest, ubuntu-22.04]` runs the un-ignored `smoke_seed_60_tick_canonical_hash_pinned` test and all three platforms produce the same BLAKE3 hash. Drift on any platform = real determinism leak; investigate + fix. | DONE (de-facto since 2026-05-13 / explicit 2026-05-16) | — | T0-7 | All three CI jobs green on the phase PR opened by `/done`; total wall-clock ≤6 min; no `--ignored` flag needed. **2026-05-16:** flipped TODO → DONE per Codex 2026-05-16 audit P1 "state docs can mislead /next" — T0 closed at `27920de6` (2026-05-13) and the cross-OS CI matrix has stayed green through every subsequent hash rebaseline (T1-2b-i / -ii / -iii-a / -iii-b / -iii-c / -iii-d / -iv / -fix R1 / T1-4a). Row stayed TODO by oversight. |
| T0-8 | `Justfile` (or `cargo make`) with dev / test / build / lint / ci-local commands | DONE | — | T0-1 | Justfile + `scripts/fw` bash front-door at 81fdeff. Reconciliation (26f1ba0) added `banned-terms` + `verify-content` + `determinism-audit` recipes. |
| T0-9 | `/next` slash-command implementation + auto-self-review hook | DONE | — | T0-1 | Full workflow reconciled at 26f1ba0. 6 commands, 7 agents, 5 hooks, path-scoped rules. |
| T0-10 | `docs/DECISIONS.md` + `protect-decisions.sh` hook | DONE | — | T0-1 | Hook live at 81fdeff; verified in reconciliation. |
| T0-11 | `README.md` + `REFERENCES.md` | DONE | — | T0-1 | Both at 81fdeff; REFERENCES.md updated at this audit-followup commit (15→7 agents). |
| T0-12 | Fix pre-existing scaffold build failures. (a) fw-tauri `#[tauri::command]` E0255 — root cause: known Tauri 2 bug when `pub` + `#[tauri::command]` are applied inside `lib.rs`. Fix: moved both commands to a sibling `commands.rs` module and re-exported. Ref: tauri-apps/tauri discussion #4665. (b) fw-content-baker dead-code — root cause: 10 consts + 4 fns authored ahead of T2-3 wiring. Fix: `#![allow(dead_code)]` at the three staging module roots with TODO(T2-3/T2-4/T3-3/T3-5) comments. (c) src-tauri frontend/dist requirement — root cause: `tauri::generate_context!` validates `frontendDist` at compile time. Fix: `build.rs` stubs `frontend/dist/index.html` on fresh clones; Vite overwrites on real build. (d) src-tauri icons missing/non-RGBA — root cause: scaffold left `icons/` empty. Fix: solid-green stub PNGs (RGBA) + icon.icns / icon.ico via magick + sips (gitignored — real art lands at T4). (e) ui-vocabulary.md meta-references — root cause: catalog file mentions banned terms it bans, tripping the lint. Fix: `<!-- ui-lint:ignore-start/end -->` blocks around the meta-references. | DONE | — | T0-1 | `cargo build --workspace` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace --release` + `cargo fmt --check` + `determinism-audit` + `banned-terms` ALL CLEAN. 19 test-runs across all crates, all green. |

### T0 Exit Gate (locked)

- `cargo build --workspace` + `cargo test --workspace` + `cargo clippy -- -D warnings` + `cargo fmt --check` all green on Mac/Win/Linux CI.
- Canonical-hash regression test pins the same BLAKE3 across all three OSes.
- `pnpm tauri dev` opens the shell on the dev machine.
- `/next` picks `T1-1`.
- Vertical-slice tag: `v0.1.0-scaffold`.

---

## Tier 1 — First Match (23 items)

**Goal:** the user can click "Play Match" between two procedural teams and get a text recap with goals, score, and key events. The developer can verify it's actually football via the 2D tactical board + behavioral assertions.

**Scope note (2026-05-13 reframe):** effort estimates ("M / L / XL / 1d / 1w") have been deprecated across this plan. They were carry-overs from the solo-developer framing that LoC-bounded the architecture choices. Per `docs/DESIGN_DOC.md` §1 "Scope ambition", implementation scope is bounded by determinism contract + maintainability + pillar promises, not by hours or lines. A row is done when its acceptance criteria pass — not when a clock-budget says so.

**Verification surface:** see `docs/design/dev-verification.md` for the three-layer dev-tier strategy (diagnostic commentary + tactical board + behavioral proptest invariants) that closes the "is this really football?" gap left by FW v1's text-only iteration.

| ID | Item | Status | ~~Effort~~ (deprecated) | Dependencies | Done Criteria |
|---|---|---|---|---|---|
| T1-1 | `fw-content` schema: `TeamTemplate` + `PlayerTemplate` + `BehaviorArchetype` (serde + RON files under `content/sources/`). `PlayerTemplate` MUST conform to **ADR-0002**'s 55-field model (38 visible + 17 hidden/support, all `Q32` in `[0,1]`, GK fields on flat struct, separate `PlayerCondition` struct, role-affinity weights in content-pack RON, FOF-style scout-range projection). Folds in Codex Imp #3 deferred from T0 (`TacticalArchetype.buildup_speed_factor: f32 → u16 bps` integer-only sampling). | DONE (2026-05-13) | — | T0-1 | `cargo test --workspace` green (65 new tests on fw-core + fw-content; pinned canonical hash UNCHANGED). 55-field player model in `fw-core::PlayerAttributes` (14/10/8/6 visible + 14/3 hidden); `AbilityCeiling` encapsulated (`pub(crate)` fields + `redraw_ceiling` breakthrough mutator); `KNOWN_ATTRIBUTE_NAMES` const enumerates all 55 for FW-VAL key validation. `fw-content::PlayerTemplate` wraps these + `schema_version: 1` + typed `RoleId` newtype; `PlayerCondition` deliberately NOT serialized into PlayerTemplate (runtime modulator state, initialized at projection time). `fw-content::RoleAffinityTable` ships with sum-to-10_000 + collect-all `invalid_roles` + `unknown_attribute_keys` validators. `TacticalArchetype.buildup_speed_factor` converted `f32 → u16 bps` with `BUILDUP_SPEED_BASELINE_BPS = 10_000` reference constant. First RON fixtures live under `content/sources/players/` + `content/sources/role-affinities/`; load tests in `crates/fw-content/tests/fixtures_load.rs`. Self-review triple ran twice — final verdict Accept across all three. |
| T1-2a | **Dev-tier 2D tactical board** (verification surface — pulled forward from T4, per ADR-0007 Layer 2; **browser-dev mode per ADR-0008**). `frontend/src/routes/Dev/TacticalBoard.tsx` consumes `MatchFrameDTO` via a `FrameSource` trait with two impls: `TauriFrameSource` (default, IPC) and `HttpFrameSource` (browser-dev, reads JSON fixture file). URL param `?source=fixture:/path.json` switches modes. Renders 22 dots + ball + tick scrubber on top-down pitch. Always-on for dev; not the shipped UI (that's T4 polish). Also lands `crates/fw-match-sim/src/bin/dump_frames.rs` — small binary that produces deterministic fixture JSON for any seed. Exposes `window.fwDev` debug surface (DEV-build only) for Claude Preview to drive the scrubber via `preview_eval`. | DONE (2026-05-13) | — | T0-2 | (1) `pnpm tauri dev` → `/dev/board` → dots render from a T0 stationary fixture via Tauri IPC; scrubber advances tick; no jank. (2) `cargo run --bin dump_frames -- --seed 0xdeadbeef --ticks 60 > frontend/public/dev-fixtures/smoke.json` produces deterministic JSON (byte-identical across reruns; gitignored ephemeral dir). (3) `pnpm --filter ./frontend dev` then `localhost:1420/dev/board?source=fixture:/dev-fixtures/smoke.json` renders the same dots from the fixture (no Tauri runtime needed; Vite serves `public/` at root). (4) Claude Preview workflow validated end-to-end: `preview_start` against the fixture URL succeeds; `preview_screenshot` produces a recognizable pitch + dots image; `preview_eval "window.fwDev.scrubTo(N)"` advances the rendered state. |
| T1-2b-i | **`fw-match-sim`: ball physics** — semi-implicit Euler in Q32 (gravity, drag, Magnus, bounce, friction). Ported from FW v1 `BallPhysics.cs`. Pinned ball-only canonical-hash sub-fixture. Codex audit Lane B + Lane I drove the T1-2b split (the original row was too broad — ball / runner / steering / events / signatures all bundled). | DONE (closed 2026-05-13) | — | T0-4, T1-1, T1-2a | 600-tick ball-only fixture (no players) produces deterministic trajectory; canonical hash pinned cross-OS; ball-physics-specific proptest invariants (energy decay monotonic, never goes infinite, bounce coefficients in archetype range) |
| T1-2b-ii | **`fw-match-sim`: tactic FSM + decision cadence stagger** — implements `docs/specs/tactic-fsm.md` (5 states + 2 Hz heartbeat + archetype params) AND `docs/specs/decision-cadence-stagger.md` (4 Hz per-player runner with deterministic slot assignment in canonical state). No BT yet; players hold position. The plumbing for the decision cadence + tactic-state propagation. | DONE (closed 2026-05-13) | — | T1-2b-i | `decision_slots: [u8; 22]` lives in canonical `MatchState`; canonical-hash regression test pins the new layout; slot-assignment determinism test green; tactic-FSM transition determinism proptest green; canonical hash REBASELINED per ADR-0012 trigger #1 |
| T1-2b-iii-a | **`fw-match-sim`: BT runner + per-role BT skeletons** — implements ADR-0006 BT infrastructure (Tree / Node / Status / traversal). Per-outfield-role BT skeletons (10 roles per `docs/specs/bt-attribute-binding.md`) at the SKELETON tier: every BT compiles + traverses + returns a "move-to-position" leaf action; NO real decision logic yet (that's -iii-b). Pure-FSM goalkeeper (separate from outfield BTs per ADR-0006). MatchState gains per-player BT state (current node index per player). Wires `should_decide()` from T1-2b-ii into the per-tick dispatch loop. Per-player `local_decision_counter` joins canonical state (RNG site derivation per ADR-0009). | DONE (closed 2026-05-13) | — | T1-2b-ii | BT runner + 10 outfield BT skeletons + GK FSM compile + run; per-player BT state encoded canonically; `tick_match` iterates 22 roster slots calling `should_decide` + dispatches; `local_decision_counter` increments deterministically; **canonical hash REBASELINED** per ADR-0012 trigger #1; insta snapshot of canonical state at tick 60 matches across reruns; BT traversal proptest (any seeded run produces a deterministic trace) |
| T1-2b-iii-b | **utility math primitives + `PlayerAttributes` baseline** — implements ADR-0003 §1–§6 as pure-function Q32 modules with NO BT wiring yet (that's -iii-c). New `fw-core::math` module: 257-entry `sigmoid_q32` LUT + `exp_q32` LUT per ADR-0003 §1 (load-bearing math primitive shared across xG / pitch-control / softmax). New `fw-match-sim::utility` module: `xg_utility` (6-feature logistic per `docs/design/xg-coefficients.md`), `xt_delta` + hand-authored 192-entry `XT_GRID` const, `pitch_control` (Spearman per-point time-to-intercept), `pressing_intensity` (Bauer 2025 product form), `pick_top_n_softmax` tie-breaker (ADR-0003 §6 + ADR-0009 `SeedLayer::UtilityTieBreak`). `PlayerState` gains `attributes: PlayerAttributes` initialized to mid-range placeholder values (real content-pack loading defers to T1-5/T1-6). | DONE (closed 2026-05-13) | — | T1-2b-iii-a | All 5 math modules exist as pure Q32 functions; proptest determinism invariants green (same inputs → same outputs across reruns); sigmoid_q32 monotonicity proptest + xG-mean-in-realistic-range proptest + pitch-control sums-to-1 proptest + pressing-intensity-in-[0,1] proptest + softmax-determinism proptest; `PlayerAttributes` field present on `PlayerState` (mid-range baseline values); **canonical hash REBASELINED** per ADR-0012 trigger #1 (PlayerState canonical schema bump for `attributes` field); NO BT wiring yet — leaves still return `MoveToFormationPosition` |
| T1-2b-iii-c | **BT site bindings + personality bias + utility-scored leaves** — wires the math primitives from -iii-b into the BT runner from -iii-a. The 21 BT sites per `docs/specs/bt-attribute-binding.md` consume `PlayerAttributes` for primary/secondary attribute reads. 14-dimensional multiplicative personality bias matrix per `docs/design/personality-bias-weights.md` applied at every documented decision site per ADR-0003 §5 (7-consideration × 14-element table; k₁..k₁₄ Phase-1 placeholder weights). New `PlayerIntent` variants (`AttemptShot` / `AttemptPass` / `Dribble` / `Press` / etc.); stub `MoveToFormationPosition` leaves replaced with utility-scored decisions picked via top-N softmax. | DONE (closed 2026-05-13) | — | T1-2b-iii-b | All 21 BT sites consume documented `PlayerAttributes` per spec; personality bias matrix applied at every decision site; expanded `PlayerIntent` enum; utility-scored leaves replace `MoveToFormationPosition` across all role-state subtrees; **canonical hash REBASELINED** (real utility outputs flow into selected actions which mutate player vel non-trivially); insta snapshot at tick 60 produces visibly differentiated player behavior; manual eyeball gate **NOT YET** (defers to -iii-d) — focus is correctness of the wiring + bias application |
| T1-2b-iii-d | **`fw-match-sim`: PlayerSeparation + visual playtest gate** — final T1-2b row before signature dispatcher. PlayerSeparation pass per the FW v1 `PlayerSeparation.cs` carry-forward. Runs at a documented step within `tick_match` (captured in canonical hash). **Manual eyeball acceptance** on the T1-2a tactical board — user watches a 600-tick smoke fixture in `/dev/board` with two `direct-pressing` archetypes and signs off that the rendered movement "visually resembles football" (per ADR-0007 dev-verification Layer 2). | DONE (closed 2026-05-15) | — | T1-2b-iii-c | **PlayerSeparation acceptance** (6 falsifiable invariants from FW v1 carry-forward): (a) min-distance invariant (no two players closer than 0.4m for >2 ticks), (b) deterministic pair-iteration order (BTreeSet/Vec only — no HashMap), (c) ball position unchanged by separation, (d) velocity preservation magnitude (\|v_after\| ∈ [0.95·\|v_before\|, 1.05·\|v_before\|]), (e) zero-distance fallback (when two players are exactly co-located, deterministic resolution by player_id), (f) runner-order regression (separation pass runs at a documented step within tick — captured in canonical hash). **Manual eyeball gate:** user opens `/dev/board?source=fixture:/dev-fixtures/smoke-600t.json` (new fixture from this row), watches the 22-dot replay scrub, confirms the movement passes the "looks like football" smell test, types `eyeball: PASS — <one-line observation>` in the commit body. **Canonical hash REBASELINED** for the separation-pass step in tick_match. |
| T1-2b-iv | **`fw-match-sim`: signature dispatcher + first 3 signatures end-to-end** — partial implementation of ADR-0011 to validate the dispatcher path. 3 representative signatures: one defensive (e.g. `BodyShieldPressure`), one attacking (e.g. `LongRangeStrike`), one build-up (e.g. `FirstTimeDiagonalSwitch`). Each implements `TriggerPredicate`, `SimBiasSnapshot`, basic `PresentationRecipe`. Cooldown state in canonical `MatchState`. Per-player `signature_candidates` schema landed in T1-3 (separate row); this row consumes it. | DONE (closed 2026-05-15) | — | T1-2b-iii-d, T1-3 | 3 signatures fire in test fixtures; cooldown enforced; softmax dispatch deterministic via `SeedLayer::SignatureTrigger`; bias snapshot multiplies into utility scoring; `MemoryEvent::SignatureFirstFired` emitted; canonical hash REBASELINED (intentional) |
| T1-3 | `fw-match-sim`: signatures stub — type system only, no triggers yet (`SignatureId` + `SimBiasSnapshot` + stacking policy types per ADR-0011). **Adds `signature_candidates: Vec<SignatureCandidate>` to `fw_content::PlayerTemplate`** (carry-forward debt from FW v1's `IdentityPacket.SignatureCandidates`, deliberately deferred at T1-1 — see `REFERENCES.md` carry-forward table). Each entry pairs `SignatureId` (content-pack-qualified, `^fwh\.core(?:\.v[0-9]+)?:signature\.[a-z0-9-]+$`) with a `Q32` affinity weight in `[0, 1]`. Without it, Pillar 5 has no per-player linkage. Real triggers + dispatch in T1-2b-iv. | DONE (closed 2026-05-15) | — | T1-2b-ii |
| T1-2b-fix | **`fw-match-sim`: T1-2b post-audit fix pass** — addresses 8 P1s + 6 P2s from the Codex Tier-2 mid-phase audit (2026-05-15) covering the full T1-2b sub-phase. Substrate violations (ADR-0009 SeedLayer + seed_fn owed move to fw-core with correct discriminants + byte layout + tick u32; signature_candidates affecting future behavior must be canonical-state for replay soundness; cross-category signature stacking must allow simultaneous firings per ADR-0011 §"Stacking policy"; signature softmax must include `affinity × event-class-fit` per ADR-0011 §"Dispatch + softmax"); architecture drift (outfield must route through SubtreeLibrary + bt::tick_tree per ADR-0006, not call select_outfield_intent directly; GK FSM must consume PlayerAttributes per bt-attribute-binding.md §"Goalkeeper-specific" 5 sites; 6 BT site bindings still drift from bt-attribute-binding.md spec); test-vacuousness recurrence (AC-2/3/4/5 from T1-2b-iv still pass when invariants never observed; rewrite to FAIL when invariant is violated, not "pass when bad thing didn't happen"); P2 cleanup (pitch_control unwrap violates T1 exit gate; ADR-0012 rebaseline-doc mislabels trigger #1 vs #3; determinism-audit file-level exemptions too broad). | DONE (closed 2026-05-15) | — | T1-2b-iv | All 8 Codex P1s addressed; ≥5 Codex P2s addressed; canonical hash REBASELINED at least once (SeedLayer move + signature_candidates encoding + cross-category stacking each force a bump); test-vacuousness pattern measurably closed (each AC test FAILS when seeded with invariant-violating state); re-run Codex Tier-2 audit on the fix-pass diff comes back APPROVE | Types compile; one no-op signature definition loads from RON without affecting hashes; `PlayerTemplate.signature_candidates` field exists + serde-round-trips + has at least one fixture entry |
| T1-3.5 | **DONE (closed 2026-05-16).** **`fw-match-sim`: Ball mutation + possession state + goal detection (audit-driven row; gates T1-4b).** Codex 2026-05-16 whole-codebase audit P0: normal match path cannot produce real ball actions — `apply_intent` treats Shot/Pass/Cross/LayOff/Dribble/GK distribution as "move the player toward target" with no ball.vel mutation, no possession transfer, no goal detection. T1-4a shipped `MatchEvent` emission, but events describe player INTENTIONS not football OUTCOMES (`Pass.completed: T1_PASS_COMPLETED = true` hardcoded; ball doesn't move on Shot; Goal variant structurally unreachable). This row adds: (a) canonical `possession: Option<PlayerSlot>` field on `MatchState` (the player currently in possession; `None` for loose ball / set-piece pause); (b) `last_touched_by: Option<PlayerSlot>` for possession-chain scorer attribution; (c) ball-physics mutation in `apply_intent`: AttemptShot sets `ball.vel` toward target with shooter-attribute-modulated power + transfers possession to None; AttemptPass-class intents set ball.vel toward (`from_slot` → `to_slot`) path + transfer possession to `to_slot` (with `T2_PASS_COMPLETED` contest model deferred — pass deterministically completes for now BUT actually moves the ball, so the event stream describes a real ball trajectory); Cross + LayOff sub-variants of the same Pass treatment; Dribble updates possession-holder position without releasing the ball; GK distribution mirrors Pass treatment but from goalkeeper slot. (d) Goal-detection: at end of each tick, check if ball crosses goal line (`ball.pos_x` past pitch end + `|ball.pos_y| < GOAL_HALF_WIDTH_M`); emit `MatchEvent::Goal { scorer_slot: last_touched_by.unwrap_or_panic, … }` + bump `home_score`/`away_score` + reset ball to centre-spot + emit `MatchEvent::KickOff { is_second_half: false }` + (Phase-2 polish: emit `TacticEvent::Goal` to drive tactic-FSM transition). (e) Tactic-FSM Goal event integration (closes the Codex backlog item: "tactic FSM event transitions are not integrated"). (f) Possession-transfer events: do NOT add `MatchEvent::PossessionTransfer` in T1 (the Vec would explode in size); possession is derivable from the Pass event stream + last-touched chain. **Acceptance criteria**: smoke seed 60-tick run produces ≥1 ball-position change (`ball.pos_x` or `pos_y` non-stationary at tick 60 vs tick 0); ≥1 `MatchEvent::Pass` produces a `ball.vel != 0` directly after that tick; if smoke seed has BT-shooting frequency high enough, ≥1 `MatchEvent::Shot` matched with ball.vel toward goal; `possession: Option<PlayerSlot>` is in canonical state + encoded; tests added: insta snapshot of ball+possession+match_events at tick 60 (anti-vacuousness guard asserts ball.pos != centre-spot); proptest invariant `ball_moves_when_pass_or_shot_fires` (force a Pass/Shot intent + assert ball.vel non-zero next tick). | TODO | — | T1-4a | Ball mutation lives in `apply_intent`; `possession: Option<PlayerSlot>` + `last_touched_by: Option<PlayerSlot>` fields in canonical `MatchState`; goal detection at tick end emits Goal + bumps score + emits new KickOff + (optional) drives `TacticEvent::Goal` through tactic-FSM; canonical hash REBASELINED per ADR-0012 trigger #1 (canonical schema bump); insta snapshot of ball+possession+match_events at tick 60 + ≥2 proptest invariants green. Tactic-FSM Goal event integration closes Codex backlog item "tactic FSM event transitions are not integrated." Codex re-audit on this row's diff returns APPROVE on the P0 ball-mutation finding. |
| T1-4a | **`fw-match-sim`: MatchEvent enum + emission + canonical encoding (sim side; row split from T1-4 per scope-discipline; T1-4b owns commentary).** `MatchEvent` enum lives in `fw-content::event` (placed there so T1-4b's `fw-content::commentary` renderer doesn't depend on fw-match-sim; reverse dep direction is what fw-match-sim already does for SignatureDefinition). 6 variants: `Goal` / `Shot` / `Pass` / `KickOff` / `FullTime` / `SignatureFirstFired`. `PlayerSlot` moves from `fw-match-sim::lib` to `fw-core::lib` (small substrate cleanup; same pattern as T1-2b-fix's SeedLayer move). `MatchState` gains `match_events: Vec<MatchEvent>` (canonical, NOT serde-skip; encoder VERSION 6→7). Emission paths: KickOff at tick=0, FullTime at match_end, Goal forward-compat encoder-only (wiring deferred to T1-9/T2 ball-in-net detection), Shot from BT `AttemptShot` intent, Pass from BT pass-class intents, SignatureFirstFired reconciled from `signature::ledger::MemoryEvent` (the local stub gets DELETED). | DONE (closed 2026-05-16) | — | T1-2b-iii-d | MatchEvent enum lives in fw-content::event; PlayerSlot moved to fw-core; encoder VERSION 6→7 (T1-2b-fix had already bumped 5→6 unnoticed in spec); 5 live emission paths wired + Goal forward-compat encoder-tested; signature::ledger::MemoryEvent stub deleted; insta snapshot at tick 60 + 2 proptest invariants green (events_chronological + determinism-across-runs); canonical hash REBASELINED `d376ba26…fa93` → `02ab97d0…27e686` per ADR-0012 trigger #1. Self-review triple: 3 agents → Revise; 8 P0/P1 closed in main-thread fix-pass (GOAL_HALF_WIDTH_M single-source-of-truth; `is_shot_on_target` + `nearest_teammate_near` `i64::MIN.abs()` panic fix via `unsigned_abs()`/i128; exhaustive `apply_intent` event-emission match; `T1_PASS_COMPLETED` const extraction; `match_events`/`match_end_tick` pub→pub(crate)+accessor; FullTime `==`→`>=` with already-emitted gate; deleted dead `apply_tactic_event_with_emission` helper; direct `encode_match_event(Goal)` unit test). |
| T1-4b | **DONE (closed 2026-05-16).** **`fw-content::commentary`: Tracery template bank + deterministic renderer (content side; row split from T1-4).** Tracery RON templates per Content/RULES.md §4 + ADR-0007 Layer-1 contract ("templates that surface bugs textually are the same ones that ship"). ≥3 variants per MatchEvent slot (≥18 templates total: 3 × 6 events). New `SeedLayer::Commentary` discriminant in `fw-core::seed` (per ADR-0009 amendment logged at `docs/DECISIONS.md` 2026-05-16 entry — prereq DONE). Renderer picks variant deterministically via `seed_fn(match_seed, tick, SeedLayer::Commentary, site)` where site = `((player_slot as u32) << 16) \| event_class_discriminant`. Read aloud once each before merge per Content/RULES.md §9. Hash UNCHANGED (templates are content; canonical event stream lives in T1-4a's match_events). Owned by `narrative-director` per CLAUDE.md §5 + ADR-0007 line 87. **Dep updated 2026-05-16 per Codex audit triage**: was `T1-4a`; now `T1-4a, T1-3.5` so commentary renders real ball-action outcomes not player-intent placeholders. | TODO | — | T1-4a, T1-3.5 | `content/sources/commentary/*.tracery.json` exist with ≥3 variants per event; renderer in fw-content compiles + renders MatchEvent → String deterministically (same seed + same events → same prose); Tracery variant-pick uses SeedLayer::Commentary; FW-VAL content validation passes; narrative-director sign-off via read-aloud gate; canonical hash UNCHANGED |
| T1-5 | **`fw-tauri`: `play_match` command + frontend IPC consolidation + match_frames bound (Codex 2026-05-16 audit folds in two P1s).** `play_match` command returns serialized `MatchResult` (final score + event list + canonical hash). `match_frames` streaming command feeds T1-2a board. **Folds in Codex T0 Imp #10 (P3 deferred) PLUS Codex 2026-05-16 audit P1 reframing**: `src-tauri/main.rs` currently has local stubs `get_dummy_state` + `play_match` while `match_frames` comes from `fw-tauri::commands` — two backend truths in one app. T1-5 makes `fw-tauri` the only command surface (delete src-tauri local stubs; src-tauri/main.rs has ZERO local `#[tauri::command]` impls; all commands delegate to fw-tauri). **PLUS** Codex 2026-05-16 audit P1: `match_frames` accepts unbounded `tick_count` parameter → caller can request millions of ticks + force OOM/CPU exhaustion. Add backend `MAX_FRAMES_PER_REQUEST` const (suggest 7200 = 2 minutes of match time at 60Hz; configurable later) + mirrored frontend validation in `FrameSource.ts` + typed `IpcError::TooManyFrames { requested, max }` per Tauri/RULES.md §4. Plus IPC contract tests per Codex audit recommendation (round-trip + error-shape tests in `fw-tauri/tests/`). | TODO | — | T1-4a, T1-4b | `pnpm tauri dev` → click Play → console shows scoreline; round-trip via Tauri IPC preserves canonical hash; src-tauri/main.rs has zero local `#[tauri::command]` impls; `match_frames` rejects request with `tick_count > MAX_FRAMES_PER_REQUEST` returning `IpcError::TooManyFrames`; IPC contract tests round-trip MatchResult + verify error shapes |
| T1-6 | Frontend: Match page with "Play" button, text recap rendering (goals + minute markers), simple event-list view. Reuses T1-2a board component (debug toggle to surface it during a live match). | TODO | — | T1-5 | Stranger reads the recap and understands what happened in <60s; toggling the dev-board mid-recap shows the moment in 2D |
| T1-7 | Procedural content stub — 22 player names (Markov chain seeded by region prior) + 2 team names + 1 manager archetype RON port | TODO | — | T1-1 | Two distinct teams generated from one seed; same seed → identical names |
| T1-8 | Replay corpus fixture #1 — smoke seed, 600 ticks, two-archetype matchup, pinned canonical hash on CI matrix | TODO | — | T1-2b-iii-d, T1-4a, T0-7 | `crates/fw-replay/fixtures/0xfeedbeefcafefade.ron` exists; CI matrix green on all three OSes |
| T1-9 | **Behavioral assertions** (verification surface — see ADR-0007 + dev-verification §Layer 3). `crates/fw-match-sim/tests/behavior_proptest.rs` with the T1 subset of ADR-0007's invariant catalogue: (a) the 4 positional invariants — GK within 30m of own goal 95%+ of ticks; team width 35-65m during in-possession; no sustained >12m/s sprint >4s; defender depth tracks tactical archetype within 8m. (b) **PlayerSeparation invariants** (Codex Lane D carry-forward from v1's `PlayerSeparation.cs`): clumping resistance — for any 100-tick window across 50 random seeds, ≤5 pairs of teammates closer than 1m for more than 30 consecutive ticks; opposing-player separation respects 0.4m floor under contest. (c) Pair-seed knob-isolation tests adopted from openfootmanager's `home_advantage_helps` pattern, for at least 3 knobs: home-advantage, press intensity, formation depth. (d) `events_chronological` proptest invariant. The remaining 5 stat-distribution assertions from ADR-0007 (goals/match, shots/match, pass completion, top-scorer concentration, card distribution) defer to T2 where season-length aggregates are observable. | TODO | — | T1-2b-iii-d | All 4 positional invariants hold over 100 random seeds; PlayerSeparation invariants hold over 50 random seeds; all 3 pair-seed tests produce directional deltas matching the hypothesis; `events_chronological` green; CI matrix runs the proptest suite |
| T1-10 | **`fw-core::math`: Replace runtime f64 LUT generation with committed Q32 const tables (Codex 2026-05-16 audit P1).** `SIGMOID_LUT` + `EXP_LUT` are currently built at process startup via `LazyLock` using `f64::exp()` then quantized to Q32. Cross-OS hash currently passes (the `lut_eval` Q32-arithmetic interpolation pins the per-tick path), but the bake step is libm/platform-dependent by design — a future libc/glibc/macOS-libsystem change in `exp()`'s ULP behavior at one of the 257 LUT points could silently drift the bake and break determinism without any code change. **This row** generates the 257-entry Q32 LUTs via a `build.rs` script (using f64 at build time only) + commits the result as `const [Q32; 257]` arrays in `fw-core::math`. Build-time bake stays platform-dependent BUT the output is a committed source byte sequence — drift becomes a code review artifact, not a silent runtime issue. Alternative: hand-author the LUT entries (load-bearing reference data; only 257 entries × 2 tables = 514 i64 literals). Either approach removes the runtime f64 path from the determinism critical chain. **Acceptance**: `SIGMOID_LUT` + `EXP_LUT` are `const`, not `LazyLock`; no `f64::exp` call in any code path the determinism-audit script scans; canonical hash UNCHANGED (the LUT values themselves don't change; only their bake mechanism). | TODO | — | T1-2b-iii-b | LUTs are `const`; `cargo expand` shows no `LazyLock<[Q32; 257]>` for math LUTs; determinism-audit script confirms no f64 in canonical path; canonical hash UNCHANGED |
| T1-11 | **`fw-match-sim`: Wire signatures into real match setup (Codex 2026-05-16 audit P1).** `tick_match` currently passes `&BTreeMap::new()` for `sig_definitions` and `MatchState::initial` creates players with empty `signature_candidates`. Signatures fire only in custom test fixtures — the normal `/dev/board` match flow can never fire a signature. **This row** adds a real match-setup context: (a) `MatchState::initial_with_content(seed, &ContentStore)` constructor variant that loads signature definitions + projects per-player `signature_candidates` from `PlayerTemplate.signature_candidates` (which T1-3 added); (b) plumbs `&BTreeMap<String, SignatureDefinition>` through `tick_match` so the actual content store's signatures flow into `dispatch::dispatch_tick`. Likely interaction with T1-7 (procgen) since real player population comes from there; this row scopes to the wiring + a smoke fixture that uses `sample-am.ron` to prove the path. (c) Smoke fixture: extend `dump_frames` binary to optionally load a real content pack, then verify that the smoke seed produces ≥1 signature firing in the dev-board scrubber within 600 ticks. | TODO | — | T1-2b-iv, T1-3 | `MatchState::initial_with_content` exists; `tick_match` accepts content-store-projected sig_definitions; smoke fixture with `sample-am.ron` content fires ≥1 signature in 600 ticks; canonical hash REBASELINED per ADR-0012 trigger #1 |
| T1-12 | **`fw-content`: Content validation hardening (Codex 2026-05-16 audit P1).** Three hardening tasks: (a) `fw-content::runtime` loaders currently silently overwrite duplicate IDs — change to fail with `ContentLoadError::DuplicateId { id, pack_id }` if two entities share the same ID within a pack OR if a mod overlay declares an `overrides:` field for an ID not actually present in the base. (b) `validators::banned_terms`, `validators::licensed_data_collision`, `validators::cliche_lookahead` currently return `Ok(())` as placeholders — change to either a real implementation (Phase-3 territory for licensed_data + cliche; Phase-1 for banned_terms which already has a Python script) OR return `Err(ContentLoadError::ValidatorNotImplemented)` with a clear deferral note so callers fail-loud rather than trust-blind. (c) `RoleId::try_new`, `SignatureId::try_new`, `SignatureCandidate::try_new` validators exist but serde bypasses them — add post-parse validation via `Deserialize` impl that calls `try_new` on parsed strings + returns the validator error if it fails. Apply to all 3 ID newtypes + SignatureCandidate. | TODO | — | T1-3 | Duplicate ID in any source/mod RON → load fails with descriptive error; unimplemented validators return Err not Ok; serde-deserialized IDs that violate the format are rejected at load time (proves with a malformed-fixture test per type) |
| T1-13 | **Frontend test scaffolding + CI gating (Codex 2026-05-16 audit P1).** CLAUDE.md §9 says `scripts/fw verify` runs `pnpm test`. It doesn't. `Justfile`'s `verify` recipe doesn't call `pnpm test`. `ci.yml` doesn't run it. And there are zero frontend test files. **This row** adds: (a) Vitest scaffold (`vitest.config.ts` if not present; `pnpm add -D vitest` if needed); (b) test files for `FrameSource` (TauriFrameSource + HttpFrameSource + frameSourceFromUrlParams), dev-board lifecycle (PixiJS app create/destroy idempotency per Frontend/RULES.md §4), and the `window.fwDev` debug surface; (c) `pnpm test` wired into `just verify` AND `.github/workflows/ci.yml`; (d) `pnpm test` runs in `<10s` so the dev loop stays tight. **Plus** the Codex backlog `cargo audit` + `cargo deny` wiring (was pulled forward in MASTER_PLAN but never wired): add to `Justfile`'s `verify` + CI as separate gate steps that don't block the dev loop but DO block CI on new vulnerabilities. | TODO | — | T1-2a | Vitest configured + ≥3 frontend test files (FrameSource + dev-board lifecycle + URL-param parsing); `just verify` runs `pnpm test` AND `cargo audit` AND `cargo deny`; CI matrix runs all three on every push; runtime `pnpm test` <10s on macOS dev box |

### T1 Exit Gate (locked)

- "Play Match" produces a sensible text recap (2-5 goals total across the 600 ticks, no NaN-tier weirdness).
- **The 2D tactical board renders the match and it visually resembles football** — a stranger watching for 30 seconds can identify formation shape, defending side, attacking side. This is the headline behavioral gate; everything else flows from it.
- **All 5 behavioral proptest invariants hold over 100 random seeds.**
- Diagnostic commentary surfaces enough position + decision context that brain-dead behavior (GK roaming midfield, defenders ignoring 1-v-1s) is spottable from text alone.
- Replay corpus has ≥2 fixtures, both pin across CI matrix.
- `cargo test --workspace` green; clippy + fmt clean.
- No `unwrap()` calls in `fw-match-sim` non-test code.
- Vertical-slice tag: `v0.1.0-first-match`.

### T1 sequencing note

The dev-tier board (T1-2a) lands BEFORE the BT runner (T1-2b-*) on purpose. The board first renders T0's stationary fixture — that proves the rendering pipeline. Then the BT runner work begins, sub-rowed (T1-2b-i ball physics → T1-2b-ii tactic FSM + cadence stagger → T1-2b-iii-a BT runner + per-role BT skeletons → T1-2b-iii-b utility math + PlayerAttributes baseline → T1-2b-iii-c BT site bindings + personality bias + utility-scored leaves → T1-2b-iii-d PlayerSeparation + visual playtest gate → T1-2b-iv signature dispatcher), and every iteration is visually verifiable in real time. Layer 3 behavioral assertions (T1-9) come last because authoring them well requires having watched matches play out first — you encode the invariants whose violation you'd notice visually.

The T1-2b → T1-2b-{i,ii,iii,iv} split was applied via Codex full-project audit Tranche 5 remediation (2026-05-13). The original single T1-2b row bundled ball physics + 22-player decision runner + steering + events + signatures + hash pinning + visual plausibility — too broad per the audit's "rows that are secretly XL" finding. Each sub-row is independently shippable + has its own canonical-hash-rebaseline opportunity.

---

## Tier 2 — League + Season (10 items)

**Goal:** a full season plays through. Many matches, league table updates, basic transfer window UI surfaces.

| ID | Class | Item | Status | ~~Effort~~ (deprecated) | Dependencies | Done Criteria |
|---|---|---|---|---|---|---|
| T2-1 | MVP | `fw-match-sim`: full BT runner with all 20-30 manager archetypes (port YAML from `MatchSim/Content/archetypes/*.yaml`). **Split-candidate:** if implementation reveals 20 archetypes is too broad for one row, sub-row by archetype-pair: T2-1a (5 archetypes — variants of the 2 T1 archetypes); T2-1b (next 8 archetypes); T2-1c (final 7 archetypes). Codex audit Lane I flagged the original as "secretly huge." Also: T2-1 is when xG/personality coefficients re-fit per `docs/design/xg-coefficients.md` + `docs/design/personality-bias-weights.md` "re-tuning cadence" sections. | TODO | — | T1-2b-iii-d | Each archetype produces visibly distinct match flow; canonical hash pinned per archetype-pair; xG mean ≈ 0.10/shot across 100-match calibration corpus |
| T2-2 | MVP | `fw-content`: 20 procedural clubs in a fantasy second-tier league (one-nation pyramid slice for season-loop testing) | TODO | — | T1-1, T1-7 | 20 clubs distinct; 380-fixture season schedule generates deterministically from a seed |
| T2-3 | MVP | `fw-content`: bake-time content-baker binary stub (`fw-cli bake`) — Claude API call → RON corpus → manifest with model-id + prompt-hash + seed. **Adopts FW v1's "validator-as-one-class" pattern** (carry-forward from `MatchSim/Content/IdentityPacketValidator.cs`, 204 LoC, single dedicated class — see `REFERENCES.md` carry-forward table): one dedicated `<Kind>Validator` type per content kind, chained checks, structured error enum. T1-1's spread-across-methods validation collapses into the dedicated form. | TODO | — | T1-1 | One end-to-end bake produces 100 player names + manifest; offline runtime sample reproduces identically; FW-VAL gauntlet runs via dedicated `PlayerTemplateValidator` / `RoleAffinityTableValidator` / `CultureValidator` / `TacticalArchetypeValidator` types; each rejects a deliberately-malformed fixture with a structured error |
| T2-4 | MVP | `fw-content`: `PlayerBio` generation — names per cultural region, basic personalities, 22-field gene model (port from `design/player-generation.md`) | TODO | — | T2-3 | 500 players generated; phenotype-label catalog covers all; ID-stability test passes round-trip |
| T2-5 | MVP | `fw-tauri`: season-controller commands — `advance_week`, `play_fixtures`, `get_standings`, `get_fixtures(club_id)` | TODO | — | T2-1, T2-2 | All four commands serialize to/from frontend; advancing a full season takes <30s on dev machine |
| T2-6 | MVP | Frontend: League page — standings table (TanStack Table with sortable columns: P / W / D / L / GF / GA / GD / Pts) | TODO | — | T2-5 | Sort by any column; dark-mode + light-mode both readable |
| T2-7 | MVP | Frontend: Squad page — player list per club (TanStack Table; columns: name / age / role / phenotype labels / contract end) | TODO | — | T2-4, T2-5 | All 20 clubs' squads viewable; phenotype labels render as text (NOT raw gene numbers — banned-terms lint catches if leaked) |
| T2-8 | MVP | Frontend: Transfer-window stub — UI shell only, no transfer mechanics; "window opens / window closes" state visible | TODO | — | T2-6 | UI shows window state per game-date; mechanic implementation deferred to T3 |
| T2-9 | MVP | `fw-save`: bincode-based save format + version-migration enum chain — first schema version locked at `1` | TODO | — | T2-5 | Save → load → byte-identical state; version `0` → `1` migration test passes |
| T2-10 | MVP | Phase-gate Codex review #1 — full T0+T1+T2 architecture review on the agent-bus (optional carry-forward; ports `scripts/agent-bus` only if review-loop is structurally in scope) | TODO | — | T2-9 | Codex posts ack on a `phase-gate-T2` topic; any blocking counters resolved or logged as deferred |

### T2 Exit Gate (locked)

- A full season plays through end-to-end on the dev machine with no panics.
- League table updates after each match-day batch.
- Save → quit → load reproduces canonical state byte-identically.
- Banned-terms lint passes on every UI surface that touched Squad / League pages.
- Vertical-slice tag: `v0.2.0-season`.

---

## Tier 3 — Career + Memory (8 items)

**Goal:** multi-season careers with event-sourced memory surfacing in player-facing copy; breakthroughs fire.

| ID | Class | Item | Status | ~~Effort~~ (deprecated) | Dependencies | Done Criteria |
|---|---|---|---|---|---|---|
| T3-1 | MVP | `fw-memory`: ledger storage + persistence — append-only `MemoryEvent` records keyed by `event_id` (port schema from `adr-0004-memory-event-schema.md`) | TODO | — | T2-9 | Append-only invariant tested; load-time migration framework in place; 1000-event ledger round-trips in <100ms |
| T3-2 | MVP | `fw-memory`: 5 readers — alumni-DB / rival-recall / promise-tracking / big-match-scars / press-fan-callbacks | TODO | — | T3-1 | Each reader has ≥3 unit tests + one integration test against a seeded multi-season ledger |
| T3-3 | MVP | `fw-content`: news headlines + manager-quote templates via Tracery-style grammars; phrase banks loaded from RON | TODO | — | T3-1, T2-3 | Slot-filling deterministic on `(career_id, event_id)` seed; banned-terms lint green |
| T3-4 | MVP | `fw-content`: breakthrough events — signature awakening + latent-flag unlock + regressive collapse triggers (port from `design/breakthrough-moments.md`) | TODO | — | T3-1 | Across a 5-season career, 1-3 breakthroughs fire per player on average; structured text recap surfaces |
| T3-5 | MVP | `fw-scouting`: scout-uncertainty model — single-scout-report variant first (Path B fallback); 3 archetypes if Phase-3 feel-prototype passes | TODO | — | T2-4 | Report data shape locked; uncertainty bands display as text labels, not numbers |
| T3-6 | MVP | Frontend: Player detail page with memory callbacks — surfaces top-5 ledger events for the player + readable phenotype block + contract block | TODO | — | T3-2, T2-7 | Callback text renders football-grade ("Scored the winner in the '98 cup final"); no template seams visible |
| T3-7 | MVP | Save migration tests — four-test-per-bump discipline (forward / callback-preservation / forward-incompat / round-trip) for version `1` → `2` (a deliberate field add) | TODO | — | T2-9, T3-1 | All four tests green; per-version fixtures committed under `fixtures/save-migration/v0001-to-v0002/` |
| T3-8 | MVP | Phase-gate Codex review #2 — multi-season-determinism + memory-ledger-integrity review | TODO | — | T3-7 | Codex posts ack on `phase-gate-T3` topic; any blocking counters resolved |

### T3 Exit Gate (locked)

- A 5-season career runs end-to-end; ledger compaction fires at the 5-season boundary.
- At least one cross-season callback surfaces in a press-conference or pre-match overlay screen.
- Save format survives one deliberate schema bump.
- Vertical-slice tag: `v0.3.0-career`.

---

## Tier 4 — Beautiful UI + Tactical Viewer (8 items)

**Goal:** polish pass. Match-day live mode reads as a finished product. Visual identity locked.

| ID | Class | Item | Status | ~~Effort~~ (deprecated) | Dependencies | Done Criteria |
|---|---|---|---|---|---|---|
| T4-1 | MVP | PixiJS tactical board — pitch + 22 dots + ball; smooth interpolation at 30Hz from the 60Hz canonical state | TODO | — | T1-2 | Visual replay of a 600-tick fixture reads as football; no jitter, no determinism leak |
| T4-2 | MVP | ECharts stat dashboards — per-player + per-team + per-season views; sortable, filterable | TODO | — | T2-6, T2-7 | Three stat views ship; data binding via Tauri IPC; no perf regressions on 5-season careers |
| T4-3 | MVP | Tailwind theming pass — lock visual identity (Anton / JetBrains Mono / Inter; muted-pitch-green + accent-yellow/red; dark-mode first-class) | TODO | — | T2-6 | Style guide doc + 3 reference screenshots committed under `docs/visual/` |
| T4-4 | MVP | Loading / empty / error states for every screen — no white-flash, no raw-error-text leaks | TODO | — | T4-2 | Manual QA pass: every route survives a Tauri-backend kill |
| T4-5 | MVP | Match-day live mode — tick-by-tick board update + commentary feed in the right rail | TODO | — | T4-1, T3-3 | Pause / play / 1× / 4× / 16× speeds; auto-sim-to-next-salience-band-N event mode works |
| T4-6 | MVP | Settings screen — accessibility (text-scale, colorblind palette toggle, reduce-motion), key rebinds, save-folder location | TODO | — | T0-2 | All settings persist via the save layer's version-migration chain |
| T4-7 | MVP | Game-shell polish — window chrome (Tauri title bar customization), main-menu, splash screen, app-icon | TODO | — | T4-3 | Looks like a finished product on a stranger's machine; app-icon ships per-OS |
| T4-8 | MVP | Phase-gate Codex review #3 — UI / accessibility / performance review | TODO | — | T4-7 | Codex posts ack on `phase-gate-T4` topic |

### T4 Exit Gate (locked)

- A stranger watching a match-day live-mode session for 3 minutes reports they understand drama + momentum + player identity without reading a design doc.
- All accessibility settings actually work, not just exist in UI.
- Vertical-slice tag: `v0.4.0-polish`.

---

## Tier 5 — Ship to Steam (8 items)

**Goal:** public Steam EA release. itch.io demo first validates the update pipeline.

| ID | Class | Item | Status | ~~Effort~~ (deprecated) | Dependencies | Done Criteria |
|---|---|---|---|---|---|---|
| T5-1 | MVP | Apple Developer enrollment ($99) + Mac code-signing via `apple-codesign` in CI | TODO | — | T4-7 | Mac DMG signs + notarizes + Gatekeeper-passes on a clean machine |
| T5-2 | MVP | Steam Direct ($100) + `steamworks-rs` integration — achievements + cloud saves + rich presence | TODO | — | T5-1 | Three test achievements unlock + cloud-save round-trip works on a second machine |
| T5-3 | MVP | Steam Deck Verified prep — 1280×800 UI sweep, controller-mappable inputs, suspend/resume safety | TODO | — | T4-6 | Steam Deck dev hardware (borrowed if needed) survives 30-minute suspend/resume; rating: Verified or Playable |
| T5-4 | MVP | Localization pipeline — i18n string-extraction + `fluent` runtime + English-only at EA but pipeline ready for translator handoff | TODO | — | T4-3 | One non-English test locale loads + renders; lint catches hardcoded strings |
| T5-5 | MVP | Performance pass — profiling with `samply` + Tracy; optimize hot paths in `fw-match-sim` + `fw-memory` readers | TODO | — | T3-2 | A full 10-season career runs in <60s on the dev machine; tactical-board live mode p95 frame time ≤16.6ms |
| T5-6 | MVP | itch.io demo release first — validate update pipeline before Steam | TODO | — | T5-4 | Two consecutive demo updates ship cleanly to itch.io; <50 external testers report no install issues |
| T5-7 | MVP | Steam EA release — store page, EA roadmap public, launch trailer cut from in-game footage | TODO | — | T5-2, T5-6 | Public EA available; first 24h crash-free rate ≥98% |
| T5-8 | MVP | Phase-gate Codex review #4 (pre-EA) — release-readiness audit, security review, content-policy + AI-disclosure audit | TODO | — | T5-7 | Codex posts ack on `phase-gate-T5` topic; no `p0` or `p1` counter remaining |

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
| CI-R1 | `cargo audit` | Start of T1 → **PULL FORWARD to T1-2b-i** (Codex audit Tranche 5) | TODO | Wire as `fw verify audit` subcommand. Trigger fired; should land alongside T1-2b-i ball physics work. |
| CI-R2 | `cargo deny` | T1 mid → **PULL FORWARD to T1-2b-i** (Codex audit Tranche 5) | TODO | Add `deny.toml`; ban GPL-class licenses in production deps. Same trigger as CI-R1. |
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

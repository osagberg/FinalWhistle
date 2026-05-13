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
| T1 | First Match | 13 (was 10; +3 from T1-2b → T1-2b-{i,ii,iii,iv} split via Codex audit Tranche 5) | Two procedural teams play one match end-to-end; 2D tactical board renders the match readably; behavioral proptest invariants hold; text recap surfaces with goals + score + key events. |
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
7. `T1-2a` dev-tier 2D tactical board for in-loop verification, then `T1-2b-i` (ball physics) → `T1-2b-ii` (tactic FSM + decision cadence stagger) → `T1-2b-iii` (FSM-of-BTs + utility selector + PlayerSeparation) → `T1-2b-iv` (signature dispatcher + 3 signatures end-to-end). The T1-2b split was applied via Codex audit Tranche 5 remediation 2026-05-13 — original single row was too broad per audit Lane I.
8. `T1-4` event emission + ledger output.
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
| T0-7b | Cross-OS canonical-hash agreement — GitHub Actions matrix `[macos-14, windows-latest, ubuntu-22.04]` runs the un-ignored `smoke_seed_60_tick_canonical_hash_pinned` test and all three platforms produce the same BLAKE3 hash. Drift on any platform = real determinism leak; investigate + fix. | TODO | — | T0-7 | All three CI jobs green on the phase PR opened by `/done`; total wall-clock ≤6 min; no `--ignored` flag needed. |
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

## Tier 1 — First Match (10 items)

**Goal:** the user can click "Play Match" between two procedural teams and get a text recap with goals, score, and key events. The developer can verify it's actually football via the 2D tactical board + behavioral assertions.

**Scope note (2026-05-13 reframe):** effort estimates ("M / L / XL / 1d / 1w") have been deprecated across this plan. They were carry-overs from the solo-developer framing that LoC-bounded the architecture choices. Per `docs/DESIGN_DOC.md` §1 "Scope ambition", implementation scope is bounded by determinism contract + maintainability + pillar promises, not by hours or lines. A row is done when its acceptance criteria pass — not when a clock-budget says so.

**Verification surface:** see `docs/design/dev-verification.md` for the three-layer dev-tier strategy (diagnostic commentary + tactical board + behavioral proptest invariants) that closes the "is this really football?" gap left by FW v1's text-only iteration.

| ID | Item | Status | ~~Effort~~ (deprecated) | Dependencies | Done Criteria |
|---|---|---|---|---|---|
| T1-1 | `fw-content` schema: `TeamTemplate` + `PlayerTemplate` + `BehaviorArchetype` (serde + RON files under `content/sources/`). `PlayerTemplate` MUST conform to **ADR-0002**'s 55-field model (38 visible + 17 hidden/support, all `Q32` in `[0,1]`, GK fields on flat struct, separate `PlayerCondition` struct, role-affinity weights in content-pack RON, FOF-style scout-range projection). Folds in Codex Imp #3 deferred from T0 (`TacticalArchetype.buildup_speed_factor: f32 → u16 bps` integer-only sampling). | DONE (2026-05-13) | — | T0-1 | `cargo test --workspace` green (65 new tests on fw-core + fw-content; pinned canonical hash UNCHANGED). 55-field player model in `fw-core::PlayerAttributes` (14/10/8/6 visible + 14/3 hidden); `AbilityCeiling` encapsulated (`pub(crate)` fields + `redraw_ceiling` breakthrough mutator); `KNOWN_ATTRIBUTE_NAMES` const enumerates all 55 for FW-VAL key validation. `fw-content::PlayerTemplate` wraps these + `schema_version: 1` + typed `RoleId` newtype; `PlayerCondition` deliberately NOT serialized into PlayerTemplate (runtime modulator state, initialized at projection time). `fw-content::RoleAffinityTable` ships with sum-to-10_000 + collect-all `invalid_roles` + `unknown_attribute_keys` validators. `TacticalArchetype.buildup_speed_factor` converted `f32 → u16 bps` with `BUILDUP_SPEED_BASELINE_BPS = 10_000` reference constant. First RON fixtures live under `content/sources/players/` + `content/sources/role-affinities/`; load tests in `crates/fw-content/tests/fixtures_load.rs`. Self-review triple ran twice — final verdict Accept across all three. |
| T1-2a | **Dev-tier 2D tactical board** (verification surface — pulled forward from T4, per ADR-0007 Layer 2; **browser-dev mode per ADR-0008**). `frontend/src/routes/Dev/TacticalBoard.tsx` consumes `MatchFrameDTO` via a `FrameSource` trait with two impls: `TauriFrameSource` (default, IPC) and `HttpFrameSource` (browser-dev, reads JSON fixture file). URL param `?source=fixture:/path.json` switches modes. Renders 22 dots + ball + tick scrubber on top-down pitch. Always-on for dev; not the shipped UI (that's T4 polish). Also lands `crates/fw-match-sim/src/bin/dump_frames.rs` — small binary that produces deterministic fixture JSON for any seed. Exposes `window.fwDev` debug surface (DEV-build only) for Claude Preview to drive the scrubber via `preview_eval`. | DONE (2026-05-13) | — | T0-2 | (1) `pnpm tauri dev` → `/dev/board` → dots render from a T0 stationary fixture via Tauri IPC; scrubber advances tick; no jank. (2) `cargo run --bin dump_frames -- --seed 0xdeadbeef --ticks 60 > frontend/public/dev-fixtures/smoke.json` produces deterministic JSON (byte-identical across reruns; gitignored ephemeral dir). (3) `pnpm --filter ./frontend dev` then `localhost:1420/dev/board?source=fixture:/dev-fixtures/smoke.json` renders the same dots from the fixture (no Tauri runtime needed; Vite serves `public/` at root). (4) Claude Preview workflow validated end-to-end: `preview_start` against the fixture URL succeeds; `preview_screenshot` produces a recognizable pitch + dots image; `preview_eval "window.fwDev.scrubTo(N)"` advances the rendered state. |
| T1-2b-i | **`fw-match-sim`: ball physics** — semi-implicit Euler in Q32 (gravity, drag, Magnus, bounce, friction). Ported from FW v1 `BallPhysics.cs`. Pinned ball-only canonical-hash sub-fixture. Codex audit Lane B + Lane I drove the T1-2b split (the original row was too broad — ball / runner / steering / events / signatures all bundled). | TODO | — | T0-4, T1-1, T1-2a | 600-tick ball-only fixture (no players) produces deterministic trajectory; canonical hash pinned cross-OS; ball-physics-specific proptest invariants (energy decay monotonic, never goes infinite, bounce coefficients in archetype range) |
| T1-2b-ii | **`fw-match-sim`: tactic FSM + decision cadence stagger** — implements `docs/specs/tactic-fsm.md` (5 states + 2 Hz heartbeat + archetype params) AND `docs/specs/decision-cadence-stagger.md` (4 Hz per-player runner with deterministic slot assignment in canonical state). No BT yet; players hold position. The plumbing for the decision cadence + tactic-state propagation. | TODO | — | T1-2b-i | `decision_slots: [u8; 22]` lives in canonical `MatchState`; canonical-hash regression test pins the new layout; slot-assignment determinism test green; tactic-FSM transition determinism proptest green; canonical hash REBASELINED per ADR-0012 trigger #1 |
| T1-2b-iii | **`fw-match-sim`: per-player FSM-of-BTs + utility selector** — implements ADR-0006 (FSM-of-BTs per outfield role; pure FSM for GK) AND ADR-0003 §1–§5 (xG / xT-LUT / pitch-control / pressing + multiplicative personality bias). BT site bindings per `docs/specs/bt-attribute-binding.md`. xG/personality coefficients per `docs/design/xg-coefficients.md` + `docs/design/personality-bias-weights.md` (Phase-1 placeholders). PlayerSeparation pass per the carry-forward debt below. | TODO | — | T1-2b-ii | 600-tick run with two `direct-pressing` archetypes; ball trajectory + player positions render on the T1-2a board and visually resemble football (manual eyeball); canonical hash pinned cross-OS; **PlayerSeparation acceptance**: (a) min-distance invariant (no two players closer than 0.4m for >2 ticks), (b) deterministic pair-iteration order (BTreeSet/Vec only — no HashMap), (c) ball position unchanged by separation, (d) velocity preservation magnitude (|v_after| ∈ [0.95·|v_before|, 1.05·|v_before|]), (e) zero-distance fallback (when two players are exactly co-located, deterministic resolution by player_id), (f) runner-order regression (separation pass runs at a documented step within tick — captured in canonical hash) |
| T1-2b-iv | **`fw-match-sim`: signature dispatcher + first 3 signatures end-to-end** — partial implementation of ADR-0011 to validate the dispatcher path. 3 representative signatures: one defensive (e.g. `BodyShieldPressure`), one attacking (e.g. `LongRangeStrike`), one build-up (e.g. `FirstTimeDiagonalSwitch`). Each implements `TriggerPredicate`, `SimBiasSnapshot`, basic `PresentationRecipe`. Cooldown state in canonical `MatchState`. Per-player `signature_candidates` schema landed in T1-3 (separate row); this row consumes it. | TODO | — | T1-2b-iii, T1-3 | 3 signatures fire in test fixtures; cooldown enforced; softmax dispatch deterministic via `SeedLayer::SignatureTrigger`; bias snapshot multiplies into utility scoring; `MemoryEvent::SignatureFirstFired` emitted; canonical hash REBASELINED (intentional) |
| T1-3 | `fw-match-sim`: signatures stub — type system only, no triggers yet (`SignatureId` + `SimBiasSnapshot` + stacking policy types per ADR-0011). **Adds `signature_candidates: Vec<SignatureCandidate>` to `fw_content::PlayerTemplate`** (carry-forward debt from FW v1's `IdentityPacket.SignatureCandidates`, deliberately deferred at T1-1 — see `REFERENCES.md` carry-forward table). Each entry pairs `SignatureId` (content-pack-qualified, `^fwh\.core(?:\.v[0-9]+)?:signature\.[a-z0-9-]+$`) with a `Q32` affinity weight in `[0, 1]`. Without it, Pillar 5 has no per-player linkage. Real triggers + dispatch in T1-2b-iv. | TODO | — | T1-2b-ii | Types compile; one no-op signature definition loads from RON without affecting hashes; `PlayerTemplate.signature_candidates` field exists + serde-round-trips + has at least one fixture entry |
| T1-4 | `fw-match-sim`: event emission — `MatchEvent` enum (Goal / Shot / Pass / KickOff / FullTime) + ledger output struct + **diagnostic commentary templates** (rich enough to spot brain-dead behavior from text alone — see dev-verification §Layer 1). | TODO | — | T1-2b-iii | Events emit in tick-order; hash includes event stream; replay reconstructs identically; commentary surfaces position + decision context per significant event |
| T1-5 | `fw-tauri`: `play_match` command returning serialized `MatchResult` (final score + event list + canonical hash) + `match_frames` streaming command feeding T1-2a board. Folds in Codex Imp #10 deferred from T0 (src-tauri consolidation — drop local placeholder commands, delegate to fw-tauri). | TODO | — | T1-4 | `pnpm tauri dev` → click Play → console shows scoreline; round-trip via Tauri IPC preserves canonical hash; src-tauri/main.rs has zero local `#[tauri::command]` impls |
| T1-6 | Frontend: Match page with "Play" button, text recap rendering (goals + minute markers), simple event-list view. Reuses T1-2a board component (debug toggle to surface it during a live match). | TODO | — | T1-5 | Stranger reads the recap and understands what happened in <60s; toggling the dev-board mid-recap shows the moment in 2D |
| T1-7 | Procedural content stub — 22 player names (Markov chain seeded by region prior) + 2 team names + 1 manager archetype RON port | TODO | — | T1-1 | Two distinct teams generated from one seed; same seed → identical names |
| T1-8 | Replay corpus fixture #1 — smoke seed, 600 ticks, two-archetype matchup, pinned canonical hash on CI matrix | TODO | — | T1-2b-iii, T1-4, T0-7 | `crates/fw-replay/fixtures/0xfeedbeefcafefade.ron` exists; CI matrix green on all three OSes |
| T1-9 | **Behavioral assertions** (verification surface — see ADR-0007 + dev-verification §Layer 3). `crates/fw-match-sim/tests/behavior_proptest.rs` with the T1 subset of ADR-0007's invariant catalogue: (a) the 4 positional invariants — GK within 30m of own goal 95%+ of ticks; team width 35-65m during in-possession; no sustained >12m/s sprint >4s; defender depth tracks tactical archetype within 8m. (b) **PlayerSeparation invariants** (Codex Lane D carry-forward from v1's `PlayerSeparation.cs`): clumping resistance — for any 100-tick window across 50 random seeds, ≤5 pairs of teammates closer than 1m for more than 30 consecutive ticks; opposing-player separation respects 0.4m floor under contest. (c) Pair-seed knob-isolation tests adopted from openfootmanager's `home_advantage_helps` pattern, for at least 3 knobs: home-advantage, press intensity, formation depth. (d) `events_chronological` proptest invariant. The remaining 5 stat-distribution assertions from ADR-0007 (goals/match, shots/match, pass completion, top-scorer concentration, card distribution) defer to T2 where season-length aggregates are observable. | TODO | — | T1-2b-iii | All 4 positional invariants hold over 100 random seeds; PlayerSeparation invariants hold over 50 random seeds; all 3 pair-seed tests produce directional deltas matching the hypothesis; `events_chronological` green; CI matrix runs the proptest suite |

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

The dev-tier board (T1-2a) lands BEFORE the BT runner (T1-2b-*) on purpose. The board first renders T0's stationary fixture — that proves the rendering pipeline. Then the BT runner work begins, sub-rowed (T1-2b-i ball physics → T1-2b-ii tactic FSM + cadence stagger → T1-2b-iii FSM-of-BTs + utility selector + PlayerSeparation → T1-2b-iv signature dispatcher), and every iteration is visually verifiable in real time. Layer 3 behavioral assertions (T1-9) come last because authoring them well requires having watched matches play out first — you encode the invariants whose violation you'd notice visually.

The T1-2b → T1-2b-{i,ii,iii,iv} split was applied via Codex full-project audit Tranche 5 remediation (2026-05-13). The original single T1-2b row bundled ball physics + 22-player decision runner + steering + events + signatures + hash pinning + visual plausibility — too broad per the audit's "rows that are secretly XL" finding. Each sub-row is independently shippable + has its own canonical-hash-rebaseline opportunity.

---

## Tier 2 — League + Season (10 items)

**Goal:** a full season plays through. Many matches, league table updates, basic transfer window UI surfaces.

| ID | Class | Item | Status | ~~Effort~~ (deprecated) | Dependencies | Done Criteria |
|---|---|---|---|---|---|---|
| T2-1 | MVP | `fw-match-sim`: full BT runner with all 20-30 manager archetypes (port YAML from `MatchSim/Content/archetypes/*.yaml`). **Split-candidate:** if implementation reveals 20 archetypes is too broad for one row, sub-row by archetype-pair: T2-1a (5 archetypes — variants of the 2 T1 archetypes); T2-1b (next 8 archetypes); T2-1c (final 7 archetypes). Codex audit Lane I flagged the original as "secretly huge." Also: T2-1 is when xG/personality coefficients re-fit per `docs/design/xg-coefficients.md` + `docs/design/personality-bias-weights.md` "re-tuning cadence" sections. | TODO | — | T1-2b-iii | Each archetype produces visibly distinct match flow; canonical hash pinned per archetype-pair; xG mean ≈ 0.10/shot across 100-match calibration corpus |
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

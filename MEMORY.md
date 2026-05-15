# Final Whistle — Working Memory

> Updated: 2026-05-15 | Phase: T1 First Match (T1-2b-iii sub-phase CLOSED + **T1-3 DONE** — fw-content gained signature schema types + per-player `signature_candidates` + one no-op fixture; canonical hash UNCHANGED. Next: T1-2b-iv signature dispatcher; final T1-2b row.)

## Project

Procedural fantasy football management sim. Rust + Tauri 2 + SolidJS. Solo dev + Claude.
Pivoted from Unity + C# v1 (preserved at git tag `v0-pre-pivot-2026-05-13` and sibling `/Users/vibelogic/dev/football-archive/`).

## Module status (post-T1-1)

| Module | State | Key file | Notes |
|---|---|---|---|
| `fw-core` | T1-1 schema lock landed | `crates/fw-core/src/player_attributes.rs` | Q32 (panic-on-overflow, Codex Q1). Durable u32 IDs (Codex Q2). Seed + Tick + cordic sqrt. **NEW post-T1-1:** `PlayerAttributes` (55-field record), `AbilityCeiling` (encapsulated + breakthrough mutator), `PlayerCondition`, `KNOWN_ATTRIBUTE_NAMES` const. CI matrix green; deterministic macOS-14 + Win + Linux. **Codex audit followups queued:** Q32Inner re-export removal (Tranche 2); AbilityCeiling::try_new validation (Tranche 2); VISIBLE_ATTRIBUTE_NAMES split (Tranche 2). |
| `fw-match-sim` | T1-2b-iii sub-phase COMPLETE | `crates/fw-match-sim/src/separation.rs` + all iii-a/b/c modules | **NEW at T1-2b-iii-d:** `separation` module with `apply_player_separation(state)` pure function + `MIN_PLAYER_DISTANCE` (0.4m) + `EPSILON_SEPARATION` (1mm) consts. Pair iteration is structurally deterministic via `for i in 0..n; for j in (i+1)..n` (no HashMap/BTreeSet needed — lex ordering built-in). Position-only adjustment via cordic sqrt; velocity untouched. Zero-distance fallback pushes lower slot -X, higher slot +X by EPSILON. `tick_match` doc-comment enumerates 6 explicit steps; step 6 = `apply_player_separation`. Manual eyeball gate PASS on 600-tick smoke fixture (`frontend/public/dev-fixtures/smoke-600t.json`). **Prior at T1-2b-iii-c:** `bt/` directory module (converted from flat `bt.rs`). `personality_bias` module with k₁..k₁₄ Q32 constants + 7 bias helpers (apply_shoot/long_pass/safe_pass/dribble/press/cover/hold) + `read_defender_pressure` PressureTolerance divisor; `DefenderPressure` + `IsProgressive` newtype wrappers prevent arg-swap bugs (Codex P2 type-design). `bt::on_ball` (7 utility functions: shoot/pass_short/pass_long/cross/dribble/hold_ball/lay_off) + `bt::off_ball` (5 functions: track_back/press/mark_player/run_off_ball/hold_formation) each reading EXACTLY the spec-mandated primary+secondary attrs per `bt-attribute-binding.md`; `*_ATTRS` const slices verified by per-site binding-correctness tests (24 total). `bt::reactive` (4 predicate functions: chase_loose_ball / foul_reaction / set_piece_switch / intercept_pass) defined but not wired into dispatch_tick — deferred to T1-2b-iii-d / T1-4. `PlayerIntent` expanded 2→19 variants (Idle / MoveToPosition + 7 on-ball + 5 off-ball + 5 GK). `subtree_library::select_outfield_intent()` builds utility-scored candidate list + picks via softmax with `SeedLayer::UtilityTieBreak` per ADR-0009; silent fallback → `unreachable!()`. `goalkeeper_fsm::evaluate_transitions` now uses real ball-position predicates (ShotStopping / SweeperKeeperRush / DistributingFromHand / InBoxPositioning); 4 of the 5 utility-bearing GK variants now reachable. `tick_match` position integration uses bare panic-on-overflow operators (Codex P0 #3). T1-2b-iii-d: PlayerSeparation + manual eyeball acceptance gate. |
| `fw-content` | T1-3 signature stub live | `crates/fw-content/src/{player.rs,role_affinity.rs,signature.rs}` | T1-1 carryover. **NEW at T1-3:** `signature.rs` module with `SignatureId` (newtype + dotted-pack-id validator per Content/RULES.md §2 — `[a-z0-9]+(?:\.[a-z0-9]+)+:signature\.[a-z0-9-]+`), `SignatureCandidate::try_new` validates affinity ∈ [0,1] (P1 fix from type-design review), 8-variant `RoleFamily`, 4-variant `BiasCategory`, `SimBiasSnapshot` (5 Q32 fields with documented 7→5 collapse rationale + NO_OP const), `CooldownPolicy` (`EveryTicks(u32)` default 600 + `PerMatchCount(u8)`), `StackingPolicy::Exclusive`, `SignatureTrigger::NoOpStub`, `SignaturePresentationRecipe` stub, `SignatureDefinition`. `PlayerTemplate.signature_candidates: Vec<SignatureCandidate>` field (`#[serde(default)]` for backwards-compat). `ContentStore` walks `content/sources/signatures/*.ron`. One no-op fixture lands; one player fixture (`sample-am.ron`) references it with affinity 0.70. Canonical hash UNCHANGED at `1db6020c…59c798` — PlayerTemplate isn't in MatchState path, schema-only addition. T1-2b-iv consumes via the dispatcher. |
| `fw-content-baker` | CLI stub | `crates/fw-content-baker/src/main.rs` | clap CLI; prompt + schema + validator modules `#![allow(dead_code)]`-staged (T2-3+). Wires to Claude API at T2-3. |
| `fw-scouting` | Empty | `crates/fw-scouting/src/lib.rs` | Compiles, no types. T3-5 begins. |
| `fw-memory` | Empty | `crates/fw-memory/src/lib.rs` | Compiles, MemoryEvent enum stub. `stakes` + `salience` Q32 (Codex Crit #3 doc fix). T3-1 fills the ledger. |
| `fw-replay` | Acceptance test live | `crates/fw-replay/tests/canonical_hash.rs` | Phase-0 acceptance test ACTIVE. Pinned BLAKE3 `d6258107…` verified cross-OS. 4 tests run (1 still ignored — insta baseline pending T1). |
| `fw-save` | Empty | `crates/fw-save/src/lib.rs` | Compiles, SaveV1 enum stub. T2-9 begins. bincode 1 vs 2 alignment (Codex Imp #12) → T2-9. |
| `fw-tauri` | Stub | `crates/fw-tauri/src/lib.rs` + `commands.rs` | Two `#[tauri::command]` handlers in sibling module (Tauri 2 `pub` bug workaround). MatchStateDto projects Q32→f64 (read-only). T1-5 wires real surface. |
| `src-tauri` | Scaffolded | `src-tauri/src/main.rs` + `build.rs` | Tauri shell. build.rs stubs frontend/dist on clean clones. Tracked icon stubs. Local placeholder commands shadow fw-tauri (Codex Imp #10 → T1-5 consolidation). |
| `frontend` | Scaffolded + green | `frontend/src/main.tsx` | SolidJS + Tailwind v3 + 6 placeholder routes. typecheck + lint + build green across CI matrix. `<For />` over `.map()` (lint pass). |

## Recent work

- 2026-05-13: **Codex full-project audit landed + Tranche 1 remediation in progress.** Audit at `docs/audits/codex-full-audit-2026-05-13.md` — ~50 findings (1 P0 + 11 headline P1 + ~30 lower). P0 (bedrock-test `#[ignore]` hole) fixed at `eb0b952e` with 3-layer guard (meta-test + CI grep + hook). Doc-drift cleanup in this commit. T1-2a is BLOCKED on Tranches 2-4 (schema follow-ups + ADR work + companion specs).
- 2026-05-13: **T1-1 closed at `69f900b9`.** ADR-0002 55-field player model in `fw-core`; `AbilityCeiling` encapsulated; `RoleId` newtype; `TacticalArchetype` f32→u16 bps. 65 tests; canonical hash UNCHANGED. Self-review triple twice → Accept. Codex audit subsequently caught additional P0/P1 issues — see audit doc.
- 2026-05-13: **Phase T0 closed.** Pivot (109 files) + blueprint reconciliation (51 files) + Codex pre-T0 audit (14 of 16 findings landed) + canonical hash pinned + CI matrix green + Codex APPROVE. See `docs/postmortems/phase-T0.md`.

## Current task

(none — T1-3 closed. `/next` picks T1-2b-iv: signature dispatcher + first 3 signatures end-to-end. Final T1-2b row.)

<details>
<summary>T1-3 task spec (closed 2026-05-15)</summary>

- **id:** T1-3
- **title:** `fw-content`: signatures stub — type system + per-player affinity field + one fixture (no triggers yet)
- **started:** 2026-05-15
- **task class:** content-narrative + sim-rust (schema lock in fw-content; ≥100 LoC; gameplay-programmer)
- **required subagent:** `gameplay-programmer` (per CLAUDE.md §5 — fw-content schema work + Rust types)
- **TDD mandate:** **EXEMPT** — data-only row mirroring T1-1's exemption (CLAUDE.md §5: schema + content-pack RON authoring). Tests still ship (serde round-trip + load) but RED-first-watch-fail per-chunk is not required. Cite exemption in commit body.
- **Canonical-hash rebaseline:** **NOT AUTHORIZED**. Per ADR-0011 + acceptance criterion #2: "one no-op signature definition loads from RON without affecting hashes." `fw-content::PlayerTemplate` is template data (consumed at procedural-generation time); the smoke seed in `MatchState::initial` uses hardcoded positions + `mid_range_baseline()`, NOT loaded PlayerTemplate. Adding `signature_candidates` to PlayerTemplate doesn't touch canonical match state. Hash stays at `blake3:1db6020c…59c798`. If the agent somehow drifts the hash, that's a scope-leak signal — STOP.

### Design references
- `docs/adr/0011-signature-system.md` — THE architectural source-of-truth. §"Mechanical shape" specifies the SignatureDefinition tuple; §"Per-player affinity" specifies SignatureCandidate; §"Stacking policy" specifies BiasCategory + StackingPolicy; §"Cooldown" specifies CooldownPolicy (default 600-tick + PerMatchCount(n) variant); §"Mod compatibility" specifies UnknownSignatureClass.
- `crates/fw-content/src/player.rs` — current PlayerTemplate shape (T1-1 baseline).
- `crates/fw-content/src/runtime.rs` (or wherever existing types live) — pattern for new schema-versioned content types.
- `Content/RULES.md` §2 — content-pack-qualified ID format. Signature IDs use the hand-authored dotted form per the T1-1 Codex audit P2 carve-out: `fwh.core:signature.<slug>` (e.g. `fwh.core:signature.no-op-stub`).
- `Content/RULES.md` §3 — `schema_version: 1` on all NEW RON-backed types.

### Acceptance criteria (verbatim from MASTER_PLAN T1-3)
1. **Types compile** — `SignatureId`, `SignatureCandidate`, `SimBiasSnapshot`, `StackingPolicy`, `CooldownPolicy`, `RoleFamily` (8-variant enum per ADR-0011 §"Catalogue size"), `BiasCategory` (4-variant per ADR-0011 §"Stacking policy"), `SignatureDefinition`, `SignatureTrigger` stub enum. All derive `Debug + Clone + PartialEq + Eq + Serialize + Deserialize` where applicable.
2. **One no-op signature definition loads from RON without affecting hashes** — fixture file at `content/sources/signatures/no-op-stub.ron` with id `fwh.core:signature.no-op-stub`. Test: load it via `ContentStore::load_sources` (or whatever fw-content uses), assert no panic + assert canonical hash on smoke seed UNCHANGED at `blake3:1db6020c…59c798`.
3. **`PlayerTemplate.signature_candidates: Vec<SignatureCandidate>` field exists + serde round-trips** — RON round-trip test: serialize → deserialize → equality. Empty vec `[]` is valid (most players have 0 candidates).
4. **At least one fixture entry** — at least one player fixture under `content/sources/players/*.ron` ships with a non-empty `signature_candidates` field referencing the no-op stub: e.g. `signature_candidates: [(signature_id: "fwh.core:signature.no-op-stub", affinity: q_frac(70, 100))]`.

### Files in scope
- `crates/fw-content/src/signature.rs` (NEW; types per ADR-0011 §"Mechanical shape" + §"Stacking policy" + §"Cooldown")
- `crates/fw-content/src/player.rs` (MODIFIED; add `signature_candidates: Vec<SignatureCandidate>` field with `#[serde(default)]` so existing fixtures still load)
- `crates/fw-content/src/lib.rs` (MODIFIED; `pub mod signature;` declaration + re-export)
- `content/sources/signatures/` (NEW directory)
- `content/sources/signatures/no-op-stub.ron` (NEW; the canonical no-op signature definition with placeholder fields)
- `content/sources/players/*.ron` (MODIFIED; at least one fixture gets `signature_candidates: [...]` field non-empty — others can stay `[]`)
- `crates/fw-content-baker/src/main.rs` (potentially MODIFIED if FW-VAL needs to know about the new signatures dir; otherwise leave alone)
- `crates/fw-content/tests/` (NEW or existing tests gain a signature_round_trip test + a load_no_op_stub_doesnt_drift_canonical_hash test)

### Files out of scope (do NOT touch — escalate if needed)
- `docs/**` — no ADR/spec/design-doc edits.
- `CLAUDE.md` / `docs/MASTER_PLAN.md` (status flip only).
- `crates/fw-match-sim/**` — TriggerFn binding lives here at T1-2b-iv; no fw-match-sim code in T1-3.
- `crates/fw-core/**` — Q32 / PlayerAttributes / SeedLayer locked.
- `crates/fw-tauri/**` / `frontend/**` — no IPC or UI changes.
- `crates/fw-replay/**` — canonical hash MUST NOT drift; if it does, that's a scope-leak signal.
- `crates/fw-match-sim/src/{ball*,tactic_fsm,decision_cadence,player,canonical,dispatch,role_states,goalkeeper_fsm,subtree_library,utility,bt,separation}.rs` — sim layer is FROZEN for this row.
- `scripts/determinism-audit.py` — no changes needed.

### Intentionally NOT done in this task
- Real trigger predicates — `SignatureTrigger` is a stub enum (the variants carry parameters but the predicate functions in fw-match-sim are T1-2b-iv).
- 24-signature catalogue — only the no-op stub is required.
- Cooldown state on `MatchState.signature_cooldowns` — T1-2b-iv (uses `BTreeMap<(PlayerId, SignatureId), Tick>` per ADR-0011 §"Cooldown"; canonical schema bump).
- Softmax dispatch — T1-2b-iv.
- `MemoryEvent::SignatureFirstFired` emission — T1-2b-iv consumes via T1-4-aligned ledger work.
- Bias snapshot propagation into BT utility scoring — T1-2b-iv (composes multiplicatively with ADR-0003 §5 personality biases).
- Counterplay cancellation predicates — T1-2b-iv stub; full counterplay defers to T2.
- `UnknownSignatureClass` mod compat — types may be declared but not exercised until mods exist.
- FW-VAL `signatures` directory validator wiring in fw-content-baker — only required if T1-3 needs it to make the load test pass; otherwise defer.

### Plan (3 chunks; data-only TDD-exempt per CLAUDE.md §5)
- [x] Chunk 1: `signature.rs` core types — `SignatureId` (newtype wrapping `String` with regex validator per ADR-0011 acceptance: `^fwh\.core(?:\.v[0-9]+)?:signature\.[a-z0-9-]+$`), `SignatureCandidate { signature_id, affinity: Q32 }`, `RoleFamily` 8-variant enum (Goalkeeper, CentreBack, FullBack, DefensiveMidfielder, CentralMidfielder, AttackingMidfielder, Winger, Striker), `BiasCategory` 4-variant enum (Attacking, Defensive, BuildUp, SetPiece), `SimBiasSnapshot { shoot_mul: Q32, pass_mul: Q32, dribble_mul: Q32, press_mul: Q32, cover_mul: Q32 }` (mirrors the 7 personality-bias surfaces but the agent picks reasonable defaults; document the field count vs ADR-0003 §5's 7 considerations), `CooldownPolicy` enum with `EveryTicks(u32)` (default 600) + `PerMatchCount(u8)` variants, `StackingPolicy::Exclusive { category: BiasCategory }`, `SignatureTrigger` stub enum (placeholder variants — agent decides cheap shape; T1-2b-iv expands), `SignatureDefinition { id, display_name, role_family, trigger, bias_snapshot, presentation: SignaturePresentationRecipe stub, cooldown, stacking, schema_version: u32 }`, `SignaturePresentationRecipe { commentary_line_bank_id: String, camera_framing_hint: String }` (stub; T2 fills the commentary line bank). All schema_versioned to 1.
- [x] Chunk 2: `PlayerTemplate.signature_candidates: Vec<SignatureCandidate>` field with `#[serde(default)]`. Modify at least one existing player RON fixture under `content/sources/players/` to include a non-empty `signature_candidates` referencing `fwh.core:signature.no-op-stub`. Other fixtures stay with `signature_candidates: []` (or omit the field entirely, relying on `#[serde(default)]`).
- [x] Chunk 3: no-op-stub.ron fixture + 3 load tests (serde round-trip + load via ContentStore + canonical-hash-unchanged at `1db6020c…59c798`). Plus 2 P1 fixes from type-design review (tightened SignatureId validator + added SignatureCandidate::try_new).

</details>

<details>
<summary>T1-2b-iii-d task spec (closed 2026-05-15)</summary>

- **id:** T1-2b-iii-d
- **title:** `fw-match-sim`: PlayerSeparation + visual playtest gate (FINAL T1-2b row before signatures)
- **started:** 2026-05-13
- **task class:** sim-rust + manual user gate (gameplay-programmer implements; user runs eyeball check)
- **required subagent:** `gameplay-programmer` for implementation; main thread for fixture generation + user-gate orchestration.
- **TDD mandate:** **YES** — sixth row under the superpowers TDD mandate. Each PlayerSeparation invariant is a proptest written RED-first.
- **Canonical-hash rebaseline:** AUTHORIZED — PlayerSeparation runs at a documented step within `tick_match`; the position adjustment captures into canonical state every tick. ADR-0012 trigger #1.
- **Adversarial test-deferral review** (lesson from iii-c BLOCK verdict): no proptests get punted in this row. The 6 invariants in the acceptance criteria ARE the test contract. Agent must ship them all as proptests; binding-correctness-style decorative-const-without-test patterns are not acceptable.

### Design references
- FW v1 carry-forward: `/Users/vibelogic/dev/football-archive/MatchSim/Sim/PlayerSeparation.cs` — design intent only; Rust idioms; do NOT port code verbatim.
- ADR-0007 dev-verification §Layer 2 (the manual eyeball gate framework).
- ADR-0012 trigger #1 (canonical schema bump for documented-step addition in tick_match).
- `docs/specs/determinism-gate.md` §Re-baselining procedure.

### Acceptance criteria (verbatim from MASTER_PLAN T1-2b-iii-d)
**PlayerSeparation acceptance** (6 falsifiable invariants from FW v1 carry-forward):
1. **Min-distance invariant** — no two players closer than 0.4m for >2 consecutive ticks.
2. **Deterministic pair-iteration order** — pair iteration uses `BTreeSet<(PlayerSlot, PlayerSlot)>` or `Vec<(PlayerSlot, PlayerSlot)>`; NO HashMap. Same input → same iteration order across reruns + platforms.
3. **Ball position unchanged** — `state.ball.pos_{x,y,z}` is byte-identical before vs after the separation pass; the pass operates on players only.
4. **Velocity preservation magnitude** — for each adjusted player, `|v_after| ∈ [0.95 · |v_before|, 1.05 · |v_before|]`. Separation can change DIRECTION but not magnitude beyond the ±5% band.
5. **Zero-distance fallback** — when two players are exactly co-located (delta = 0,0), resolve deterministically by slot ID (lower slot pushes -X, higher pushes +X by a fixed `EPSILON_SEPARATION` offset). NEVER NaN, NEVER panic, NEVER infinite-loop.
6. **Runner-order regression** — the separation pass runs at a documented step within `tick_match` (capture in canonical hash). The chosen step is AFTER `apply_intent` velocity update + position integration, BEFORE the next tick's heartbeat. Documented in `tick_match` doc-comment.

**Manual eyeball gate:**
- New fixture `frontend/public/dev-fixtures/smoke-600t.json` (600 ticks, smoke seed `0xdeadbeefdeadbeef`, generated via `cargo run -p fw-match-sim --bin dump_frames -- --seed 0xdeadbeefdeadbeef --ticks 600`).
- User opens `/dev/board?source=fixture:/dev-fixtures/smoke-600t.json`, watches the 22-dot replay scrub.
- User confirms the movement passes the "looks like football" smell test.
- User types `eyeball: PASS — <one-line observation>` in the commit body before the commit lands. **This gate is the actual PAUSE point of the row.**

**Other:**
- Canonical hash REBASELINED — `b3b0e64f… → c0b5e395… → b3b0e64f… → 235f6c5e…` history, now `235f6c5e… → <NEW>` per ADR-0012 trigger #1. Both `PINNED_60_TICK` + RON fixture `expected_hash` update atomically.

### Files in scope
- `crates/fw-match-sim/src/separation.rs` (NEW; `apply_player_separation(&mut state)` pure function + helper types + `MIN_PLAYER_DISTANCE` + `EPSILON_SEPARATION` consts)
- `crates/fw-match-sim/src/lib.rs` (MODIFIED; `tick_match` calls `apply_player_separation` at the documented step; doc-comment updated)
- `crates/fw-match-sim/tests/separation_proptest.rs` (NEW; 6 proptest invariants — one per acceptance criterion)
- `frontend/public/dev-fixtures/smoke-600t.json` (NEW; 600-tick smoke seed fixture for the dev-board playtest gate)
- `crates/fw-replay/tests/canonical_hash.rs` (MODIFIED; `PINNED_60_TICK` rebaselined)
- `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron` (MODIFIED; `expected_hash` rebaselined)

### Files out of scope (do NOT touch — escalate if needed)
- `docs/DESIGN_DOC.md` / `docs/DECISIONS.md` / `docs/adr/*.md` / `docs/specs/*.md` / `docs/design/*.md`
- `CLAUDE.md` / `docs/MASTER_PLAN.md` (status flip only)
- `crates/fw-core/**` — Q32 + math LUTs + PlayerAttributes locked.
- `crates/fw-content/**`.
- `crates/fw-tauri/**` / `frontend/src/**` (the dev-board renderer was shipped at T1-2a; just add a fixture file).
- `crates/fw-match-sim/src/utility/**` — utility math FROZEN at iii-b.
- `crates/fw-match-sim/src/{ball,ball_physics,tactic_fsm,decision_cadence,player,canonical,dispatch,role_states,goalkeeper_fsm,subtree_library}.rs` and `bt/`. Only `lib.rs` is touched for the integration point.
- `scripts/determinism-audit.py`.

### Intentionally NOT done in this task
- Reactive interrupt firing during dispatch (still T1-4).
- MatchEvent emission (T1-4).
- Signature dispatcher (T1-2b-iv).
- Content-pack RON loading of subtrees (T2-3).
- Tackling reactive site — `bt-attribute-binding.md` flags it as "T1-2b polish pass"; defer until needed.
- Real PlayerTemplate loading on PlayerState — still mid_range_baseline (T1-5/T1-6).

### Plan (5 chunks; TDD RED-GREEN-REFACTOR per chunk; NO TEST DEFERRALS per the iii-c lesson)
- [x] Chunk 1 (RED+GREEN): `separation.rs` types — `MIN_PLAYER_DISTANCE: Q32` (0.4m), `EPSILON_SEPARATION: Q32` (small jitter for zero-distance fallback, e.g. 1mm = 0.001m). Pair-iteration via `Vec<(PlayerSlot, PlayerSlot)>` built by `(0..22).flat_map(|i| (i+1..22).map(move |j| (i, j)))` — structurally deterministic, NO BTreeSet needed since (i,j) with i<j is unique. Tests: pair-iteration produces 22*21/2 = 231 pairs in lex order; (slot_a, slot_b) always has slot_a < slot_b.
- [x] Chunk 2 (RED+GREEN): `apply_player_separation(state: &mut MatchState)` — for each pair (i, j), compute delta = `players[j].pos - players[i].pos`, dist² = `delta.dot(delta)`. If dist² > MIN_DISTANCE², skip (no adjustment). Else if dist² == 0, apply zero-distance fallback (lower slot pushes -X by EPSILON, higher pushes +X). Else compute unit-vector via `cordic::sqrt`, push both players apart by half the overlap each. Tests: two players at distance 0.3 get pushed to ≥ 0.4 apart; two players at distance 0.5 untouched; two players at (0,0) and (0,0) resolve to (-EPSILON, 0) and (+EPSILON, 0) deterministically by slot.
- [x] Chunk 3 (RED+GREEN): velocity preservation. After position adjustment, the velocity vector keeps its MAGNITUDE but may rotate. Approach: rescale vel to preserve `|v|` exactly... actually the spec says ±5% band is OK. Simplest implementation: don't touch velocity at all — adjustment is position-only. Then invariant 4 is `|v_after| == |v_before|` trivially. Verify this satisfies the spec; if so, ship and document.
- [x] Chunk 4 (RED+GREEN): wire into `tick_match` at documented step. After ball physics + heartbeat + dispatch (apply_intent + position integration), BEFORE state.tick = state.tick.successor(). Wait — tick advance is currently at the TOP of tick_match. So position integration happens during this tick's body, and separation should run at the end. Update `tick_match` doc-comment to enumerate the steps explicitly:
  1. Increment tick.
  2. Ball physics.
  3. 2 Hz heartbeat (every 30 ticks).
  4. Per-player dispatch (`dispatch_tick`).
  5. Per-player position integration.
  6. **PlayerSeparation pass (T1-2b-iii-d).**
- [x] Chunk 5 (RED+GREEN + REBASELINE): 6 proptest invariants in `tests/separation_proptest.rs`. Each maps 1:1 to an acceptance criterion. Then regenerate the 600-tick fixture via `cargo run --release -p fw-match-sim --bin dump_frames -- --seed 0xdeadbeefdeadbeef --ticks 600 > frontend/public/dev-fixtures/smoke-600t.json`. Rebaseline canonical hash. Commit-body marker: `canonical hash: REBASELINED (trigger: #1 — PlayerSeparation pass added to tick_match at documented step 6; old: blake3:235f6c5e…181288d; new: blake3:<NEW>; reason: T1-2b-iii-d PlayerSeparation)`.

### Eyeball gate (PASS 2026-05-15)
- All 7 unit tests + 7 proptest invariants green; 6 MEMORY acceptance criteria have dedicated coverage.
- 600-tick fixture at `frontend/public/dev-fixtures/smoke-600t.json` (2.3MB, 601 frames; regenerated cleanly with `2>/dev/null` after initial cargo-stderr-into-stdout leak).
- User eyeball verdict: **PASS** — "players start overlapped at 4-3-3 spawn positions but fan out within a few ticks; some back-and-forth movement after separation resolves (expected skeletal-BT oscillation; iii-d's separation invariant holds)."

</details>

<details>
<summary>T1-2b-iii-c task spec (closed 2026-05-13)</summary>

- **id:** T1-2b-iii-c
- **title:** BT site bindings + personality bias + utility-scored leaves (make the BTs actually decide)
- **started:** 2026-05-13
- **task class:** sim-rust (wires math primitives from iii-b into BT runner from iii-a; canonical-state behavior change; gameplay-programmer required)
- **required subagent:** `gameplay-programmer`
- **TDD mandate:** **YES** — fifth row under the superpowers TDD mandate.
- **Canonical-hash rebaseline:** AUTHORIZED — real utility outputs now flow into BT-selected actions which mutate player velocity non-trivially. The smoke-seed canonical state at tick 60 will differ from iii-b's stub `MoveToFormationPosition` outputs. ADR-0012 trigger #1 (canonical schema bump for new PlayerIntent variants + new utility-scored behavior). Both `PINNED_60_TICK` and the RON fixture `expected_hash` update atomically.

### Design references
- `docs/adr/0003-decision-utility-math.md` §5 (personality bias — multiplicative per consideration).
- `docs/design/personality-bias-weights.md` — the 7-consideration × 14-bias mapping table with Phase-1 seed values (k₁..k₁₄ in [0.25, 0.45]) + PressureTolerance divisor (0.75 coefficient).
- `docs/specs/bt-attribute-binding.md` — THE authoritative table of which PlayerAttributes fields each of 21 BT sites reads. Read top to bottom: 7 on-ball + 5 off-ball + 4 reactive-interrupt + 5 GK-specific. Note the field-path conventions section (work_rate is on PersonalityVector NOT MentalAttributes; marking is on TechnicalAttributes; goalkeeper.positioning doesn't exist — use mental.positioning).
- `docs/adr/0006-bt-vs-fsm-decision-layer.md` — outfield uses FSM-of-BTs; GK uses pure FSM. The 21 BT sites split: 12 outfield decision sites (on-ball + off-ball) + 4 reactive-interrupt predicates + 5 GK-specific sites.

### Acceptance criteria
1. **All 21 BT sites consume documented `PlayerAttributes` per spec** — every site reads ONLY the attributes named in its binding (proptest enforced).
2. **Personality bias matrix applied at every decision site** — 7 named bias helpers (`apply_shoot_bias`, `apply_long_pass_bias`, `apply_safe_pass_bias`, `apply_dribble_bias`, `apply_press_bias`, `apply_cover_bias`, `apply_hold_bias`) implementing the multiplicative `utility · (1 + k_i · bias)` form per `personality-bias-weights.md`.
3. **`PressureTolerance` divisor** — `read_defender_pressure(player, raw)` returns `raw / (1 + 0.75 · PT)` per `personality-bias-weights.md` §"PressureTolerance — the one bias amplifier".
4. **Expanded `PlayerIntent` enum** — new variants for each on-ball + off-ball + GK action class (e.g. `AttemptShot { target_x, target_y, utility }`, `AttemptPass { receiver_slot, target_x, target_y, utility }`, `Dribble { direction_x, direction_y }`, `Press { target_slot }`, `MarkPlayer { target_slot }`, `TrackBack { target_x, target_y }`, `HoldFormation`, `RunOffBall { target_x, target_y }`, `LayOff { receiver_slot }`, `GkShotStop { aim_x, aim_y }`, `GkSweeperRush { target_x, target_y }`, `GkDistribute { receiver_slot, target_x, target_y }`).
5. **Stub `MoveToFormationPosition` leaves replaced** with utility-scored decisions across all role-state subtrees in `subtree_library`. Each role-state's BT now scores N candidate actions, picks one via `pick_top_n_softmax`.
6. **Reactive interrupt predicates exist as pure functions** — 4 predicates per spec; tested in isolation. Full reactive-firing path (which would consume the `interrupt_cooldown_until` field from T1-2b-ii) is **defer**red to a later row; iii-c only ships the predicate functions.
7. **GK FSM utility-scored** — the 5 GK decision sites land in `goalkeeper_fsm.rs` as utility functions; each GK state's tick function now picks among its candidate actions via softmax.
8. **Canonical hash REBASELINED** per ADR-0012 trigger #1. Old `blake3:b3b0e64f…d4da1169` → new BLAKE3 (TBD; players now make real decisions at every 4 Hz cadence tick).
9. **insta snapshot at tick 60 produces visibly differentiated player behavior** — different roster slots produce different velocity outputs (formation-relative) reflecting their roles + attributes. Smoke test pins this differentiation.
10. **No off-binding reads** — every `player.attributes.<field>` read inside the new BT site functions corresponds to a binding row in `bt-attribute-binding.md`. Manual review enforced via proptest #1; future lint deferred.

### Files in scope
- `crates/fw-match-sim/src/bt/personality_bias.rs` (NEW; k₁..k₁₄ constants + 7 bias helpers + PT divisor)
- `crates/fw-match-sim/src/bt/on_ball.rs` (NEW; 7 utility functions: shoot_utility, pass_short_utility, pass_long_utility, cross_utility, dribble_utility, hold_utility, layoff_utility)
- `crates/fw-match-sim/src/bt/off_ball.rs` (NEW; 5 utility functions: defensive_cover_utility, press_utility, mark_utility, running_off_ball_utility, hold_formation_utility)
- `crates/fw-match-sim/src/bt/reactive.rs` (NEW; 4 predicate functions: ball_reached_defensive_third, shot_incoming, marker_arrived, through_ball_intercept)
- `crates/fw-match-sim/src/bt/mod.rs` (MODIFIED; add mod declarations for the 4 new submodules)
- `crates/fw-match-sim/src/role_states.rs` (MODIFIED; expand `PlayerIntent` enum with new variants)
- `crates/fw-match-sim/src/goalkeeper_fsm.rs` (MODIFIED; 5 GK utility functions replace stub PlayerIntent::MoveToPosition output)
- `crates/fw-match-sim/src/subtree_library.rs` (MODIFIED; per-role-state subtrees now build a candidate list + pick via softmax instead of returning MoveToFormationPosition)
- `crates/fw-match-sim/src/dispatch.rs` (MODIFIED; apply_intent extends to translate new PlayerIntent variants to vel; `local_decision_counter` continues to increment for RNG site)
- `crates/fw-match-sim/tests/utility_site_proptest.rs` (NEW; binding-correctness proptest + bias-monotonicity proptests + per-site determinism)
- `crates/fw-replay/tests/canonical_hash.rs` (MODIFIED; PINNED_60_TICK rebaselined)
- `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron` (MODIFIED; expected_hash rebaselined)

### Files out of scope (do NOT touch — escalate if needed)
- `docs/DESIGN_DOC.md` / `docs/DECISIONS.md` / `docs/adr/*.md` / `docs/specs/*.md` / `docs/design/*.md`
- `CLAUDE.md` / `docs/MASTER_PLAN.md` (status flip only)
- `crates/fw-core/**` — q32 + math LUTs locked; PlayerAttributes locked. iii-c CONSUMES fw-core; no additions.
- `crates/fw-content/**` (BT subtrees as content-pack data defers to T2-3; iii-c uses hardcoded Rust subtrees via subtree_library)
- `crates/fw-tauri/**` / `frontend/**` (DTO + UI for new PlayerIntent variants defers)
- `crates/fw-match-sim/src/utility/**` — math primitives from iii-b are FROZEN; iii-c only READS them (xg_utility, xt_delta, pitch_control, pressing_intensity, pick_top_n_softmax).
- `crates/fw-match-sim/src/ball*.rs` / `tactic_fsm.rs` / `decision_cadence.rs` / `bt.rs` / `player.rs` / `canonical.rs` (prior rows; only the named "MODIFIED" files in scope above)
- `scripts/determinism-audit.py` — fully tightened at iii-b; no changes here

### Intentionally NOT done in this task
- **Real reactive-interrupt firing during dispatch** — iii-c ships the 4 reactive predicates as pure functions; the `interrupt_cooldown_until` consumer wiring (which would update cooldowns + suppress next 4 Hz decision) defers to a separate row (T1-2b-iv polish OR T1-4 when MatchEvent provides the timing).
- **Content-pack RON loading of subtrees** — subtree_library remains hardcoded Rust. Real content-pack loading defers to T2-3.
- **Universal pre-emption hooks** — `preempt_check` stub continues to return None. ADR-0006 universal pre-emption (single-chaser claim, foul reaction, set-piece switchover) defers to T1-4 / T1-2b-iv when MatchEvent + reactive integration are needed.
- **Real PlayerAttributes loading from content packs** — players keep `mid_range_baseline()` defaults from iii-b. Real PlayerTemplate loading defers to T1-5/T1-6.
- **MatchEvent emission from BT decisions** — `AttemptShot` doesn't emit `MatchEvent::Goal` yet. T1-4.
- **Signature dispatcher** — T1-2b-iv. iii-c does NOT consume `SimBiasSnapshot` (signatures aren't firing yet).
- **xT-LUT bake-time pipeline** — iii-c reads `xt_delta` against the hardcoded const from iii-b. Real bake via fw-content-baker defers to T2-3.
- **Manual eyeball acceptance gate** — that's iii-d. iii-c focuses on correctness of the wiring + bias application via tests.

### Plan (6 chunks; TDD RED-GREEN-REFACTOR per chunk)
- [x] Chunk 1 (RED+GREEN): `bt/personality_bias.rs` module. `pub const K_SHOOT_FLAIR: Q32` ... `pub const K_HOLD_PT: Q32` (14 constants per `personality-bias-weights.md` 7×8 mapping table). 7 named bias helpers (`apply_shoot_bias`, `apply_long_pass_bias`, `apply_safe_pass_bias`, `apply_dribble_bias`, `apply_press_bias`, `apply_cover_bias`, `apply_hold_bias`) returning `Q32`. `read_defender_pressure(player, raw)` applies PT divisor (0.75 coefficient). Tests: k values pinned; each helper at boundary (bias=0 → identity, bias=1 → expected swing); PT divisor at PT=0 → raw, PT=1 → ~57% of raw.
- [x] Chunk 2 (RED+GREEN): `PlayerIntent` enum expansion in `role_states.rs`. ~12 new variants per the acceptance criteria. Tests: serde round-trip for each variant; pattern-match exhaustiveness check.
- [x] Chunk 3 (RED+GREEN): on-ball + off-ball utility sites — `bt/on_ball.rs` (7 functions) + `bt/off_ball.rs` (5 functions). Each function takes `(&PlayerState, &MatchState, &ShotContext-equivalent or similar)` and returns `Q32` utility. Each calls the relevant math primitive from iii-b's `utility` module + applies the personality bias helper from Chunk 1 + reads PlayerAttributes per the spec's binding table. Tests: per site, binding-correctness (assert only documented attrs are read — proptest walks the binding table); per site, bias-monotonicity (max-FlairBias player has higher shoot utility than min-FlairBias all else equal); per site, output in unit range or expected range; per site, determinism (same input → same output).
- [x] Chunk 4 (RED+GREEN): reactive interrupt predicates — `bt/reactive.rs` (4 functions). Each predicate takes `(&PlayerState, &MatchState)` and returns `bool`. Each reads ONLY the documented attrs per spec's "Reactive interrupt predicates" section. Tests: per predicate, fires when canonical conditions met; doesn't fire otherwise; field-path correctness.
- [x] Chunk 5 (RED+GREEN): GK utility sites — modify `goalkeeper_fsm.rs` so each of the 5 GK states (ShotStopping, CrossCollection, SweeperKeeperRush, DistributionShort, DistributionLong) calls a utility function per spec's "Goalkeeper-specific decision sites" section + applies relevant bias (Composure for shot-stopping; Aggression for sweeper-rush; etc.). Each utility function reads ONLY the documented GK attrs + composes via softmax pick where multiple candidates exist (e.g. distribution short vs long picked via softmax in the GK FSM transition predicate). Tests: GK shot-stopping with high vs low reflexes produces different intent; sweeper rush with high vs low Aggression triggers differently.
- [x] Chunk 6 (REBASELINED): utility-scored leaves wired into subtree_library; apply_intent extended for 19 PlayerIntent variants; canonical hash rebaselined twice (first to `c392bac5…14c7f7d2` then to `235f6c5e…181288d` after the P0/P1 self-review fix pass). The fix pass closed 4 P0s + 4 P1s + 1 P2 in-place: P0-1 binding-correctness drift across ~10/12 utility sites (all 12 utility functions rewritten to align with `bt-attribute-binding.md` spec; 24 binding-correctness tests added consuming the `*_ATTRS` constants that were previously decorative); P0-2 GK FSM frozen at InBoxPositioning fixed (real ball-position-based transitions added for ShotStopping / SweeperKeeperRush / DistributingFromHand; 10 GK predicate tests added); P0-3 `checked_mul/add` recidivism in lib.rs position integration replaced with bare panic-on-overflow operators; P0-4 `xt_delta` silently zeroing negative deltas fixed via bare subtraction (iii-b carryover scope expansion); P1-1 `debug_assert!` on bias inputs in all 7 helpers; P1-2 `select_outfield_intent` silent fallback → `unreachable!()`; P1-3 binding-correctness tests shipped (load-bearing for P0-1); P1-4 RNG seed-layer corrected from `SeedLayer::Decision` to `SeedLayer::UtilityTieBreak` per ADR-0009; P2-2 `DefenderPressure` + `IsProgressive` newtypes prevent bias-helper arg-swap bugs.

</details>

<details>
<summary>T1-2b-iii-b task spec (closed 2026-05-13)</summary>

- **id:** T1-2b-iii-b
- **title:** utility math primitives + `PlayerAttributes` baseline (pure-function Q32 math; no BT wiring yet)
- **started:** 2026-05-13
- **task class:** sim-rust (math primitives in fw-core + fw-match-sim; canonical-state schema bump for PlayerAttributes; gameplay-programmer required)
- **required subagent:** `gameplay-programmer`
- **TDD mandate:** **YES** — fourth row under the superpowers TDD mandate.
- **Canonical-hash rebaseline:** AUTHORIZED — `PlayerState` gains the 55-field `PlayerAttributes` struct from `fw-core` (ADR-0002 player model); +440 bytes per player × 22 = +9680 bytes per match-state encoding. ADR-0012 trigger #1 (canonical schema bump). Both `PINNED_60_TICK` and the RON fixture `expected_hash` update atomically.
- **Cross-crate scope expansion AUTHORIZED:** new `fw-core::math` module (sigmoid_q32 + exp_q32 LUTs) per ADR-0003 §1 + §6. fw-core was locked at T0 but this addition is load-bearing per the ADR — sibling crates (fw-scouting, fw-memory) will consume the same math primitives once they land. User approved 2026-05-13 alongside the T1-2b-iii further-split.

### Design references
- `docs/adr/0003-decision-utility-math.md` — THE source-of-truth. Six closed-form Q32 primitives. Read §1 (xG), §2 (xT), §3 (pitch-control), §4 (pressing), §6 (top-N softmax). §5 (personality bias) DEFERS to -iii-c — math primitives only here.
- `docs/design/xg-coefficients.md` — Phase-1 third-pass β values (β₀=-5.50 / β₁=+4.80 / β₂=+1.80 / β₃=-3.00 / β₄=+0.45 / β₅=+0.55 / β₆=+0.50) + 6 feature normalization ranges + 3 canonical sanity-check reference points (30m long shot → 0.02-0.04; 12-yard central → 0.25-0.35; penalty → 0.76).
- `docs/adr/0009-rng-seed-derivation.md` — `SeedLayer::UtilityTieBreak` is the lane for softmax draws.
- `docs/adr/0012-hash-rebaseline-policy.md` — trigger #1 (canonical schema bump).
- `crates/fw-core/src/player_attributes.rs` — `PlayerAttributes` 55-field struct (already locked at T1-1). iii-b adds a `mid_range_baseline()` constructor that sets every field to Q32 ≈ 0.5.

### Acceptance criteria
1. **`fw-core::math` module exists** with `sigmoid_q32(x: Q32) -> Q32` and `exp_q32(x: Q32) -> Q32` — both 257-entry symmetric Q32 LUTs over `[-8, +8]` per ADR-0003 §1 + §6. Linear interpolation; saturates outside the range.
2. **`fw-match-sim::utility::xg` module** with `xg_utility(ctx: &ShotContext) -> Q32` — 6-feature logistic per `xg-coefficients.md` third-pass β values. Sanity-check tests against the 3 canonical reference points pass (30m → 0.02–0.04; 12-yard central → 0.25–0.35; penalty → 0.76 ± 0.03).
3. **`fw-match-sim::utility::xt` module** with `XT_GRID: [Q32; 192]` const (hand-authored 16×12 grid; football-shaped — high at attacking goalmouth ~0.85, near-zero at own-goalmouth, smooth gradient) + `xt_delta(src: PitchZone, dst: PitchZone) -> Q32` + `PitchZone` newtype with `flat_index() -> usize`. Doc comment on XT_GRID notes the placeholder authorship; T2-1 owes a dedicated `xt-resolution.md` design doc + bake-time pipeline.
4. **`fw-match-sim::utility::pitch_control` module** with `pitch_control(point, attackers, defenders) -> PitchControlOutcome` per ADR-0003 §3 — Spearman closed-form per-point. Uses `cordic::acos` for angular penalty + `sigmoid_q32`. Returns `attacker_control + defender_control + neutral = 1` (proptest invariant).
5. **`fw-match-sim::utility::pressing` module** with `pressing_intensity(carrier, defenders) -> Q32` — product form per ADR-0003 §4 / Bauer 2025. Returns Q32 in `[0, 1]` (proptest invariant).
6. **`fw-match-sim::utility::softmax` module** with `pick_top_n_softmax(candidates, rng, temperature) -> ActionId` — top-3 softmax per ADR-0003 §6. Uses `exp_q32` + `seed_fn(match_seed, tick, SeedLayer::UtilityTieBreak, decision_id)` per ADR-0009. Determinism proptest (same seed → same pick over 100 candidate sets).
7. **`PlayerAttributes` baseline on `PlayerState`** — `PlayerState` gains `attributes: PlayerAttributes` field, initialized via `PlayerAttributes::mid_range_baseline()` (new constructor in fw-core that sets every field to Q32 ≈ 0.5). Real content-pack loading defers to T1-5/T1-6.
8. **Canonical encoder extended** to emit all 55 PlayerAttributes Q32 fields per player. Wire-format diagram updated. VERSION bumped 3→4.
9. **Canonical hash REBASELINED** per ADR-0012 trigger #1. Old `blake3:c0b5e395…c1430ff` → new BLAKE3 (TBD).
10. **NO BT wiring** — `MoveToFormationPosition` leaves still the only thing dispatch produces. The math modules exist as pure functions; iii-c wires them into BT site bindings.

### Files in scope
- `crates/fw-core/src/math.rs` (NEW; sigmoid_q32 + exp_q32 LUTs + tests)
- `crates/fw-core/src/lib.rs` (MODIFIED; add `pub mod math;`)
- `crates/fw-core/src/player_attributes.rs` (MODIFIED; add `mid_range_baseline()` constructor)
- `crates/fw-match-sim/src/utility/mod.rs` (NEW; mod declarations)
- `crates/fw-match-sim/src/utility/xg.rs` (NEW; xg_utility + ShotContext)
- `crates/fw-match-sim/src/utility/xt.rs` (NEW; XT_GRID const + xt_delta + PitchZone)
- `crates/fw-match-sim/src/utility/pitch_control.rs` (NEW)
- `crates/fw-match-sim/src/utility/pressing.rs` (NEW)
- `crates/fw-match-sim/src/utility/softmax.rs` (NEW; pick_top_n_softmax)
- `crates/fw-match-sim/src/lib.rs` (MODIFIED; add `pub mod utility;`; `MatchState::initial` populates attributes via mid_range_baseline; encoder VERSION bump propagation)
- `crates/fw-match-sim/src/player.rs` (MODIFIED; PlayerState gains attributes field)
- `crates/fw-match-sim/src/canonical.rs` (MODIFIED; encode_player extended for attributes; VERSION 3→4; wire-format diagram updated)
- `crates/fw-match-sim/tests/utility_proptest.rs` (NEW; xG-mean-realistic / pitch-control-sums-to-1 / pressing-in-range / softmax-determinism invariants)
- `crates/fw-core/tests/math_proptest.rs` (NEW; sigmoid monotonicity + exp positive + LUT-edge-clamp invariants)
- `crates/fw-replay/tests/canonical_hash.rs` (MODIFIED; PINNED_60_TICK rebaselined)
- `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron` (MODIFIED; expected_hash rebaselined)

### Files out of scope (do NOT touch — escalate if needed)
- `docs/DESIGN_DOC.md` / `docs/DECISIONS.md` / `docs/adr/*.md` / `docs/specs/*.md` / `docs/design/*.md` (source-of-truth; no spec mutation during impl. xt-resolution.md design doc is owed at T2-1, NOT here)
- `CLAUDE.md` / `docs/MASTER_PLAN.md` (status flip only)
- `crates/fw-content/**` (BT site bindings consume content; defers to iii-c. xT bake-time pipeline defers to T2-3)
- `content/sources/**` / `content/baked/**`
- `crates/fw-tauri/**` / `frontend/**` (DTO + UI for new attributes defers)
- `crates/fw-match-sim/src/bt.rs` / `dispatch.rs` / `subtree_library.rs` / `goalkeeper_fsm.rs` / `role_states.rs` (iii-a is closed; iii-c wires utility into these; iii-b only ADDS the utility module)
- `crates/fw-match-sim/src/ball*.rs` / `tactic_fsm.rs` / `decision_cadence.rs` (prior rows)

### Intentionally NOT done in this task
- **Personality bias matrix application** (ADR-0003 §5 7-consideration × 14-element table) — that's -iii-c.
- **BT site bindings to PlayerAttributes** (the 21 sites per `bt-attribute-binding.md`) — that's -iii-c. iii-b only ADDS attributes to PlayerState; no BT leaf reads them yet.
- **Wiring utility scoring into BT leaves** — `dispatch_tick` continues to produce `MoveToFormationPosition` only. iii-c replaces the stub leaves with `AttemptShot` / `AttemptPass` / etc.
- **Real PlayerAttributes loading from content packs** (PlayerTemplate from fw-content) — defers to T1-5/T1-6. Phase-1 uses `mid_range_baseline()` placeholder.
- **xT bake-time pipeline** (content/sources/xt/transitions.ron + fw-content-baker xt subcommand) — defers to T2-3. iii-b hand-authors XT_GRID directly as a const.
- **xt-resolution.md design doc** — owed at T2-1 when xG/xT coefficients re-fit per ADR-0003 §References. iii-b just inline-documents the placeholder shape.
- **`pitch_control_field` (full-grid 16×12 evaluation)** — deferred Tauri-callable per ADR-0003 §3. iii-b only ships `pitch_control` (per-point).
- **Universal pre-emption hooks** — preempt_check stub stays a no-op.

### Plan (6 chunks; TDD RED-GREEN-REFACTOR per chunk)
- [x] Chunk 1 (RED+GREEN): `fw-core::math` LUTs. 257-entry symmetric `[-8, +8]` Q32 LUTs for sigmoid + exp. Linear interpolation. Saturates outside range. Tests: monotonicity (sigmoid strictly increasing); bit-exact LUT values at known anchor points (sigmoid(0)=0.5, sigmoid(-8)≈0.000335, sigmoid(8)≈0.999665); exp positivity; interpolation correctness.
- [x] Chunk 2 (RED+GREEN): `fw-match-sim::utility::xg`. `ShotContext` struct + `xg_utility()` with the 6 features per `xg-coefficients.md`. Third-pass β values. Tests: 3 canonical sanity checks (30m long shot, 12-yard central, penalty) all hit their expected football-empirical ranges.
- [x] Chunk 3 (RED+GREEN): `fw-match-sim::utility::xt`. `PitchZone` newtype (16×12 grid index) + hand-authored `XT_GRID: [Q32; 192]` const + `xt_delta()`. Tests: progressive forward pass yields positive xt_delta; backward pass yields negative; same zone yields zero; entries are bounded in `[0, 1]`. Doc comment on XT_GRID notes "T2-1 owes `xt-resolution.md` + bake-time pipeline; this is hand-authored Phase-1 seed."
- [x] Chunk 4 (RED+GREEN): `fw-match-sim::utility::pitch_control` + `pressing`. Spearman per-point time-to-intercept (cordic acos + sigmoid_q32). Pressing as product form. Tests: pitch-control sums to 1 over a query point (attacker + defender + neutral); pressing in `[0, 1]`; pressing-monotonic-in-number-of-nearby-defenders.
- [x] Chunk 5 (RED+GREEN): `fw-match-sim::utility::softmax`. `pick_top_n_softmax()` using exp_q32 + ChaCha8Rng seeded via `seed_fn(match_seed, tick, SeedLayer::UtilityTieBreak, decision_id)`. Tests: determinism (same seed → same pick); temperature→0 → argmax; uniform candidates → reasonable distribution over 1000 draws.
- [x] Chunk 6 (REBASELINED): PlayerAttributes::mid_range_baseline + PlayerState attributes field + encoder extension. VERSION 3→4. Canonical hash rebaselined to `blake3:b3b0e64f…d4da1169`. Hash stayed stable through self-review fix pass (pure-Q32 lut_eval produces bit-identical results to the f64 path; LUT entries baked once in f64 then stored as Q32).

</details>

<details>
<summary>T1-2b-iii-a task spec (closed 2026-05-13)</summary>

- **id:** T1-2b-iii-a
- **title:** `fw-match-sim`: BT runner + per-role BT skeletons (FSM-of-BTs skeleton tier per ADR-0006)
- **started:** 2026-05-13
- **task class:** sim-rust (canonical-state extension + per-player decision dispatch; ≥100 LoC; gameplay-programmer required)
- **required subagent:** `gameplay-programmer`
- **TDD mandate:** **YES** — third row under the superpowers TDD mandate (per `docs/DECISIONS.md` 2026-05-13). RED-GREEN-REFACTOR per chunk.
- **Canonical-hash rebaseline:** AUTHORIZED in this task-spec — adding per-player BT state + `local_decision_counter` to `MatchState` is a canonical schema bump per ADR-0012 trigger #1. Both `PINNED_60_TICK` and the RON fixture `expected_hash` update atomically.

### Design references
- `docs/adr/0006-bt-vs-fsm-decision-layer.md` — the architectural source-of-truth. FSM-of-BTs for outfield (Defender / Midfielder / Forward); pure FSM for Goalkeeper. Nodes are code; trees are content-pack data.
- `docs/specs/bt-attribute-binding.md` — 21 BT sites grouped (7 on-ball + 5 off-ball + 4 reactive-interrupt + 5 GK-specific). T1-2b-iii-a wires the SCAFFOLDING; -iii-b wires the actual attribute reads + utility scoring.
- `docs/specs/decision-cadence-stagger.md` — `should_decide()` predicate from T1-2b-ii is the dispatch trigger.
- `docs/adr/0009-rng-seed-derivation.md` — per-player BT draws use `seed_fn(match_seed, tick, SeedLayer::Decision, (player_id << 16) | local_decision_counter)`.
- `docs/adr/0012-hash-rebaseline-policy.md` — trigger #1 (canonical schema bump).

### Acceptance criteria (from MASTER_PLAN T1-2b-iii-a row)
1. **BT runner compiles + runs** — `Tree` / `Node` / `NodeStatus` / `Selector` / `Sequence` / `Decorator` / `Leaf` / `Condition` types with deterministic tree traversal.
2. **10 outfield BT skeletons + GK FSM compile** — every outfield role-state has a stub BT that traverses to a `MoveToFormationPosition` leaf returning `NodeStatus::Success`; GK FSM has stub Rust functions per state returning a stub `PlayerIntent`.
3. **Per-player BT state encoded canonically** — `PlayerState` gains `role: Role` + `role_state: u8` (per-role state enum tag) + `local_decision_counter: u32`. Canonical encoder extended; wire-format diagram updated.
4. **`tick_match` iterates 22 roster slots calling `should_decide`** — per ADR-0006 `dispatch_tick`. Pre-emption hooks stubbed (returns None). Role-specific tick dispatches to GK FSM or outfield BT runner. `apply_intent` mutates player vel based on returned `PlayerIntent`.
5. **`local_decision_counter` increments deterministically** — increments per BT decision invocation; resets at match-init to 0. Tested via fixture that fires N decisions + asserts counter == N for that player.
6. **Canonical hash REBASELINED** per ADR-0012 trigger #1; both `PINNED_60_TICK` + RON fixture updated atomically.
7. **insta snapshot of canonical state at tick 60 matches across reruns** — intra-process determinism witness.
8. **BT traversal proptest** — any seeded run produces a deterministic trace (same seed → same node-visit order → same PlayerIntent outputs across reruns).

### Files in scope
- `crates/fw-match-sim/src/bt.rs` (NEW; BT runner core types — `Tree`, `Node`, `NodeStatus`, `Selector`, `Sequence`, `Decorator`, `Leaf`, `Condition`)
- `crates/fw-match-sim/src/role_states.rs` (NEW; `Role` enum + per-role state enums — `GoalkeeperState`, `DefenderState`, `MidfielderState`, `ForwardState` — plus `PlayerIntent` type)
- `crates/fw-match-sim/src/dispatch.rs` (NEW; `dispatch_tick`, role-specific tick functions, pre-emption-hooks stub, `apply_intent` velocity mutation)
- `crates/fw-match-sim/src/goalkeeper_fsm.rs` (NEW; pure-FSM stub Rust functions per `GoalkeeperState` per ADR-0006 §"Concrete sketch")
- `crates/fw-match-sim/src/subtree_library.rs` (NEW; hardcoded stub subtree library for T1-2b-iii-a — one stub subtree per role-state returning `MoveToFormationPosition`. Defer content-pack RON loading to -iii-b or T2-3.)
- `crates/fw-match-sim/src/player.rs` (MODIFIED; `PlayerState` gains `role` + `role_state: u8` + `local_decision_counter: u32`)
- `crates/fw-match-sim/src/lib.rs` (MODIFIED; mod declarations + `MatchState::initial` assigns roles by slot (1 GK + 4 DEF + 3 MID + 3 FWD per side = 4-3-3 default for T1) + `tick_match` calls `dispatch_tick` after the heartbeat)
- `crates/fw-match-sim/src/canonical.rs` (MODIFIED; encoder extension for new PlayerState fields)
- `crates/fw-match-sim/tests/bt_runner_proptest.rs` (NEW; traversal determinism + role assignment determinism + decision-counter increment invariants)
- `crates/fw-match-sim/tests/dispatch_proptest.rs` (NEW; dispatch_tick determinism over 100 random seeds)
- `crates/fw-replay/tests/canonical_hash.rs` (MODIFIED; `PINNED_60_TICK` rebaselined)
- `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron` (MODIFIED; `expected_hash` updated)

### Files out of scope (do NOT touch — escalate if needed)
- `docs/DESIGN_DOC.md` / `docs/DECISIONS.md` / `docs/adr/*.md` / `docs/specs/*.md` (source-of-truth; no spec mutation during impl)
- `CLAUDE.md` / `docs/MASTER_PLAN.md` (status flip is the only allowed mutation)
- `crates/fw-content/**` (BT subtrees as content-pack data defer to -iii-b/T2-3; T1-2b-iii-a uses hardcoded stubs)
- `content/sources/**` (no RON authoring this row)
- `crates/fw-core/**` (Q32 + Tick + Seed locked; `SeedLayer` defer-to-move at T1-3)
- `crates/fw-tauri/**` (no IPC changes; per-player BT state DTO projection defers to T1-6 frontend)
- `frontend/**` (TacticalBoard doesn't surface role/role_state yet)
- `crates/fw-match-sim/src/ball*.rs`, `tactic_fsm.rs`, `decision_cadence.rs` (prior rows; no changes)
- `crates/fw-match-sim/src/dto.rs` (DTO projection defers)

### Intentionally NOT done in this task
- Real BT decision logic — every leaf is `MoveToFormationPosition`. -iii-b adds xG / xT / pitch-control / pressing / personality bias to make decisions real.
- BT subtree authoring as content-pack RON data. Subtree library is hardcoded Rust for -iii-a.
- Universal pre-emption hooks (single-chaser claim, foul reaction, set-piece switchover) — stubbed to return `None`. Wired in -iii-b or T1-4 when MatchEvent exists.
- PlayerSeparation pass — that's -iii-c.
- Manual eyeball acceptance gate — that's -iii-c.
- Real GK FSM behavior — each GK state's Rust function returns a stub PlayerIntent. Real positioning + shot-stopping + sweeper-keeper logic at -iii-b.
- `MatchEvent` emission from BT decisions (Goal / Shot / Pass) — T1-4.
- BT site bindings to specific `PlayerAttributes` reads — -iii-b consumes `bt-attribute-binding.md` proper.
- Resumable BT state (a leaf returning `NodeStatus::Running`) — for -iii-a, every leaf returns `Success` immediately. Resumable trees defer to -iii-b when leaves can actually pause (e.g. "execute a 3-tick dribble move").

### Plan (6 chunks; TDD RED-GREEN-REFACTOR per chunk per superpowers mandate)
- [x] Chunk 1 (RED+GREEN): `bt.rs` core types — `NodeStatus` (Success / Failure / Running) + `Node` enum (Selector / Sequence / Decorator / Leaf / Condition) + `Tree` wrapper + `tick()` traversal function. Tests: empty tree returns Success; single-leaf tree returns leaf's status; Sequence short-circuits on Failure; Selector short-circuits on Success; deterministic traversal order over BTreeMap-keyed subtree refs.
- [x] Chunk 2 (RED+GREEN): `role_states.rs` — `Role` enum (GK/DEF/MID/FWD) + per-role state enums (`GoalkeeperState` ~10 variants, `DefenderState` ~7 variants, `MidfielderState` ~7 variants, `ForwardState` ~7 variants per ADR-0006 §"Concrete sketch") + `PlayerIntent` type (initially: enum with `MoveToPosition { x, y }` variant only). Tests: enum round-trip (serde); default state per role.
- [x] Chunk 3 (RED+GREEN): `subtree_library.rs` — `SubtreeLibrary::default_skeleton()` returns hardcoded BTreeMap of (Role, state_tag) → stub Tree. Each stub Tree is a single `Leaf(MoveToFormationPosition)` returning Success. Tests: every (role, state) combo resolves to a tree; stub tree returns Success.
- [x] Chunk 4 (RED+GREEN): `goalkeeper_fsm.rs` — pure Rust per-state functions per ADR-0006. T1-2b-iii-a stub: every function returns `PlayerIntent::MoveToPosition { x: own_goal_x, y: 0 }`. `GoalkeeperState::evaluate_transitions()` defaults to `InBoxPositioning` always. Tests: GK tick returns goal-line position regardless of state.
- [x] Chunk 5 (RED+GREEN): `PlayerState` extensions — `role: Role` + `role_state: u8` (tagged into per-role enum) + `local_decision_counter: u32`. `MatchState::initial` assigns 4-3-3 formation: slot 0 + 11 = GK; slots 1-4 + 12-15 = DEF; slots 5-7 + 16-18 = MID; slots 8-10 + 19-21 = FWD. Canonical encoder extended for new fields (4-byte counter + 1-byte role + 1-byte role_state per player; +6 bytes × 22 = +132 bytes per match-state). Wire-format module doc updated.
- [x] Chunk 6 (RED+GREEN + REBASELINE): dispatch_tick + apply_intent + position integration wired. Canonical hash rebaselined to `blake3:c0b5e395…c1430ff`. Subsequently the self-review fix pass (P0 player-position integration + P1 PlayerRoleState collapse + 5 other findings) landed without changing the hash further — `PlayerRoleState` is byte-identical to the prior split-field encoding, and position integration produces zero displacement when all 22 players start at formation positions.

</details>

<details>
<summary>T1-2b-ii task spec (closed 2026-05-13)</summary>

- **id:** T1-2b-ii
- **title:** `fw-match-sim`: tactic FSM + decision-cadence stagger
- **started:** 2026-05-13
- **task class:** sim-rust (canonical-state extension in fw-match-sim; new module x 2)
- **required subagent:** `gameplay-programmer` (per CLAUDE.md §5 — sim-rust ≥100 LoC; expected ~600-1000 LoC including tests; two specs in scope)
- **TDD mandate:** **YES** — second row under the superpowers TDD mandate (per `docs/DECISIONS.md` 2026-05-13). RED-GREEN-REFACTOR per chunk; cite the cycle in commit body.
- **Canonical-hash rebaseline:** AUTHORIZED in this task-spec — adding three new fields to `MatchState` is a canonical schema bump per ADR-0012 trigger #1. Both `crates/fw-replay/tests/canonical_hash.rs::PINNED_60_TICK` AND `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron::expected_hash` update atomically in the same commit. Cross-OS verification via post-commit CI matrix.

### Design references
- `docs/specs/tactic-fsm.md` — 5 states + 2 Hz heartbeat + transition table + test contract (Tranche-4 spec; Codex-audited).
- `docs/specs/decision-cadence-stagger.md` — SLOT_TEMPLATE + Fisher-Yates + `should_decide()` + reactive-interrupt cooldown semantics + test contract (Tranche-4 spec; Codex P1 fixed re: birthday-problem random-modulo).
- `docs/adr/0001-match-engine-architecture.md` §"Concrete shape" (layer 1 = tactic FSM; layer 2 = per-player runner).
- `docs/adr/0009-rng-seed-derivation.md` — `SeedLayer::Decision` with `site = 0` reserved for the stagger-shuffle draw; tactic FSM is RNG-free.
- `docs/adr/0012-hash-rebaseline-policy.md` — trigger #1 (canonical schema bump); commit-body marker required.

### Acceptance criteria (from MASTER_PLAN T1-2b-ii row + cadence/FSM specs)
1. **`decision_slots: [u8; 22]` in canonical `MatchState`** — populated at match-init via `assign_decision_slots(seed)` Fisher-Yates over SLOT_TEMPLATE.
2. **`interrupt_cooldown_until: [Tick; 22]` in canonical `MatchState`** — initialized to `Tick::ZERO`; consumed by `should_decide()` predicate; mutated by reactive-interrupt path (future T1-2b-iii consumes; T1-2b-ii adds field + predicate).
3. **`team_tactic_states: [TeamTacticState; 2]` in canonical `MatchState`** — one per side; carries `(state: TacticState, entry_tick: Tick)`.
4. **Canonical-hash regression test pins new layout** — release-mode `cargo test -p fw-replay --test canonical_hash` green at the new pinned constant.
5. **Slot-assignment determinism proptest** — same seed → byte-identical `[u8; 22]` over 100 random seeds.
6. **Balanced-multiset invariant proptest** — for any seed: count(slot==k) is 2 for k in 0..7, 1 for k in 7..15. Structural (Fisher-Yates is permutation).
7. **`decision_slots` immutability proptest** — slot array is byte-identical at start vs after a 600-tick run that fires synthetic interrupts; only `interrupt_cooldown_until` mutates.
8. **Tactic-FSM transition determinism proptest** — same `(from_state, archetype_params, event)` → same `to_state` over 100 random inputs.
9. **No RNG in tactic FSM** — instrumented `seed_fn` call count inside `tactic_fsm::*` is zero.
10. **Heartbeat-drift test** — fixture where smoke seed sits in HighPress for >10s (>600 ticks) without recovery → heartbeat transitions to MidBlock at the next 30-tick heartbeat boundary.
11. **Canonical hash REBASELINED** — old `blake3:0ddf91ef…c5722090` → new BLAKE3 (TBD post-implementation); both PINNED + RON fixture updated atomically.

### Files in scope
- `crates/fw-match-sim/src/tactic_fsm.rs` (NEW; `TacticState` + `SetPieceKind` + `TeamTacticState` + `TacticEvent` (local stub until T1-4 MatchEvent lands) + `PressIntensity` + `CounterIntent` (local until T1-2b-iii moves them to fw-content) + `transition()` + `heartbeat_check()`).
- `crates/fw-match-sim/src/decision_cadence.rs` (NEW; `SLOT_TEMPLATE` const + `assign_decision_slots(seed)` Fisher-Yates + `should_decide()` predicate).
- `crates/fw-match-sim/src/lib.rs` (mod declarations + `MatchState` field additions + `MatchState::initial` populates new fields + `tick_match` runs heartbeat every 30 ticks).
- `crates/fw-match-sim/src/canonical.rs` (extend `encode_match_state` to emit `decision_slots` + `interrupt_cooldown_until` + `team_tactic_states` deterministically; update wire-format module doc + bump VERSION constant if structurally required — current `VERSION: u16 = 1` may stay since the version is a coarse compat handle, not per-field).
- `crates/fw-match-sim/tests/decision_cadence_proptest.rs` (NEW; slot determinism + balanced multiset + immutability invariants).
- `crates/fw-match-sim/tests/tactic_fsm_proptest.rs` (NEW; transition determinism + no-RNG + heartbeat-drift invariants).
- `crates/fw-replay/tests/canonical_hash.rs` (`PINNED_60_TICK` updated; rebaseline-history block appended).
- `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron` (`expected_hash` + metadata block updated).

### Files out of scope (do NOT touch — escalate if needed)
- `docs/DESIGN_DOC.md`
- `docs/DECISIONS.md` (no new architectural decision; ADR-0001 + ADR-0009 + ADR-0012 already cover; rebaseline marker goes in commit body)
- `docs/adr/*.md` (no ADR changes — T1-2b-ii implements existing ADR-0001 layers 1-2)
- `docs/specs/tactic-fsm.md` + `docs/specs/decision-cadence-stagger.md` (source-of-truth; do NOT mutate during implementation)
- `CLAUDE.md`
- `docs/MASTER_PLAN.md` (status flip is the only allowed mutation)
- `crates/fw-content/**` (archetype param extensions — `default_in_defence_state`, `line_height_metres_per_state`, `counter_intent`, `press_intensity` — defer to T1-2b-iii when BT consumes them; T1-2b-ii uses local enums in `tactic_fsm.rs`)
- `content/sources/archetypes/**` (no RON changes; default param values hardcoded into `tactic_fsm.rs` for tests)
- `crates/fw-core/**` (Q32 + Tick + Seed locked at T0)
- `crates/fw-tauri/**` (no new IPC; `MatchFrameDto` projection of new fields defers to UI work)
- `frontend/**` (TacticalBoard doesn't surface decision_slots or tactic-state yet — diagnostic UI defers to T1-6)
- `crates/fw-match-sim/src/ball*.rs` (T1-2b-i is closed; no further changes)
- `crates/fw-match-sim/src/dto.rs` (DTO projection of new fields defers to T1-6 frontend work)
- `crates/fw-match-sim/src/bin/dump_frames.rs` (no changes; reads MatchState by canonical projection)

### Intentionally NOT done in this task
- BT runner / FSM-of-BTs / utility selector (T1-2b-iii)
- PlayerSeparation pass (T1-2b-iii)
- `MatchEvent` enum (T1-4 — T1-2b-ii defines a local `TacticEvent` enum as a stub; reconciliation at T1-4 may rename / consolidate).
- Archetype param extensions in `fw-content` (T1-2b-iii — `default_in_defence_state` / `line_height_metres_per_state` / `counter_intent` / `press_intensity` move to `TacticalArchetype` then; T1-2b-ii uses local hardcoded defaults).
- Per-player decision counters (`local_decision_counter` for RNG site derivation — T1-2b-iii when BT firing draws RNG).
- Heartbeat drift rules requiring spatial state (`own_mean_x`, `score_lead_for_team`) — partially-doable now (score IS in MatchState) but full set defers to T1-2b-iii when spatial inputs exist.
- Wire `should_decide` into the per-tick loop — T1-2b-iii when BT exists to dispatch to. T1-2b-ii defines the predicate but doesn't yet iterate-and-call.

### Plan (6 chunks; TDD RED-GREEN-REFACTOR on each behavior chunk per superpowers mandate)
- [x] Chunk 1 (RED+GREEN): `tactic_fsm.rs` types — `TacticState` enum (5 variants + `SetPiece(SetPieceKind)`) + `SetPieceKind` enum (11 variants per spec) + `TeamTacticState { state, entry_tick }` struct + `PressIntensity` + `CounterIntent` enums + `TacticEvent` local stub enum (mirrors transition-table events). Tests: type serde round-trip; default `TeamTacticState::initial()` is `MidBlock @ Tick::ZERO`.
- [x] Chunk 2 (RED+GREEN): `tactic_fsm::transition(state, archetype_params, event, now_tick) -> TacticState` pure function per the spec's transition table. Tests: every transition row hits; guards work (recovery_likely=false→LowBlock); thrash-prevention guard (HighPress re-entry requires 600+ ticks since prior entry); no RNG.
- [x] Chunk 3 (RED+GREEN): `tactic_fsm::heartbeat_check(state, match_state, archetype_params) -> Option<TacticState>` — HighPress-timeout-10s rule (the only rule with sufficient inputs at T1-2b-ii; others defer). Tests: HighPress at entry_tick 0, current tick 600 → returns Some(MidBlock); HighPress at entry_tick 0, current tick 599 → returns None.
- [x] Chunk 4 (RED+GREEN): `decision_cadence.rs` — `SLOT_TEMPLATE: [u8; 22]` const + `assign_decision_slots(seed) -> [u8; 22]` Fisher-Yates over `ChaCha8Rng::seed_from_u64(seed_fn(match_seed, 0, SeedLayer::Decision, 0))` + `should_decide(roster_slot, decision_slots, interrupt_cooldown_until, tick) -> bool`. Tests: balanced multiset; determinism (same seed → same [u8;22]); should_decide fires on `tick % 15 == slot`; cooldown suppression works.
- [x] Chunk 5 (RED+GREEN): `MatchState` field extensions + canonical-encoder wire-format extension + `MatchState::initial` populates via `assign_decision_slots(seed)` + `team_tactic_states: [TeamTacticState; 2]` defaults to `[MidBlock @ Tick::ZERO; 2]` + canonical doc-comment updated. `tick_match` runs heartbeat every 30 ticks per team (calls `heartbeat_check`; applies if Some + advances entry_tick). Canonical hash test WILL drift here.
- [x] Chunk 6 (RED+GREEN + REBASELINE): proptest invariants per acceptance criteria + canonical hash rebaselined to `blake3:5aea582b…cf5c544`.

</details>

<!-- Historical scope-spec for the prior T1-2a retained below for grep-back reference -->

<details>
<summary>T1-2a task spec (closed 2026-05-13)</summary>

- **id:** T1-2a
- **title:** Dev-tier 2D tactical board (per ADR-0007 Layer 2 + ADR-0008 browser-dev mode)
- **started:** 2026-05-13
- **completed:** 2026-05-13
- **task class:** frontend (SolidJS + PixiJS + Tauri IPC) + small Rust binary
- **subagent rotation:** main-thread for Rust chunks (1-2 + 6); `ui-programmer` for frontend chunks (3-5).
- **TDD exemption:** YES — UI + binary serialization wrapper around existing canonical state; NOT sim/memory/replay/save/content-runtime behavior code.

Implements ADR-0007 Layer 2 (dev verification surface) + ADR-0008 (browser-dev mode). Reuses ADR-0004 IPC contract patterns.

</details>

<!-- Historical scope-spec for the prior T1-1 retained below for grep-back reference -->

<details>
<summary>T1-1 task spec (closed 2026-05-13)</summary>

- **id:** T1-1
- **title:** `fw-content` schema — `PlayerTemplate` (ADR-0002 55-field model) + `TacticalArchetype` Codex Imp #3 conversion + first RON fixtures
- **started:** 2026-05-13
- **completed:** 2026-05-13
- **task class:** architecture-cross-crate (schema lock across fw-core + fw-content)
- **TDD exemption:** YES (data-only)

### Files out of scope (do NOT touch — escalate if needed)
- `docs/DESIGN_DOC.md`
- `docs/DECISIONS.md` (no new architectural decision logged here — T1-1 IMPLEMENTS ADR-0002, doesn't change it)
- `docs/adr/0002-player-attribute-model.md` (the source of truth — don't drift)
- `CLAUDE.md`
- `docs/MASTER_PLAN.md` (status flip is the only allowed mutation)
- `crates/fw-match-sim/**` (T1-2b's job)
- `crates/fw-memory/**`
- `crates/fw-replay/**`
- `crates/fw-save/**`
- `crates/fw-tauri/**`
- `frontend/**`
- `crates/fw-core/src/q32.rs` (locked at T0)

</details>

## Recently completed

- 2026-05-15 — **T1-3 signature schema stub (fw-content; type system only, no triggers).** Lands the signature schema lock per ADR-0011 + the per-player affinity carry-forward from FW v1's `IdentityPacket.SignatureCandidates`. New `crates/fw-content/src/signature.rs` with full ADR-0011 type surface: `SignatureId` (newtype + try_new validator enforcing dotted-pack-id format per Content/RULES.md §2 — `vendor.pack[.vN]:signature.<slug>` with ≥2-segment dotted-lowercase prefix + `[a-z0-9-]+` slug); `SignatureCandidate { signature_id, affinity: Q32 }` with `try_new` validator enforcing affinity ∈ [0,1] (P1 fix from type-design review); 8-variant `RoleFamily` (Goalkeeper=0 through Striker=7, stable u8 discriminants); 4-variant `BiasCategory` (Attacking=0 / Defensive=1 / BuildUp=2 / SetPiece=3); `SimBiasSnapshot` with 5 Q32 multipliers (shoot/pass/dribble/press/cover) collapsing ADR-0003 §5's 7 personality-bias surfaces (long_pass + safe_pass → pass_mul; hold_mul dropped — documented in type doc-comment with rollback path to 7 fields as a backwards-compat extension); `CooldownPolicy::EveryTicks(u32)` (default 600 = 10s @ 60Hz) + `CooldownPolicy::PerMatchCount(u8)` per ADR-0011; `StackingPolicy::Exclusive { category: BiasCategory }`; `SignatureTrigger::NoOpStub` placeholder enum (T1-2b-iv expands with real predicate variants); `SignaturePresentationRecipe` string-placeholder stub (T2 fills with real commentary-line-bank-id + camera-framing-hint); `SignatureDefinition` composes everything with schema_version: 1. `PlayerTemplate.signature_candidates: Vec<SignatureCandidate>` field added with `#[serde(default)]` (backwards-compat with existing fixtures). `ContentStore.signature_definitions: BTreeMap<String, SignatureDefinition>` with `load_sources` walking `content/sources/signatures/*.ron`. One no-op fixture at `content/sources/signatures/no-op-stub.ron` (id `fwh.core:signature.no-op-stub`); one player fixture `content/sources/players/sample-am.ron` references it with affinity 0.70. **Canonical hash UNCHANGED at `blake3:1db6020c…59c798`** — PlayerTemplate is template data for procedural generation; the smoke seed used by canonical_hash regression uses hardcoded positions + `mid_range_baseline()`, NOT loaded PlayerTemplate. Schema additions are invisible to canonical match-state bytes by design. **Data-only TDD-exempt row** per CLAUDE.md §5 (mirrors T1-1 exemption); tests still ship (14 unit + 5 try_new + 3 load = 22 new tests in fw-content; 32 → 54). Self-review: 1 type-design pass → Revise verdict with 2 P1s closed in-place: P1-1 tightened SignatureId validator (the agent's initial split-on-colon approach allowed ANY non-empty whitespace-free prefix; reviewer caught the gap; the tightened version enforces dotted-lowercase segments ≥2 + reconciles with Content/RULES.md §2's mod-pack carve-out — both `fwh.core` and `mod.community.somerset` accepted, rejecting uppercase/underscore/single-segment/empty-segment forms); P1-2 added `SignatureCandidate::try_new` with affinity range validator (previously any caller could write Q32::from_int(5) and skew T1-2b-iv's softmax). P3/P4 deferred to T2-3: `SignatureId::Deserialize` accepts arbitrary strings (validation only at try_new) — `ContentStore::load_baked` should call try_new post-parse when load_baked actually walks signatures/; `SignaturePresentationRecipe` carries its own schema_version separate from `SignatureDefinition`'s — reconcile at T2 when placeholders become real types. **Cross-crate dep addition (authorized)**: `fw-content/Cargo.toml` gains `fw-match-sim` + `blake3` as `[dev-dependencies]` so the canonical-hash-unchanged test can run end-to-end. ~640 LoC total (~370 type code + ~270 tests + fixture). `scripts/fw verify` clean.
- 2026-05-15 — **T1-2b-iii-d PlayerSeparation + visual playtest gate (FINAL T1-2b-iii row).** `crates/fw-match-sim/src/separation.rs` ships `apply_player_separation(state: &mut MatchState)` per the FW v1 `PlayerSeparation.cs` carry-forward (design intent only; Rust idioms). 231-pair iteration (22 players choose 2) in lex order via `for i in 0..n; for j in (i+1)..n` — structurally deterministic, no HashMap/BTreeSet needed. `MIN_PLAYER_DISTANCE: Q32 = 0.4m`; `EPSILON_SEPARATION: Q32 ≈ 1mm`; sqrt via `cordic`. Position-only adjustment (velocity untouched ⇒ vel magnitude preserved trivially, satisfying ±5% invariant exactly). Zero-distance fallback: lower-slot pushes -X, higher-slot pushes +X by EPSILON, deterministic by slot order. `tick_match` doc-comment now enumerates 6 explicit steps; step 6 = `apply_player_separation` (runs AFTER position integration, BEFORE next tick). **Canonical hash REBASELINED** per ADR-0012 trigger #1: `blake3:235f6c5e…181288d` → `blake3:1db6020c…59c798` (player positions now corrected after integration; vel + ball + tactic state untouched). 7 unit tests in `separation.rs` (pair-iteration count + lex order, overlap resolution, zero-distance fallback for slots 0/1 AND slots 17/18 — the latter pins that loop-order doesn't reverse the slot-direction rule, ball unchanged) + 7 proptests in `tests/separation_proptest.rs` covering all 6 MEMORY acceptance invariants 1:1: (1) isolated pair resolved, (2) non-overlapping unchanged, (3) ball untouched by separation, (4) velocities unchanged (exact equality, stronger than the spec's ±5%), (5) deterministic via canonical-encoding equality, (6) tick_match wiring 100-tick run, plus inv6b "no pair stays overlapped >2 consecutive ticks across 100 ticks." 600-tick smoke fixture generated cleanly at `frontend/public/dev-fixtures/smoke-600t.json` (2.3MB, 601 frames). **Manual eyeball gate PASSED 2026-05-15** — user opened `/dev/board?source=fixture:/dev-fixtures/smoke-600t.json` and verbatim observed: "players start overlapped at 4-3-3 spawn positions but fan out within a few ticks; some back-and-forth movement after separation resolves (expected skeletal-BT oscillation; iii-d's separation invariant holds)." The oscillation is downstream of separation (utility-scored BT picks at every 4Hz cadence with identical mid_range_baseline attrs) — not a regression. Sixth row under the superpowers TDD mandate. **Pre-eyeball test-coverage audit caught 3 gaps post-implementation** (ball-unchanged was untested; zero-distance fallback was partial; Inv6 was 10 ticks instead of 100) — closed via a focused fix message rather than full self-review triple since the gaps were already identified via close-reading the proptest file. This is the iii-c lesson applied prospectively: when an agent ships tests but they don't 1:1 cover acceptance, that's the BLOCK pattern; catch it BEFORE the row commits. Two regression edits to the fixture were also needed: cargo build output leaked into stdout during initial generation (used `2>/dev/null` for the regen); plus a stale vite process held port 1420 from a prior session and had to be killed before Claude Preview could manage a fresh dev server. ~200 LoC source + ~250 LoC tests. `scripts/fw verify` clean. **T1-2b-iii sub-phase fully closed**; next is T1-2b-iv signature dispatcher.
- 2026-05-13 — **T1-2b-iii-c BT site bindings + personality bias + utility-scored leaves.** Wires the iii-b math primitives into the iii-a BT runner. New `bt/` directory module (converted from flat `bt.rs`) with 4 new submodules: `personality_bias` (k₁..k₁₄ Q32 constants + 7 bias helpers per `docs/design/personality-bias-weights.md` mapping + `read_defender_pressure` PT divisor with `DefenderPressure` + `IsProgressive` newtype wrappers); `on_ball` (7 utility scorers: shoot / pass_short / pass_long / cross / dribble / hold_ball / lay_off); `off_ball` (5 utility scorers: track_back / press / mark_player / run_off_ball / hold_formation); `reactive` (4 predicate functions, defined but not wired into dispatch — T1-4 carries forward). Each utility function reads EXACTLY the spec-mandated primary + secondary `PlayerAttributes` per `docs/specs/bt-attribute-binding.md`; bias inputs flow through the bias-helper path (not direct reads). 24 binding-correctness tests consume the `*_ATTRS` const slices to enforce the spec contract. `PlayerIntent` expanded 2→19 variants (Idle / MoveToPosition + 7 on-ball + 5 off-ball + 5 GK). `subtree_library::select_outfield_intent()` builds candidate `[(ActionId, Q32)]` lists scored by utility + applied bias, then picks via `pick_top_n_softmax` seeded by `seed_fn(match_seed, tick, SeedLayer::UtilityTieBreak, decision_id)` per ADR-0009. `goalkeeper_fsm::evaluate_transitions` now uses real ball-position-based predicates (ShotStopping when ball in own box moving toward goal; SweeperKeeperRush past halfway no defender between ball + goal; DistributingFromHand when ball at GK position; InBoxPositioning default fallback) so 4 of 5 utility-bearing GK variants are reachable. `tick_match` position integration uses bare panic-on-overflow operators (no `checked_*().unwrap_or()` silent fallback). **Canonical hash REBASELINED** per ADR-0012 trigger #1: `blake3:b3b0e64f…d4da1169` → `blake3:235f6c5e…181288d` (utility outputs now drive BT-selected actions which mutate player vel non-trivially; behavior change at every 4 Hz cadence tick). Fifth row under the superpowers TDD mandate. **Self-review triple landed 4 P0 + 4 P1 + 1 P2 fixed in-place** — silent-failure-hunter verdict was BLOCK on the original implementation due to convergent contract violations: P0-1 binding-correctness drift across ~10/12 utility sites (the agent's `*_ATTRS` const slices were decorative; no test consumed them; ~10 sites diverged from spec including `utility_hold_formation` reading `personality.work_rate` directly — exact caveat the spec called out as bias-path-only). **Fix: walked every site against the spec; rewrote each utility function to read EXACTLY the documented attrs; shipped the load-bearing binding-correctness test suite the agent originally punted (24 tests).** P0-2 GK FSM frozen at InBoxPositioning (all 5 new `Gk*` variants were unreachable production code; 4/5 silently dead). **Fix: implemented real `evaluate_transitions` ball-position-based predicates; 10 GK predicate tests added.** P0-3 `checked_mul/add` recidivism in lib.rs position integration (reintroduced the iii-b purge anti-pattern). **Fix: bare operators (Q1 panic-on-overflow).** P0-4 `xt_delta` silently zeroing negative deltas in utility/xt.rs (iii-b carryover bug surfaced by iii-c use; backward passes were 0 xT instead of negative — miscalibrated long-pass utility). **Fix: bare subtraction; authorized iii-b scope expansion.** P1-1 personality bias inputs unvalidated (`bias > 1` propagated silently); **fix: debug_assert! on bias inputs in all 7 helpers.** P1-2 `select_outfield_intent` silent fallback (empty candidates silently emitted MoveToPosition; convergent type-design F2); **fix: `unreachable!()` matching GK panic discipline.** P1-3 the punted proptest was load-bearing for P0-1 (now shipped as unit tests instead). P1-4 RNG seed-layer mismatch (`SeedLayer::Decision` used where softmax tie-breaking is canonically `UtilityTieBreak` per ADR-0009); **fix: corrected layer in dispatch.rs RNG construction (this DID change the canonical hash — captured in the same rebaseline).** P2-2 `DefenderPressure` + `IsProgressive` newtype wrappers prevent arg-swap bugs in `apply_shoot_bias` vs `apply_long_pass_bias`. Two type-design follow-ups deferred to iii-d / T1-4: F1 PlayerIntent collapse to `Target` struct (18 of 19 variants carry identical `{ target_x, target_y }`; refactoring touches every match arm — defer); F5 GK return-type constraint (separate `GkIntent` subset enum). The proptest deferral pattern was directly responsible for the BLOCK verdict — captured as a lesson: when an agent punts a test, the punt's reasoning needs adversarial review BEFORE the row commits, not after. ~1500 LoC source delta (initial implementation + fix pass); 67 new tests (15 personality_bias + 24 binding-correctness + 10 GK predicate + 18 PlayerIntent / dispatch / softmax integration). 262+ unit tests + 26 proptest integrations all green; `scripts/fw verify` clean.
- 2026-05-13 — **T1-2b-iii-b utility math primitives + PlayerAttributes baseline (pure-function Q32 math; no BT wiring yet).** Implements ADR-0003 §1-§6 as deterministic pure functions. Five new utility modules in `fw-match-sim::utility`: `xg` (6-feature logistic with third-pass β values from `docs/design/xg-coefficients.md`; `ShotContext::try_new()` validates [0,1] invariants and returns Result), `xt` (`PitchZone` newtype + hand-authored 192-entry `XT_GRID` const at `pub(crate)` visibility + `xt_delta()`), `pitch_control` (Spearman closed-form per-point time-to-intercept; `PitchControlOutcome` carries `attacker_control + defender_control + neutral_control == 1` by construction), `pressing` (Bauer 2025 product form), `softmax` (`pick_top_n_softmax -> Option<ActionId>` using exp_q32 + seed_fn via ADR-0009 `SeedLayer::UtilityTieBreak`). New `fw-core::math` module with 257-entry symmetric Q32 LUTs (`sigmoid_q32` + `exp_q32`) backed by pure-Q32 `lut_eval` interpolation; f64 confined to one-shot `LazyLock` bake at process startup. `Q32` gained `to_raw` alias, `acos` (unconditional `assert!` matching `sqrt` precedent), `from_f64_clamped` (`pub(crate)` — LUT-bake-only escape per ADR-0003 §1). `PlayerAttributes::mid_range_baseline()` constructor in fw-core sets all 55 fields to Q32 ≈ 0.5; `mid_range_baseline_is_in_unit_range` sync test catches drift between baseline + `validate_unit_range`. `PlayerState` gained `attributes: PlayerAttributes` (`pub(crate)` + `attributes()` accessor). Canonical encoder VERSION 3→4; emits 55 attribute Q32 fields per player (+9680 bytes per match-state encoding). `tick_match` UNCHANGED structurally — math modules are pure functions exposed for iii-c to wire into BT site bindings. **Canonical hash REBASELINED** per ADR-0012 trigger #1: `blake3:c0b5e395…c1430ff` → `blake3:b3b0e64f…d4da1169`. Fourth row under the superpowers TDD mandate. **Self-review triple landed 4 P0 + 9 P1 + 3 P2 fixed in-place** (the P0 silent-failure verdict was BLOCK on the original implementation): P0-1 `lut_eval` rewritten in pure Q32 — removed f64 from per-tick canonical path (hash stayed bit-identical because LUT entries stored as Q32 + Q32 index arithmetic `(x+8)*16` is exact); P0-2 all `checked_*().unwrap_or()` silent-failure patterns replaced with bare operators (panic-on-overflow per Codex Q1) + invariant docs; P0-3 `Q32::acos` debug_assert → unconditional assert!; P0-4 missing 12-yard central xG canonical sanity-check test added + xG bands tightened (30m: [1.5%, 5%]; penalty: [58%, 72%]); P1-1 `PitchControlOutcome` gained `neutral_control: Q32` field + normalization so `attacker + defender + neutral == 1` holds by construction; P1-4 7-invariant proptest file added (`utility_proptest.rs`); P1-5 `ShotContext::try_new + ShotContextError`; P1-6 `from_f64_clamped` pub → pub(crate); P1-7 `XT_GRID` pub → pub(crate); P1-8 `pick_top_n_softmax` returns `Option<ActionId>` (drops `Default` bound; eliminates ActionId=0 collision); P1-9 misleading `PlayerSnapshot` doc-comment removed; P2-1 `PlayerState::attributes` pub → pub(crate) + accessor mirroring iii-a `local_decision_counter` pattern; P2-2 `q32.rs` `EXEMPT_FILES` exemption justified narrowly (post-P1-6); P2-3 `mid_range_baseline_is_in_unit_range` sync test added. P3 deferred: `determinism-audit.py` cfg(any(test, feature)) shape coverage; `pitch_control` Vec allocation per call (T1-2c performance pass). **Cross-crate scope expansions authorized**: new `fw-core::math` module (load-bearing per ADR-0003 §1; sibling crates fw-scouting/fw-memory will consume); new `Q32` methods (`to_raw`/`acos`/`from_f64_clamped`); `determinism-audit.py` cfg(test) stripping pass + narrow EXEMPT_FILES additions for `math.rs` + `q32.rs`. ~370 LoC source (+ 9680-byte hash-paid encoding cost; 13 fixes). 255 unit tests + 26 proptest integrations all green; `scripts/fw verify` clean.
- 2026-05-13 — **T1-2b-iii-a `fw-match-sim` BT runner + per-role BT skeletons (skeleton tier).** Five new modules in fw-match-sim: `bt` (Tree/Node/NodeStatus/Selector/Sequence/Decorator/Leaf/Condition + ordered Vec traversal), `role_states` (Role 4-variant enum + per-role state enums + `PlayerRoleState` typed-pair enum + PlayerIntent), `subtree_library` (4-3-3 hardcoded stub trees + FORMATION_4_3_3_POSITIONS const), `goalkeeper_fsm` (pure Rust functions per GK state per ADR-0006), `dispatch` (dispatch_tick + apply_intent + preempt_check stub + evaluate_transitions stub). `PlayerState` gained `role_state: PlayerRoleState` (replaces split role+role_state via atomic typed pairing — illegal states unrepresentable) + `local_decision_counter: u32` (pub(crate) + accessor). Canonical encoder VERSION 2→3; wire-format diagram extended for new player section. `tick_match` now: (1) advances ball physics, (2) runs tactic-FSM heartbeat, (3) calls `dispatch_tick` iterating 22 roster slots gated by `should_decide`, (4) integrates player position from velocity each tick (P0 fix — without it the sim modeled nothing). Roles assigned by slot per default 4-3-3 (1 GK + 4 DEF + 3 MID + 3 FWD per side). **Canonical hash REBASELINED** per ADR-0012 trigger #1: `blake3:5aea582b…cf5c544` → `blake3:c0b5e395…c1430ff`. Third row under the superpowers TDD mandate. Self-review triple landed 1 P0 + 4 P1 + 3 P2 fixed in-place (player position integration P0; PlayerRoleState typed-pair collapse making illegal states unrepresentable, P1 — load-bearing fix per type-design; `SubtreeLibrary::lookup_outfield` infallible — panics loudly on miss instead of silent Idle fallback, P1; `local_decision_counter` visibility tightening with `bump_decision_counter()` method, P1; outfield `evaluate_transitions` step before BT lookup matching ADR-0006 §"Concrete sketch", P1; `formation_position` unconditional `assert!` replacing release-stripped debug_assert+clamp, P2; misleading test renamed + honest movement-verifying replacement added, P2; `roster_slot`/`formation_slot` naming disambiguation, P2). P3 deferred: PlayerState::at Midfielder default footgun, PlayerIntent::priority field, LeafKind/PlayerIntent Idle redundancy, wire-format insta snapshot. ~880 LoC source + ~470 LoC tests + ~150 LoC self-review refactor. 138 unit tests + 19 proptest integrations; `scripts/fw verify` clean.
- 2026-05-13 — **T1-2b-ii `fw-match-sim` tactic FSM + decision-cadence stagger.** Two new modules (`tactic_fsm.rs` + `decision_cadence.rs`) implementing `docs/specs/tactic-fsm.md` (5 states + 2 Hz heartbeat + transition table) + `docs/specs/decision-cadence-stagger.md` (Fisher-Yates balanced [u8;22] slot assignment + `should_decide` predicate + reactive-interrupt cooldown semantics). `MatchState` gained 3 canonical fields: `decision_slots: [u8;22]` + `interrupt_cooldown_until: [Tick;22]` + `team_tactic_states: [TeamTacticState;2]`. Canonical encoder VERSION 1→2; wire-format diagram updated. Heartbeat wired into `tick_match` (home tick % 30 == 0, away tick % 30 == 15; offset reduces peak load). **Canonical hash REBASELINED**: `blake3:0ddf91ef…c5722090` → `blake3:5aea582b…cf5c544`. Second row under the superpowers TDD mandate; RED-GREEN-REFACTOR per chunk. ~900 LoC code + ~470 LoC tests = ~1370 LoC total. Self-review triple landed: 5 P1s + 2 P2s + 2 P3s fixed in-place (rand dep redundancy → `rand_chacha::rand_core` re-export; `should_decide`/`seed_fn` negative-tick wrap → `rem_euclid` + `debug_assert`; `TeamTacticState` field visibility tightened to `pub(crate)` with `state()` + `entry_tick()` accessors; `heartbeat_check` signature returns full `TeamTacticState` to remove caller-must-transition contract; cadence test #3 immutability now fires synthetic interrupts; `SLOT_TEMPLATE` → `pub(crate)`; `TOTAL_PLAYERS` const-assert against u8 truncation; TacticEvent → MatchEvent reconciliation TODO for T1-4). 2 P1s deferred to T1-3 (SeedLayer owed move to `fw-core` per ADR-0009) + T1-2b-iii (SetPieceKind For/Against naming harmonization). `scripts/fw verify` clean.
- 2026-05-13 — **T1-2b-i `fw-match-sim` ball physics** — semi-implicit Euler in Q32 (gravity, drag, Magnus stub, bounce, friction). `BallState` extended with `spin_{x,y,z}: Q32` (canonical schema bump per ADR-0012 trigger #1); canonical-state ball block grew from 48 → 72 bytes per ball. `BallPhysicsCoefficients` + `phase1_seeds()` (g=9.81, drag=0.02, magnus=0, bounce=0.55, friction=0.25) + `is_well_formed()` validator. `ball_step(state, coeffs)` integrator wired into `tick_match`. 3 proptest invariants live (energy-monotone, no-overflow over 1800 ticks, validator rejects out-of-range). **Canonical hash REBASELINED**: `blake3:d6258107…d96b1a49` → `blake3:0ddf91ef…c5722090` (both `crates/fw-replay/tests/canonical_hash.rs::PINNED_60_TICK` and the RON fixture `0xdeadbeefdeadbeef.ron::expected_hash` updated atomically). First row under the superpowers-plugin TDD mandate; RED-GREEN-REFACTOR observed per chunk. Self-review triple: 1 P0 (rolling-friction-fires-on-zero-bounce-tick) + 2 P1s (is_well_formed deadcode gate, stale wire-format diagram) + 1 P3 (#[must_use] on CanonicalEncoder::new) fixed in-place; P2/P3 follow-ups captured in commit body. ~480 LoC; 27 fw-match-sim tests + 3 proptest invariants + 5 fw-replay canonical_hash tests all green; `scripts/fw verify` clean.
- 2026-05-13 — **T1-2a Dev-tier 2D tactical board** (ADR-0007 Layer 2 + ADR-0008). `MatchFrameDto` in `fw-match-sim::dto` (camelCase serde; Q32→f64 projection; `#![allow(clippy::float_arithmetic)]` scoped + determinism-audit exemption documented). `match_frames(seed_hex, tick_count)` IPC command in fw-tauri returning `Vec<MatchFrameDto>` (length tick_count+1; pinned by 2 sync tests via `tauri::async_runtime::block_on`). `dump_frames` clap CLI binary in `crates/fw-match-sim/src/bin/` — bit-identical stdout across reruns. SolidJS `TacticalBoard.tsx` with PixiJS Application (one-time create in onMount, destroy in onCleanup per Frontend/RULES.md §4). `FrameSource` interface + `TauriFrameSource` + `HttpFrameSource` impls + `frameSourceFromUrlParams` factory (fail-loud on bad `?source=` values per Codex audit). `MatchStateDto` retroactively gained `#[serde(rename_all = "camelCase")]` (Codex audit P0 fix on pre-existing rule violation). `window.fwDev` DEV-only debug surface. E2E verified via Claude Preview: navigated `/dev/board?source=fixture:/dev-fixtures/smoke.json`, fixture loaded, scrubTo(30) + scrubTo(45) drove the scrubber, pitch + 22 dots + ball + readout rendered. ~800 LoC; canonical hash UNCHANGED. Self-review triple: 1 P0 + 4 P1 closed in-place.
- 2026-05-13 — T1-1 `fw-content` schema lock at commit `69f900b9` (ADR-0002 55-field player model + Codex Imp #3 conversion + first RON fixtures). `PlayerAttributes` in `fw-core` (14/10/8/6 visible + 14/3 hidden = 55 Q32 fields); `KNOWN_ATTRIBUTE_NAMES` const + size-of static asserts pin schema shape. `AbilityCeiling` encapsulated (`new_unchecked` pub(crate) post-audit-pass-1 follow-up). `RoleId` newtype + `RoleAffinityTable` with collect-all validators. `TacticalArchetype.buildup_speed_factor` → `u16 bps` with `BUILDUP_SPEED_BASELINE_BPS = 10_000`. `schema_version: 1` on new content types + fixtures. Followed by ~10 audit-remediation commits (tranches 1-7 of the full-project audit + pre-T1-2b re-audit passes 1, 2, 3). Canonical hash UNCHANGED throughout. All audits ultimately GREEN at `e780792`.
- 2026-05-13 — T0-12 Fix pre-existing scaffold build failures — fw-tauri commands moved to sibling module (known Tauri 2 `pub` + `#[tauri::command]` bug); fw-content-baker `#![allow(dead_code)]` on staging modules; src-tauri build.rs stubs frontend/dist for clean-clone `cargo build`; tauri icons generated (gitignored); ui-vocabulary.md meta-references wrapped in sentinels. `cargo test --workspace --release` 19 test-runs all green.
- 2026-05-13 — T0-7 Pin BLAKE3 canonical hash on dev box — `d6258107…` pinned; `cargo test -p fw-replay` 4/4 green; cross-OS matrix → T0-7b.

## Active decisions

- Q32.32 (`fixed::FixedI64<U32>`) for all canonical-state quantities. `#[deny(clippy::float_arithmetic)]` on sim crates.
- BLAKE3 (not SHA-256) for canonical-state hashing.
- RON files for content sources + replay fixtures (human-diffable).
- Bincode 2 for save format.
- `ChaCha8Rng` for all sim randomness. `thread_rng` / `StdRng` banned in sim code.
- `BTreeMap` / `IndexMap` only in canonical-state-emitting code. `HashMap` banned in sim crates.
- Tauri 2 + SolidJS + Tailwind v3 + TanStack Table v8 + PixiJS v8 + ECharts.
- Single primary workflow command: `/next` (see `.claude/skills/next/SKILL.md`).
- Codex review at phase-gates via PR (not per-task).
- Per-task self-review via `pr-review-toolkit` subagents on ≥100 LoC code changes.

## Carry-forward debts (FW v1 → v2)

Logged 2026-05-13 from the T1-1-vs-IdentityPacket comparison. Full detail + pinning rows in `REFERENCES.md` "Carry-forward debts" table. Headline items:

- **T1-3** owes `PlayerTemplate.signature_candidates: Vec<SignatureCandidate>` — v1 had this in Phase 3, v2 T1-1 deferred. Pillar 5 has no per-player linkage until this lands.
- **T1-3** owes the move of `SeedLayer` enum from `fw-match-sim::decision_cadence` to `fw-core::seed` per ADR-0009 (Codex P1 self-review type-design, T1-2b-ii). Currently sibling crates that need RNG layers (`fw-scouting::ScoutObservation`, `fw-memory::MemoryEvent`, `fw-content-baker::ContentBake`) cannot import without depending on fw-match-sim — wrong dep direction.
- **T1-2b-iii** owes the move of `PressIntensity`, `CounterIntent`, `default_in_defence_state`, `line_height_metres_per_state` from `fw-match-sim::tactic_fsm` (local stubs) to `fw-content::TacticalArchetype` RON-loaded params. Also owes the SetPieceKind For/Against naming harmonization (`GoalKickOpponent` → `GoalKickAgainst`; will spec-amend `docs/specs/tactic-fsm.md` at the same time).
- **T1-4** owes `TacticEvent` → `MatchEvent` reconciliation; see the TODO comment block on `TacticEvent` in `tactic_fsm.rs` for variant-by-variant mapping notes. Two timer-derived events (`PressTimeoutExpired`, `CounterWindowClosed`) need a Tactic decision: either add timer-tick MatchEvent classes OR move the timeout check into the 2 Hz heartbeat path (would fire at next 2 Hz boundary instead of at the 5s mark — re-audit before signing off).
- **T2-3** owes the dedicated `<Kind>Validator` pattern (one class per content kind) instead of v2's current spread-across-methods validation. Easier to audit.
- **T2-4** owes the 46-label phenotype catalog from FW v1's `design/player-generation.md`. The 55-field ADR-0002 model supersedes the 22-field gene model, but the phenotype labels haven't been ported.

These are encoded in MASTER_PLAN T1-3 + T2-3 done-criteria so they fire when the rows do.

## Queued user actions (Codex audit Tranche 7 cleanup)

Cleanup the user runs at their convenience (NOT auto-applied by Claude — these touch user-global state):

- **Remove Unity MCPs from global `claude mcp list`.** Currently `unity-mcp` + `UnityMCP` are still registered. They were used by FW v1; v2 is Rust-only. Run `claude mcp remove unity-mcp` + `claude mcp remove UnityMCP`. Confirm via `claude mcp list`. Codex audit Lane F P2.
- **Install Claude Preview MCP** if not yet done. When the dev-server prompt fires in Claude Code, click through. Until installed, the `mcp__Claude_Preview__preview_*` tools are loaded as deferred but not exercisable end-to-end. Codex audit Lane F P2. ADR-0008's workflow becomes runnable post-install.
- **(Optional) Install repo-local git pre-commit hook for canonical-hash-guard.** Per ADR-0012 §"Three-layer guard" reconciliation, the `.claude/hooks/canonical-hash-guard.sh` is convenience-only — it only fires for commits made via Claude Code. Adding a repo-local `.githooks/pre-commit` that calls the same script would make layer 3 durable. Not required (layers 1 + 2 are already durable + CI-enforced), but useful if you frequently commit via the terminal. To install: `mkdir -p .githooks && cp .claude/hooks/canonical-hash-guard.sh .githooks/pre-commit && chmod +x .githooks/pre-commit && git config core.hooksPath .githooks`.

## Queued research

### Frontend research wave (run before T1-6, NOT now)

**Trigger:** when T1-2a's PR is in flight (background work while waiting for review). Synthesis must land before `/next` picks up T1-6 (frontend Match route + text recap).

**Headline lens — why FM26 sucked:** FM26 (Football Manager 2026 → "FM26" in player parlance) shipped to widely negative reception on UI/UX specifically. The wave must understand WHY before it can recommend what we do instead. Hypotheses to test: (a) menu depth + nesting; (b) lost information density vs prior FMs; (c) inconsistent navigation grammar; (d) Unity-port artefacts (FM moved engines); (e) "designed for a different player" — gamification at the cost of the management-sim core. The wave must read the actual reviews/forums, not assume.

**Other targets:**
- FM 24 (the last well-received FM): match-day page hierarchy, sidebar density, scrubber affordances, post-match recap layout. The reference for "what was lost in FM26."
- OOTP 25: how dense tabular surfaces stay readable — directly relevant to our TanStack Table v8 management screens.
- FOF (Front Office Football): the "ugly but information-perfect" school — what makes its UI work despite the visuals. Inform our text-first stance.
- EHM (Eastside Hockey Manager): text-first match presentation. Closest sibling to our presentation model.
- Tennis Elbow + PCM (Pro Cycling Manager): niche-sport UI patterns we can crib without inheriting their problems.
- Visual style references: color systems, mono vs sans for stats, table zebra-striping, hover affordances, keyboard nav, player-profile page layouts.

**Deliverable:** `docs/research/frontend/00-synthesis.md` with concrete recommendations for T1-6 + the eventual T4 polish pass. Must explicitly answer "what FM26 got wrong, and what we do instead."

## Open questions

(See `docs/DESIGN_DOC.md` §12 for the gameplay design open questions.)

Technical open questions for T0 follow-up:

1. Pinned hash placeholder — fill on first CI green pass (T0-7).
2. Apple-codesign secrets — defer until T5-1 release pipeline.
3. Tailwind v3 vs v4 — picked v3 for May 2026 stability; revisit at T4.

## Next up — Phase T1 — First Match

Per `docs/MASTER_PLAN.md`:
- T1-1 `fw-content` schema + first RON fixtures (also: Codex Imp #3 buildup_speed_factor f32 → u16 bps conversion)
- T1-2 ball physics + 22-player BT runner (XL — biggest row; consider splitting at task-spec time)
- T1-3 signatures stub (types only)
- T1-4 MatchEvent enum + ledger output
- T1-5 fw-tauri play_match command (also: Codex Imp #10 — src-tauri command consolidation)
- T1-6 frontend Match route + text recap
- T1-7 procedural content stub (Markov names + 2 teams)
- T1-8 replay corpus fixture #1 (smoke seed, 600 ticks, two-archetype matchup)

Acceptance gate for T1: two procedural teams play one match end-to-end; text recap surfaces with goals + score + key events; ≥2 replay-corpus fixtures pin across CI matrix.

Critical path: T1-1 → T1-2 → T1-4 → T1-5 → T1-6.

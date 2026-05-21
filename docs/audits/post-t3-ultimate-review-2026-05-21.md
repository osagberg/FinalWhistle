# Post-T3 Ultimate Review — 2026-05-21

Multi-track adversarial review at the Phase T3 close (`/done` Step 5.5). Claude tracks A–D + Codex tracks E–F. Claude is strong on implementation-drift / test-quality / systemic-pattern detection; Codex is strong on adversarial red-team + property explosion at scale — the convergence between the two halves is the highest-confidence signal.

## Scope

- **Commit range:** `v0.2.0-season..a0b16310` — the full Phase T3 window, 29 commits.
- **Rows in scope (incl. rolled-in T2 rows):** T3-1..T3-7, T3-9, **T2-4**, **T2-7**.
- All subagents READ-ONLY; findings consolidated by the main thread.

## T3 Exit Gate (verified at `/done` Step 3)

| Criterion | Status |
|---|---|
| 5-season career end-to-end; compaction at the 5-season boundary | MET (T3-9; `five_season_career_integration_fast`) |
| A cross-season callback surfaces on a screen | MET (T3-9 `/career` screen) |
| Save format survives one deliberate schema bump | MET (V1→V2, T3-1 + T3-7 fixtures) |
| Vertical-slice tag `v0.3.0-career` | created at this close |

---

## Track A — Claude mutation-test analysis

- **P0 (test-coverage hole)** — `crates/fw-memory/src/ledger.rs` `compact()` boundary `event.season.0 + 5 <= current`: the load-bearing `5` survives a `±1` mutation — no test calls `compact(SeasonNumber(4))` with a season-0 event and asserts it is NOT compacted. Code is correct; the test is mutation-vacuous on the constant.
- **P0 (test-coverage hole)** — `crates/fw-tauri/src/commands.rs` `advance_season_inner` `new_season_num.0 >= 5` compaction guard: a `>=5 → >=4` mutation survives; no test asserts `compaction_fired == false` for seasons 1–4 / `true` at 5.
- **P1** — breakthrough gate: `GATE_MIN_STAKES` floor not independently tested (no "stakes just below 0.50 → gate does NOT fire" case); `determine_positive_kind` Kind-1-over-Kind-2 priority untested (no player with BOTH a matching signature candidate AND a flag).
- **P1** — `emit_title_won_event` sets the champion participant role to `Beneficiary`; mutating it to `Subject` breaks no test.
- **P2** — `project_salience`: every T3 test uses `Tick::ZERO` for both emission + query tick, so the `elapsed <= 0` guard always fires — the Linear/Exponential decay path is unit-tested in isolation but never exercised in the integrated `evaluate`/`SalienceReader::top_n` path.
- **P2** — `compute_ca_delta` integer `/2` truncates; no test pins an exact `delta_ca` for an odd `delta_pa`. **(converges with Track C P3.)**
- **P3** — `season.rs` TitleWon `salience: Q32::ONE` initializer is dead (overwritten by `compute_salience`). UncertaintyBand boundary tests + `observe_player` snapshot are mutation-tight.

## Track B — Claude architectural drift (docs vs code)

Overall docs↔code alignment is tight (ADR-0005 schema, `design/scouting.md`, `design/breakthrough-moments.md`, `design/player-generation.md` all match their crates closely). Drift found:

- **P1** — ADR-0005 references `docs/specs/compaction-strategy.md` (for the rich aggregate-collapsing algorithm) — the file was never created, and the shipped `compact()` is simpler than the ADR's "Compaction is well-defined" prose claims (tick-nulling + a `Compaction` event only, no aggregation). Reconcile the ADR.
- **P1** — ADR-0005 + `event.rs:34` + `Content/RULES.md §3` all say memory migrations live in `fw-content::migrations` — they actually live in `fw-save`; `crates/fw-content/src/migrations/` does not exist. 3 sites to fix.
- **P1** — ADR-0005 reads as though the live "career system" emitter ships in T3, and the Phase T3 *Goal* prose says "breakthroughs fire" — but breakthrough `evaluate` + scout `observe_player` are infrastructure-only, exercised solely by synthetic test harnesses; no played career produces a `BreakthroughMoment`. Honestly disclosed in the T3-9 row, but not top-down. **(converges with Track C P1, Codex E5.)**
- **P2** — ADR-0005 §Consequences/Negative still says "28 event classes" (it is 30); `compute_salience` doc + `event.rs:87` say the 5-term blend "lands at T3-2" (T3-2 shipped the degenerate `stakes`-only form; the blend is deferred to Phase 4).
- **P2** — two independent `NarrativeFlag` enums (`fw_content::gene` + `fw_memory::breakthrough`), same 4 variants, different declaration order, no doc cross-reference — a deliberate dependency-avoidance mirror that is an undocumented contract.
- **P2** — `BreakthroughState` is documented "serialized in the career save" + references a `BreakthroughEvaluator` type that does not exist; `SaveV2` has no breakthrough field. **(converges with Codex E5.)**
- **P3** — `CLAUDE.md §3` "~8 crates" (9 `fw-*` exist); ADR-0005 over-describes `CoachReader`'s shipped scope.

## Track C — Claude whole-codebase silent-failure sweep

The T3 code is "unusually disciplined" — `fw-save`, the 5 `fw-memory` readers, the news/memory-callback renderers, the `fw-tauri` IPC handlers, and `fw-scouting::observe_player` were all swept clean; no determinism leaks (no `HashMap`/clock/`thread_rng`/`f64` in any canonical path). Findings:

- **P1** — `crates/fw-memory/src/breakthrough.rs` `evaluate()` panics on a legitimate career-state boundary: when a player's PA is at the ceiling (200) or at the family floor, the clamped redraw delta is `0`, and `make_breakthrough_event`/`make_regressive_event` `assert!` the delta is strictly signed → panic in debug AND release. Not reachable today (`evaluate` is unwired — see Track B P1), but it will fire the moment T4 connects it to multi-season careers. Fix: guard `if delta_pa > 0` / `if actual_delta_pa < 0` around emission; skip the zero-delta fire. **(converges with Track B P1.)**
- **P2** — `render_memory_callback` (+ `news.rs`): the var-shadowing merge silently deletes any grammar sub-rule whose key collides with a reserved context-var name — a content-authoring footgun with no diagnostic. Fix: reject/`log::warn!` the collision at bank construction.
- **P3** — `fan.rs` recency filter admits future-dated events (no `elapsed < 0` guard, unlike the sibling `project_salience`).
- **P3** — `compute_ca_delta` integer `/2` truncates instead of the documented `0.5 × delta_pa`; the `CA_LIFT_FRACTION` Q32 constant is defined but dead. **(converges with Track A P2.)**

## Track D — Claude test-the-tests

Review agent stalled — its transcript went quiet with no result returned (subagent-infra failure; not blocked-on per the `/done` "do not block the phase close on a stalled review agent" rule). The test-quality lens is substantially covered by **Track A's mutation analysis**, which surfaced the concrete coverage gaps (the two compaction-boundary P0s, the breakthrough-gate P1s, the `Tick::ZERO`-decay-bypass P2). A dedicated test-the-tests pass folds into the `T3-R-C` cleanup row below.

## Track E — Codex adversarial red-team

Full report: `docs/audits/post-t3-codex-gate-2026-05-21.md`. **Verdict: ACCEPT — no T3 gate-blocker.** `scripts/fw verify` PASS.

- **E1 canonical-hash bypass** — negative result. No T3 match-state canonical hash bypass; both pins unchanged; 1000 smoke + 100 extended determinism reruns → one hash.
- **E2 content-pack semantic poisoning** — **P1 carry-forward.** `validate-structural` accepts a pack with a single player-bio (deleted all but `player_00001.ron`, exit 0). No pack-level roster invariant. Fix: a `ContentStore`/baker-level squad-roster validator (exactly-22 bios, or a manifest-backed roster).
- **E3 malicious mod overlay** — negative result; runtime mod overlays not implemented yet (validation must run post-overlay-merge when they land).
- **E4 determinism leak** — negative result. The `RwLock<CareerState>` serialises mutable career state correctly.
- **E5 SaveV2 runtime-career-state gap** — **P1 carry-forward.** `SaveV2` stores `career_seed` + `content_pack_version` + `ledger` only — NOT `season_number` or the active `SeasonState`. A save preserves the ledger but not where the career actually is. Fix: SaveV3 before any save/load UI, including `season_number` + the season state (or a deterministic replay cursor). **(converges with Track B P2 BreakthroughState gap.)**
- **E6 frozen V2 fixture missing** — **P2.** The migration fixture set freezes V0/V1/V99 but no real V2 non-empty-ledger payload. Fix: add `v2_nonempty_ledger_sample.fwsave`.

## Track F — Codex property explosion

No property failures. 10,000-case sweeps PASSED for: `fw-scouting` observation invariants; `fw-memory` append-monotonicity / append-count / compaction / readers / breakthrough; `fw-save` V1/V2 byte-identical round-trips; `fw-replay` encoder field-order invariants; `fw-content` fixture-schedule pair-coverage. Determinism stress: smoke 1000 reruns → one hash; extended 100 reruns → one hash.

---

## Consolidated verdict

**ACCEPT — Phase T3 closes.** Codex returned ACCEPT with no gate-blocker; the locked 4-criterion T3 Exit Gate is met in substance; `scripts/fw verify` is green; 10k-case property sweeps + determinism stress reruns passed across all five determinism-critical crates; canonical match-state hashes are UNCHANGED on both pins throughout the phase.

### Cross-track convergence (the high-confidence signal)

1. **The breakthrough + scout subsystems are infrastructure-complete but UNWIRED** — Track B (P1: ADR + Goal read as shipped; no career-loop emitter), Track C (P1: `evaluate` zero-delta panic that "will fire the moment T4 connects it"), Track A (P1×2: breakthrough-gate coverage holes), and adjacent to Codex E5 (`BreakthroughState` is part of the unpersisted runtime state). This is the #1 systemic finding: a top-down reader mistakes infrastructure-complete for feature-complete. The wiring itself is genuine T4 work (it needs the per-club career-roster); the **`evaluate` zero-delta panic is a real latent bug fixable now**.
2. **SaveV2 does not persist runtime career state** — Codex E5 (P1) + Track B P2 (`BreakthroughState` not in SaveV2). A save remembers the story but not where the career is.
3. **`compute_ca_delta` integer `/2` ≠ the documented `0.5 ×`** — Track A P2 + Track C P3, independently.
4. **Stale-doc cluster** — Track B (ADR-0005 compaction-strategy.md / migrations-location / "28 event classes"; `compute_salience` "T3-2").

### Findings classification

- **Gate-blocker:** NONE.
- **Pre-T4 recommended** (new `T3-R` rows below — land before T4 user-facing polish):
  - T3-R-A doc reconciliation; T3-R-B breakthrough `evaluate` panic-fix + gate tests + `compute_ca_delta`; T3-R-C compaction/advance_season boundary tests + frozen V2 fixture; T3-R-D pack-level roster validation; T3-R-E SaveV3 career-state persistence; T3-R-F career clock for salience.
- **Genuine T4 work** (not a cleanup row): wiring breakthrough + scout into a played career — depends on the T4+ career-roster layer.

The phase tag `v0.3.0-career` is created at this close. The `T3-R-A..F` cleanup cluster is added to `docs/MASTER_PLAN.md` and lands via `/next` before T4's first user-facing-polish row.

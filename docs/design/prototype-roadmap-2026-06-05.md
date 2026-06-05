# Prototype Roadmap + Consolidated Backlog (2026-06-05)

> Plan-of-record for the owner-approved 5-track execution model
> (DECISIONS 2026-06-05; commit `7a693dde`). Produced by the
> `fw-prototype-scoping` workflow (6 agents: backend-loop / frontend /
> desktop-build / findings x2 audits + Opus synthesis). This doc is the
> durable reference; the full 179-item backlog table is at the bottom.

---

## The headline

Most of the playable loop **already exists and returns real data**:
`get_roster_for_club`, `get_standings`, `get_fixtures`, `advance_week`
(AI-sims all 10 fixtures + harvests memory + scouting), `get_press_inbox`,
`get_career_overview`, `advance_season`, `save_career` / `load_career`
(SaveV4) are all real. The **only** thing blocking "you can play it" is that
a career cannot be *started* or *anchored to a club*: there is no
`new_career` command, no managed-club concept, no `get_clubs`, and the Home
NEW CAREER / LOAD SAVE buttons are hard-disabled. That gap is small.

The prototype is a **spectator-manager loop**: start a career, pick a club,
advance week-by-week through a season watching results / standings / press
accumulate, roll seasons, save/load. Playing your *own* fixture in the live
tactical board, setting tactics, and transfers are explicitly **out** of the
minimal prototype (they deepen later) — the loop runs on `advance_week`'s
AI-sim result, which is real today.

---

## The prototype (Track E milestone)

**Definition.** From Home: NEW CAREER → choose seed/name + pick one of the
20 generated clubs → Squad screen shows that club's real 22-slot roster →
league table + fixture list → advance week-by-week (AI sims the round) →
read press inbox + career overview across seasons → save/load. A complete
loop built almost entirely on commands that already return real data.

**Build path (macOS .app).** No blockers per the build audit — icons exist,
`signingIdentity: null` (no Apple cert needed for a local build), tauri-cli
2.11.1 vendored, Rust 1.95 matches `rust-toolchain.toml`, `content/sources/`
present, desktop schema committed.

- Pre-flight (once, must be green): `cargo build --workspace` then
  `scripts/fw verify`.
- Produce the .app: `pnpm install && just bundle`
  (= `pnpm install --frozen-lockfile && pnpm --filter ./frontend build &&
  pnpm exec tauri build`; alias `scripts/fw bundle`). Bundle id
  `com.vibelogic.finalwhistle`.
- Fast dev-run with HMR (no .app): `just dev` / `scripts/fw dev`.
- Caveat: building the .app today produces an app whose NEW CAREER button is
  still disabled — it is only playable once B1–B4 land.

---

## Execution model — 5 tracks, conductor-gated

Claude is **conductor + canonical gatekeeper**. Discovery and non-canonical
work fan out via dynamic workflows (Sonnet fan-out, Opus synthesis — never
Opus-1M fan-outs). Canonical and integrity-critical merges stay **serial
through Claude** with independent re-measure + adversarial diff review +
trace-claims-to-output on every behavioural DONE flip.

| Track | What | Gate |
|---|---|---|
| **A — Engine** | match-sim believability (drift-goals, lanes, cross, physics, FSM, signatures) | canonical hash + drama-sweep; **strictly one in flight** |
| **B — Loop** | the playable career loop (prototype spine) | pnpm + integration tests; mostly non-canonical |
| **C — Findings** | every review finding (correctness, health, data-flow) | per-finding verify; canonical ones route through the A-gate |
| **D — Visuals** | body-orientation canonical field + match-frame DTO contract hygiene + 2D board | mostly additive; BK-V-2 is canonical |
| **E — Prototype** | runnable macOS Tauri build of the loop | `just bundle` green + manual run |

**The hard rule:** exactly ONE canonical-touching task is in flight at a
time and it owns the BLAKE3 pins (60-tick + 600-tick) until it rebaselines
and the CI matrix is green. Everything else parallelizes freely.

---

## Phased execution order

**PHASE 0 (serial, unblocks all).** Confirm clean baseline: `cargo build
--workspace` then `scripts/fw verify` green before any new work. (Tree is
already clean; the just-committed sim/test changes must compile + pass.)

**PHASE 1 — PROTOTYPE FIRST (mostly serial, small).** Land B1→B2→B3→B4 in
order, fold in BK-E-5 (SaveLoadFailed TS union — already shipped in
`eba6b617`) before B5, then build the .app and hand the owner a runnable
prototype.

- **B1** `new_career(seed, name?)` — fw-tauri, **non-canonical**: construct
  AppState via the existing `AppState::new_with_career_seed`.
- **B2** `get_clubs` / `list_clubs` — fw-tauri, **non-canonical**: enumerate
  the 20 generated clubs (id + name).
- **B3** managed-club anchor — **handled carefully**: hold `managed_club_id`
  in the in-memory `AppState` session (set by `new_career` /
  `select_managed_club`, read by `get_squad_roster` to replace the
  lowest-ClubId placeholder). This **avoids a SaveV4→V5 schema bump**.
  Persisting the managed club across save/load (the proper SaveV5 migration,
  4-test owe) is **flagged for the owner** as a §8 schema-bump decision — not
  done autonomously.
- **B4** frontend — enable Home NEW CAREER + LOAD SAVE; add a thin
  club-selection screen wiring `get_clubs → new_career/select_managed_club →
  /squad`; populate sidebar career-context + managed-club.
- **B5/B6** wire LOAD SAVE → `load_career`; populate sidebar Season +
  Next-fixture.

**PHASE 2 — fan out four lanes once the prototype runs.**

- **Lane A (CANONICAL, STRICTLY SERIAL):** drift-goals cluster
  (BK-E-6/7/49) → lane_openness (BK-E-10) → negative-tick assert (BK-E-4) →
  **body-orientation (BK-V-2, the visuals door)** → archetype.formation
  (BK-E-52) → cross-gate-by-zone (BK-E-47) → loose-ball contest (BK-E-50) →
  ball-leaves-ground (BK-E-51) → clock+score+FSM (BK-E-53/54/55) → signatures
  (BK-E-75/57/56/65) → home/away+fitness (BK-E-76/77) → Tick::ZERO breakthrough
  (BK-E-12) → remaining canonical C-findings. **Each repins; CI matrix green
  before the next starts.**
- **Lane B (non-canonical findings + loop depth, parallel):** baker stub,
  load_career-not-panic, atomic writes, the missing EventClass emitters, then
  loop depth (per-fixture result, tactics persistence, transfers).
- **Lane C (harness + strategy, parallel, mostly non-code):** **land the P0
  harness gates BK-H-1..5 FIRST** (they govern Lane A honesty), then
  drama_sweep/inspect_frames into CI, then doc-hygiene + strategy logs.
- **Lane D (visuals contract + 2D board, parallel except BK-V-2):**
  DTO-contract hygiene (additive-only) early, then the cheap 2D-board wins
  (possession highlight, ball height, depth sort), then polish.

---

## Risks (from the synthesis)

- **Build risk LOW but unconfirmed:** the audit verdict was "likely" — a
  clean `cargo build --workspace` was never run in-session. PHASE 0 must
  confirm it before bundling.
- **B3 schema hotspot:** adding `managed_club_id` to the save envelope forces
  a SaveV4→V5 bump (4 migration tests). Mitigation: AppState-session field
  for the prototype (above); persistence flagged for owner.
- **Canonical serialization is a single contended resource:** ~20
  canonical-touching tasks each repin BLAKE3 across macOS/Win/Linux. Two in
  parallel = un-attributable drift. Enforce one-in-flight.
- **Masking-prone slices:** drift-goals, lane_openness, cross-gate, any
  shot/goal-rate work — a classifier+counter rate-floor can pass while the
  mechanism is mislabeled (two such defects already caught). These ship ONLY
  with the harness P0 gates in place: raw 4-metric tuple + claim-trace +
  before/after drama-sweep in the commit body.
- **Believability-vs-playability relapse:** without a logged stopping rule
  (BK-S-2) the session can drift back to engine-only. Prototype-first +
  the stopping rule address it.
- **drama_sweep flakiness (#26):** std sub-guard FAILs at <100 seeds. Pin any
  CI invocation to ≥100 seeds + `--vary-quality true` together.
- **load_baked() is a stub:** prototype relies on procgen rosters (fine);
  any depth assuming a real baked corpus hits the stub — separate track.

---

## Owner-decision items (FLAGGED, not decided autonomously)

These are governing but non-code; surfaced for the owner, not silently picked:

- **SaveV5 managed-club persistence** — schema bump (§8) to remember the
  managed club across save/load. Prototype works without it.
- **BK-V-19 / Fork B** — keep text-first/2D (Fork A) or commit to opening the
  2.5D lane (Fork B). Body-orientation (BK-V-2) keeps the door open cheaply
  either way; the owner already said "do body orientation for sure."
- **BK-S-4** — positioning: procedural-fantasy vs licensed-data market risk.
- **BK-S-5** — signature-identity (24 signatures) investment vs management depth.
- **BK-H-22..26** — Codex cadence, integrity-role, management-metagame CI gate,
  MASTER_PLAN cap, FUN-rows-as-TODOs.

---

## Consolidated backlog

179 items, generated below from the scoping synthesis. Tracks: A-engine /
B-loop / C-findings / D-visuals / E-prototype / harness / strategy.
`canon` = touches canonical match state (routes through the A-gate).

| ID | Sev | Track | Canon | Title |
|---|---|---|---|---|
| BK-FE-1 | P0 | B-loop | - | Home NEW CAREER button hard-disabled (no new-career entry point) |
| BK-FE-3 | P0 | B-loop | - | Club-selection screen does not exist anywhere in the router |
| BK-LOOP-1 | P0 | B-loop | - | No new_career command — world seed hardcoded DEFAULT_CAREER_SEED, every session loads the same world |
| BK-LOOP-2 | P0 | B-loop | Y | No managed-club concept on CareerState; get_squad_roster returns lowest ClubId placeholder |
| BK-LOOP-3 | P0 | B-loop | - | No get_clubs / list_clubs command for a club-selection screen |
| BK-H-1 | P0 | harness | - | Independent re-measure mandatory gate on every behavioral DONE flip (raw 4-metric tuple in commit body) |
| BK-H-2 | P0 | harness | - | Claim-trace step in /next Step 6 for every claimed mechanism (trace read-line to MatchEvent/score) |
| BK-H-3 | P0 | harness | - | Mechanism-vs-outcome pairing required in AC-to-test matrix; ban same-task classifier+counter floors |
| BK-H-4 | P0 | harness | - | Main-thread independent re-measure for any behavioral-metric commit |
| BK-H-5 | P0 | harness | - | Structural honesty gate: before/after drama-sweep comparison in commit body |
| BK-S-1 | P0 | strategy | - | Build a playable game loop — metagame is absent from code (Path B rebalance) |
| BK-E-10 | P1 | A-engine | Y | lane_openness computed then dead-dropped — pass completion omits the promised factor |
| BK-E-6 | P1 | A-engine | Y | GK save gated on xg_score>0 — ~29% of goals are uncontested drift goals with no shot cause |
| BK-E-7 | P1 | A-engine | Y | GK save emits no MatchEvent — saves invisible to commentary/replay/momentum |
| BK-V-2 | P1 | A-engine | Y | Add player body facing/orientation field to canonical PlayerState (visuals door open) |
| BK-E-1 | P1 | B-loop | - | 25 of 30 EventClass variants never emitted in production career loop |
| BK-FE-2 | P1 | B-loop | - | Home LOAD SAVE button hard-disabled despite real load_career command |
| BK-FE-4 | P1 | B-loop | - | Tactics screen (/tactics) is a full placeholder — no IPC, no controls |
| BK-LOOP-4 | P1 | B-loop | - | start_live_match/play_match/match_frames use DEFAULT_ARCHETYPE_ID both sides — cannot play the player's real fixture |
| BK-LOOP-5 | P1 | B-loop | - | apply_match_command: all 9 manager-intent variants return LiveMatchCommandUnimplemented |
| BK-LOOP-7 | P1 | B-loop | - | No get_tactics/set_tactics/get_formation/set_formation — archetypes random + immutable via IPC |
| BK-LOOP-8 | P1 | B-loop | - | No transfer commands (market/offer/accept/release) — roster fixed for whole career |
| BK-E-11 | P1 | C-findings | - | assert!/assert_eq! panics inside async load_career_inner for corrupted saves — should return IpcError |
| BK-E-12 | P1 | C-findings | Y | advance_season_inner feeds Tick::ZERO to breakthrough evaluate() — salience decay silenced |
| BK-E-2 | P1 | C-findings | - | fw-content news.rs render_with_vars passes empty strings to Tracery — NewsRenderError on empty field |
| BK-E-3 | P1 | C-findings | - | fw-content-baker stub_unimplemented returns Ok(()) — 7 bake subcommands exit zero silently |
| BK-E-4 | P1 | C-findings | Y | debug_assert! on negative tick in should_decide guards real silent failure (Sim RULES §11) |
| BK-E-5 | P1 | C-findings | - | IpcError::SaveLoadFailed missing from TS union — save/load errors fall to generic handler |
| BK-E-9 | P1 | C-findings | Y | GK SS3 save model: 130+ lines canonical logic, zero dedicated unit tests |
| BK-E-8 | P1 | D-visuals | - | App() called as plain function inside JSX — Solid reactive ownership broken |
| BK-V-1 | P1 | D-visuals | - | Add per-stream schemaVersion fields to all DTO contracts (frame/event/meta) |
| BK-H-6 | P1 | harness | - | Tier-2 Codex audit trigger on masked-regression or three-attempts catch |
| BK-S-2 | P1 | strategy | - | Add a stopping rule on believability-first to prevent continued match-engine-only drift |
| BK-S-3 | P1 | strategy | - | Log the Path B rebalance decision (metagame + engine in tandem) |
| BK-V-19 | P1 | strategy | - | Owner: product-vision fork — keep text-first/2D (Fork A) or open 2.5D lane (Fork B) |
| BK-E-16 | P2 | A-engine | Y | FUN-TS1 shuffle-toward-ball unimplemented — block_centroid_y computed and dead-dropped |
| BK-E-47 | P2 | A-engine | Y | Cross gate width-only — deep-wide fullback can cross from own half |
| BK-E-49 | P2 | A-engine | Y | Defenders do not clear a ball rolling toward own goal — second half of drift-goals |
| BK-E-50 | P2 | A-engine | Y | Loose-ball resolution is pure geometric race with no attribute contest or 50-50 roll |
| BK-E-51 | P2 | A-engine | Y | Ball never leaves ground — vel_z hardcoded Q32::ZERO, gravity/Magnus/bounce dead code |
| BK-E-52 | P2 | A-engine | Y | All teams play hardcoded 4-3-3 — initial_with_content never reads archetype.formation |
| BK-E-53 | P2 | A-engine | Y | Score + clock never reach tactic FSM — late-match decisions identical to early; is_second_half hardcoded false |
| BK-E-54 | P2 | A-engine | Y | CounterAttack FSM state inert — BTs never read tactic_state in possession |
| BK-E-55 | P2 | A-engine | Y | Set-piece restart mis-awards possession — nearest-body pickup instead of rule-based |
| BK-E-57 | P2 | A-engine | Y | Signature candidate routing excludes GK/DEF/FWD slots — 5 of 8 triggers unreachable |
| BK-E-65 | P2 | A-engine | Y | Signature readiness resets to zero every match — never seeded from career state |
| BK-E-75 | P2 | A-engine | Y | Signature thresholds universally 0.45 — every eligible player fires on almost any tick |
| BK-E-76 | P2 | A-engine | Y | MatchState carries no home/away club IDs or environmental context — match is symmetric |
| BK-E-77 | P2 | A-engine | Y | match_fitness has no writer or reader — in-match fatigue + cross-match carry-forward absent |
| BK-E-15 | P2 | B-loop | - | Scout Pillar-4 truth-narrowing hollow — each observation independent uniform draw ignoring prior |
| BK-E-25 | P2 | B-loop | - | SEASON_MATCH_TICK_BUDGET=600 undercounts LegacyGoal — most season matches produce zero goals |
| BK-E-56 | P2 | B-loop | Y | SignatureActivated consequence never consumed — activated signatures stay candidates forever |
| BK-E-58 | P2 | B-loop | - | Biased scout archetypes are enum variants with no impl — all scouts produce neutral fog |
| BK-E-64 | P2 | B-loop | Y | Match-to-season feedback absent — MatchEvent stream discarded, no MatchSummary |
| BK-E-67 | P2 | B-loop | - | Ledger O(n) readers degrade silently over multi-season careers at 96-club scale |
| BK-E-69 | P2 | B-loop | Y | PlayerId namespace has no allocation plan for newgens/transfers — ledger collision risk |
| BK-E-72 | P2 | B-loop | - | Ledger readers emit ranked EventId lists nothing renders — no EventId->prose bridge |
| BK-E-73 | P2 | B-loop | - | TitleWon not attributed to individual players — no per-player ledger trace or meter signal |
| BK-E-74 | P2 | B-loop | - | Salience degenerate — all events stamped stakes=Q32::ONE, compute_salience is identity |
| BK-E-78 | P2 | B-loop | - | Procgen is name-generator only — ContentStore.load_baked() is a stub returning Ok(default) |
| BK-FE-5 | P2 | B-loop | - | Transfers screen is a stub — only a window-state pill, no mechanics |
| BK-FE-8 | P2 | B-loop | - | Sidebar Career context shows 'No career active.' with no path to set selectedClubId |
| BK-LOOP-6 | P2 | B-loop | - | No per-fixture match-result detail command after advance_week |
| BK-E-13 | P2 | C-findings | - | TitleWon event silently dropped when standings rows unexpectedly empty |
| BK-E-14 | P2 | C-findings | - | CultureWeights::first_alpha_diversity_bps loaded/stored but never read by name sampler |
| BK-E-17 | P2 | C-findings | Y | ONE_OVER_199 constant off by one raw bit with wrong comment |
| BK-E-18 | P2 | C-findings | Y | to_q32_seconds silently truncates via 'as i32' in release — should use checked cast |
| BK-E-19 | P2 | C-findings | - | PlayerBio.player_id not cross-validated against player_templates at load |
| BK-E-20 | P2 | C-findings | - | ContentError enum in fw-content lib.rs is dead code — real type is ContentLoadError |
| BK-E-21 | P2 | C-findings | - | design/scouting.md stale after F2 fix — wrong signature, seed site, type |
| BK-E-22 | P2 | C-findings | - | observe_player does not guard on scout.kind — non-Basic scouts silently run Basic algorithm |
| BK-E-23 | P2 | C-findings | - | SalienceReader::BySubject claims O(log n) but does full O(n) linear scan |
| BK-E-24 | P2 | C-findings | - | Consequence additive variants claim safe old-save decode with no fixture test |
| BK-E-26 | P2 | C-findings | - | fw-save fixture README + lib.rs comment stale — V4 undocumented, listed as V3 |
| BK-E-27 | P2 | C-findings | - | Proptest round-trip covers only SaveV1/V2 — V3/V4 serde surfaces untested |
| BK-E-28 | P2 | C-findings | - | No direct migrate_v3_to_v4 test — only tested through load_envelope |
| BK-E-29 | P2 | C-findings | - | ADR-0010 save wire format (FWS1 magic + zstd) does not match implementation — still Proposed |
| BK-E-30 | P2 | C-findings | - | fw-content-baker performance_delta CurvePoint has no range check — unbounded multiplier passes validate |
| BK-E-31 | P2 | C-findings | - | MVP_ROSTER_SIZE=22 exact-match constant will break CI when bio 23 is added |
| BK-E-32 | P2 | C-findings | - | Content/RULES §8 milestone label + NotImplemented defer_to still say T2-3 |
| BK-E-33 | P2 | C-findings | - | MatchEventKind TS union contains phantom variants (HalfTime, Card, Substitution) sim never emits |
| BK-E-34 | P2 | C-findings | - | FinalMatchResult.total_events guarded with isU32 but field is u64 on the wire |
| BK-E-35 | P2 | C-findings | Y | Offside proptest statistical-only — five named geometric cases uncovered |
| BK-E-36 | P2 | C-findings | - | PlayerSeasonStats.goals accumulation has zero test coverage |
| BK-E-37 | P2 | C-findings | - | calibrate_smoke_test shot floor is 10 across 5 matches — 80% degradation undetected |
| BK-E-38 | P2 | C-findings | Y | setpiece_state_auto_exits test weakened to accept MidBlock — correctness guard reduced |
| BK-E-39 | P2 | C-findings | Y | fw-core: no proptest on seed_fn determinism, SeedLayer collision absence, Q32 overflow |
| BK-E-40 | P2 | C-findings | Y | seed_fn pinned vector test tautological — computes expected at runtime not golden u64 |
| BK-E-41 | P2 | C-findings | - | save_career_inner + set_settings_inner write files non-atomically — partial write loses save |
| BK-E-42 | P2 | C-findings | - | TOCTOU race in get_settings_inner — path.exists() then fs::read |
| BK-E-48 | P2 | C-findings | - | FUN-TS2 MASTER_PLAN row over-claims cover-shadow + last-defender offside line — both undelivered |
| BK-E-66 | P2 | C-findings | Y | CLUBS_PER_LEAGUE fixed-size array [ClubId;20] — pyramid refactor needs multi-crate blast |
| BK-E-68 | P2 | C-findings | - | content_pack_version hardcoded to 1 — no guard vs silent corruption on post-EA patches |
| BK-E-70 | P2 | C-findings | - | Name collision near-certain at 2000+ procgen players |
| BK-E-43 | P2 | D-visuals | - | ECharts imported at module top level in Stat.tsx — violates lazy-import rule |
| BK-E-44 | P2 | D-visuals | - | createEffect nested in onMount in Stat.tsx risks leaked resize listener on HMR |
| BK-E-45 | P2 | D-visuals | - | Five local copies of isIpcError duplicate the canonical one in route-errors.ts |
| BK-E-46 | P2 | D-visuals | - | Unsafe discriminated-union 'as' casts in ScoutSection + PressInboxSection bypass TS narrowing |
| BK-V-10 | P2 | D-visuals | - | Add golden frame fixture per frameSchemaVersion as JSON round-trip regression net |
| BK-V-11 | P2 | D-visuals | - | Kill dev/prod pitch-constant duplication in TacticalBoard components |
| BK-V-12 | P2 | D-visuals | - | Possession highlight + carrier-to-ball tether on the 2D board |
| BK-V-13 | P2 | D-visuals | - | Ball height: lifted dot + ground shadow ellipse on the 2D board |
| BK-V-18 | P2 | D-visuals | - | Switch live match from per-call sim re-run to Tauri Channel API streaming |
| BK-V-3 | P2 | D-visuals | - | Un-drop ball spinX/Y/Z at the DTO boundary (already canonical, not projected) |
| BK-V-4 | P2 | D-visuals | - | Add event primarySlot/secondarySlot, posX/posY, endX/endY, outcome to MatchEventDto |
| BK-V-5 | P2 | D-visuals | - | Add phase enum (in/out/transition/dead-ball) to frame DTO |
| BK-V-8 | P2 | D-visuals | - | Make canonical->frame projection a stable named versioned Rust type (renderer-neutral) |
| BK-V-9 | P2 | D-visuals | - | Adopt per-stream versioning rules: additive-only at boundary, tolerate-unknown on consumer |
| BK-E-59 | P2 | harness | - | Believability co-equal gate wired nowhere — drama_sweep not in CI or scripts/fw verify |
| BK-E-60 | P2 | harness | - | inspect_frames glitch-coherence gate always exits SUCCESS — never called from CI |
| BK-E-61 | P2 | harness | - | anti_script_suspicious guard warn-only — not in all_guards_pass, no CI enforcement |
| BK-E-62 | P2 | harness | - | drama_sweep defaults --vary-quality false — gate measures mirror teams that cannot fail |
| BK-E-63 | P2 | harness | - | S2 upset-frequency metric only in prose — no impl, no test, no SweepReport field |
| BK-E-71 | P2 | harness | - | Full-sim perf gate tests 600-tick proxy not 5400-tick production budget — 36x gap |
| BK-E-79 | P2 | harness | - | drama_sweep exit-code contract has no regression test — silent bypass risk |
| BK-H-10 | P2 | harness | - | Rewrite CLAUDE.md §6 to match ADR-0015's 3-tier Codex policy |
| BK-H-11 | P2 | harness | - | Update CLAUDE.md §1 + MASTER_PLAN T5 to the no-EA dynamic roadmap (remove $20 EA pricing) |
| BK-H-12 | P2 | harness | - | Acknowledge two-level verify pattern in CLAUDE.md §4.1 and §9 |
| BK-H-13 | P2 | harness | - | Run /audit as session-start gate, not just on demand |
| BK-H-14 | P2 | harness | - | Formalize probe-and-back-out as a first-class workflow concept |
| BK-H-15 | P2 | harness | - | Retire /done phase-gate cadence for the dynamic FUN-track (milestone tags + selective Codex) |
| BK-H-16 | P2 | harness | - | Trust subagent-reported drama-sweep; do not re-run 40 seeds in main thread |
| BK-H-17 | P2 | harness | - | Split 60-tick pin into a pure cross-platform-integrity role |
| BK-H-18 | P2 | harness | - | Add a third pinned corpus seed: full 5400-tick match as a pinned fixture |
| BK-H-19 | P2 | harness | - | Resolve fw-tauri 6th-pin documentation inconsistency |
| BK-H-20 | P2 | harness | - | Deferred-Codex-P1 tracking discipline (becomes MASTER_PLAN row within one /next cycle) |
| BK-H-7 | P2 | harness | - | Demote DESIGN_DOC.md + MEMORY.md from mandatory per-session read list |
| BK-H-8 | P2 | harness | - | Trim MEMORY.md to under 200 live lines |
| BK-H-9 | P2 | harness | - | Split MASTER_PLAN: delivery table only, DONE post-mortems to ship-records |
| BK-BUILD-1 | P2 | strategy | - | Clean cargo build --workspace not confirmed in audit session (dirty files now committed; verify pre-bundle) |
| BK-H-22 | P2 | strategy | - | Owner: decide Codex cadence at sub-phase milestones vs phase boundaries only |
| BK-H-23 | P2 | strategy | - | Owner: introduce a thin integrity-check role or fold output-tracing into qa-lead |
| BK-H-24 | P2 | strategy | - | Owner: standing management-metagame depth CI gate, symmetric to believability gate |
| BK-S-4 | P2 | strategy | - | Owner: positioning call on procedural-fantasy vs licensed-data market risk |
| BK-S-5 | P2 | strategy | - | Owner: weigh signature-identity (24 signatures) investment vs management depth |
| BK-E-106 | P3 | B-loop | Y | Breakthrough age_years hardcoded to 22 for all players — age-curve modifier never applied |
| BK-E-90 | P3 | B-loop | - | AfterCompaction callback routing defined+routed but never emitted in production |
| BK-FE-6 | P3 | B-loop | - | Player Contract block is a deferred placeholder |
| BK-E-101 | P3 | C-findings | - | STATUS.md reports stale FUN-CB1 canonical hashes after FUN-TS3b rebaseline |
| BK-E-102 | P3 | C-findings | - | schemas.rs in fw-content-baker is entirely dead code — silenced with file-level allow(dead_code) |
| BK-E-103 | P3 | C-findings | - | fw-save perf test targets SaveV2 not production SaveV4 — measured path not a real cycle |
| BK-E-80 | P3 | C-findings | Y | Q32Inner reachable despite 'not re-exported' claim — bypasses checked-operator policy |
| BK-E-81 | P3 | C-findings | - | seed_fn Decision-layer site doc wrong — says player_id but call sites use player_slot |
| BK-E-82 | P3 | C-findings | - | generate_fixtures doc duplicates Panics section verbatim |
| BK-E-83 | P3 | C-findings | - | Commentary-load comment says '6 discriminants' but loop checks 8 |
| BK-E-84 | P3 | C-findings | - | Label-confidence interval notation mismatch between scouting.md and observe.rs |
| BK-E-85 | P3 | C-findings | - | GeneCategoryEstimate.low/high are pub — try_new invariant bypassable |
| BK-E-86 | P3 | C-findings | Y | days_since saturating_sub in breakthrough.rs has no test pinning saturate-to-0 edge |
| BK-E-87 | P3 | C-findings | - | content_pack_version stored, hardcoded to 1, never validated — deferral mistaken for guard |
| BK-E-88 | P3 | C-findings | - | mod_load_fingerprint in RULES+ADR-0010 absent from every SaveVN |
| BK-E-89 | P3 | C-findings | - | procgen::generate_team ignores naming_pattern — inconsistent name render vs sample_player_name |
| BK-E-91 | P3 | C-findings | Y | debug_assert! on roster_slot range in decision_cadence.rs missing §11 documentation |
| BK-E-92 | P3 | C-findings | - | Season-number guards use isU32 for u16-backed DTO fields — inconsistent |
| BK-E-94 | P3 | C-findings | Y | fw-content gene->attribute compiler: no proptest on ceiling enforcement or monotonicity |
| BK-E-95 | P3 | C-findings | - | emit_season_end_events is dead code — unreachable since borrow-checker forced direct call |
| BK-E-96 | P3 | C-findings | - | Unknown commentary grammar stems silently continue — misnamed file skipped without hard error |
| BK-E-97 | P3 | C-findings | - | MemoryCallbackLoadError::InvalidBank erased to TraceryParse — authoring bug indistinguishable |
| BK-E-98 | P3 | C-findings | - | Unmatched save deltas silently reset player progression on content-pack change |
| BK-E-100 | P3 | D-visuals | - | DataTable hardcodes aria-label='Data table' for every instance |
| BK-E-99 | P3 | D-visuals | - | Legacy playerNameCell in squad.columns.ts uses document.createElement bypassing Solid tree |
| BK-FE-7 | P3 | D-visuals | - | Sidebar Season + Next-fixture fields display literal dashes, never populated |
| BK-V-14 | P3 | D-visuals | - | Depth sorting on the 2D board (zIndex=screenY) |
| BK-V-15 | P3 | D-visuals | - | Finish pitch furniture: goal frames, 6-yard box, penalty arcs, corner/centre arcs |
| BK-V-16 | P3 | D-visuals | - | Sprite billboards + velocity-facing on the 2D board |
| BK-V-17 | P3 | D-visuals | - | Camera (pan/zoom/follow-the-ball) on the 2D board |
| BK-V-20 | P3 | D-visuals | - | Adopt features:[...] capability list in metadata header instead of version arithmetic |
| BK-V-21 | P3 | D-visuals | - | Borderless window + custom chrome (titleBarStyle overlay + drag region) |
| BK-V-22 | P3 | D-visuals | - | Visual identity overhaul: escape the generic-AI look (typography, palette, density) |
| BK-V-23 | P3 | D-visuals | - | Add motion language via solid-motionone (120/220/400ms, ease-out/spring) |
| BK-V-24 | P3 | D-visuals | - | Cohesive UI sound kit: shared AudioContext, gesture-gated, family of three sounds |
| BK-V-25 | P3 | D-visuals | - | Boot ceremony: Tauri splash window masking real load, dismissed on ready event |
| BK-V-6 | P3 | D-visuals | - | Add period field to frame + event DTOs for stoppage/ET/shootout readiness |
| BK-V-7 | P3 | D-visuals | - | Declare reserved unfilled slots: actionState, accel, event subType, ball spin precision |
| BK-E-104 | P3 | harness | - | gate bands hardcode-duplicate drama-model.md with only a comment as sync contract |
| BK-E-105 | P3 | harness | - | S1/S3/S4 season-level drama metrics absent from SweepReport — 25% of composite index zeroed |
| BK-E-93 | P3 | harness | - | LUT drift-detection tests are #[ignore]-gated and never run in CI |
| BK-H-21 | P3 | harness | - | Replace boilerplate-as-safety-net with AC-matrix-before-coding enforcement |
| BK-H-25 | P3 | strategy | - | Owner: recalibrate MASTER_PLAN item-count cap to TODO/IN-PROGRESS only, or retire it |
| BK-H-26 | P3 | strategy | - | Owner: FUN-track rows as first-class MASTER_PLAN TODOs vs STATUS-only |


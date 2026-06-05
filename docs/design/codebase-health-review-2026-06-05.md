# Codebase Health Review — 2026-06-05

Status: REVIEW — read-only sweep 2026-06-05; findings to triage, not yet actioned.

## Scope and method

A read-only health sweep across the Rust workspace (`fw-core`, `fw-match-sim`, `fw-content`,
`fw-content-baker`, `fw-memory`, `fw-replay`, `fw-save`, `fw-scouting`, `fw-tauri`), the
frontend, and the two cross-cutting boundaries that bind them (determinism, IPC). Thirteen
areas were read against the code, not the roadmap prose.

The sweep leads with a **marked-DONE-vs-actually-delivered integrity** section because that is
the highest-signal class — it extends the pattern Codex started at the post-T1 ultimate review
and again at the 2026-06-05 believability pass, where two green-tested findings hid a model
less honest than its narrative (MEMORY: "trace claimed terms to output"). A row marked DONE
whose shipped surface is narrower, stubbed, computed-then-discarded, or renamed is a structural
debt that no test catches, because the test was written to the narrower surface.

Severity is P0 (ship-blocker) / P1 (correctness or integrity, fix before the next phase gate) /
P2 (real gap, schedule it) / P3 (drift, doc, or smell). No P0 surfaced.

### Dedup — deliberately out of scope

Owned by the believability-arc review (`docs/reviews/codex-believability-arc-review.md`) and the
football-authenticity gap map (`docs/design/football-authenticity-gap-map.md`); **not**
re-litigated here:

- Match-feel drift goals (goal-production realism, momentum/salience/signature surfacing).
- Behaviour-tree lane-openness and lane-evaluation as a *believability* gap — but the structural
  fact that `lane_openness` is computed and dead-dropped with no wiring test IS reported here
  (§5, test-health), because that is an integrity/test surface, not a feel-tuning surface.
- Cross-gate / cross-phase believability sequencing.
- Offside *modelling* and its commentary feel — but the absence of the five named geometric unit
  tests IS reported here (§5), as a test-coverage fact.

Where a finding brushes the believability docs, it is reported only at the structural/integrity
altitude and the feel question is deferred.

---

## 1. Marked-DONE vs delivered — integrity (highest signal)

Nine findings. One P1, six P2, two P3. These extend the Codex integrity pattern: each is a
MASTER_PLAN row, STATUS claim, or in-code "done" whose delivered surface is narrower than the
claim, or a knob that is loaded and stored but never read.

### 1.1 [P1] 25 of 30 `EventClass` variants are never emitted in the production career loop
`crates/fw-memory/src/event.rs:238-323` (30 variants defined) vs
`crates/fw-tauri/src/season.rs:111,320,408,880` + `commands.rs:2930` (only five ever emitted).

The production career loop emits exactly five classes: `DebutSenior`, `LegacyGoal`, `TitleWon`,
`RegressiveCollapse`, and `BreakthroughMoment` (via `evaluate()`). The other 25 —
`HatTrickScored`, `BigMatchScar`, `RivalryFormed`, `MentorTeammate`, `DerbyControversy`,
`CupFinalWin/Loss`, `PromotionWon`, `RelegationSuffered`, `InjuryLongTerm`, `InternationalCallUp`,
`BrokenPromise`, `SoldUnderProtest`, the contract/transfer arc, and the rest — exist only in
`fw-memory` tests and reader filter tables. The `family_relevance` table, the
`is_positive_gate`/`is_regressive_gate` tables, and the Press/Fan/Coach readers all carry
extensive logic keyed on these classes, none of which a running career ever exercises. Pillar 2
("careers that remember") and the press inbox content therefore depend on an emission breadth
that does not yet exist. This is the single most consequential integrity gap: the readers are
real, the events that feed them are not.

### 1.2 [P2] `CultureWeights::first_alpha_diversity_bps` is loaded and stored but never read
`crates/fw-content/src/runtime.rs:367` (field) vs `runtime.rs:1062-1089` (`sample_player_name`
reads only `compound_last_chance_bps`). The first-letter-diversity knob is deserialized from RON
and held in the struct, but name sampling never consults it — the cultural-diversity dial is a
silent no-op. Either wire it into sampling or drop the field and document the deferral.

### 1.3 [P2] Path-B "truth emerging over seasons" is structurally hollow
`crates/fw-scouting/src/observe.rs:43-78` (each call is an independent noisy draw; no prior
report consulted), `crates/fw-tauri/src/roster.rs:203` (only `last_scout_report` stored, no
history), `season.rs:757-764` (new report overwrites old; `observation_count` increments but the
prior is discarded). The Pillar-4 promise in `crates/fw-scouting/src/lib.rs:5` and
`design/scouting.md:13-15` is that truth narrows over repeated observation. In Path B label
confidence is a fresh uniform draw in [0.40, 0.95) every time; a player observed 30 times gives
the manager exactly the certainty of one observed once. The deferral ("scout track record") is
known, but the crate doc and design doc state the emerging-truth property without the caveat —
misleading documentation against a real but partial implementation.

### 1.4 [P2] FUN-TS1 "shuffle toward the ball" unimplemented; `block_centroid_y` is dead data
`docs/MASTER_PLAN.md:437` (DONE row claims "horizontal compactness/shuffle toward the ball") vs
`crates/fw-match-sim/src/team_shape.rs:252-272` (`block_centroid_y` computed and stored) and
`team_shape.rs:369-375` (`zonal_slot()` Y transform is `form_y * scale_h` only — uniform
vertical compression, no `block_centroid_y` term, no `ball.pos_y` influence). The defensive block
compresses vertically but never slides laterally toward the ball. Half the claimed mechanic
shipped; `block_centroid_y` is computed every tick and consumed nowhere.

### 1.5 [P3] `content_pack_version` is stored, hardcoded to 1, never validated at load
`crates/fw-tauri/src/commands.rs:2201` (writes `content_pack_version: 1` unconditionally),
`crates/fw-save/src/lib.rs:159` + `commands.rs:2264` (mismatch deferred to T5). Present in every
`SaveVN` since V1, advertised in the field doc as a version-mismatch guard, read back correctly
through the migration chain — but never compared against the loaded pack. Deferral is documented
at two sites; flagged so it is not mistaken for a working guard when T5 scope is set.

### 1.6 [P3] `mod_load_fingerprint` described in RULES + ADR-0010 is absent from every `SaveVN`
`.claude/rules/Content/RULES.md:77` and `docs/adr/0010-save-format.md:57-63` both state saves
stamp the BLAKE3 mod-set hash and warn on mismatch; `crates/fw-save/src/lib.rs:309-345` (SaveV4)
has no such field. The code comment at `commands.rs:2264` correctly names it deferred. The risk
is that RULES §6 reads as a *current* invariant ("is stamped") rather than a target, which would
mislead a mod-overlay implementer.

### 1.7 [P3] `procgen::generate_team` ignores `naming_pattern`, diverging from `sample_player_name`
`crates/fw-content/src/procgen.rs:54` (`PlayerName::display()` hardcodes `"{first} {last}"`) vs
`runtime.rs:1062-1089` (reads `naming_pattern`). Two name-rendering paths produce inconsistent
output for non-standard cultures. The T2 deferral is known but the inconsistency between the
paths is documented at neither site.

> Note: two further done-vs-delivered items — `fw-content-baker`'s exit-zero stub bake commands
> (§4.2) and `fw-memory`'s `AfterCompaction` dead routing (§3) — are filed under their crate
> sections because the integrity angle there is secondary to the correctness/dead-code angle.

---

## 2. Per-crate — `fw-core`, `fw-content`, `fw-scouting`

### `fw-core` (determinism primitives)

- **[P2] `ONE_OVER_199` is off by one raw bit with a wrong comment.**
  `crates/fw-core/src/player_attributes.rs:890`. `apply_breakthrough_delta` commits
  `Q32::from_raw(21_582_749)` with a comment claiming `round(4_294_967_296 / 199) = 21_582_749`.
  The true quotient is `21_582_750.23`, so round-to-nearest is `21_582_750` (46 from 2^32, vs the
  committed value's 245). One raw bit of error per step, ~200 bits over a full PA delta
  (~4.6e-8 — negligible to gameplay, but wrong). Deterministic across platforms, so it is a
  correctness flaw, not a determinism flaw; no test pins the raw bits so it is invisible. Fix the
  const to `21_582_750_i64` and the comment.

- **[P2] `to_q32_seconds` silently truncates via `as i32` in release.**
  `crates/fw-core/src/tick.rs:127-133`. `Q32::from_int(self.0 as i32)` guarded only by a
  `debug_assert!`; in release a tick `> i32::MAX` (~828 days at 60Hz, not reachable today, not
  compiler-enforced) wraps. The function is public API with no current callers. Per Sim RULES §11
  use `i64::try_from(...).expect(...)` or a release-active `assert!` before the cast.

- **[P3] `Q32Inner` is reachable despite the lib.rs "not re-exported" claim.**
  `crates/fw-core/src/lib.rs:52-55` claims exposing the inner type is prevented; but
  `q32.rs:33` is `pub type Q32Inner` inside `pub mod q32`, so `use fw_core::q32::Q32Inner;` reaches
  the raw `FixedI64<U32>` and bypasses the checked-operator policy. Make the module `pub(crate)`
  or the alias `pub(crate)`; fix the comment regardless.

- **[P3] `seed_fn` Decision-layer site doc is wrong twice.**
  `crates/fw-core/src/seed.rs:141-143` says the site is `(player_id.as_u32() as u64) << 16 | ...`;
  actual call sites use `player_slot` (u8) not `player_id`, and `site` is a `u32` so `as u64` is
  spurious. The Commentary-layer doc (`seed.rs:112`) is correct — align Decision to it.

### `fw-content`

- **[P1] `news.rs::render_with_vars` passes empty strings to Tracery without the empty-slot fix.**
  `crates/fw-content/src/news.rs:400-421` vs `crates/fw-content/src/memory_callback.rs:454-490`.
  `memory_callback.rs` has an explicit T4-2.5k fix: substitute `" "` for any empty context field
  before building the `Grammar`, because `Grammar::from_map` returns a `ParseError` on
  empty-string rule values. `news.rs` injects context directly. Any empty `HeadlineContext` or
  `QuoteContext` field makes `render_headline`/`render_manager_quote` return
  `NewsRenderError::Tracery` instead of a string. The fix already exists; mirror it.

- **[P2] `PlayerBio.player_id` is not cross-validated against `player_templates` at load.**
  `runtime.rs` + `player_bio.rs`. `load_sources` validates manager→archetype and
  template→signature cross-refs but not that a bio's `player_id` resolves. A bio for a misspelled
  ID loads silently and fails only at lookup — inconsistent with the cross-ref discipline applied
  elsewhere.

- **[P2] `ContentError` enum in lib.rs is dead code.**
  `crates/fw-content/src/lib.rs:89-102`. Public three-variant enum never instantiated; the real
  type is `ContentLoadError` in `runtime.rs`. Remove it to avoid confusion about which error
  callers handle.

- **[P3] `generate_fixtures` doc duplicates its Panics section verbatim.**
  `crates/fw-content/src/league.rs:103-146`.

- **[P3] Commentary-load comment says "6 discriminants" but the loop checks 8.**
  `crates/fw-content/src/runtime.rs:177`. `MatchEventDiscriminant::all()` returns 8 (Offside,
  PassIncomplete added in FUN-TS2b/FUN-CB1). Stale comment.

### `fw-scouting`

- **[P2] `design/scouting.md` stale on the F2 fix: wrong signature, wrong seed site.**
  `design/scouting.md:195,199,163` vs `observe.rs:43,50-55` + `report.rs:137`. The F2 audit
  (2026-06-02) added a 5th `subject: PlayerId` param and changed the RNG site from hardcoded `0`
  to `subject.raw()`; the doc still shows the 4-param signature, the `…, 0)` site, and
  `player_id: String` (actual: `PlayerId`). A Path-A implementer reading the doc recreates the
  byte-identical-report bug. Three sentence fixes.

- **[P2] `observe_player` does not guard on `scout.kind`.**
  `observe.rs:24` (doc requires `BasicScoutUncertainty`) vs `observe.rs:58` (reads only
  `base_observation_noise` + `archetype_id`; ignores `kind`, `biases`,
  `regional_noise_penalty`). Pass a `PhysicalProfiler`/`RegionalExpert` and it silently runs the
  Basic algorithm. Latent because both production callers hardcode `Scout::basic_uncertainty()`,
  but Path-A expansion inherits an invisible pitfall. A `debug_assert_eq!` on `kind` would
  document intent and catch misuse in tests.

- **[P3] No unit test for `technical_true_mean`; shared fixture would make one fail if added.**
  `observe.rs:239-245,263-292,295-320`. `physical`/`mental` have all-half-genes tests;
  `technical` has none, and `all_half_genes()` sets `left_foot: Q32::ZERO`, so the mean of
  `[0, 0.5, 0.5, 0.5, 0.5]` is 0.4 — a hypothetical test would need a corrected fixture.

- **[P3] Label-confidence interval notation mismatch.** `design/scouting.md:224` says `[min, max]`
  (closed); `observe.rs:161-163` produces `[min, max)` (half-open, `uniform_01` is `[0,1)`).
  Negligible numerically; confusing to a verifier. Align the doc to `[min, max)`.

- **[P3] `GeneCategoryEstimate.low`/`high` are `pub`, bypassing the `try_new` invariant; deferral
  is undated.** `report.rs:54-63`. A deserialized or tampered report can violate `low <= high` /
  `[0,1]` without `validate()`; `band()` then yields garbage. No MASTER_PLAN or DECISIONS row
  tracks the deferred encapsulation — needs a tracking entry.

---

## 3. Per-crate — `fw-memory`

The ledger is real, not scaffolding: append-only `Vec` store, `BTreeMap` indexes, five readers,
breakthrough mechanism, compaction, and decay projection all implemented with working IPC wiring.
Determinism discipline is clean. Beyond the P1 event-diversity gap (§1.1):

- **[P2] `SalienceReader::BySubject` claims O(log n) index pre-filtering but does a full linear
  scan.** `crates/fw-memory/src/readers/salience.rs:36-37` (doc) vs `:61-76` (impl scans all
  `ledger.events`). The `by_subject` `BTreeMap` index is built and maintained by
  `rebuild_indexes` but `SalienceReader` never calls `ledger.by_subject()`. For a 10-season career
  (10k+ events) this is O(n) per call, not O(k log k). A false architectural claim in the doc that
  coaches/callers may rely on — correctness-of-contract, not just perf.

- **[P2] `AfterCompaction` callback-eligibility is defined and routed but never emitted in
  production.** `event.rs:529` (variant), `readers/press.rs:52` (routed via `!= Never`), no
  emission anywhere in `season.rs`/`commands.rs`. Designed for long-arc summaries (Retirement);
  `PressReader` correctly treats it as eligible, but no `MemoryEvent` is ever built with it
  outside `readers_integration_test.rs:467`. Until Retirement events are wired with
  `AfterCompaction`, the routing is untested in production and the "events bloom after
  compaction" semantic is invisible to players.

- **[P2] `Consequence` additive variants declared safe for old saves with no old-fixture test.**
  `event.rs:457-511` (`PaRedraw`, `PaReductionRedraw`, `SignatureActivated` "appended additive —
  old saves decode None"). No save written pre-these-variants is decoded post. `fw-save` uses
  bincode, where unknown enum discriminants can *error* rather than degrade to `None` — the claim
  is asserted by comment, contradicted by the likely format, and tested by nothing. Add a
  Consequence-evolution decode fixture to the save migration suite.

- **[P2] `SEASON_MATCH_TICK_BUDGET = 600` undercounts `LegacyGoal`.**
  `season.rs:45` (budget) + `:408` (LegacyGoal harvested from `match_events()`). A full match is
  5400 ticks; the season runs 600 and "deliberately does NOT reach FullTime." Goals between ticks
  601-5400 never exist, so `LegacyGoal` — one of only five emitted classes — fires far less than a
  real season, and the goal-scoring signal feeding Finishing/Composure readiness is proportionally
  absent. Deferred to T5-5b, but it quietly compounds the §1.1 diversity gap.

- **[P3] `days_since` `saturating_sub` is justified but unpinned.**
  `breakthrough.rs:563-571`. The SAFETY rationale (past-after-now saturates to 0, cooldown stays
  blocked) is sound per Sim RULES §11, but no test pins it. A test asserting `past > now ⇒
  cooldown_clear() == false` would kill the saturate→subtract mutation.

---

## 4. Per-crate — `fw-save`, `fw-content-baker`

### `fw-save`

The V0→V4 chain is structurally sound — explicit discriminants, trailing-byte rejection,
watermark validation, ledger-integrity checks, frozen fixtures. Gaps are tests and docs, no
production data at risk.

- **[P2] Fixture README + lib.rs comment stale: V4 undocumented, production schema listed as V3.**
  `fixtures/save-migration/v0001-to-v0002/README.md:169,72` ("current production schema is V3",
  "4 variants") and `crates/fw-save/src/lib.rs:966-967` ("N = 4"). V4 is current (5 variants);
  `v4_career_sample.fwsave` exists on disk and is tested but has no README entry. Contradictory
  schema info in the doc a developer reads for bump guidance.

- **[P2] Proptest round-trip covers only V1/V2; V3/V4 serde surfaces have no randomized test.**
  `crates/fw-save/tests/encode_decode_proptest.rs:73-143` (only `arb_save_v1`/`arb_save_v2`). V3
  added `SeasonState`, V4 added `SavedPlayerInstance` (8 fields incl. nested `AbilityCeiling`,
  `BreakthroughState`, `PlayerSeasonStats`). A discriminant shift in any nested enum evades the
  single frozen fixture and is invisible without a proptest.

- **[P2] V3→V4 callback-preservation has no direct `migrate_v3_to_v4` test.**
  `migration_fixtures_test.rs:1063` exercises it through `load_envelope` (which also runs
  `validate_for_load` + `restore_transient_state`), breaking the direct-call pattern that
  `v1_to_v2`/`v2_to_v3` follow. Nobody has tested what `migrate_v3_to_v4` produces from a V3 with a
  non-empty `breakthrough_states` map, even though the code drops that field unconditionally.

- **[P2] ADR-0010 wire format (FWS1 magic + zstd) does not match the implementation; still
  Proposed.** `docs/adr/0010-save-format.md:36,40,7` vs `crates/fw-save/Cargo.toml` (no zstd) and
  `lib.rs` (raw bincode-2 enum, no magic, no compression; migrations in lib.rs not
  `src/migrations/`). A T5 implementer following the ADR builds an incompatible loader.

- **[P3] Perf test targets SaveV2, not production SaveV4.** `perf_test.rs:83-92,19`. `load_envelope`
  on a V2 envelope runs the full migration chain, so the measured path is not a production
  save/load cycle and omits roster serialization. Harmless at current scale; wrong schema.

### `fw-content-baker`

`validate-structural` is honest — real fail-closed checks, genuine validators, real empty-corpus
guard. But:

- **[P1] `stub_unimplemented` returns `Ok(())` — seven bake subcommands exit zero.**
  `crates/fw-content-baker/src/main.rs:469-476` + `:198-239`. `bake-bios`, `bake-headlines`,
  `bake-scout-phrases`, `bake-manager-quotes`, `bake-fan-reactions`, `bake-commentary`,
  `bake-all`, `manifest` all `println!` and return `Ok(())`. A CI step running `bake-all` gets exit
  0 and produces nothing, with no programmatic way to tell "succeeded" from "not implemented." A
  not-yet-implemented command should `bail!` with a non-zero exit.

- **[P2] `performance_delta` in `CurvePoint` has no range check.**
  `validators.rs:513-555` (explicitly skips it) + `player_bio.rs:310-318` (signed, no bounds). A
  fixture with `performance_delta: 10.0` or `-100.0` passes `validate-structural` silently; the sim
  then receives an unbounded multiplier at match-stake events. Even a loose `[-2, +2]` band would
  close this — it is the only gate content authors run before committing.

- **[P2] `MVP_ROSTER_SIZE = 22` is an exact-match constant that will actively break T4.5 CI.**
  `validators.rs:600-626` + `main.rs:428-434`. Errors on any bio count != 22. The first time T4.5-E1
  (the ~2000-player procedural compiler) adds bio 23, `verify-content` (part of `ci-local`)
  fail-louds. No flag, no floor, no "not the MVP pack" escape — a deliberate CI break T4.5 must
  unblock. Make it a configurable floor.

- **[P2] Content/RULES §8 milestone still says T2-3; T2-3 closed 2026-05-17 without delivering.**
  `.claude/rules/Content/RULES.md:88-90` vs `main.rs:146-148` + `validators.rs:638-643` (correctly
  name T4.5-D/QA-2). The `defer_to` field in the `NotImplemented` errors for `check_banned_terms`
  and `check_licensed_data` also still prints "T2-3" (`validators.rs:642,656`). An operator reading
  RULES expects banned-terms/licensed-data checks to be wired; they are not.

- **[P3] `schemas.rs` is entirely dead, silenced with file-level `#![allow(dead_code)]`.**
  `schemas.rs:14,26-142`. Five schema constants never imported. Low risk, but the strings drift
  from any real validation impl with no compiler signal.

- **[P3] Semantic validators are `pub` API with `#![allow(dead_code)]` masking their uncalled
  status.** `validators.rs:18,632-702`. `check_banned_terms`/`check_licensed_data`/`check_cliche`/
  `validate_fragment` all return `NotImplemented`; their tests assert the stub still errors. The
  blanket allow means a future implementer gets no signal that the "MUST be REMOVED OR REWRITTEN"
  test contract needs updating.

---

## 5. Cross-cutting

### 5.1 Determinism — `xcut`

The sim determinism contract is overwhelmingly clean: no `HashMap`/`HashSet`, no `f32`/`f64` in
canonical state or `MatchEvent`/`MemoryEvent`, no clocks, no `thread_rng`, no `async`/`tokio`,
BLAKE3 the exclusive canonical hash, `clippy::float_arithmetic = deny` on every sim crate, all
`saturating_*` carry SAFETY rationale (`xcut:determinism` PASS at `lib.rs:280`, `ledger.rs:22`,
`lib.rs:78`). One active risk:

- **[P1] `debug_assert!` on negative tick in `should_decide` guards a real silent failure with no
  Sim RULES §11 justification.** `crates/fw-match-sim/src/decision_cadence.rs:128-133`. A negative
  tick makes `rem_euclid(15)` return a valid-but-wrong schedule; the surrounding comment names the
  silent-failure risk and says the assert is the only guard — yet it is `debug_assert!`, skipped in
  release. `Tick` is `i64` under the hood, so a negative value is constructible and passable. §11
  forbids `debug_assert!` for gameplay-truth invariants. Change to `assert!` or add the required
  `// debug_assert OK here because:` justification.

- **[P3] `debug_assert!` on `roster_slot` range missing §11 documentation (release still panics).**
  `decision_cadence.rs:116-119`. `roster_slot == 0` underflows to `usize::MAX` and the slice index
  panics in release (loud, correct) — so not a true silent failure, but §11 requires the
  `// debug_assert OK here because:` comment proving the author verified the release path.

### 5.2 IPC drift — `fw-tauri` DTOs vs `frontend/src/lib/types.ts`

- **[P1] `IpcError::SaveLoadFailed` missing from the TS union; `save_career`/`load_career` already
  registered.** `crates/fw-tauri/src/error.rs:130-143` + `lib.rs:46-47` vs
  `frontend/src/lib/types.ts:112-140` + `route-errors.ts:42-55`. The Rust variant is fully
  implemented and returned by both registered commands; the TS union and `KNOWN_IPC_ERROR_KINDS`
  omit it (the Rust comment at `error.rs:138` admits this). When the save UI is wired, a
  disk/decode failure makes `isIpcError()` return false, the route falls to the generic handler,
  and the user gets a generic message instead of a save-specific one. Safe today (no caller), a
  correctness trap the moment the save UI lands.

- **[P2] `MatchEventKind` TS union contains phantom variants the sim never emits.**
  `types.ts:55-67` + `runtime-validators.ts:262-275` list `HalfTime`, `Card`, `Substitution`; the
  sim's `MatchEvent` (`fw-content/src/event.rs:118`) has none — `all()` enumerates only 8 real
  variants. `ipc_contract_test.rs:697-713` has the correct set. The phantoms exist only in
  `Match.tsx`'s `makeMockResult`. The closed-union "every exhaustive switch fails to compile"
  guarantee (types.ts:44-51) is defeated for these three slots, which become dead switch arms.

- **[P2] `FinalMatchResult.total_events` is `usize` (u64 wire) but the guard uses `isU32`.**
  `live_match/types.rs:290` vs `types.ts:449` + `runtime-validators.ts:780`. `isU32` caps at
  ~4.29B; if `total_events` is ever plumbed from a multi-season ledger length, the guard silently
  rejects a valid payload. The TS annotation also does not reflect the u64 nature.

- **[P3] Season-number guards use `isU32` for `u16`-backed fields.**
  `lib.rs:256-286` (`ChampionHistoryEntryDto.season`, `AdvanceSeasonSummaryDto.completed_season` /
  `new_season_number`, `CareerOverviewDto.season_number` all u16) vs `runtime-validators.ts:648,
  655,657,664` (`isU32`). `PressItemDto`/`PressInboxDto` correctly use `isU16` — inconsistent, and
  weakens the validator as a wire-contract document.

### 5.3 Tests — `xcut:test-health`

Large suite, good per-subsystem proptest coverage, no missing migration fixtures for V1-V4. The
gaps are high-value canonical-state paths with no discriminating test:

- **[P1] SS3 GK save model: 130+ lines of canonical-state logic, zero dedicated unit tests.**
  `crates/fw-match-sim/src/lib.rs:1004-1040` (six tunable constants) + `:2117-2350` (save
  resolution). No test verifies elite > poor save probability, positional penalty firing,
  `SAVE_PROB_MAX` clamp, or high-xG-harder-to-save ordering. `goal_detection_unit_tests.rs:384-450`
  covers only the `xg_score == 0` gather path. A constant-drift or formula-inversion here is caught
  by nothing.

- **[P1] `lane_openness` computed then dead-dropped; no test for the promised wiring.**
  `pass_completion.rs:213` (computed via O(n) pitch-control call) + `:242-247` (`let _ =
  lane_openness;`). No test asserts it gets wired into `p_complete`, nor that completion drops when
  the lane is blocked. Reported here as a *test/integrity* fact only — the believability feel of
  pass lanes is the gap-map's (MASTER_PLAN task #23 tracks the wiring).

- **[P2] Offside: TS2-P2 tests the statistical observable, not the five named geometric cases.**
  `ts2_proptest.rs:429-530` (comment promises cases a-e: beyond-line, midfield, backward pass, two
  boundary cases) vs the two emergent-scan tests actually implemented. Neither discriminates a
  correct impl from one firing on backward passes. `is_offside_at_pass_launch`
  (`dispatch.rs:1554-1628`) is private, so the cases need controlled integration tests. Reported as
  a coverage fact; the offside *model* is the gap-map's (task #25).

- **[P2] `PlayerSeasonStats.goals`: no test verifies the season tally.**
  `season.rs:183-195,363-375` (increment contract + `goals += 1`) vs `season_commands_test.rs`
  (zero assertions on `season_stats.goals`). The goals→ledger→reader→narrative path feeds Pillar 2
  and has zero accumulation-correctness coverage.

- **[P2] `calibrate_smoke_test` shot floor is 10 across 5 full matches — 20% of expected.**
  `calibrate_smoke_test.rs:91-98,73`. The comment says ~10 shots/match (~50 expected) but the floor
  is `>= 10`, so 4 of 5 matches could produce zero shots and still pass — an 80%-degraded shot
  machine goes undetected. A floor of `>= 30` would match the comment's own claimed rate.

- **[P2] `setpiece_state_auto_exits` weakened to accept MidBlock; correctness rides one unit test.**
  `tactic_event_emission_test.rs:156-194` (integration test now accepts MidBlock) + `lib.rs:3052-
  3130` (controlled unit test drives only the `Some(home)→None` branch). If the production
  cross-team transition follows a different branch, the silent-failure guard protects only the
  tested branch; the integration test is now presence-of-entry/exit, not correctness.

- **[P2] `fw-core`: no proptest on `seed_fn` determinism, SeedLayer collision absence, or Q32
  overflow.** `seed.rs` (no proptest) + `q32.rs` (inline tests only). No property asserts two
  different `SeedLayer` discriminants produce different u64s, or that `seed_fn` is injective over a
  broad space. A new discriminant aliasing an existing value would silently share an RNG stream
  with no test failing — and the §1 P1-class "seed_fn pinned vector is tautological" finding below
  compounds this.

- **[P2] `seed_fn` "pinned vector" test is tautological — computes its expected value at runtime.**
  `seed.rs:334-350`. Named/commented as a pin, but it rebuilds the same 17-byte buffer and
  `blake3::hash`es it rather than asserting a hardcoded golden u64. A refactor that changes both
  sides identically passes while the output changes. Hardcode the expected u64.

- **[P3] LUT drift-detection tests are `#[ignore]`-gated and never run in CI.**
  `lut_drift_detection.rs:64,81,26-30`. The sigmoid/exp LUT reference-bake comparisons run only
  with `-- --ignored`; no `scripts/fw verify` hook runs them. A libc/libsystem change drifting the
  LUTs is caught only post-hoc by a developer who remembers to run them.

- **[P3] `fw-content` gene→attribute compiler: no proptest on ceiling enforcement or monotonicity.**
  `gene.rs:235-340` (4 inline tests only). No property asserts compiled attributes fall within
  `[0, AbilityCeiling]`, that `ability_ceiling <= 1.0`, or that growth curves are
  non-decreasing where specified.

### 5.4 Robustness — `xcut:robustness` + `fw-tauri`

- **[P1] `assert!`/`assert_eq!` panics inside the async `load_career_inner` handler.**
  `crates/fw-tauri/src/commands.rs:2277,2334`. Line 2277 asserts a deserialized player's `club_id`
  matches its map key; 2334 asserts the re-derived gene snapshot matches the stored bio. Both can
  be violated by a corrupted-but-migration-valid save (the comment itself says
  "corrupted/hand-edited save"). Tauri RULES §4 forbids panics in handlers; Sim RULES §11 is scoped
  to in-process canonical invariants in sim crates, not user save files. Both must return
  `Err(IpcError::SaveLoadFailed { reason })`.

- **[P1] `advance_season_inner` feeds `Tick::ZERO` to breakthrough `evaluate()`, silencing decay.**
  `commands.rs:1303`. The comment admits it is wrong ("see `CareerState::current_tick()`") but
  never applies the fix. `evaluate()` uses `now_tick` as the salience-decay reference; at
  `Tick::ZERO`, decay guards produce wrong values, so meter accumulation ignores event age/recency.
  The control-arm test at `:2889` demonstrates `Tick::ZERO` suppresses decay. Fix with the same
  two-phase pattern used for `club_names`: compute `now_tick = career.current_tick()` before the
  `&mut career.ledger` borrow.

- **[P2] `TitleWon` event silently dropped when standings are unexpectedly empty.**
  `commands.rs:1258,1273`. The `None` arm of `standings.rows.first()` only logs `error!` and
  continues without emitting `TitleWon` — the inline comment says "this event must never fail
  silently," then does. `TitleWon` is Pillar-2 load-bearing; a title win goes permanently
  unrecorded. Propagate `IpcError::SeasonAdvanceFailed` and abort the transition.

- **[P2] `save_career_inner` and `set_settings_inner` write files non-atomically.**
  `commands.rs:2226-2228,2081-2082,2133`. Plain `fs::write` overwrite; a mid-write kill leaves a
  partial file and the next load gets a bincode decode error — save lost. Documented as a T4-6b
  deferral; acceptable for EA but a real data-loss vector that grows as saves accumulate memory and
  breakthrough state. Standard write-temp + `rename` is atomic on all three platforms.

- **[P2] TOCTOU race in `get_settings_inner`.** `commands.rs:2095,2103`. `path.exists()` then
  `fs::read`; a delete between them is silently mapped to default settings. Drop the `exists()`
  guard, read unconditionally, and match `io::ErrorKind::NotFound` as the first-run path —
  propagate every other kind as `IpcError`.

- **[P3] Unmatched save deltas silently reset player progression on content-pack change.**
  `commands.rs:2299,2307`. Players whose `PlayerId` does not match the current pack's roster are
  skipped with `warn!` and revert to base-template stats with no user signal. At minimum return the
  unmatched count in `LoadCareerResponse` so the UI can warn; stable content-pack-qualified IDs are
  the long-term fix.

- **[P3] `emit_season_end_events` is dead code.** `season.rs:59-80` vs `commands.rs:1258-1273`. The
  wrapper is unreachable since the borrow-checker two-field issue forced `advance_season_inner` to
  call `emit_title_won_event` directly. Remove or `#[cfg(test)]`.

- **[P3] Unknown commentary grammar stems silently `continue` in `load_commentary_grammars`.**
  `runtime.rs:142,161`. A misnamed grammar file is `eprintln!`'d and skipped; the downstream
  missing-discriminant check catches the gap at sample time, long after the root cause is visible.
  Make it a hard `ContentLoadError` so `validate-structural` catches it pre-ship.

- **[P3] `MemoryCallbackLoadError::InvalidBank` erased to `ContentLoadError::TraceryParse`.**
  `runtime.rs:924,935`. Collapses "did not parse as JSON" and "parsed but violated the root-rule
  invariant" into one variant; the baker's validator cannot distinguish the authoring bug. Add an
  `InvalidGrammarBank` variant or a discriminating `kind` field.

### 5.5 Frontend

Solid in shape overall — no React pattern leaks, no runtime `any` in production paths, IPC guarded
end-to-end with `safeInvoke` + hand-written guards. Seven findings:

- **[P1] `App()` called as a plain function inside JSX, breaking Solid reactive ownership.**
  `frontend/src/main.tsx:58`. `<Router>{App()}</Router>` calls `App` as a function instead of
  `<App />`, so every `createResource`/`createSignal`/`onCleanup` inside App and its lazy routes
  runs without a reactive owner — Solid's "computations created outside createRoot will never be
  disposed" warning, and in production, signal subscriptions and cleanup handlers in the router
  subtree are silently never registered. Fix to `<Router><App /></Router>`; the `createRoot`
  band-aid in `state.ts` then becomes unnecessary.

- **[P2] `Stat.tsx` imports ECharts at module top level, violating the lazy-import rule.**
  `Stat.tsx:9-29` + `Stats.tsx:29`. Frontend RULES §5 requires `await import('echarts')`; the
  top-level `import * as echarts from 'echarts/core'` pulls ~280kb into the main chunk on every
  route. Lazy-load `Stat` (single-route use) or move registration into `onMount`.

- **[P2] `createEffect` nested in `onMount` in `Stat.tsx` risks a leaked resize listener.**
  `Stat.tsx:43-57`. The `window.addEventListener` resize listener's `onCleanup` is registered
  inside `onMount`; if `echarts.init` throws the listener is registered but never cleaned, and each
  HMR cycle leaks another. A `ResizeObserver` on the host is self-cleaning.

- **[P2] Five local copies of `isIpcError` duplicate the canonical one in `route-errors.ts`.**
  `Settings.tsx:59-65`, `League.tsx:87-93`, `Player.tsx:66-72`, `Career.tsx:81-87`, `Squad.tsx:60-66`
  vs `route-errors.ts:70-74` (already imported by `Stats.tsx`). Six call sites — past the
  three-site extraction threshold. Each carries its own `KNOWN_IPC_ERROR_KINDS` and `normaliseError`;
  a new IpcError variant means six edits. The `satisfies` annotations give build-time coverage, so
  drift is caught — but the duplication is gratuitous.

- **[P2] Unsafe discriminated-union `as` casts in `ScoutSection` and `PressInboxSection`.**
  `Player.tsx:397-399`, `Career.tsx:424-426`. `props.scoutOutcome?.kind === 'ok' ?
  (props.scoutOutcome as {...}).report : null` bypasses TS narrowing; a renamed variant or field
  passes type-check while runtime access breaks. Use a memo/local in the `Show` accessor so TS
  narrows.

- **[P3] Legacy `playerNameCell` uses `document.createElement`, bypassing Solid's reactive tree.**
  `squad.columns.ts:76-83`. TanStack Solid cell renderers can return JSX; the imperative anchor is
  marked TODO-for-retirement but still exported and type-checked. The comment claims tests use it,
  but `Squad.test.tsx:30` only mocks `getSquad` — it never exercises `squadColumns`/`playerNameCell`.
  Delete it.

- **[P3] `DataTable` hardcodes `aria-label='Data table'` for every instance.**
  `DataTable.tsx:67`. Screen-reader users hear "Data table region" for standings, roster, and
  everything else. Add an optional `label?: string` prop; callers have semantic context
  (`'League standings'`, `'Squad roster'`).

---

## 6. STATUS / ledger drift

- **[P3] STATUS.md "Last canonical hash" section reports stale FUN-CB1 hashes after FUN-TS3b
  rebaselined them.** `STATUS.md:51` shows `eddb9ddc…` (60-tick) + `95ee3978…` (600-tick); the
  FUN-TS3b ship record at `STATUS.md:29` correctly shows the post-rebaseline `110158b9…` /
  `885888ec…`, and `canonical_hash.rs:341,879-880` confirms those are the live pins. Two
  contradictory canonical-hash records inside one file; the standalone section was not updated on
  the FUN-TS3b rebaseline.

---

## 7. Completeness critic — what this review did NOT cover

This was a read-only structural/integrity/test/IPC sweep on the macOS dev box. It did **not**
cover:

- **Cross-platform hash drift.** Windows/Linux matrix agreement is asserted by GH Actions, not
  re-run here. The §5.3 LUT-drift `#[ignore]` finding is a flag, not a check.
- **Runtime behaviour and performance.** No sim was run, no tick budgets profiled, no allocation or
  hot-path cost measured. The §3 O(n) `SalienceReader` and the O(n) `lane_openness` pitch-control
  call are reasoned from code, not benchmarked.
- **Believability / football-feel.** Deferred to the believability-arc review and gap map by design
  (see Dedup) — drift goals, lane *feel*, cross-gate sequencing, offside *modelling*.
- **Content corpus quality.** RON prose tone, Tracery variant counts (the ≥3-per-slot rule),
  banned-terms *semantic* judgement, and cliché detection were not read — and §4's finding is that
  the validators that would check them are stubs, so this gap is doubly unverified.
- **Save-migration four-test obligation end-to-end.** §4 names specific missing tests (V3/V4
  proptest, direct `migrate_v3_to_v4`, Consequence decode) but did not author or run them.
- **Dependency / supply-chain.** No `cargo audit`, license, or version-drift pass.
- **Tauri capabilities / security.** `src-tauri/capabilities/default.json` least-privilege (Tauri
  RULES §6) was not audited.
- **The bake pipeline as a system.** §4 read the stub surface; the actual LLM-authored content-bake
  flow (it is session-authored, not API — DECISIONS 2026-05-29) was not exercised.
- **Forward drift.** Point-in-time snapshot; nothing here predicts drift after 2026-06-05. The
  marked-DONE-vs-delivered integrity pass (§1) is a standing obligation for `/next` Step 6 and the
  next phase-gate Codex pass, not a one-time clearance — re-run it each phase boundary, since drift
  enters through re-pinning, status edits that outrun the diff, and rows closed under pressure.

A sweep that found this many done-vs-delivered drifts (nine) at this altitude is the expected
result for a project moving fast against an ambitious scope, not an alarm — the integrity pattern
is doing its job. The honest read is that the *readers, validators, and DTO mirrors are
consistently more complete than the producers, emitters, and callers that feed them* (the §1.1
event-diversity gap, the §4 bake stubs, the §5.2 unwired SaveLoadFailed, the §1.3 hollow scout
accumulation all share that shape). That asymmetry — infrastructure ahead of the data flowing
through it — is the single theme most worth tracking forward.

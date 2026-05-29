# Roadmap Re-baseline — PROPOSAL (2026-05-29)

> **Status: PROPOSED — NOT YET RATIFIED / NOT APPLIED.** Produced by a 16-agent
> re-baseline workflow (12 audit slices → producer synthesis → 2 adversarial
> critics [completeness + scope-discipline, both gaps-found, all incorporated] →
> producer finalize), at the user'''s request, driven by the confirmed decision that
> EA ships the full 6-tier ~96-club pyramid + LLM-baked content pipeline. Reuses the
> mid-T4 fresh-eyes review (docs/audits/mid-t4-fresh-eyes-review-2026-05-29.md) as
> ground truth. Nothing in MASTER_PLAN / DECISIONS / DESIGN_DOC is changed until the
> user ratifies; on ratification this proposal'''s §4 (decisions) + §5 (doc edits) +
> the phase restructure get applied.

---

# Final Whistle — Revised MASTER_PLAN Proposal (2026-05-29 Re-baseline, Final)

## Preamble

This is the ratification-ready re-baseline synthesizing the mid-T4 fresh-eyes review findings and two rounds of adversarial critique. One fixed fact drives the whole restructure: **EA ships the full 6-tier ~96-club pyramid + LLM-baked content pipeline** (user-confirmed 2026-05-29). That makes two currently-zero-row epics EA-critical, which cannot be entered until a new phase T4.5 exists.

The scope-discipline critic was right on four of its six points. The completeness critic was right on all eight. Both are incorporated below. Changes from the draft are catalogued in §7.

---

## 1. Revised Phase Structure

### Closed History (T0–T3) — annotations for MASTER_PLAN

All four phases are locked. Add the following "what actually shipped vs. pillars" annotations to each phase header:

**T1 (CLOSED 2026-05-16 at `v0.1.0-first-match`):** Shipped 22-player tick engine, ball physics, BT runner with personality bias, 24-sig dispatcher (3 real defs + 1 no-op stub; catalogue frozen at 3-of-24), Tracery commentary, frontend Match page. Pillar 5 mechanism end-to-end but content-thin. `signature_candidates` never populated onto a played roster — no roster existed. The "procedural content stub" row (T1-7) delivered a Markov name chain for 22 names, not a per-seed player generator.

**T2 (CLOSED 2026-05-18 at `v0.2.0-season`):** Shipped deterministic 20-club single-tier league, 380-fixture circle-method schedule, season advance at 600-tick/10s proxy per fixture (real tick engine, shared 22-bio pool — no per-club rosters), ECharts per-team stats, first save chain (V0–V2). The 20 clubs are name-only shells. The LLM bake pipeline shipped offline-names-only (T2-3); 7-of-8 baker subcommands are `stub_unimplemented`, no API client, `content/baked/` empty.

**T3 (CLOSED 2026-05-21 at `v0.3.0-career`):** Shipped multi-season career loop (T3-9 emits exactly ONE event class: `TitleWon` club-only per season), compaction, 5 ledger readers (only `SalienceReader` has IPC callers — `PressReader`/`FanReader`/`CoachReader`/`ScoutReader` have zero callers), breakthrough `evaluate()` + synthetic-harness coverage (zero production callers), scout `observe()` + `ScoutReport` schema (zero gameplay callers), `SaveV3`. The T3 exit-gate "≥1 TitleWon callback surfaces" was met in the thin sense; blank `player_name` render defect is T4-2.5e scope.

**Annotation required on T3 exit gate bullet 2:** "NOTE: callback renders with blank player_name fragment (T4-2.5e fix scope); gate passes on surface-presence, not fidelity."

**T2-3 annotation:** "DELIVERED: bake-names offline path only. 7/8 subcommands are `stub_unimplemented`; no API client; `content/baked/` empty. The EA-critical pipeline is T4.5-D."

**T2-2 annotation:** "DELIVERED: 20 name-only TeamTemplate shells + 380-fixture schedule. Did NOT deliver per-club rosters (T4-2.5b) or the 6-tier pyramid (T4.5-B)."

**T3-2 annotation:** "5 readers shipped; only SalienceReader is IPC-wired. Press/Fan/Coach/Scout surfacing is T4-2.5k (PressReader IPC only; panel UIs deferred post-EA)."

**T3-4 annotation:** "DELIVERED: `evaluate()` engine + synthetic-harness coverage only. ZERO production callers. Wiring is T4-2.5d."

**T3-9 annotation:** "DELIVERED: multi-season runner + compaction + TitleWon-only club emission. Player-subject events require the roster layer (T4-2.5e)."

---

### T4 — Pillar Wiring + Polish

**Rename from:** "Beautiful UI + Tactical Viewer"

**Goal:** Wire all 5 pillars into a played career. Finish the visual identity.

**Current state:** T4-1/2/3/4/5a/6a DONE. T4-2.5a DONE. T4-2.5b is next.

#### Ordered row list

| ID | Title | Deps | Done-criteria | Tag |
|---|---|---|---|---|
| T4-2.5a | AttributeFamily bridge | — | DONE | MVP |
| T4-2.5b | Roster data model + generation | T4-2.5a | 20 clubs × 22 `PlayerInstance`s; distinct `PlayerId`s from `(career_seed, club_idx, slot)`; `get_roster_for_club` IPC; `BTreeMap<ClubId, Vec<PlayerInstance>>` + PlayerId scheme must NOT assume exactly 20 clubs (forward-compat clause — see Decision 5 below, log before this row starts) | MVP |
| T4-2.5c + T1-25 + T1-26 | Pillar 5 candidates onto all 22 slots + dispatch-hardening tests | T4-2.5b | `SignatureFirstFired` fires for a non-slot-7 player in a fixed-seed smoke test; T1-25/T1-26 promoted and green; canonical hash **REBASELINED (authorized)** | MVP |
| T4-2.5d | Pillar 3 wired — breakthrough in season loop | T4-2.5a, T4-2.5b | 5-season integration test on seed `0xfeedbeefcafefade` asserts ≥1 `BreakthroughMoment` with `delta_pa > 0` and non-null player reference | MVP |
| T4-2.5e | Pillar 2 wired — player-subject events + blank-name fix | T4-2.5b | ≥1 `DebutSenior` (or `DebutClub`) + ≥1 `LegacyGoal` emitted in a 2-season career; `get_player_detail` returns non-empty `memoryCallbacks`; no orphaned `" — "` fragment | MVP |
| T4-2.5f | Pillar 4 wired — single-scout observe per match-day | T4-2.5b | `get_scout_report` returns a banded estimate; report after 5 observations differs from 0; IPC contract test round-trips the DTO | MVP |
| T4-2.5g | SaveV4 — roster + player state (minimal subset) | T4-2.5b, T4-2.5d | Four migration tests pass; V3 fixture has non-empty `SeasonState` + ≥1 non-empty `BreakthroughState`; wire byte `0x04` pinned. **Scope: roster + player-instance data only.** World-gen descriptor fields (world_seed, tier_count, etc.) are deferred to a single SaveV5 in T4.5-H rather than adding them now. | MVP |
| T4-2.5h | Per-player match stats + Squad UI | T4-2.5c, T4-2.5e | Stats accumulate across a season; Squad route shows per-player apps/goals/minutes; `pnpm test` green + preview screenshot; `scripts/fw verify` green | MVP |
| T4-P2-fixes | Four P2 correctness fixes (panic-in-handler ×2, raw-message leak, bake-content breakage) | none | `advance_week_inner` + `get_fixtures_inner` return `IpcError` not panic; Transfers raw-message gone; `just bake-content` parses without clap-parse failure | MVP |
| QA-3 | World-gen seed-diversity proptest | T4-2.5b | Two different seeds produce structurally distinct leagues in a 50-pair proptest | MVP |
| QA-5 | `Home.test.tsx` — loading/error/success coverage | T4-4 | Loading skeleton, football-native error copy (not raw `err.message`), success render all tested; `pnpm test` green | MVP |
| T4-2.5i | Pillar 5 — per-signature commentary routing + slot→name | T4-2.5c, T4-2.5e | Two different signatures produce two different commentary strings each naming the player; ≥3 variants per per-signature bank | MVP |
| T4-2.5j | Pillar 5 — signature catalogue toward 8 live (design/signatures.md + ≥8 predicates) | T4-2.5i | `design/signatures.md` authored with all 24 entries (≥8 with implemented predicates, remaining 16 as stubs carrying `not_yet_implemented: true`); ≥8 trigger predicates implemented + tested (≥1 per role family); canonical hash rebaselined (authorized) | MVP |
| T4-2.5k | Pillar 2 — PressReader IPC registration | T4-2.5e | `PressReader` IPC command registered and callable; press-inbox panel renders non-empty output in a 2-season fixture career. Scope bounded to PressReader only — Fan/Coach/Scout panel UIs are post-EA (Deferred). | MVP |
| T4-2.5L | Pillar 2 — cross-decade callback proof + compaction corpus | T4-2.5e, T4-2.5g | In a ≥8-season career, a player-subject event from season 1 that survives the 5-season compaction boundary still renders as non-empty callback prose on `/player` in season 8 (the falsifiable Pillar 2 end-to-end proof); `RegressiveCollapse` emitted at career end; QA-1 compaction-retention corpus (100 ledgers / 10 seasons, ≥95% event survival) passes on CI matrix | MVP |
| T4-2b | Per-player + per-season ECharts stat dashboards | T4-2.5h | Per-player goals/apps/minutes chart; per-season numeric stats; `pnpm test` green | MVP |
| T4-5b | Frontend live-mode UI (Pause/1×/4×/16×, commentary feed) | T4-5, T3-3, T4-2.5h | Live match session drives tick-by-tick board; commentary feed renders; speed controls work; salience-band semantics logged via `/log-decision` before this row starts | DEFERRED→TODO when T4-2.5h closes |
| T4-7 | Game-shell polish — window chrome, main-menu, splash, app-icon | T4-3 | Looks like a finished product on a stranger's machine; app-icon ships per-OS | MVP |
| T4-8 | Phase-gate Codex review #3 | T4-2.5h, T4-7, T4-2b, T4-2.5L | Codex posts ack on `phase-gate-T4` topic; T4 pillar-wiring surfaces including roster-layer reviewed | MVP |

**Cuts vs. draft (scope-discipline):**
- T4-2.5m / T4-2.5n / T4-2.5p (multi-scout disagreement + track records + scouting-board UI) → Deferred. Single-scout uncertainty (T4-2.5f) is the EA floor per DESIGN_DOC §228 and `career-roster-layer.md §9`. Multi-scout work is behind a feel-prototype gate per DESIGN_DOC §238; it goes to the Deferred section with that explicit promotion trigger.
- T4-2.5q (counterplay/cancellation predicate layer) → Deferred. Post-EA, behind a "≥16 signatures live" promotion trigger per the scope-discipline verdict.
- T4-6b remains DEFERRED. Not a T4 gate blocker. Unblocks whenever the `/log-decision` for palettes + text-scale + rebindable-action catalog is made.
- T4-9 (enhanced 2D tactical viewer) → CUT from Stretch to Deferred. The scope-discipline verdict is correct: a Stretch row that the project can't afford to pursue while the world-gen phase is unscheduled should be parked, not kept as TODO. It can be promoted back post-EA.

**Parallelism:** T4-P2-fixes, QA-3, QA-5 can ship immediately. T4-2.5d / T4-2.5e / T4-2.5f are parallelizable after T4-2.5b. T4-2.5i is parallelizable with d/e/f. T4-2b and T4-5b auto-promote to TODO when T4-2.5h closes.

**Revised T4 Exit Gate (replaces the stale 2-bullet gate):**

1. A stranger watching a 600-tick fixture replay on the 2D board can identify the score, possession player, and at least one named signature moment (not "slot 7") without reading a design doc. [T4-2.5i done]
2. Opening `/player` for a squad member who has played shows ≥1 memory callback (DebutSenior/DebutClub or LegacyGoal); the scout-report panel shows a banded estimate. [T4-2.5e + T4-2.5f done]
3. The Squad screen shows per-player apps/goals/minutes for at least one season. [T4-2.5h + T4-2b done]
4. A player-subject event from season 1 renders as non-empty callback prose in season 8 after compaction — the Pillar 2 cross-decade proof. [T4-2.5L done]
5. The compaction-retention corpus test (QA-1, 100 ledgers / 10 seasons) passes on CI matrix. [T4-2.5L done]
6. The press-inbox panel renders non-empty output. [T4-2.5k done]
7. Theme, fonts, visual identity locked; app-icon ships per-OS. [T4-7 done]
8. `scripts/fw verify` green; SaveV4 migration tests pass; canonical hashes stable since T4-2.5c rebaseline.
9. Vertical-slice tag: `v0.4.0-polish`.

NOTE: T4-5b (live-mode UI) is DEFERRED — gate passes on replay surface. T4-6b is DEFERRED and is not a gate blocker.

---

### T4.5 — World Scale + Content Bake (NEW PHASE — EA-critical)

**Goal:** Build the procedural world and LLM-baked content corpus the EA product promises. Zero rows exist today.

**Exit Gate:** 6-tier ~96-club pyramid generates deterministically; ~2000+ procedural players; LLM-baked corpus committed + manifest-pinned; composed-name collision lint green; promotion/relegation wired; perf budget re-derived. Vertical-slice tag: `v0.5.0-world`.

| ID | Title | Deps | Done-criteria | Tag |
|---|---|---|---|---|
| T4.5-A | Port `docs/design/worldbuilding.md` from archive | T4-8 | v2 worldbuilding.md defines nation structure, 6-tier pyramid shape (~96 clubs), promotion/relegation rules, `RegionPriors` shape (cultural-cohort weights → name-bank selection, club-culture tags, regional rivalry adjacency). Unity→Rust + f32→Q32 + HashMap→BTreeMap applied; `RegionPriors` type sketched so T4.5-B0 has a concrete input shape | MVP |
| T4.5-B0 | Nation + RegionPriors generator | T4.5-A | `generate_nation(world_seed)` produces a deterministic set of regions each carrying a `RegionPriors` struct (cultural-cohort weights + name-bank selector + rivalry-adjacency flag); two seeds produce structurally distinct region/culture assignments (proptest over 20 seeds); output consumed by T4.5-B/C/E as the per-seed cultural layer | MVP |
| T4.5-B | 6-tier pyramid generator — generalize single-20-club tier to 6 tiers ~96 clubs | T4.5-B0 | `generate_pyramid(world_seed)` builds ~96 clubs across 6 tiers per worldbuilding.md; `generate_fixtures` refactored from `&[ClubId; CLUBS_PER_LEAGUE]` (fixed-size array) to `&[ClubId]` (slice) + per-tier size config — this is a `Vec`/slice refactor of `generate_fixtures`, `Standings`, and all invariant asserts against `CLUBS_PER_LEAGUE`; canonical hash impact assessed and if the season path drifts, rebaseline is **authorized and documented**; same-seed determinism + cross-seed divergence proptest | MVP |
| QA-2 | Composed-name banned-terms + licensed-data semantic validator | T4.5-B | `fw-content-baker validate-semantic` exits non-zero on a deliberately composed banned/licensed name; WARNING comment at `main.rs:460` replaced with `// TODO(T4.5-D/QA-2): remove once validate-semantic lands` | MVP |
| T4.5-C | Club identity layer — names, crests/colors, culture tag, rivalry adjacency | T4.5-B, T4.5-B0 | Each club carries: deterministic regional culture tag (from `RegionPriors`), crest/color set, same-region/same-tier rivalry adjacency flag (the minimum ADR-0005 `rivalry_boost` needs). Scope: NO founding-history prose here — that is LLM-baked flavor under T4.5-D | MVP |
| T4.5-D | Real Anthropic API client + 7 stub baker subcommands | T4.5-A, QA-2 | `bake-bios` / `bake-headlines` / `bake-scout-phrases` / `bake-manager-quotes` / `bake-fan-reactions` / `bake-commentary` / `bake-all` all implemented (not stubbed); `just bake-content` runs without clap-parse failure; each manifest pins `model_id + prompt_hash + seed`; `content/baked/` non-empty; `validate-structural` passes. Note: founding-history flavor text for T4.5-C clubs baked here | MVP |
| T4.5-E0 | Gene→PlayerAttributes forward compiler + balance validation | T4-2.5a | `gene_to_attributes(gene: &GeneSnapshot, role: Role) -> PlayerAttributes` implemented on the 55-attribute schema; balance-sanity sweep: 1000 random genes produce no attribute outside 1..=200; proptest invariant covers the 1..=200 range; `design/player-generation.md §MVP-boundary` updated to RESOLVED. This is the missing link between the existing gene-sourced PA/CA bridge (T4-2.5a) and the full attribute map T4.5-E1 requires | MVP |
| T4.5-E1 | Procedural ~2000-player compiler — (career_seed, club, role, cohort) → PlayerInstance | T4.5-B, T4.5-E0 | ~2000–2400 distinct `PlayerId`-unique `PlayerBio`/`GeneSnapshot`/`PlayerAttributes` records from one seed; gene→attribute mapping uses T4.5-E0; cultural-cohort name-bank draws from T4.5-B0's `RegionPriors`; `PlayerBioRosterValidator` re-scoped from exactly-22 to pyramid scale; 10K-match balance-sanity sweep | MVP |
| T4.5-F | Promotion/relegation + one cup competition, with player-club tier-mobility | T4.5-B | End-of-season promotion/relegation moves clubs between tiers and persists; cup bracket generates over the pyramid; multi-season test asserts a club changes tier; **done-criterion extension:** when the player's club is promoted/relegated, the career loop continues to run the real engine for that club in its NEW tier and the career-overview reflects the tier change (reconciles with Decision 3 match-engine policy) | MVP |
| T4.5-G | Wire pyramid + compiler into career-roster layer | T4.5-B, T4.5-E1, T4-2.5g | `CareerState.roster` populated from compiler across pyramid (~96 clubs, ~2000 players), replacing the 20-club template-assign; `BTreeMap<ClubId, Vec<PlayerInstance>>` + PlayerId scheme survive pyramid scale; forward-compat from T4-2.5b's forward-compat clause confirmed | MVP |
| T4.5-H | SaveV5 — world-gen pyramid descriptor (additive on V4) | T4-2.5g, T4.5-G | `world_seed`, `tier_count`, `club_count`, `player_count` added as serde-default fields (additive migration: V4 saves load cleanly, world descriptor fields default to the legacy 20-club state); **mod_fingerprint NOT included** — mods are out of EA scope (see Decision 6 below); wire byte `0x05` pinned; 4-test discipline | MVP |
| T4.5-I | Perf smoke + re-derive T5-5 budget | T4.5-G | 1-season pyramid career (player's-club real engine per Decision 3 + seeded-procgen AI fixtures + full `evaluate()`/`observe_player` on ~2000 players) runs and records wall-clock; `/log-decision` sets new perf target; old `<60s/20-club-seeded-procgen` number formally retired | MVP |
| T4.5-J | Phase-gate Codex review #3.5 | T4.5-C, QA-2, T4.5-G, T4.5-H | Codex reviews pyramid determinism cross-OS, compiler reproducibility, bake-manifest pinning, composed-name safety, perf budget, mod_fingerprint exclusion; posts ack on `phase-gate-T45` | MVP |

**T4.5 Exit Gate:**

1. `generate_pyramid(seed)` produces a deterministic 6-tier pyramid of ~96 clubs; two different seeds produce distinct nation/club/culture assignments (proptest over 20 seeds). [T4.5-B + T4.5-B0]
2. Career start populates ~2000–2400 `PlayerInstance`s across ~96 clubs; `PlayerId` uniqueness holds over a 100-seed proptest. [T4.5-E1 + T4.5-G]
3. `just bake-content` completes without error; `content/baked/` non-empty; each manifest pins `model_id + prompt_hash + seed`; `validate-structural` + `validate-semantic` both pass. [T4.5-D + QA-2]
4. Composed-name semantic validator (QA-2) rejects a known banned-name test fixture.
5. SaveV5 `0x05` wire byte pinned; pyramid-generated career saves and loads with correct club/player counts; V4 saves migrate cleanly (no mod_fingerprint). [T4.5-H]
6. Perf budget re-derived and logged via `/log-decision` (T4.5-I); the retired `<60s/20-club` number superseded in DECISIONS.md.
7. Promotion/relegation runs across a 3-season test; a club changes tier; player-club tier-change continues with the real engine. [T4.5-F]
8. Codex T4.5 phase-gate ACCEPT. [T4.5-J]
9. Vertical-slice tag: `v0.5.0-world`.

---

### T5 — Ship to Steam

**Goal:** Public Steam EA release. itch.io demo validates update pipeline first.

**All T5 rows depend on T4.5 completing.** Revised dep graph: T4 → T4.5 → T5.

| ID | Title | Deps (revised) | Done-criteria (revised) | Tag |
|---|---|---|---|---|
| T5-1 | Apple code-signing + notarization | T4-8 | Mac DMG signs + notarizes + Gatekeeper-passes on a clean machine. Pre-task: hello-world Tauri-2 notarization dry-run as its own `/next` pick before full T5-1 starts (RISK-T5 mitigation) | MVP |
| T5-2 | Steam Direct + Steamworks achievements + cloud saves | T5-1, T4.5-H | Three test achievements unlock; cloud-save round-trip on a second machine after a schema-version patch exercises the V4→V5 migration chain | MVP |
| T5-3 | Steam Deck Verified prep — 1280×800 sweep, controller inputs, suspend/resume | T4-6b, T4-2.5h, T4-2b, T4.5-B | Deck hardware survives 30-min suspend/resume; Verified or Playable rating; sweep covers populated pillar surfaces and the pyramid world | MVP |
| T5-4 | Localization pipeline — string extraction + fluent runtime | T4-2.5e, T4-2.5f, T4-2.5i, T4.5-D | One non-English test locale loads; lint catches hardcoded strings; extraction runs after differentiating copy (callbacks, scout bands, sig commentary) exists | MVP |
| T5-5 | Performance pass | T4.5-B, T4.5-E1, T4-2.5d, T4-2.5h, T4.5-I | Full 10-season career (player's-club 38 real-engine matches/season per Decision 3; AI fixtures seeded-procgen) runs within the **T4.5-I re-derived budget** (the old `<60s` was against the retired 20-club seeded-procgen substrate — it is retired); tactical-board p95 frame ≤16.6ms | MVP |
| T5-6 | itch.io demo release | T5-4, T4.5-J | Two consecutive updates ship cleanly; <50 external testers report no install issues; demo contains EA content (pyramid world, not 20-club stub) | MVP |
| T5-7 | Steam EA release | T5-2, T5-6 | Public EA live; first 24h crash-free ≥98% | MVP |
| T5-8 | Phase-gate Codex review #4 (pre-EA) | T5-7, T4.5-D, QA-2 | Codex reviews release-readiness + security + AI-disclosure: manifest pins `model_id + prompt_hash + seed` for every baked pack; composed names pass QA-2 validator; Steam AI-disclosure form filled; CI-R3 (coverage floors) green; CI-R4 (build reproducibility) green | MVP |

**Revised T5 Exit Gate:**

1. Public Steam EA live.
2. Steam Deck Verified or Playable rating.
3. First-week crash-free ≥98%.
4. itch.io demo live.
5. The shipped EA build generates a 6-tier ~96-club procedural pyramid with an LLM-baked content corpus; ~2000+ procedural players; all 5 pillars produce player-visible output in a played career (≥1 breakthrough fires, single-scout banded estimate surfaces, player-subject callbacks render non-empty, ≥1 named signature fires per match).
6. Full 10-season career completes within the T4.5-I re-derived perf budget (not the stale `<60s`).
7. CI-R1 (cargo audit) DONE; CI-R2 (cargo deny) DONE; CI-R3 (coverage floors) green; CI-R4 (build reproducibility) green.
8. Vertical-slice tag: `v1.0.0-ea`.

---

## 2. Deferred-Pile Resolution

Every DEFERRED item with its final disposition. The standing `/done` rule: no DEFERRED row may survive a phase close without a Deferred-section entry naming a scheduled blocker or explicit "off critical path" tag.

**Items promoted to TODO:**

- **T4-2b** → Promote to TODO when T4-2.5h closes. (Already in T4 row list above.)
- **T4-5b** → Promote to TODO when T4-2.5h closes. Add `T4-2.5h` as explicit dep. Owed `/log-decision` on salience-band semantics before implementation starts.
- **T1-25 / T1-26** → Promoted to TODO, bundled into T4-2.5c row (same `/next` cycle).

**Items kept DEFERRED with explicit triggers:**

| ID | Title | Blocker / Promotion trigger |
|---|---|---|
| T4-6b | Settings: text-scale, colorblind, key rebinds | Unblocks on `/log-decision` for palettes + text-scale steps + rebindable-action catalog. Orthogonal to world-gen. T5-3 (Deck sweep) needs rebindable inputs — promote before T5-3. |
| T4-2.5m/n/p | Multi-scout disagreement (3 archetypes), track records, scouting-board UI | Conditional-EA: behind the "Month-4 feel-prototype gate (3 external testers per DESIGN_DOC §13 OQ3)". Park here. Single-scout uncertainty (T4-2.5f) is the EA floor. |
| T4-2.5q | Counterplay/cancellation predicate layer | Post-EA. Promotion trigger: ≥16 signatures live. |
| T4-2.5k-panels | Fan/Coach/Scout reader panel UIs (the UI half of T4-2.5k) | Post-EA. PressReader IPC (the cheap half) ships in T4; the three remaining reader panels defer here. |
| T4-9 | Enhanced 2D tactical/replay viewer | Post-EA. Was Stretch; moved to Deferred. Promotion trigger: EA ships clean + user wants it. |
| T1-17 | Friction-test discrimination | Anchor: next `ball_physics.rs` touch OR T5-8 pre-EA gate, whichever first. |
| T1-27 | (test-quality) | T5-8 pre-EA gate. |
| T1-28 | (separation.rs test-quality) | Next `separation.rs` touch OR T5-8. |
| T2-R-E3 | Mod-overlay negative-path tests | Blocker: mod-overlay loader. Mods are **out of EA scope** (Decision 6). Deferred post-EA as "UNSCHEDULED — off critical path." |
| T2-1d2 | xg_utility honesty / cross-band oscillation invariant | Re-anchor: T4-2.5c rebaseline window (fold into authorized rebaseline) OR T5-5 start (xg_utility honesty feeds perf target). Remove from STATUS.md live Blockers. |

**Items to mark DONE:**

- **CI-R1** (`cargo audit`) → **DONE**. Confirmed wired in `Justfile` at T1-13 (`audit:` recipe + wired into `ci-local`). Cross-reference: MASTER_PLAN Snapshot build-health note at T1 close states "cargo audit + cargo deny green." Mark DONE with cite to T1-13 commit.
- **CI-R2** (`cargo deny`) → **DONE**. Same rationale as CI-R1. Mark DONE with cite to T1-13 commit.
- **CI-R5** (banned-terms lint) → **DONE**. `scripts/fw banned-terms` wired in `scripts/fw verify`; green in every recent STATUS. Mark DONE with citation to T0-8 reconciliation commit.

**Re-anchored:**

- **CI-R3** (coverage floors) → Re-anchor to `before T5-8`. It is a ship-readiness item.
- **CI-R4** (build reproducibility) → Re-anchor to `alongside T5-1` (build reproducibility belongs with code-signing).

---

## 3. Critical Path to EA

The genuine spine from today to a shippable EA. Items in `[ ]` are parallelizable.

```
T4-P2-fixes / QA-3 / QA-5  (no deps — ship immediately)

T4-2.5b  (roster data model — THE chokepoint)
  → [T4-2.5c+T1-25+T1-26 || T4-2.5d || T4-2.5e || T4-2.5f]  (parallel after b)
  → T4-2.5g  (SaveV4 — minimal: roster + player state only; world descriptor deferred to V5)
  → T4-2.5h  (per-player stats + Squad UI)
     → [T4-2b, T4-5b, T4-2.5i, T4-2.5j, T4-2.5k, T4-2.5L]
     → T4-7  (game-shell polish)
  → T4-8  (Codex gate #3)

T4.5-A  (worldbuilding.md port — phase entry gate)
  → T4.5-B0  (nation + RegionPriors generator — cultural layer)
  → [T4.5-B + QA-2]  (parallel)
  → [T4.5-C, T4.5-D, T4.5-E0]  (parallel after B + B0)
  → T4.5-E1  (needs B + E0)
  → [T4.5-F, T4.5-G]  (F needs B; G needs B + E1)
  → T4.5-H  (SaveV5 — needs G)
  → T4.5-I  (perf smoke — needs G)
  → T4.5-J  (Codex gate #3.5)

T5-1 → T5-2 → [T5-3, T5-4, T5-5] → T5-6 → T5-7 → T5-8
```

**Chokepoints:**
- **T4-2.5b** — every pillar-wiring row waits on this. Decision 5 must be logged before it starts.
- **T4-2.5h** — T4-2b, T4-5b, the Codex gate, and T4.5-A all wait on this (T4.5 rows need the roster shape proven before scaling to 96 clubs).
- **T4.5-B0** — the cultural layer that T4.5-C and T4.5-E1's cohort param both consume. New row, no code yet.
- **T4.5-G** — EA cannot ship until the procedural world is wired into the career; the merge of the two streams.
- **QA-2** — must gate T4.5-D and T4.5-E1. Semantic validator must exist before non-hand-authored content is committed.

**Row count check:** T4 adds ~17 net new rows (T4-2.5b-h + T4-P2-fixes + QA-3/5 + T4-2.5i/j/k/L + T4-2b + T4-7/8, minus T4-2.5m/n/p/q/9 cut). T4.5 adds 11 rows. T5 is 8 rows. Total from today to EA: ~36 rows. The ~55-item ceiling applies to the whole plan including T0-T3 (already DONE); the ceiling is not violated. See Decision 7 below for the formal cap update.

---

## 4. Decisions to Log

These must be logged via `/log-decision` at the timing noted. None require user ratification beyond this re-baseline — they follow from the fixed EA-scope decision.

**Decision 1 — EA scope fixed (log immediately; unblocks T4.5 phase creation):**
EA ship target is the full 6-tier ~96-club pyramid + LLM-baked content pipeline + ~2000-player procedural compiler. NOT a narrowed hand-authored single-league slice. The career-roster layer (T4-2.5a-h) ships the 20-club template-assign first increment; the pyramid + compiler + bake pipeline follow in T4.5. Resolves DESIGN_DOC §13 OQ1 ("Pyramid scope at launch" — one nation, 6 tiers, ~96 clubs; two-nation pyramid is post-EA). Supersedes the "open follow-up" language in DECISIONS 2026-05-29 T4-resequence entry.

**Decision 2 — Phase T4.5 inserted between T4 and T5 (log with Decision 1):**
T4.5 (World Scale + Content Bake) is a new phase containing the 6-tier pyramid generator, `RegionPriors`/nation layer, ~2000-player procedural compiler, gene→attribute forward compiler, LLM bake pipeline, and associated save/validation rows. T5 depends on T4.5. Supersedes the implicit T4→T5 direct chain in the original MASTER_PLAN dependency graph.

**Decision 3 — Match-engine policy correction (log before T4.5-I):**
The 2026-05-29 DECISIONS "seeded-procgen scorelines" framing was imprecise. Correct state: the season ALREADY runs the real 22-player tick engine for all 380 fixtures (season.rs:108 → `play_one_match`), discarding all output except the scoreline. The T4-2.5 roster layer changes what output is KEPT. The match-engine POLICY (player's club keeps full output; AI-vs-AI fixtures use seeded-procgen scorelines) is a NEW code reduction, not a description of current behavior. T4.5-F's tier-mobility done-criterion and T5-5's perf target must both be measured against whichever policy actually ships. Supersedes the "seeded-procgen scorelines rather than the 22-player tick engine" phrasing in DECISIONS 2026-05-29 T4-resequence.

**Decision 4 — T5-5 perf target retired (log before T4.5-I):**
The `<60s / 10-season career` target was derived against the retired 20-club seeded-procgen substrate. It is retired. The new target is re-derived via T4.5-I and logged via a separate `/log-decision` at that point. T5-5's done-criteria reference the T4.5-I-derived number.

**Decision 5 — T4-2.5b forward-compatibility requirement (log BEFORE T4-2.5b — it is the next row):**
The `BTreeMap<ClubId, Vec<PlayerInstance>>` and `PlayerId` derivation scheme in T4-2.5b must NOT assume exactly 20 clubs or template-sourced players. The scheme must scale to ~96 clubs without structural change. This is a binding done-criteria clause, not prose.

**Decision 6 — Mods are out of EA scope (log before T4.5-H):**
The mod-overlay loader has no row in the EA plan. `mod_fingerprint` (ADR-0010) is therefore NOT included in SaveV5 (T4.5-H). T2-R-E3 (mod-overlay negative-path tests) remains Deferred post-EA behind "mod-overlay loader (UNSCHEDULED — off critical path)." The save format remains forward-compatible with adding `mod_fingerprint` as an additive serde-default field in a future SaveV6. Supersedes the ADR-0010 implication that `mod_fingerprint` lands at EA.

**Decision 7 — Item-budget cap update (log with Decision 2):**
The original ~55-item ceiling (MASTER_PLAN line 76) was set before the EA-scope decision fixed the 6-tier pyramid + LLM bake pipeline as EA-critical. The revised plan adds ~23 net new rows to handle the previously-unscheduled world-gen phase. The new ceiling is ~80 items total (T0-T5 including already-DONE rows). Cut discipline remains in force: Stretch before scope; every feature in exactly one bucket. Supersedes the "~55 items" cap.

---

## 5. Doc Edits to Apply

Apply in the re-baseline commit (or immediately adjacent commits). DESIGN_DOC edits wait for Decision 1 to land in DECISIONS.md first (per design-docs/RULES §1).

**`/Users/vibelogic/dev/football/docs/MASTER_PLAN.md`**
- Rewrite Snapshot block: `2026-05-29 — T4 IN PROGRESS`; T0–T3 closed; T4-1/2/3/4/5a/6a + T4-2.5a done; T4-2.5b next; open EA-scope decision now resolved per Decision 1 above.
- Rewrite Now/Next/Blocked: Now = T4-2.5b (pending Decision 5 log); Next = T4-2.5c..h then T4-7/T4-8; Blocked = none live (T2-1d2 re-anchored to Deferred).
- Rename T4 section from "Beautiful UI + Tactical Viewer" to "Pillar Wiring + Polish."
- Add T4.5 section with 11 rows, exit gate, `v0.5.0-world` tag.
- Add T4.5 row to Tier Overview table between T4 and T5.
- Update T4 Tier Overview cell: ~20 rows, updated exit-gate one-liner.
- Update T5 dep from T4 to T4.5; update T5 exit gate to add bullets 5, 6, 7.
- Rewrite `§Dependencies` graph block per the critical path in §3 above.
- Add `## Deferred` product-feature section (before `## Deferred Scaffolding Trackers`) with all items from §2.
- Flip CI-R1/R2/R5 to DONE with commit cites (T1-13 for R1/R2; T0-8 reconciliation for R5).
- Re-anchor CI-R3 to `before T5-8`; CI-R4 to `alongside T5-1`.
- Apply T3 row annotations listed in §1.
- Correct T4-2.5e done-criteria: replace `PlayerDebut` with `DebutSenior (or DebutClub)`; resolve the non-existent variant.
- Correct T5-5 done-criteria: reference T4.5-I-derived budget; drop `<60s`.
- Add T4.5-B done-criteria note: the `&[ClubId; CLUBS_PER_LEAGUE]`→`&[ClubId]` refactor is a structural change touching `generate_fixtures`, `Standings`, and all `CLUBS_PER_LEAGUE`-keyed invariants; authorized rebaseline if season path drifts.
- Add Risk Register rows: LLM bake pipeline is EA-critical with 7-of-8 subcommands stub at T4.5 entry; gene→attribute forward compiler has no existing code.
- Update budget cap line from `~55` to `~80` per Decision 7.
- Flip T4-9 from Stretch/TODO to Deferred.
- `last_verified: 2026-05-29`

**`/Users/vibelogic/dev/football/docs/DESIGN_DOC.md`** (after Decision 1 lands in DECISIONS.md)
- Add `### MVP scope` sub-heading in §8 so `§MVP-scope` citations resolve.
- Re-bucket "6-tier ~96-club pyramid + ~2000-player compiler + LLM bake pipeline" into the `IN — MVP / EA launch` list.
- Resolve §13 OQ1: change to `RESOLVED 2026-05-29 — one nation, 6 tiers, ~96 clubs; two-nation pyramid is post-EA. See DECISIONS.md 2026-05-29.`
- `last_verified: 2026-05-29`

**`/Users/vibelogic/dev/football/STATUS.md`**
- Remove T2-1d2 from live Blockers; add pointer to its Deferred-section entry.
- Once DESIGN_DOC §8 gains the `### MVP scope` anchor, update the `§MVP-scope` citation; until then flag it as a phantom anchor.

**`/Users/vibelogic/dev/football/docs/design/career-roster-layer.md`**
- §0 (line 21) + §3 + §9 items 1-2: replace "T4+ / later phase" with concrete row IDs (T4.5-B/B0 for pyramid/nation, T4.5-E0 for forward compiler, T4.5-E1 for compiler, T4.5-G for wiring).
- §5: Replace `PlayerDebut` with `DebutSenior (or DebutClub)`.
- §6: Add one-line clarification that "Option 1" REDUCES today's all-380-real behavior rather than describing current state.
- §9: Mark T4-2.5m/n/p as moved to Deferred (not in scope for T4 or T4.5-EA).

**`/Users/vibelogic/dev/football/crates/fw-content-baker/src/main.rs`**
- Stub milestone tags: update from closed phase references to T4.5-D.
- Line ~460 WARNING comment: add `// TODO(T4.5-D/QA-2): remove once validate-semantic lands`.

**`/Users/vibelogic/dev/football/crates/fw-save/src/lib.rs`**
- Line 144 (SaveV2 doc): reword from "V2 is the CURRENT production schema" to "PRESERVED FOREVER — locked at T3-1, superseded by V3."
- Lines 170/192-193/300: reword "loader regenerates a fresh season" to "CALLER RESPONSIBILITY: when season is None the caller must regenerate a fresh SeasonState."

**`/Users/vibelogic/dev/football/frontend/src/components/Layout.tsx`**
- Lines 47 + 121: drop/refresh "T0 scaffold" pill and "v0.1.0 · T0 scaffold" footer; update line-8 comment from "lands at T4-6" to "lands at T4-6b."

**`/Users/vibelogic/dev/football/frontend/src/routes/Tactics.tsx`**
- Lines 18 + 30: remove closed-phase claims ("land at T1-6", "ship at T2-1"); replace with "Not yet wired."

**`/Users/vibelogic/dev/football/frontend/src/routes/Home.tsx`**
- Lines 28 + 59: remove "T0 placeholder" and "land at T2-5" copy; replace with current-state descriptions.

---

## 6. Open Questions for the User

These are genuine product-vision forks that nothing above is blocked on. Defaults are stated; redirect if wrong.

**Q1 — T4-5b live-mode vs replay-only at T4-8:** The T4 exit gate passes on replay if T4-5b stays DEFERRED. Is live-mode required before the T4-8 Codex gate, or does a polished 600-tick replay satisfy "stranger understands player identity"? Default: replay is sufficient; T4-5b promotes in parallel with T4.5 work if time allows.

**Q2 — T4-6b timing:** Can unblock any time with a `/log-decision` on palettes + text-scale + rebind catalog. Do you want that decision made now (while T4-6a design is fresh) or defer until post-T4.5? It is not on the EA critical path, but T5-3 (Deck sweep) needs rebindable inputs — promote before T5-3 starts.

**Q3 — Match-engine policy confirmation:** Decision 3 corrects the framing but the policy itself (player's club = real engine, AI fixtures = seeded-procgen) still needs confirmation. At 96 clubs there are ~3,000+ AI fixtures per season. The plan assumes seeded-procgen is sufficient for AI (zero new code; just confirm the reduction). If you want a lightweight scoreline model instead, that is a new T4.5 row. Default: confirm seeded-procgen for AI fixtures.

**Q4 — Signature EA floor confirmation:** The plan holds the EA floor at ≥8-of-24 predicates implemented, with all 24 authored in `design/signatures.md` (stubs for the 16 not-yet-implemented). Full 24 implemented is post-EA. Is 8-of-24 the right floor, or should EA commit to a different number? Default: 8-of-24 live, all 24 in the catalogue.

**Q5 — T4-2.5k scope confirmation:** The plan scopes T4-2.5k to PressReader IPC only (the cheapest Pillar 2 win beyond `/player` callbacks), with Fan/Coach/Scout panel UIs deferred. Is PressReader the right single reader to promote, or should it be CoachReader? Default: PressReader (most player-facing of the four).

---

## 7. Changes from Draft (Critic-driven)

**Completeness gaps fixed:**

1. **Pillar 1 cultural layer missing** → Added T4.5-B0 (`generate_nation` + `RegionPriors`) as a required predecessor to T4.5-B/C/E1. The cultural layer that drives name-bank selection and club-culture tags now has a generating row.

2. **T4.5-E gene→attribute dependency unresolved** → Split T4.5-E into T4.5-E0 (gene→PlayerAttributes forward compiler, dep T4-2.5a) and T4.5-E1 (apply compiler across pyramid, dep T4.5-B + T4.5-E0). The hidden multi-row epic is now explicit.

3. **Pillar 5 has no row completing the 24-signature catalogue for EA** → Resolved by DEMOTING the EA floor to 8-of-24 implemented (Q5 asks for confirmation). Full-24 implementation is post-EA in Deferred. T4-2.5j scope corrected to ≥8 predicates + all 24 authored in `design/signatures.md` with stubs.

4. **Non-existent `PlayerDebut` variant** → All T4-2.5e done-criteria corrected to use `DebutSenior (or DebutClub)`. `career-roster-layer.md §5` edit added to §5 doc-edits.

5. **Mod overlay orphan** → Decision 6 explicitly scopes mods out of EA. `mod_fingerprint` removed from T4.5-H done-criteria. T2-R-E3 deferred with "UNSCHEDULED — off critical path" tag. ADR-0010's mod_fingerprint is noted as a future additive field, not EA scope.

6. **T4.5-B fixed-size array refactor understated** → T4.5-B done-criteria now explicitly calls out the `&[ClubId; CLUBS_PER_LEAGUE]`→`&[ClubId]` structural refactor of `generate_fixtures` + `Standings` + invariant asserts, and names the authorized-rebaseline condition.

7. **Pillar 2 cross-decade callback not proven end-to-end** → T4-2.5L done-criteria now includes the explicit 8-season cross-compaction render proof. Added as T4 exit gate bullet 4.

8. **Player-club tier-mobility unspecified** → T4.5-F done-criteria extended to require that when the player's club promotes/relegates, the real engine continues for that club in its new tier, and career-overview reflects the change.

9. **Decision 5 deadline not flagged** → Decision 5 is now marked "log BEFORE T4-2.5b — the next row" in §4 with explicit urgency.

10. **CI-R1/R2 unresolved contradiction** → Resolved: confirmed wired at T1-13 (Justfile `audit:` + `deny:` recipes). Both marked DONE in §2.

**Scope-discipline cuts:**

11. **Pillar 4 balloon (T4-2.5m/n/p)** → Demoted to Deferred behind the explicit "Month-4 feel-prototype gate (DESIGN_DOC §13 OQ3)." Single-scout uncertainty (T4-2.5f) is the EA floor. T4 exit gate bullet 4 rewritten to "banded estimate from single scout," not multi-scout disagreement.

12. **Item budget cap** → Formally updated from ~55 to ~80 via Decision 7. The cap update is logged rather than silently exceeded.

13. **Pillar 5 gold-plating (T4-2.5q counterplay)** → Moved to Deferred behind "≥16 signatures live" trigger. Post-EA.

14. **Pillar 2 reader-wiring overreach (T4-2.5k full)** → T4-2.5k scoped to PressReader IPC only. Fan/Coach/Scout panel UIs moved to Deferred post-EA.

15. **SaveV4+V5 double-bump** → T4-2.5g scoped to minimal roster-only V4; world-gen descriptor fields deferred to a single V5 at T4.5-H. No double-bump on the critical path. T4-2.5n (scout track record save-state) eliminated along with T4-2.5m/n/p demotion — that migration cycle is gone.

16. **T4.5-C scope tightened** → Founding-history prose moved to T4.5-D (LLM-baked flavor). T4.5-C delivers names + crests + colors + culture tag + rivalry adjacency flag only.

17. **T4-9 Stretch→Deferred** → Cut from Stretch/TODO to Deferred per scope-discipline verdict. The plan cannot afford a viewer polish row while world-gen is unscheduled.
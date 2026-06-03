# Post-T4 ultimate review — 2026-06-03

Phase-close multi-track adversarial review for **Phase T4 — Pillar Wiring + Polish**.
Per `/done` Step 5.5. Codex budget is constrained this cycle, so the heavy
review is Claude-side (Tracks A–D); Codex gets a single focused determinism /
canonical-rebaseline pass (see the Codex prompt handed off separately).

## Scope

Every row shipped in the Phase-T4 window (incl. rows rolled in from earlier
phases / promoted from DEFERRED):

T4-1, T4-2, T4-3, T4-4, T4-5a, T4-6a, T4-2.5a, T4-sim-halt, T4-2.5b,
T4-2.5c (+ T1-25, T1-26), T4-2.5d, T4-2.5e, T4-2.5f, T4-F1, T4-F2,
T4-2.5g (SaveV4), T4-2.5h, T4-F4, T4-I1, QA-3, QA-5, T4-2.5i, T4-2.5j
(signature catalogue + **canonical-hash rebaseline** `206bddae…`→`12ce5ab7…`),
T4-2.5k (PressReader IPC + **memory-callback render bugfix**), T4-2.5L
(cross-decade proof + compaction corpus + CAREER_END_SEASON), T4-7 (game-shell).

Commit window: `b54ce0be` → `a787619a` (HEAD).
Verify state: `scripts/fw verify` exit 0; pins `85f45bf8…` (60-tick, UNCHANGED)
/ `12ce5ab7…` (600-tick, REBASELINED at T4-2.5j, authorized); 293 frontend
tests; banned-terms + cargo-deny clean. The per-task self-review triple ran on
every commit — so the value here is cross-task / whole-codebase / systemic.

---

## Track A — Mutation-test analysis (Claude code-explorer)

Mental mutation map; no `cargo-mutants` runs.

- **A-P0-1 — `poachers_dart` forward role-gate hole.** `in_team != 9` mutated to
  `in_team > 9` lets slots 0–9 through; slot-9 fire test still passes and the
  wrong-role test probes slot 8 (not 10), so it survives. Add
  `poachers_dart_trigger(&state, 10) == Q32::ZERO` (max attrs). (The other 4 new
  predicates' gates are killed by existing fire+reject tests.)
- **A-P0-2 — `select_career_end_collapse_player` identity untested (CONVERGENT w/ D-P0-1).**
  A `total_pressure > best_pressure` → `false` mutation (always pick first player)
  passes all tests — `regressive_collapse_emitted_at_career_end` only asserts a
  RegressiveCollapse *exists*, not *which* player. Highest-value gap.
- **A-P0-3 — `breakthrough_eval_watermark` advancement untested.** `commands.rs`
  setting the watermark to the pre-season value (the season-N events re-fire in
  N+1 bug) survives — no test asserts `watermark == ledger.len()` post-advance,
  nor a fire-exactly-once across two seasons.
- **A-P2-2 — `body_shield_fit_score_is_product_of_attributes` vacuous** (>0 && <1,
  not the exact product) — pre-existing T1 test; the 5 new T4-2.5j predicates use
  `assert_eq!(fit, expected)` and are better. Back-port the exact-product form.
- Well-covered (mutation-killing, no action): compaction `+5`/`<=` boundary; F1
  global-EventId keying; GATE_MIN_STAKES floor; cooldown `< tick`; SaveV4 wire
  tag `0x04`; affinity×fit softmax; the 100-ledger corpus tick-null invariant.

## Track B — Architectural drift, docs vs code (Claude code-reviewer)

Verdict: **code honors its contracts; drift is doc-side staleness.**

- **B-P1-1 — STATUS.md:27 "Last canonical hash" still says `206bddae…` "REBASELINED
  at T4-2.5c".** WRONG + self-contradicts STATUS.md:7/:23 (`12ce5ab7…` at T4-2.5j).
  The authoritative field a Codex reviewer would grep is the wrong one.
- **B-P1-2 — MASTER_PLAN:15-20 snapshot stale** ("T4-2.5h is NOW NEXT", DONE set
  ending at T4-2.5f, build-health hash `206bddae…`, "SaveEnvelope V0–V3"). The
  per-row DONE summaries are accurate; only the top-of-file snapshot lags.
- **B-P2 — gate bullet 8 wording** "stable since T4-2.5c rebaseline" → should be
  "since the T4-2.5j rebaseline" (still satisfiable; wording only).
- **B-P2 — dangling spec refs:** ADR-0010 + CLAUDE.md §5/§9 cite
  `docs/specs/save-migration-fixtures.md` + `save-format-benchmarks.md` which
  don't exist; the 4-test contract is honored + documented in CLAUDE.md §9 + the
  fixtures README. Repoint, or create the specs.
- **B-P2 — TS comment** `types.ts` says `PressItemDto.topic` "mirrors PressTopicDto
  on the Rust side" — no Rust `PressTopicDto` exists (the field is `String` mapped
  from `fw_memory::readers::PressTopic`). Repoint the comment.
- Verified HONORED (green): `design/signatures.md` ↔ `build_trigger_table` (exact,
  machine-guarded); ADR-0012 labeling on the T4-2.5j note; ADR-0010/SaveV4 4-test
  contract; Sim determinism RULES (no HashMap/f64/clock/RNG/async in T4 sim code;
  saturating_* justified); Tauri §2/§3/§5; DTO↔TS mirror; CHANGELOG current.

## Track C — Whole-codebase silent-failure sweep (Claude silent-failure-hunter)

Verdict: **substantially clean + fail-loud; no P0.**

- Verified clean: the T4-2.5k memory-callback fix is complete (all 3 callers log
  before the static fallback; empty-output guard intact); press-inbox per-item
  render logs (never silent-drops); CAREER_END None-roster branch logs; SaveV4
  migration uniformly fail-loud; regenerate-on-load overlay log::warn!s unmatched
  entries; scouting log::warn!s its only skip; frontend safeInvoke + all route
  resources log AND surface; no non-deterministic leak.
- **C-P2-1 — `get_career_overview_inner` champion-name `unwrap_or_default` blanks
  silently** (no log) when a past champion's club isn't in the current league
  snapshot — inconsistent with the adjacent callback loop that log::error!s the
  same condition. Add a log::warn!.
- **C-P2-2 — `advance_season_inner` empty-standings TitleWon skip is silent**
  (`if let Some(cid)` no-ops; blank champion name). Structurally unreachable (20-row
  league invariant) but it's a load-bearing pillar-2 ledger event that would
  vanish with no diagnostic if standings ever empty. Make the None arm log::error!
  (Sim/RULES §11 fail-loud). The one C-finding most worth fixing.
- **C-P2-3 — `PressTopic`→string is a hand-maintained array, not exhaustive match**
  (a 5th variant silently vanishes from the inbox; no compile error). Add
  `PressTopic::as_dto_str` w/ exhaustive match. (= the already-filed follow-up.)

## Track D — Test-the-tests (Claude qa-lead)

- **D-P0-1 — career-end player-identity untested (CONVERGENT w/ A-P0-2).** Top gap.
  Add `regressive_collapse_targets_most_pressured_player`.
- **D-P0-2 — `body-shield-pressure` RON `role_family: CentreBack` vs gate `in_team
  1..=7`** — fires for 13/22 players (DEF + all MID), not just CB. The
  `implemented_signature_role_family_agrees_with_predicate_gate` guard only checks
  the family slot FIRES, not that non-family slots are rejected. Pre-existing +
  the broad gate is documented in `design/signatures.md:60`, so likely intentional
  ("CB-or-mid shielding") — but add an exclusivity test (or tighten the RON family).
- **D-P1 — cross-decade proof doesn't assert the INJECTED event specifically renders**
  (could be crowded out of top-5 while other events keep the list non-empty);
  press-inbox AC1b doesn't assert `event_class == 22`; `five_season_…` uses
  `title_won >= 5` not `== 5`; the 5 new predicates lack a fit-varies-with-attrs test.
- **D-P2 — corpus survival-rate is structurally 100%** (season-0 stakes=ONE always
  beats later 0.5) so the ≥95% floor is never approached — add a stakes-inverted
  variant that exercises `DecayFunction::Never` as the actual driver;
  `scouting_wiring` early-return on empty bios should panic; 5-seed envelope
  `[0,7]` admits an all-zero-goals regression on the sanity seeds.
- Solid surfaces: SaveV4 4-test contract complete + non-vacuous; F1/AC3 ledger
  filters strong; `bedrock_pinned_test_is_not_ignored` meta-guard exemplary;
  corpus invariants (a)/(b) tight.

---

## Consolidated verdict — ACCEPT (with doc-hygiene fixed pre-hand-off + a QA-hardening row)

**No correctness bugs.** Across 4 tracks, zero P0 *behavior* defects — every P0 is
a **test-coverage hole** (the code is correct; the test doesn't pin it) and the
P1/P2s are doc-drift + unreachable-but-unlogged defensive blanks + test-strengthening.
That is a strong phase-close outcome, consistent with the per-task self-review
triple having run on every commit.

**Cross-track convergence (the highest-value signal):** `select_career_end_collapse_player`'s
most-pressured-player selection has zero identity coverage (A-P0-2 + D-P0-1) — both
the mutation lens and the coverage lens landed on it independently. It is the #1
thing to harden.

**Classification + disposition:**
- **Gate-blocker (fix before the Codex hand-off):** none on correctness. The
  doc-drift (B-P1-1/B-P1-2/B-P2 ×3) is fixed in this close — it would actively
  mislead the Codex reviewer grepping STATUS/MASTER_PLAN for the current hash.
  **DONE in this commit.**
- **Pre-next-phase recommended (land before T4.5's first /next):** the test
  gaps + the two unlogged silent-blanks + the `PressTopic::as_dto_str` hardening →
  tracked as MASTER_PLAN row **QA-T4H** (and the existing follow-up chips).
- **Note (likely intentional):** the body-shield broad gate vs its CB family label
  (D-P0-2) — documented in design/signatures.md; QA-T4H adds the exclusivity test
  + a decision on whether to tighten the RON family.

The phase is in good architectural standing. Hand to Codex for the focused
determinism / canonical-rebaseline pass; apply any Codex gate-blockers + the
QA-T4H batch before creating the `v0.4.0-polish` tag.

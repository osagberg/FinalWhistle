# Codex pre-T1-2b re-audit prompt (Tier 2, ADR-0015)

**Use this prompt verbatim** in a Codex CLI session targeting commits `c3945227..27920de6`. The prompt is read-only; Codex does not mutate code/docs. Findings come back as a structured report; Claude applies P0/P1 fixes via `/next` cycles.

---

```
CODEX PRE-T1-2b RE-AUDIT (Tier 2, focused)

Repo: /Users/vibelogic/dev/football
v1 archive (read-only): /Users/vibelogic/dev/football-archive
Commit range in scope: c3945227..27920de6 (after the audit landed +
through the 7-tranche remediation). Optionally also re-verify the prior
T1-1 commit 69f900b9 if you want to confirm Tranche 2 fixes resolved
those findings.

Read-only audit. DO NOT mutate code or docs.

================================================================================
CONTEXT
================================================================================

Final Whistle is a procedural-fantasy football management sim in Rust +
Tauri + SolidJS. Phase T1 (First Match) active. T1-1 (fw-content
schema lock) shipped at commit 69f900b9. Your full-project audit
2026-05-13 returned ~50 findings (1 P0 + 11 headline P1 + ~30 lower-
severity) at docs/audits/codex-full-audit-2026-05-13.md.

Claude has applied 7 remediation tranches in sequence (commits
eb0b952e + ccd4d20b + bf439f7 + fba546b + e54d59a + 1dc2fd0 + e79adb0 +
27920de6). The tranches:

- Tranche 1: P0 #[ignore] guard (3-layer: meta-test + CI grep + hook)
  + doc-drift cleanup + .claude/launch.json tracked + push to origin.
- Tranche 2: T1-1 schema follow-ups (AbilityCeiling::try_new +
  VISIBLE/HIDDEN/KNOWN attribute name split + Q32Inner hidden + RoleId
  try_new + validate_unit_range + buildup-speed range tightening).
- Tranche 3: 4 ADR amendments (ADR-0001 cadence 8 Hz → 4 Hz, ADR-0001/3
  personality vector 8 → 14, ADR-0005 event count → 30 + Compaction
  variant, ADR-0006 RNG ref) + 7 new ADRs (0009 RNG seed derivation,
  0010 save format, 0011 signature system, 0012 hash rebaseline policy,
  0013 licensed-data policy, 0014 runtime AI/content boundary, 0015
  phase-gate review policy).
- Tranche 4: 5 companion specs for T1-2b (docs/specs/tactic-fsm.md +
  bt-attribute-binding.md + decision-cadence-stagger.md + docs/design/
  xg-coefficients.md + personality-bias-weights.md).
- Tranche 5: MASTER_PLAN restructure — T1-2b split into 4 sub-rows
  (i ball physics / ii tactic FSM + stagger / iii FSM-of-BTs + utility
  + PlayerSeparation / iv signature dispatcher). T1-9 done-criteria
  expanded with PlayerSeparation invariants. T2-1 split-candidate noted.
  Cargo audit/deny pulled forward.
- Tranche 6: Real ContentStore::load_sources (was Ok(Self::default()))
  + fw-content-baker validate runs real validators (was stub) +
  Justfile verify-content uses real subcommand (was `||echo`-masked).
  verify-content wired into ci-local.
- Tranche 7: Workflow/rules cleanup (retired-command refs in SKILL.md
  + Frontend @apply rule clarified + Content schema_version + ID
  carve-outs + Unity-MCP cleanup queued in MEMORY).

================================================================================
AUDIT SCOPE — FOCUSED LANES (not the 10-lane full-project shape)
================================================================================

This is a Tier-2 audit per ADR-0015: focused on the remediation +
T1-2b readiness, not the full project.

LANE A — DID THE TRANCHES ACTUALLY CLOSE THE FINDINGS?

For each P0/P1 from your 2026-05-13 audit, verify the corresponding
remediation commit resolves it. Specifically:

A1. P0 #[ignore] hole. Read commit eb0b952e:
    - crates/fw-replay/tests/canonical_hash.rs has
      `bedrock_pinned_test_is_not_ignored` test that reads its own
      source via include_str! at compile time?
    - .github/workflows/determinism-gate.yml has the "Bedrock-test
      ignore-attr guard" step before the canonical-hash run?
    - .claude/hooks/canonical-hash-guard.sh grep now includes
      `crates/fw-replay/tests/canonical_hash.rs` + fixtures dir?
    - Three layers all independently fire? Try to construct a bypass
      that evades all three.

A2. P1 ContentStore::load_baked stub. Read commit e79adb0:
    - load_sources walks content/sources/{cultures,archetypes,
      role-affinities,players}/*.ron?
    - Loader is fail-closed (missing root → ContentLoadError, not
      empty default)?
    - File walk is sorted for cross-platform determinism?
    - load_baked delegates to load_sources (until T2-3)?
    - Tests cover the happy path + missing-root + determinism?

A3. P1 FW-VAL stubs fail-open. Read commit e79adb0:
    - fw-content-baker `validate` subcommand runs real validators?
    - RoleAffinityTable.invalid_roles + unknown_attribute_keys actually
      wired in?
    - PlayerAttributes.validate_unit_range actually wired in?
    - Justfile verify-content uses `validate` (not the broken
      `validate-content`)?
    - The `|| echo` swallow is gone?
    - verify-content is in ci-local now so scripts/fw verify exercises
      it?

A4. P1 AbilityCeiling unchecked. Read commit bf439f7:
    - try_new validates current ∈ [0,1], potential ∈ [0,1],
      current ≤ potential?
    - Fields are pub(crate)?
    - External crates can't construct via struct literal?

A5. P1 CA weights accept hidden fields. Read commit bf439f7:
    - VISIBLE_ATTRIBUTE_NAMES (38) split from HIDDEN_ATTRIBUTE_NAMES
      (17) + KNOWN_ATTRIBUTE_NAMES (55)?
    - RoleWeights::unknown_attribute_keys validates against VISIBLE
      only?
    - injury_proneness as a CA-weight key is rejected (test confirms)?

A6. P1 RNG seed inconsistency. Read commit fba546b:
    - ADR-0009 lands with seed_fn(match_seed, tick, layer, site)?
    - DESIGN_DOC + ADR-0001 + ADR-0003 + ADR-0006 all updated to the
      canonical signature?
    - 8 SeedLayer discriminants enumerated + non-overlapping?

A7. P1 8 Hz cadence math wrong. Read commit fba546b + e54d59a:
    - ADR-0001 amended to 4 Hz per-player decision runner?
    - 60/4 = 15 confirmed clean?
    - docs/specs/decision-cadence-stagger.md spells out the exact
      stagger algorithm + decision_slots: [u8; 22] in canonical state?
    - Influence maps stay at 8 Hz (independent cadence)?

A8. P1 T1-2b companion specs missing. Read commit e54d59a:
    - All 5 specs exist + each is substantive (not just a TOC)?
    - bt-attribute-binding.md names every BT site's attribute reads?
    - tactic-fsm.md has the 5-state transition table?
    - decision-cadence-stagger.md has the slot assignment math?
    - xg-coefficients.md has Phase-1 seeds + re-fit cadence?
    - personality-bias-weights.md has the full 7×8 k₁..k₁₄ table?

A9. P1 PlayerSeparation carry-forward. Read commit 1dc2fd0:
    - T1-2b-iii done-criteria explicitly lists 6 PlayerSeparation
      invariants (min distance, deterministic pair order, ball
      non-mutation, velocity preservation magnitude, zero-distance
      fallback, runner-order regression)?
    - T1-9 done-criteria adds PlayerSeparation invariants?

A10. P1 hooks not durable. Read .claude/hooks/canonical-hash-guard.sh
    (no commit changed it beyond eb0b952e's scope expansion). Codex
    flagged hooks as Claude-Code-only, not git-scoped. Was that
    addressed? Tranche 7 should have either installed git hooks too
    OR explicitly documented as "convenience only". If neither, this
    is still open.

A11. P1 local-main-ahead-of-origin. After Tranche 1 (commit ccd4d20b
    pushed origin to 20314655..ccd4d20b), confirm subsequent pushes
    keep CI proving HEAD. The user is expected to push 1dc2fd0..
    27920de6 after the full tranche run.

For any finding that ISN'T resolved, surface it as a NEW P0/P1.

LANE B — DO THE NEW ADRs (0009..0015) COMPOSE COHERENTLY?

7 new ADRs landed in Tranche 3. Run a consistency check across them
+ against the existing ADRs (0001..0008).

B1. ADR-0009 + ADR-0001 (cadence). seed_fn signature consistent in
    both? Layer discriminants documented identically?
B2. ADR-0010 + ADR-0002 + ADR-0005. Save format gates by
    schema_version; do PlayerTemplate / RoleAffinityTable / MemoryEvent
    have schema_version fields? ADR-0010 claims bincode 2 + zstd; does
    the workspace Cargo.toml reflect this yet OR is it a T2-9 deliverable
    (and is that deferral documented)?
B3. ADR-0011 + ADR-0002 + ADR-0005. SignatureCandidate on
    PlayerTemplate is T1-3 deferral; does T1-3 MASTER_PLAN row reflect
    this? MemoryEvent::SignatureFirstFired is the corresponding ledger
    event; is it in ADR-0005's 30-variant catalogue?
B4. ADR-0012 + the P0 fix. Hash rebaseline policy says the 3-layer
    guard is "untouched" by rebaselines; the eb0b952e fix implements
    those 3 layers; do they agree on what triggers a rebaseline?
B5. ADR-0013 + ADR-0014. Licensed-data policy + runtime AI boundary
    both reference content/baked/. Do they agree on bake-time vs
    runtime split?
B6. ADR-0015 + the workflow files. Three-tier review policy says Tier
    2 fires on 5 explicit criteria; do .claude/skills/next + .claude/
    skills/done + Justfile reflect this?

LANE C — DETERMINISM CHECK ON THE NEW CODE

C1. The new ContentStore::load_sources uses fs::read_dir (filesystem-
    dependent iteration). Does the implementation sort the result for
    cross-OS determinism? (Codex Tranche 6 review should confirm yes;
    re-verify.)
C2. The new validate_unit_range in PlayerAttributes uses a macro
    `check!`. Does it preserve declaration order (matches ADR-0002
    §Concrete shape verbatim)?
C3. The new VISIBLE_ATTRIBUTE_NAMES + HIDDEN_ATTRIBUTE_NAMES static
    asserts pin the array lengths against the count constants. Are
    they actually `const _: () = assert!`-form?
C4. Any new `unwrap` / `unwrap_or` / `expect("not implemented")` in
    Tranche 2 / Tranche 6 code that's a silent-failure risk?

LANE D — T1-2a + T1-2b-i READINESS

T1-2a (dev-tier 2D tactical board) is the next /next pick. Is it
ready to start?

D1. T1-2a's done-criteria in MASTER_PLAN currently says "Claude
    Preview workflow validated end-to-end" — does this depend on the
    Claude Preview MCP install (which is a queued user action per
    MEMORY)? Surface as a soft-block if the MCP isn't yet active.
D2. T1-2b-i (ball physics) is the first sub-row. Are its dependencies
    in place? Specifically:
    - fw-core::Q32 arithmetic primitives — yes
    - fw-content::TacticalArchetype.buildup_speed_factor_bps — yes (in
      canonical state? No — content type, not match state)
    - Pinned ball-physics-only canonical-hash sub-fixture path —
      noted in the T1-2b-i row done-criteria; does the path exist?
D3. The 5 companion specs (Tranche 4) — are they self-consistent? Did
    Claude get any cross-references wrong?
D4. bt-attribute-binding.md notes 3 "field-path corrections" inline
    (mental.work_rate → personality.work_rate, etc). Are those actually
    corrected in the binding tables, or just flagged?

LANE E — REMAINING P2/P3 FINDINGS FROM THE 2026-05-13 AUDIT

Cross-check the Tranche-1-through-7 commit log against the audit's
~30 lower-severity findings. Which P2/P3s are STILL open? Triage by
impact:
- High-impact P2 surfaces → recommend pulling forward.
- Low-impact P3 → defer to a backlog.

================================================================================
DELIVERABLE
================================================================================

Single Markdown report. Structure:

```markdown
# Pre-T1-2b Re-Audit Report (Codex, 2026-MM-DD)

## Executive summary (≤200 words)
[Tranche outcome verdict: green / yellow / red. The 5 most important
things — new P0s, residual P1s, T1-2a readiness.]

## Verdict per tranche (per Lane A)
[For each of the 11 P1 sub-findings in Lane A: CLOSED / PARTIALLY
CLOSED / OPEN, with file:line evidence.]

## New findings (P0/P1 only)
[Anything you found that the 2026-05-13 audit missed OR that the
remediation introduced. Full detail per finding.]

## ADR coherence check (Lane B)
[Pass/fail per ADR pair; flag any contradictions.]

## Determinism floor (Lane C)
[Pass/fail; flag any new leak vectors.]

## T1-2a readiness (Lane D)
[GO / NO-GO with prerequisites.]

## Residual P2/P3 backlog (Lane E)
[Punch list of remaining lower-severity findings ranked by impact.]

## Recommended next steps
[Numbered list of actions in priority order. Each: owner role (agent
type per CLAUDE.md §5), estimated lift, justification.]
```

Quality bar: same as the 2026-05-13 audit. Brutal, specific,
file:line-anchored, recommended-fix-shape on every P0/P1.

If the tranches genuinely resolved the prior findings without
introducing new P0/P1s, say so — don't manufacture work. The point
of this re-audit is to ratify the remediation OR catch its gaps, NOT
to find more things to do.

Begin.
```

---

## How to run this

1. Open Codex CLI in `/Users/vibelogic/dev/football`.
2. Paste the prompt above verbatim.
3. Wait for the report.
4. Paste the report back to Claude. Claude triages → applies P0/P1
   fixes via `/next` cycles → re-runs verify → pushes.
5. T1-2a starts only after the re-audit clears (no new P0/P1) OR after
   any new findings are fixed.

Per ADR-0015, this Tier-2 audit is invoked manually by the user. It's
not automated. Cost estimate: ~30 min of user time + 1 Codex CLI
session (~$5-15 in API spend).

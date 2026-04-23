---
description: Full regression — unity-check L2 + L3 + Unity Test Framework suite
argument-hint: "[update | audit | report]"
---

# /regression-suite — regression coverage + full test run

Maintains a curated list of tests that cover the game's critical paths and known failure points. Ensures every bug fix has a regression test that would have caught the original bug. Also triggers the full test run at L2 + L3.

**Phase:** 5-7. Run: after a bug fix, before a release gate (`/gate-check polish` requires it), as part of sprint close.

## Procedure

1. **Parse args:**
   - `update` — scan new bug fixes; add regression tests to suite manifest
   - `audit` — full audit of GDD critical paths vs existing coverage
   - `report` — read-only status snapshot (suitable for sprint reviews)
   - No arg — `update` if sprint active, else `audit`
2. **Load** `tests/regression-suite.md` if exists (prior manifest). Extract: registered tests, last-updated, any `STALE` / `QUARANTINED` flags.
3. **Scan recent bug fixes.**
   - `git log --since='last sprint' --grep='fix\|bug'` or `production/hotfixes/`
   - For each, find the test file that covers it. Missing → flag as **coverage gap**.
4. **GDD critical-path audit** (`audit` mode). For each GDD's Acceptance Criteria:
   - Does a test file assert that AC?
   - If no → gap in regression suite.
5. **Invoke `unity-check` skill at L2** (runtime) — run the current test suite through Unity Test Framework:
   - `Window > General > Test Runner` OR batchmode `-runTests -testPlatform EditMode/PlayMode`
   - Collect PASS / FAIL per test
6. **Invoke `unity-check` skill at L3** (visual) for stories of type Visual/Feel — scene captures vs golden images if golden-image convention exists.
7. **Classify results:**
   - **PASS** — test passes cleanly
   - **FAIL** — regression detected (must block advancement)
   - **FLAKY** — inconsistent (quarantine + flag for investigation)
   - **STALE** — test exists but references removed API
8. **Update `tests/regression-suite.md`:**
   - Add new bug-fix regression tests
   - Flip flags (STALE, QUARANTINED, FLAKY)
   - Coverage-gap list
9. **Report** verdict: PASS / CONCERNS (flaky only) / FAIL (any regression).

## If args provided

- `update` / `audit` / `report` — mode selector

## If no tests directory

Fail: "No `tests/` directory. Run `/test-setup` or create manually."

## If Unity Test Framework not installed

Warn: "UTF not detected in `Packages/manifest.json`. Regression suite can track manually-authored tests but can't execute Unity tests. Install UTF via Package Manager."

## Output

- `tests/regression-suite.md` (updated manifest)
- Console: verdict + per-test summary + coverage gaps

## Related

- Typical follow-ups (PASS): `/gate-check polish`, `/milestone-review`
- Typical follow-ups (FAIL): fix regressions; if bug-fix-without-test, author regression test before re-running
- Invokes agents: optionally `qa-lead` for test-strategy review
- Invokes skills: `.claude/skills/unity-check/SKILL.md` (L2 + L3)
- Reads files: `tests/**`, `production/hotfixes/**`, `design/gdd/**`, git log
- Writes files: `tests/regression-suite.md`

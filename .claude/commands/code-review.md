---
description: Architectural + quality code review on uncommitted/changed files
argument-hint: "[path-to-file-or-directory | --changed]"
---

# /code-review — code review

Architectural + quality review on specified files or changed files. Checks standards compliance, ADR alignment, SOLID, testability, perf. Delegates to specialist subagents where available.

**Phase:** 5-6. Run after `/dev-story` and before `/story-done`. Can also run on any uncommitted changes.

## Procedure

1. **Resolve target files.**
   - `<path>` → read that file or all files in that directory
   - `--changed` (default if no arg) → `git diff --name-only HEAD` + uncommitted file list
2. **Read `CLAUDE.md`** for project coding standards + locked tech stack.
3. **Read `docs/architecture/control-manifest.md`** if exists — layer rules.
4. **ADR compliance.** For each target file, grep for `ADR-NNN` or header comments referencing ADRs. For every referenced ADR, read its Decision + Consequences. Classify any deviation as:
   - **ARCHITECTURAL VIOLATION** (blocking)
   - **ADR DRIFT** (warning)
   - **MINOR DEVIATION** (info)
5. **Invoke plugin subagents** (preferred over hand-rolled review):
   - If `feature-dev:code-reviewer` plugin is installed → spawn via `Agent` tool with the target files + CLAUDE.md + control-manifest as context
   - If `pr-review-toolkit:code-reviewer` is installed → spawn in parallel for a second opinion
   - If neither → fall back to `lead-programmer` subagent (or `general-purpose` with LP persona)
6. **Standards checks** (specialist will cover most; cross-check):
   - Public methods have doc comments
   - Cyclomatic complexity ≤ 10 per method
   - No method > 40 lines (excluding data declarations)
   - Dependencies injected (no static singletons for game state)
   - Config values loaded from ScriptableObject data (not hardcoded)
7. **Architecture + SOLID checks:**
   - Dependency direction (engine ← gameplay, not reverse)
   - No circular asmdef refs
   - UI does not own game state
   - Single Responsibility per class
8. **Game-specific checks:**
   - Frame-rate independence (`Time.deltaTime`)
   - No allocations in Update/hot paths
   - Proper resource cleanup (Dispose, OnDestroy)
9. **Compile verification.** Invoke `.claude/skills/unity-check/SKILL.md` at L1 before finalizing review.
10. **Report.** Table: Severity | File:Line | Issue | Proposed fix. Output to `reviews/code-review-<date>.md`.

## If args provided

- `<path>` → scoped review
- `--changed` → uncommitted + HEAD diff

## If no changes

Report "No changed files. Use `<path>` arg to review a specific file." and stop.

## Output

- `reviews/code-review-<date>.md` with severity-sorted findings
- Console: summary count by severity

## Related

- Typical follow-ups: fix findings → `/story-done`
- Invokes agents: `feature-dev:code-reviewer`, `pr-review-toolkit:code-reviewer`, `lead-programmer` (fallback)
- Invokes skills: `.claude/skills/unity-check/SKILL.md` (L1)
- Reads files: target code files, `CLAUDE.md`, `control-manifest.md`, referenced ADRs
- Writes files: `reviews/code-review-<date>.md`

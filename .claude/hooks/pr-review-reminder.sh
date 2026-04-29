#!/usr/bin/env bash
# PreToolUse hook: pr-review-reminder.sh
#
# Event:     PreToolUse (matcher: Bash) — filters internally to git commit
# Purpose:   Soft-remind Claude to run pr-review-toolkit subagents on
#            substantial code commits per CLAUDE.md §6.3. Codex review
#            cycles keep finding issues these local subagents would catch
#            first — running them before commit closes the gap and makes
#            Codex review pages actually focused on cross-model insights.
# Exit:      Always 0. Soft reminder via stderr; commit is NEVER blocked.
# Deps:      git (already required by the rest of the workflow).
#
# stdin JSON: { "tool_name":"Bash", "tool_input":{"command":"...", ...} }

set +e

INPUT=$(cat 2>/dev/null)

# Bail if not a git commit. The PreToolUse Bash matcher fires on ALL
# bash calls — we filter inside the hook to keep settings.json simple.
case "$INPUT" in
  *'git'*'commit'*) ;;
  *) exit 0 ;;
esac

# Find the repo root from anywhere — Claude's CWD may not be it.
ROOT=$(git rev-parse --show-toplevel 2>/dev/null)
if [ -z "$ROOT" ] || [ ! -d "$ROOT" ]; then
  exit 0
fi

# Staged-diff stats. Empty = no staged changes (e.g. amend with no
# index changes); skip the reminder entirely.
stat=$(git -C "$ROOT" diff --cached --shortstat 2>/dev/null)
if [ -z "$stat" ]; then
  exit 0
fi

# Extract insertions count. The shortstat format is:
#   "N files changed, M insertions(+), L deletions(-)"
# (any of the three subterms may be absent for diff-only-deletions etc.)
insertions=$(echo "$stat" | grep -oE '[0-9]+ insertion' | grep -oE '[0-9]+')
[ -z "$insertions" ] && insertions=0

# Threshold: 100 lines added is the smell-test boundary CLAUDE.md §6.3
# names. Below this, the commit is small enough that a self-review pass
# is sufficient. Above this, pr-review-toolkit subagents earn their cost.
THRESHOLD=100
if [ "$insertions" -lt "$THRESHOLD" ]; then
  exit 0
fi

# Substantial code commits only — pure doc/SPEC/STATUS commits are
# correctly handled by the existing decisions-log + drift-check
# discipline; pr-review-toolkit's value is on actual code.
code_files=$(git -C "$ROOT" diff --cached --name-only 2>/dev/null \
  | grep -cE '\.(cs|py|sh|hlsl|shader|cginc|shadergraph|subshader|asmdef|csproj|slnx)$')
[ -z "$code_files" ] && code_files=0
if [ "$code_files" -lt 1 ]; then
  exit 0
fi

# Emit the reminder. stderr is visible to Claude; the commit proceeds
# regardless (exit 0 below).
cat >&2 <<EOF
=== pr-review-toolkit reminder ===
About to commit $insertions insertions across $code_files code file(s).

CLAUDE.md §6.3 mandates running pr-review-toolkit subagents on
substantial code changes BEFORE commit:

  Agent("pr-review-toolkit:silent-failure-hunter") on the diff
  Agent("pr-review-toolkit:type-design-analyzer")  on the diff
  Agent("feature-dev:code-reviewer")               on the diff

If you have already run them THIS commit cycle, proceed. Otherwise,
consider running them now — Codex's recent reviews keep finding
issues these subagents would catch first (and cheaper than a full
cross-model round-trip).

This is a soft reminder; commit is NOT blocked.
=================================
EOF

exit 0

#!/usr/bin/env bash
# Stop hook: session-stop.sh
#
# Event:     Stop (fires when Claude's turn completes for the session)
# Purpose:   Print an end-of-session summary — files modified, commits made,
#            remaining SPEC phase tasks. Read-only, informational.
# Exit:      0 always.
# Deps:      git, POSIX grep.
#
# NOTE:      Coexists with update-status-timestamp.sh (separate Stop hook).
#            This hook adds ~500ms overhead; disable by removing from settings.

set +e

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$ROOT" 2>/dev/null || exit 0

echo "=== Session summary ==="

if command -v git >/dev/null 2>&1 && [ -d "$ROOT/.git" ]; then
  # Files modified in working tree + index
  modified=$(git status --short 2>/dev/null)
  if [ -n "$modified" ]; then
    echo "Working tree changes:"
    echo "$modified" | sed 's/^/  /'
  else
    echo "Working tree: clean"
  fi

  # Commits in last 4 hours (session-y window)
  recent=$(git log --oneline --since="4 hours ago" 2>/dev/null)
  if [ -n "$recent" ]; then
    echo ""
    echo "Recent commits (last 4h):"
    echo "$recent" | sed 's/^/  /'
  fi
fi

# Remaining SPEC tasks in active phase
if [ -f "$ROOT/SPEC.md" ]; then
  remaining=$(awk '
    /🟡 ACTIVE/ { show=1; next }
    show && /^## / { exit }
    show && /^\s*-\s+\[ \]/ { print }
  ' "$ROOT/SPEC.md" 2>/dev/null)
  if [ -n "$remaining" ]; then
    echo ""
    echo "Remaining tasks in active phase:"
    echo "$remaining" | head -10 | sed 's/^/  /'
    total=$(echo "$remaining" | wc -l | tr -d ' ')
    if [ "$total" -gt 10 ]; then
      echo "  ... ($total total)"
    fi
  fi
fi

echo "======================="
exit 0

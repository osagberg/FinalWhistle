#!/usr/bin/env bash
# PreCompact hook: pre-compact.sh
#
# Event:     PreCompact (fires before conversation summarization)
# Purpose:   Snapshot current phase / active task / git state / WIP markers
#            to .claude/session-snapshots/pre-compact-<timestamp>.md so Claude
#            can re-orient after compaction even if STATUS/SPEC drift.
# Exit:      0 always. Fails open.
# Deps:      POSIX grep/sed, git.

set +e

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$ROOT" 2>/dev/null || exit 0

SNAPSHOT_DIR="$ROOT/.claude/session-snapshots"
mkdir -p "$SNAPSHOT_DIR" 2>/dev/null

TS=$(date +%Y%m%d_%H%M%S)
SNAP="$SNAPSHOT_DIR/pre-compact-${TS}.md"

{
  echo "# Pre-compact snapshot — $(date)"
  echo ""

  echo "## Active phase (SPEC.md)"
  if [ -f "$ROOT/SPEC.md" ]; then
    phase=$(grep -m1 '🟡 ACTIVE' "$ROOT/SPEC.md" 2>/dev/null | sed 's/\*\*//g' | sed 's/^[[:space:]]*//')
    if [ -n "$phase" ]; then
      echo "$phase"
    else
      echo "(no 🟡 ACTIVE marker)"
    fi
  else
    echo "(SPEC.md missing)"
  fi
  echo ""

  echo "## Current task (STATUS.md first 40 lines)"
  if [ -f "$ROOT/STATUS.md" ]; then
    head -n 40 "$ROOT/STATUS.md"
  else
    echo "(STATUS.md missing)"
  fi
  echo ""

  echo "## Git state"
  if command -v git >/dev/null 2>&1 && [ -d "$ROOT/.git" ]; then
    echo "- Branch: $(git rev-parse --abbrev-ref HEAD 2>/dev/null)"
    echo ""
    echo "### Recent commits"
    git log --oneline -5 2>/dev/null | sed 's/^/  /'
    echo ""
    echo "### Uncommitted changes"
    st=$(git status --short 2>/dev/null)
    if [ -n "$st" ]; then
      echo "$st" | sed 's/^/  /'
    else
      echo "  (clean)"
    fi
  fi
  echo ""

  echo "## WIP markers in design docs"
  wip=""
  if [ -d "$ROOT/design" ]; then
    wip=$(grep -rn -E 'TODO|WIP|PLACEHOLDER|\[TBD\]|\[TO BE' "$ROOT/design" 2>/dev/null | head -20)
  fi
  if [ -n "$wip" ]; then
    echo "$wip" | sed 's/^/  /'
  else
    echo "(none)"
  fi
} > "$SNAP" 2>/dev/null

echo "=== Pre-compact snapshot written ==="
echo "File: $SNAP"
echo "Claude: after compaction, post-compact.sh will point you back here."
echo "===================================="

# Keep only the 10 most recent snapshots
ls -1t "$SNAPSHOT_DIR"/pre-compact-*.md 2>/dev/null | tail -n +11 | while IFS= read -r old; do
  rm -f "$old"
done

exit 0

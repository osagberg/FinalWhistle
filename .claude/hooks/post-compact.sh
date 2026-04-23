#!/usr/bin/env bash
# PostCompact hook: post-compact.sh
#
# Event:     PostCompact (fires after conversation summarization)
# Purpose:   Re-orient Claude with most recent pre-compact snapshot + CLAUDE.md
#            + STATUS.md + active SPEC phase block, so the post-compact turn
#            has the same working context the pre-compact turn had.
# Exit:      0 always. Fails open.
# Deps:      POSIX grep/sed, ls.

set +e

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$ROOT" 2>/dev/null || exit 0

echo "=== Context restored after compaction ==="
echo ""

# --- 1. Most recent snapshot -------------------------------------------
SNAPSHOT_DIR="$ROOT/.claude/session-snapshots"
LATEST=""
if [ -d "$SNAPSHOT_DIR" ]; then
  LATEST=$(ls -1t "$SNAPSHOT_DIR"/pre-compact-*.md 2>/dev/null | head -1)
fi

if [ -n "$LATEST" ] && [ -f "$LATEST" ]; then
  echo "## Pre-compact snapshot: $LATEST"
  echo ""
  cat "$LATEST"
  echo ""
else
  echo "## No pre-compact snapshot found."
  echo ""
fi

# --- 2. CLAUDE.md top (contract reminder) ------------------------------
if [ -f "$ROOT/CLAUDE.md" ]; then
  echo "## CLAUDE.md (head — read the rest from disk)"
  head -n 30 "$ROOT/CLAUDE.md"
  echo ""
fi

# --- 3. STATUS.md ------------------------------------------------------
if [ -f "$ROOT/STATUS.md" ]; then
  echo "## STATUS.md (head)"
  head -n 30 "$ROOT/STATUS.md"
  echo ""
fi

# --- 4. Active phase block from SPEC.md --------------------------------
if [ -f "$ROOT/SPEC.md" ]; then
  echo "## SPEC.md — active phase block"
  # Print from the 🟡 ACTIVE line until the next phase heading or 30 lines max.
  awk '
    /🟡 ACTIVE/ { show=1 }
    show {
      print
      n++
      if (n > 1 && $0 ~ /^## /) { exit }
      if (n > 40) { exit }
    }
  ' "$ROOT/SPEC.md" 2>/dev/null
  echo ""
fi

echo "IMPORTANT: re-read the files above before continuing the prior task."
echo "========================================="
exit 0

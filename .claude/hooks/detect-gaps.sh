#!/usr/bin/env bash
# SessionStart (follow-up) hook: detect-gaps.sh
#
# Event:     SessionStart (or on-demand via skill)
# Purpose:   Surface missing design docs, stub-only folders, empty SPEC phases.
# Exit:      0 always. Prints warnings to stdout. Never blocks.
# Deps:      POSIX grep / find / wc. No jq. No python3 required.
#
# Reports gaps as warnings only; never fails the session. Intended to run after
# session-start.sh has already printed the live phase/task/git context.

set +e

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$ROOT" 2>/dev/null || exit 0

echo "=== Documentation gap probe ==="

GAPS=0

# --- Check 1: design/ is README-only or empty ---------------------------
if [ -d "$ROOT/design" ]; then
  design_files=$(find "$ROOT/design" -maxdepth 2 -type f -name "*.md" 2>/dev/null | wc -l | tr -d ' ')
  non_readme=$(find "$ROOT/design" -maxdepth 2 -type f -name "*.md" ! -iname "README.md" 2>/dev/null | wc -l | tr -d ' ')
  if [ "$design_files" -gt 0 ] && [ "$non_readme" -eq 0 ]; then
    echo "WARN: design/ contains only README.md — no substantive design docs yet."
    GAPS=$((GAPS + 1))
  fi
else
  echo "WARN: design/ directory missing — consider creating it for GDD/lore notes."
  GAPS=$((GAPS + 1))
fi

# --- Check 2: SPEC.md present but no active phase / no tasks ------------
if [ -f "$ROOT/SPEC.md" ]; then
  if ! grep -q '🟡 ACTIVE' "$ROOT/SPEC.md" 2>/dev/null; then
    echo "WARN: SPEC.md has no phase marked '🟡 ACTIVE' — /next will have nothing to pick up."
    GAPS=$((GAPS + 1))
  fi
  task_count=$(grep -cE '^\s*-\s+\[[ x]\]' "$ROOT/SPEC.md" 2>/dev/null || echo 0)
  task_count=$(echo "$task_count" | tr -d ' ')
  if [ "$task_count" -eq 0 ]; then
    echo "WARN: SPEC.md has no checkbox tasks yet — define phase tasks before /next."
    GAPS=$((GAPS + 1))
  fi
else
  echo "WARN: SPEC.md missing."
  GAPS=$((GAPS + 1))
fi

# --- Check 3: STATUS.md present ----------------------------------------
if [ ! -f "$ROOT/STATUS.md" ]; then
  echo "WARN: STATUS.md missing."
  GAPS=$((GAPS + 1))
fi

# --- Check 4: CHANGELOG.md present -------------------------------------
if [ ! -f "$ROOT/CHANGELOG.md" ]; then
  echo "WARN: CHANGELOG.md missing — append-only change log expected."
  GAPS=$((GAPS + 1))
fi

# --- Check 5: commonly expected docs -----------------------------------
for doc in PROJECT_CONTEXT.md TECH_APPROACH.md; do
  if [ ! -f "$ROOT/$doc" ]; then
    echo "WARN: $doc missing — referenced by CLAUDE.md source-of-truth map."
    GAPS=$((GAPS + 1))
  fi
done

if [ "$GAPS" -eq 0 ]; then
  echo "No gaps detected."
fi
echo "==============================="
exit 0

#!/usr/bin/env bash
# SessionStart hook: surface current project state at session boot.
#
# Purely read-only — no file writes, no git mutations. Output is shown to the
# user (and Claude) at session start so the first turn has the live context.
#
# Emits:
#   - Active phase from SPEC.md
#   - Current task from STATUS.md
#   - Last 3 git commits (branch + log)
#   - Phase-gate flags (missing STATUS.md timestamp, SPEC.md / CHANGELOG.md drift warnings)
#
# Cross-platform: POSIX bash, works on macOS (bash 3.2) and Linux.
set -u

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$ROOT" 2>/dev/null || exit 0

echo "=== Final Whistle — session context ==="

# --- Active phase (SPEC.md) ---------------------------------------------
if [ -f "$ROOT/SPEC.md" ]; then
  phase=$(LC_ALL=C.UTF-8 grep -m1 '🟡 ACTIVE' "$ROOT/SPEC.md" 2>/dev/null \
    | sed 's/^[[:space:]]*[-*+>#]\{1,\}[[:space:]]*//' \
    | sed 's/[[:space:]]*[^[:alnum:]]*🟡.*$//' \
    | sed 's/\*\*//g' \
    | sed 's/^[[:space:]]*//;s/[[:space:]]*$//' || true)
  if [ -n "$phase" ]; then
    echo "Phase: $phase"
  else
    echo "Phase: (no 🟡 ACTIVE marker found in SPEC.md)"
  fi
else
  echo "Phase: (SPEC.md missing)"
fi

# --- Current task (STATUS.md) -------------------------------------------
if [ -f "$ROOT/STATUS.md" ]; then
  task=$(grep -m1 -iE '^[[:space:]#*_-]*((currently working on)|(active task))[[:space:]]*:' "$ROOT/STATUS.md" 2>/dev/null \
    | sed 's/^[^:]*:[[:space:]]*//' \
    | sed 's/[[:space:]]*$//' || true)
  if [ -n "$task" ]; then
    echo "Task:  $task"
  fi
fi

# --- Git state ----------------------------------------------------------
if command -v git >/dev/null 2>&1 && [ -d "$ROOT/.git" ]; then
  branch=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || true)
  [ -n "$branch" ] && echo "Branch: $branch"

  echo ""
  echo "Recent commits:"
  git log --oneline -3 2>/dev/null | while IFS= read -r line; do
    echo "  $line"
  done
fi

# --- Current context scope ----------------------------------------------
# Reads .claude/.current-scope file (written by /bootstrap, /expand-studio,
# /deep-research, /contract-scope). Reports scope + hints at alternatives.
scope_file="$ROOT/.claude/.current-scope"
if [ -f "$scope_file" ]; then
  current_scope=$(tr -d '[:space:]' < "$scope_file" 2>/dev/null || echo "")
  if [ -n "$current_scope" ]; then
    echo ""
    case "$current_scope" in
      minimal)  echo "Scope: minimal (~15KB loaded) — /expand-studio or /contract-scope <target> to change" ;;
      standard) echo "Scope: standard (~80KB loaded) — typical 200K-tier default" ;;
      rich)     echo "Scope: rich (~150KB loaded) — recommended default for 1M-tier" ;;
      studio)   echo "Scope: studio (~300KB loaded) — extended agent roster + sprint management" ;;
      research) echo "Scope: research (~800KB loaded) — full reference library (Phase 0/2/8)" ;;
      *)        echo "Scope: $current_scope (unrecognized — check .claude/context-scopes.json)" ;;
    esac
  fi
fi

# --- Phase-gate flags ---------------------------------------------------
flags=0

# Flag 1: STATUS.md "Last updated" missing or stale-looking
if [ -f "$ROOT/STATUS.md" ]; then
  last_updated=$(grep -m1 '^\*\*Last updated\*\*:' "$ROOT/STATUS.md" 2>/dev/null | sed 's/.*: *//' | awk '{print $1}')
  if [ -z "$last_updated" ]; then
    echo ""
    echo "Flag: STATUS.md has no '**Last updated**:' line — timestamp hook has nothing to update."
    flags=$((flags + 1))
  fi
fi

# Flag 2: CHANGELOG.md exists?
if [ ! -f "$ROOT/CHANGELOG.md" ]; then
  echo ""
  echo "Flag: CHANGELOG.md missing."
  flags=$((flags + 1))
fi

# Flag 3: completed SPEC tasks without matching CHANGELOG lines (coarse probe)
if [ -f "$ROOT/SPEC.md" ] && [ -f "$ROOT/CHANGELOG.md" ]; then
  done_count=$(grep -cE '^\s*-\s+\[x\]' "$ROOT/SPEC.md" 2>/dev/null || echo 0)
  log_count=$(grep -cE '^\s*-' "$ROOT/CHANGELOG.md" 2>/dev/null || echo 0)
  if [ "$done_count" -gt 0 ] && [ "$log_count" -eq 0 ]; then
    echo ""
    echo "Flag: SPEC.md has completed tasks but CHANGELOG.md has no entries — run /refresh-docs."
    flags=$((flags + 1))
  fi
fi

if [ "$flags" -eq 0 ]; then
  :
fi

echo "============================================"
exit 0

#!/usr/bin/env bash
# Final Whistle — Claude Code statusline
#
# Reads a JSON blob on stdin (Claude provides session info) and emits ONE line.
#
# Displayed segments (left → right):
#   1. ctx:<pct>%         — context window used / total
#   2. <model>            — model display name (e.g., opus-4.7)
#   3. <phase>            — current SPEC.md phase marked "🟡 ACTIVE"
#   4. > <task>           — Currently-working-on line from STATUS.md (if present)
#   5. <branch><dirty?>   — git branch; trailing "*" if working tree dirty
#   6. ~$<cost>           — session cost estimate, if Claude provided one
#
# Customize:
#   - Reorder / drop segments by editing the final `printf` at the bottom.
#   - Add your own segment: compute it as a variable above, splice it in.
#   - Tested on bash 3.2+ (macOS default) and bash 4+ (Linux).
#
# Deps: bash, git (optional), jq (optional — grep fallback if absent), grep, sed.

set -u

input=$(cat)

# --- JSON parse (jq preferred, grep fallback) ----------------------------
if command -v jq >/dev/null 2>&1; then
  model=$(printf '%s' "$input" | jq -r '.model.display_name // .model.id // "unknown"')
  used_pct=$(printf '%s' "$input" | jq -r '.context_window.used_percentage // empty')
  cwd=$(printf '%s' "$input" | jq -r '.workspace.current_dir // .cwd // ""')
  cost=$(printf '%s' "$input" | jq -r '.cost.total_cost_usd // .cost.usd // empty')
else
  model=$(printf '%s' "$input" | grep -oE '"display_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/')
  used_pct=$(printf '%s' "$input" | grep -oE '"used_percentage"[[:space:]]*:[[:space:]]*[0-9.]+' | head -1 | sed 's/.*:[[:space:]]*//')
  cwd=$(printf '%s' "$input" | grep -oE '"current_dir"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | sed 's/.*"\([^"]*\)"$/\1/')
  cost=$(printf '%s' "$input" | grep -oE '"total_cost_usd"[[:space:]]*:[[:space:]]*[0-9.]+' | head -1 | sed 's/.*:[[:space:]]*//')
  [ -z "$model" ] && model="unknown"
fi

# Normalize Windows paths (Git Bash edge case).
cwd=$(printf '%s' "$cwd" | sed 's|\\|/|g')
[ -z "$cwd" ] && cwd="."

# --- Segment 1: context usage -------------------------------------------
if [ -n "$used_pct" ] && [ "$used_pct" != "null" ]; then
  ctx_seg="ctx:${used_pct}%"
else
  ctx_seg="ctx:--"
fi

# --- Segment 2: model ---------------------------------------------------
# Trim "claude-" prefix if present, for compactness.
model_seg=$(printf '%s' "$model" | sed 's/^claude-//')

# --- Segment 3: phase (parse SPEC.md for "🟡 ACTIVE") -------------------
phase_seg=""
spec_file="$cwd/SPEC.md"
if [ -f "$spec_file" ]; then
  # Grab the first line containing "🟡 ACTIVE" and extract a phase label.
  # Expected shape: "## Phase 3 — Unity Bootstrap 🟡 ACTIVE" or similar.
  raw_phase=$(grep -m1 '🟡 ACTIVE' "$spec_file" 2>/dev/null || true)
  if [ -n "$raw_phase" ]; then
    # Strip markdown list/heading markers, emphasis, trailing " 🟡 ACTIVE".
    # Strip (in order): leading markdown markers, trailing "🟡 ACTIVE" and
    # anything after it (including a separator em/en/hyphen dash), bold markers.
    phase_seg=$(LC_ALL=C.UTF-8 printf '%s' "$raw_phase" \
      | sed 's/^[[:space:]]*[-*+>#]\{1,\}[[:space:]]*//' \
      | sed 's/[[:space:]]*[^[:alnum:]]*🟡.*$//' \
      | sed 's/\*\*//g' \
      | sed 's/^[[:space:]]*//;s/[[:space:]]*$//')
  fi
fi
[ -z "$phase_seg" ] && phase_seg="no-active-phase"

# --- Segment 4: active task (STATUS.md) ---------------------------------
task_seg=""
status_file="$cwd/STATUS.md"
if [ -f "$status_file" ]; then
  # Look for "Currently working on:" or "Active task:" line; take text after the colon.
  raw_task=$(grep -m1 -iE '^[[:space:]#*_-]*((currently working on)|(active task))[[:space:]]*:' "$status_file" 2>/dev/null || true)
  if [ -n "$raw_task" ]; then
    task_seg=$(printf '%s' "$raw_task" \
      | sed 's/^[^:]*:[[:space:]]*//' \
      | sed 's/[[:space:]]*$//' \
      | cut -c1-60)
  fi
fi

# --- Segment 5: git branch + dirty --------------------------------------
git_seg=""
if command -v git >/dev/null 2>&1 && [ -d "$cwd/.git" -o -f "$cwd/.git" ]; then
  branch=$(cd "$cwd" && git rev-parse --abbrev-ref HEAD 2>/dev/null || true)
  if [ -n "$branch" ]; then
    dirty=""
    if ! (cd "$cwd" && git diff --quiet --ignore-submodules HEAD 2>/dev/null); then
      dirty="*"
    fi
    git_seg="${branch}${dirty}"
  fi
fi

# --- Segment 6: cost ----------------------------------------------------
cost_seg=""
if [ -n "$cost" ] && [ "$cost" != "null" ] && [ "$cost" != "empty" ]; then
  # Format to 2 decimal places.
  cost_seg=$(printf '~$%.2f' "$cost" 2>/dev/null || true)
fi

# --- Assemble (skip empty segments) -------------------------------------
out="$ctx_seg | $model_seg | $phase_seg"
[ -n "$task_seg" ] && out="$out > $task_seg"
[ -n "$git_seg" ]  && out="$out | $git_seg"
[ -n "$cost_seg" ] && out="$out | $cost_seg"

printf '%s' "$out"

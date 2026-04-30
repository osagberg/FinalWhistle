#!/usr/bin/env bash
# PreToolUse hook: validate-commit.sh
#
# Event:     PreToolUse (matcher: Bash)
# Purpose:   Inspect `git commit` commands for secrets, JSON validity,
#            decisions-log preservation, TODO/FIXME markers.
# Exit:      0 allow (default, even with warnings).
#            2 block ONLY when hardcoded secrets are detected.
# Deps:      python3 for JSON validation (fallback: skip JSON check if absent).
#            No jq dependency — JSON parsed via python3 or grep.
#
# stdin JSON: { "tool_name": "Bash", "tool_input": { "command": "..." } }

INPUT=$(cat)
ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"

# Parse command field
if command -v python3 >/dev/null 2>&1; then
  COMMAND=$(printf '%s' "$INPUT" | python3 -c 'import sys,json
try: d=json.load(sys.stdin); print((d.get("tool_input") or {}).get("command",""))
except: pass' 2>/dev/null)
else
  COMMAND=$(printf '%s' "$INPUT" | grep -oE '"command"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/"command"[[:space:]]*:[[:space:]]*"//;s/"$//')
fi

# Only process git commit commands
if ! echo "$COMMAND" | grep -qE '(^|[[:space:]])git[[:space:]]+commit'; then
  exit 0
fi

cd "$ROOT" 2>/dev/null || exit 0

STAGED=$(git diff --cached --name-only 2>/dev/null)
[ -z "$STAGED" ] && exit 0

WARNINGS=""
SECRETS_FOUND=""

# --- Check 1: hardcoded secret patterns (BLOCKING) ----------------------
SECRET_PATTERNS='(AKIA[0-9A-Z]{16}|sk-[A-Za-z0-9]{20,}|ghp_[A-Za-z0-9]{20,}|glpat-[A-Za-z0-9_-]{20,}|xox[abpr]-[A-Za-z0-9-]{10,}|AIza[0-9A-Za-z_-]{35}|-----BEGIN (RSA |OPENSSH |EC |DSA )?PRIVATE KEY-----|Bearer[[:space:]]+[A-Za-z0-9._-]{20,}|api[_-]?key[[:space:]]*[:=][[:space:]]*["\x27][A-Za-z0-9_-]{16,})'

while IFS= read -r file; do
  [ -z "$file" ] && continue
  [ -f "$file" ] || continue
  case "$file" in
    *.png|*.jpg|*.jpeg|*.gif|*.webp|*.pdf|*.zip|*.gz|*.tar|*.exe|*.dll|*.so|*.dylib) continue ;;
  esac
  if hit=$(grep -nIE "$SECRET_PATTERNS" "$file" 2>/dev/null | head -3); then
    if [ -n "$hit" ]; then
      SECRETS_FOUND="$SECRETS_FOUND
  $file:
$hit"
    fi
  fi
done <<< "$STAGED"

if [ -n "$SECRETS_FOUND" ]; then
  {
    echo "=== BLOCKED: potential secret detected in staged files ==="
    echo "$SECRETS_FOUND"
    echo ""
    echo "Unstage the file or redact the value before committing."
    echo "If false-positive, commit via terminal directly (hook only runs for Claude)."
  } >&2
  exit 2
fi

# --- Check 2: JSON files parse (WARNING) --------------------------------
JSON_FILES=$(echo "$STAGED" | grep -E '\.json$' || true)
if [ -n "$JSON_FILES" ] && command -v python3 >/dev/null 2>&1; then
  while IFS= read -r file; do
    [ -z "$file" ] && continue
    [ -f "$file" ] || continue
    if ! python3 -m json.tool "$file" >/dev/null 2>&1; then
      WARNINGS="$WARNINGS
  JSON: $file fails to parse — fix before merging."
    fi
  done <<< "$JSON_FILES"
fi

# --- Check 3: SPEC.md decisions log preserved (BLOCKING) ---------------
# Codex P1 round 4 (2026-04-30): a Write that wholesale rewrites SPEC.md
# can pass the PreToolUse hook; commit time is the catch-all. Compare the
# staged SPEC.md against HEAD's version line-by-line; any decision-log
# bullet present in HEAD but missing from staged → block the commit.
SPEC_DELETIONS=""
SPEC_INTEGRITY_ERROR=""
if echo "$STAGED" | grep -qE '(^|/)SPEC\.md$'; then
  # Use mktemp for race-safety. Fixed paths (/tmp/.bp_spec_head etc.)
  # would stomp each other on concurrent hook invocations (CI runner +
  # IDE git-commit + fw verify), causing one run to read another run's
  # SPEC.md pre-image (per feature-dev:code-reviewer 2026-04-30 review).
  spec_head_tmp=$(mktemp -t fw-spec-head.XXXXXX) || spec_head_tmp=""
  spec_staged_tmp=$(mktemp -t fw-spec-staged.XXXXXX) || spec_staged_tmp=""
  if [ -n "$spec_head_tmp" ] && [ -n "$spec_staged_tmp" ]; then
    if git show "HEAD:SPEC.md" >"$spec_head_tmp" 2>/dev/null; then
      # Build the staged post-image (cwd version is normally identical, but
      # check `:0:SPEC.md` for accuracy if files have unstaged tweaks).
      if git show ":0:SPEC.md" >"$spec_staged_tmp" 2>/dev/null; then
        missing_lines=""
        missing_count=0
        while IFS= read -r line; do
          case "$line" in
            "- **"[0-9][0-9][0-9][0-9]-*)
              if ! grep -qF -- "$line" "$spec_staged_tmp" 2>/dev/null; then
                missing_count=$((missing_count + 1))
                if [ "$missing_count" -le 3 ]; then
                  missing_lines="$missing_lines
    $line"
                fi
              fi
              ;;
          esac
        done < "$spec_head_tmp"
        if [ "$missing_count" -gt 0 ]; then
          SPEC_DELETIONS="$missing_count decisions-log entry/entries removed from SPEC.md.$missing_lines"
        fi
      else
        # Staged-read failed despite SPEC.md being in $STAGED — index race,
        # interrupted `git add`, or partial index. Fail-closed posture per
        # pr-review-toolkit:silent-failure-hunter 2026-04-30 P2 #4.
        SPEC_INTEGRITY_ERROR="Could not read staged SPEC.md (':0:SPEC.md') for the append-only integrity check."
      fi
    fi
  fi
  # Clean up tempfiles unconditionally; mktemp creation may have failed.
  [ -n "$spec_head_tmp" ] && rm -f "$spec_head_tmp"
  [ -n "$spec_staged_tmp" ] && rm -f "$spec_staged_tmp"
fi

if [ -n "$SPEC_INTEGRITY_ERROR" ]; then
  {
    echo "=== BLOCKED: SPEC.md decisions-log integrity check failed ==="
    echo ""
    echo "$SPEC_INTEGRITY_ERROR"
    echo ""
    echo "Possible causes: index race (interrupted git add), partial index,"
    echo "filesystem race. Re-stage SPEC.md (\`git add SPEC.md\`) and retry."
    echo "If this is a deliberate audited rewrite, commit through the"
    echo "terminal (this hook only runs in Claude Code sessions)."
  } >&2
  exit 2
fi

if [ -n "$SPEC_DELETIONS" ]; then
  {
    echo "=== BLOCKED: SPEC.md decisions log is append-only ==="
    echo ""
    echo "$SPEC_DELETIONS"
    echo ""
    echo "Restore the missing bullets and re-stage. To supersede a prior"
    echo "decision, append a NEW entry at the end with 'Supersedes: <prior date>'."
    echo "See SETUP.md and the /log-decision skill."
    echo ""
    echo "If this is a deliberate, audited rewrite (rare), commit via terminal"
    echo "(this hook only runs in Claude Code sessions)."
  } >&2
  exit 2
fi

# --- Check 4: TODO/FIXME/HACK markers (WARNING) -------------------------
while IFS= read -r file; do
  [ -z "$file" ] && continue
  [ -f "$file" ] || continue
  case "$file" in
    *.md|*.mdc|*.txt|*.yaml|*.yml|*.json) continue ;;
  esac
  if hits=$(grep -nE '(TODO|FIXME|HACK|XXX)([^a-zA-Z0-9]|$)' "$file" 2>/dev/null | head -2); then
    if [ -n "$hits" ]; then
      WARNINGS="$WARNINGS
  TODO: $file has TODO/FIXME/HACK marker(s)."
    fi
  fi
done <<< "$STAGED"

if [ -n "$WARNINGS" ]; then
  {
    echo "=== Commit validation warnings (non-blocking) ==="
    echo "$WARNINGS"
    echo "================================================"
  } >&2
fi

exit 0

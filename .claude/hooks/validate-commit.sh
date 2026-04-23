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

# --- Check 3: SPEC.md decisions log preserved (belt+braces) ------------
if echo "$STAGED" | grep -qE '(^|/)SPEC\.md$'; then
  # Cheap probe: verify HEAD's decision lines still appear in staged version.
  if git show "HEAD:SPEC.md" >/tmp/.bp_spec_head 2>/dev/null; then
    missing=0
    while IFS= read -r line; do
      case "$line" in
        "- **"[0-9][0-9][0-9][0-9]-*)
          if ! git diff --cached -- SPEC.md | grep -qF -- "$line" && \
             ! grep -qF -- "$line" SPEC.md 2>/dev/null; then
            missing=$((missing + 1))
          fi
          ;;
      esac
    done < /tmp/.bp_spec_head
    rm -f /tmp/.bp_spec_head
    if [ "$missing" -gt 0 ]; then
      WARNINGS="$WARNINGS
  SPEC: ~$missing decisions-log line(s) appear removed from SPEC.md. Log is append-only."
    fi
  fi
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

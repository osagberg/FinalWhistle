#!/usr/bin/env bash
# PreToolUse hook: validate-push.sh
#
# Event:     PreToolUse (matcher: Bash)
# Purpose:   Warn on pushes to protected branches / force-pushes to main.
#            Never blocks — user explicitly opted out of hard block by dropping
#            donchitos' `git push --force` deny from permissions.deny.
# Exit:      0 always. Warnings to stderr.
# Deps:      git, POSIX grep/sed.
#
# stdin JSON: { "tool_name": "Bash", "tool_input": { "command": "git push ..." } }

INPUT=$(cat)

if command -v python3 >/dev/null 2>&1; then
  COMMAND=$(printf '%s' "$INPUT" | python3 -c 'import sys,json
try: d=json.load(sys.stdin); print((d.get("tool_input") or {}).get("command",""))
except: pass' 2>/dev/null)
else
  COMMAND=$(printf '%s' "$INPUT" | grep -oE '"command"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/"command"[[:space:]]*:[[:space:]]*"//;s/"$//')
fi

if ! echo "$COMMAND" | grep -qE '(^|[[:space:]])git[[:space:]]+push'; then
  exit 0
fi

CURRENT_BRANCH=$(git rev-parse --abbrev-ref HEAD 2>/dev/null || echo "")

PROTECTED=""
for branch in main master develop; do
  if [ "$CURRENT_BRANCH" = "$branch" ]; then
    PROTECTED="$branch"
    break
  fi
  if echo "$COMMAND" | grep -qE "[[:space:]]${branch}([[:space:]]|$)"; then
    PROTECTED="$branch"
    break
  fi
done

# release/* match
if [ -z "$PROTECTED" ]; then
  if [ "${CURRENT_BRANCH#release/}" != "$CURRENT_BRANCH" ]; then
    PROTECTED="$CURRENT_BRANCH"
  elif echo "$COMMAND" | grep -qE '[[:space:]]release/[A-Za-z0-9._/-]+'; then
    PROTECTED=$(echo "$COMMAND" | grep -oE 'release/[A-Za-z0-9._/-]+' | head -1)
  fi
fi

FORCE=0
if echo "$COMMAND" | grep -qE '(--force([[:space:]]|=|$)|(^|[[:space:]])-f([[:space:]]|$)|--force-with-lease)'; then
  FORCE=1
fi

if [ -n "$PROTECTED" ]; then
  {
    echo "=== git push warning ==="
    echo "Pushing to protected branch: $PROTECTED"
    if [ "$FORCE" = "1" ]; then
      if [ "$PROTECTED" = "main" ] || [ "$PROTECTED" = "master" ]; then
        echo "HIGH SEVERITY: force-push to $PROTECTED detected."
        echo "  This rewrites history shared with collaborators."
        echo "  Proceed only if you are certain no one else has pulled recent commits."
      else
        echo "WARNING: force-push to $PROTECTED. Ensure teammates are aware."
      fi
    else
      echo "Reminder: ensure tests pass + no S1/S2 bugs before push lands."
    fi
    echo "========================"
  } >&2
fi

exit 0

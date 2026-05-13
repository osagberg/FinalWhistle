#!/usr/bin/env bash
# validate-commit.sh — PreToolUse hook on Bash(git commit*).
#
# Catches commit anti-patterns before they land:
#   1. --amend (CLAUDE.md §4.5 forbids; always create new commits)
#   2. --no-verify (bypasses hooks)
#   3. -i / interactive flags (interactive input not supported)
#   4. Obvious secrets in the staged diff (AWS keys, GitHub tokens,
#      Anthropic API keys, generic high-entropy 32+ char sk- patterns)
#   5. Mutation of historical entries in docs/DECISIONS.md
#      (the protect-decisions hook also catches this on Edit/Write;
#      this is a defence-in-depth at commit time)
#
# Hook contract:
#   - PreToolUse on Bash, matcher filters to `git commit`.
#   - Reads tool input as JSON on stdin (command field).
#   - Exit 0 = allow. Exit 2 = block with stderr message.
#
# Designed to be fast (<200ms) — pure bash + one short python, no cargo.

set -euo pipefail

HOOK_INPUT="$(cat)"

# Extract the bash command from the JSON input.
COMMAND="$(printf '%s' "$HOOK_INPUT" | python3 -c '
import json, sys
try:
    data = json.load(sys.stdin)
    print(data.get("tool_input", {}).get("command", ""))
except Exception:
    print("")
' 2>/dev/null || echo "")"

# Only inspect commands that look like git commit.
case "$COMMAND" in
  *"git commit"*) : ;;
  *) exit 0 ;;
esac

# Block 1: --amend
if printf '%s' "$COMMAND" | grep -q -- '--amend'; then
  echo "BLOCKED: --amend is forbidden by CLAUDE.md §4.5. Create a new commit instead." >&2
  echo "If you need to fix the last commit, run: git revert HEAD && /next" >&2
  exit 2
fi

# Block 2: --no-verify
if printf '%s' "$COMMAND" | grep -q -- '--no-verify'; then
  echo "BLOCKED: --no-verify bypasses hooks. CLAUDE.md §4.5 forbids." >&2
  echo "Fix the root cause: either the hook is wrong (open an issue) or the commit is wrong." >&2
  exit 2
fi

# Block 3: -i interactive
if printf '%s' "$COMMAND" | grep -qE -- '(^| )-i( |$)|--interactive'; then
  echo "BLOCKED: interactive git is not supported in Claude Code sessions." >&2
  exit 2
fi

# Block 4: secrets in staged diff
PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$PROJECT_DIR"

STAGED_DIFF="$(git diff --cached 2>/dev/null || true)"
if [ -n "$STAGED_DIFF" ]; then
  if printf '%s' "$STAGED_DIFF" | grep -qE 'AKIA[0-9A-Z]{16}'; then
    echo "BLOCKED: staged diff contains an AWS access key pattern (AKIA...)." >&2
    exit 2
  fi
  if printf '%s' "$STAGED_DIFF" | grep -qE 'ghp_[A-Za-z0-9]{36}'; then
    echo "BLOCKED: staged diff contains a GitHub PAT pattern (ghp_...)." >&2
    exit 2
  fi
  if printf '%s' "$STAGED_DIFF" | grep -qE 'sk-ant-[A-Za-z0-9_-]{20,}'; then
    echo "BLOCKED: staged diff contains an Anthropic API key pattern (sk-ant-...)." >&2
    exit 2
  fi
  if printf '%s' "$STAGED_DIFF" | grep -qE '\bsk-[A-Za-z0-9]{40,}\b'; then
    echo "BLOCKED: staged diff contains a generic API-key-looking pattern (sk-...)." >&2
    exit 2
  fi
fi

# Block 5: DECISIONS.md historical mutation defence-in-depth.
if echo "$STAGED_DIFF" | grep -qE '^-\s*-\s*\*\*[0-9]{4}-'; then
  echo "BLOCKED: staged diff deletes a dated bullet in docs/DECISIONS.md." >&2
  echo "DECISIONS.md is append-only. To change a prior decision, append a new entry that supersedes it (cite the prior bullet verbatim)." >&2
  exit 2
fi

exit 0

#!/usr/bin/env bash
# pr-review-reminder.sh — Stop hook (soft reminder, non-blocking).
#
# Reminds when the last commit was ≥100 LoC of code without the
# mandatory self-review footer.
#
# CLAUDE.md §5 mandates pr-review-toolkit self-review on any commit
# ≥100 LoC of code. This hook does NOT enforce — it reminds. The binding
# rule is in CLAUDE.md §5; this is a soft check so a missed self-review
# doesn't slip past the next session boundary unnoticed.
#
# Hook contract:
#   - Stop hook, fires at end of every Claude turn.
#   - Always exits 0 (non-blocking).
#   - Writes a yellow reminder to stderr if the trigger condition fires.

set -euo pipefail

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$PROJECT_DIR" 2>/dev/null || exit 0

git rev-parse --is-inside-work-tree >/dev/null 2>&1 || exit 0
git rev-parse HEAD >/dev/null 2>&1 || exit 0

LAST_SUBJECT="$(git log -1 --pretty=%s 2>/dev/null || echo "")"
LAST_BODY="$(git log -1 --pretty=%B 2>/dev/null || echo "")"

# Skip merge commits + initial commits.
case "$LAST_SUBJECT" in
  Merge\ *|"") exit 0 ;;
esac

# Skip if body already mentions self-review.
case "$LAST_BODY" in
  *[Ss]elf-review:*) exit 0 ;;
  *self-review*) exit 0 ;;
esac

# Count code LoC in last commit. Code = .rs, .ts, .tsx, .js source files.
CODE_LOC="$(git show --numstat HEAD 2>/dev/null \
  | awk '$3 ~ /\.(rs|ts|tsx|js)$/ { added += $1; removed += $2 } END { print (added + removed) }' \
  || echo 0)"
CODE_LOC="${CODE_LOC:-0}"

if [ "$CODE_LOC" -ge 100 ]; then
  if command -v tput >/dev/null 2>&1; then
    YELLOW="$(tput setaf 3 2>/dev/null || echo "")"
    RESET="$(tput sgr0 2>/dev/null || echo "")"
  else
    YELLOW=""
    RESET=""
  fi
  cat >&2 <<EOF
${YELLOW}reminder: last commit was ${CODE_LOC} LoC of code without a "Self-review:" footer.
CLAUDE.md §5 mandates pr-review-toolkit + feature-dev:code-reviewer on >=100 LoC.
Next session should either (a) confirm review happened and add a follow-up commit
noting it, or (b) run the three subagents now on the committed diff.${RESET}
EOF
fi

exit 0

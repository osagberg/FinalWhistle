#!/bin/bash
# Stop hook: update STATUS.md "Last updated" line to today.
#
# Ported verbatim from FW v1's .claude/hooks/update-status-timestamp.sh.
# Keeps the "Last updated" line honest without Claude having to remember
# every session. Wired by .claude/settings.json (Stop event, no matcher).
#
# Layout assumption: STATUS.md contains a line of the form:
#     **Last updated**: YYYY-MM-DD [optional trailing notes]
# Anything matching that pattern gets its date replaced with today's
# date in YYYY-MM-DD form. The trailing notes are preserved.

set -euo pipefail

STATUS_FILE="${CLAUDE_PROJECT_DIR:-$(pwd)}/STATUS.md"
[ -f "$STATUS_FILE" ] || exit 0

TODAY="$(date +%Y-%m-%d)"

# Replace the YYYY-MM-DD after "**Last updated**: ", preserving any
# trailing annotation on the same line.
# macOS sed: -i needs an empty '' argument.
# Linux sed: use sed -i -E instead (remove the empty '' arg).
if [[ "$OSTYPE" == "darwin"* ]]; then
  sed -i '' -E "s/^(\*\*Last updated\*\*: )[0-9]{4}-[0-9]{2}-[0-9]{2}/\1${TODAY}/" "$STATUS_FILE"
else
  sed -i -E "s/^(\*\*Last updated\*\*: )[0-9]{4}-[0-9]{2}-[0-9]{2}/\1${TODAY}/" "$STATUS_FILE"
fi

exit 0

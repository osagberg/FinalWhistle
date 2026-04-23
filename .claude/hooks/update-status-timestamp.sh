#!/bin/bash
# Stop hook: update STATUS.md "Last updated" line to today.
#
# Keeps the "Last updated" line honest without Claude having to remember every
# session. The simple timestamp update is the high-value half of a potential
# broader SPEC-vs-CHANGELOG drift check (drift check is deferred).
set -euo pipefail

STATUS_FILE="${CLAUDE_PROJECT_DIR:-$(pwd)}/STATUS.md"
[ -f "$STATUS_FILE" ] || exit 0

TODAY="$(date +%Y-%m-%d)"

# Replace the YYYY-MM-DD after "**Last updated**: ", preserving any trailing
# annotation on the same line.
# macOS sed: -i needs an empty '' argument.
# Linux sed: use sed -i -E instead (remove the empty '' arg).
if [[ "$OSTYPE" == "darwin"* ]]; then
  sed -i '' -E "s/^(\*\*Last updated\*\*: )[0-9]{4}-[0-9]{2}-[0-9]{2}/\1${TODAY}/" "$STATUS_FILE"
else
  sed -i -E "s/^(\*\*Last updated\*\*: )[0-9]{4}-[0-9]{2}-[0-9]{2}/\1${TODAY}/" "$STATUS_FILE"
fi

exit 0

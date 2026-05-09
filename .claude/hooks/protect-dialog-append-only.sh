#!/usr/bin/env bash
# protect-dialog-append-only.sh — PreToolUse hook on Edit/Write/MultiEdit.
#
# Purpose: enforce the agent-bus append-only invariant. Topic files at
# dialog/*.jsonl are immutable line-by-line; only line additions at
# end-of-file are permitted. Any tool call that would modify a prior line
# is BLOCKED with exit 2.
#
# This hook auto-promoted from P2 to P1 on 2026-05-10 per the mutual-fade
# closure of agent-bus topic 2026-05-09-mcp-migration-debate (item B-1 of
# the workflow-cleanup commit), triggered by the SPEC decisions-log entry
# at SPEC.md citing dialog/<topic>.jsonl in its review-trail bullet.
#
# Spec: docs/tooling/agent-bus-spec.md §6 (append-only enforcement).
#
# Hook contract:
#   - Reads tool input as JSON on stdin (Claude Code PreToolUse format).
#   - Exits 0 if the tool call is allowed.
#   - Exits 2 with stderr message if the tool call is blocked.
#   - Any other exit code is treated as an internal error (allow + warn).

set -euo pipefail

# Read the tool input JSON from stdin into a variable for inspection.
INPUT="$(cat)"

# Skip if jq is unavailable — fail-open with a warning rather than blocking.
if ! command -v jq >/dev/null 2>&1; then
    printf 'protect-dialog-append-only.sh: jq not found; allowing tool call (install jq for enforcement)\n' >&2
    exit 0
fi

# Extract tool name and the relevant file path from the input.
# Claude Code's PreToolUse hook input includes:
#   { "tool_name": "Edit"|"Write"|"MultiEdit", "tool_input": { "file_path": "...", ... } }
TOOL_NAME="$(printf '%s' "$INPUT" | jq -r '.tool_name // ""')"
FILE_PATH="$(printf '%s' "$INPUT" | jq -r '.tool_input.file_path // ""')"

# Only inspect tool calls targeting dialog/*.jsonl files.
case "$FILE_PATH" in
    */dialog/*.jsonl|dialog/*.jsonl)
        ;;
    *)
        exit 0
        ;;
esac

# Edit and MultiEdit modify in place — by definition they touch prior lines.
# Block both. If a Claude session legitimately needs to ADD to a topic file,
# it should call scripts/agent-bus post (which uses an atomic mv-of-tmp
# pattern that doesn't go through Edit/MultiEdit hooks).
case "$TOOL_NAME" in
    Edit|MultiEdit)
        cat <<EOF >&2
========================================================================
BLOCKED: agent-bus append-only invariant violated.

  Tool: $TOOL_NAME
  File: $FILE_PATH

dialog/*.jsonl files are append-only per docs/tooling/agent-bus-spec.md §6.
The only valid mutation is appending NEW lines at end-of-file via:

    scripts/agent-bus post --topic <topic> --type <type> ...

If you genuinely need to amend an event (you almost certainly don't):
    1. The append-only invariant is structural; never amend in place.
    2. To supersede a prior event, append a new event citing it via
       --in-reply-to <sha256>.
    3. To overturn a closed topic decision, open a new topic that cites
       the closed one in --links.

If this is a false positive (e.g., editing a non-event field at file
header), the hook itself is too coarse and the unblock procedure is
to add an explicit allow-marker in the file or amend this hook.
========================================================================
EOF
        exit 2
        ;;
    Write)
        # Write is "create or fully replace" — replacing an existing topic
        # file is structurally a rewrite of all prior lines, hence blocked.
        # Creating a new topic file (Write to a path that does not yet
        # exist) is fine; let it through.
        if [ -e "$FILE_PATH" ]; then
            cat <<EOF >&2
========================================================================
BLOCKED: agent-bus append-only invariant violated.

  Tool: Write (would replace existing file)
  File: $FILE_PATH

Writing over an existing dialog/*.jsonl file rewrites all prior lines,
breaking the append-only invariant. Use scripts/agent-bus post to add a
new event, or delete the file via Bash if you intend to start over (and
expect the topic-creation history to be lost).
========================================================================
EOF
            exit 2
        fi
        # Creating a new topic file is allowed.
        exit 0
        ;;
    *)
        # Any other tool (Read, Bash, etc.) — allow.
        exit 0
        ;;
esac

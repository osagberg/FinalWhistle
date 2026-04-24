#!/bin/sh
# dump-and-read.sh — one-shot: trigger Unity state dump, read + pretty-print JSON.
#
# Usage:
#   ./dump-and-read.sh                # uses $PROJECT_ROOT/unity-project
#   PROJECT_ROOT=/path/to/proj ./dump-and-read.sh
#   UNITY_PATH=/path/to/Unity ./dump-and-read.sh
#
# Exit codes:
#   0 — dump present and parsed
#   1 — dump file missing (Unity not running, or dump path wrong)
#   2 — jq not installed
#   3 — Unity invocation failed
#
# POSIX-safe. No bashisms. Works on macOS and Linux.

set -eu

PROJECT_ROOT="${PROJECT_ROOT:-$(pwd)}"
UNITY_PROJECT="${UNITY_PROJECT:-$PROJECT_ROOT/unity-project}"
DUMP_PATH="$UNITY_PROJECT/Library/StateDump.json"
METHOD="${DUMP_METHOD:-FinalWhistle.Debug.McpRemoteControl.DumpState}"

if ! command -v jq >/dev/null 2>&1; then
    echo "error: jq not found. Install with 'brew install jq' (macOS) or 'apt install jq' (linux)." >&2
    exit 2
fi

# Prefer the MCP path — if Unity Editor is already running with the project
# open, execute_menu_item is instant. We can't call MCP from shell though, so
# this script falls back to batchmode. If you have an Editor open, use the
# menu entry "FinalWhistle/Debug/Dump State" directly.
if [ -n "${USE_BATCHMODE:-}" ]; then
    UNITY_PATH="${UNITY_PATH:-}"
    if [ -z "$UNITY_PATH" ]; then
        # Autodetect on macOS
        if [ -d "/Applications/Unity/Hub/Editor" ]; then
            UNITY_PATH="$(ls -1 /Applications/Unity/Hub/Editor 2>/dev/null | tail -1)"
            if [ -n "$UNITY_PATH" ]; then
                UNITY_PATH="/Applications/Unity/Hub/Editor/$UNITY_PATH/Unity.app/Contents/MacOS/Unity"
            fi
        fi
    fi

    if [ -z "$UNITY_PATH" ] || [ ! -x "$UNITY_PATH" ]; then
        echo "error: UNITY_PATH not set and autodetect failed." >&2
        exit 3
    fi

    echo "Running Unity batchmode dump via -executeMethod $METHOD ..." >&2
    "$UNITY_PATH" \
        -batchmode -quit -nographics \
        -projectPath "$UNITY_PROJECT" \
        -executeMethod "$METHOD" \
        -logFile /tmp/state-dump.log || {
            echo "error: Unity batchmode failed. See /tmp/state-dump.log" >&2
            exit 3
        }
fi

if [ ! -f "$DUMP_PATH" ]; then
    echo "error: dump file not found at $DUMP_PATH" >&2
    echo "hint:  ensure Unity Editor is open with the project, then invoke" >&2
    echo "       the menu entry 'FinalWhistle/Debug/Dump State', or re-run" >&2
    echo "       this script with USE_BATCHMODE=1" >&2
    exit 1
fi

jq '.' "$DUMP_PATH"

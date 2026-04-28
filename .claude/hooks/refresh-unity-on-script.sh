#!/usr/bin/env bash
# PostToolUse hook: refresh-unity-on-script.sh
#
# Event:     PostToolUse (matcher: mcp__UnityMCP__manage_script)
# Purpose:   After Claude modifies a Unity C# script via the UnityMCP bridge,
#            trigger an AssetDatabase refresh so the Editor recompiles without
#            needing a manual window-focus.
# Exit:      0 always. Fails silently if the MCP server is not running.
# Deps:      curl (best-effort). No strict requirement.
#
# stdin JSON: { "tool_name":"mcp__UnityMCP__manage_script",
#               "tool_input":{...}, "tool_response":{...} }

set +e

INPUT=$(cat 2>/dev/null)

# Only proceed if the tool was a unity-mcp script operation.
# The matcher in settings.json should already filter this, but double-check.
if [ -n "$INPUT" ]; then
  case "$INPUT" in
    *manage_script*) ;;
    *) exit 0 ;;
  esac
fi

# UnityMCP bridge endpoint (CoplayDev unity-mcp HTTP server).
# Single source of truth: matches .mcp.json :: mcpServers.UnityMCP.url
# (http://localhost:8080/mcp). Override via UNITY_MCP_URL env var if you
# relocate the server. If the server isn't running, curl fails silently.
ENDPOINT="${UNITY_MCP_URL:-http://localhost:8080/mcp}"

# Best-effort: use the MCP manage_editor endpoint with a refresh action.
# Different MCP server versions accept different payload shapes — try a
# simple refresh POST and swallow any error.
if command -v curl >/dev/null 2>&1; then
  curl -sS -m 3 \
    -X POST \
    -H "Content-Type: application/json" \
    -d '{"tool":"manage_editor","action":"refresh"}' \
    "$ENDPOINT" \
    >/dev/null 2>&1 || true
fi

exit 0

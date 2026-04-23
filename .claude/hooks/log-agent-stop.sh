#!/usr/bin/env bash
# SubagentStop hook: log-agent-stop.sh
#
# Event:     SubagentStop
# Purpose:   Audit trail — append JSON line to .claude/logs/agent-invocations.jsonl
#            when a subagent completes. Records token usage + duration if
#            available in the event payload.
# Exit:      0 always. Fails open.
# Deps:      python3 preferred; falls back to printf.
#
# IMPORTANT: Same `agent_type` vs `agent_name` pitfall as log-agent.sh.
#
# stdin JSON: { "session_id":"...", "agent_id":"...", "agent_type":"Explore",
#               "last_assistant_message":"...", "usage":{...}, ... }

set +e

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
LOG_DIR="$ROOT/.claude/logs"
LOG_FILE="$LOG_DIR/agent-invocations.jsonl"

mkdir -p "$LOG_DIR" 2>/dev/null

INPUT=$(cat)
TS=$(date -u +%Y-%m-%dT%H:%M:%SZ)

if command -v python3 >/dev/null 2>&1; then
  AGENT_TS="$TS" HOOK_INPUT="$INPUT" python3 <<'PYEOF' >> "$LOG_FILE" 2>/dev/null
import sys, json, os
raw = os.environ.get("HOOK_INPUT", "")
try:
    d = json.loads(raw) if raw else {}
except Exception:
    d = {}
usage = d.get("usage") or d.get("token_usage") or {}
out = {
    "event": "SubagentStop",
    "timestamp": os.environ.get("AGENT_TS", ""),
    "agent_type": d.get("agent_type") or "unknown",
    "agent_id": d.get("agent_id") or "",
    "session_id": d.get("session_id") or "",
    "duration_ms": d.get("duration_ms") or d.get("duration") or None,
    "input_tokens": usage.get("input_tokens") if isinstance(usage, dict) else None,
    "output_tokens": usage.get("output_tokens") if isinstance(usage, dict) else None,
    "cache_read_tokens": usage.get("cache_read_input_tokens") if isinstance(usage, dict) else None,
}
sys.stdout.write(json.dumps(out) + "\n")
PYEOF
else
  AGENT_TYPE=$(printf '%s' "$INPUT" | grep -oE '"agent_type"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*:"//;s/"$//')
  [ -z "$AGENT_TYPE" ] && AGENT_TYPE="unknown"
  printf '{"event":"SubagentStop","timestamp":"%s","agent_type":"%s"}\n' "$TS" "$AGENT_TYPE" >> "$LOG_FILE" 2>/dev/null
fi

exit 0

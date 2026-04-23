#!/usr/bin/env bash
# SubagentStart hook: log-agent.sh
#
# Event:     SubagentStart
# Purpose:   Audit trail — append JSON line to .claude/logs/agent-invocations.jsonl
#            when a subagent starts.
# Exit:      0 always. Fails open.
# Deps:      python3 preferred for JSON encoding; falls back to printf.
#
# IMPORTANT: Read `agent_type`, NOT `agent_name` (the latter is always null,
#            per donchitos' own bug comment). See their log-agent.sh header.
#
# stdin JSON: { "session_id":"...", "agent_id":"...", "agent_type":"Explore", ... }

set +e

ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"
LOG_DIR="$ROOT/.claude/logs"
LOG_FILE="$LOG_DIR/agent-invocations.jsonl"

mkdir -p "$LOG_DIR" 2>/dev/null

# Ensure logs/ is gitignored
GITIGNORE="$ROOT/.claude/.gitignore"
if [ ! -f "$GITIGNORE" ] || ! grep -qxF "logs/" "$GITIGNORE" 2>/dev/null; then
  echo "logs/" >> "$GITIGNORE" 2>/dev/null
fi

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
out = {
    "event": "SubagentStart",
    "timestamp": os.environ.get("AGENT_TS", ""),
    "agent_type": d.get("agent_type") or "unknown",
    "agent_id": d.get("agent_id") or "",
    "session_id": d.get("session_id") or "",
    "description": (d.get("tool_input") or {}).get("description") or d.get("description") or "",
}
sys.stdout.write(json.dumps(out) + "\n")
PYEOF
else
  AGENT_TYPE=$(printf '%s' "$INPUT" | grep -oE '"agent_type"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/.*:"//;s/"$//')
  [ -z "$AGENT_TYPE" ] && AGENT_TYPE="unknown"
  printf '{"event":"SubagentStart","timestamp":"%s","agent_type":"%s"}\n' "$TS" "$AGENT_TYPE" >> "$LOG_FILE" 2>/dev/null
fi

exit 0

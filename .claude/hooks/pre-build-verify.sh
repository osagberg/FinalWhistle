#!/usr/bin/env bash
# PreToolUse hook: pre-build-verify.sh
#
# Event:     PreToolUse (matcher: Bash)
# Purpose:   Before a Unity batchmode build (`-buildTarget ...`), confirm
#            verification was run recently (VerificationReport.json modified
#            within 30 min) and that it reports zero CS errors.
# Exit:      0 — allow, possibly with stale-verify warning.
#            2 — block (only when report exists and shows CS compile errors).
# Deps:      POSIX date arithmetic, python3 for JSON.
#
# stdin JSON: { "tool_name":"Bash", "tool_input":{"command":"... -buildTarget ..."} }

INPUT=$(cat)
ROOT="${CLAUDE_PROJECT_DIR:-$(pwd)}"

if command -v python3 >/dev/null 2>&1; then
  COMMAND=$(printf '%s' "$INPUT" | python3 -c 'import sys,json
try: d=json.load(sys.stdin); print((d.get("tool_input") or {}).get("command",""))
except: pass' 2>/dev/null)
else
  COMMAND=$(printf '%s' "$INPUT" | grep -oE '"command"[[:space:]]*:[[:space:]]*"[^"]*"' | sed 's/"command"[[:space:]]*:[[:space:]]*"//;s/"$//')
fi

# Only intervene on Unity batchmode builds.
if ! echo "$COMMAND" | grep -qE -- '-buildTarget[[:space:]]|-batchmode'; then
  exit 0
fi
if ! echo "$COMMAND" | grep -qE -- '-buildTarget[[:space:]]'; then
  exit 0
fi

REPORT="$ROOT/unity-project/Library/VerificationReport.json"
if [ ! -f "$REPORT" ]; then
  REPORT_ALT="$ROOT/Library/VerificationReport.json"
  if [ -f "$REPORT_ALT" ]; then
    REPORT="$REPORT_ALT"
  fi
fi

if [ ! -f "$REPORT" ]; then
  {
    echo "=== pre-build-verify WARNING ==="
    echo "VerificationReport.json not found. Run /unity-check (or the Unity"
    echo "harness verification pass) before building to catch CS compile errors."
    echo "================================"
  } >&2
  exit 0
fi

# mtime age in seconds (cross-platform)
if [[ "$OSTYPE" == "darwin"* ]]; then
  MTIME=$(stat -f %m "$REPORT" 2>/dev/null)
else
  MTIME=$(stat -c %Y "$REPORT" 2>/dev/null)
fi
NOW=$(date +%s)
AGE=$(( NOW - ${MTIME:-0} ))

if [ "$AGE" -gt 1800 ]; then
  {
    echo "=== pre-build-verify WARNING ==="
    echo "VerificationReport.json is stale (> 30 min old)."
    echo "Run /unity-check first to confirm no compile errors before building."
    echo "================================"
  } >&2
fi

# Parse for CS errors — block only on confirmed compile errors.
if command -v python3 >/dev/null 2>&1; then
  ERROR_COUNT=$(python3 - <<PYEOF
import json
try:
    d=json.load(open("$REPORT"))
except Exception:
    print(0); raise SystemExit
# Accept a few common shapes: {"errors": [...]}, {"cs_errors": N}, {"compile":{"errors":[...]}}
n = 0
if isinstance(d, dict):
    for k in ("errors","cs_errors","compileErrors","compile_errors"):
        v = d.get(k)
        if isinstance(v, list): n += len(v)
        elif isinstance(v, int): n += v
    c = d.get("compile") if isinstance(d.get("compile"), dict) else {}
    if isinstance(c.get("errors"), list): n += len(c["errors"])
print(n)
PYEOF
)
  if [ "${ERROR_COUNT:-0}" -gt 0 ]; then
    {
      echo "=== BLOCKED: pre-build-verify ==="
      echo "VerificationReport.json reports $ERROR_COUNT CS compile error(s)."
      echo "Fix compile errors before running a Unity build."
      echo "Report: $REPORT"
      echo "================================="
    } >&2
    exit 2
  fi
fi

exit 0

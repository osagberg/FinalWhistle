#!/bin/bash
# PreToolUse hook: protect SPEC.md decisions log as append-only.
#
# Rejects Edit/Write operations on SPEC.md that would mutate or delete existing
# decisions-log entries. Enforces the pattern described in /log-decision and
# SETUP.md: the log is append-only; revisions go via "Supersedes" entries.
#
# The invariant: for any Edit that touches a line matching `^- \*\*\d{4}-`
# (a decision-log bullet), new_string must contain old_string as a literal
# substring. That rules out any mutation while still permitting pure appends
# of the form: old=last-existing-line, new=last-existing-line + new-bullet.

# Read JSON from stdin into an env var, then let the heredoc provide the
# Python script. Avoids the stdin double-use bug.
HOOK_INPUT="$(cat)"
export HOOK_INPUT

python3 <<'PYEOF'
import json, sys, os, re

raw = os.environ.get("HOOK_INPUT", "")
try:
    data = json.loads(raw) if raw else {}
except Exception:
    sys.exit(0)

tool = data.get("tool_name", "")
if tool not in ("Edit", "Write", "NotebookEdit"):
    sys.exit(0)

ti = data.get("tool_input") or {}
fp = ti.get("file_path") or ti.get("path") or ""
if os.path.basename(fp) != "SPEC.md":
    sys.exit(0)

old = ti.get("old_string") or ""
new = ti.get("new_string") or ""
content = ti.get("content", "")  # Write uses content, not old/new

# Is the decisions log section being touched? Coarse probe — anything
# containing a bullet matching a decision-log entry pattern.
decision_line = re.compile(r'^- \*\*\d{4}-', re.MULTILINE)
if not decision_line.search(old + new + content):
    sys.exit(0)

def block(msg):
    sys.stderr.write(msg.rstrip() + "\n")
    sys.exit(2)

# Full-file rewrites on SPEC.md when they touch the log: unsafe, block.
if tool == "Write":
    block(
        "BLOCKED: Write on SPEC.md is disallowed when the content touches the\n"
        "decisions log. The log is append-only. Use Edit with:\n"
        "  old_string = last existing decision line\n"
        "  new_string = last existing decision line + new bullet\n"
        "See SETUP.md and the /log-decision skill."
    )

if ti.get("replace_all"):
    block(
        "BLOCKED: Edit with replace_all=true on SPEC.md may rewrite decision-log\n"
        "entries. Apply the edit with a targeted, append-only pattern."
    )

# The append-only rule.
if old and old not in new:
    block(
        "BLOCKED: edit to SPEC.md would mutate or delete an existing decisions-log\n"
        "entry. The log is append-only.\n\n"
        "To revise a prior entry:\n"
        "  1. Leave the original line intact.\n"
        "  2. Append a NEW entry at the end with 'Supersedes: <prior date>'.\n"
        "  3. Optionally pure-append '(Superseded <date>)' to the prior entry.\n\n"
        "See SETUP.md and /log-decision skill step 5."
    )

sys.exit(0)
PYEOF

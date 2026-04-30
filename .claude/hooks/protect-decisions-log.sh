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

decision_line = re.compile(r'^- \*\*\d{4}-', re.MULTILINE)

def block(msg):
    sys.stderr.write(msg.rstrip() + "\n")
    sys.exit(2)

# Read the on-disk pre-image so we can detect REMOVALS — i.e., a full
# rewrite Write whose proposed content drops decision bullets entirely.
# (Codex P1 round 4, 2026-04-30 — the prior probe-only-the-proposed-content
# version exited early when `new`/`content` happened to have no decision
# lines, letting wholesale erasures pass.)
#
# Fail-closed posture per pr-review-toolkit:silent-failure-hunter 2026-04-30
# P1 #3: only FileNotFoundError is benign (fresh repo, SPEC.md not yet
# created). Any OTHER exception when reading the very file under
# decisions-log protection means the integrity check cannot be performed,
# which is precisely the moment a fail-open default could miss a wholesale
# rewrite. Block with a descriptive error so the operator can either
# diagnose the read failure or commit through the terminal (where this
# hook does not run).
disk_decisions = set()
try:
    with open(fp, "r", encoding="utf-8") as f:
        for line in f.readlines():
            if decision_line.match(line):
                disk_decisions.add(line.rstrip("\n"))
except FileNotFoundError:
    # Genuinely benign: SPEC.md is being created for the first time.
    # The Write/Edit operation itself defines the initial decisions log.
    pass
except Exception as exc:
    block(
        f"BLOCKED: protect-decisions-log.sh could not read {fp} for the "
        f"append-only integrity check ({type(exc).__name__}: {exc}). The "
        f"hook is fail-closed by design — if the SPEC.md pre-image cannot "
        f"be read, removals cannot be detected, and a wholesale-rewrite "
        f"Write could pass undetected.\n\n"
        f"Diagnose the read failure (encoding, permissions, file lock, "
        f"transient I/O), or — if this is a deliberate audited rewrite — "
        f"commit through the terminal (this hook only runs in Claude "
        f"Code sessions)."
    )

# --- Removal-detection: any disk decision missing from the proposed image? --
# For Write: post-image is `content` directly. Each disk decision must appear
# in `content` verbatim, otherwise the rewrite drops it.
# For Edit: post-image construction is harder (we don't have the full file),
# but the append-only invariant `old in new` already covers targeted edits
# that mutate; the missing-bullet probe still catches the case where a
# decision line appears in `old` but not `new` (i.e., the edit deletes it).
if tool == "Write":
    # Always inspect Writes — even if proposed content has zero decision
    # lines (precisely the bypass: drop the log entirely).
    missing = sorted(d for d in disk_decisions if d not in content)
    if missing:
        sample = "\n  ".join(missing[:3])
        block(
            "BLOCKED: Write on SPEC.md drops "
            f"{len(missing)} decisions-log entry/entries from disk.\n"
            "The log is append-only.\n\n"
            f"First missing entries:\n  {sample}\n\n"
            "Use Edit with:\n"
            "  old_string = last existing decision line\n"
            "  new_string = last existing decision line + new bullet\n"
            "See SETUP.md and the /log-decision skill."
        )

# Beyond this point, behaviour matches the original log-touching pre-flight:
# require the change to involve decision-line context before applying the
# append-only literal-substring rule.
if not decision_line.search(old + new + content):
    sys.exit(0)

if tool == "Write":
    # Defensive: a Write that DOES touch decision lines but didn't drop any
    # disk entries is still risky — operator should use Edit for log work.
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

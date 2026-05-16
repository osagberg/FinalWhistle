#!/bin/bash
# PreToolUse hook: protect docs/DECISIONS.md as append-only.
#
# Ported from FW v1's .claude/hooks/protect-decisions-log.sh, retargeted
# from SPEC.md to docs/DECISIONS.md per the CLAUDE.md §4.3 contract: the
# decisions log is append-only; revisions go via "Supersedes" entries
# that cite the prior bullet verbatim.
#
# Invariant: for any Edit that touches a line matching `^- \*\*\d{4}-`
# (a dated decision bullet), new_string must contain old_string as a
# literal substring. That permits pure appends (old=last-existing-line,
# new=last-existing-line + new-bullet) and rejects mutations / deletions.
#
# Wired by .claude/settings.json (PreToolUse, matcher
# 'Edit|Write|MultiEdit', file_path filter on docs/DECISIONS.md). The
# matcher passes the tool input JSON on stdin.
#
# **Codex 2026-05-16 whole-codebase audit (backlog): MultiEdit coverage gap.**
# Prior implementation whitelisted ("Edit", "Write", "NotebookEdit") — the
# settings.json matcher routes MultiEdit to this hook, but the hook
# silently exited with sys.exit(0) when invoked for MultiEdit because the
# tool_name wasn't in the whitelist. A MultiEdit batch on DECISIONS.md
# could therefore mutate dated bullets undetected. Fix: include
# "MultiEdit" in the whitelist + iterate its `edits: [{old_string,
# new_string}]` array, applying the same append-only literal-substring
# rule to each pair.

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
if tool not in ("Edit", "Write", "MultiEdit", "NotebookEdit"):
    sys.exit(0)

ti = data.get("tool_input") or {}
fp = ti.get("file_path") or ti.get("path") or ""

# Match docs/DECISIONS.md anywhere in the path so the hook works for
# both absolute and relative invocations.
if not fp.endswith("docs/DECISIONS.md") and os.path.basename(fp) != "DECISIONS.md":
    sys.exit(0)

# Only fire on the canonical decisions log — not on archived copies.
# (FW v1 hit a false-positive on docs/archive/DECISIONS.md once.)
if "/archive/" in fp:
    sys.exit(0)

old = ti.get("old_string") or ""
new = ti.get("new_string") or ""
content = ti.get("content", "")  # Write uses content, not old/new

# MultiEdit fans out to a list of {old_string, new_string} edits applied
# sequentially. Treat as N back-to-back Edits: any single pair that would
# mutate a dated bullet trips the same block.
multi_edits = ti.get("edits") or []
if tool == "MultiEdit" and not isinstance(multi_edits, list):
    multi_edits = []

decision_line = re.compile(r'^- \*\*\d{4}-', re.MULTILINE)

def block(msg):
    sys.stderr.write(msg.rstrip() + "\n")
    sys.exit(2)

# Read the on-disk pre-image so we can detect REMOVALS — i.e., a full
# rewrite Write whose proposed content drops decision bullets entirely.
#
# Fail-closed posture per FW v1 pr-review-toolkit:silent-failure-hunter
# 2026-04-30 finding: only FileNotFoundError is benign (fresh repo,
# DECISIONS.md not yet created). Any OTHER exception when reading the
# file under append-only protection means the integrity check cannot be
# performed — block with a descriptive error.
disk_decisions = set()
try:
    with open(fp, "r", encoding="utf-8") as f:
        for line in f.readlines():
            if decision_line.match(line):
                disk_decisions.add(line.rstrip("\n"))
except FileNotFoundError:
    # Genuinely benign: DECISIONS.md is being created for the first time.
    pass
except Exception as exc:
    block(
        f"BLOCKED: protect-decisions.sh could not read {fp} for the "
        f"append-only integrity check ({type(exc).__name__}: {exc}). The "
        f"hook is fail-closed by design — if the pre-image cannot be "
        f"read, removals cannot be detected.\n\n"
        f"Diagnose the read failure (encoding, permissions, file lock, "
        f"transient I/O), or commit through the terminal (this hook "
        f"only runs in Claude Code sessions)."
    )

# --- Removal-detection: any disk decision missing from the proposed image? --
if tool == "Write":
    missing = sorted(d for d in disk_decisions if d not in content)
    if missing:
        sample = "\n  ".join(missing[:3])
        block(
            f"BLOCKED: Write on {os.path.basename(fp)} drops "
            f"{len(missing)} decisions-log entry/entries from disk.\n"
            "The log is append-only.\n\n"
            f"First missing entries:\n  {sample}\n\n"
            "Use Edit with:\n"
            "  old_string = last existing decision line\n"
            "  new_string = last existing decision line + new bullet\n"
            "See CLAUDE.md §4.3 and the /log-decision skill."
        )

# Aggregate all decision-line-bearing strings the proposed change touches
# (single-Edit old+new, Write content, and every MultiEdit pair) for the
# "does this touch the decisions log at all?" gate.
multi_blob = "".join((e.get("old_string") or "") + (e.get("new_string") or "")
                     for e in multi_edits)
if not decision_line.search(old + new + content + multi_blob):
    sys.exit(0)

if tool == "Write":
    block(
        "BLOCKED: Write on DECISIONS.md is disallowed when the content touches\n"
        "the decisions log. The log is append-only. Use Edit with:\n"
        "  old_string = last existing decision line\n"
        "  new_string = last existing decision line + new bullet\n"
        "See CLAUDE.md §4.3 and the /log-decision skill."
    )

if ti.get("replace_all"):
    block(
        "BLOCKED: Edit with replace_all=true on DECISIONS.md may rewrite\n"
        "decision-log entries. Apply the edit with a targeted, append-only\n"
        "pattern."
    )

# The append-only rule for single Edit.
if old and old not in new:
    block(
        "BLOCKED: edit to DECISIONS.md would mutate or delete an existing\n"
        "decisions-log entry. The log is append-only.\n\n"
        "To revise a prior entry:\n"
        "  1. Leave the original line intact.\n"
        "  2. Append a NEW entry at the end with 'Supersedes: <prior date>'.\n"
        "  3. Optionally pure-append '(Superseded <date>)' to the prior entry.\n\n"
        "See CLAUDE.md §4.3 and the /log-decision skill."
    )

# The append-only rule for MultiEdit: any pair touching a dated bullet
# whose new_string drops the old_string trips. Per-pair replace_all also
# blocks (same risk as single-Edit replace_all on this file).
for idx, e in enumerate(multi_edits):
    e_old = e.get("old_string") or ""
    e_new = e.get("new_string") or ""
    if e.get("replace_all"):
        block(
            f"BLOCKED: MultiEdit edit #{idx + 1} on DECISIONS.md uses "
            "replace_all=true. Apply each edit with a targeted, "
            "append-only pattern."
        )
    if e_old and e_old not in e_new and decision_line.search(e_old + e_new):
        block(
            f"BLOCKED: MultiEdit edit #{idx + 1} on DECISIONS.md would "
            "mutate or delete an existing decisions-log entry. The log "
            "is append-only.\n\n"
            "To revise a prior entry:\n"
            "  1. Leave the original line intact.\n"
            "  2. Append a NEW entry at the end with 'Supersedes: <prior date>'.\n"
            "  3. Optionally pure-append '(Superseded <date>)' to the prior entry.\n\n"
            "See CLAUDE.md §4.3 and the /log-decision skill."
        )

sys.exit(0)
PYEOF

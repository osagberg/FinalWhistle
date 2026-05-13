---
description: Append a new entry to docs/DECISIONS.md (append-only, hook-enforced).
argument-hint: <topic> — <decision>
---

# /log-decision — append-only decisions log

`docs/DECISIONS.md` is append-only. The PreToolUse hook at `.claude/hooks/protect-decisions.sh` rejects any edit that mutates a line matching `^- \*\*\d{4}-`. To change a prior decision, append a new entry that supersedes it by citing the prior bullet verbatim.

## Procedure

### 1. Gather the decision

If the user didn't supply `<topic>` and `<decision>` in the slash-command arguments, prompt:

```
Logging a decision. Provide:
  - Topic (3-6 words, e.g. "save schema bump cadence")
  - Decision (one sentence, declarative, the call)
  - Context (one to two sentences — why now, what alternatives considered)
  - Supersedes (optional — prior DECISIONS.md bullet date + topic, quoted verbatim)
```

Wait. Do not invent content.

### 2. Format the entry

Exactly this shape:

```
- **YYYY-MM-DD** — <topic>: <decision>. Context: <why + alternatives>. Supersedes: <prior bullet date + topic verbatim, or "none">.
```

Date = today (see `currentDate` in session context). One line per entry; wrap fine but no internal bullets.

### 3. Append to docs/DECISIONS.md

Read the current file. Locate the end of the entries section. Insert the new bullet at the bottom.

Do NOT modify any existing line. The hook will reject the write if you do — even a whitespace touch on a dated bullet is enough.

### 4. Confirm append succeeded

```bash
git diff docs/DECISIONS.md
```

Show only the additions. Confirm zero `-` lines outside trailing whitespace.

### 5. Stage and offer commit

```bash
git add docs/DECISIONS.md
```

Suggest (do not auto-commit) the commit message:

```
docs(decisions): <topic>

<one-line restatement of decision>
```

The user runs `/commit` or includes it in the next `/next` cycle's commit.

## Hard rules

- Never edit a prior dated bullet. Append + supersede only.
- Never delete. Even mistaken entries are superseded, not removed.
- Never date-shift. Bullet date = day decision was made.
- Never invent context. If WHY is missing, ask. Decisions without rationale rot.

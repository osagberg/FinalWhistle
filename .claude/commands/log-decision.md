---
description: Record a project decision in SPEC.md decisions log (with reasoning)
---

# /log-decision — record a decision

Append a decision to the immutable decisions log in `SPEC.md`.

## Procedure

1. Parse the decision from the command arguments (everything after `/log-decision`)
2. If no args, ask user to state the decision
3. Ask user for the reasoning if it's not self-evident from the decision statement
4. Append to `SPEC.md` → "Decisions log" section:
   ```
   - **YYYY-MM-DD** — **<Decision headline>**. Reasoning: <short why>.
   ```
5. Check if the decision contradicts or supersedes a prior decision:
   - If yes, add `Supersedes: <prior decision date>` to the new entry
   - Append a note on the prior entry: `(Superseded <date>)`
6. Check if the decision requires changes to other docs:
   - `CLAUDE.md` (contract-level changes)
   - `PROJECT_CONTEXT.md` (project-level changes)
   - `TECH_APPROACH.md` (engineering changes)
   - `design/*.md` (content changes)
   - `SPEC.md` phase tasks (plan changes)
   - `TOOLING.md` (tool adopt/drop)
   - `SETUP.md` (install-procedure changes)
7. If doc changes are needed, propose them inline and execute after user confirmation
8. Brief confirm message to user: `Decision logged. Docs updated: [list]` or `Decision logged. No doc changes needed.`

## Hook behavior

The `protect-decisions-log.sh` PreToolUse hook enforces append-only. If your Edit fails with a BLOCKED message, you've tried to mutate a prior entry. Reformulate as pure append — your `old_string` should be the LAST existing decision line, and your `new_string` should be that same line + your new bullet appended.

## Examples

- `/log-decision change skeleton standard from VRoid to Mixamo`
- `/log-decision drop ElevenLabs; use Kokoro TTS instead`
- `/log-decision bump Unity version from 6.0 to 6.1 LTS`
- `/log-decision commit to Steam Early Access release track`

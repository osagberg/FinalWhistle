---
name: check-reviews
description: Claude-side review-inbox skill. Polls the agent-bus for Codex's review-findings (counter / evidence events) on topics where Claude is the implementing agent, then applies fixes — but only when the fix is cascade-safe per ADR-0012 §Component 6. If applying a fix would touch files that are ALSO in scope of an in-flight downstream task, the fix is DEFERRED to user triage instead of auto-applied (prevents cascading bugs when Claude is already working on a follow-up task that depends on the same files). Used while Claude is doing other work — periodically picks up Codex's findings without breaking flow. Triggers on "check reviews", "codex findings", "review inbox", "pick up reviews", "apply codex feedback".
triggers:
  - check reviews
  - check-reviews
  - codex findings
  - review inbox
  - pick up reviews
  - apply codex feedback
  - codex review pickup
---

# Check-reviews — Claude-side review-inbox with cascade-prevention

Claude polls the agent-bus for Codex's review-findings (`counter` / `evidence` / `question` events posted in response to Claude's `commit-proposal` claims) and applies fixes for cascade-safe findings. Cascade-risk findings are deferred to user triage.

## The problem this solves

Per the user's mandate: "you can pick up the info from the bus messages while you are working on next tasks, and u can go back and fix the previous ones in accordance to the reviews (only if the next task doesn't depend on the previous one, so we don't get cascading bugs)."

Without cascade-prevention, a naive review-pickup loop produces this failure:

1. Claude implements Task A; Codex reviews → finds bug → Claude moves on.
2. Claude implements Task B; Task B depends on Task A's files.
3. Codex's Task-A review-finding lands; Claude auto-applies fix.
4. Task A's files now differ from what Task B was built against.
5. Task B is broken; verification fails; cascade.

Cascade-prevention guards against this by checking `files_in_scope` overlap between the candidate fix and any in-flight task-spec before applying.

## When to invoke

| Situation | Reason |
|---|---|
| Between Tier-2 tasks, when Claude has spare cycles | Pick up review findings without disrupting current work |
| At session start | Catch any Codex findings from the previous session |
| Explicitly when the user says "check the bus" | Manual sync |
| Periodically during long-running `/duo-implement` runs (every 10 wake-ups) | Ambient catch-up |

## When NOT to invoke

- Mid-chunk during an active task implementation (interrupts flow; do at chunk boundaries)
- When `/duo-implement` is escalated (waiting for user decision; don't add noise)
- When no commit-proposal events are pending (cheap check; just `scripts/agent-bus pending-reviews` exits 0)

## Workflow

### Step 1 — Inventory pending reviews

```sh
scripts/agent-bus pending-reviews
```

Exit 0 = nothing pending → done. Exit 5 = topics listed → continue.

### Step 2 — For each pending review topic, read the latest counter / evidence / question

```sh
scripts/agent-bus read --topic <topic-name> --from codex --type counter
scripts/agent-bus read --topic <topic-name> --from codex --type evidence
scripts/agent-bus read --topic <topic-name> --from codex --type question
```

Identify the most-recent finding(s) that respond to Claude's commit-proposal. The `in_reply_to` field on Codex's events should reference Claude's commit-proposal sha256.

### Step 3 — Classify each finding

For each finding, determine:

- **Severity** — from the event's `severity` field (p0 / p1 / p2 / p3)
- **Type of fix** — what files would the fix touch? Extract from Codex's body + `links` field.
- **Scope** — is the fix on the original topic, or does it require a new task-spec?

### Step 4 — Cascade-check before applying

For each finding that would modify a file:

```sh
scripts/agent-bus cascade-check --files <comma-separated-file-paths> --target-topic <original-topic>
```

Exit codes:
- **0** — cascade-safe; no in-flight task-spec has overlapping `files_in_scope`. Safe to apply.
- **4** — overlap detected; in-flight task(s) depend on the same files. Cascade risk.
- **2** — argument error; fix the invocation.

### Step 5a — Cascade-safe: apply the fix

Open the original topic, post Claude's acknowledgment of the finding:

```sh
scripts/agent-bus post --topic <topic-name> --type ack --from claude --to codex \
  --in-reply-to <codex-finding-sha> --body "ack — applying fix"
```

Apply the fix using Edit / Write / the appropriate `Unity_*` MCP tool. Run `scripts/fw verify`. Post a new `commit-proposal`:

```sh
scripts/agent-bus claim --topic <topic-name> --severity p1 --from claude --to codex \
  --body "commit-proposal: applied Codex review-finding <finding-sha>. Files touched: <list>. Verification: <output>. Awaiting reviewer ack."
```

Wait for new ack, then commit. (This is essentially a mini `/duo-implement` cycle on the same topic.)

### Step 5b — Cascade-risk: DEFER to user triage

Do NOT apply the fix. Post a `note` event on the original topic AND an `escalation` event for the user:

```sh
scripts/agent-bus post --topic <topic-name> --type note --from claude --to user \
  --body "Codex finding deferred: applying would touch <files>, which are in scope of in-flight topic <other-topic>. Cascade risk — user triage needed. Finding sha: <codex-finding-sha>."

scripts/agent-bus post --topic <topic-name> --type escalation --severity p2 \
  --from claude --to user \
  --body "cascade-risk fix deferred. proposed action: (a) abandon in-flight topic <other-topic> and apply this fix first, OR (b) keep <other-topic> moving and apply this fix after it lands, OR (c) merge the fixes into a combined task-spec. decision needed."
```

The escalation pauses Claude's review-pickup for that finding until the user posts a `type=decision`. The original `/duo-implement` for the in-flight topic continues uninterrupted.

### Step 6 — Continue to next pending review

Loop back to Step 2 for the next topic. After all topics processed:

```sh
scripts/agent-bus post --topic <main-task-topic-or-suitable-meta-topic> --type note \
  --from claude --to user \
  --body "review-pickup pass complete: <N> findings applied, <M> deferred to triage. continuing with primary work."
```

## Cascade-check internals (for the curious)

`scripts/agent-bus cascade-check --files X,Y,Z --target-topic T` does:

1. Iterate `dialog/*.jsonl` for in-flight topics (task-spec landed, no task-complete, no user-decision closure)
2. Skip the target topic itself
3. For each in-flight topic, read its `task-spec` body
4. Lowercase-substring-match each candidate file against the body
5. If any match → exit 4 (with topic listed)
6. If no matches → exit 0

The check is **coarse and defensive** by design: false-positive (over-counting overlap) is safe (just triggers a user check-in); false-negative (missing real overlap) is dangerous (cascade bug ships). The match is whole-string-in-body, not glob-aware — a path like `unity-project/Assets/Viewer/Adapters/Dots/Foo.cs` in the candidate matches a glob like `unity-project/Assets/Viewer/Adapters/Dots/**` in the body because the substring `unity-project/Assets/Viewer/Adapters/Dots/` is shared. Good enough for tonight; future precision improvements are tracked as P3 follow-up.

## Cross-references

- [ADR-0012](../../../design/adr/adr-0012-autonomous-implementation-protocol.md) §Component 6 — cascade-prevention design
- [agent-bus-spec.md](../../../docs/tooling/agent-bus-spec.md) §16 — cascade-prevention semantics + dependency-tracking
- [/duo-implement](../duo-implement/SKILL.md) — the implementing-agent skill that PRODUCES the commit-proposals this skill reads
- [/codex-review-loop](../codex-review-loop/SKILL.md) — the Codex-side skill that PRODUCES the findings this skill reads

## Quick-check shortcut

Just want to see what's pending without applying?

```sh
scripts/agent-bus pending-reviews
```

That's the whole check. If nothing's listed → no action needed.

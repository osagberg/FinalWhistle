---
name: duo-implement
description: Execute an autonomous Tier-2 bounded coding task per ADR-0012. The user issues a `task-spec` event to a new agent-bus topic (acceptance criteria, files-in-scope, files-out-of-scope, cost/time caps); Claude (implementing agent) sets up the ScheduleWakeup polling loop, drafts the implementation, runs `scripts/fw verify`, posts a `commit-proposal` event, waits for Codex (reviewing agent) to ack via agent-bus, then commits. Canonical-hash regression auto-blocks via `.claude/hooks/canonical-hash-guard.sh`; user-escalation triggers (out-of-scope file / design-doc / SPEC mutation / asset-gen call / cascade risk) hard-stop polling. Use this for grunt-work tasks where the design is settled and the user has already decided scope. NOT for creative or money decisions — those escalate to the user. Triggers on "duo implement", "implement task spec", "autonomous task", "tier 2 task", "agents code together".
triggers:
  - duo implement
  - duo-implement
  - implement task spec
  - autonomous task
  - tier 2 task
  - tier-2 task
  - agents code together
  - autonomous implementation
---

# Duo-implement — autonomous Tier-2 bounded coding task

Execute one bounded coding task end-to-end with Claude as implementing agent + Codex as reviewing agent, coordinated via the agent-bus, with verification gates + escalation triggers per [ADR-0012](../../../design/adr/adr-0012-autonomous-implementation-protocol.md).

## When to invoke

| Situation | Reason |
|---|---|
| User issued a `task-spec` event with bounded scope | The protocol's primary use case |
| Implementing a feature where design is settled (ADR / SPEC entry already exists) | Grunt work — agents handle it |
| Slice work in the dots-adapter ladder (Slice 7+) | Bounded; well-defined acceptance |
| Bug fix on a previously-shipped slice (post-review) | Apply Codex's review-findings + re-verify |
| Test-coverage additions for an existing system | Bounded, no design risk |

## When NOT to invoke

- Anything creative: tone, aesthetics, narrative content — escalates anyway
- Anything that touches `design/**.md` or `SPEC.md` decisions log — escalates
- Anything that costs money: `Unity_AssetGeneration_*` invocations, paid-package installs — escalates
- Architectural decisions: use `/duo-debate` first to settle the design, THEN `/duo-implement` to execute
- Multi-task work where dependencies cross task boundaries — issue separate task-specs per ADR-0012 §Component 6 cascade-prevention

## Pre-flight (the user does this once)

The user issues a task-spec via:

```sh
AGENT_BUS_AUTHOR=user scripts/agent-bus task-spec --topic <topic-name> --body "$(cat <<'EOF'
acceptance:
  - <falsifiable criterion 1>
  - <falsifiable criterion 2>

files_in_scope:
  - <glob>
  - <glob>

files_out_of_scope:
  - design/**.md
  - SPEC.md
  - CLAUDE.md
  - <task-specific additions>

max_tokens: 400000
max_wall_clock_seconds: 7200
max_turns: 50

escalation_triggers:
  - <any custom triggers; default set in agent-bus-spec §14 always applies>

required_subagents:
  - <subagent ID per CLAUDE.md §6.3 mandatory rotation>

implementing_agent: claude
reviewing_agent: codex

depends_on:
  - <other-topic-id, if any>

notes:
  <free-form context, links to design docs, prior art>
EOF
)"
```

The user typically doesn't type this directly — they describe the task in natural language ("implement Slice 7 motion lines") and Claude scaffolds the task-spec from the conversation context, then posts it on the user's behalf using `AGENT_BUS_AUTHOR=user`.

## Workflow (Claude executes)

### Step 1 — Read the task-spec

```sh
scripts/agent-bus read --topic <topic-name> --type task-spec
```

Parse the structured fields. Acknowledge the spec by posting an `ack` event:

```sh
scripts/agent-bus post --topic <topic-name> --type ack --from claude --to user \
  --in-reply-to <task-spec-sha> --body "task-spec received; implementing"
```

### Step 2 — Plan + post the plan as a `claim`

Decompose the task into 3-7 chunks. Each chunk is one logical unit of work (one method, one component, one test fixture). Post the plan:

```sh
scripts/agent-bus claim --topic <topic-name> --severity p1 --from claude --to codex \
  --body "implementation plan: (1) ... (2) ... (3) ... Estimated commit-proposal in <N> wake-ups."
```

The reviewing agent (Codex) reads the plan via its own polling loop. If Codex disagrees with the approach, they post a `counter` — Claude responds before starting implementation.

### Step 3 — Implement, chunk by chunk

For each chunk:
1. Use the appropriate `Unity_*` MCP tool per [`docs/tooling/unity-mcp-routing.md`](../../../docs/tooling/unity-mcp-routing.md) for Editor-touching work.
2. Use Edit / Write for pure file-system work.
3. Post a `note` event for progress visibility:
   ```sh
   scripts/agent-bus post --topic <topic-name> --type note --from claude --to all \
     --body "chunk N/M complete: <one-line summary>"
   ```
4. Run `scripts/fw verify` after non-trivial chunks. If red, fix or escalate per Step 6.

Stay strictly within `files_in_scope`. If a chunk would touch a file outside that scope, **escalate** (Step 6) — do not silently expand scope.

### Step 4 — Verification before commit-proposal

Run all gates that the user named in `acceptance`:

- `scripts/fw verify` — 644 MatchSim + banned-terms + shader-audit + verify-unity-plugins
- L1 / L2 / L3 verification via `unity-check` skill if Unity changed
- Pinned canonical-hash check (run the targeted test; should pass — the `.claude/hooks/canonical-hash-guard.sh` will block commit otherwise)

Collect verification output for the commit-proposal body.

### Step 5 — Post commit-proposal + wait for reviewer ack

```sh
scripts/agent-bus claim --topic <topic-name> --severity p1 --from claude --to codex \
  --body "commit-proposal: <one-paragraph summary of work>. Files touched: <list>. Verification: <verification output summary>. Awaiting reviewer ack."
```

The body MUST start with `commit-proposal:` (case-insensitive) — this is what `scripts/agent-bus pending-reviews` detects.

After posting, ScheduleWakeup with delaySeconds 240 and the wake-up prompt:

```
Check dialog/<topic-name>.jsonl for a reviewer response to the commit-proposal
(in_reply_to matching its sha256, type=ack or type=counter from=codex).

If ack: proceed to Step 6 — git commit.
If counter: read the counter; apply fixes; re-post commit-proposal (new sha);
  ScheduleWakeup again.
If neither after `max_wall_clock_seconds / 2` (default 60 min): post escalation
  per Step 7.

If a from=user, type=decision event lands: stop polling, follow the decision.
```

### Step 6 — Commit (only after reviewer ack)

When Codex posts `type=ack` with `in_reply_to` matching the commit-proposal's sha256:

```sh
git add <files>
git commit -m "$(cat <<EOF
<task title>: <one-line summary>

<commit-proposal body, restated>

Reviewer (codex): ack at <iso-time> on <ack-sha>
Topic: dialog/<topic-name>.jsonl

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>
EOF
)"
```

The `.claude/hooks/canonical-hash-guard.sh` re-runs the pinned-hash test at commit time — if MatchSim canonical state drifted unexpectedly, commit is blocked. The `.claude/hooks/validate-commit.sh` runs the SPEC append-only check + secret scan.

If commit blocked: the hook output names the cause. Either fix and retry OR post `escalation` per Step 7.

### Step 7 — Escalation triggers (any of these hard-stop polling)

Per [`docs/tooling/agent-bus-spec.md §14`](../../../docs/tooling/agent-bus-spec.md):

- Out-of-scope file change attempted
- `design/**.md` / SPEC / CLAUDE / pillar-doc mutation proposed
- `unity-project/Packages/manifest.json` mutation proposed
- Any `Unity_AssetGeneration_*` invocation proposed
- Canonical-hash drift not authorized in task spec
- `scripts/fw verify` red after 2+ fix attempts
- Test-suite regression unfixed after 2+ attempts
- Deadlock with Codex: 3+ rounds of counter without ack convergence
- Reviewer ack timeout (`max_wall_clock_seconds / 2` elapsed since commit-proposal)
- Token budget 80% exhausted (per `scripts/agent-bus stats --topic <topic>`)
- Wall-clock 80% exhausted
- Cascade-risk detected when applying a review-fix: `scripts/agent-bus cascade-check --files <csv> --target-topic <topic>` returns exit 4

On escalation:

```sh
scripts/agent-bus post --topic <topic-name> --type escalation --severity p1 \
  --from claude --to user --body "<trigger>. proposed action: <X>. decision needed: <Y or Z>."
```

Then **stop polling — do NOT ScheduleWakeup**. Wait for user `type=decision` event resolving it. When user posts decision: resume per the decision.

### Step 8 — Task-complete

After commit lands + post-commit `scripts/fw verify` confirms green:

```sh
scripts/agent-bus post --topic <topic-name> --type task-complete --from claude --to user \
  --body "task complete. files touched: <list>. commit: <sha>. acceptance criteria: <bullet-by-bullet pass/fail>. fw verify: GREEN. pinned canonical-hash: UNCHANGED."
```

Topic stays OPEN (only `from=user, type=decision` closes per spec). User reviews at next check-in.

## Cost expectations

- One Tier-2 task ≈ 100-200 KB body bytes (per agent-bus-spec §13) ≈ $5-15 USD at current Claude pricing assuming reasonable cache hit rate
- Codex's cost is on Codex CLI's billing — separate
- Use `scripts/agent-bus stats --topic <topic>` mid-task to monitor

## Cross-references

- [ADR-0012](../../../design/adr/adr-0012-autonomous-implementation-protocol.md) — protocol design
- [agent-bus-spec.md](../../../docs/tooling/agent-bus-spec.md) §13-§16 — schema + cost caps + escalation triggers
- [/duo-debate](../duo-debate/SKILL.md) — Tier-1 architectural debate (use BEFORE Tier-2 if design is not settled)
- [/check-reviews](../check-reviews/SKILL.md) — post-task review-pickup with cascade-prevention
- [/codex-review-loop](../codex-review-loop/SKILL.md) — Codex-CLI-side continuous-poll-and-review

## Sanity checklist before invoking

- [ ] Is the design settled (ADR or SPEC entry exists)? If no → `/duo-debate` first.
- [ ] Is the task bounded (clear acceptance, named files, no creative judgment)? If no → too big for Tier-2; break into smaller specs.
- [ ] Are the cost/time caps reasonable? Default 400K bytes / 2 hours / 50 turns covers most tasks; tighten for narrow ones.
- [ ] Is `scripts/fw verify` currently green on `main`? If no → fix that first; don't run Tier-2 against a broken baseline.
- [ ] Is Codex available + has its `/codex-review-loop` been activated by the user? If no → Claude proceeds and queues for review-on-next-codex-session.

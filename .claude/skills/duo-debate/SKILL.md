---
name: duo-debate
description: Open an autonomous Tier-1 agent-bus topic for Claude + Codex review/brainstorm/architectural-debate. Sets up the polling loop on Claude's side via ScheduleWakeup, generates the prompt for the user to relay to Codex, and posts opening claims. Both agents auto-poll, reply, and terminate via mutual-ack-and-fade or user decision-event. NO repo changes during the discussion — review-only. Use this BEFORE implementing architecturally load-bearing work, when Claude and Codex disagree, or when a design tradeoff warrants a durable decision-record. Do NOT use for one-shot tasks, single-file edits, or anything resolvable in one turn. Triggers on "duo debate", "discuss with codex", "agent-bus debate", "codex review topic".
triggers:
  - duo debate
  - duo-debate
  - discuss with codex
  - agent-bus debate
  - codex review topic
  - cross-model discussion
  - codex brainstorm
---

# Duo-debate — Tier-1 autonomous agent-bus discussion

Set up an autonomous review/brainstorm/architectural-debate session between Claude (this session) and Codex (external CLI). Both agents auto-poll the agent-bus topic; the user can walk away. Discussion ends via mutual-ack-and-fade, user decision-event, or hard time/turn cap.

This is **Tier 1** of the agent-bus protocol per [ADR-0012](../../../design/adr/adr-0012-autonomous-implementation-protocol.md). Tier 1 is review-only — no repo changes. For autonomous bounded-coding tasks, use `/duo-implement` (Tier 2 — ships in a future commit after Codex review of ADR-0012 + Slice 7 dogfood).

## When to invoke

| Situation | Reason |
|---|---|
| About to author an ADR | Draft, debate with Codex, refine before commit |
| Codex reviewed a commit and disagrees on direction | Capture the debate as a durable record before re-implementing |
| Cross-cutting design tradeoff (e.g. "should we use approach A or B?") | Audit-trail beats a single round of copy-paste |
| Validating an architectural assumption that would be expensive to walk back | Get a second pass before commit |
| Closing the loop after a Codex review pass | Record what was applied / rejected / superseded |

## When NOT to invoke

- One-shot tasks: "rename this variable", "fix this typo"
- Single-file edits: "implement this method"
- Anything resolvable in one turn without external review
- Tier-2 implementation work: use `/duo-implement` (when shipped)
- Decisions that need user judgment, not agent debate (creative direction, money, scope cuts)

## Workflow

### Step 1 — User invokes the skill

The user types something like "duo debate slice 7 architecture" or "discuss with codex about X". Claude:

1. Confirms the topic name (kebab-case-ascii; optional ISO date prefix). Example: `2026-05-10-slice7-architecture`.
2. Drafts 3-5 opening events as `from=claude` claims/questions addressed `to=codex`. Each event:
   - `claim` (severity required, p0-p3) for a position Claude is taking
   - `question` for things Claude wants Codex's read on
   - `links` populated with file:line citations or git SHAs
3. Posts the opening events via `scripts/agent-bus claim` / `scripts/agent-bus post`. Each post returns a sha256 hash for `--in-reply-to` threading.

### Step 2 — Generate Codex prompt

Claude writes a self-contained prompt for the user to relay to Codex. Template:

```
The Final Whistle repo has an append-only JSONL agent-bus protocol I am
dogfooding with you in autonomous mode. The user is away. Both of us are
expected to keep polling and responding without human relay until the topic
closes.

Topic file: dialog/<TOPIC>.jsonl
Spec:       docs/tooling/agent-bus-spec.md
CLI:        scripts/agent-bus

I (Claude) have already posted N opening events. Their sha256 hashes are
on each line; use them as --in-reply-to when you reply.

Your job:
1. cd to the repo root
2. cat docs/tooling/agent-bus-spec.md (read the strict schema)
3. cat dialog/<TOPIC>.jsonl (read all my opening events)
4. Reply via:
       export AGENT_BUS_AUTHOR=codex
       scripts/agent-bus post --topic <TOPIC> \
           --type counter --severity p1 --to claude \
           --in-reply-to <sha256> --body "your text" \
           --links "file:line,git:sha"
5. AUTONOMOUS POLLING: poll the topic file every 3-5 minutes for new
   from=claude events; respond to each. Repeat until termination.
6. Termination conditions:
   (a) from=user, type=decision event → topic closed; stop.
   (b) Mutual ack-and-fade: 3+ consecutive polls with no new substantive
       content from either side. Post a final type=note summary.
   (c) Token/wall-clock budget exhausted → post type=escalation, stop.
7. NO REPO CHANGES during the discussion. Read-only.
```

### Step 3 — Claude sets up auto-polling

Use `ScheduleWakeup` with delaySeconds 240 (4 min cadence; keeps cache warm under the 300s TTL). The wake-up prompt includes:

- Re-read `dialog/<TOPIC>.jsonl`
- Identify new from=codex / from=user events since last claude post
- If new: read each, formulate response (counter / evidence / ack / note / question / decision-proposal), post via CLI, ScheduleWakeup again
- If empty: increment empty-poll counter; reschedule
- If counter ≥3 AND prior round had substantive content: post final type=note summary, stop polling, report to user
- If from=user, type=decision lands: topic closed; ack + stop

Hard limits:
- Max 5 reply posts per wake-up
- Max 30 wake-ups total
- Max 90 min wall-clock from first kickoff

### Step 4 — Discussion runs autonomously

Both agents post replies. Subthreads close via mutual ack. New threads may open if either agent surfaces additional concerns. The `dialog/<TOPIC>.jsonl` file is the binding debate-trail.

### Step 5 — Termination

When termination condition fires (mutual-fade, user decision, hard cap), Claude posts a final `note` summary and stops. Reports back to user with:

- Termination reason
- Total turns + total events
- Topic file path
- One-paragraph executive summary
- Workflow-cleanup commit items derived from the discussion (if any)

User reviews the topic file + executive summary, decides what to implement.

## Smoke-tested precedent

`2026-05-09-mcp-migration-debate` — first real-world Tier-1 dogfood. 25 events, 5 rounds, ~30 min wall-clock, mutual-ack-and-fade closure. Outcome: 4 concrete implementation items + 4 tracking items captured for the next workflow-cleanup commit.

## Cost expectations

- Each event ~500 bytes (~125 tokens). 25-event discussion ≈ 12 KB ≈ 3000 tokens of body content.
- Plus polling-loop overhead: ~5 wake-ups × ~10K context = ~50K tokens of polling
- Total: ~$1-2 per Tier-1 discussion at current Claude pricing (assuming reasonable cache hit rate)
- Codex side: variable, depends on Codex's CLI pricing

For tighter budgets, set `AGENT_BUS_MAX_TOKENS` env var before invoking. The CLI enforces a hard ceiling on combined body bytes.

## Debugging

- **Codex doesn't auto-poll on its side.** If Codex's CLI doesn't have a continuous-mode equivalent, the user nudges Codex with "check the bus and reply" each round. Claude's side keeps polling regardless; the loop completes when Codex eventually posts.
- **`agent-bus list` shows the topic but no events**: check that the opening posts actually landed. `cat dialog/<topic>.jsonl` — should have 3-5 lines from the kickoff.
- **Topic doesn't close**: check that the user has not posted a `from=user, type=decision` event. Mutual-fade requires 3+ empty polls AFTER substantive activity. If it's stuck open, post a user decision: `AGENT_BUS_AUTHOR=user scripts/agent-bus post --topic <T> --type decision --severity p1 --to all --body "wrap it up"`.
- **My session ended before the discussion completed**: ScheduleWakeup state is per-session. To resume: re-invoke the skill, name the same topic, Claude reads the existing events, posts any pending replies, re-arms the polling loop.

## See also

- [docs/tooling/agent-bus-spec.md](../../../docs/tooling/agent-bus-spec.md) — protocol spec v0.2.0
- [scripts/agent-bus](../../../scripts/agent-bus) — CLI shim
- [ADR-0012](../../../design/adr/adr-0012-autonomous-implementation-protocol.md) — Tier-2 protocol design (`/duo-implement` lands here)
- [dialog/2026-05-09-mcp-migration-debate.jsonl](../../../dialog/2026-05-09-mcp-migration-debate.jsonl) — first Tier-1 dogfood; reference precedent
- [dialog/README.md](../../../dialog/README.md) — user-facing protocol guide

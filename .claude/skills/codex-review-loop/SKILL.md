---
name: codex-review-loop
description: Continuous-polling continuous-review mode for Codex CLI. Codex polls the agent-bus every N minutes for topics with unreplied commit-proposals or pending review-requests, reviews each, posts ack or counter, and keeps polling. The user activates this once at session start; Codex stays in review-loop mode until they post a `from=user, type=decision, body="stop codex loop"` event or the Codex CLI hits its own hard time/turn cap. Pairs with Claude's `/duo-implement` to enable autonomous Claude-implements / Codex-reviews coordination per ADR-0012. The user never invokes this directly — they hand it to Codex once and Codex sets up the loop. Triggers on "codex review loop", "codex review mode", "codex polling review", "start codex review", "continuous review".
triggers:
  - codex review loop
  - codex-review-loop
  - codex review mode
  - codex polling review
  - start codex review
  - continuous review
---

# Codex-review-loop — continuous-polling continuous-review mode

Codex-CLI-side continuous review of agent-bus topics. While Claude executes Tier-2 implementation tasks via `/duo-implement`, Codex polls the bus, reviews `commit-proposal` events, and posts `ack` (approve) or `counter` (block + reasons) without user relay.

## When to invoke

This skill is **Codex-facing, not Claude-facing**. The user activates it on the Codex side once per session by handing Codex a prompt (this skill file IS the prompt template).

| Situation | Reason |
|---|---|
| User is starting a Tier-2 `/duo-implement` session and wants Codex reviewing in parallel | Primary use case |
| Multiple in-flight Tier-2 topics need review attention | Codex picks them up automatically |
| User is away and wants reviews to happen overnight | The whole point of autonomous mode |

## When NOT to invoke

- One-shot review of a single specific commit/topic (use a single round of agent-bus posting instead)
- Reviews requiring user judgment (creative, design-doc text, money) — Codex escalates these via `type=question --to user` instead of acking/blocking
- Reviewing a topic where Claude isn't actively working (no point polling if nothing's landing)

## How Codex activates the loop

The user pastes this verbatim prompt to Codex CLI:

```
You are entering continuous-review-loop mode on the Final Whistle repo.
Set up an autonomous polling loop using whatever continuous-mode your CLI
supports (a watch-and-poll bash script you author + run in background;
a self-scheduled re-prompt; --continuous flag). Poll cadence: 4-5 minutes.

Repo: /Users/vibelogic/dev/football
Spec: docs/tooling/agent-bus-spec.md
CLI: scripts/agent-bus

Each poll cycle:

1. Run `scripts/agent-bus pending-reviews` (exit code 5 = pending reviews
   exist; exit 0 = nothing pending; exit 2 = error).

2. For each topic returned, run:
   `scripts/agent-bus read --topic <topic-name>`

   Identify the most-recent `commit-proposal:` claim event whose
   `in_reply_to` is NOT referenced by any subsequent ack/counter event.

3. Read the actual files the commit-proposal cites:
   - `git show HEAD -- <file>` to see what changed in the proposed commit
   - `cat <file>` for context
   - `scripts/fw verify` to independently confirm verification (optional;
     cap=1 single-driver per ADR-0011 means you may not have Editor MCP
     access; the implementing agent's verification output in the
     commit-proposal body is the canonical statement)

4. Decide:
   (a) ACK: post `type=ack` with `--in-reply-to <commit-proposal-sha>`,
       body summarizing what you verified.
   (b) COUNTER (block): post `type=counter --severity p1` with
       `--in-reply-to <commit-proposal-sha>`, body naming the specific
       issue + the proposed fix. Be concrete: file:line refs in --links.
   (c) EVIDENCE (supplement without blocking): post `type=evidence`
       supporting an ack with citation; usually a follow-up to your own
       ack.

5. If you find issues that span multiple in-flight topics (i.e. a fix on
   topic A would also fix a bug in topic B), DO NOT chain-ack. Post a
   `note` event on topic A naming the cross-reference; the user triages
   on next check-in.

6. After reviewing all pending topics in this cycle:
   - Sleep 4-5 minutes
   - Repeat from step 1

7. Termination conditions (stop polling):
   (a) A `from=user, type=decision` event lands on ANY topic with body
       "stop codex loop" or "stop review loop" — stop entirely.
   (b) Your CLI hits its own hard time/turn cap — stop, post a `type=note`
       on each in-flight topic explaining you stopped.
   (c) Token budget exhausted (track via `scripts/agent-bus stats`) —
       stop, post `escalation` events as needed.
   (d) 30 consecutive polls with no new pending-reviews — back off to
       longer cadence (20 min) and continue; this is idle-mode.

8. NEVER post a `task-spec` event (those are user-only per spec §15).
   NEVER post a `decision` event closing a topic (only user can).

9. NEVER apply repo changes during a review. Your job is to read +
   comment via the bus. The implementing agent applies fixes based on
   your counters.

10. If a review issue requires creative judgment (tone, aesthetic, scope
    creep, money, design-doc text), DO NOT block — escalate to the user
    via `type=question --to user --severity p1` on the topic. The user
    decides; the implementing agent waits.

11. Cost discipline: budget ~$0.50-2.00 per review cycle at typical
    Codex pricing. Post `type=note` with cost-summary every 10 cycles
    so the user can monitor.

Begin polling now.
```

## What "review" actually means for Codex

A good Codex review:

- **Reads the diff** the commit-proposal cites. Not just the body summary.
- **Re-runs verification** when possible (Codex doesn't need Editor MCP — file system + `scripts/fw verify` is enough for L1).
- **Names specific concerns** with file:line refs in `--links`, not vague "I'm uncomfortable with this approach."
- **Distinguishes blocking from advisory**: counter (P1) blocks the commit; counter (P2/P3) flags concern but doesn't block. Ack means commit can land.
- **Respects scope**: don't counter on out-of-scope concerns. If the task-spec said "don't refactor MatchSim/Sim" and Codex thinks it should be refactored — that's a separate task-spec for the user to issue, not a counter on the current task.

## What Codex MUST NOT do

- Post `task-spec` events (user-only)
- Post `from=user, type=decision` events (user-only; closes topics)
- Make repo changes (Codex's role is read + comment)
- Block a commit on aesthetic / tone / creative grounds (escalate instead)
- Get into agree-loops with Claude (3+ counter rounds without convergence = escalation per §14)
- Pull tasks off topics Codex did NOT receive — only respond to topics where Codex is named `to:` or where commit-proposals are pending

## Coordination with Claude

Claude polls topics where it's the implementing agent. Codex polls for pending-reviews across all topics. They use the same JSONL files but track different "next-action" semantics:

- **Claude on a topic**: waiting for reviewer-ack to commit; waiting for user-decision after escalation; or implementing the next chunk.
- **Codex on a topic**: reading commit-proposals; posting ack/counter; reading task-specs to understand scope.

Cap=1 single-driver per ADR-0011 means only ONE agent can hold the Editor MCP at a time. Convention: Claude holds it during `/duo-implement` runs. Codex stays as filesystem-only reviewer. If Codex genuinely needs Editor access (e.g. to re-capture L2 evidence Claude posted), Codex requests via `type=question --to user` and waits for the user to disconnect Claude.

## Cross-references

- [ADR-0012](../../../design/adr/adr-0012-autonomous-implementation-protocol.md) — protocol design (Codex's reviewer-gate role is Component 2)
- [agent-bus-spec.md](../../../docs/tooling/agent-bus-spec.md) §14 — escalation triggers
- [/duo-implement](../duo-implement/SKILL.md) — Claude's paired implementing-agent skill
- [/duo-debate](../duo-debate/SKILL.md) — Tier-1 architectural debate (no commits to review there)
- [/check-reviews](../check-reviews/SKILL.md) — Claude-side review-pickup that consumes Codex's outputs

## Pre-flight checklist for the user (before pasting to Codex)

- [ ] Has Codex CLI been kept updated? Stale CLI = stale MCP routing knowledge.
- [ ] Does Codex have repo filesystem access? Required for review.
- [ ] Is there at least one in-flight `/duo-implement` topic for Codex to review? Otherwise idle-mode wastes cycles.
- [ ] Token budget agreed with the user — Codex's billing is separate from Claude's; user should know the rough cost expectation.

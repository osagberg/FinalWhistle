# dialog/ — agent-bus topic transcripts

This directory holds append-only JSONL transcripts of structured debate
between Claude (the in-repo agent), Codex (the external review-pass agent),
and the user. One file per topic. Append-only.

The protocol exists because copy-paste relay between two LLMs loses
context, encourages agreement loops, and leaves no audit trail. Agent-bus
gives both models a shared transcript so claims, counter-claims, evidence,
and decisions are all addressable + grep-able six months later.

For the full spec see [`docs/tooling/agent-bus-spec.md`](../docs/tooling/agent-bus-spec.md).

---

## Quick start

### Read

```sh
# All topics, with status
scripts/agent-bus list

# Just open topics
scripts/agent-bus list --open

# A specific topic, full transcript
scripts/agent-bus read --topic 2026-05-09-mcp-migration-routing

# Filter
scripts/agent-bus read --topic foo --since 2026-05-08T00:00:00Z --type claim
```

You can also just `cat dialog/<topic>.jsonl` — every line is a JSON object.

### Post

```sh
export AGENT_BUS_AUTHOR=claude   # or codex, or user

# Open a topic with a claim
scripts/agent-bus claim \
    --topic 2026-05-09-foo-vs-bar \
    --severity p1 \
    --to codex \
    --body "I think foo beats bar because X."

# The CLI prints the sha256 of the appended event. Use it as in-reply-to.

scripts/agent-bus post \
    --topic 2026-05-09-foo-vs-bar \
    --type counter \
    --severity p1 \
    --to claude \
    --in-reply-to <sha256-printed-above> \
    --body "Disagree — Y is the load-bearing factor."
```

### Close

Only an event with `from: user` and `type: decision` closes a topic.
Agents may post their own `decision`-type events as proposals, but only
the user's decision is binding.

```sh
AGENT_BUS_AUTHOR=user scripts/agent-bus post \
    --topic 2026-05-09-foo-vs-bar \
    --type decision \
    --severity p1 \
    --to all \
    --body "Going with foo. Append SPEC entry."
```

---

## File layout

```
dialog/
  README.md                                  # this file
  example-topic.jsonl                        # worked example
  2026-05-09-mcp-migration-routing.jsonl     # one topic per file
  ...
```

Each `.jsonl` file is append-only. The future pre-commit hook
(`.claude/hooks/protect-dialog-append-only.sh`, deferred — not in v0.1.0) will
reject any commit that modifies prior lines. Until that ships, the
append-only invariant is a social contract.

---

## Topic naming

- **kebab-case-ascii**, lowercase. Match `^[a-z0-9-]+$`.
- **Optional ISO-date prefix** when temporal context matters:
  `2026-05-09-slice7-identity-cues`. The date is the topic's open date,
  not a deadline.
- Be specific. `2026-05-09-routing` is too vague — what routing? Use
  `2026-05-09-mcp-migration-routing` instead.

---

## Lifecycle

```
open ──── debate ──── user-decision ──── closed
 │           │                              │
 │           │                              └─ later events allowed but advisory
 │           │
 │           └─ claim, counter, evidence, question, ack, note events
 │
 └─ first event: usually a claim
```

A topic is **open** until an event with `from: user` and `type: decision`
is appended. Then it is **closed**. Subsequent events on a closed topic
are advisory (e.g. a `note` recording that the decision held up; an
`evidence` event from later work confirming it).

To overturn a closed decision, **open a new topic** that cites the closed
one in `links`. Never rewrite a prior line.

---

## How each reader uses this

### Claude (this repo's main agent)

At session start, after reading CLAUDE.md / STATUS.md / SPEC.md:

```sh
scripts/agent-bus list --open
```

For each open topic where Claude is `to:` or has prior events: read it,
think, post. CLAUDE.md §6.3 mandates this on session start.

### Codex (external review-pass agent)

Codex has repo filesystem access during review sessions. It reads
`dialog/*.jsonl` directly. When the user pastes a Codex review back into
the Claude session, Codex's review should also be appended to the relevant
topic file by Codex itself (or by the user paste-relaying as
`AGENT_BUS_AUTHOR=codex`).

### The user

Read with `agent-bus read`, or open the JSONL in any editor. Decide by
posting a `decision` event with `AGENT_BUS_AUTHOR=user`.

---

## Examples of good usage

- **Architectural disagreement.** Claude proposes ScriptableObject for
  X; Codex counters with plain C# record. Both post claim/counter with
  evidence (file:line refs to similar prior decisions). User decides.
  SPEC decisions-log entry cites the topic.
- **Closing a Codex review loop.** After Codex round-N findings land,
  Claude posts an `evidence` event summarising what was applied vs
  what was rejected, with reasoning. Future sessions can grep for it.
- **Recording a near-miss.** Topic closes with the user's decision. A
  week later, work in that area surfaces an edge case the rejected
  alternative would have handled. Append a `note` to the closed topic
  recording the lesson, cross-link from the new topic that re-opens
  the question.

## Anti-patterns

- **Chat without claims.** Don't use agent-bus for "hi how are you"
  filler. Every event should be a claim, counter, evidence, question,
  decision, ack, or note. If it's none of those, don't post it.
- **Missing severity on a claim.** The CLI rejects this. The point of
  severity is to let the user prioritise — without it, the topic is
  noise.
- **No links/evidence.** A claim without a `links` array citing
  file:line refs or git SHAs is hard to verify. Evidence events without
  links are nearly worthless.
- **Rewriting a prior event.** Don't. Append a new event with
  `in_reply_to` pointing at the prior one. This is the load-bearing
  invariant of the entire protocol.
- **Closing your own topic.** Agents posting `decision` events with
  `from: claude` or `from: codex` is allowed (as a proposal) but does
  NOT close the topic. Only `from: user` decisions close.
- **One-shot mundane tasks.** "Should I name this variable foo or bar?"
  is not a dialog topic. Just pick one. Use agent-bus for things worth
  recording six months from now.

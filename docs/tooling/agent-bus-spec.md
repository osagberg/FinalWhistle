---
description: Agent-bus protocol specification — append-only JSONL dialog files at dialog/<topic>.jsonl for asynchronous structured collaboration between Claude (in-repo agent), Codex (external review-pass agent), and the user. Replaces ad-hoc copy-paste relay with a versioned, audit-able, threaded debate ledger. Authored 2026-05-09.
---

# Agent-bus protocol specification

> Append-only JSONL dialog files at `dialog/<topic>.jsonl` for asynchronous
> structured collaboration between Claude (this repo's main agent), Codex
> (external review pass agent), and the user.
>
> Version: 0.1.0. Authored 2026-05-09.

---

## 1. Why

The established cross-model rhythm (Claude drafts → Codex reviews → user
relays findings → Claude applies) has shipped working code through five+
audit rounds, but the relay layer has three failure modes:

1. **Context loss in copy-paste.** When the user pastes Codex findings into
   the Claude session, the framing around each finding (severity, file refs,
   prior reasoning) is often dropped or compressed. Claude responds to the
   compressed version, not the original. Codex's next round then re-litigates
   what was already said.
2. **Agreement-loop risk.** With no shared transcript, both agents drift
   toward agreeing with the user's last paste. There is no audit trail
   showing where one model held a position the other initially disagreed
   with — important context for understanding *why* a decision landed.
3. **No audit trail.** SPEC.md decisions-log captures the *outcome*. It does
   not capture the debate, the alternatives Codex flagged, or the evidence
   Claude cited. Six months from now nobody will remember why row 3 of the
   slice-6 routing table reads the way it does.

The agent-bus fixes those by giving both models (and the user) a shared,
append-only transcript per topic. Claude reads it at session start. Codex
reads it via repo filesystem access. The user reads it as plain JSONL.

This protocol is **not** real-time. Latency is minutes, not seconds. It is
**not** a chat replacement — it is a structured-claim + counter-claim +
evidence + decision ledger.

## 2. Format

- **Encoding**: UTF-8 JSONL. One JSON object per line. No trailing newline
  required on the final line, but tooling MUST tolerate one.
- **Location**: `dialog/<topic>.jsonl` at repo root.
- **Append-only**: every event is a new line. Prior lines are immutable.
  Enforced by social contract + a future pre-commit hook (see §6).
- **One topic per file**. Cross-topic references via `links`.

## 3. Topic naming

- **kebab-case-ascii**, lowercase, hyphens only. Match `^[a-z0-9-]+$`.
- **Optional ISO date prefix** when temporal context matters:
  `2026-05-08-slice6-uitoolkit-vs-ugui`.
- **Lifecycle**:
  - **open** — no `decision` event with `from: user` exists yet.
  - **closed** — at least one `decision` event with `from: user` exists.
- Subsequent events to a closed topic are advisory only. They do not
  re-open the topic. To overturn a closed decision, open a new topic that
  cites the prior one in `links`.

## 4. Schema

Canonical record:

```json
{
  "time": "2026-05-09T00:12:33Z",
  "from": "claude",
  "to": "codex",
  "topic": "2026-05-08-slice6-uitoolkit-vs-ugui",
  "type": "claim",
  "severity": "p1",
  "body": "Free-form markdown body. Cite file:line in links, not here.",
  "links": ["unity-project/Assets/Viewer/UI/Hud.uxml:42", "git:67c0905"],
  "in_reply_to": null
}
```

### 4.1 Field semantics

| Field | Required | Type | Notes |
|---|---|---|---|
| `time` | yes | string | Strict ISO 8601 UTC, second precision, `Z` suffix. Match `^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$`. |
| `from` | yes | enum | `claude` \| `codex` \| `user`. The author. |
| `to` | yes | enum | `claude` \| `codex` \| `user` \| `all`. Primary addressee. Advisory only — anyone can read any event. |
| `topic` | yes | string | Matches the filename (without `.jsonl`). |
| `type` | yes | enum | See §4.2. |
| `severity` | conditional | enum or null | See §4.3. |
| `body` | yes | string | UTF-8 markdown. Non-empty. ≤ 4096 bytes. |
| `links` | no | array or null | Each entry a string. Free-form refs (file:line, git:sha, PR/issue URL). |
| `in_reply_to` | no | string or null | sha256 hex of a prior event's canonical encoding (§5). |

### 4.2 Type enum

- **claim** — a position the author is taking. Most opening shots are claims.
- **counter** — a position that disagrees with a prior claim/counter. MUST set `in_reply_to`.
- **evidence** — supporting data for a claim/counter. SHOULD set `in_reply_to` and SHOULD populate `links`.
- **question** — a request for clarification. Often from `user` to one of the agents, or between agents.
- **decision** — an authoritative ruling. **Only `from: user` decisions close a topic.** Agents may post `decision`-type events to record their own concurrence or proposed ruling, but those do not close the topic.
- **ack** — explicit acknowledgement of a prior event without adding new content. Use sparingly; silence is also acknowledgement.
- **note** — out-of-band annotation (timing, context, cross-ref). Does not advance the debate.

### 4.3 Severity enum

- **p0** — blocker. The current state is broken; nothing else matters until this is resolved.
- **p1** — high. Materially affects scope, schedule, or correctness; should be resolved before merge.
- **p2** — medium. Worth resolving but not blocking.
- **p3** — low. Polish, nit, future-look.
- **null** — no severity (only valid for `note`, `ack`, `question`).

**Required for**: `claim`, `counter`, `evidence`. (Evidence inherits the severity of the claim it supports, but stating it explicitly avoids ambiguity when evidence cuts across multiple claims.)

**Forbidden for**: `note`, `ack`. (Set `severity: null` or omit.)

**Optional for**: `question`, `decision`. (Decisions inherit the severity of the highest open claim they resolve.)

## 5. Canonical encoding + sha256

`in_reply_to` and any future content-addressed reference uses sha256 over the **canonical encoding** of the target event:

1. Object keys serialised in this exact order:
   `time, from, to, topic, type, severity, body, links, in_reply_to`.
2. JSON serialisation with no extra whitespace (`{"time":"...","from":"..."}`),
   no trailing newline.
3. `null` values written explicitly (do NOT omit absent fields).
4. `links` array serialised in the order it appears on the line; do not re-sort.
5. UTF-8 encode, sha256, lowercase hex.

This means each line in the file should already be in canonical form when written, and `sha256(line_bytes)` gives the addressable hash. The `agent-bus` CLI writes canonical form by construction.

## 6. Append-only enforcement

A pre-commit hook (or CI check) rejects any commit where a `dialog/*.jsonl`
file has any line modified that was present in the file's previous git
revision. Only line *additions* at end-of-file are permitted. Truncation,
mid-file insertion, line edits — all rejected.

The hook implementation lives at `.claude/hooks/protect-dialog-append-only.sh`
(deferred — not part of v0.1.0; the social contract is documented here in
the meantime). The `agent-bus` CLI does not bypass this — it always uses
append (`>>` to a tmp file, `mv` atomic).

## 7. Ordering

Events have a partial order via `in_reply_to`. Total order for display is
**stable sort** by:

1. `time` ascending.
2. sha256 ascending (tiebreak — two events at the same second).

Threading clients SHOULD render `in_reply_to` chains as nested replies but
MUST also tolerate flat chronological rendering.

## 8. Closure semantics

A topic is **closed** the moment a `decision` event with `from: user` is
appended. The CLI's `agent-bus list --closed` filter uses this rule.

After closure:
- New events to the topic file are valid (the file is still append-only)
  but are **advisory**. They do not re-open the topic.
- Either agent may append a `note` recording follow-up insight, or an
  `evidence` event documenting that the decision held up under later work.
- To **overturn** a closed decision, open a new topic. The new topic's
  opening `claim` should cite the closed topic in `links` (e.g.
  `links: ["dialog/2026-05-08-foo.jsonl"]`).

This mirrors the SPEC.md decisions-log discipline: append-only, supersession
by reference, never by rewriting.

## 9. Threading

Nested replies via `in_reply_to`. The reply chain is a DAG (an event MAY have
multiple events replying to it — branches), not a strict linear thread.

Agents SHOULD set `in_reply_to` whenever an event is a direct response to
exactly one prior event. When responding to multiple prior events
synthetically, set `in_reply_to: null` and reference them in `links`.

## 10. Non-goals

- **Real-time chat.** Latency is minutes. Both agents read at session start
  and periodically when the user prompts them to.
- **General-purpose IPC.** This is for cross-model design debate, not for
  e.g. queueing build jobs or live coordination.
- **Replacing SPEC.md decisions-log.** SPEC captures the outcome and binds
  the project to it. Agent-bus captures the debate that led there. Both
  are append-only. They are complementary.
- **Replacing PR review.** Code review still happens via the
  `pr-review-toolkit` subagent rotation per CLAUDE.md §6.3. Agent-bus is
  for design/architecture debate, not line-by-line code review.

## 11. Validation rules summary

A consumer MUST reject an event if:

- `time` does not match the strict ISO 8601 UTC regex.
- `from`, `to`, or `type` is not in its enum.
- `severity` is missing on a `claim` / `counter` / `evidence`.
- `severity` is non-null on a `note` / `ack`.
- `body` is empty or > 4096 bytes.
- `links` is present and not an array of strings.
- `in_reply_to` is present and not 64-hex-char or `null`.
- The line is not valid JSON.

A consumer MAY warn (not reject) if:

- An event has `in_reply_to` pointing to a hash not present earlier in the file.
- A `decision` from a non-user author is appended to a topic.
- A topic file's filename does not match the `topic` field of its events.

## 12. Versioning

This is spec v0.1.0. Schema additions (new optional fields) are
backwards-compatible. Schema removals or enum-value removals require a
new major version + migration plan. Topic files do not embed a version —
the CLI version (`scripts/agent-bus version`) is the source of truth for what
schema is being written.

## Last verified

2026-05-09 — schema validated round-trip via the example dialog at `dialog/example-topic.jsonl`. CLI smoke-test (post / read / list) succeeded.

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
4. **Cap=1 single-driver.** With Unity AI Assistant `MaxDirect = 1` per
   Pro seat (per ADR-0011), Claude AND Codex cannot both hold the Editor
   MCP at the same time. The bus is the only viable async-coordination
   channel between us when one is actively driving the Editor and the
   other cannot connect. Confirmed by 2026-05-09 local-test: Codex hit
   `ValidationReason: Your MCP connections limit is reached (1/1)` while
   Claude was approved as the active driver. The bus exists to let us
   coordinate work *despite* this constraint, not in spite of it.

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

**Tier-1 types (review / brainstorm / debate — protocol v0.1.0 base):**

- **claim** — a position the author is taking. Most opening shots are claims.
- **counter** — a position that disagrees with a prior claim/counter. MUST set `in_reply_to`.
- **evidence** — supporting data for a claim/counter. SHOULD set `in_reply_to` and SHOULD populate `links`.
- **question** — a request for clarification. Often from `user` to one of the agents, or between agents.
- **decision** — an authoritative ruling. **Only `from: user` decisions close a topic.** Agents may post `decision`-type events to record their own concurrence or proposed ruling, but those do not close the topic.
- **ack** — explicit acknowledgement of a prior event without adding new content. Use sparingly; silence is also acknowledgement.
- **note** — out-of-band annotation (timing, context, cross-ref). Does not advance the debate.

**Tier-2 types (autonomous bounded-coding tasks — added v0.2.0 per [ADR-0012](../../design/adr/adr-0012-autonomous-implementation-protocol.md)):**

- **task-spec** — a structured task brief from the user that opens an autonomous-implementation topic. Body MUST include the fields named in §15 (acceptance, files_in_scope, files_out_of_scope, max_tokens, max_wall_clock_seconds, escalation_triggers, required_subagents). Severity OPTIONAL (defaults to p1). Posted by `from: user` only — agents must not self-issue task specs.
- **escalation** — a hard-stop signal. The implementing or reviewing agent posts when an escalation trigger fires (out-of-scope file change attempted, fw-verify red >2 attempts, canonical-hash drift unauthorized, design-doc/SPEC mutation proposed, manifest.json mutation proposed, asset-generation tool invocation proposed, deadlock 3+ counter rounds, token/wall-clock budget 80% exhausted). Body MUST name the trigger + the proposed action + the decision the user needs to make. Severity REQUIRED (typically p0 or p1). Agent stops polling until user posts a `decision` event resolving it.
- **task-complete** — implementing agent posts when all task-spec acceptance criteria are met + reviewer agent has acked the commit-proposal + the commit has landed. Body summarizes work done, files touched, verification output. Severity FORBIDDEN. Topic stays OPEN (only `from: user, type: decision` closes); user reviews on next check-in and may post a `decision` to close, or a `counter` if regression discovered later.

**Tier-2 reviewer-gate events (use existing `commit-proposal` / `commit-approval` / `commit-block` semantics via existing types):**

- **commit-proposal** — implementing agent posts a `claim` (severity p1) with body header `commit-proposal:` and the diff summary + verification output. Reviewer responds with `ack` (approve) or `counter` (block + reasons). Commit lands ONLY after reviewer `ack`. Different semantic from current `pr-review-toolkit` rhythm where review runs post-commit.

The Tier-2 types extend Tier-1 — they do not replace it. A `/duo-debate` topic uses only Tier-1 types; a `/duo-implement` topic uses both.

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

This is spec v0.2.0 (was v0.1.0 at first ship 2026-05-09; v0.2.0 adds
Tier-2 type enum + §13-§15 sections per [ADR-0012](../../design/adr/adr-0012-autonomous-implementation-protocol.md)).
Schema additions (new optional fields, new enum values) are
backwards-compatible. Schema removals or enum-value removals require a
new major version + migration plan. Topic files do not embed a version —
the CLI version (`scripts/agent-bus version`) is the source of truth for what
schema is being written.

## 13. Cost + time + turn caps (Tier-2 enforcement)

Autonomous Tier-2 implementation runs need hard ceilings to bound runaway
loops + cost surprise. The CLI enforces three caps via env vars set by
the user when issuing the task spec:

- **`AGENT_BUS_MAX_TOKENS`** — sum of `body` byte counts across all events
  on the topic, used as a proxy for combined Claude+Codex token spend
  (rough; a 4-byte body roughly equals 1 token). Default 400000 bytes
  (~$5-15 USD per task at current pricing depending on cache hit rate).
  Behavior: agent posts `escalation` at 80% reached; CLI rejects further
  posts at 100% reached.
- **`AGENT_BUS_MAX_WALL_CLOCK`** — seconds since the topic's first event.
  Default 7200 (2 hours). Agent posts `escalation` at 80%; CLI rejects
  posts at 100%.
- **`AGENT_BUS_MAX_TURNS`** — max events per agent author before mandatory
  user check-in. Default 50 per agent. Agent posts `escalation` at limit;
  CLI rejects further posts from that author until the user posts an
  `ack` resetting the count.

These are best-effort defenses, not bulletproof. The implementing agent's
own `ScheduleWakeup` chain is the primary cost driver — caps cannot
prevent every runaway, but they bound it and force escalation. Caps are
optional for Tier-1 review/brainstorm topics (debate is naturally
self-limiting via mutual-ack-and-fade).

## 14. Escalation triggers (Tier-2)

The implementing or reviewing agent posts a `type: escalation` event when
any of these fire. Escalation is a HARD STOP: the agent stops polling
until the user posts a `decision` event resolving it.

**Scope-violation triggers:**

- Out-of-scope file change attempted (touches a path not in `files_in_scope`
  or matched by `files_out_of_scope`).
- `design/**.md` modification proposed (creative-judgment domain).
- `SPEC.md` decisions-log mutation proposed (architecture-bearing).
- `CLAUDE.md` / `TECH_APPROACH.md` / `TOOLING.md` / `PROJECT_CONTEXT.md`
  modification proposed (process / pillar domain).
- `unity-project/Packages/manifest.json` mutation proposed (paid-package
  risk).
- `Unity_AssetGeneration_*` tool invocation proposed (Unity AI credit burn
  risk; user pre-approval mandatory).

**Quality-violation triggers:**

- `scripts/fw verify` red after 2+ fix attempts.
- Canonical-hash drift not authorized in the task spec (the
  `.claude/hooks/canonical-hash-guard.sh` hook also blocks the commit, but
  the agent should escalate before reaching commit time).
- Pre-existing test suite (`MatchSim.Tests` 644 / EditMode 89+) regresses
  with no fix in 2+ attempts.

**Coordination-violation triggers:**

- Deadlock: 3+ rounds of `counter` between Claude and Codex without
  convergence. (Counts only when severity p1 or p0; p2-p3 disagreements
  aren't escalation-worthy.)
- Reviewer agent does not ack a `commit-proposal` within
  `max_wall_clock_seconds * 0.5` (default 60 min). Implementing agent
  must NOT auto-commit without ack.
- Token budget 80% exhausted (per §13).
- Wall-clock 80% exhausted (per §13).
- Turn count 80% exhausted (per §13).

User responds to escalation with a `type: decision` event naming the
resolution: "approve, proceed", "deny, revert", "expand scope", "extend
budget", "pivot". Agent resumes after decision (topic stays open).

## 15. Task-spec event format (Tier-2)

The opening event of a `/duo-implement` topic. Posted by `from: user`. Body
is structured markdown with these required fields:

**Required fields** (CLI rejects a task-spec missing any of these with exit 2):

```
acceptance:
  - <falsifiable acceptance criterion 1>
  - <falsifiable acceptance criterion 2>
  - ...

files_in_scope:
  - <glob pattern 1>
  - <glob pattern 2>
  - ...

files_out_of_scope:
  - design/**.md
  - SPEC.md
  - CLAUDE.md
  - TECH_APPROACH.md
  - TOOLING.md
  - PROJECT_CONTEXT.md
  - SETUP.md
  - .claude/agents/**
  - .claude/rules/**
  - .claude/hooks/**
  - MatchSim/Sim/Q3232.cs
  - MatchSim/Sim/Tick.cs
  - MatchSim/Sim/Seed.cs
  - <any task-specific additions>

max_tokens: 400000
max_wall_clock_seconds: 7200
```

**Recommended fields** (strongly encouraged for Tier-2 implementation tasks; not CLI-enforced because some flows legitimately omit them — e.g. a task-spec posted manually for a hotfix may not name `implementing_agent` because it's implicit context):

```
max_turns: 50

escalation_triggers:
  - <any custom triggers; default set in §14 always applies>

required_subagents:
  - <subagent ID per CLAUDE.md §6.3 mandatory rotation table>
  - <e.g. unity-specialist if the task touches Unity/Viewer>
  - <e.g. pr-review-toolkit:silent-failure-hunter for the pre-commit triple>

implementing_agent: claude    # or codex
reviewing_agent: codex        # the other

depends_on:
  - <other-topic-id-that-must-task-complete-before-this-one-starts>
  - <or that this one is logically downstream of>

notes:
  <free-form context, links to design docs, prior-art, anything the
   implementing agent needs to know that doesn't fit the structured
   fields above>
```

Per Codex 2026-05-11 P1 (ca1cc8ff): the prior spec text listed all 10 fields as required, but cmd_task_spec only validated 5. Two options were considered: (a) tighten the CLI to enforce all 10, (b) narrow the spec text to match the 5 actually-enforced. Option (b) chosen because the recommended-5 set has legitimate omission cases — see inline notes. Strict workflows (e.g. autonomous Tier-2 via `/duo-implement`) SHOULD provide all 10; manual/hotfix workflows MAY omit recommended-5 without rejection.

The CLI's `task-spec` subcommand validates the required-5 structure on post. Agents read the spec on first poll + acknowledge with `ack` before proceeding.

The `depends_on` field is OPTIONAL but RECOMMENDED for any task that's
logically downstream of another. It feeds two systems: (a) the cascade-
prevention check in §16 reads `depends_on` as a declarative hint
alongside the mechanical file-overlap check; (b) future tooling may
gate a task's start on its `depends_on` topics reaching `task-complete`.
The field is informational at v0.2.0 — the spec doesn't enforce
ordering yet. Agents SHOULD respect it; the user MAY override.

## 16. Cascade-prevention semantics (Tier-2 review-finding application)

When a Tier-2 task ships, Codex reviews post-commit and posts findings
(`counter` / `evidence` events). If Claude has moved on to a downstream
task that depends on the same files, naive auto-application of the
finding produces cascading bugs (the downstream task was built against
the pre-fix state; the fix mutates that state; the downstream task
breaks).

The cascade-prevention rule: **before applying any review-fix that would
mutate a file, run `scripts/agent-bus cascade-check --files <csv>
--target-topic <orig-topic>`**. The CLI walks all in-flight task-spec
topics (task-spec landed AND no `task-complete` AND no user-decision
closure) and reports overlap between the candidate files and the
in-flight `files_in_scope` lists.

Exit codes:
- **0** — no overlap; cascade-safe; apply the fix.
- **4** — overlap detected; defer to user triage. Post a `note` on the
  affected topic naming the cross-reference + an `escalation` event
  with proposed-action alternatives.
- **2** — argument error.

Match semantics: lowercase-substring match of each candidate file path
against the task-spec body. **Coarse and defensive by design**: false-
positive (over-counting overlap → triggers user check-in → safe) is
preferred over false-negative (missing real overlap → cascade bug ships).
Glob-aware matching is a future precision improvement (P3; tracked but
not blocking).

The `--target-topic <orig-topic>` flag excludes the topic the fix is
TARGETING from the in-flight overlap check — overlap with self is
expected (the fix is on the task's own files; that's allowed).

Operational pattern (from [/check-reviews](../../.claude/skills/check-reviews/SKILL.md)):

1. `scripts/agent-bus pending-reviews` — find topics with unreplied
   commit-proposals (exit 5 = pending exists).
2. For each pending topic, read Codex's latest counter/evidence/question.
3. Identify files the fix would touch (from Codex's body + links).
4. `scripts/agent-bus cascade-check --files <csv> --target-topic <topic>` —
   exit 0 means safe to apply; exit 4 means defer + escalate.
5. If safe: apply, run `fw verify`, re-post commit-proposal on the
   original topic, wait for new reviewer ack, commit.
6. If risky: post `note` + `escalation` on the original topic; wait for
   user decision. Continue with primary work uninterrupted.

## Last verified

2026-05-09 — Tier-1 schema validated end-to-end via `2026-05-09-mcp-migration-debate` topic (25 events, mutual-ack-and-fade closure). Tier-2 schema v0.2.0; cascade-prevention §16 + `cascade-check` / `pending-reviews` CLI subcommands smoke-tested 2026-05-11. Not yet dogfooded on a real Tier-2 task — Slice 7 is the proposed first dogfood per [ADR-0012](../../design/adr/adr-0012-autonomous-implementation-protocol.md) §Acceptance criteria.

---
description: ADR-0012 — Autonomous Tier-2 implementation protocol. Bounded coding tasks executed by Claude + Codex via the agent-bus, with structured scope contracts, reviewer-gate-before-commit, auto-rollback on canonical-hash drift, user-escalation triggers for design/money/pillar decisions, and hard cost+time caps. The user defines the task, agents execute the implementation, reviewer agent gates the commit, user reviews diff at next check-in. Ships scaffolding (ADR + spec extensions + hooks + Tier-1 /duo-debate skill); /duo-implement skill deferred to next session pending Codex review of this ADR. Authored 2026-05-09 in autonomous mode per user instruction "make improvements so it works with you guys auto working through tasks, coding together etc."
---

# ADR-0012: Autonomous Tier-2 implementation protocol

## Status

**Proposed** — 2026-05-09; amended 2026-05-11 with §Component 6 cascade-prevention. Drafted by Claude in autonomous mode per user instruction. **All four skills (`/duo-debate`, `/duo-implement`, `/codex-review-loop`, `/check-reviews`) shipped 2026-05-09 → 2026-05-11** alongside the agent-bus extensions; Codex review pass IS NOW IN PROGRESS via agent-bus topic `2026-05-10-adr-0012-autonomous-tier-2-review` (Codex's 5 P1/P2 counters posted 2026-05-11; fixes in progress). Promotion to **Accepted** gated on (a) Codex's review pass landing on the bus (in progress; 5 P1/P2 blockers identified, fix commits expected before promotion), (b) at least one successful dogfood of `/duo-implement` against a real bounded task (proposed first dogfood: Slice 7 of dots-adapter ladder, post-P1-blocker-fixes per Codex's 2026-05-11 ack), and (c) user sign-off on both. The protocol scaffolding (skills + CLI subcommands + hooks + spec) is shipped; the structural review pass + first dogfood are the remaining acceptance gates.

## Date

2026-05-09 (drafted)

## Last Verified

2026-05-09

## Decision Makers

osagberg (project owner — defined the scope: "for the grunt work, the stuff you guys can discuss and code together"; explicitly NOT for asset/money decisions or one-command-builds-game), Claude (workhorse author, autonomous mode), Codex (review pending, agent-bus topic).

---

## Summary

A protocol that lets Claude + Codex execute **bounded coding tasks** autonomously between user check-ins, with the user retaining authority over creative + scope + money decisions. The user writes a task spec; the implementing agent works the spec; the reviewing agent gates the commit; both agents escalate to the user when triggers fire. Verification via `scripts/fw verify` + canonical-hash regression check is mandatory pre-commit. Hard cost + time caps prevent runaway loops. Implementation is layered:

- **Tier 1 — `/duo-debate`**: review / brainstorm / architectural debate. No repo changes. **Ships 2026-05-09.** Proven by the `2026-05-09-mcp-migration-debate` dogfood.
- **Tier 2 — `/duo-implement`**: bounded coding task with scope contract + reviewer gate + auto-rollback + escalation triggers. **Skill ships 2026-05-11** alongside `/codex-review-loop` (Codex-CLI continuous-poll continuous-review mode) and `/check-reviews` (Claude-side review-pickup with cascade-prevention). First operational dogfood (Slice 7) gated on Codex's 2026-05-11 P1 blockers being addressed.
- **Tier 3 — phase-spanning autonomous work**: explicitly OUT of scope. Creative judgment + scope + money decisions stay with the user. The protocol is grunt-work-tier only.

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Unity 6.0.4 (`6000.4.4f1` tech-stream); URP 17.4.0. Protocol is engine-agnostic but interacts with Unity-side workflows via `unity-check` skill + Editor MCP routing per ADR-0011 |
| Domain | Process / tooling / cross-agent coordination | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | MEDIUM — first attempt at structured cross-agent autonomous coding workflow in this project; Tier-1 dogfooded successfully tonight, Tier-2 unproven |
| References Consulted | `2026-05-09-mcp-migration-debate` agent-bus topic (Tier-1 dogfood, 25 events, mutual-ack-and-fade closure), [ADR-0011](adr-0011-unity-ai-assistant-mcp-migration.md) (Editor MCP migration), `docs/tooling/agent-bus-spec.md` v0.1.0, CLAUDE.md §6.3 (delegation discipline), `scripts/fw verify` (verification floor) |
| Post-Cutoff APIs Used | `ScheduleWakeup` (Anthropic Claude Code harness self-firing wake-up; lets Claude poll the bus autonomously between user check-ins) |
| Verification Required | Tier 1: already verified end-to-end 2026-05-09. Tier 2: requires Codex review + Slice 7 dogfood + user sign-off before promotion to Accepted. |

## Dependencies

| Field | Value |
|---|---|
| Depends On | Agent-bus protocol v0.1.0 (`docs/tooling/agent-bus-spec.md`), `scripts/agent-bus` CLI, `scripts/fw verify` Tier-A umbrella, ADR-0011 (Editor MCP routing — single-driver constraint enforces "Claude implements / Codex reviews via filesystem" pattern), CLAUDE.md §6.3 (subagent rotation table) |
| Enables | Reduced user burden on bounded coding tasks; agents can take Slice 7 (and analogous bounded work) end-to-end while user does creative + scope decisions; next-session implementation efficiency on grunt work |
| Blocks | Nothing — protocol is opt-in via `/duo-implement` invocation. Manual implementation rhythm (Claude drafts → Codex reviews → user approves) remains available indefinitely |

---

## Context

### Problem statement

Solo-dev project. The user has been doing manual relay between Claude (workhorse implementer) + Codex (adversarial reviewer) via copy-paste of findings + diffs. The agent-bus protocol (shipped in commit `452ab95`) addressed the relay's audit-trail and context-loss failure modes for *architectural debate*. Tonight's `2026-05-09-mcp-migration-debate` dogfood confirmed Tier-1 review/brainstorm works end-to-end with zero user relay (25 events, 5 rounds, ~30 min, mutual-ack-and-fade termination).

But the user's actual time sink is not architectural debate. It's the relay during *implementation* — copy-pasting code review findings, applying diffs, running verify, syncing SPEC/CHANGELOG. The user named it directly: "for the grunt work, the stuff you guys can discuss and code together, then yeah."

The natural extension of the agent-bus is therefore Tier 2: bounded coding tasks where one agent implements + the other reviews, both via the bus, with verification gates and escalation triggers preventing runaway. This ADR designs that protocol.

### Constraints

- **Creative authority stays with the user.** Anything touching `design/**.md`, SPEC decisions log, ADRs, pillars (PROJECT_CONTEXT.md USPs, CLAUDE.md §1), or visible tone — escalate. Agents implement specs, they do not author them.
- **Money authority stays with the user.** Anything that costs credits, calls a paid API, mutates `unity-project/Packages/manifest.json` to add a paid package — escalate. Asset-generation tools especially: a `Unity_AssetGeneration_GenerateAsset` call burns Unity AI credits and should require user pre-approval.
- **Determinism is non-negotiable.** Pinned 60-tick `MatchCanonicalState` hash must remain unchanged unless the task explicitly authorizes it. Any unexpected hash drift triggers auto-rollback.
- **Verification floor is `scripts/fw verify` green.** No commit lands without it. The reviewer agent verifies by re-running locally (or trusting the implementing agent's output if cap=1 makes parallel verification expensive).
- **Cost + time caps are mandatory.** Token budget per task + wall-clock cap per task. Runaway agents stop and post `escalation`.
- **Cap=1 single-driver per ADR-0011** means Claude AND Codex cannot both hold the Editor MCP simultaneously. The implementing agent holds the slot during implementation; the reviewing agent reviews via filesystem + diff + bus. Reviewing agent does NOT need Editor access for code review — it needs Editor access only for L2/L3 visual verification, which can happen post-commit on the next user check-in if the verification floor is `scripts/fw verify` (L1 + tests + canonical-hash).
- **Reversibility.** ADR-0012 supersession via new ADR; protocol can be downgraded (e.g., Tier-2 disabled) by editing the `/duo-implement` skill's enabled flag, no `.mcp.json` or asmdef changes required.

---

## Decision

Define a Tier-2 autonomous implementation protocol with five binding components.

### Component 1 — Scope contract (task-spec event)

User issues a task by posting a `type: task-spec` event to a new agent-bus topic. The event MUST include:

- **`acceptance`** — falsifiable acceptance criteria. Examples: "L1 + L2 visual + ADR-0009 polish bar §motion-lines passes solo-eye-test", "all 644 MatchSim.Tests + 89 EditMode tests green", "pinned 60-tick `MatchCanonicalState` hash unchanged". Multiple criteria allowed; ALL must be met.
- **`files_in_scope`** — glob list of files the implementing agent is authorized to create / modify / delete. Globs MUST be specific. Example: `unity-project/Assets/Viewer/Adapters/Dots/IdentityCueRenderer.cs`, `unity-project/Assets/Viewer/Adapters/Dots/SelectionRing.cs`, `unity-project/Assets/Viewer/Adapters/Dots/MotionLineFeature.cs`, `unity-project/Assets/Viewer/Adapters/Dots/Shaders/MotionLines.shader`. Wildcard `**` MAY appear inside the existing `Adapters/Dots/` tree but MUST NOT escape it.
- **`files_out_of_scope`** — explicit denylist that overrides any wildcard match in `files_in_scope`. Defensive. Always includes: `design/**.md`, `SPEC.md`, `CLAUDE.md`, `TECH_APPROACH.md`, `TOOLING.md`, `PROJECT_CONTEXT.md`, `SETUP.md`, `.claude/agents/**`, `.claude/rules/**`, `.claude/hooks/**`, `MatchSim/Sim/Q3232.cs`, `MatchSim/Sim/Tick.cs`, `MatchSim/Sim/Seed.cs` (the canonical primitives — touching them requires a SPEC entry).
- **`max_tokens`** — token budget across both agents combined. Default `400000` (~$5-15 USD per task at current pricing depending on cache hit rate). Hard ceiling: agents stop posting when shared budget exhausted.
- **`max_wall_clock_seconds`** — total wall-clock cap. Default `7200` (2 hours). Hard ceiling: agents post `escalation` when exceeded.
- **`escalation_triggers`** — list of conditions that force escalation to user. Default set: any out-of-scope file touched, `fw verify` red >2 attempts, deadlock (3+ rounds of counter without convergence), canonical-hash drift not authorized, design-doc text touched, SPEC decisions-log mutation, `manifest.json` mutation, asset-generation tool invocation.
- **`required_subagents`** — optional list of subagents the implementing agent MUST consult per CLAUDE.md §6.3 mandatory rotation. The bus tracks which were invoked; missing required subagents become a pre-commit blocker.

### Component 2 — Reviewer-gate-before-commit

Implementing agent posts `type: commit-proposal` event with the diff + verification output (passing `fw verify`, pinned hash unchanged, all tests green). Reviewing agent reads the diff via filesystem, posts `type: counter` (block + reasons) OR `type: ack` (approve). **Commit lands only after reviewing-agent ack.** Differs from current `pr-review-toolkit` rhythm where review runs post-commit.

If reviewing agent does not respond within `2 * max_wall_clock_seconds / 4` (default 60 min for a 120-min task), implementing agent posts `escalation` — never auto-commits without ack.

### Component 3 — Auto-rollback on canonical-hash drift

New hook `.claude/hooks/canonical-hash-guard.sh` PreToolUse on `Bash(git commit*)`. If the staged diff includes any `MatchSim/**` change AND the resulting `MatchCanonicalState` pinned-hash test would fail, the commit is BLOCKED with exit 2. Implementing agent must either (a) explicitly authorize the hash change in the task spec (requires SPEC entry; auto-escalation), or (b) revert the offending change.

The hook re-runs `dotnet test --filter Match_SmokeFixture60TicksWithSignaturePackets_ProducesIdenticalPinnedHash` — narrow, fast (~2s), targeted. Not the full `fw verify`.

### Component 4 — User-escalation triggers

The agent-bus gains `type: escalation` events. An escalation is a hard stop: the agent posts and stops polling until user posts a `type: decision` event resolving it. Triggers:

- Out-of-scope file change attempted
- `fw verify` red after 2+ fix attempts
- Canonical-hash drift not authorized
- `design/**.md` modification proposed
- `SPEC.md` decisions-log modification proposed
- `manifest.json` modification proposed (paid-package risk)
- Any `Unity_AssetGeneration_*` tool invocation proposed (credit burn risk)
- Deadlock: 3+ rounds of `counter` without `ack` convergence
- Token budget 80% exhausted
- Wall-clock 80% exhausted

Escalations are explicit: agent posts `type: escalation, severity: p0`, names the trigger, names the proposed action, names the decision the user needs to make. User responds with `type: decision`. Topic stays open; agent resumes after decision.

### Component 5 — Hard cost + time caps

Implemented via env-var-driven enforcement in `scripts/agent-bus`:

- `AGENT_BUS_MAX_TOKENS` — sum of `body` bytes posted on this topic across all events; agent posts `escalation` when 80% reached, refuses post when 100% reached.
- `AGENT_BUS_MAX_WALL_CLOCK` — seconds since first event on topic; agent posts `escalation` when 80% reached, refuses post when 100% reached.
- `AGENT_BUS_MAX_TURNS` — max events per agent before mandatory user check-in; defaults to 50 per agent.

These are best-effort defenses, not bulletproof. The implementing agent's own `ScheduleWakeup` chain is the primary cost driver — caps cannot prevent every runaway, but they bound it and force escalation.

### Component 6 — Cascade-prevention when applying review-findings

When Codex reviews a task post-commit and posts a `counter` event with findings, Claude (via `/check-reviews`) may want to apply the fix. But if Claude has moved on to a downstream task that depends on the same files, naive auto-application produces cascading bugs:

1. Task A ships → Codex reviews → finds bug → Claude moves on.
2. Claude implements Task B; Task B's `files_in_scope` overlaps with Task A's affected files.
3. Codex's Task-A finding lands; Claude auto-applies the fix.
4. Task A's files now differ from what Task B was built against.
5. Task B is broken; verification fails; cascade.

The cascade-prevention rule: **before applying any review-fix that would mutate a file, run `scripts/agent-bus cascade-check --files <csv> --target-topic <orig-topic>`**. The CLI walks all in-flight task-spec topics (task-spec landed, no task-complete, no user-decision closure) and reports overlap between the candidate files and the in-flight `files_in_scope` lists.

Exit codes:
- **0** — no overlap; cascade-safe; apply the fix.
- **4** — overlap detected; defer to user triage. Post a `note` on the affected topic + an `escalation` event with the proposed-action alternatives ((a) abandon in-flight; (b) apply after in-flight lands; (c) merge into combined task-spec).
- **2** — argument error.

The check is **coarse and defensive by design**: false-positive (over-counting overlap) is safe (just triggers a user check-in); false-negative (missing real overlap) ships the cascade bug. Match algorithm: lowercase-substring match of each candidate file path against the task-spec body. Glob-aware matching is a future precision improvement (P3 follow-up; tracked but not blocking the protocol).

Dependency-tracking hint: task-spec body MAY include a `depends_on: [other-topic-id, ...]` field per agent-bus-spec §15. The implementing agent uses this for declarative dependency reasoning beyond the file-overlap check. Cascade-prevention is the **mechanical floor**; `depends_on` is the **declarative semantics layer**.

## Consequences

### Positive

- User burden on bounded coding tasks drops materially. The relay user-time per task: minutes-of-paste-relay → seconds-of-task-spec-then-walk-away.
- Reviewer-gate-before-commit shifts review left, catching issues before they need a follow-up commit. Faster overall.
- Auto-rollback on canonical-hash drift turns a class of determinism bugs into compile-time-equivalent errors instead of "commit, push, Codex catches in review, rollback later."
- Escalation triggers preserve user authority over creative + scope + money decisions while allowing agents to drive the grunt work.
- Token + time caps create predictable cost ceilings per task. User can budget by phase.
- The protocol is layered (Tier 1 / Tier 2 / Tier 3-out-of-scope). Tier 1 already proven; Tier 2 ships next; Tier 3 is explicitly the wrong target for a creative project.

### Negative

- **Creative-judgment leakage risk.** Even with escalation triggers, an implementing agent might make small tone/aesthetic choices that should have been escalated. Mitigation: triggers are deliberately broad (any `design/**.md` touch, not just structural ones); user reviews the diff at next check-in regardless.
- **Cap=1 sequencing risk.** Reviewing agent cannot independently verify Unity-side L2 visual evidence without taking the slot. Mitigation: implementing agent attaches L2 captures to `commit-proposal` event; reviewer trusts (or asks for re-capture). Visual regression catches happen on user check-in.
- **Pre-release Unity package risk** (inherited from ADR-0011): tool-shape changes between pre.N versions could break the protocol. Mitigation: pinned package version + ADR-0011's tracked-SPEC-task discipline on upgrades.
- **Cost surprise risk.** A user who sets `max_tokens: 400000` and walks away might expect $5 spent and find $15 if the cache hit rate is poor. Mitigation: caps are clearly named in the task spec; user can lower defaults; first 5 task-spec invocations should be supervised to calibrate.
- **Quality regression risk.** Without the user's eye-on-diff during implementation, subtle bugs can land. Mitigation: reviewer gate + `fw verify` + pinned hash + post-merge user review-of-diff.

### Reversibility

Two-way door at multiple levels:

- **Skill level**: `/duo-implement` not invoked = no autonomous tier-2 activity. Manual rhythm continues uninterrupted.
- **Per-task level**: user can post `type: decision, body: "stop"` to any open topic; implementing agent halts within one poll cycle.
- **Protocol level**: ADR-0012 supersession via new ADR. Skill can be removed; hooks can be disabled; agent-bus extensions stay (used by Tier 1) but Tier-2 events become inert.
- **Commit level**: every commit lands on a feature branch by default (TBD: implementation detail of `/duo-implement`); user reviews on check-in; one-line `git revert` undoes.

## Alternatives considered

**A) Skip Tier 2 entirely; stay on manual rhythm.** Lowest-disruption. Forfeits the user-time savings on grunt work, which the user has explicitly named as the goal. **Rejected.**

**B) Build "agent builds the game" autonomy directly.** What the user originally framed as "the dream." Forfeits the creative authority guardrail. The user themselves walked back from this framing in the same message ("not a one command builds game solution i want"). **Rejected.**

**C) Make Tier 2 fully autonomous including push to main.** Removes the user check-in. Bad: user-as-creative-director loses the diff-review rhythm; trust is one-shot rather than continuous. **Rejected.**

**D) Implement Tier 2 without escalation triggers** (assume agents always know when to stop). Bad: agents demonstrably drift on out-of-scope work without explicit guards. The mandatory-rotation table in CLAUDE.md §6.3 was added 2026-04-30 specifically because agents had been drifting. **Rejected.**

**E) Use a different cross-agent transport** (Slack / Discord / shared git branch with PR comments). The agent-bus is in-repo + version-controlled + grep-able; alternatives lose the audit-trail discipline that's the whole point. **Rejected.**

## Acceptance criteria

- [x] ADR text drafted (this document).
- [x] Agent-bus spec extended with 3 new event types (`task-spec`, `escalation`, `task-complete`) — spec change lands in same commit.
- [x] `scripts/agent-bus` extended with `task-spec` + `archive` + `stats` subcommands.
- [x] `.claude/hooks/protect-dialog-append-only.sh` ships (auto-promoted to P1 per `2026-05-09-mcp-migration-debate` mutual-fade closure).
- [x] `.claude/hooks/canonical-hash-guard.sh` ships.
- [x] CLAUDE.md §6.3 gains autonomous-implementation discipline subsection.
- [x] TECH_APPROACH.md §13 documents the protocol.
- [x] `/duo-debate` skill ships (Tier 1 wrapper, ready-to-use today).
- [x] **`/duo-implement` skill ships** (2026-05-11; Claude-side Tier-2 orchestrator with ScheduleWakeup polling + commit-proposal flow + reviewer-gate-before-commit + escalation-trigger discipline).
- [x] **`/codex-review-loop` skill ships** (2026-05-11; Codex-CLI-side continuous-polling continuous-review mode).
- [x] **`/check-reviews` skill ships** (2026-05-11; Claude-side review-inbox with cascade-prevention via `scripts/agent-bus cascade-check`).
- [x] **`scripts/agent-bus pending-reviews`** subcommand ships — lists open topics with unreplied commit-proposals; exit 5 when found, exit 0 when none. Detection via sha256-of-commit-proposal vs `in_reply_to` of subsequent ack/counter events.
- [x] **`scripts/agent-bus cascade-check`** subcommand ships — overlap-detection for Component 6 cascade-prevention; exit 4 on overlap, exit 0 cascade-safe.
- [x] **Component 6 cascade-prevention** designed + scaffolded (this ADR amendment + agent-bus-spec §16 + `/check-reviews` skill).
- [ ] **Codex review** of this ADR via agent-bus topic `2026-05-10-adr-0012-autonomous-tier-2-review`. Required for Accepted promotion. The 5 opening events posted 2026-05-09 are unchanged; Codex's review of the amended ADR (Component 6 added) is part of this review pass.
- [ ] **Slice 7 dogfood** — first real Tier-2 task using `/duo-implement` end-to-end. Required for Accepted promotion.
- [ ] **User sign-off** on ADR + Slice 7 dogfood outcome. Required for Accepted promotion.

## Review trail

- **2026-05-09 (drafted, autonomous mode):** Authored by Claude in autonomous mode per user instruction "Make improvements so it works with you guys auto working through tasks, coding together etc." User explicitly scoped the goal: bounded grunt work where agents code + review together; user retains authority on creative + scope + money. **No Codex round-trip yet** — agent-bus topic `2026-05-10-adr-0012-autonomous-tier-2-review` opened in the same commit for async review.
- **Next session expected:** Codex review pass on ADR-0012 + agent-bus extensions. Cross-model rhythm per CLAUDE.md §6.3 expected to flag any blind spots in the escalation triggers, scope contract, or reviewer-gate sequencing.
- **First Tier-2 dogfood expected:** Slice 7 of dots-adapter ladder (`docs/plans/dots-adapter-blueprint.md §B Slice 7` — identity cues + selection ring + motion lines + observer-rubric pass). Bounded scope, well-defined acceptance, no money/asset-gen risk, no design-doc edits required. Right shape for the first dogfood.

## References

- [docs/tooling/agent-bus-spec.md](../../docs/tooling/agent-bus-spec.md) — protocol foundation; extended in this ADR's commit
- [scripts/agent-bus](../../scripts/agent-bus) — CLI shim
- [ADR-0011](adr-0011-unity-ai-assistant-mcp-migration.md) — Editor MCP routing; `cap=1` constraint informs Tier-2 sequencing
- [CLAUDE.md §6.3](../../CLAUDE.md) — subagent rotation; new autonomous-implementation discipline subsection lands in this commit
- [scripts/fw](../../scripts/fw) — verification floor; `fw verify` is the pre-commit gate
- `dialog/2026-05-09-mcp-migration-debate.jsonl` — Tier-1 dogfood transcript; first proof the bus works end-to-end
- `dialog/2026-05-10-adr-0012-autonomous-tier-2-review.jsonl` — opened in this commit for Codex's async review

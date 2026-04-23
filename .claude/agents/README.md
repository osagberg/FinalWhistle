# .claude/agents/ — project-authored subagents

This folder ships with the blueprint's **14-agent studio roster** pre-installed. Every blueprint project boots with these available for invocation. The roster is tuned for solo Unity dev targeting Steam; in a solo context, each agent is YOU wearing a specific hat for a specific decision.

## Roster

**Tier 1 — Directors (3, opus-capable, opt-in to heavier model per session):**
- `creative-director` — vision guardian; pillars, tone, scope-cut arbitration
- `technical-director` — architecture + Unity package + performance-budget authority
- `producer` — cross-discipline coordinator + phase-gate enforcement

**Tier 2 — Leads (5, sonnet default):**
- `game-designer` — mechanics, systems, GDD authoring (MDA / SDT / Flow / Bartle)
- `lead-programmer` — team-level code architecture + review (SOLID)
- `art-director` — visual identity, art bible, asset standards
- `narrative-director` — story architecture, world-rules, voice bibles
- `qa-lead` — test strategy, acceptance criteria, phase-gate quality

**Tier 3 — Specialists (6, sonnet, tactical invocation):**
- `gameplay-programmer` — moment-to-moment feel, combat, movement
- `engine-programmer` — performance, memory, hot-path, Addressables
- `systems-designer` — formulas, curves, economy, interaction matrices
- `ui-programmer` — UGUI/UI Toolkit implementation + accessibility
- `unity-specialist` — deep Unity API knowledge + engine quirks
- `unity-ui-specialist` — UI Toolkit (UXML/USS) + UGUI mastery

## Invocation patterns

- Slash commands automatically pick the right agent (e.g., `/brainstorm` → creative-director; `/design-system` → game-designer; `/code-review` → lead-programmer).
- Claude can also invoke any agent via `Task(subagent_type: "agent-name", prompt: "...")`.
- Directors operate on the **Question → Options → User Decision → Draft → Approval** collaboration cycle; they never make binding calls autonomously.

## Escalation map (quick reference)

- game-designer ⇄ narrative-director conflict → creative-director
- art-director ⇄ audio-director (when spawned) tonal conflict → creative-director
- lead-programmer ⇄ art-director tech/visual trade-off → technical-director
- any agent scope concern → producer
- specialists → their lead; leads → their director; directors → the user

## Expanding the roster

More specialists (audio-director, ai-programmer, technical-artist, ux-designer, level-designer, writer, world-builder, accessibility-specialist, devops-engineer, localization-lead, unity-dots-specialist, unity-shader-specialist, unity-addressables-specialist, etc.) are available via `/expand-studio` when a project needs them. Don't add them prematurely — the 14-agent core handles most solo Unity workflows.

## Authoring a custom agent

When a project needs a voice-specific NPC writer, a specialized reviewer, or a multi-agent coordinator, add a new Markdown file here. See `plugin-dev:agent-development` skill for the frontmatter spec. Minimal shape:

```markdown
---
name: my-agent
description: <When to invoke — 3-5 lines with specific triggers>
tools: [All tools]         # or subset: [Read, Write, Grep]
color: "#rrggbb"           # optional
---

## Role
<one paragraph purpose>

## Voice + style
<how this agent talks>

## When to invoke / Don't invoke when
<specific scenarios + anti-patterns>

## Core knowledge
<domain frameworks this agent cites>

## Collaboration protocol
<Question → Options → Approval cycle>

## Blueprint integration
<slash commands, files read, escalation paths>

## DO / DON'T
<concrete behavior checklist>
```

## Don't author a subagent when

- One-off research — use `general-purpose` or `Explore` instead
- Code review — already covered by `lead-programmer` + `feature-dev:code-reviewer` plugin
- Codebase exploration — `Explore` already does this

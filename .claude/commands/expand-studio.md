---
description: Opt this project into the full studio scope — load additional agents, sprint management patterns, and extended templates
argument-hint: "[permanent|session-only]"
---

# /expand-studio — scale up to studio scope

Elevates the current project's context scope from `rich` (default) to `studio`. Adds access to extended agent roster (audio-director, ai-programmer, technical-artist, level-designer, writer, world-builder, accessibility-specialist, devops-engineer, unity-shader-specialist, unity-dots-specialist, unity-addressables-specialist, localization-lead), proactive loading of `patterns/phase-gate-workflow.md` + `patterns/budget-tier-trigger-table.md`, plus all 14 design-templates as references.

## When to invoke

| Signal | Why studio makes sense |
|---|---|
| Cross-ref phase is Phase 6+ (Content Scaling / Launch) | More specialist decisions need distinct-agent voices |
| Team size grew (solo → small team) | Each team member wears a different specialist hat |
| Scope upgrade via `/log-decision` | "We decided to add multiplayer" / "added audio lead" / "localization pass starting" |
| Struggling to orient across many open fronts | Studio's sprint management (`/create-epics`, `/create-stories`, `/story-readiness`, `/dev-story`, `/story-done`, `/milestone-review`) scales better than raw `/next` |
| On 1M-context tier with budget for ~300KB proactive-load | Cost of scope upgrade is manageable |

## When NOT to invoke

| Signal | Why studio is premature |
|---|---|
| Project is Phase 0-3 | Rich scope covers setup + bootstrap + first character; extending now is overhead |
| Solo + tight scope game | 12-agent core already covers all decisions a solo dev needs to make |
| 200K-context tier | Studio's proactive-load (~300KB) eats too much working memory |
| "I'll just browse the extended agents" | That's not scope elevation — just invoke the agent via `Task` directly |

## Procedure

1. Read `.claude/context-scopes.json` to confirm current scope (default: `rich`)
2. Confirm with user: "Scope elevation from `<current>` → `studio` will add ~150KB of proactive-load at next session start + make these new agents Task-invocable: <list>. Proceed?" (unless argument `session-only` supplied — in which case proceed without confirmation but don't persist)
3. If confirmed OR if `permanent` argument supplied:
   - Write `studio` to `.claude/.current-scope` file (creates if missing)
   - Update `STATUS.md` next session context budget note if one exists
4. If `session-only`: don't write `.current-scope`; instead emit a NOTE for this session only ("Scope elevated in-session to studio. Next session will default back to rich.")
5. Announce to user: list the newly-available agents, slash commands, and patterns now in scope
6. Recommend follow-up if applicable: "Consider `/log-decision scope: expanded to studio at <phase> — <reason>`"

## If already at studio scope

Report current state and suggest `/deep-research` if the user wants even more breadth (loads reference library).

## If at research scope

Studio is lower than research. If user runs `/expand-studio` while at research, ask: "You're currently at `research` scope. Did you mean `/contract-scope studio`?"

## Related

- `/deep-research` — load reference library for heavy research phases (higher than studio)
- `/contract-scope <target>` — scale back down if studio turns out to be too much
- `.claude/context-scopes.json` — the full scope declaration
- `docs/context-scope-architecture.md` — user-facing explainer
- `patterns/phase-gate-workflow.md` — tactical phase + sprint discipline
- Reads files: `.claude/context-scopes.json`, `.claude/.current-scope`, `STATUS.md`, `SPEC.md`
- Writes files: `.claude/.current-scope`

## Reversibility

Fully reversible via `/contract-scope rich` (or `standard` / `minimal`). No files are destroyed; only the proactive-load manifest changes.

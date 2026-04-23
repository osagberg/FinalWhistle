---
description: Open-ended ideation with Creative Director — genre exploration, pillars, elevator pitch
argument-hint: "[genre or theme hint, or 'open']"
---

# /brainstorm — guided concept ideation

Collaborative game-concept exploration. From zero idea (or a vague hint) to a structured concept doc. Facilitation over replacement — the author drives; Claude helps shape.

**Phase:** 1 (Concept). Gates: produces `design/game-concept.md` + `design/pillars.md`.

## Procedure

1. **Parse args.** If `open` or no arg, start from scratch. Otherwise treat arg as a genre/theme hint.
2. **Check resume state.** Read `design/game-concept.md` + `design/pillars.md` if either exists. If present: ask "resume or restart?" via `AskUserQuestion`. Resume preserves prior work.
3. **Spawn Creative Director subagent.** Invoke via `Agent` tool:
   - If `.claude/agents/creative-director.md` exists → spawn that agent
   - Else → spawn `general-purpose` agent with the Creative Director persona baked into the prompt
   - Context block: project name + platform + scope (from `PROJECT_CONTEXT.md`), any hint arg, any existing concept work
4. **Run the 4 ideation phases** interactively (ask first, generate after answers):
   - **Phase 1 — Discovery:** emotional anchors, taste profile, design pillars, constraints
   - **Phase 2 — Divergence:** generate 5-8 concept directions using MDA + player-psychology frames
   - **Phase 3 — Convergence:** user picks 2-3 finalists via `AskUserQuestion`; develop each into a pitch
   - **Phase 4 — Lock:** user picks one; author `design/game-concept.md` (pitch, pillars, core loop, target audience, visual anchor)
5. **Write outputs** after user approval: `design/game-concept.md`, `design/pillars.md`
6. **Log decision** via `/log-decision` for the locked concept
7. **Recommend next step:** `/map-systems` (decompose into systems) or `/art-bible` (visual identity)

## If args provided

- `open` → full exploration mode, no hint
- `<hint>` (e.g., `roguelike`, `cozy farming`) → use as starting constraint in Phase 1

## If concept already exists and user wants to pivot

Don't overwrite silently. Ask user to confirm the pivot, then invoke `/log-decision` to record it with reasoning, then run brainstorm fresh.

## Output

- `design/game-concept.md` (pitch, core loop, pillars, audience, visual anchor)
- `design/pillars.md` (ranked design pillars with rationale)
- Decisions log entry in `SPEC.md`

## Related

- Typical follow-ups: `/map-systems`, `/art-bible`, `/design-system`
- Invokes agents: `creative-director` (or `general-purpose` with persona)
- Invokes skills: `/log-decision`
- Reads files: `PROJECT_CONTEXT.md`, `design/game-concept.md` (if resuming)
- Writes files: `design/game-concept.md`, `design/pillars.md`

---
description: List all slash commands + typical workflows by phase
argument-hint: "[phase-number]"
---

# /help — command catalog + workflows

Comprehensive reference for every slash command in the blueprint. Use when stuck or when you want to discover what's available.

## Procedure

1. If `$ARGUMENTS` is a phase number (0-7), scope output to that phase. Otherwise show full catalog.
2. Read `SPEC.md` "Current state" to determine current phase — star commands relevant to that phase.
3. Output the catalog (see format below).
4. If the user's prompt suggested confusion ("I'm stuck", "lost", "don't know"), add a short "Need more detail?" block at the end recommending `/status` or `/start`.

## Output format

```
# Slash commands

## Project state (always useful)
- /status — fast snapshot: phase, progress, blockers, next action
- /next — pick up next unblocked task from active phase
- /done — close current task; update SPEC / STATUS / CHANGELOG
- /audit — run all project validators
- /refresh-docs — audit docs for staleness + contradictions
- /log-decision — append to SPEC.md decisions log

## Onboarding
- /bootstrap — one-shot project setup (run once, then delete)
- /start — deeper project orientation after /bootstrap
- /help — this command

## Design (Phase 1-2)
- /brainstorm — open-ended ideation with Creative Director
- /design-system <name> — section-by-section GDD authoring
- /map-systems — decompose concept into systems + dependency graph
- /quick-design <desc> — lightweight spec for small tweaks
- /review-all-gdds — cross-GDD consistency + design-theory review
- /art-bible — visual identity spec (gates asset production)

## Architecture (Phase 3)
- /create-architecture — master architecture doc from GDDs
- /architecture-decision <title> — author a new ADR
- /architecture-review — validate arch vs GDDs (PASS/CONCERNS/FAIL)

## Sprint / stories (Phase 4-5)
- /create-epics <layer|all> — break phase into epics
- /create-stories <epic> — break epic into implementable stories
- /story-readiness <path> — verify story is implementation-ready
- /dev-story <path> — scaffold implementation of one story
- /story-done <path> — verify AC + close the story

## Review + QA
- /code-review [path] — architectural + quality review
- /design-review <path> — validate GDD before handing to dev
- /balance-check <system> — stat/formula sanity check
- /gate-check [phase] — validate phase-gate conditions
- /smoke-check — quick L1+L2 build verification
- /regression-suite [update|audit|report] — regression coverage

## Production + release (Phase 6-7)
- /milestone-review — end-of-phase comprehensive review
- /hotfix <bug-id> — scaffold emergency fix with audit trail
- /release-checklist [platform] — pre-launch validation
```

## Typical workflow chains

- **Starting from zero**: `/bootstrap` → `/start` → `/brainstorm` → `/map-systems` → `/design-system` → `/review-all-gdds`
- **Phase-3 architecture**: `/create-architecture` → `/architecture-decision` (×N) → `/architecture-review` → `/gate-check`
- **Implementation loop**: `/create-stories` → `/story-readiness` → `/dev-story` → `/code-review` → `/story-done`
- **Phase close**: `/smoke-check` → `/regression-suite` → `/milestone-review` → `/gate-check`
- **Launch**: `/release-checklist` → `/gate-check release`

## Related

- Reads files: `SPEC.md` (current phase detection)
- Writes files: none

# Postmortem — Phase T{{N}}: {{name}}

**Author:** {{producer agent}}
**Date:** {{YYYY-MM-DD (date phase closed)}}
**Phase duration:** {{first commit date}} → {{last commit date}}

---

## What shipped

One bullet per shipped MASTER_PLAN row (cross-reference CHANGELOG):
- {{T<N>-<n>: title — commit <sha>}}
- {{...}}

## What went well

- {{observation 1}} — why it worked
- {{observation 2}} — why it worked

## What went poorly

- {{observation 1}} — root cause guess
- {{observation 2}} — root cause guess

## What we learned

Not "what we'll do differently" — what we *understand* now that we didn't at phase start. Insights that should outlive this phase.

- {{insight 1}}
- {{insight 2}}

## Action items for next phase

- {{change to workflow / process / tool}}
- {{ADR to author}}
- {{risk to track in MASTER_PLAN Risks section}}
- {{thing to delegate to a different agent next time}}

## Stats

- Commits: {{N}}
- LoC delta: +{{added}} / -{{removed}}
- Tests added: {{N}} (insta + proptest + integration)
- Decisions logged: {{N}}
- Canonical-hash repins: {{N (with reasons)}}
- Phase gate verdict: {{PASS / PASS-WITH-FOLLOWUPS / DEFERRED}}

## Cross-references

- `CHANGELOG.md` — Phase {{N}} entry
- Phase PR (Codex review): {{URL}}
- `docs/DECISIONS.md` — entries added during this phase

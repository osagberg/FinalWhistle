---
description: Project state in under 150 words — current phase, active task, blockers, next.
---

Read in order:

1. `/Users/vibelogic/dev/football/STATUS.md` — state pointer
2. `/Users/vibelogic/dev/football/MEMORY.md` — working context for active task
3. `git log --oneline -5` — last 5 commits
4. `/Users/vibelogic/dev/football/docs/MASTER_PLAN.md` — active phase header + count of DONE / TODO / BLOCKED rows

Output under 150 words, in this shape:

```
**Phase:** <name> (<n DONE> / <n TODO> / <n BLOCKED>)
**Active task:** <STATUS.md active task, or "(none)">
**Last verify:** <STATUS green-verify timestamp>
**Canonical hash:** <STATUS last hash>
**Blocked:** <list, or "none">
**Recent commits:**
  - <hash> <subject>
  - <hash> <subject>
  - <hash> <subject>
**Next up:** <first TODO row in current phase whose deps are DONE>
```

No prose, no fluff. Just the table. If STATUS.md timestamp is >24h stale, prepend a one-line warning: `STATUS.md stale (<n>h) — consider running /next or /audit.`

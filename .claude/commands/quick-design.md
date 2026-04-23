---
description: Rapid 10-minute design pass for small features, tuning changes, tweaks
argument-hint: "<brief description of the change>"
---

# /quick-design — lightweight design spec

For changes too small for a full GDD but too meaningful to implement without a written rationale. Produces a Quick Design Spec that embeds directly into a story file or sits alongside a GDD as a tuning addendum.

**Phase:** 2-6 (any phase once GDDs exist). Output: `design/quick-specs/<name>-<date>.md`.

## Procedure

1. **Parse arg** (brief description). If missing, ask the user to describe in one sentence.
2. **Classify the change** — ask via `AskUserQuestion` if not obvious:
   - **Tuning** — pure numeric change, no behavior change (e.g., "jump height 5 → 6")
   - **Tweak** — small behavioral change, no new states (e.g., "dash is invincible on frame 1")
   - **Addition** — new state/interaction added to existing system (1-2 new branches)
   - **New Small System** — standalone feature <1 week of work, no existing GDD
3. **Decide document path:**
   - Tuning / Tweak → append to existing GDD's "Tuning Knobs" or "Revisions" section
   - Addition / New Small System → new file in `design/quick-specs/`
4. **Author sections** (abbreviated vs full GDD):
   - Context (what prompted this)
   - Change (precise before/after)
   - Rationale (why — pillar / playtest / bug)
   - Impact (systems affected, migration risk)
   - Acceptance Criteria (testable)
5. **Write** file after user approval
6. **If phase allows**, flag whether this should also go through `/log-decision` (e.g., if it supersedes a prior GDD choice)

## If args provided

Treat all args as the description. Classification happens interactively.

## If change is actually big

If the scoping reveals >1 week of work or multiple new systems, stop and recommend `/design-system` (full GDD) instead. Don't force a big change into the quick-spec template.

## Output

- `design/quick-specs/<name>-<date>.md` OR appended section to existing GDD
- Console summary: classification + path

## Related

- Typical follow-ups: `/dev-story` (if ready to implement), `/log-decision` (if it changes prior design)
- Invokes agents: `game-designer` (lightweight — single-shot, not full walkthrough)
- Invokes skills: `/log-decision` (conditional)
- Reads files: existing GDD if the change targets one
- Writes files: `design/quick-specs/<name>-<date>.md` OR edit to existing GDD

---
paths:
  - "design/**/*.md"
  - "design/**/*.yaml"
  - "docs/architecture/**/*.md"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Design docs — authoring discipline

Design docs are the contract. Code implements them. If a doc is wrong, the game is wrong.

## MUST

- Every design doc opens with YAML frontmatter containing at least `description`. No frontmatter = no doc.
- Every cross-reference resolves to an existing file. Broken links are a hard fail — fix on sight.
- Before adding a new system to any design doc, confirm an ADR exists or is being written in the same turn. Systems without ADRs accumulate architectural debt.
- Use one of the templates in [`design-templates/`](../../../design-templates/) as the skeleton. Start from a FinalWhistle-substituted copy, never an empty file.
- Formulas stay formulas. `salience = 0.4 * stakes + 0.2 * prominence + ...` not "big events surface more often". Every variable typed, ranged, sourced.
- Content safety floor (age gate, consent, no real people) respected in every content-generating doc — cite `design/content_philosophy.md` if it exists in the project.

## SHOULD

- Single source of truth per fact. If a stat / rule / name appears in two docs, one references the other; both never duplicate.
- "Last Verified" date updated when the doc is re-read and confirmed accurate — even if nothing changed. Stale dates signal "no one has checked this recently".
- Acceptance criteria are falsifiable. A QA tester should be able to verify each one without asking the author.
- Prefer tables over prose for structured data (states, parameters, AC lists). Easier to scan, diff, and machine-read.
- One system per GDD — don't pack two mechanics into one file unless they truly can't be separated.

## AVOID

- Prose descriptions of math. "The multiplier gets bigger as you level up" is a bug breeding ground. Write the formula.
- Editing an ADR after it's marked Accepted. Create a superseding ADR instead (append-only, per blueprint doctrine).
- Duplicating content between docs. If signature readiness rules appear in both `signatures.md` and `player-generation.md`, one copy is wrong and you don't know which.
- Inferring design details at implementation time. If the GDD doesn't specify edge-case behavior, update the GDD before writing code.
- Uncited comparables. "Like Hades" is not a design spec; "Like Hades' boon-rerolling where each floor offers 3 options at 60% / 30% / 10% rarity" is.

## RATIONALE

Design docs are the long-lived assets of a game project — they outlive tools, engines, even individual team members. Frontmatter makes them greppable. Cross-link discipline makes them navigable. Template reuse makes them comparable across systems. The single-source-of-truth rule is how you avoid the classic "the GDD says X, the wiki says Y, the code does Z" failure mode — by making it structurally impossible to disagree.

## References

- [templates/design-templates/](../../../design-templates/) — the 14 canonical doc skeletons
- [tests/RULES.md](../tests/RULES.md) — sibling: how tests verify GDD formulas
- [CSharp/RULES.md](../CSharp/RULES.md) — implementation-side counterpart

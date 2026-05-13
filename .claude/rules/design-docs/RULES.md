---
description: Design-doc writing conventions for docs/. Append-only DECISIONS, coefficient-free SPEC, MASTER_PLAN cadence.
applies_to:
  - docs/**
auto_load: when_editing_matching_path
---

# Design-doc rules

## §1. DESIGN_DOC.md is the stable contract

- `docs/DESIGN_DOC.md` is the SoT for pillars, scope, and ruled-out items.
- Edits go through `/log-decision` first — the decision lands in `docs/DECISIONS.md`, THEN DESIGN_DOC is updated to reflect it.
- DESIGN_DOC is not casual editing surface.

## §2. DECISIONS.md is append-only

- Format: `- **YYYY-MM-DD** — <topic>: <decision>. Context: <why>. Supersedes: <prior bullet date + topic, or "none">.`
- One line per entry; wrap fine, no internal bullets.
- The PreToolUse hook at `.claude/hooks/protect-decisions.sh` rejects any edit that mutates a line matching `^- \*\*\d{4}-`.
- To change a prior decision: append a new entry that supersedes it, citing the prior bullet verbatim. Do NOT edit the old one.

## §3. MASTER_PLAN.md is the delivery SoT

- One row per task. Format:
  ```
  - **T<phase>-<n>** — <title>. Status: TODO|IN-PROGRESS|DONE|BLOCKED. Deps: T<a>-<b>, T<c>-<d>.
  ```
- Status updates happen via `/next` (TODO → IN-PROGRESS → DONE) or `/done` (phase boundary).
- Phase-level acceptance gates live at the top of each phase section.
- Deferred items go to a `## Deferred` section near the bottom — never silently lost.

## §4. Tuning coefficients stay out of SPEC

- Numeric values (signature thresholds, salience weights, dev curves, scout error bands) live in **design docs** (`docs/design/*.md`), NOT in SPEC.md and NOT in DECISIONS.md.
- Per user memory: "SPEC locks structure; numeric seeds live in design docs as Phase-N tuning values."
- Rationale: coefficients churn during balancing; SPEC stays stable.

## §5. ADRs

- Architecture Decision Records in `docs/adr/NNNN-<slug>.md`. 4-digit zero-padded.
- Template: `templates/design-templates/architecture-decision-record.md`.
- Status field: `Proposed | Accepted | Superseded`.
- A superseded ADR is **kept**, its Status field updated, with a link to the new ADR.

## §6. STATUS.md is a state pointer

- NOT a diary. State pointer only.
- Rewritten on `/done` to reflect the next phase.
- Under 150 words.
- Stop hook auto-stamps the timestamp.

## §7. CHANGELOG.md is append-only ship log

- Reverse-chronological inside each phase section.
- One line per shipped task or commit cluster.
- Phase-summary block on phase close (per `/done`).

## §8. REFERENCES.md catalogues archive provenance

- Tracks what's in the `/Users/vibelogic/dev/football-archive/` sibling (FW v1 Unity state).
- Cross-walks each new Rust crate → FW v1 source file(s) for design intent (NOT code copy).
- Do not delete REFERENCES.md — it's load-bearing for `/next`'s ability to consult prior art.

## §9. Vocabulary

- Football-native. No D&D / Final Fantasy / generic-RPG vocabulary.
- No emojis in design docs unless explicitly requested.
- No marketing-speak ("revolutionary", "next-gen", "robust"). Plain technical English.

## §10. File naming

- kebab-case for `.md` files.
- ADRs: `NNNN-<short-slug>.md` (4-digit prefix for chrono order).
- Specs: `docs/specs/<system>-<aspect>.md`.

## Cross-references

- `CLAUDE.md` §2 (source-of-truth map), §4 (workflow contract)
- `MEMORY.md`: append rules from feedback memory
- `/log-decision` slash command — append-only enforcement

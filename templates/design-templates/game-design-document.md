# {{system-name}} — Design Document

**Author:** {{name}}
**Date:** {{YYYY-MM-DD}}
**Status:** Draft | Approved | Shipped | Superseded
**Owning crate:** {{e.g., fw-match-sim, fw-memory, fw-content}}

---

## Overview

What this system is, in 2-3 sentences.

## Player experience goal

What does the player feel when this works? See the concept doc (link if applicable) for the upstream intent.

## System boundary

**In scope:**
- {{thing 1}}
- {{thing 2}}

**Out of scope (explicit non-goals):**
- {{thing 1}} — handled by {{other system / not at all}}
- {{thing 2}}

## Mechanics

Formula-by-formula. Each mechanic has:

### Mechanic: {{name}}

- **Inputs:** {{...}}
- **Formula:** {{expression in Q32-safe math}}
- **Outputs:** {{...}}
- **Worked example:** Input X → Output Y (cite at least 3 representative cases)
- **Tuning coefficients:** {{values}} (note: these live HERE in the design doc, NOT in `docs/DECISIONS.md` per `MEMORY.md`)

## Interaction with other systems

| This system | Other system | Direction | What flows |
|---|---|---|---|
| {{this}} | {{other}} | read / write / both | {{data shape}} |

## Determinism considerations

- **Which crate:** {{e.g., fw-match-sim}}
- **Q32-safe?** {{yes/no — if no, why is it allowed outside sim crates?}}
- **BTreeMap iteration order matters?** {{yes/no — if yes, document the order semantic}}
- **ChaCha8Rng seeding scheme:** {{how is `(match_seed, tick, event_id)` constructed for this system?}}
- **Banned-terms lint relevance:** {{does this system surface player-facing text? if yes, what vocabulary discipline applies}}

## Failure modes

What goes wrong if input is malformed, exhausted, or unexpected? Each failure mode → its handling (typed error, default value, panic, etc.).

## Telemetry / acceptance tests

- **Insta snapshot test:** {{name}} at `crates/{{crate}}/tests/{{file}}.rs`
- **Proptest invariant:** {{name}} at `crates/{{crate}}/tests/{{file}}.rs`
- **Pinned canonical hash impact:** {{will this change the pinned hash? if yes, what's the re-pinning plan?}}
- **Acceptance criteria (observable):**
  1. {{criterion 1}}
  2. {{criterion 2}}
  3. {{criterion 3}}

## Open questions

- {{question 1}}
- {{question 2}}

## References

- `docs/DESIGN_DOC.md` §{{section}}
- Concept doc: {{link if applicable}}
- Related ADRs: {{links}}
- FW v1 reference (design intent only — NO code copy): {{path in football-archive}}

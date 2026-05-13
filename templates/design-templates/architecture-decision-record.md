# ADR-{{NNNN}} — {{slug}}

**Status:** Proposed | Accepted | Superseded

**Date:** {{YYYY-MM-DD}}

**Decider:** {{name}} (+ Claude / Codex review where applicable)

---

## Context

What problem are we solving? What constraints apply? Why now?

Include relevant pillar references (`docs/DESIGN_DOC.md` §3 pillars 1-5) and any prior ADRs this builds on.

## Decision

The call, in one paragraph. Declarative, present tense: "We will X."

## Consequences

Positive, negative, neutral. What does this enable? What does it foreclose? What's the rollback path?

- **Positive:** {{...}}
- **Negative:** {{...}}
- **Neutral:** {{...}}

## Alternatives considered

For each: what it was, why we rejected it.

- **Alternative A:** {{description}} — rejected because {{reason}}.
- **Alternative B:** {{description}} — rejected because {{reason}}.

## References

- `docs/DESIGN_DOC.md` §{{section}}
- `docs/DECISIONS.md` bullet from {{YYYY-MM-DD}}
- Prior ADRs: {{links}}

---

## Worked-example bullet topics (delete this section in real ADRs)

Real Final Whistle ADRs in the queue or already authored:
- ADR-0001 — Q32.32 vs f64 in canonical state (Q32 won; cordic for trig)
- ADR-0002 — RON vs JSON for content sources (RON for human-diffability)
- ADR-0003 — Bincode 2 vs serde-cbor for saves (Bincode 2 for size + speed)
- ADR-0004 — BTreeMap-only in sim crates (HashMap iteration randomness breaks canonical hash)
- ADR-0005 — BLAKE3 vs SHA-256 for canonical-state hashing (BLAKE3 for speed; collision-resistance equivalent for our threat model)

## When to author an ADR

- A new crate / module boundary
- A pinned dep choice with non-obvious alternatives
- A determinism choice (collection type, RNG seeding scheme, hash algorithm)
- A workflow choice that affects multiple sessions (e.g., the /next dispatcher rules)
- A scope choice that closes future doors

## When NOT to author an ADR

- Routine implementation choices within a settled architecture
- Numeric tuning values (those live in design docs, not ADRs — per `MEMORY.md`)
- Reversible micro-decisions

# Game pillars — {{game-name}}

The 5 pillars that every feature must serve. Final Whistle's pillars are LOCKED at the top of `docs/DESIGN_DOC.md` §3 — this template is for future pivots or pillar reaffirmation, not casual edits.

---

## Pillar 1 — {{name}}

**One-sentence definition:** {{...}}

**Why it's a pillar (not just a feature):** What does the game *fundamentally fail* without this? A pillar is load-bearing — remove it and the game collapses.

**Player experience promise:** What feeling does this pillar deliver?

**Anti-patterns (features that would violate this pillar):**
- {{anti-pattern 1}}
- {{anti-pattern 2}}

**The pillar test:** When evaluating a proposed feature, ask: "does this strengthen pillar N, neutral, or weaken?" Weakening features get killed.

---

## Pillar 2 — {{name}}

(Same shape.)

---

## Pillar 3 — {{name}}

(Same shape.)

---

## Pillar 4 — {{name}}

(Same shape.)

---

## Pillar 5 — {{name}}

(Same shape.)

---

## Final Whistle's locked pillars (delete this section in a real pillars-revision doc)

For reference — `docs/DESIGN_DOC.md` §3:

1. **Procedural fantasy world** — every save is a different world; LLM-baked content packs.
2. **Careers that remember** — append-only event ledger surfaces decisions years later.
3. **Breakthrough-driven development** — players grow because of what happened, not XP.
4. **Scouting uncertainty** — disagreeing biased scouts; truth emerges over seasons.
5. **Signature identity** — 24 readable on-pitch moves, not stat lines.

## When to rewrite the pillars

- Major pivot (like the 2026-05-13 Unity→Rust pivot — pillars survived intact)
- A pillar is consistently *not* being served by shipped features (signal that the pillar is aspirational, not real)
- The team has converged on a stronger framing

## When NOT to rewrite

- Single feature feels off — fix the feature, not the pillar
- Phase boundary retrospective — postmortems revise tactics, not pillars
- New genre influence — admire from afar; pillars are stable

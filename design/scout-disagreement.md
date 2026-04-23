---
description: Scout Disagreement system spec. Conditional-MVP feature gated on Month-4 feel prototype.
last_verified: 2026-04-22
status: scaffolded; awaiting Phase 2 prototype spec lock + Month-4 feel-gate decision
---

# Scout Disagreement — uncertainty as gameplay

## Purpose

Answer "can we make scouting a decision-generating system rather than an obscured-numbers system?"

GPT-5.5's framing: FM uses "fog over hidden numbers." Final Whistle tries to make the fog *itself* a gameplay surface. Scouts have models, biases, blind spots, regional familiarity, "taste." A late bloomer looks like a slow winger to one scout and a specialist set-piece freak to another. The player builds truth from disagreement.

## Locked decisions

See SPEC.md 2026-04-22. Summary:

- **Conditional MVP.** System ships in Month-12 EA **only if** Month-4 feel prototype passes its gate.
- **Feel gate criterion:** *"Does scout disagreement create interesting decisions, or does it just obscure truth?"* Pass = MVP inclusion. Fail = fall back to simpler scout-uncertainty (fog-over-numbers FM-style).
- **No omniscient scout reports.** Every scout is a biased observer with a model.

## What the system is (if it ships)

### Scout archetype (draft)

```
Scout {
    id: stable string
    display_name: generated
    archetype: enum
        // 5-8 archetypes at MVP
        "tempo_reader"          // sees midfielders' rhythm, misses finishers
        "academy_spotter"       // bias toward teenagers, miss vets
        "physical_profiler"     // overweights pace + frame, misses decision-making
        "technical_purist"      // bias toward first-touch / passing range
        "set_piece_specialist"  // deep on dead-ball talent, shallow on open play
        "regional_expert"       // accurate in home region, biased/noisy elsewhere
        ...
    regions: [string]  // which nations/regions the scout is familiar with
    observation_noise: f32  // base noise level
    biases: { gene_category: weight }  // per-gene-category weighting
    taste_markers: [string]  // keyword tags this scout emphasizes in reports
    experience: f32  // affects confidence
}
```

### Report generation (per player)

```
1. Scout queries player's internal Identity Packet
2. Bias filter applied: each gene category's visibility weighted by scout.biases
3. Noise filter applied: observation_noise + (1 - regional_familiarity) * regional_noise
4. Scout labels generated from bias-filtered view (e.g., "Late Bloomer" vs "Slow Winger")
5. Confidence score per label
6. Report rendered as structured object + football-native prose
```

Two scouts seeing the same player produce DIFFERENT reports. The player compares reports, weights by scout's track record (which emerges over seasons of observed outcomes), decides who to trust.

### Truth emergence

The player never sees the Identity Packet directly. Truth emerges through:

- Multiple scout reports, weighted by scout track record
- Match-watching (first-person observation when scout sent to watch a specific fixture)
- Time (player ages; dormant traits activate on triggers; signature-readiness leaks clues)
- Memory (former scout reports versus eventual outcomes update the player's priors about each scout)

## MVP boundary (if conditional-MVP gate passes)

At Month 4 (feel prototype):
- 3 scout archetypes
- Generate disagreeing reports on same 10 test players
- Play 2-week feel test; verdict decides MVP inclusion

At Month 12 EA (if included):
- 5-8 scout archetypes
- 8-12 scout instances in the game world (each with track record accumulating)
- Scout assignment UI: send scouts to watch specific fixtures / regions / age groups
- Report comparison UI: side-by-side scout reports with confidence + track record
- Track-record system: scouts who've been right get weighted higher

## Fallback: Scout Uncertainty (if conditional gate fails)

If the feel prototype shows disagreement feels like noise:

- Single scouting report per player with uncertainty bars (FM-like fog)
- Scout quality = bar-tightening speed
- Numbers hidden behind phenotype labels at all times

Preserves the hidden-gene-model pillar without the full disagreement mechanic.

## Deferred

- Regional scouting network evolution (scouts develop taste over seasons) — post-EA
- Rival clubs' scouts visible to player (bidding war intelligence) — post-EA
- Scout poaching / hiring / firing — Phase 6+ post-gate-pass

## Open questions (Phase 2 lock / Month 4 gate)

1. **Archetype count for feel test** — 3 is minimum to show disagreement; 5-8 is MVP target. Lock 3 for prototype.
2. **Report format** — structured data → prose template, or prose-first? Recommend structured → prose templates (see `event-sourced-memory.md` bake-time template approach).
3. **Feel-gate observer set** — user's own judgment + 2-3 trusted testers, or broader itch cohort? Recommend user + 3 testers for Month-4 speed; no broad cohort yet.
4. **Interesting vs obscuring — how do we measure?** Proposal: after the feel test, testers must identify at least one scout-specific trust pattern and make at least one different recruitment decision than they would have made from a neutral aggregate report. Preference without decision impact is not enough.

## Prototype gate

**Month 4 feel-test gate (decisive):**

- 3 scout archetypes implemented
- 10 test players generated with identity packets
- Reports generated for each player from each scout
- User + 3 testers play a bounded feel test with these players + scouts
- Each tester picks 3 players to sign/avoid after seeing disagreeing reports, then repeats with a neutral aggregate report for comparison
- Post-test interview:
  - "Which scouts did you come to trust? Why?"
  - "Did you make decisions you wouldn't have made with one neutral report?"
  - "Was the disagreement engaging or annoying?"
- Pass criterion: ≥3 of 4 testers answer "engaging", cite scout-specific decisions, and change at least one sign/avoid decision versus the neutral aggregate report
- Fail criterion: majority "annoying" or "felt like RNG"

**Pass → Phase 5 expansion of system to 5-8 archetypes + scouting UI work.**
**Fail → log decision, fall back to Scout Uncertainty system, save the engineering effort for other MVP features.**

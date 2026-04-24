---
description: Scout Disagreement system spec. Conditional-MVP feature gated on Month-4 feel prototype.
last_verified: 2026-04-24
status: Phase 0 open questions resolved; Month-4 prototype spec locked (3 archetypes, hand-authored packets, staged-time ledger feedback, one-remediation-pass ceiling). Gate decision still at Month 4. One Phase-2 ADR pre-seeded.
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

## Resolved (2026-04-24)

See SPEC.md decisions log entry `2026-04-24 — Scout Disagreement open questions resolved`. One Phase-2 ADR pre-seeded.

### Q1 — Prototype archetypes (locked)

**3 archetypes for Month-4 prototype:**

- **`physical_profiler`** — overweights pace / frame, misses decision-making
- **`technical_purist`** — bias toward first-touch / passing range, underweights athleticism
- **`regional_expert`** — accurate in home region, noisy elsewhere

**Why these three:** `physical` vs `technical` gives a predictable-but-interesting 1D disagreement axis on the same player (the classic "late-bloomer midfielder or limited physical specimen?"). `regional_expert` adds an orthogonal axis — right-vs-wrong-based-on-where-they-watched. Disagreement surface is 2D, not 1D.

**Explicitly excluded from prototype:** `set_piece_specialist` (Phase-4 system dependency), `tempo_reader` (too subtle for first-contact testing — re-evaluate after pass), `academy_spotter` (overlaps with `physical_profiler`'s age-weighting).

MVP target if gate passes: expand to 5-8 archetypes in Phase 5.

### Q2 — Report format (locked)

**Structured data is canonical; prose is rendered at generation time from templates and stored alongside the structured report for deterministic save/replay.**

```
ScoutReport {
  scout_id: Id
  player_id: Id
  observed_on: career_date
  confidence: f32 [0,1]
  labels: [{ label_id: PhenotypeTag, confidence: f32 }]  // CANONICAL — UI + tests read this
  prose: string                     // rendered from template at generation time; stored for replay
  source_template_id: TemplateId    // audit / regeneration path
}
```

UI + tests read **`labels`** as the canonical data. `prose` is a rendered artifact; regenerating it from the structured data + template must be bit-identical. No runtime LLM (consistent with bake-time content pipeline).

### Q3 — Feel-gate observer set (locked)

**3 external management-game-literate testers.** User facilitates and observes but does NOT count toward the pass criterion — self-testing a system you designed is a known blindspot farm.

**Tester profile:** 20+ hours in FM / OOTP / Motorsport Manager / similar. This is different from the Month-3 football-literate cold-observer pool — the Month-4 test is about **decision-making with uncertain information**, not match legibility.

**Recruitment pool:** Month-2 observer-pool + trusted-friends Discord. Different cohort from Month-3; same recruitment discipline — name 3 by end of Month 3 or flag risk.

### Q4 — Pass criterion + fail-mode routing (locked)

**Pass requires ≥2 of 3 external testers to satisfy ALL THREE criteria:**

1. **Trust attribution** — tester verbalizes at least one scout-specific trust pattern ("I trust the technical guy on midfielders but not defenders") WITHOUT being prompted with the archetype names.
2. **Decision divergence** — tester makes at least one different sign/avoid decision with the disagreement-view than they did with the neutral-aggregate baseline. Preference without decision-impact is not enough.
3. **Affective response** — unprompted self-report frames scouts as "characters to figure out" / "interesting puzzle" / similar. NOT "random," "annoying," "frustrating," or "I just picked one and ignored the others." "I liked it" does not count unless criteria 1 and 2 are also met.

**Fail-mode taxonomy + routed remediations:**

| Fail mode | Signal | Remediation |
|---|---|---|
| **RNG-fail** | testers describe scouts as "random" — archetypes didn't read as systems with models | re-tune biases so each scout has a legible model |
| **Ignore-fail** | testers picked one scout and ignored the rest | scout-track-record surfacing was invisible; add / strengthen the feedback loop |
| **Overload-fail** | testers reported analysis paralysis — 3 reports per player too expensive | reduce to 2 reports per player on the test, OR mark one report as "primary" |

**One remediation pass allowed before gate verdict.** Exactly one. No more. If the remediated prototype still fails any criterion on re-test, the gate is called failed and Scout Uncertainty takes over. This ceiling exists so the conditional-MVP gate doesn't become a feature-rescue loop.

### Item A — Test-player authorship (locked)

**10 hand-authored Identity Packet stubs**, NOT generated by the Identity Packet compiler (which isn't ready at Month 4 anyway). Each stub is deliberately shaped to exercise the 3 scouts' blind spots:

- A **late-bloomer midfielder** who reads as "slow winger" to `physical_profiler` but "underrated passer" to `technical_purist`.
- A **foreign-region talent** with real ability — `regional_expert` is noisy but wrong in the right direction; home-region scouts read him accurately.
- A **pure physical specimen** with poor decision-making — `physical_profiler` rates him highly, `technical_purist` correctly flags him as a bust.
- A **quiet home-region technician** with weak athletics — the inverse of the above.
- Plus 6 more calibrated for partial-blindspot cases (ambiguous disagreement, not clean splits).

**Budget:** ~2 days authoring + iteration. Authoring the packets IS the hidden cost of the feel test; skimp and the test measures the packet, not the system.

### Item B — Minimal ledger writes + staged-time feedback (locked)

The feel test must show **scout reliability changing over time** — without that, testers just pick the scout whose prose sounds best. Full memory system isn't needed; a minimal slice is:

**Staged-time feedback loop:**
1. All 3 scouts report on a player.
2. Simulated "later outcome" is scripted per test player (e.g., "2 seasons later this player signed for Top-Flight and produced 12 goals").
3. Ledger writes `ScoutReportConfirmed` or `ScoutReportDisagreement` events per scout × outcome.
4. Scout reliability scores visibly update between test players.

**Staged, not real:** outcomes are authored for the 10 packets. No need for the full season sim at Month 4. Testers experience scout learning without waiting for 20 in-game years.

**Event-class implication:**
- **If gate passes:** both `ScoutReportConfirmed` + `ScoutReportDisagreement` stay in the event-class enum.
- **If gate fails:** `ScoutReportDisagreement` is dropped at a schema-version bump (already flagged in `event-sourced-memory.md`). `ScoutReportConfirmed` stays — basic scouting still emits it.

## Prototype gate

**Month 4 feel-test gate (decisive):**

- 3 scout archetypes implemented (`physical_profiler`, `technical_purist`, `regional_expert`)
- 10 hand-authored Identity Packet stubs
- Reports generated for each player × each scout
- Staged-time feedback loop showing scout reliability updates between test players
- 3 external management-game-literate testers play a bounded feel test
- User observes + runs the test; does NOT count in the pass criterion
- Each tester picks 3 players to sign/avoid after seeing disagreeing reports, then repeats with a neutral aggregate report for comparison
- Post-test interview covers trust attribution, decision divergence, affective response
- **Pass:** ≥2 of 3 testers satisfy all three criteria (see Q4)
- **Fail-with-remediation-budget:** exactly one remediation pass allowed, routed by fail mode
- **Hard fail after remediation:** fall back to Scout Uncertainty; drop `ScoutReportDisagreement` at schema-version bump; save engineering effort for other MVP features

**Pass → Phase 5 expansion to 5-8 archetypes + scouting UI work.**
**Fail → log decision, fall back to Scout Uncertainty (FM-like fog-over-numbers). Counterplay surfacing (per `design/signatures.md`) still works through the same scout-report UI — simpler certainty levels, same surface.**

# Fun-evaluation harness — automating "is this fun?"

Status: SPEC / strategic. Pairs with `docs/design/drama-model.md` (the metrics) and
`docs/design/match-quality-inspection.md` (DX-2 glitch-detectors + GIF). Owner:
`producer` / `qa-lead` for the protocol; `gameplay-programmer` for the sweep tool.

## The problem this solves

Fun work has no green checkmark. "Is this match gripping?" is a taste judgment, so
it perpetually slips behind the comfortable, testable systems work — and a sim ends
up technically complete and emotionally flat. The project owner does not want to be
the manual oracle who plays builds and reports "felt fun / didn't." This harness
replaces that manual loop with an **automated fun-evaluation** that runs without a
human in the loop for most of the signal, and uses the agent (not the owner) as the
judge for the rest. It is the substrate for an eventual **director mode** that
iterates on feel autonomously.

## Fun splits into two halves — measure one, judge the other

**1. Measurable (no human): drama + coherence metrics.**
Deterministic functions over the match/season record (`drama-model.md`) and the
frame stream (`match-quality-inspection.md`). Run an N-seed sweep, get a
distribution, compare to target bands. A tuning change is A/B'd on the IDENTICAL
seeds, so a moved distribution is the change, not RNG noise. This covers: drama
(late winners, comebacks, tight races), realism guards (goals/match, timing spread),
and physical coherence (no teleports / phasing / phantom goals).

**2. Judgment (agent, not owner): prose + emotional landing + "looks like football".**
Some fun can't be reduced to a number — commentary quality, whether a memory callback
lands as earned rather than generic, whether the dots on the board read as football.
Here the AGENT is the oracle: it reads sampled artifacts against a written rubric and
scores them. The owner trusts the agent for this (stated preference); the rubric makes
the judgment repeatable and reviewable rather than a vibe.

## Component A — `drama-sweep` (the measurable half)

A dev tool (`scripts/fw drama-sweep` front-door over a `fw-replay`/`fw-tauri` bin).

- **Input:** a seed set (count + base seed), scope (`--match` or `--season`), and an
  optional `--baseline <file>` for A/B.
- **Does:** run each seed deterministically (match via the engine; season via
  `advance_season`), compute every `drama-model.md` metric, aggregate to a
  distribution.
- **Output:** a report — per-metric distribution (mean / spread / band pass-fail),
  REALISM-GUARD violations listed loudly, DRAMA-TARGET scores vs target, and (in
  baseline mode) a per-metric DELTA so a coefficient change shows exactly what moved.
  Emits JSON (for the agent to parse) + a human summary.
- **Determinism contract:** pure replay over fixed seeds; no clocks/threads that
  perturb order; identical input → identical report. No sim/canonical change.

This is the controlled-experiment loop: edit a coefficient → `drama-sweep
--baseline before.json` → read the deltas → keep or revert. The owner never watches a
match to know whether scoring got more dramatic.

## Component B — the LLM-judge protocol (the judgment half)

The agent scores sampled artifacts against fixed rubrics (1-5 per dimension; a
dimension scoring ≤2 is a fail with a named reason). Sampling is seed-pinned so a
re-run judges the same artifacts.

- **Commentary** (transcript from a sweep match): *readable* (reads like a match
  report, not robotic), *varied* (no repeated phrasing within a session), *football-
  native* (real footballing language; no banned mystical nouns), *specific* (names
  the player/where on the pitch, not "a midfielder did a thing").
- **Memory callback** (rendered callback on `/player` or `/career`): *specific*
  (references the actual event — that kid, that decision — not a template), *earned*
  (the emotional weight is justified by what happened), *legible* (a player would
  understand why it surfaced).
- **Match motion** (a DX-2 GIF/contact-sheet): *coherent* (players hold shape, move
  like footballers, no drifting/teleporting), *football-shaped* (recognisable phases
  of play), paired with the glitch-detector report so a clean GIF + zero flags = pass.

The judge is the agent reading these via the DX-1 harness + DX-2 artifacts. Its
verdicts are logged (seed + rubric + score + reason) so they're auditable and the
owner can spot-check rather than originate every judgment.

## Component C — feel-probes (how fun tasks are scheduled)

Every fun task is framed as a FALSIFIABLE feel-probe combining A + B, so it can be
scheduled and "passed" like any other acceptance criterion. Examples:

- *Match drama:* "Across 20 seeds, `drama-sweep` shows late-drama rate in band [M5]
  AND ≥12/20 matches score ≥4/5 on the LLM-judge 'gripping' rubric."
- *Commentary:* "On a 20-match sample, ≥90% of commentary lines score ≥4/5 on
  readable+specific, and zero repeated phrasings within a match."
- *Callback lands:* "The season-3 debut callback references the specific event and
  scores ≥4/5 on earned-ness across 10 seeded careers."
- *Coherent football:* "Zero `inspect-frames` glitch flags on a 5-seed × full-match
  sweep AND the GIF passes the 'football-shaped' rubric."

A feel-probe failing tells you WHAT to tune; the same-seed replay tells you whether
your tune fixed it.

## The autonomy ladder → director mode

This harness is the substrate for increasing autonomy, with the existing caveats
intact (the canonical-hash gate, the self-review triple, append-only DECISIONS, and
owner approval on product-vision forks all still bind):

1. **Assisted** (now): agent runs `drama-sweep` + judges artifacts, proposes tuning, owner okays.
2. **Supervised loop:** agent runs the play→measure→judge→tune→verify→commit loop on
   a feel-probe, stopping at the probe boundary for owner review.
3. **Director mode:** agent runs the loop unattended (e.g. overnight) against a queue
   of feel-probes, committing improvements that pass verify + the probe, leaving a
   report; only product-vision forks escalate.

Each rung needs the rung below it solid first. DX-1 (play/view) + DX-2 (coherence) +
`drama-model` (drama) + this harness are rungs 1-2; rung 3 is the goal.

## Scope note — this is match-feel + season-arc, not feature breadth

This harness evaluates the fun of what EXISTS (match-feel, season drama, commentary,
callbacks). The OTHER axis of fun — management depth (scouting lower leagues for a
wonderkid, rebuilding a youth side, an underdog cup run, hours in the tactics screen)
— is feature BREADTH, tracked separately via the feature-backlog research
(`docs/design/feature-backlog.md`, forthcoming). Both axes matter; this doc owns only
the first.

## Harness-evolution directions (considered 2026-06-04; sequence-gated)

A brainstorm explored pushing the DX/eval harness further. Verdict + when each lands.
GUARDRAIL (and the test for ALL of these): build the next harness piece ONLY when it
unblocks actual fun-progress, not because the infra is seductive. The #1 priority
stays a WATCHABLE match; everything below is gated on that + the FUN-H1 drama-sweep
existing.

- **Archetype-distinctness probe (ADOPT → FUN-1).** Run all manager archetypes through
  the sweep and assert each produces a statistically distinguishable match signature
  (press distance, transition speed, pass-network shape). Falsifiable proof that
  tactical depth reaches the pitch — currently unproven. Add as a drama-model metric;
  meaningful only once the match is watchable (on broken football it just reports
  "tactics don't express yet").
- **Glitch-coherence CI gate (ADOPT, light, soon).** Run the DX-2 detectors in CI to
  fail a commit that reintroduces teleport/phasing/velocity glitches — cheap, protects
  the FUN-0 fix immediately. (Pairs with the existing canonical-hash + determinism gates.)
- **Drama CI gate (ADOPT, full, later).** Extend the gate to fail when drama drops
  below band or an archetype loses its fingerprint — a "you cannot merge football that
  got worse" gate. Only once there's TUNED fun worth protecting (post FUN-1).
- **Product spin-offs (CAPTURE → feature-backlog).** The DX-2 drama-detector + GIF
  renderer, pointed at a finished match/season, are an auto-highlight-reel + "season in
  N GIFs"; a deterministic match is shareable as a seed. Near-free EA delight; the
  primitives already exist. Product feature, not dev-tool — schedule against the EA roadmap.
- **Agent-as-manager (FUTURE BET → director mode).** An agent plays hundreds of
  deterministic careers; mine for skill-expression (does a good manager outperform a
  bad one?) and "where did the agent have no meaningful choice / get bored" = automated
  feature-gap discovery feeding the breadth backlog. Blocked on the management surface
  existing; the seed (agent drives the existing season loop via the DX-1 bridge) is
  feasible once there's a season worth playing.
- **Auto-tuner (DEFER, maybe never).** A deterministic optimizer (CMA-ES / Bayesian)
  searching tuning coefficients toward target bands. The cleanest possible optimization
  problem BUT carries Goodhart risk (optimizes into football that scores great and feels
  wrong) and is redundant while manual A/B-on-identical-seeds tuning is fast. Revisit
  only if manual tuning proves too slow across many coefficients; never run unsupervised
  on measurable metrics alone (the LLM-judge must anchor it).
- **Season/economy balance sweeps (ALREADY PLANNED).** Covered by the drama-model
  season metrics (S1-S4) + FUN-4; economy sweeps wait on the economy features existing.
- **Reusable-blueprint extraction (SKIP).** Portfolio/methodology meta; not
  shipping-relevant.

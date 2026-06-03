# Signature catalogue

Pillar 5 — *signature identity*: players are readable on the pitch through
recurring, role-flavoured moves, not stat lines. This file is the design
source-of-truth for the signature catalogue: which signatures exist, what
role family each belongs to, what triggers them, and which are live vs.
planned.

- **Mechanical architecture:** `docs/adr/0011-signature-system.md` (trigger →
  fit-score → softmax → bias snapshot → cooldown/stacking → commentary).
- **Trigger predicates:** Rust functions in
  `crates/fw-match-sim/src/signature/triggers.rs`, bound to a signature id in
  `build_trigger_table()`. The RON `trigger` field stays `NoOpStub` — the
  predicate logic lives in code, not RON, until a future data-driven pass.
- **Definitions (RON):** `content/sources/signatures/<slug>.ron` (one per
  *implemented* signature). Planned entries below have NO RON file yet — the
  RON + predicate land together when each is implemented.
- **Commentary:** per-signature Tracery banks at
  `content/sources/commentary/signature_first_fired.<slug>.tracery.json`
  (T4-2.5i routing), falling back to the generic bank.

## Scope

Initial scope is **24 signatures = 8 role families × 3 each**. This is the
EA target, not a cap (CLAUDE.md §1: "the 24 signature number is initial
scope, not a cap"). As of T4-2.5j, **8 are implemented** — one per role
family, so every family is represented on the pitch — and **16 are planned
stubs** (`not_yet_implemented: true`), kept here for traceability so the
catalogue grows against a known shape rather than ad hoc.

## Role families

The eight `RoleFamily` variants (stable discriminants — do not reorder):
`Goalkeeper`, `CentreBack`, `FullBack`, `DefensiveMidfielder`,
`CentralMidfielder`, `AttackingMidfielder`, `Winger`, `Striker`.

### Slot convention (4-3-3)

Trigger predicates gate by formation position via `in_team = slot % 11`
(slots 0–10 = home XI, 11–21 = away XI). The convention:

| `in_team` | Position | Role family |
|---|---|---|
| 0 | Goalkeeper | Goalkeeper |
| 1 | Right-back | FullBack |
| 2 | Right centre-back | CentreBack |
| 3 | Left centre-back | CentreBack |
| 4 | Left-back | FullBack |
| 5 | Defensive-mid pivot | DefensiveMidfielder |
| 6 | Central midfielder | CentralMidfielder |
| 7 | Attacking midfielder | CentralMidfielder / AttackingMidfielder |
| 8 | Right winger | Winger |
| 9 | Centre-forward | Striker |
| 10 | Left winger | Winger |

This is the deterministic discriminator until canonical match state carries a
finer role tag than the 4-way `Role` enum (spatial role state, post-T2-1).
Some existing predicates intentionally use broader gates (e.g.
`long-range-strike` fires for any midfielder-or-forward); the per-family
gates above apply to the T4-2.5j additions.

## Catalogue

Status legend: **live** = implemented predicate + RON + commentary;
**planned** = `not_yet_implemented: true`, design intent only.

Thresholds are Q32 attribute floors (~0.45 unless noted); fit-score is the
product of the gating attributes (each in [0, 1]). Bias multipliers are the
five `SimBiasSnapshot` lanes (shoot / pass / dribble / press / cover) applied
while the signature is firing.

### Goalkeeper

- **`commanding-claim`** — *live*. The keeper dominates the box to claim a
  cross or high ball. Gate `in_team == 0`. Attributes:
  `goalkeeper.aerial_reach` × `goalkeeper.handling` ×
  `goalkeeper.command_of_area`. Bias: shoot ×0.5, press ×1.2, cover ×1.4
  (others ×1.0). Cooldown 600 ticks. Stacking: Exclusive(Defensive).
- **`sweeper-rush`** — *planned* (`not_yet_implemented: true`). The keeper
  reads the through-ball early and sweeps off the line to smother it outside
  the area. Family Goalkeeper.
- **`reflex-wall`** — *planned* (`not_yet_implemented: true`). A point-blank
  reaction save from a deflection or close-range strike. Family Goalkeeper.

### CentreBack

- **`body-shield-pressure`** — *live*. A defender shields the ball / man under
  pressure, using strength and timing to break up the attack. Gate
  `in_team` 1–7. Attributes: `technical.marking` × `physical.strength` ×
  `personality.aggression`. Bias: shoot ×0.3, pass ×0.5, dribble ×0.3,
  press ×1.5, cover ×1.5. Cooldown 600 ticks. Stacking: Exclusive(Defensive).
- **`last-ditch-block`** — *planned* (`not_yet_implemented: true`). A sliding
  block thrown in front of a goal-bound shot. Family CentreBack.
- **`stepout-intercept`** — *planned* (`not_yet_implemented: true`). The
  centre-back steps out of the line to intercept a pass into feet. Family
  CentreBack.

### FullBack

- **`overlapping-surge`** — *live*. A full-back makes the overlapping run,
  combining physical drive with a cross from deep. Gate `in_team == 1 || 4`.
  Attributes: `physical.pace` × `physical.stamina` × `technical.crossing`.
  Bias: pass ×1.3, dribble ×1.4, cover ×0.7 (others ×1.0). Cooldown 600
  ticks. Stacking: Exclusive(BuildUp).
- **`recovery-sprint`** — *planned* (`not_yet_implemented: true`). The
  full-back tracks back at pace to cover the space behind after losing the
  ball. Family FullBack.
- **`underlap-drive`** — *planned* (`not_yet_implemented: true`). An
  underlapping run inside the winger to break the line through the
  half-space. Family FullBack.

### DefensiveMidfielder

- **`screening-interception`** — *live*. The pivot reads the passing lane and
  steps in to intercept, shielding the back four. Gate `in_team == 5`.
  Attributes: `mental.anticipation` × `mental.positioning` ×
  `technical.tackling` × `technical.marking`. Bias: shoot ×0.5, press ×1.5,
  cover ×1.4 (others ×1.0). Cooldown 600 ticks. Stacking: Exclusive(Defensive).
- **`tempo-reset`** — *planned* (`not_yet_implemented: true`). The holding
  midfielder slows the game and recycles possession to reset the shape.
  Family DefensiveMidfielder.
- **`foul-stopper`** — *planned* (`not_yet_implemented: true`). A tactical
  break-up of a counter — the cynical-but-controlled stop. Family
  DefensiveMidfielder.

### CentralMidfielder

- **`first-time-diagonal-switch`** — *live*. A first-time raking pass that
  switches the point of attack across the pitch. Gate `in_team` 5–7.
  Attributes: `mental.vision` × `technical.passing`. Bias: shoot ×1.1,
  pass ×2.0, dribble ×1.5, press ×0.4, cover ×0.6. Cooldown 450 ticks.
  Stacking: Exclusive(BuildUp).
- **`line-breaking-carry`** — *planned* (`not_yet_implemented: true`). The
  midfielder drives forward with the ball, beating the press by carrying
  through the lines. Family CentralMidfielder.
- **`third-man-combination`** — *planned* (`not_yet_implemented: true`). A
  one-two release that springs the third runner. Family CentralMidfielder.

### AttackingMidfielder

- **`long-range-strike`** — *live*. The player stays composed to unleash a
  shot from distance when space opens beyond the line. Gate `in_team` 5–10.
  Attributes: `mental.composure` × `technical.long_shots`. Bias: shoot ×2.0,
  pass ×0.3, dribble ×1.2, press ×0.3, cover ×0.4. Cooldown 900 ticks.
  Stacking: Exclusive(BuildUp).
- **`throughball-vision`** — *planned* (`not_yet_implemented: true`). The
  killer pass that splits the defensive line and releases a runner. Family
  AttackingMidfielder.
- **`half-space-glide`** — *planned* (`not_yet_implemented: true`). The
  number ten drifts into the half-space to receive between the lines. Family
  AttackingMidfielder.

### Winger

- **`touchline-beat`** — *live*. The winger takes on the full-back on the
  touchline, using pace and control to reach the byline and cross. Gate
  `in_team == 8 || 10`. Attributes: `technical.dribbling` × `physical.pace` ×
  `technical.crossing`. Bias: pass ×1.1, dribble ×1.8, cover ×0.5 (others
  ×1.0). Cooldown 600 ticks. Stacking: Exclusive(BuildUp).
- **`cut-inside-curler`** — *planned* (`not_yet_implemented: true`). The
  inverted winger cuts inside onto the stronger foot to curl one toward the
  far corner. Family Winger.
- **`chalk-on-boots-hug`** — *planned* (`not_yet_implemented: true`). The
  winger hugs the touchline to stretch the defence and hold width. Family
  Winger.

### Striker

- **`poachers-dart`** — *live*. The striker times the run and darts in behind
  the line to get on the end of the chance. Gate `in_team == 9`. Attributes:
  `mental.off_the_ball` × `mental.anticipation` × `technical.finishing` ×
  `physical.acceleration` × `physical.pace`. Bias: shoot ×1.9, press ×0.6,
  cover ×0.4 (others ×1.0). Cooldown 900 ticks. Stacking: Exclusive(BuildUp).
- **`hold-up-link`** — *planned* (`not_yet_implemented: true`). Back to goal,
  the striker holds the ball up under pressure and lays it off to a runner.
  Family Striker.
- **`near-post-glance`** — *planned* (`not_yet_implemented: true`). A flicked
  near-post header that redirects the cross. Family Striker.

## Determinism notes

- Predicates are pure Q32 functions of `(&MatchState, PlayerSlot)` — no RNG.
  RNG enters only in the dispatcher's softmax when 2+ candidates are
  simultaneously eligible (`SeedLayer::SignatureTrigger`).
- A signature's `signature_candidates` entry on a slot is canonical match
  state. Adding/wiring candidates changes the canonical hash; the T4-2.5j
  catalogue expansion rebaselined the 600-tick extended pin (the 60-tick
  smoke pin runs without content, so it is unaffected). See the re-baseline
  history in `crates/fw-replay/tests/canonical_hash.rs`.

## Implementation status (T4-2.5j)

8 of 24 live, one per role family (every family represented). 16 planned
stubs documented above. The next signatures to implement should fill out the
families that today have only their single anchor; each new one adds its RON
+ predicate + commentary bank + (where it fires in the pinned scenario) a
re-baseline.

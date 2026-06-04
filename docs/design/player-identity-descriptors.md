# Player identity descriptors — football-fan playing identities

> Status: DESIGN (2026-06-04). The descriptor taxonomy for the player stats panel — 16 position-gated playing identities + 5 universal trait tags, computed deterministically from attribute groups + genes + role. Replaces the placeholder 5 (technical artist / physical specimen / etc.) with identities a fan actually uses. Consumed by `docs/design/player-stats-presentation.md` §2 (the identity chip) and grounded by `docs/design/gene-attribute-compiler.md` (the real attribute spread that makes them fire). Owner: narrative-director (labels) + systems-designer (thresholds). Football-native; thresholds are SOFT.

## Player Identity Descriptors — Replacement Proposal for §2

### What changes and why

The current five descriptors are group-mean labels wearing football clothes. "Technical artist" and "mentality monster" say almost nothing about how a player actually plays — they describe attribute-cluster dominance, not a playing identity. Real fans talk about roles and behaviours: what a player does with the ball, where they do it, what they solve on the pitch. This proposal replaces the five with fifteen position-gated playing identities plus four universal trait tags. The primary identity goes on the squad chip; the trait tags are additive secondaries (show one, maximum two).

---

### Part 1: The descriptor set

Each entry: label | position gate | derivation rule | why it lands with fans.

---

**Striker identities**

**"poacher"**
Gate: role family = striker (centre-forward / second striker).
Derivation: `technical.finishing > 0.68` AND `mental.off_the_ball > 0.65` AND `physical.pace < 0.60` AND `mental.vision < 0.52`.
A high finisher who reads space early and doesn't need to create it. The vision/pace cuts prevent it firing on all-rounders. Fans know exactly what they're getting — someone who lives in the six-yard box and doesn't do much else.

**"fox in the box"**
Gate: role family = striker.
Derivation: `technical.finishing > 0.65` AND `mental.off_the_ball > 0.62` AND `mental.anticipation > 0.65` AND `technical.technique < 0.60`.
Slightly wider than a poacher — reads the game, gets in positions, but technically limited. Fans use this for the player who scores without looking pretty.

**"target man"**
Gate: role family = striker.
Derivation: `physical.jumping_reach > 0.68` AND `physical.strength > 0.65` AND `genes.physical.height_ceiling > 0.70` AND `technical.heading > 0.65`.
Aerial gene plus strength plus heading skill. Height ceiling gene makes this honest — a short strong striker won't mislabel. Fans picture the tall number nine who wins flick-ons.

**"false nine"**
Gate: role family = striker.
Derivation: `technical.dribbling > 0.62` AND `mental.vision > 0.65` AND `technical.passing > 0.62` AND `mental.off_the_ball < 0.58`.
A forward who drops deep and creates rather than arriving late. The `off_the_ball` floor prevents it firing on someone who just happens to have good vision.

---

**Wide player identities**

**"inverted winger"**
Gate: role family = wide forward / winger.
Derivation: `genes.technical.left_foot > 0.60` AND role is right-sided (or `left_foot < 0.35` and role is left-sided) AND `technical.dribbling > 0.62` AND `technical.finishing > 0.58`.
A wide player whose strong foot is on the inside — cuts in, shoots, creates. The footedness gene doing actual positional work here. Fans immediately picture the player who always comes inside.

**"touchline winger"**
Gate: role family = wide forward / winger.
Derivation: `physical.pace > 0.68` AND `physical.acceleration > 0.65` AND `technical.crossing > 0.62` AND `genes.physical.fast_twitch_ratio > 0.62`.
Hugs the line, delivers. Fast-twitch gene makes the pace feel genetic rather than trained. Fans see the flying winger who gets to the byline.

**"wide target man"**
Gate: role family = wide forward / winger.
Derivation: `physical.jumping_reach > 0.65` AND `physical.strength > 0.65` AND `genes.physical.height_ceiling > 0.68` AND `technical.crossing < 0.52`.
The wide player who holds up, flicks on, competes in the air rather than delivering into the box. Less common than the others, distinctive enough to be interesting.

---

**Midfield identities**

**"deep-lying playmaker"**
Gate: role family = central midfielder / defensive midfielder.
Derivation: `mental.vision > 0.68` AND `technical.passing > 0.68` AND `mental.positioning > 0.62` AND `physical.pace < 0.55`.
High vision plus high passing plus positional sense. Slow pace locks it to the deeper archetype — a fast deep-lying playmaker is something else entirely. Fans call this the regista, the quarterback, the one who makes the team tick.

**"box-to-box"**
Gate: role family = central midfielder.
Derivation: `personality.work_rate > 0.68` AND `physical.stamina > 0.65` AND `mental.off_the_ball > 0.58` AND `technical.tackling > 0.52` AND `technical.passing > 0.52`.
Work rate (hidden field) plus stamina plus breadth of contribution. Uses a personality field which is hidden from scouts, so this label takes longer to confirm — appropriate for Pillar 4. Fans mean the tireless runner who does everything.

**"ball-winner"**
Gate: role family = central midfielder / defensive midfielder.
Derivation: `technical.tackling > 0.68` AND `technical.marking > 0.62` AND `personality.aggression > 0.62` AND `physical.strength > 0.58`.
Tackling plus marking plus aggression gene (hidden, takes scouting to confirm). Fans call this the destroyer, the enforcer. The aggression threshold being from `PersonalityVector` makes it suitably uncertain to discover.

**"no. 10"**
Gate: role family = attacking midfielder / second striker.
Derivation: `mental.vision > 0.65` AND `technical.technique > 0.65` AND `genes.mental.pattern_recognition > 0.62` AND `technical.dribbling > 0.58`.
The creative hub behind the striker. Pattern recognition gene from `MentalGenes` grounding the label in something deeper than just skill. Fans know exactly what a "number ten" is.

---

**Defender identities**

**"ball-playing centre-back"**
Gate: role family = centre-back.
Derivation: `technical.passing > 0.62` AND `technical.first_touch > 0.60` AND `mental.vision > 0.58` AND `technical.marking > 0.58`.
The defender who starts attacks. Requires marking to confirm they're actually a defender, not a mislabelled midfielder. Fans use this for the Beckenbauer/Cruyff type who brings it out.

**"no-nonsense centre-half"**
Gate: role family = centre-back.
Derivation: `technical.heading > 0.65` AND `technical.tackling > 0.65` AND `physical.strength > 0.62` AND `technical.passing < 0.52`.
The passing floor inverts here — a high passer isn't a no-nonsense type, they're a ball-player. This label fires on the stopper who wins headers and makes clearances. Fans love calling a player a "proper old-fashioned centre-half."

**"marauding full-back"**
Gate: role family = full-back / wing-back.
Derivation: `physical.pace > 0.65` AND `physical.stamina > 0.62` AND `technical.crossing > 0.60` AND `mental.off_the_ball > 0.55`.
The overlapping full-back. Off-the-ball intelligence plus pace plus crossing and stamina to get back. Fans picture the full-back who's essentially a winger half the time.

---

**Goalkeeper identities**

**"shot-stopper"**
Gate: role family = goalkeeper.
Derivation: `goalkeeper.reflexes > 0.68` AND `goalkeeper.one_on_ones > 0.62` AND `goalkeeper.command_of_area < 0.55`.
Strong shot-stopping instincts, weaker at dominating the area. The command floor is the distinguishing cut. Fans call this the keeper who wins one-on-ones but sometimes disappears on set pieces.

**"sweeper-keeper"**
Gate: role family = goalkeeper.
Derivation: `goalkeeper.command_of_area > 0.65` AND `goalkeeper.kicking > 0.62` AND `physical.pace > 0.55` AND `goalkeeper.one_on_ones > 0.58`.
Comes for crosses, plays off the line, distributes well. Pace gene included because a sweeper-keeper who isn't comfortable rushing out is a liability. Fans have used this term since Neuer made it mainstream.

---

### Part 2: Primary selection + secondary trait tags

**Primary identity selection — priority order**

When a player qualifies for more than one descriptor, the rule is:

1. Apply the position gate first — only eligible descriptors enter the pool.
2. Within the eligible pool, score each descriptor as the sum of its trigger attributes minus their thresholds (a measure of "how clearly" the player fits). The descriptor with the highest positive margin wins.
3. If no descriptor clears all its thresholds, use a fallback label per position group: "forward" / "midfielder" / "defender" / "goalkeeper" — plain, honest, no pretension.

This is deterministic: same attributes, same result. No random tie-breaking. The margin calculation is arithmetic on Q32 means, so it computes in the same DTO build step.

**Secondary trait tags — show 0 to 2**

Trait tags are additive and not position-gated. They fire from gene flags and a small number of attribute thresholds independent of the primary identity. Show a maximum of two. Order by salience (show the rarest/most distinctive first).

| Tag | Fan-facing label | Derivation |
|---|---|---|
| `LeftFooted` | "left-footed" | `genes.technical.left_foot > 0.65` |
| `PaceyRunner` | "pacey" | `physical.pace > 0.70` AND `physical.acceleration > 0.68` AND `genes.physical.fast_twitch_ratio > 0.65` |
| `DeadBallSpecialist` | "set-piece threat" | `genes.technical.dead_ball > 0.70` AND (`technical.free_kicks > 0.65` OR `technical.corners > 0.65`) |
| `AerialThreat` | "aerial threat" | `genes.technical.aerial > 0.65` AND `physical.jumping_reach > 0.65` AND `technical.heading > 0.62` |
| `LateBloomer` | "late developer" | `narrative_flags.contains(LateBloomer)` OR `genes.physical.growth_curve > 0.40` |

"Late developer" replaces the current "late bloomer" — same meaning, sounds less like a gardening metaphor. The gene fields are the honest source of truth for all of these; they're not derivable from attributes alone, which means a scout who hasn't dug deep enough won't see them. That's the correct Pillar 4 behaviour.

---

### Part 3: Knowledge-gating per the existing UncertaintyBand model

The primary chip follows the existing gating from §2.5 unchanged. What changes is the confidence logic for descriptors that depend on personality fields (`work_rate`, `aggression`) or mental genes (`pattern_recognition`, `composure_floor`):

These fields are hidden in the sense that scouts don't observe them directly — they accumulate evidence through matches watched. In practice this means "box-to-box" and "ball-winner" and "no. 10" (which uses `pattern_recognition`) will take longer to reach Confident/Settled than "target man" (height is immediately visible) or "poacher" (finishing reveals itself in minutes). This emerges naturally from the existing scouting model without any special casing — personality fields have higher `BASIC_SCOUT_BAND_HALF_WIDTH` by construction.

Gene-to-descriptor influence additions beyond the current spec:

| Gene field | Effect |
|---|---|
| `genes.technical.left_foot > 0.60` + role side | Unlocks `inverted winger` or tips `touchline winger` away |
| `genes.physical.fast_twitch_ratio > 0.62` | Required for `touchline winger`; tips `pacey` trait tag |
| `genes.physical.height_ceiling > 0.70` | Required for `target man` and `wide target man` |
| `genes.technical.aerial > 0.65` | Required for `aerial threat` tag |
| `genes.technical.dead_ball > 0.70` | Required for `set-piece threat` tag |
| `genes.mental.pattern_recognition > 0.62` | Required for `no. 10` |
| `genes.physical.growth_curve > 0.40` | Triggers `late developer` tag |
| `narrative_flags.LateBloomer` | Triggers `late developer` tag |

---

### Part 4: DTO/enum changes needed for §2

**Replace `IdentityDescriptorDTO` enum** with 17 variants instead of 5:

```
Poacher, FoxInTheBox, TargetMan, FalseNine,
InvertedWinger, TouchlineWinger, WideTargetMan,
DeepLyingPlaymaker, BoxToBox, BallWinner, NumberTen,
BallPlayingCentreBack, NoNonsenseCentreHalf, MaraudingFullBack,
ShotStopper, SweeperKeeper,
Fallback,   // plain positional label — "forward", "midfielder", etc.
```

**Add `trait_tags: Vec<TraitTagDTO>` to `PlayerAssessmentDTO`** (max 2 elements in practice, Vec for clean serialization). `TraitTagDTO` is:

```
LeftFooted, PaceyRunner, DeadBallSpecialist, AerialThreat, LateDeveloper,
```

**Add `role_family: RoleFamilyDTO` to `PlayerAssessmentDTO`** (or derive from existing `role_label`) so the classifier has the position gate available without re-parsing a string. `RoleFamilyDTO`:

```
Striker, WideForward, CentralMidfielder, AttackingMidfielder,
DefensiveMidfielder, FullBack, CentreBack, Goalkeeper,
```

**Classifier changes:** the `if/else if` chain in §2.3 becomes a position-gated loop — gate by `role_family`, score eligible descriptors by margin, select the highest. Still a pure function with no allocations that matter (15 candidates, each a handful of Q32 comparisons). The margin scoring is a sum of `(attribute - threshold).max(Q32::ZERO)` across each rule's conditions, computed inline.

**Radar colour additions:** each new descriptor needs a colour token. The existing palette (slate blue, burnt orange, deep green, amber) can map as follows — strikers get warm tones (amber, red-orange), wide players get yellow-green, midfielders get blue range, defenders get slate/steel, keepers grey-green. Exact hex stays in the Tailwind token file, not here.

---

### On the existing five

`TechnicalArtist` and `MentalityMonster` are fair descriptions of attribute dominance but not playing identities — a fan can't picture a player from either label. `PhysicalSpecimen` is a tabloid quality label. `LeftFootedWand` is charming but it's a trait tag, not a playing identity. `LateBloomer` was correct in spirit and survives here as `late developer` in the trait tag layer. None of the five survive as primary identities in this proposal; they're replaced by labels a fan would use in actual conversation.

---

## Refinements (owner feedback, 2026-06-04)

### Precedence + specificity (sharpens primary selection)
Primary identity is chosen by: (1) the position gate filters the eligible pool; (2) the **most-SPECIFIC fully-satisfied identity wins** — specificity = how many signals a descriptor demands, so a 5-signal combo (e.g. box-to-box) outranks a 2-signal generic and a player who clears a demanding combo is labelled *that*, not a lazy partial match; (3) **margin** (`Σ (attribute − threshold).max(0)` over the satisfied conditions) is the tie-break only between equally-specific candidates; (4) plain positional fallback ("forward"/"midfielder"/…) when nothing clears. Deterministic Q32 throughout. (This supersedes "margin alone decides" → "specificity-first, margin tie-break".)

### Knowledge-gated, level-RELATIVE quality tier — the "elite"/"world-class" prefix, done our way (NOT an FM copy)
A quality prefix MAY precede the identity ("a world-class poacher", "a promising ball-winner"). Three rules make it ours, not FM's omniscient star:
- **Knowledge-gated:** the tier is itself uncertain — it hedges until the scouting band firms ("looks a top-class poacher" at Tentative → "world-class poacher" at Confident/Settled). You earn certainty about the *level*, not just the role.
- **RELATIVE to the player's level, not absolute:** the tier = the player's CA measured against the *division baseline* — "elite in the fourth tier" ≠ "elite in the top flight"; a wonderkid reads elite where he plays and you find out if it travels. (Uses the division/level context from world-scale; until that lands it derives from CA vs a neutral baseline.)
- **Sparse + pundit-voiced:** NO prefix for an average player (just the identity); prefixes only at the ends — *promising* (young + high ceiling) / *accomplished* / *top-class* / *world-class* / *generational*. Avoid FM's exact labels; use football-native pundit words. Combined label = `[quality] [identity]`.
DTO add: `quality_tier: Option<QualityTierDTO>` carrying the tier + its hedge state from the band. The CA-vs-baseline cutoffs are SOFT tuning and live here.

### Identity descriptor vs tactical ROLE (distinct, reinforcing layers — not a clash)
The identity descriptor is a **descriptive read** — *what a player IS* (the scout's/pundit's verdict on the stats panel), derived from attributes/genes/position. A **tactical role** (the FM-style role you SELECT in tactics — NOT yet built) is **prescriptive** — *what you deploy him AS*. They share vocabulary ("deep-lying playmaker") on purpose. The **bridge is role-fit**: deploy a player in a role matching his identity → he performs; out of it → the runtime role-fit penalty (FUN-TI2). So this descriptor layer becomes the natural "suited role" hint when the tactical-role system lands. Keep them distinct-but-reinforcing; the naming overlap is a feature, not duplication.

---

**Source files consulted:**
- `/Users/vibelogic/dev/football/docs/design/player-stats-presentation.md`
- `/Users/vibelogic/dev/football/crates/fw-core/src/player_attributes.rs`
- `/Users/vibelogic/dev/football/crates/fw-content/src/gene.rs`
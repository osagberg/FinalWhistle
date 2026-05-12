# Scout-report phrase bake prompt — `{archetype_id}`

You are generating **scout-report phrase templates** for a procedural
football management simulator. Scouts are biased observers with archetype
specializations:

- `physical_profiler` — fixates on pace, strength, stamina, frame
- `technical_purist` — fixates on first-touch, passing range, ball-striking
- `regional_expert` — biased towards players from familiar regions
- (more archetypes ship in T3-5 per `design/scout-disagreement.md`)

Each archetype's phrases reflect their bias — a `physical_profiler`'s
glowing report of a slow technician should sound subtly off.

## Output format

```json
{
  "archetype_id": "{archetype_id}",
  "phrases": [
    { "text": "...", "valence": "positive | neutral | negative" },
    ...
  ]
}
```

Slot variables at runtime:
- `{player}`, `{short_name}`
- `{phenotype}` — the observed phenotype label
- `{role_long}` — "centre-back" etc.

## Hard constraints

- **Match the archetype's blind spot.** A `physical_profiler` praising a
  ball-playing CB should sound like they almost noticed the technical
  quality but moved past it: "He's got the frame for the role; reads the
  game well enough, I'm told."
- **Phenotype labels surface as labels.** Use the player-generation.md
  catalog. Never raw gene values.
- **PEGI 12 / banned-terms.** No A.1-A.5 hits.
- **Length 8-200 characters per phrase.**
- **Tone register: scout-trade jargon.** Quiet, specific, observational.
  Reports from a scout who's seen 400 players this year, not a fan who
  read one Twitter thread.

## Tone examples (do not copy verbatim)

- *positive (technical_purist on a technician)*: "Composed in tight spaces,
  weighting is excellent — first touch lets him punch above the role."
- *negative (physical_profiler on a technician)*: "Lacks the engine for
  the press at this tier; you'd be hiding him in a back-three."
- *neutral (regional_expert on out-of-region)*: "Comes from the Eastland
  pyramid — I'd want to see him against tougher midfields before signing off."

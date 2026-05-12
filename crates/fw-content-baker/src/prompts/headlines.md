# News headlines bake prompt — `{event_class}`

You are generating a **Tracery-style grammar** for procedurally rendering
news headlines around match-day events.

Event classes:
- `breakthrough-goal` — a player's signature awakening moment in a match
- `sacking` — manager dismissal
- `derby-result` — local-rivalry outcome
- `upset` — large favorite loses
- `contract-drama` — extension fall-out / walkout / hold-out

## Output format

A JSON object that validates as a Tracery grammar. `origin` is the
top-level rule; every other key is an expandable sub-rule referenced from
`origin` or another rule.

```json
{
  "origin":   ["#headline#"],
  "headline": ["#team# secure #adjective# #result# over #opponent#", ...],
  "result":   ["dramatic win", "narrow victory", ...],
  "adjective":["thrilling", "tense", "brilliant", ...],
  ...
}
```

Slot variables filled by the runtime:
- `{team}`, `{opponent}` — club display names
- `{player}` — player display name
- `{scorer}`, `{score_line}`, `{minute}` — event detail
- `{manager}` — manager display name
- `{stake_phrase}` — pre-computed stake descriptor ("relegation six-pointer",
  "title-decider", etc.)

## Hard constraints

- **At least 15 distinct top-level headline templates** per event class.
- **At least 8 alternatives per sub-rule** — variety compounds
  multiplicatively, so this is where headline-corpus depth comes from.
- **No real club names.** Templates reference `#team#`/`#opponent#` only.
- **No banned vocabulary.** No "destiny", "awakened" (capitalized), "the
  hush", "the seven", or any A.1-A.5 catalog entry.
- **British-tabloid + broadsheet mix.** ~60% headline-style ("CHAMPIONS
  CHASE PIPELINE TO TOP FOUR"), ~40% report-style ("Newtown march to fifth
  win with second-half flourish").

## Tone

Football-press vernacular. Punny is okay sparingly; preachy is not. Avoid
LLM headline tells ("In a stunning turn of events"; "All eyes are on...").

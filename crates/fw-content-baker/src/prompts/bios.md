# Biographies bake prompt — `{culture_id}` × `{archetype_id}`

You are generating **short factual-summary biography templates** for
procedural fantasy footballers. The templates fill at runtime with the
player's deterministically-sampled name, age, birthplace, role, and
phenotype labels — so they must read naturally with placeholder
substitution.

## Slot placeholders

Use these tokens; the runtime substitutes them. Do not invent new ones:

- `{name}` — player display name (full)
- `{short_name}` — surname or commentator-shorthand
- `{age}` — integer
- `{birthplace}` — fictional city name
- `{role_short}` — "GK", "CB", "CM", etc.
- `{role_long}` — "goalkeeper", "centre-back", etc.
- `{phenotype_a}`, `{phenotype_b}` — phenotype labels from the catalog at
  `design/player-generation.md` (e.g. "Late Bloomer", "Reads the Game")

## Output format

Single JSON object:

```json
{
  "culture_id":   "{culture_id}",
  "archetype_id": "{archetype_id}",
  "templates": [
    { "body": "...", "tone": "positive | neutral | negative" },
    ...
  ]
}
```

Generate exactly `{templates_per_cell}` templates. Distribute tone roughly
`40% neutral / 35% positive / 25% negative` — a realistic mix.

## Hard constraints

- **Football vernacular only.** No anime tropes, no light-novel narration,
  no "destiny / kismet / awakening" framing. See `docs/design/ui-vocabulary.md`
  Category A for the full banned-term catalog.
- **Phenotype labels surface as labels, not gene numbers.** Never write
  "Genes:" or "DNA Score:" or "+5 Finishing".
- **No real player references.** No "the next Messi", no "compared to
  Beckenbauer". Comparisons are to fictional in-world legends only.
- **Length ceiling.** Each `body` field 40-400 characters. One paragraph max.
- **Cliché discipline.** Avoid the LLM tells documented in
  `crates/fw-content-baker/src/validators.rs`: "passionate about",
  "exceptional ability to", "rising star with bright future", "the world
  of football". The cliché detector will reject these by default.

## Tone register

British-football vernacular default for English. Reserved, observational,
specific — what a beat reporter writes after a midweek fixture, not what a
publicist writes for a transfer announcement.

Examples (do not copy verbatim — generate fresh variants):

- *Positive*: "A {role_long} who reads the game two passes ahead. {name}
  came up through the {birthplace} academy and has spent the last three
  seasons quietly building a reputation among scouts."
- *Neutral*: "{age}, {birthplace}-born. {name}'s contract runs to next
  summer; the club have made no public moves on extension."
- *Negative*: "Once tipped for higher tiers, {short_name}'s career has
  settled into a different rhythm. The first touch isn't what it was; the
  pace, by his own admission, less still."

## Audit

Same as `names.md` — dev review + manifest hash + reject on Category-A hit.

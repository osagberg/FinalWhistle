# Names bake prompt — `{culture_id}`

You are generating a corpus of **novel fictional player names** for a
procedural-fantasy football management simulator. The names must read as
culturally credible within the `{culture_id}` archetype WITHOUT being real
names of any current or historical professional footballer, public figure,
or notable family.

## Output format

Return a single JSON object validating against this schema:

```json
{
  "culture_id":  "{culture_id}",
  "first_names": ["...", ...],   // exactly {count_per_bank} entries
  "last_names":  ["...", ...]    // exactly {count_per_bank} entries
}
```

No prose outside the JSON. No code-fence markers around the JSON.

## Hard constraints

- **No real footballers.** Cross-check against the major-league rosters of
  the last 30 years. If a name matches, replace it.
- **No real public figures.** Politicians, celebrities, royalty, military
  figures — out.
- **Plausible phonotactics.** A `{culture_id}` reader should not flinch at
  any name. Adjacent-consonant clusters that don't occur in the source
  language are rejected; vowel harmony violations in agglutinative cultures
  are rejected.
- **Reasonable diversity.** First names should span ~12 starting letters,
  not 4. Last names should span 3-5 different formation patterns
  (patronymic, occupational, locative, descriptive, compound).
- **PEGI 12 safe.** No vulgar puns, no innuendo, no aggressive in-joke
  references.
- **Banned vocabulary.** None of the surfaces below may appear anywhere in
  the output — see `docs/design/ui-vocabulary.md` for the catalog:
  Category A.1 (mystical state nouns), A.4 (stigmatizing framings),
  A.5 (real-world place names).

## Tone

Match the `{culture_id}` archetype's naming register. Football-credible
defaults — names a commentator could say without sounding like they lost a
bet. Avoid fantasy-novel theatrics for real-world-coded cultures; for
fantasy archetypes (`fantasy-elvish`, `fantasy-dwarven`, `fantasy-orcish`)
preserve genre conventions while staying football-sayable.

## Audit

The committed corpus is the source of truth. The dev reviewing this output
will reject any fragment that violates the constraints. The bake manifest
records this prompt's BLAKE3 hash + the model_id + the seed used for any
sampling temperature variation.

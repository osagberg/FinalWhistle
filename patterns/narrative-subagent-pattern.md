# Pattern: Invoking the narrative-director subagent

When player-facing text is in scope, the `narrative-director` agent owns voice + vocabulary discipline.

## Why

Pillar 2 (careers that remember) and pillar 1 (procedural fantasy world) deliver value through text the player reads. That text MUST sound like football, not like Final Fantasy. Centralizing this with a single agent voice prevents tone drift across cultures, commentary banks, scout prose, and Tracery grammars.

## When to invoke `narrative-director`

- Event-template authoring for `fw-memory` readers (turning canonical events into prose)
- Tracery grammar work (commentary, club history, scout reports, manager flavor)
- Scout-prose template authoring (the disagreeing-biased-scout language)
- Vocabulary catalog updates in `docs/design/ui-vocabulary.md`
- Banned-terms lint failures — review the offending copy + propose a football-native alternative
- Salience-rule semantics (what *kind* of event is memorable, separate from `systems-designer`'s numeric weights)
- Any new player-facing copy surface before it ships
- Bake-time LLM prompt-template authoring (voice + tone constraints)

## When NOT to invoke

- Numeric salience weights / distribution shapes → `systems-designer`
- Implementation of memory readers in Rust → `gameplay-programmer`
- UI layout / where text appears → `ui-programmer`
- Architectural decisions about content-pack schema → `lead-programmer`

## Hand-off shape

When dispatching a content-narrative task, hand the `narrative-director` agent:

1. The task spec (from `/next` step 3)
2. Explicit vocabulary constraints (banned-terms catalog reference: `docs/design/ui-vocabulary.md`)
3. Voice / tone reference (3-5 lines of in-target prose for what we're aiming for)
4. Cardinality (how many variants per slot? ≥3 is the floor for player-facing copy)
5. Cross-references (related templates, related readers)

Example hand-off:

> Task: write 12 commentary variants for "shot saved by goalkeeper".
> Vocabulary: no capitalized state-nouns (banned list in docs/design/ui-vocabulary.md). Football-native; think Premier League TV commentary.
> Voice reference: "Got a hand to it. Conceding the corner." | "Tipped wide. He's earning his money tonight." | "Strong wrists. Set ball goes out."
> Cardinality: 12 variants, no near-duplicates.
> Cross-ref: existing variants in content/sources/grammars/match-commentary.tracery.json under #shotSaved#.

## Working norms

The agent reports under 250 words, leads with the proposed phrases, then justifies vocabulary choices.

## Banned-terms enforcement

- Run `scripts/fw banned-terms` mentally before claiming a phrase bank done.
- Sentinel exemption (sparingly, only for meta-references):

<!-- ui-lint:ignore-start -->
  ```
  // ui-lint:allow term="The Hush" reason="quoting fictional player's autobiography title" reviewer="narrative-director"
  ```
<!-- ui-lint:ignore-end -->

## Common failures

- **Tone drift:** copy starts football, ends fantasy ("the keeper *ascended* to claim the cross"). Read aloud once. Cut anything that doesn't sound like football.
- **Prose loops:** <3 variants per slot → players see the same line twice in 30 minutes. Hard floor of 3, target 5-8 per slot.
<!-- ui-lint:ignore-start -->
- **Numbers leaking:** "+5 Finishing" never ships. Surface as "looks sharper" / "his eye for goal is back" / "the chances are dropping for him."
<!-- ui-lint:ignore-end -->

## Cross-references

- `.claude/agents/narrative-director.md` — agent spec
- `docs/design/ui-vocabulary.md` — banned-terms catalog
- `Content/RULES.md` — RON authoring + sentinel exemption
- `bake-time-llm-content-pipeline.md` — for prompt-template author flow

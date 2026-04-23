---
paths:
  - "Assets/_Project/Scripts/AI/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# AI — manager archetypes and deterministic tactical behavior

Covers deterministic football decision systems: manager archetypes, tactical behavior trees, scouting evaluators, and AI-assisted bake-time tooling. Player builds do not run LLM inference.

## MUST

- Behavior-tree archetypes are authored in YAML or generated into YAML, then validated before runtime use.
- Runtime tactical AI must be deterministic for a given save/match seed.
- No runtime LLM calls in Player builds. LLM use is bake-time only through compiler tools.
- Every tactical evaluator declares its tick budget and target scale.
- Safety filters run on any bake-time AI output before it reaches content packs.

## SHOULD

- Keep BT runtime plain C# where possible so MatchSim can execute headless.
- Use ScriptableObject wrappers only for Unity-side authoring convenience.
- Log bake-time prompts, seeds, model version, and validation output to content-pack provenance.
- Add harness tests proving archetypes produce distinct, non-degenerate styles.

## AVOID

- Any LLM dependency in match-day, transfer, scout, or UI flows.
- Trusting LLM output unvalidated — parse, schema-check, lint, and reject malformed responses.
- Embedding API keys in code or ScriptableObjects.
- Per-player `Update()` with LINQ queries — batch tactical ticks through MatchSim.

## RATIONALE

Tactical AI must be debuggable, deterministic, and balance-harness reproducible. Runtime LLMs break all three constraints, so they belong only in offline content compilation.

## References

- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md) AI stack
- [CSharp/RULES.md](../../CSharp/RULES.md) async guidance

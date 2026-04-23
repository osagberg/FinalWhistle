---
paths:
  - "Assets/_Project/Scripts/AI/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# AI — runtime AI (behavior + LLM)

Covers both classical AI (BTs, GOAP, nav) and any LLM-backed runtime features.

## MUST

- Every AI agent declares a per-frame ms budget in its class XML doc (`// Budget: 0.3 ms/frame @ 100 agents`).
- LLM features support a local-model fallback path (Ollama / llama.cpp) — never hard-require a cloud endpoint.
- Prompt caching enabled on all Claude API calls. Cache the system prompt + large context blocks.
- Safety filters on any LLM output routed to the player: age gate, identity gate, banned-terms list.
- Depends on `Core` + `Stats` + `Characters`. No UI.

## SHOULD

- Behavior trees as ScriptableObjects; edit in an Editor tool, not code.
- Navmesh agents pooled and reused — re-baking navmesh at runtime only for intentional streaming seams.
- LLM calls batched where possible; async with UniTask and a cancellation token scoped to the scene.
- Log every LLM prompt + response to a gitignored dev-log directory for replay debugging.

## AVOID

- Blocking the main thread on LLM calls. Always async; show a "thinking" indicator if the player waits.
- Trusting LLM output unvalidated — parse, schema-check, and reject malformed responses.
- Embedding API keys in code or SOs. Use `Secrets/` (gitignored) + `ISecretProvider` interface.
- Per-agent `Update()` with LINQ queries — batch agent ticks via a scheduler.

## RATIONALE

Runtime AI is the first budget-killer. Declaring a ms target per agent makes regressions detectable before they're a Steam review about stutter. LLM safety filters are a content-floor invariant — a model can be coaxed into violations; the filter is the backstop.

## References

- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md) AI stack
- [CSharp/RULES.md](../../CSharp/RULES.md) async guidance

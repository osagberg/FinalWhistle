---
paths:
  - "Assets/_Project/Scripts/Dialog/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Dialog — narrative runner

Ink only. No hardcoded dialogue strings in `.cs`. Every user-visible line routes through a localized key.

## MUST

- All dialogue authored in `.ink`; runtime runs `ink-unity-integration`'s `Story`.
- Zero `"quoted literal dialogue"` in C#. If you're tempted, write an Ink stitch and reference it.
- Scene transitions via `SceneDirector` — never `SceneManager.LoadScene` from dialog code directly.
- Localization-aware: every player-facing string has a locale key; default locale is `en`.
- Depends on `Core` + `Stats` + `Characters`. No UI (UI subscribes to dialog events).

## SHOULD

- Compile Ink → JSON at build time; never at runtime in Player builds.
- Use typed variable bridges between Ink and C# (`story.variablesState["shame"]`) — wrap in a `DialogContext`.
- Emit `DialogLineEvent`, `DialogChoiceEvent`, `DialogEndEvent` — UI renders, Dialog doesn't.
- Keep character voice bibles in `.claude/agents/<character>.md`; Ink references the character's voice.

## AVOID

- `textField.text = "Hello, player."` inside Dialog code — UI is the consumer, not Dialog.
- `SceneManager.LoadScene("Library")` — route through `SceneDirector.Go(sceneId, transitionSO)`.
- Runtime Ink compilation — it's slow, and a compile error should fail the build, not the session.
- Conditional content by `string.Contains` — use typed Ink tags and C# enum dispatch.

## RATIONALE

Decoupling narrative authoring (Ink) from rendering (UI) from navigation (SceneDirector) means each team member (or each you-on-different-days) works in the right tool. Localization-from-day-one is non-negotiable: retrofitting it after shipping is painful and expensive.

## References

- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md) Narrative stack
- [UI/RULES.md](../UI/RULES.md)

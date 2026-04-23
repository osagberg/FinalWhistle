---
paths:
  - "Assets/_Project/Scripts/UI/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# UI — UI Toolkit management screens and overlays

UI renders state; UI does not OWN state. Gameplay systems emit events; UI listens.

## MUST

- No direct references to gameplay systems from UI scripts. UI binds to viewmodels and dispatches commands.
- State changes reach UI via events/signals or observable viewmodels — `PlayerReportChangedEvent`, `FixtureStateChangedEvent`, etc.
- Auto-scale for accessibility: text has a `MinReadableSize`, large-text mode, and tested responsive breakpoints.
- Interaction feedback fires within 150 ms of input — animate acknowledgement even if the action is async.
- Localization via a `LocalizedText` component backed by locale keys, not hardcoded strings.
- Dense management screens must support keyboard and gamepad focus paths, not mouse-only operation.

## SHOULD

- One UI screen = one UXML/USS surface under `UI/Screens/` with a matching viewmodel.
- Use UI Toolkit for management screens and match overlays. uGUI is fallback only for documented UI Toolkit blockers.
- Virtualize roster, transfer, fixture, and event-ledger lists from day one.
- Keep common workflows within two interactions where practical: squad, tactics, next fixture, player report, inbox.

## AVOID

- `PlayerStats.Instance` or direct roster/sim singletons in UI — subscribe/bind to viewmodels.
- `Update()` that polls gameplay state — listen to the event instead.
- Raw `new Color(1,0,0)` literals — use a `UIThemeSO` palette.
- Mixing UI Toolkit and uGUI in the same screen unless a documented fallback requires it.

## RATIONALE

UI is a load-bearing product differentiator because the project is explicitly anti-FM26-regression. Event-driven UI survives refactors, virtualized lists keep dense screens fast, and the 150 ms feedback rule protects perceived responsiveness.

## References

- [Stats/RULES.md](../Stats/RULES.md) (UI subscribes to stat/progression events)
- [design/ui-vocabulary.md](../../../../../design/ui-vocabulary.md)

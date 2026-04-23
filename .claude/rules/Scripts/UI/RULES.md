---
paths:
  - "Assets/_Project/Scripts/UI/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# UI — UGUI / UIToolkit / presentation

UI renders state; UI does not OWN state. Gameplay systems emit events; UI listens.

## MUST

- No direct references to gameplay systems from UI scripts. UI `using` lists `Core` and event types only.
- State changes reach UI via events/signals — `PlayerStatsChangedEvent`, `DialogLineEvent`, etc.
- Auto-scale for accessibility: text has a `MinReadableSize`, canvases use `CanvasScaler.UIScaleMode.ScaleWithScreenSize`.
- Interaction feedback fires within 150 ms of input — animate acknowledgement even if the action is async.
- Localization via a `LocalizedText` component backed by locale keys, not hardcoded strings.

## SHOULD

- One UI screen = one scene or one prefab under `UI/Screens/` — composable, testable in isolation.
- Use `UIDocument` (UIToolkit) for non-diegetic UI (menus, HUD); `UGUI` for diegetic (in-world signs, books).
- Pool frequent-spawn UI elements (damage numbers, notifications) — don't `Instantiate` per-tick.
- Cache `TMP_Text.text` sets behind a dirty flag — `TextMeshPro` setter is non-trivial.

## AVOID

- `PlayerStats.Instance.HP` in UI — subscribe to the event, cache locally.
- `Update()` that polls gameplay state — listen to the event instead.
- Raw `new Color(1,0,0)` literals — use a `UIThemeSO` palette.
- Coupling UI show/hide to `SetActive(false)` trees — prefer CanvasGroup alpha + interactable toggles.

## RATIONALE

UI is the first thing to break when underlying systems change. Event-driven UI survives refactors — polling UI breaks. The 150 ms feedback rule is a perceived-responsiveness invariant; async operations that skip the acknowledgement feel laggy even when they're fast.

## References

- [Dialog/RULES.md](../Dialog/RULES.md) (UI subscribes to Dialog events)
- [Stats/RULES.md](../Stats/RULES.md) (UI subscribes to Stats events)

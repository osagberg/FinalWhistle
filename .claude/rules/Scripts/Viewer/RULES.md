---
paths:
  - "UnityProject/Assets/_Project/Scripts/Viewer/**"
  - "UnityProject/Assets/_Project/Viewer/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Viewer — stylized 2D semantic cinema

Viewer code renders MatchSim events through the 2D manga-broadcast grammar. It is presentation only.

## MUST

- Treat MatchSim events as read-only input; viewer code cannot author, modify, or correct canonical match state.
- Map events into the 7 shot-type vocabulary from `design/semantic-cinema.md`.
- Keep Month-3 scope to 3 shot types until the legibility gate passes.
- Verify visual work with runtime evidence: Unity MCP screenshot/capture or exported match-replay clip.
- Respect `design/ui-vocabulary.md`; no capitalized state nouns in overlays or commentary.

## SHOULD

- Keep camera/shot recipes data-driven with ScriptableObjects or content-pack data.
- Use stakes and memory as modulation inputs, not new bespoke shot types.
- Include reduce-motion paths for impact frames, flashes, shakes, and hard panel transitions.
- Log selected shot type + source MatchEvent for replay debugging.

## AVOID

- Per-signature unique cinematics before all 7 base shot types work.
- Decorative 3D dependencies in MVP viewer code.
- UI Toolkit overlays that cover critical match action.
- Hardcoded text strings outside localization/content tables.

## References

- [design/semantic-cinema.md](../../../../../design/semantic-cinema.md)
- [design/ui-vocabulary.md](../../../../../design/ui-vocabulary.md)
- [design/month-3-vertical-slice.md](../../../../../design/month-3-vertical-slice.md)

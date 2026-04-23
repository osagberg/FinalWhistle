---
paths:
  - "UnityProject/Assets/_Project/Scripts/Signatures/**"
  - "UnityProject/Assets/_Project/Signatures/**"
  - "content-packs/**/signatures/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Signatures — football-readable player actions

Signatures are authored football behaviors, not powers.

## MUST

- Follow `design/signatures.md`: behavior + trigger conditions + sim bias + execution modifier + presentation recipe + counterplay.
- Keep player-facing names football-native: "Looks for early crosses", not power-name branding.
- Implement sim effects inside MatchSim-compatible pure C# logic; Unity-side code only presents the result.
- Include counterplay data for every signature.
- Test that each signature changes behavior measurably without becoming always-correct.

## SHOULD

- Author signature definitions in text-first content data before creating ScriptableObject wrappers.
- Keep Month-3 signatures active by setup data; latent unlock lifecycle starts in Phase 4.
- Map presentation recipes into the 7 semantic-cinema shot types.
- Add harness cases for signature vs counterplay.

## AVOID

- Composable signature atoms before all 24 MVP signatures are authored.
- Per-signature bespoke cinematics.
- Mystical vocabulary in UI, commentary, achievements, or tutorials.
- Hardcoded thresholds in C#.

## References

- [design/signatures.md](../../../../../design/signatures.md)
- [design/breakthrough-moments.md](../../../../../design/breakthrough-moments.md)
- [design/semantic-cinema.md](../../../../../design/semantic-cinema.md)

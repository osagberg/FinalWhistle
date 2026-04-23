---
paths:
  - "UnityProject/Assets/_Project/Scripts/Players/**"
  - "UnityProject/Assets/_Project/Players/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Players — identity packets, development, and roster state

Players are data-backed footballers, not generic RPG characters.

## MUST

- Key players by content-pack-qualified stable IDs; never by display name, GameObject name, or roster index.
- Keep internal generation fields invisible in player-facing UI. Use phenotype labels and football-readable descriptions.
- Support youth players. Do not add blanket age >= 18 validation; use PEGI-safe content rules instead.
- Put signature affinities, pressure response, development hooks, scout labels, and commentary handles in the Identity Packet or its runtime projection.
- Route player-facing text through localization/content tables and `design/ui-vocabulary.md`.

## SHOULD

- Keep runtime player state as plain C# records where possible; Unity components are view/authoring bridges.
- Serialize save references by stable ID plus content-pack version.
- Treat Coaching Lineage fields as seeded data only until post-EA surfacing.
- Add validation for impossible player records: duplicate IDs, missing role family, invalid age range, invalid signature candidate.

## AVOID

- "Genes", "bloodline", or mystical trigger names in UI.
- Player development controlled by one giant stat blob.
- Randomly regenerated IDs after content-compiler reruns.
- Hardcoded player names in code.

## References

- [design/player-generation.md](../../../../../design/player-generation.md)
- [design/signatures.md](../../../../../design/signatures.md)
- [design/ui-vocabulary.md](../../../../../design/ui-vocabulary.md)

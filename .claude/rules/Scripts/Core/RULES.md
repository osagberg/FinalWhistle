---
paths:
  - "Assets/_Project/Scripts/Core/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Core — base utility layer

Foundation asmdef. Everything depends UP into Core; Core depends on NOTHING.

## MUST

- No dependencies on Stats, Players, Memory, MatchSim, Viewer, UI, AI, Debug, or Editor.
- No `using UnityEngine.UI`, `UnityEngine.AI`, `Addressables` imports in pure-data types. MonoBehaviour bridges are allowed but must live in dedicated `*Behaviour.cs` files.
- Zero-allocation in hot paths — pre-allocate buffers, reuse via `Clear()` not `new`.
- Thread-safe utilities must document their thread contract (top-of-class XML doc).
- Public API changes require a migration note in `CHANGELOG.md` under the current phase.

## SHOULD

- Prefer `struct` over `class` for small value types (≤16 bytes, no identity).
- Use `readonly struct` where immutability is intended.
- Expose primitives, not framework types, across asmdef boundaries.
- Keep files under 200 lines; split by concern when they grow.

## AVOID

- Static mutable state. If you need a service, use a `ScriptableObject` registry injected at bootstrap.
- LINQ in `Update`, `FixedUpdate`, or any per-frame path — allocates enumerators.
- `string` concatenation in hot paths — use `StringBuilder` or `Span<char>`.
- Premature generics. Three call sites is the bar, not one.

## RATIONALE

Core is the bedrock. Every other asmdef transitively references it, so a bad dependency here poisons the whole graph. Zero-alloc discipline here keeps the 60 FPS budget intact at the base of the stack.

## References

- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md) §7 Assembly graph
- [CSharp/RULES.md](../../CSharp/RULES.md) for struct/class guidance

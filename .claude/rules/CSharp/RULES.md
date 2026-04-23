---
paths:
  - "Assets/_Project/**/*.cs"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# C# — language idioms for the whole project

Unity-flavored modern C#. Async via UniTask. Struct for data; class for identity.

## MUST

- Async work uses `UniTask` / `UniTaskVoid`, not `Task` / `Task<T>`. Reason: zero-alloc, PlayerLoop-integrated, cancellation-aware.
- `async void` is banned EXCEPT for Unity event handlers (`UnityEvent`, `Button.onClick`). Everywhere else: `UniTaskVoid` or `UniTask`.
- Pass `CancellationToken` into every async method — plumb it from the scene / component lifetime.
- `struct` for data-sized types (≤16 bytes, no reference semantics): `Vector2Int`, stat snapshots, small IDs.
- `class` for anything with identity, lifetime, or mutability beyond the struct-sized frontier.

## SHOULD

- `readonly struct` over `struct` when immutable — enables compiler optimizations, prevents defensive copies.
- Use expression-bodied members (`=> ...`) for trivial getters/setters.
- `nameof()` over string literals for member names.
- `record` types for pure-data DTOs only. Prefer `readonly struct` where boxing matters.
- File-scoped namespaces (`namespace X;`) — Unity 6 supports them; keeps indentation cheaper.

## AVOID

- `async Task` inside gameplay code. `UniTask` for all engine-integrated async.
- `Thread.Sleep`, `Task.Delay` — use `UniTask.Delay(ms, cancellationToken: ct)`.
- `async void` event handlers that aren't actually Unity event handlers.
- Mutable structs with mutator methods. Either `readonly struct` + `With*` methods or a class.
- `object` boxing in hot paths (`Dictionary<int, object>` vs typed generic dict).

## RATIONALE

UniTask exists because `Task` is the wrong abstraction for a game loop: it allocates, it crosses threads by default, and it's hard to cancel per-scene. The struct/class guidance is an allocation-budget choice — games die at 10,000 small-object GC pressure, not at 100 big-object.

## References

- [Scripts/Core/RULES.md](../Scripts/Core/RULES.md) zero-alloc hot paths
- [TECH_APPROACH.md](../../../TECH_APPROACH.md) UniTask dependency

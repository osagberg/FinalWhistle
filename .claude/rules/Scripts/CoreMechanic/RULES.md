---
paths:
  - "Assets/_Project/Scripts/CoreMechanic/**"
  - "Assets/_Project/Scripts/<RenamedMechanic>/**"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# `CoreMechanic` — project-specific signature gameplay system

> **BOOTSTRAP STEP:** during `/bootstrap`, rename this folder from `CoreMechanic/` to your project's
> specific mechanic name (e.g., `Combat/`, `Platforming/`, `Stealth/`, `Deckbuilding/`, `Crafting/`).
> Update the `paths:` glob above to match. See
> [unity/scripts-folder-structure.md](../../../../../unity/scripts-folder-structure.md) §`CoreMechanic`.

The signature gameplay system that makes THIS game different. Everything else is table stakes.

## MUST

- One asmdef for the whole mechanic. No splitting across assemblies until profiled pain exists.
- All moves / abilities / interactions defined as SOs (e.g., `CoreMechanicMoveSO` or your renamed equivalent). Code executes; SOs declare.
- Depends on `Core` + `Stats` + `Characters` + `Outfits`. No UI, no Dialog.
- Replay-safe: every mechanic state transition emits a typed event; logs rebuild state from events.
- Deterministic given the same input seed — no `Random.value` without an injected `IRng`.

## SHOULD

- Expose a testable core: the mechanic's resolution function is plain C#, callable from editor tests.
- Gate side effects (FX, SFX, UI notifications) behind events the mechanic emits — don't call into UI directly.
- Keep per-frame allocations at zero; this is a hot path if the mechanic is central to gameplay.
- Profile early: set a target ms/frame budget for the mechanic and record it in `TECH_APPROACH.md`.

## AVOID

- Coroutines for multi-step mechanic logic — use UniTask + async/await.
- Hardcoded timing constants — use `MoveSO.Duration`, etc.
- Reflection-based dispatch (`Invoke("MoveName")`) — switch on a typed enum or SO reference.
- Leaking the mechanic's internal state onto the Character class — compose, don't merge.

## RATIONALE

The core mechanic is what the player talks about. It deserves the tightest API, the lowest allocation
budget, and the sharpest test coverage of any system. Keep it isolated enough that replacing its
implementation is possible without touching every other asmdef.

## References

- [TECH_APPROACH.md](../../../../../TECH_APPROACH.md)
- [unity/scripts-folder-structure.md](../../../../../unity/scripts-folder-structure.md)

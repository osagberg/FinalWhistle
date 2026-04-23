---
name: state-dump
description: Dump live Unity game state to JSON (player stats, scene hierarchy summary, active coroutines, event-bus state, component data). Invoke after runtime changes and before asserting "done" on behavioral work — proves the feature actually did what you claim. Triggers on "dump state", "what's in the scene right now", "show me current game state", "state dump".
triggers:
  - dump state
  - state dump
  - current game state
  - what is in the scene
  - show runtime state
---

# State Dump — runtime inspection

Serialize current Play-Mode game state to JSON so Claude can read what the game *actually* is, not what the code *claims* it should be. Pairs with [unity-check L2](../unity-check/SKILL.md) — L2 proves "doesn't crash", state-dump proves "did the right thing".

## When to invoke

| Situation | Reason |
|---|---|
| After implementing a runtime behavior change | Verify state transitions produced the expected values |
| Before `/done` on a gameplay-logic task | Evidence of correct behavior beyond "compiles + no exceptions" |
| Debugging a reproducible bug in Play Mode | Inspect live values without spamming `Debug.Log` |
| Writing a new test case | Harvest realistic state as a fixture |

Skip for pure art/shader/UI work — use unity-check L3 instead.

## Architecture

```
[Claude] ──┐
           ├─ execute_menu_item("FinalWhistle/Debug/Dump State")
           │     │
           │     ▼
           │  McpRemoteControl.DumpState()            ← Editor menu entry
           │     │
           │     ├─ walks active scene hierarchy
           │     ├─ collects every MonoBehaviour : IDumpable
           │     ├─ calls DumpState() on each
           │     └─ writes Library/StateDump.json
           │
           └─ Read Library/StateDump.json             ← Claude reads back
```

`McpRemoteControl` also exposes god-mode helpers (`SetHealth`, `TeleportTo`, `GodMode`, etc.) for driving state during a run. See [templates/McpRemoteControl.cs](templates/McpRemoteControl.cs).

## Procedure — dump-only flow

1. Ensure Play Mode is running. If not: `manage_editor(action="play")`, wait 3s.
2. `execute_menu_item(menu_path="FinalWhistle/Debug/Dump State")`.
3. Wait ~500ms for file write.
4. Read `unity-project/Library/StateDump.json` (or use helper script [scripts/dump-and-read.sh](scripts/dump-and-read.sh)).
5. Parse and inspect relevant sections. Do NOT slurp the full JSON into context if only one component is relevant — JSON is structured, read the needle.

## Procedure — dump + drive flow (behavioral verification)

Use when you need to prove "when I do X, state becomes Y".

1. `manage_editor(action="play")`, wait 3s.
2. `execute_menu_item("FinalWhistle/Debug/Dump State")` → baseline snapshot.
3. Drive state via god-mode commands:
   - `execute_menu_item("FinalWhistle/Debug/God Mode")` — toggle invincibility
   - `execute_menu_item("FinalWhistle/Debug/Set Health 50")` — custom — you add these to McpRemoteControl as needed
   - Or set `McpRemoteControl.PendingCommand` via `manage_components` for parameterized calls
4. `execute_menu_item("FinalWhistle/Debug/Dump State")` → post-action snapshot.
5. Diff baseline vs post. Report the delta.
6. `manage_editor(action="stop")`.

## Output JSON shape

```json
{
  "timestamp": "2026-04-21T10:15:30Z",
  "sceneName": "Main",
  "playMode": true,
  "hierarchy": {
    "rootObjects": ["Player", "LevelManager", "UICanvas"],
    "activeCount": 24,
    "totalCount": 31
  },
  "components": {
    "PlayerStats": { "hp": 100, "dread": 45, "shame": 12 },
    "LevelManager": { "currentPhase": "Combat", "elapsedSeconds": 47.3 },
    "QuestLog": { "activeQuests": ["intro", "findKey"] }
  },
  "eventBus": {
    "subscriberCounts": { "OnPlayerDied": 3, "OnQuestComplete": 2 }
  },
  "coroutines": {
    "runningCount": 4,
    "owners": ["PlayerController", "LevelManager"]
  }
}
```

Only components implementing `IDumpable` (see [templates/IDumpable.cs](templates/IDumpable.cs)) appear under `components`. Everything else is structural metadata.

## Extending — new dumpable component

1. Implement `IDumpable` on the MonoBehaviour:
   ```csharp
   public object DumpState() => new { hp, maxHp, lastDamageSource = lastDamageSource?.name };
   ```
2. Use an anonymous object — `JsonUtility` doesn't like anonymous types but Newtonsoft (which `McpRemoteControl` uses for `DumpState`) handles them fine.
3. That's it. The scanner picks it up automatically next dump.

## Extending — new god-mode command

1. Open [templates/McpRemoteControl.cs](templates/McpRemoteControl.cs) (copied into `Assets/_Project/Editor/` at bootstrap).
2. Add a `[MenuItem("FinalWhistle/Debug/<YourCommand>")]` static method.
3. Inside, find the relevant runtime manager (e.g. `GameManager.Instance`) and call into it.
4. Guard anything touching scene state with `if (!Application.isPlaying) return;`.
5. Call via `execute_menu_item("FinalWhistle/Debug/<YourCommand>")`.

## Safety / constraints

- McpRemoteControl is `#if UNITY_EDITOR || DEVELOPMENT_BUILD` — never ships in Release builds.
- Dump output lives in `Library/` which is gitignored by convention — no accidental commits of live state.
- God-mode commands mutate scene state; always dump before AND after to keep reasoning honest.
- Don't rely on god-mode for automated test assertions — use EditMode/PlayMode tests for permanent checks. State dump is exploratory.

## See also

- [templates/McpRemoteControl.cs](templates/McpRemoteControl.cs) — menu entries + god-mode toolkit
- [templates/IDumpable.cs](templates/IDumpable.cs) — opt-in interface for components
- [scripts/dump-and-read.sh](scripts/dump-and-read.sh) — one-shot dump + pretty-print
- [../unity-check/SKILL.md](../unity-check/SKILL.md) — L2 runtime verification (pair with dump)

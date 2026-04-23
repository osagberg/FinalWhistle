---
name: unity-check
description: Three-level Unity verification (L1 compile / L2 runtime / L3 visual). Invoke at `/done` on Phase 3+ tasks, pre-commit when Unity code changed, or explicitly via `/unity-check`. Triggers on phrases like "verify Unity", "runtime check", "screenshot verify", "compile check", "does it actually run".
triggers:
  - verify unity
  - unity check
  - runtime verify
  - play mode check
  - compile check
  - screenshot verify
  - does it run
---

# Unity Check — three-level verification

Verify a Unity change actually works before claiming done. Three escalating levels: compile, runtime, visual. Each level is cheap-to-expensive in tokens and time; stop at the first failure, fix, restart from L1.

**Golden rule:** a green type check is NOT verification. "Compiles" does not mean "runs". "Runs" does not mean "looks right". Run the level that matches the claim you're about to make.

## When to invoke

| Change shape | Minimum level |
|---|---|
| Pure C# refactor, no scene/asset touched | L1 |
| Logic change to runtime behavior | L2 |
| UI, camera, shader, animation, VFX, new scene | L3 |
| `/done` on a Phase 3+ task | L1 mandatory, L2 if runtime-affecting, L3 if visual |
| Pre-commit on a PR that touches `Assets/` | L2 minimum |

## Integration — Unity MCP

Assumes `com.coplaydev.unity-mcp` is running and the handshake succeeded (`claude mcp list` shows unity-mcp green). Tool names used below match that package. If MCP is down, fall back to batchmode (see L1 fallback).

Batchmode gotchas: [/Users/vibelogic/dev/blueprint/unity/batchmode-gotchas.md](/Users/vibelogic/dev/blueprint/unity/batchmode-gotchas.md) — read before running headless Unity.

---

## L1 — Compile verification (~1–2K tokens, seconds)

Confirm no C# compile errors, no missing script references, no broken asmdef refs.

### Procedure (MCP path — preferred)

1. Call `read_console(type="Error", count=50)` via unity-mcp.
2. Filter for `error CS` (C# compile) and `error ` (engine errors including missing scripts).
3. If zero matches → L1 pass, proceed to L2 if warranted.
4. If matches → L1 fail. Return first 10 errors with file+line. Fix. Re-run L1.

### Procedure (batchmode fallback — when MCP is down or project is fresh)

1. Run:
   ```bash
   "/Applications/Unity/Hub/Editor/<version>/Unity.app/Contents/MacOS/Unity" \
     -batchmode -quit -nographics \
     -projectPath "$PROJECT_ROOT/unity-project" \
     -executeMethod {{PROJECT_NAME}}.Editor.Verification.VerificationReport.Run \
     -logFile /tmp/unity-l1.log
   ```
2. After exit, read `Library/VerificationReport.json` (see [templates/VerificationReport.cs](templates/VerificationReport.cs)).
3. Scan log for `error CS` regardless of exit code (Unity can exit 0 with CS errors in background assemblies).

### Pass criteria

- `compileErrorCount == 0`
- `missingScripts` empty
- `brokenAsmdefRefs` empty

### Failure output format

```
L1 FAIL — 3 compile errors
  Assets/_Project/Scripts/Player/Health.cs:42  CS0103  'currentHp' not found
  Assets/_Project/Scripts/Player/Health.cs:47  CS1061  'Stats' has no 'MaxHp'
  Assets/_Project/Scripts/UI/HUD.cs:18         CS0246  'HealthBar' namespace missing
Remediation: check recent edits to Health.cs and Stats SO — likely a rename drift.
```

---

## L2 — Runtime verification (~3–5K tokens, ~10–20s)

Confirm Play Mode enters, initializes, and runs N seconds without runtime errors / null refs / uncaught exceptions.

### Procedure

1. `manage_editor(action="play")` — enter Play Mode.
2. Wait 3 seconds for initialization (Awake → Start → first Update cycles).
3. Optional: exercise one entry point (see [state-dump](../state-dump/SKILL.md) for god-mode commands like `MCP/StartGame`).
4. Wait `monitor_seconds` (default 5; bump to 30 for soak-style runs).
5. `read_console(types=["Error", "Exception"])` — capture runtime errors.
6. `manage_editor(action="stop")` — exit Play Mode cleanly.
7. If errors found → L2 fail, return captured errors + first stack frame each. Restart from L1 after fix.

### Pass criteria

- Play Mode entered without errors
- Zero `Error` or `Exception` console lines during the monitor window
- Play Mode exited cleanly (no hang, no crash)

### Common failure signatures

| Signature | Likely cause |
|---|---|
| `NullReferenceException` in `Awake`/`Start` | Serialized field unassigned in prefab/scene |
| `MissingReferenceException` | Scene has a stale script GUID |
| `UnityException: Find can only be called from the main thread` | Async code calling Unity API off-thread |
| `NullReferenceException` at frame N > 60 | Late-init race — Start order dependency |

### Failure output format

```
L2 FAIL — 1 runtime error after 5s in Play Mode
  NullReferenceException at PlayerController.Update() Assets/_Project/Scripts/Player/PlayerController.cs:67
  First stack frame: PlayerController.Update () (at Assets/_Project/Scripts/Player/PlayerController.cs:67)
Remediation: `inputReader` serialized field is null — check PlayerPrefab in scene.
```

---

## L3 — Visual verification (~10–300K tokens depending on screenshot count)

Capture scene screenshots in Play Mode, AI-inspect for obvious visual regressions.

Preconditions (check once per project; fails loud if not met):
- Main Camera tagged `MainCamera`
- Canvas `Render Mode` set to `Screen Space - Camera` or `Overlay` with Main Camera assigned
- Scene lighting baked or real-time sane (no purple skybox)

### Procedure

1. `manage_editor(action="play")`.
2. Wait 3s for initialization.
3. Capture screen:
   - `manage_graphics(action="screenshot", include_image=true, max_resolution=640)` — game view
   - Optional: multiple camera angles via `manage_camera(action="screenshot", camera_name="<name>")`
4. Analyze each screenshot for:
   - Pink/magenta surfaces → missing shader
   - Solid black/white fullscreen → camera pointing wrong way or not clearing
   - `[No Texture]` placeholder art → asset not assigned
   - UI text boxes overflowing, overlapping, clipped
   - Missing character mesh → rig/prefab reference broken
   - Z-fighting stripes → co-planar geometry
5. `read_console(types=["Error", "Warning"])` — catch "shader not supported", "texture format unsupported" etc.
6. `manage_editor(action="stop")`.

### Pass criteria

- No pink shaders visible
- No fullscreen anomaly (black/white wash)
- UI renders inside safe area
- At least one recognizable gameplay element visible (character, environment, HUD)

### Failure output format

```
L3 FAIL — visual anomaly detected
  Screenshot: /tmp/unity-l3-main.png
  Observation: Character model renders with pink/magenta material on torso.
  Likely cause: lilToon shader not assigned after material copy — re-link shader on Body mesh.
  Console: "Shader 'Hidden/InternalErrorShader' used on material 'Wren_Body'"
```

---

## Composite flow — when invoked with no level argument

Default: L1 → if pass run L2 → if pass run L3 (unless change is code-only refactor, then stop at L2).

```
unity-check (default)
  ├─ L1 compile
  │   └─ FAIL → report, stop
  ├─ L2 runtime (if change is runtime-affecting)
  │   └─ FAIL → report, stop
  └─ L3 visual (if change touches UI/scene/shader/animation)
      └─ FAIL → report, stop
  └─ PASS → emit consolidated green report
```

## Explicit level requests

User can force a level: `/unity-check L1`, `/unity-check L2`, `/unity-check L3`. Run only the requested level, don't escalate.

## Consolidated green report format

```
unity-check PASS
  L1 compile       0 errors, 0 missing scripts, 0 broken asmdefs
  L2 runtime       5s Play Mode, 0 errors, 0 exceptions
  L3 visual        2 screenshots captured, no anomalies detected
  evidence: /tmp/unity-l3-main.png, Library/VerificationReport.json
```

## See also

- [templates/VerificationReport.cs](templates/VerificationReport.cs) — batchmode report emitter (copied into `Assets/_Project/Editor/` at bootstrap)
- [../state-dump/SKILL.md](../state-dump/SKILL.md) — runtime state dump (pairs with L2 when verifying behavior, not just "didn't crash")
- [/Users/vibelogic/dev/blueprint/unity/batchmode-gotchas.md](/Users/vibelogic/dev/blueprint/unity/batchmode-gotchas.md) — batchmode edge cases

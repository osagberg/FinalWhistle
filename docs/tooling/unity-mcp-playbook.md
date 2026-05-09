---
description: Unity AI Assistant MCP integration playbook for Final Whistle. Architecture, security/entitlement model, complete tool inventory, custom tool registration, AssistantApi usage, capability comparison vs CoplayDev unity-mcp, and FW-specific menu-item proposals. Authored 2026-05-08 against com.unity.ai.assistant 2.7.0-pre.3 by general-purpose research agent.
---

# Unity AI Assistant MCP — Integration Playbook

> Research compiled 2026-05-09 against `com.unity.ai.assistant@2.7.0-pre.3` (fingerprint `198d71476a35`), shipped 2026-05-06. All file paths are inside `unity-project/Library/PackageCache/com.unity.ai.assistant@198d71476a35/`.

This is the source-of-truth deep-research doc that informed [ADR-0011](../../design/adr/adr-0011-unity-ai-assistant-mcp-migration.md) + [docs/tooling/unity-mcp-routing.md](unity-mcp-routing.md). Read this when you need detail beyond the routing table — full tool inventory, custom-tool registration patterns, entitlement model internals.

## Server names in `.mcp.json` (this project)

- **`UnityAIAssistant`** — official Unity AI Assistant MCP (stdio relay, primary).
- **`UnityMCP`** — CoplayDev unity-mcp (HTTP :8080, fallback).

## 1. Architecture

Unity acts as the MCP **server**; external AI clients (Claude Code, Cursor, Windsurf) act as MCP **clients**. They never talk directly. A separate **relay binary** sits between them, speaking MCP's stdio transport on the client side and Unity's local IPC on the server side.

```
AI Client (Claude Code, Cursor, etc.)
    |
    | MCP protocol (stdio)
    |
Relay binary (~/.unity/relay/) with --mcp flag
    |
    | IPC (named pipe / Unix socket)
    |
Unity Editor (MCP Bridge)
    |
    | McpToolRegistry
    |
Registered tools (built-in + custom)
```

### Relay binary, per platform

| Platform | Path |
|---|---|
| macOS Apple Silicon | `~/.unity/relay/relay_mac_arm64.app/Contents/MacOS/relay_mac_arm64` |
| macOS Intel | `~/.unity/relay/relay_mac_x64.app/Contents/MacOS/relay_mac_x64` |
| Windows | `%USERPROFILE%\.unity\relay\relay_win.exe` |
| Linux | `~/.unity/relay/relay_linux` |

The relay must be invoked with `--mcp`. Optional targeting flags `--project-path <path>` and `--instance-id <pid>` (with env-var equivalents `UNITY_PROJECT_PATH` and `UNITY_INSTANCE_ID`) bind it to a specific Editor when multiple are running. CLI arg wins over env var. The 2.7.0-pre.3 changelog flags a fix here: "UNITY_PROJECT_PATH / --project-path targeting for the Unity MCP server now matches the project root as documented." Pre-`pre.3` builds had a path-resolution bug.

### Bridge boot

When the Unity Editor loads with the `com.unity.ai.assistant` package present, `Unity.AI.MCP.Editor.Bridge` auto-starts via `[InitializeOnLoad]` and opens an IPC listener (named pipe on Windows, Unix socket on macOS/Linux — see `Modules/Unity.AI.MCP.Editor/Connection/NamedPipeListener.cs` and `UnixSocketListener.cs`). It also installs the relay binary into `~/.unity/relay/` from `RelayApp~/` on first run.

Newline-delimited JSON over the IPC stream. Protocol version 2.0. There is no fixed TCP port — connections are file-system-bound (Unix socket path or named-pipe name).

The relay is started by the *MCP client*, not by Unity. Claude Code reads `.mcp.json`, spawns `relay_mac_arm64 --mcp` over stdio, and the relay opens the local socket to whichever Unity instance matches the targeting hints. If no Unity is up, the relay returns an error and the MCP client retries.

### Bridge state UI

`Edit > Project Settings > AI > Unity MCP Server` shows Running/Stopped, connected/pending clients, the per-tool enable list, and integration auto-config. **Note:** `AI > MCP Client` is a separate page that subscribes Unity Assistant to outbound MCP servers — easy to confuse, do not.

## 2. Security and entitlement model

Two connection classes, two trust models:

- **AI Gateway connections** — automatic trust. Unity Assistant connecting to its own backend bypasses the approval prompt.
- **Direct connections** — every external MCP client (Claude Code, Cursor) must be explicitly approved by the user the first time. Approved clients auto-reconnect; the approval is keyed by a signed/hashed identity of the client binary.

### Per-binary-signature re-approval

`Modules/Unity.AI.MCP.Editor/Settings/UI/ConnectionDetailsView.cs` handles this. When the exact identity record (binary hash + signature) doesn't match but the publisher matches a previously-accepted record, the dialog shows "Previously approved (application may have been updated)" and asks for re-approval. **Practical takeaway:** every Claude Code update bumps the binary hash and fires a fresh approval dialog.

### Connection caps — `ConnectionCensus.Policy`

```csharp
record ConnectionPolicy(int MaxDirect, int MaxGateway)
{
    public static ConnectionPolicy Unlimited { get; } = new(-1, -1);
}
```

`MaxDirect` = how many distinct logical clients can hold direct (non-gateway) connections. `MaxGateway` = how many gateway-pool clients (the Unity Assistant itself + any ACP sessions). `-1` means unlimited.

Caps come from three layered sources:

1. **Backend** — values pushed from `SettingsRecord.AllowedMcpConnections` / `AllowedGatewayConnections` (server-side per-org config).
2. **Licensing entitlement** — the `com.unity.editor.ai` entitlement carries `CustomData` JSON with `MCPConnectionCount` / `AIGatewayConnectionCount`. Default fallback when fields are missing is **1** for each pool.
3. **Pro-license fallback** — if entitlements are unavailable but `Application.HasProLicense() == true`, the resolver assigns **3** to each pool.

The resolver picks the **maximum** across the three sources — `-1` always wins.

**Per-tier mapping (inferred):**

| Tier | Source | Effective `MaxDirect` (default) |
|---|---|---|
| Personal (free) | entitlement default fallback | **0-1** (changes between pre.N) |
| Pro / Enterprise / Industry | Pro-license fallback OR backend override | **1+** depending on seat config |
| Custom (Enterprise w/ org-tier override) | backend > entitlement | as configured |

`ConnectionCensus` dedups clients by deepest non-shell ancestor PID, so two simultaneous Claude Code sessions on the same machine are two slots, but a single client opening multiple transports is still one slot.

### Seat revocation

When `Account.settings.OnChange` fires (sign-out, org switch, seat removal), `AcpEntitlementWiring.Apply()` calls `ConnectionCensus.SetPolicy(...)` with the new caps. If the new cap is *lower* than the current live count, `EnforcePolicyAsync()` evicts the **oldest** logical clients first. **Practically: pull the seat → MCP goes dark on the affected client within an editor tick, and any inflight tool calls fail.**

## 3. Tool inventory (52 tools)

Two registries register tools into the MCP server:

- **`[McpTool]`** — declared in `Modules/Unity.AI.MCP.Editor/ToolRegistry/`. Native MCP tools, exposed only through MCP, not callable from in-Editor Assistant.
- **`[AgentTool]`** with `[AgentToolSettings(mcp: McpAvailability.Available | McpAvailability.Default)]` — declared in `Unity.AI.Assistant.FunctionCalling`. Native to in-Editor Assistant; the `AgentToolMcpAdapter` reflects each one and re-registers it as an MCP tool. `Default` = registered AND enabled by default; `Available` = registered but disabled until user toggles in `Project Settings > AI > Unity MCP > Tools`.

The relay surfaces tool names to clients with a `Unity_` prefix (the dot in `Unity.RunCommand` becomes an underscore in the wire schema).

### 3.1 Editor / state (5 tools)

| Tool | What it does | Key params | FW use |
|---|---|---|---|
| `Unity_ManageEditor` | Play/Pause/Stop play mode; GetState (isPlaying, isCompiling, isUpdating); GetSelection; GetWindows; GetActiveTool; SetActiveTool; AddTag/RemoveTag/GetTags; AddLayer/RemoveLayer/GetLayers; GetPrefabStage; GetProjectRoot | `Action`; `WaitForCompletion`; `ToolName, TagName, LayerName` | Drive play-mode for runtime captures (L2 unity-check). Poll `isCompiling=false` after script edits. Read selection to capture per-object screenshots. |
| `Unity_ReadConsole` | Get/Clear console; filter by type (Log/Warning/Error); count; time window; regex `Filter`; format (Plain/Detailed/Json); IncludeStacktrace | `Action: Get \| Clear`; `Types`; `Count`; `Filter` | Compile-error detection in `unity-check` L1. Drive the build-fix loop: edit script → wait for `isCompiling=false` → ReadConsole → fix. |
| `Unity_GetProjectData` | Project-level context (taxonomy, asset summary) | `maxAssetItems`, `maxOutputChars`, `maxTaxonomyDepth` | Bootstrap context for new chats. |
| `Unity_GetSha` | SHA-256 + size + last-modified UTC for a script URI without sending content | `Uri` | Cheap pre-flight before `Unity_ApplyTextEdits`. Useful for our determinism work. |
| `Unity_GetUserGuidelines` | Returns project-defined Editor preferences / guidelines as a string | n/a | Lets Claude pick up project-level guidance authored inside Unity. |

### 3.2 Search / read (5 tools)

| Tool | What it does | Key params |
|---|---|---|
| `Unity_Grep` | Ripgrep over project (default `.cs`; `--glob`/`--type` to widen); `-l` for path-only output | `args`; `path` |
| `Unity_FindInFile` | Regex search a single resource URI with `MaxResults`; returns line+col + SHA256 | `Uri`; `Pattern`; `IgnoreCase`; `MaxResults` |
| `Unity_FindProjectAssets` | Name + semantic-visual search over project assets | natural-language query |
| `Unity_ListResources` | Glob over `Assets/` returning `unity://path/...` URIs | `Pattern`, `Under`, `Limit` |
| `Unity_ReadResource` | Read a `unity://path/...` resource with optional slicing: `StartLine`/`LineCount`, `HeadBytes`, `TailLines`, or natural-language `Request: "last 120 lines"` | `Uri`, slicing fields |

`ReadResource` enforces a 2 MB size limit and rejects binaries.

### 3.3 Scripts (7 tools)

| Tool | What it does |
|---|---|
| `Unity_CreateScript` | Create a new C# script at `Assets/...` path with provided contents |
| `Unity_DeleteScript` | Delete by URI or `Assets/`-relative path |
| `Unity_ManageScript` | Compatibility router for legacy ops (read/write whole file) |
| `Unity_ManageScript_capabilities` | Returns supported ops, limits, and guards |
| `Unity_ApplyTextEdits` | Raw text edits keyed by `Uri` + offset; returns conflict if SHA mismatch |
| `Unity_ScriptApplyEdits` | **Preferred.** Structured edits at method/class boundary. Schema at `unity://spec/script-edits` |
| `Unity_ValidateScript` | Roslyn diagnostics on a script without committing |

Capability flags from live test: `replace_class` / `delete_class` / `replace_method` / `delete_method` / `insert_method` / `anchor_insert` / `anchor_delete` / `anchor_replace`; text ops `replace_range` / `regex_replace` / `prepend` / `append`; max edit payload 256 KB; `using_guard` enabled.

### 3.4 Scene / GameObject / asset (4 tools)

| Tool | Action enum | Notes |
|---|---|---|
| `Unity_ManageScene` | Load, Save, Create, GetHierarchy | Path normalized to `Assets/`. |
| `Unity_ManageGameObject` | create, modify, find, add_component, remove_component, set_component_property, get_components | Component property values support nested paths and asset path strings. **Known crash**: don't `get_components` on Canvas / RectTransform. |
| `Unity_ManageAsset` | import, create, modify, delete | Asset DB-level ops |
| `Unity_ManageMenuItem` | List, Execute, Exists, Refresh | Lists every `[MenuItem]` in the project; executes by path. **Replaces "describe menu paths to user" anti-pattern**. |

### 3.5 C# execution

`Unity_RunCommand` — the closest analogue to CoplayDev's `execute_code`. Two-step compile + execute. Class **must** be named `CommandScript`, **must** be `internal`, must implement `IRunCommand`.

**Golden Template:**

```csharp
using UnityEngine;
using UnityEditor;

internal class CommandScript : IRunCommand
{
    public void Execute(ExecutionResult result)
    {
        // 1. Your logic here
        GameObject cube = GameObject.CreatePrimitive(PrimitiveType.Cube);

        // 2. Register changes for Undo/Redo and tracking
        result.RegisterObjectCreation(cube);

        // 3. Log the result
        result.Log("Created {0}", cube);
    }
}
```

Rules:
- `internal class CommandScript` (public causes inconsistent-accessibility error).
- `result.RegisterObjectCreation(obj)` after creating; `result.RegisterObjectModification(obj)` BEFORE changing properties; `result.DestroyObject(obj)` instead of `Object.DestroyImmediate`.
- Logging via `result.Log("{0}", obj)`, `result.LogWarning`, `result.LogError`.
- Avoid top-level statements.
- Blocked namespaces (Phase-3-onward security): `System.Reflection`, `System.Net`, etc. Returns specific error message naming the blocked namespace.

Live-tested 2026-05-09 — works end-to-end with `IRunCommand` template.

### 3.6 Visual capture (3 tools)

| Tool | What it does |
|---|---|
| `Unity_Camera_Capture` | Renders 1920×1080 from a specific Camera GameObject (by `cameraInstanceID`). When null, falls back to Scene View camera. |
| `Unity_SceneView_Capture2DScene` | Orthographic capture by world-coords rectangle (`worldX/Y/Width/Height`, `pixelsPerUnit`). Purpose-built for our dots-on-pitch L2 evidence. |
| `Unity_SceneView_CaptureMultiAngleSceneView` | 4-angle 3D scene grid (Isometric, Front, Top, Right). 3D-only; perspective for iso, orthographic for the rest. Optionally takes `focusObjectIds` to frame specific objects. |

### 3.7 Profiler (11+ tools, all `Unity_Profiler_*`)

| Tool | Differentiation |
|---|---|
| `Unity_Profiler_GetFrameRangeTopTimeSummary` | Top samples summed over a frame range — finds dominant cost over a window |
| `Unity_Profiler_GetFrameTopTimeSamplesSummary` | Top samples in one specific frame by total time |
| `Unity_Profiler_GetFrameSelfTimeSamplesSummary` | One frame, ranked by **self-time** (excludes children) — actual hot leaf samples |
| `Unity_Profiler_GetSampleTimeSummary` | Stats for a *named* sample (mean/min/max/p99) over the available data |
| `Unity_Profiler_GetSampleTimeSummaryByMarkerPath` | Same but addressed by full marker path (disambiguates same-name samples) |
| `Unity_Profiler_GetBottomUpSampleTimeSummary` | Bottom-up view of one sample — for "who calls X" analysis |
| `Unity_Profiler_GetRelatedSamplesTimeSummary` | Samples on **other threads** firing concurrently — diagnoses main-thread waits on jobs |
| `Unity_Profiler_GetOverallGcAllocationsSummary` | All GC allocs over the loaded data — top offenders ranked |
| `Unity_Profiler_GetFrameGcAllocationsSummary` | One frame, GC allocs ranked |
| `Unity_Profiler_GetFrameRangeGcAllocationsSummary` | Frame range, GC allocs |
| `Unity_Profiler_GetSampleGcAllocationSummary` + `..ByMarkerPath` | GC alloc stats for a named or marker-path-addressed sample |

Phase-7 budget enforcement substrate. Any frame regression in the dots adapter can be diagnosed end-to-end by `GetFrameRangeTopTimeSummary` (find regressed window) → `GetFrameSelfTimeSamplesSummary` (find leaf cost) → `GetRelatedSamplesTimeSummary` (cross-thread blocker).

### 3.8 Asset generation (12 tools, `Unity_AssetGeneration_*`)

| Tool | Function ID |
|---|---|
| `Unity_AssetGeneration_GenerateAsset` | Master generation tool — image / sprite / texture / 3D model / animation / sound / cubemap dispatch |
| `Unity_AssetGeneration_GetModels` | Lists 32 first-party models (see §3.8.1) |
| `Unity_AssetGeneration_GetCompositionPatterns` | Returns supported composition presets |
| `Unity_AssetGeneration_ConvertToMaterial` | Wraps existing Texture2D/Cubemap into a `.mat` |
| `Unity_AssetGeneration_ConvertToTerrainLayer` | Texture → Terrain Layer asset |
| `Unity_AssetGeneration_ConvertSpriteSheetToAnimationClip` | Slice a sprite sheet → AnimationClip |
| `Unity_AssetGeneration_CreateAnimatorControllerFromClip` | AnimationClip → AnimatorController |
| `Unity_AssetGeneration_EditAnimationClipTool` | Edit existing animation clip (trim / scale / loop) |
| `Unity_AssetGeneration_ManageInterrupted` | List / clean up generations interrupted by domain reload |
| `Unity_AudioClip_Edit` | Edit existing audio clip (trim, fade) |
| `Unity_ImportExternalModel` | Import FBX/glTF from URL; instantiate; save as prefab |

Generation is metered as **credits** (renamed from "points" in 2.7.0-pre.1). Pro/Enterprise/Industry plans bundle MCP access + credit pools.

#### 3.8.1 Available models (32, live-tested 2026-05-09)

**Image / sprite:** `flux-2-dev`, `flux-2-pro`, `gpt-image-1`, `gpt-image-1-5`, `gpt-image-1-5-recolor`, `gemini-3.0-pro`, `gemini-3.1-flash`, `game-ui-essentials-2`, `scenario-image-transform`, `photoroom-bg-removal`, `scenario-gemini-upscale`

**Material / texture:** `gemini-3.1-flash-texture`, `hand-painted-textures-2-0`, `realistic-textures-3-0`, `scenario-texture-upscale-v3`, `unity-texture2d`

**3D model:** `model3d-tripo-p1`, `model3d-tripo-p1-multiview`, `model3d-tripo3-texturing`, `model3d-tripo-retopo`, `model3d-tripo-rigging-v1`

**Audio:** `elevenlabs-multilingual-v2` (TTS, 21 voices), `elevenlabs-sound-effects-v2`, `google-lyria-3-clip` (30s loops), `google-lyria-3-pro` (~2min songs), `meta-musicgen`

**Video:** `video-kling-v3-i2v-pro`, `video-kling-v3-i2v-standard`, `video-seedance-1-pro`

**Animation:** `unity-text-to-motion`

**Skybox:** `skybox-cinematic`, `skybox-standard`

### 3.9 Package manager

| Tool | What it does |
|---|---|
| `Unity_PackageManager_GetData` | Read package metadata. **Read-only — cannot install/remove.** |
| `Unity_PackageManager_ExecuteAction` | Add/Remove packages. **`McpAvailability.Available` — off by default.** User must enable in `Project Settings > AI > Unity MCP > Tools`. |

CoplayDev's `manage_packages` is on by default and covers install/remove without user-toggle action. Until the user enables `Unity_PackageManager_ExecuteAction`, package install/remove routes to CoplayDev.

## 4. Custom tool registration

Two distinct registries.

### 4.1 `[McpTool]` — external clients only

```csharp
using Unity.AI.MCP.Editor.ToolRegistry;

[McpTool("FW.RunDeterminismCheck",
         "Runs N MatchSim ticks and returns the canonical state hash.")]
public static object RunDeterminismCheck(DeterminismCheckParams p)
{
    var hash = MatchSimHarness.Run(p.Seed, p.Ticks);
    return new { success = true, hash, ticks = p.Ticks, seed = p.Seed };
}

public class DeterminismCheckParams
{
    [McpDescription("Match seed (uint32 stringified)", Required = true)]
    public string Seed { get; set; }

    [McpDescription("Number of fixed-step ticks to simulate", Default = 90 * 60)]
    public int Ticks { get; set; } = 5400;
}
```

Four registration patterns: typed-static (above), JObject-static with `[McpSchema(toolName)]`, class-based implementing `IUnityMcpTool<T>` or `IUnityMcpTool`, runtime API via `McpToolRegistry.RegisterTool<T>(name, instance, description)`.

`[McpTool]` properties: `Name` (required), `Description`, `Title`, `Groups: string[]`, `EnabledByDefault`.

`[McpDescription]` on parameters: `Description` (required), `Required`, `EnumType`, `Default`.

### 4.2 `[AgentTool]` — in-Editor Assistant + optionally MCP

```csharp
using Unity.AI.Assistant.FunctionCalling;

public static class FwTools
{
    [AgentTool("Runs N MatchSim ticks and returns the canonical hash.",
               "FW.RunDeterminismCheck")]
    [AgentToolSettings(
        assistantMode: AssistantMode.Agent | AssistantMode.Ask,
        toolCallEnvironment: ToolCallEnvironment.EditMode | ToolCallEnvironment.PlayMode,
        mcp: McpAvailability.Default)]
    public static string RunDeterminismCheck(
        ToolExecutionContext context,
        [ToolParameter("Match seed.")] uint seed,
        [ToolParameter("Tick count.")] int ticks = 5400)
    {
        return MatchSimHarness.Run(seed, ticks);
    }
}
```

`mcp: McpAvailability.Default` registers the tool for both Assistant chat AND external MCP clients. Tools in assemblies whose name contains `Tests`, `Benchmark`, or `Sample` are excluded.

ID convention: dot-separated PascalCase, project-namespaced. Use `FW.*` for FW. The MCP wire form replaces dots with underscores.

### Pick the right attribute

- Pure MCP-only tool → `[McpTool]`. Less ceremony.
- Tool callable from BOTH Claude/Codex AND in-Editor Assistant → `[AgentTool]` with `mcp: McpAvailability.Default`. One implementation, two surfaces.

For FW's `FW_RunDeterminismCheck`: `[AgentTool]` so we can also wire it into a `Final Whistle/Verify/...` menu item.

## 5. AssistantApi — drive the in-Editor Assistant from code

```csharp
using Unity.AI.Assistant.Editor.Api;

[MenuItem("Final Whistle/Verify/Run Slice 6 Visual Pass")]
public static async Task VerifySlice6()
{
    var ctx = new AssistantApi.AttachedContext();
    ctx.Add(new VirtualAttachment(
        payload: "{ \"slice\": 6, \"shotTypes\": 7 }",
        type: "application/json",
        displayName: "slice-6-spec.json",
        metadata: null));
    await AssistantApi.Run(
        "Run FW.RunDeterminismCheck for seed 42 ticks 5400, then capture the in-game camera, post hashes + screenshot to dialog/.",
        ctx);
}
```

Three entry points:
- **`AssistantApi.Run(prompt, AttachedContext, CT)`** — opens Assistant window + submits.
- **`AssistantApi.PromptThenRun(buttonRect, placeholder, ctx, CT)`** — popup to review before submit.
- **`agent.RunHeadless(prompt, ctx, CT)`** — extension on `IAgent`. Runs without UI, returns final answer as `string`. Unity Assistant provider only.

`AttachedContext` accepts: `Add(UnityEngine.Object)`, `Add(VirtualAttachment(...))`, `AddImageContent(Texture, displayName)`.

## 6. Plan / Agent / Ask modes

**These are in-Editor Assistant concepts. They do not apply when Claude Code drives Unity over MCP.** When the main session routes calls through `mcp__UnityAIAssistant__Unity_*` tools, mode selection is irrelevant.

For completeness:
- **Ask** — read-only inspection.
- **Plan** — generates a saved implementation plan first; switches to Agent on approval.
- **Agent** — performs writes; respects per-tool permissions.

The `[AgentToolSettings(assistantMode: ...)]` flag on custom `[AgentTool]` registrations gates which Assistant mode can call the tool, but does not affect MCP-driven invocations.

## 7. Best-practice prompt patterns

Unity's own best-practice doc says:

> **Current state > Desired outcome > Technical details**

Apply this to tool-call sequences, not just chat prompts. Stale Phase-3 pattern was:

> "Apply this UXML/USS." → `Unity_ApplyTextEdits`

Better:

> "Scoreboard renders blank at 1280×720 in Phase-3 dots viewer (current). Want title-card + score + commentary line visible by frame 60 (desired). Rebuild via `Unity_CreateScript` for the controller, `Unity_ManageScene` to reload Bootstrap, `Unity_ManageEditor` Play, then `Unity_Camera_Capture` for the visual contract (technical)."

Other rules:
- **Use Unity-specific terminology**: `GameObject` not "3D object"; `Sprite/Texture2D/Material/RenderTexture` not "image"; `Cinemachine/VirtualCamera/FreeLook` not "camera plugin".
- **Reference scene objects by instance ID**, not name lookups across calls.
- **Iterative refinement**: split a 5-tool batch across multiple turns; verify each step via `Unity_ReadConsole` before the next.
- **Match prompt complexity to task**: don't construct a 12-step plan for a single field flip.

## 8. Live-tested capability comparison vs CoplayDev

Tested 2026-05-09 against `com.unity.ai.assistant 2.7.0-pre.3` and CoplayDev `unity-mcp` (current).

| Capability | CoplayDev `UnityMCP` | Official `UnityAIAssistant` |
|---|---|---|
| Inline C# execution | `execute_code` | `Unity_RunCommand` (parity; tighter blocked-namespace allowlist) |
| Package install/remove | `manage_packages` (on by default) | `Unity_PackageManager_ExecuteAction` (off by default; user must enable) |
| Prefab CRUD | `manage_prefabs` granular | Subsumed in `Unity_ManageGameObject` + `Unity_ManageAsset`. Granularity loss; workaround: drive prefab ops via `Unity_RunCommand`. |
| Animation authoring | `manage_animation` | `Unity_AssetGeneration_*` only — no first-class transition API; workaround via `Unity_RunCommand`. |
| VFX | `manage_vfx` | None |
| ProBuilder | `manage_probuilder` | None |
| Transactional batches | `batch_execute` | None — but `Unity_RunCommand` collapses most batches into one C# script. |
| Profiler | `manage_profiler` (single tool) | 11 specialized tools (granular) |
| Visual capture | `manage_camera` | `Unity_Camera_Capture` + `Unity_SceneView_Capture2DScene` + `Unity_SceneView_CaptureMultiAngleSceneView` |
| Asset generation | None | `Unity_AssetGeneration_*` family + 32 first-party models |
| File SHA | manual | `Unity_GetSha` |
| Editor menu invocation | None | `Unity_ManageMenuItem` |
| Semantic C# edits | partial | `Unity_ManageScript_capabilities` (replace_class / replace_method / anchor_*) |

**Workaround pattern for gaps:** when official is missing a granular tool that CoplayDev exposed, fall back to inline C# in `Unity_RunCommand` using the Editor API. The 2.7.0-pre.3 namespace blocklist (no Reflection / Net) is the only hard wall.

## 9. Editor menu items as a substrate

`Unity_ManageMenuItem` lets MCP clients enumerate every `[MenuItem]` in the project + execute them. Source: `Modules/Unity.AI.MCP.Editor/Tools/ManageMenuItem.cs`. Cache built from `TypeCache.GetMethodsWithAttribute<MenuItem>()` at `[InitializeOnLoad]`; refreshed on `Refresh: true` flag.

Currently authored in our project: `Final Whistle/Setup/Repair URP Renderer` (`unity-project/Assets/Scripts/Editor/Setup/RepairUniversalRenderer.cs`). One menu item across the whole repo.

### Proposed FW menu items to author

Each is a one-screen `[MenuItem]` that internalizes a manual step. The MCP path becomes `Unity_ManageMenuItem Execute "<path>"` instead of a multi-tool compose. Each menu item also remains usable from the Editor without MCP — useful for the user's manual sessions.

1. **`Final Whistle/Verify/Run Determinism Check`** — spawn N MatchSim runs from a fixed seed across `MatchSim.Tests` fixtures, return canonical hash array.
2. **`Final Whistle/Verify/Capture Slice N Visual`** — drive `EditorApplication.isPlaying = true`, wait N seconds, capture the in-game camera, save PNG to `dialog/captures/slice-N-<git-sha>.png`.
3. **`Final Whistle/Verify/Refresh Addressables`** — `AddressableAssetSettingsDefaultObject.Settings.BuildPlayerContent()` programmatic.
4. **`Final Whistle/Setup/Reseat Asmdef Graph`** — call `AssetDatabase.ImportAsset` on every `.asmdef` and `Refresh()`. Idempotent.
5. **`Final Whistle/Setup/Reset MatchSim Fixtures`** — restore `MatchSim.Tests/Fixtures/*.json` from canonical seeds.
6. **`Final Whistle/Diagnostics/Dump Adapter Routing Table`** — write the live `ShotPresentationContract`-to-adapter mapping to JSON in `dialog/`.
7. **`Final Whistle/Diagnostics/Open Latest Determinism Hash Log`** — opens `Library/FW/determinism-hashes.log`.

Authoring time: half a day total. Payoff: every workflow currently requiring a multi-tool MCP sequence collapses to one `Unity_ManageMenuItem Execute` call.

## 10. Risks + entitlement fragility

**Pre-release version.** `2.7.0-pre.3` shipped 2026-05-06. Three risks:
- Tool-shape changes between pres (already happened: `points → credits` rename in pre.1; `--project-path` resolution fix in pre.3).
- Tool removal (CHANGELOG pre.1 already moved Profiler from a first-class tool family into the Skills implementation).
- Parameter-shape changes — `RunCommand`'s Golden Template is enforced; any contract change breaks every authored `CommandScript`.

**Mitigation:** pin `2.7.0-pre.3` in `manifest.json`; surface the version in `STATUS.md`; treat any new pre/stable release as a tracked SPEC task that re-tests the routing table.

**Entitlement fragility.** Cap=1 for Personal seats per `EditorConnectionLimitProvider.ExtractConnectionLimits` defaults. Two simultaneous Claude Code sessions on the same Personal seat → second one rejected. Sign-out / org-switch → `EnforcePolicyAsync` evicts the oldest connection within an editor tick.

**Pricing model.** May 2026 open-beta announcement: Personal = 14-day trial + 1000 credits; Pro/Enterprise/Industry = bundled MCP access + bundled credit pools. Post-beta pricing not yet public.

**Reversibility.** CoplayDev's `unity-mcp` package remains installed in `unity-project/Packages/manifest.json` and registered in `.mcp.json` for the duration of Phase 3 → Phase 7 minimum. Removal is deferred per the deprecation gates in the routing table. Worst-case fallback: re-enable both, route the gap-tools through CoplayDev, route everything else through Unity.

## 11. Community + third-party landscape

At least four MCP-for-Unity implementations existed by May 2026:

| Implementation | Maintainer | Status |
|---|---|---|
| `unity-mcp` (`com.coplaydev.unity-mcp`) | CoplayDev | Most popular community option pre-official. Granular APIs; TCP/HTTP transport. |
| `UnityAIAssistant` (`com.unity.ai.assistant`) | Unity Technologies (official) | Subject of this playbook. Bundles relay binary + AssistantApi + 32 first-party AI models. Open-beta May 4 2026. |
| `mcp-unity` | CoderGamester | Smaller scope; primarily script + asset CRUD. |
| `UnityMCP` | Bluepuff71 | Single-developer, partial coverage. |

**Pricing model from the May 4 2026 open-beta announcement** ([Unity Discussions thread](https://discussions.unity.com/t/cli-mcp-ai-ide-chatgpt-codex-cursor-antigravity-claude-code-windsurf-next-steps-for-unity-workflow-automation/1705679)):

- Personal: 14-day trial, 1000 credits, then prompts to upgrade.
- Pro: bundled MCP access; AI generation models still bill credits.
- Enterprise: same as Pro + org-level admin.
- Industry: same as Enterprise + extended bundled credits.

CoplayDev has zero metering — it's a free relay over stdio MCP. **For Final Whistle:** the official package is the right primary pick now that the seat is funded; CoplayDev stays installed as the parachute.

## Appendix: file-path reference

All paths inside `unity-project/Library/PackageCache/com.unity.ai.assistant@198d71476a35/`:

- Manifest: `package.json`
- Changelog: `CHANGELOG.md`
- Doc index: `Documentation~/index.md`
- MCP server overview: `Documentation~/integration/unity-mcp-overview.md`
- MCP get-started: `Documentation~/integration/unity-mcp-get-started.md`
- Custom MCP tool registration: `Documentation~/integration/unity-mcp-tool-registration.md`
- AssistantApi: `Documentation~/integration/assistant-api.md`
- Custom AgentTool: `Documentation~/integration/custom-agent-tool.md`
- Best-practice prompts: `Documentation~/best-practice/best-practice-assistant.md`
- Modes: `Documentation~/about/assistant-modes.md`
- Connection census source: `Modules/Unity.AI.MCP.Editor/Connection/ConnectionCensus.cs`
- AgentTool→MCP adapter: `Modules/Unity.AI.MCP.Editor/Adapters/AgentToolMcpAdapter.cs`
- Bridge: `Modules/Unity.AI.MCP.Editor/Bridge.cs`
- Tool sources: `Modules/Unity.AI.MCP.Editor/Tools/*.cs` (18 files)
- Existing FW menu item to model new ones on: `unity-project/Assets/Scripts/Editor/Setup/RepairUniversalRenderer.cs`

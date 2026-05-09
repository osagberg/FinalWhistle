---
description: Unity MCP routing table for Final Whistle. Names primary tool + fallback per task class. Authority is ADR-0011. Server names are UnityAIAssistant (official, primary, stdio relay) and UnityMCP (CoplayDev, fallback, HTTP :8080). Read on every subagent invocation that may touch the Editor + every session start during Phase 3-7.
---

# Unity MCP routing table

> **Authority:** [ADR-0011](../../design/adr/adr-0011-unity-ai-assistant-mcp-migration.md) (Accepted 2026-05-09).
> **Scope:** All Editor automation invoked from Claude Code, Codex CLI (when permitted — see §single-driver), and any subagent operating against the Unity project.
> **Read this on:** every subagent invocation that may touch the Editor; every session start during Phase 3 → Phase 7.

This table names the **primary** MCP tool for each Editor-automation task class, the **fallback**, and the rationale. Status column flags which routings are live-tested, which are inferred from documentation, and which are not yet exercised.

## Single-driver rule

`MaxDirect = 1` per Unity Pro seat. Claude holds the slot by default. If Codex or another tool needs the official MCP, the user explicitly disconnects Claude first. Routing assumes Claude is the active driver unless otherwise stated. Codex's review role does not require Editor MCP — Codex reads the file system + runs `scripts/fw verify` + reads `dialog/` topics; it never connects to `UnityAIAssistant`.

## MCP server names (this project's `.mcp.json`)

- **`UnityAIAssistant`** — official Unity AI Assistant MCP (`com.unity.ai.assistant 2.7.0-pre.3`, stdio relay child of Editor process, requires Pro seat). **Primary.**
- **`UnityMCP`** — CoplayDev `com.coplaydev.unity-mcp` (HTTP :8080, no entitlement, name predates the migration). **Fallback.**

Tool-name prefixes:
- Official tools surface as `mcp__UnityAIAssistant__Unity_*` (e.g., `mcp__UnityAIAssistant__Unity_ManageEditor`, `mcp__UnityAIAssistant__Unity_RunCommand`).
- CoplayDev tools surface as `mcp__UnityMCP__*` (e.g., `mcp__UnityMCP__execute_code`, `mcp__UnityMCP__manage_packages`).

## Routing table

| Task class | Primary | Fallback | Rationale | Status |
|---|---|---|---|---|
| **C# in-Editor execution (one-off ops)** | `UnityAIAssistant / Unity_RunCommand` (with `IRunCommand` template) | `UnityMCP / execute_code` | Official supports full C# with structured templates; tighter blocked-namespace allowlist (no `System.Reflection`/`System.Net`). | live |
| **Editor scene state introspection** | `UnityAIAssistant / Unity_ManageEditor GetState` + `Unity_ManageScene` queries | `UnityMCP / manage_scene` | Both work; official returns richer structured payload + IsCompiling/IsUpdating flags. | live |
| **Editor menu item invocation** | `UnityAIAssistant / Unity_ManageMenuItem` | (none — CoplayDev has no equivalent; fall back to `execute_code` calling `EditorApplication.ExecuteMenuItem`) | Replaces "describe menu path to user" anti-pattern. | live |
| **Console reads (filtered)** | `UnityAIAssistant / Unity_ReadConsole` | `UnityMCP / read_console` | Official supports format/severity/stacktrace filters cleanly + ISO-8601 since-timestamp. | live |
| **File search — string** | `UnityAIAssistant / Unity_Grep` | `UnityMCP / find_in_file` | Official wraps ripgrep; defaults to `.cs`; faster on large repos. | live |
| **File search — regex with line+col** | `UnityAIAssistant / Unity_FindInFile` | `UnityMCP / find_in_file` | Official returns line+col + per-result SHA256 — useful for downstream `Unity_ScriptApplyEdits` anchored edits. | live |
| **File search — semantic / visual content** | `UnityAIAssistant / Unity_FindProjectAssets` (sparse on code-heavy projects) | (compose via `Unity_RunCommand` for AST-shaped queries) | Semantic search needs visual content; under-served on a code-heavy project until Phase 4+. | experimental |
| **File SHA / metadata for determinism verification** | `UnityAIAssistant / Unity_GetSha` | (compute via Bash `shasum` outside Unity) | Returns SHA256 + size + last-modified UTC in one call. | live |
| **Read file content (with slicing)** | `UnityAIAssistant / Unity_ReadResource` | Read tool (file system, outside MCP) | Slicing avoids full-file payload; 2 MB cap, rejects binaries. | live |
| **C# script semantic edits** | `UnityAIAssistant / Unity_ScriptApplyEdits` (`replace_class` / `replace_method` / `delete_method` / `insert_method` / `anchor_insert` / `anchor_replace`) | Edit tool (text-based) | Semantic edits respect Roslyn structure; reduce drift on determinism-sensitive code. 256 KB max payload; `using_guard` enabled. | live |
| **C# script text edits (regex / range)** | `UnityAIAssistant / Unity_ApplyTextEdits` (`regex_replace` / `replace_range` / `prepend` / `append`) | Edit tool | Use semantic variants first; fall back to regex only when the edit is genuinely not method/class-shaped. | live |
| **L2 visual evidence — 2D orthographic scene** | `UnityAIAssistant / Unity_SceneView_Capture2DScene` | `UnityMCP / manage_camera` + screenshot composition | Purpose-built for dots-on-pitch L2 captures; replaces hand-rolled flow in `unity-check` skill. | live |
| **L2 visual evidence — 3D multi-angle** | `UnityAIAssistant / Unity_SceneView_CaptureMultiAngleSceneView` | (compose via 4× run-command + camera-pose script) | Direct value for Phase 5/6 3D-pipeline spike. 3D-only. | live |
| **L2 visual evidence — specific camera capture** | `UnityAIAssistant / Unity_Camera_Capture` (by instance ID) | `UnityMCP / manage_camera` | Official renders any camera deterministically by ID; 1920×1080 hard-coded. | live |
| **Profiler — granular GC alloc per frame** | `UnityAIAssistant / Unity_Profiler_GetFrameGcAllocationsSummary` | (manual profiler capture by user) | Phase 7 budget enforcement; programmatic GC=0 verification in hot paths. | not yet tested |
| **Profiler — top time samples per frame** | `UnityAIAssistant / Unity_Profiler_GetFrameTopTimeSamplesSummary` | (manual) | Find dominant cost in one frame. | not yet tested |
| **Profiler — frame-range top-time-summary** | `UnityAIAssistant / Unity_Profiler_GetFrameRangeTopTimeSummary` | (manual) | Find regressed window. | not yet tested |
| **Profiler — self-time leaf samples** | `UnityAIAssistant / Unity_Profiler_GetFrameSelfTimeSamplesSummary` | (manual) | Find actual hot leaf samples (excludes child time). | not yet tested |
| **Profiler — related samples cross-thread** | `UnityAIAssistant / Unity_Profiler_GetRelatedSamplesTimeSummary` | (manual) | Diagnose main-thread waits on jobs. | not yet tested |
| **Profiler — bottom-up sample** | `UnityAIAssistant / Unity_Profiler_GetBottomUpSampleTimeSummary` | (manual) | "Who calls X" analysis. | not yet tested |
| **Profiler — overall GC allocations** | `UnityAIAssistant / Unity_Profiler_GetOverallGcAllocationsSummary` | (manual) | Top GC offenders ranked. | not yet tested |
| **Asset generation — sprite / 2D image** | `UnityAIAssistant / Unity_AssetGeneration_GenerateAsset` (Flux 2 Pro / Gemini 3.1 / GPT Image 1.5) | external (GPT Image 2 via existing pipeline) | **Bake-time only** per CLAUDE.md §3. Pick model per cost/quality. Crest concepts, scout portraits, signature card art. | experimental |
| **Asset generation — material / texture** | `UnityAIAssistant / Unity_AssetGeneration_GenerateAsset` (Flux 2 Dev for materials, Tripo 3 Texturing for 3D textures, hand-painted-textures-2-0, realistic-textures-3-0) | external | **Bake-time only.** Phase 5/6 spike. | experimental |
| **Asset generation — 3D model / rigging / retopo** | `UnityAIAssistant / Unity_AssetGeneration_GenerateAsset` (Tripo P1 image-to-3D → Tripo Rigging v1 → Tripo Retopo → Tripo 3 Texturing) | external (deferred per `design/3d-pipeline.md`) | Direct accelerant for Phase 5/6 production-feasibility spike. **Spike-gated** — do not productionize before spike-green outcome per ADR-0011 + 2026-04-26 decisions-log entry. | experimental |
| **Asset generation — audio (SFX / music / TTS)** | `UnityAIAssistant / Unity_AssetGeneration_GenerateAsset` (ElevenLabs multilingual-v2 TTS + sound-effects-v2; Lyria 3 clip/pro music; MusicGen) | external | **Bake-time only.** Scout-prose TTS drafts, stadium SFX, signature-moment stings. FMOD remains the runtime audio engine. | experimental |
| **Asset generation — animation (text-to-motion)** | `UnityAIAssistant / Unity_AssetGeneration_GenerateAsset` (unity-text-to-motion) | external (deferred) | Spike-gated; only relevant if 3D candidate stack proceeds. | experimental |
| **Audio clip editing** | `UnityAIAssistant / Unity_AudioClip_Edit` | external DAW | Convenience for trim/normalize on bake-time audio. | not yet tested |
| **External 3D model import** | `UnityAIAssistant / Unity_ImportExternalModel` | manual drag-into-Project | Spike-relevant. | not yet tested |
| **UPM package install / remove** | **`UnityMCP / manage_packages`** | `UnityAIAssistant / Unity_PackageManager_ExecuteAction` (only after user enables it under `Project Settings > AI > Unity MCP > Tools` — currently `McpAvailability.Available`, off by default) | **CoplayDev primary here.** Official is gated behind a project-settings toggle the user must flip. Until enabled, package mutation routes to CoplayDev. | live |
| **UPM package metadata read** | `UnityAIAssistant / Unity_PackageManager_GetData` | `UnityMCP / manage_packages` (read mode) | Official is the simpler read path. | live |
| **GameObject create / modify / delete** | `UnityAIAssistant / Unity_ManageGameObject` | `UnityMCP / manage_gameobject` | Both work; official accepts richer component-spec payloads. **Avoid** `get_components` on Canvas / RectTransform on official (known crash). | live |
| **Scene create / load / save** | `UnityAIAssistant / Unity_ManageScene` | `UnityMCP / manage_scene` | Parity. | live |
| **Asset CRUD (general)** | `UnityAIAssistant / Unity_ManageAsset` | `UnityMCP / manage_asset` | Parity for general asset ops. | live |
| **Asset ops — prefab-specific (granular)** | **`UnityMCP / manage_prefabs`** | `UnityAIAssistant / Unity_RunCommand` (compose via `PrefabUtility` C#) | **CoplayDev primary** when the operation is granularly prefab-shaped (variants, override application, instance-vs-asset). Otherwise official `Unity_ManageAsset`. | live |
| **Asset ops — animation-specific (granular)** | **`UnityMCP / manage_animation`** | `UnityAIAssistant / Unity_RunCommand` (compose `AnimationClip` / `AnimatorController` via C#) | **CoplayDev primary** until official ships dedicated animation tooling. No first-class transition API on official. | live |
| **Asset ops — VFX / probuilder** | **`UnityMCP / manage_vfx` / `manage_probuilder`** | (compose via `Unity_RunCommand`) | **CoplayDev primary** until parity. Low Phase 3 usage; revisit if either becomes hot. | live |
| **Transactional / batched ops** | `UnityAIAssistant / Unity_RunCommand` (compose inside one C# script) | `UnityMCP / batch_execute` | Official requires composition; CoplayDev's explicit batch is sometimes cleaner for review. | live |
| **Custom tool authoring (FW-specific)** | `UnityAIAssistant / [McpTool]` or `[AgentTool]` attributes on Editor scripts (auto-discovered) | `UnityMCP` custom-tool registry (browseable via `mcpforunity://custom-tools`) | Official has no runtime-browsable custom-tool resource yet — track tools in `docs/tooling/unity-mcp-custom-tools.md` (TBD if/when we author any). | not yet exercised |

## Deprecation candidates (CoplayDev retirement gates)

CoplayDev `unity-mcp` (`UnityMCP` server) is retained as fallback through Phase 7 minimum. CoplayDev can be retired when **all** of the following are true:

1. **Stable `com.unity.ai.assistant` release** (i.e., not `-pre.N`). Pre-release tool-shape churn is the dominant breakage risk; stable removes it.
2. **Official MCP ships writeable UPM package management on by default** — currently `Unity_PackageManager_ExecuteAction` is `McpAvailability.Available` (off by default). When it ships as `Default`, package mutation can route to official.
3. **Official MCP ships dedicated prefab-granular tooling** OR we confirm via 30-day usage data that `Unity_ManageAsset` + `Unity_RunCommand` composition is sufficient for our Phase 4-7 prefab workflow.
4. **Official MCP ships dedicated animation-clip/controller tooling** OR same confirmation as (3) for animation workflow.
5. **Entitlement model has been stable for ≥90 days post-beta** — no surprise pricing shift, no `MaxDirect` model change. This protects against retiring the fallback exactly when we need it.

When all five gates are green, append an ADR-0012+ citing this one + ADR-0011, remove `com.coplaydev.unity-mcp` from `unity-project/Packages/manifest.json`, and remove `UnityMCP` from `.mcp.json`. Until then, CoplayDev stays.

## Operational notes

- **Subagent entry contract:** any subagent invoked for Editor-touching work reads this table first. The mandatory-rotation table in `CLAUDE.md §6.3` does not duplicate the routing — it points here.
- **`unity-check` skill:** L2 visual evidence captures route to `Unity_SceneView_Capture2DScene` (dots-phase 2D) or `Unity_SceneView_CaptureMultiAngleSceneView` (3D candidate spike). Skill code should call official tools first; CoplayDev path retained as fallback branch.
- **Determinism work:** prefer `Unity_GetSha` over Bash `shasum` when verifying Unity-imported assets, because import settings can shift the .meta and the actual asset hash separately. For pure `MatchSim/**` files (zero UnityEngine refs), Bash `shasum` is fine.
- **Asset-generation gating:** every asset-generation tool is bake-time-only per `CLAUDE.md §3`. Generated assets land under `Assets/Generated/**` (path TBD) and go through the same content-pack ID + schema-version discipline as hand-authored content. Do not invoke generation tools at runtime; do not invoke speculatively without a tracked task in SPEC.
- **Pre-release upgrade discipline:** when `com.unity.ai.assistant` ships a new version (pre or stable), do not auto-upgrade. Open a SPEC task to: (a) live-test the existing routing rows against the new version, (b) update this table, (c) amend ADR-0011 if tool shapes shifted materially, (d) commit with `pr-review-toolkit` per §6.3.
- **Cap=1 single-driver:** Claude holds the slot. If Codex needs Editor access, the user disconnects Claude first. Codex's review role normally does not need it — Codex reads the file system + runs `scripts/fw verify` + reads `dialog/` topics.
- **Reversibility:** `.mcp.json` keeps both servers registered. Routing entries can shift back to CoplayDev primary by editing this table — no `.mcp.json` change required for fallback. ADR-0011 supersession path: append a new ADR citing this one; do not edit ADR-0011 in place.

## Last verified

2026-05-09 — live-tested 12+ tools end-to-end (`Unity_ManageEditor GetState`, `Unity_GetUserGuidelines`, `Unity_AssetGeneration_GetModels`, `Unity_ManageScript_capabilities`, `Unity_GetProjectData`, `Unity_PackageManager_GetData`, `Unity_ListResources`, `Unity_Grep`, `Unity_GetSha`, `Unity_ManageMenuItem List`, `Unity_RunCommand`, `Unity_FindInFile`).

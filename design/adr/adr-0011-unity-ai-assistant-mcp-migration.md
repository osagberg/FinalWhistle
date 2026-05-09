---
description: ADR-0011 — Migrate from CoplayDev unity-mcp (HTTP :8080) to the official Unity AI Assistant MCP (com.unity.ai.assistant 2.7.0-pre.3, stdio relay) as the PRIMARY editor automation surface from Phase 3 onward. CoplayDev retained as FALLBACK for capability gaps + entitlement-failure recovery. Authored 2026-05-09 in autonomous mode (no Codex round-trip per user instruction).
---

# ADR-0011: Unity AI Assistant MCP migration — official primary, CoplayDev fallback

## Status

**Accepted** — 2026-05-09 in autonomous mode per user instruction. No Codex round-trip tonight; recommended for next session per the established cross-model rhythm in CLAUDE.md §6.3. Append-only from this point — supersession only via a new ADR.

## Date

2026-05-08 (drafted) / 2026-05-09 (Accepted)

## Last Verified

2026-05-09

## Decision Makers

osagberg (project owner — authorized migration + funded the Pro seat), Claude (workhorse author — synthesized from 5 parallel research/design subagents).

---

## Summary

The official Unity AI Assistant MCP (`com.unity.ai.assistant 2.7.0-pre.3`, registered in `.mcp.json` as **`UnityAIAssistant`**, stdio relay child of the Editor process) becomes the **primary** editor automation surface for Final Whistle from Phase 3 onward. CoplayDev `unity-mcp` (registered as **`UnityMCP`**, HTTP :8080) is retained as the **fallback** for capability gaps the official MCP does not cover today (UPM package install/remove, granular prefab/animation/VFX/probuilder ops, explicit `batch_execute`) and for entitlement-failure recovery. Routing is defined at [`docs/tooling/unity-mcp-routing.md`](../../docs/tooling/unity-mcp-routing.md).

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Unity 6, currently pinned to tech-stream `6000.4.4f1` (LTS migration re-evaluated at Phase 7); URP 17.4.0 |
| Domain | Tooling / Editor automation (does NOT touch MatchSim canonical sim, which is engine-independent) | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | MEDIUM — pre-release Unity package (`2.7.0-pre.3`); tool-shape churn observed between pre.1 → pre.2 → pre.3 over two weeks; stable `2.7.0` unscheduled |
| References Consulted | `docs/tooling/unity-mcp-playbook.md` (deep-research artifact), live testing of 12+ tools 2026-05-09, Unity Discussions thread on May 4 2026 open beta, `com.unity.ai.assistant 2.7.0-pre.3` source code |
| Post-Cutoff APIs Used | `Unity_RunCommand` with `IRunCommand` template, `Unity_ManageMenuItem`, `Unity_SceneView_Capture2DScene`, 11 `Unity_Profiler_*` tools, `Unity_AssetGeneration_*` family with 32 first-party models |
| Verification Required | Phase-3-through-Phase-7: every official-MCP tool added to the routing table goes through the standard `unity-check` L1/L2/L3 verification; `pr-review-toolkit` triple before commit; cross-platform `scripts/fw verify` continues to gate determinism |

## Dependencies

| Field | Value |
|---|---|
| Depends On | Active Unity Pro/Enterprise seat (`MaxDirect = 1` direct connection slot); `com.unity.ai.assistant 2.7.0-pre.3` package installed in `unity-project/Packages/manifest.json`; relay binary at `~/.unity/relay/relay_mac_arm64.app/Contents/MacOS/relay_mac_arm64` (Apple Silicon; equivalents on Intel/Windows/Linux) |
| Enables | (1) Phase 3 dots-phase polish via `Unity_SceneView_Capture2DScene` for L2 evidence; (2) Phase 4 narrative + content via bake-time asset generation (ElevenLabs TTS, GPT Image 1.5, Gemini 3.x); (3) Phase 5/6 3D-pipeline spike via Tripo P1 + Rigging + Retopo + Texturing + Unity Text-to-Motion; (4) Phase 7 perf budget enforcement via 11 granular `Unity_Profiler_*` tools |
| Blocks | None — both MCPs registered, routing table flips back to CoplayDev primary in <30 minutes if needed. Two-way door |

---

## Context

### Problem statement

Phase 3 of Final Whistle is mid-flight. The dots-phase viewer adapter (ADR-0009) is shipping-quality candidate. Phase 5/6 will run a production-feasibility spike on the 3D candidate stack per `design/3d-pipeline.md`. All Unity Editor automation since the start of Phase 3 has flowed through the CoplayDev `com.coplaydev.unity-mcp` server (HTTP :8080), invoked from Claude Code over MCP. That tool unblocked the Phase-3 anti-pattern logged 2026-04-28 ("describe menu paths to user") and has been the substrate for ~all Editor-driving session work since.

Two things changed this week:

1. **Unity shipped an official first-party MCP** as part of `com.unity.ai.assistant 2.7.0-pre.3` (released 2026-05-06). It runs as a stdio relay child of the Editor process rather than as a localhost HTTP server, and exposes a substantially larger tool surface than CoplayDev — including 32 first-party asset-generation models, 11 granular profiler tools, multi-angle scene capture, semantic C# edits, and `Unity_ManageMenuItem` for arbitrary Editor menu invocation.

2. **The user paid for a Unity Pro seat** specifically to enable that official MCP. The official server gates direct Claude/Codex connections behind a paid tier (`MaxDirect = 1` per seat). With one seat we have exactly one direct slot — Claude holds it; Codex stays out of the Editor and continues to operate as a file-system reviewer + `scripts/fw verify` runner.

We live-tested 12+ tools end-to-end on 2026-05-09 (`Unity_RunCommand` with `IRunCommand` template, `Unity_ManageEditor`, `Unity_ManageMenuItem`, `Unity_SceneView_Capture2DScene`, `Unity_GetSha`, `Unity_ManageScript_capabilities`, `Unity_GetProjectData`, `Unity_PackageManager_GetData`, `Unity_ListResources`, `Unity_Grep`, `Unity_FindInFile`, `Unity_GetUserGuidelines`, `Unity_AssetGeneration_GetModels`). All worked. Capability gap vs CoplayDev is real and material to the next four phases.

### Capability uplift mapped to phases

- **Phase 3 dots-phase polish** — `Unity_SceneView_Capture2DScene` is purpose-built for the 2D orthographic L2 evidence we already render manually via `manage_camera`. Reduces drift between L2 captures. `Unity_ManageMenuItem` ends the "describe menu path" anti-pattern definitively.
- **Phase 4 narrative + content** — `Unity_AssetGeneration_*` (ElevenLabs TTS for scout-prose drafts, GPT Image 1.5 for crest concepts) plugs in cleanly to a bake-time-only content pipeline without violating CLAUDE.md §3's "no runtime LLMs" constraint, because everything is Editor-time.
- **Phase 5/6 3D-pipeline spike** — Tripo P1 + Tripo Rigging + Tripo Retopo + Texturing + Unity Text-to-Motion materially shorten the spike's "can one human + Claude actually drive a 3D candidate stack to shippable polish" question. This was the dominant unknown in `design/3d-pipeline.md`.
- **Phase 7 perf budget enforcement** — 11 granular profiler tools replace "ask user to capture profiler frame" with direct programmatic queries. Tightens the 16.67 ms/frame budget loop.

### Real costs

Two costs deserve explicit accommodation, not hand-waving:

- **Pre-release churn.** `2.7.0-pre.3` shipped two weeks after `pre.1`, with breaking tool-shape changes between pres. Stable `2.7.0` is unscheduled. Tool names, parameter shapes, and return schemas can shift again before stable. Routing table + ADR may need an amendment when stable lands.
- **Entitlement fragility.** Cap=1 is per active Pro/Enterprise seat. If the seat lapses, Unity changes pricing post-beta, or the AI Assistant package's entitlement model shifts, the MCP goes dark with no migration window. CoplayDev stays installed and registered specifically to absorb that failure mode.

A third constraint deserves naming: **single-driver.** Cap=1 means Claude OR Codex can hold the Editor MCP at any time, not both. The cross-model rhythm in `CLAUDE.md §6.3` (Claude drafts → Codex reviews → Claude applies) survives this — Codex was already reviewing diffs from the file system, not the Editor. But any future Codex Editor-driving sessions are now blocked until the user disconnects Claude. Documented; not a deal-breaker.

### Constraints

- **Bake-time content discipline (CLAUDE.md §3)** is unchanged. Asset-generation tools are bake-time-only by definition — they run in the Editor, write to `Assets/Generated/**` (path TBD), and feed the same content-pack ID + schema-version pipeline as hand-authored content. No runtime LLM invocations; no speculative generation outside a tracked SPEC task. The §3 ban survives intact.
- **Determinism (CLAUDE.md §3, ADR-0001)** is unchanged. MatchSim canonical state is pure C#, zero UnityEngine references; the Unity Editor automation surface (this ADR's scope) does not touch the canonical pipeline. Pinned 60-tick `MatchCanonicalState` hash unaffected.
- **Two-way door.** Reversibility is a hard requirement. `.mcp.json` keeps both servers registered. Routing entries can shift back to CoplayDev primary by editing the routing table — no `.mcp.json` change required.

---

## Decision

The official Unity AI Assistant MCP (`com.unity.ai.assistant 2.7.0-pre.3`, server name `UnityAIAssistant`, stdio relay) becomes the **PRIMARY** editor automation surface for Final Whistle from Phase 3 onward. CoplayDev `unity-mcp` (server name `UnityMCP`) remains installed and registered as the **FALLBACK** for capability gaps the official MCP does not cover today and for entitlement-failure recovery.

Routing is defined in full at [`docs/tooling/unity-mcp-routing.md`](../../docs/tooling/unity-mcp-routing.md). Summary by gap:

- **Official primary:** all C# in-Editor execution, scene/console/file introspection, semantic script edits, menu-item invocation, scene captures (2D/3D/per-camera), profiler, asset generation, audio editing, external model import, GameObject/scene/asset CRUD.
- **CoplayDev fallback:** UPM package install/remove (`manage_packages` — official `Unity_PackageManager_ExecuteAction` exists but is `McpAvailability.Available`, off by default until user-toggled), granular prefab/animation/VFX/probuilder ops where official `Unity_ManageAsset` is too general, explicit transactional `batch_execute`, custom-tool registry browsing.

Claude holds the cap=1 direct slot. Codex continues as file-system + `scripts/fw verify` reviewer; if Codex needs an Editor session, the user explicitly disconnects Claude first.

## Consequences

### Positive

- Asset-generation surface unlocks the Phase 5/6 3D-pipeline spike at lower human-bake cost. This was the largest unknown in `design/3d-pipeline.md` outside of the polish-bar question itself.
- Semantic C# edits (`replace_method` / `replace_class` / `anchor_*`) reduce edit-drift on `MatchSim/**` and `unity-project/Assets/**` vs raw text replace. Determinism-sensitive code (`MatchSim/Sim/Q3232.cs`, ball physics, BT runner) is exactly the kind of file where semantic edits beat regex.
- `Unity_ManageMenuItem` ends the "describe menu path" anti-pattern definitively. Anything reachable from the Editor menu bar is now scriptable.
- `Unity_SceneView_Capture2DScene` + `Unity_SceneView_CaptureMultiAngleSceneView` cleanly replace the hand-rolled `manage_camera` + screenshot dance for L2 evidence in the `unity-check` skill.
- 11 granular profiler tools enable programmatic perf-budget enforcement in Phase 7. `gameplay-programmer` and `engine-programmer` agents can query GC alloc per frame range without user intervention.
- File SHA + size + last-modified via `Unity_GetSha` is a small but real win for cross-platform determinism verification.

### Negative

- **Entitlement fragility.** Loss of Pro seat or pricing-model shift post-beta = MCP goes dark. Mitigation: CoplayDev stays installed; routing table can shift entries back to CoplayDev primary in <30 min by editing the routing table.
- **Pre-release churn risk.** Tool names + shapes can change before stable `2.7.0`. Mitigation: pin to `2.7.0-pre.3` in `manifest.json`; only upgrade after live-testing the new tool surface; amend ADR + routing table on stable release.
- **Cap=1 single-driver.** Codex loses Editor access for the duration. Mitigation: documented; Codex's existing role (file-system review + `fw verify`) does not require Editor MCP; cross-model rhythm in §6.3 is preserved.
- **Two-MCP cognitive overhead.** Routing table is the load-bearing artifact that prevents accidentally hitting CoplayDev for tasks the official MCP does better. Mitigated by the table + by §6.3 mandating subagent rotation (each agent reads the routing table on invoke).

### Reversibility

**Two-way door.** Explicit revert path:

1. CoplayDev `unity-mcp` package remains installed in `unity-project/Packages/manifest.json` and registered in `.mcp.json` for the duration of Phase 3 → Phase 7 minimum. Removal is deferred to post-Phase-7 review per the deprecation gates in the routing table.
2. To revert: edit `docs/tooling/unity-mcp-routing.md` to flip "Primary" back to CoplayDev for affected rows; restart MCP servers; re-test the affected workflows. Estimated revert cost: <1 session.
3. ADR-0011 supersession: append a new ADR (ADR-0012+) citing this one. Do NOT edit ADR-0011 in place per the SPEC append-only convention.

## Alternatives considered

**A) Keep CoplayDev only.** Lowest-disruption option. Forfeits asset-generation (Phase 5/6 spike acceleration), granular profiler (Phase 7), `Unity_ManageMenuItem` (ongoing convenience), 2D/multi-angle scene capture (Phase 3 L2 evidence quality). The Pro seat is already paid for — keeping CoplayDev only would waste that spend. **Rejected.**

**B) Deprecate CoplayDev entirely now.** Cleanest single-MCP setup. Forfeits UPM package install/remove (off-by-default on official), granular prefab/animation/VFX ops, explicit `batch_execute`. Worse: removes the entitlement-failure fallback, leaving us with no Editor automation at all if the Pro seat lapses or Unity changes the entitlement model. **Rejected** — premature given pre-release status.

**C) Wait for stable `2.7.0` before migrating.** Avoids pre-release churn risk. Costs: the Pro seat is already paid; the asset-generation tools are exactly what accelerates the Phase 5/6 spike that is the dominant unknown right now; we'd be sitting on capability for an unscheduled stable date. **Rejected.** Migrating to pre-release with CoplayDev as fallback gives us the upside while bounding the downside.

**D) Run both as co-equal primaries with task-class routing only.** What we are effectively doing, but without the "official is primary" framing. **Rejected** because the framing matters: subagents need an unambiguous default when a task could route to either MCP. The default is official; CoplayDev is invoked only on the listed gap rows.

## Acceptance criteria

- [x] Both MCPs visible in `claude mcp list`.
- [x] One `Unity_ManageEditor GetState` tool call against `UnityAIAssistant` returns valid response (live-tested 2026-05-09).
- [x] `Unity_RunCommand` with `IRunCommand` template compiles + executes successfully.
- [x] `unity-project/Packages/manifest.json` pins `com.unity.ai.assistant 2.7.0-pre.3` and retains `com.coplaydev.unity-mcp`.
- [x] Routing table at `docs/tooling/unity-mcp-routing.md` covers ≥30 task classes with primary + fallback + status columns.
- [x] CLAUDE.md §3 / §4 / §8 reflect the new primary; SETUP.md reflects the new install path; TOOLING.md catalog updated.
- [ ] **Deferred to next session** (per autonomous-mode constraint): Codex review pass on this ADR + routing table.

## Review trail

- **2026-05-09 (drafted + Accepted, autonomous mode):** Authored by Claude in autonomous mode per user instruction (user was unavailable for live review; explicitly authorized the migration tonight). Drafted by `technical-director` subagent; corrected by main thread (server names, ADR path); cross-checked against `feature-dev:code-explorer` change-surface inventory and `producer` migration plan. **No Codex round-trip yet** — explicitly deferred to next session.
- **Next session expected:** Codex review pass on the ADR text, routing-table contents, and the broader migration commit set. Cross-model rhythm per CLAUDE.md §6.3 expected to flag any blind spots (different-bones-different-blindspots discipline).

## References

- Unity AI Assistant package: `com.unity.ai.assistant 2.7.0-pre.3` (manifest.json verified 2026-05-09)
- CoplayDev unity-mcp: `com.coplaydev.unity-mcp` (UPM, SHA-pinned in manifest)
- `MaxDirect = 1` entitlement: per Unity Pro/Enterprise seat
- Live-test session 2026-05-09: 12+ tools confirmed working
- [docs/tooling/unity-mcp-playbook.md](../../docs/tooling/unity-mcp-playbook.md) — deep-research artifact
- [docs/tooling/unity-mcp-routing.md](../../docs/tooling/unity-mcp-routing.md) — operational routing matrix
- [docs/plans/unity-mcp-migration-plan.md](../../docs/plans/unity-mcp-migration-plan.md) — migration sequencing + risk register
- Cross-refs: [ADR-0008](adr-0008-shot-presentation-contract.md) (ShotPresentationContract — renderer-agnostic; this migration does not change the contract), [ADR-0009](adr-0009-dots-phase-render-adapter.md) (dots-phase adapter — L2 evidence flow benefits directly from `Unity_SceneView_Capture2DScene`)
- `CLAUDE.md §3` — runtime LLM ban remains intact; asset-generation tools are bake-time-only by definition
- `CLAUDE.md §6.3` — subagent rotation table; each invoked agent should read the routing table on entry
- Unity Discussions thread: [CLI + MCP + AI IDE: Next Steps for Unity Workflow Automation](https://discussions.unity.com/t/cli-mcp-ai-ide-chatgpt-codex-cursor-antigravity-claude-code-windsurf-next-steps-for-unity-workflow-automation/1705679) — May 4 2026 open beta announcement

---
name: match-replay
description: Replay a deterministic match from a seed end-to-end — validate canonical determinism against the corpus fixture, drive the Unity dots-viewer Play mode, capture viewer frames at specified ticks, and return the screenshot paths. Phase-3 minimum scope per SPEC line 152. Wires together `scripts/fw replay --compare-corpus` (canonical-determinism gate) + UnityMCP scene/Editor/camera APIs (Play-mode capture path). Phase-4+ adds headless batch-mode capture + video recording. Use this when the user wants to SEE a specific seed run (devlog clips, regression visual diff, "show me what happened with this seed", semantic-cinema A/B comparison). Triggers on "replay this seed", "match replay", "capture match", "show me the smoke fixture", "/match-replay <seed>".
triggers:
  - replay this seed
  - match replay
  - match-replay
  - capture match
  - show me the smoke fixture
  - replay <seed>
  - viewer capture
---

# Match-replay — seed → headless match → viewer capture

Given a canonical match seed, drive a deterministic replay through the Phase-3 dots viewer and capture frames at semantic milestone ticks. Output is a set of `docs/screenshots/replay-<seed>-tick-NNN.png` captures the user can inspect, share, or diff against prior runs.

Built on the pieces that landed Slices 1-7 of the dots-adapter ladder (per [`docs/plans/dots-adapter-blueprint.md`](../../../docs/plans/dots-adapter-blueprint.md)) plus the Phase-3 enforcement skeletons (`scripts/fw replay <seed> --compare-corpus`) shipped 2026-04-30.

## When to invoke

| Situation | Reason |
|---|---|
| User asks to see a specific seed render | The primary use case — visual replay |
| Devlog / external-audience clip authoring | Capture canonical sequences for sharing |
| Cross-platform visual diff investigation | Drift between Win/Mac/Linux — capture each side + compare |
| Regression: a viewer-side change shouldn't alter framing for a known seed | Capture pre + post; diff PNGs |
| Semantic-cinema A/B comparison | Run two seeds side-by-side, compare shot rhythm |

## When NOT to invoke

- Determinism check WITHOUT a viewer capture — that's `scripts/fw replay <seed> --compare-corpus` direct, no skill orchestration needed.
- Headless batch-mode capture (no Editor) — Phase-4+ scope.
- Video / Unity Recorder timeline capture — UnityMCP exposes single-frame screenshots; multi-frame via repeated calls. Phase-4+ replaces with proper recorder integration.
- Corpus regeneration — `scripts/fw replay --regenerate-corpus` (Phase-4+; not Phase-3).
- Multi-seed batch runs — call this skill multiple times instead.

## Pre-flight (one-time per session)

1. **CoplayDev `UnityMCP` connected at HTTP `:8080`.** Verify via `claude mcp list`. If the Unity AI Assistant MCP is the active surface, the routing per [ADR-0011](../../../design/adr/adr-0011-unity-ai-assistant-mcp-migration.md) + [`docs/tooling/unity-mcp-routing.md`](../../../docs/tooling/unity-mcp-routing.md) still works for scene / GameObject / camera ops.
2. **`scripts/fw verify` currently green on `main`.** A broken baseline contaminates every capture — fix that first.
3. **Unity Editor open on the project** (not just MCP connected). Play-mode capture requires the Editor process. CoplayDev's HTTP server runs inside Unity; if the Editor isn't open, MCP returns "not connected."

## Workflow (Claude executes)

### Step 1 — Validate seed + corpus

**Canonicalize first + client-side membership check.** The user may provide a seed in several forms (`0xdeadbeefdeadbeef`, `0XDEADBEEFDEADBEEF`, raw `deadbeefdeadbeef`). `scripts/fw replay` at Phase 3 supports exactly **two** lowercase 0x-prefixed canonical seeds: `0xdeadbeefdeadbeef` (Tier-A smoke fixture, no signatures fire naturally) and `0xfeedbeefcafefade` (C4 LowCutback primed fixture, signatures fire tick 0 — added 2026-05-11). Anything else prints a friendly per-seed list and **exits rc=2** (verified at HEAD: `scripts/fw replay deadbeefdeadbeef --compare-corpus` and `scripts/fw replay 0x0000000000000000 --compare-corpus` both rc=2). The skill must therefore CHECK SEED MEMBERSHIP CLIENT-SIDE before invoking `fw replay`, otherwise an unsupported-seed rc=2 gets mislabeled as a canonical-determinism break further down the failure-modes table. Canonicalize BEFORE invoking:

```sh
# Pseudocode canonicalization (skill orchestrator does this):
seed_raw="$user_input"               # e.g. "DEADBEEFDEADBEEF" or "0xdeadbeefdeadbeef"
seed_clean="${seed_raw#0x}"           # strip 0x/0X
seed_clean="${seed_clean#0X}"
seed_lower="$(printf '%s' "$seed_clean" | tr '[:upper:]' '[:lower:]')"
# Validate 16 hex digits exactly:
if [[ ! "$seed_lower" =~ ^[0-9a-f]{16}$ ]]; then
  abort "malformed seed: expected 16-hex-digit form"
fi
seed_canonical="0x${seed_lower}"
```

Then **client-side membership check** before any `fw replay` invocation:

```sh
# Phase-3 supported set (extended 2026-05-11 by C4 commit 10efbdf):
SUPPORTED_SEEDS=("0xdeadbeefdeadbeef" "0xfeedbeefcafefade")
is_supported=false
for s in "${SUPPORTED_SEEDS[@]}"; do
  if [ "$seed_canonical" = "$s" ]; then is_supported=true; break; fi
done

if ! $is_supported; then
  # DO NOT call fw replay — it would exit rc=2 and the user-facing failure
  # would conflate with the canonical-determinism-break path below.
  ask user "seed $seed_canonical has no corpus fixture at Phase 3 (only the Tier-A smoke seed is supported). Capture anyway without the determinism gate? [y/N]"
  # Default abort. If user opts in, SKIP Step 1's hash-compare entirely
  # and proceed to Step 2 with a loud note that the capture has no
  # canonical-determinism backing.
fi
```

Only when the supported-seed gate passes, run the actual hash compare:

```sh
scripts/fw replay "$seed_canonical" --compare-corpus
```

For the supported seed: locates the matching corpus fixture (`MatchSim.Tests/fixtures/replay-corpus/<seed>.json`) + runs `MatchSimulationRunner` (via `FromArchetypeFormations` for the smoke fixture or `FromLowCutbackPrimedFixture` for the C4 primed fixture per the `fixture_factory` field in the JSON) + computes `MatchCanonicalState` hash + compares. **rc=0 on match; rc non-zero on hash mismatch — and at this point the only way to get non-zero is a real determinism/tooling failure (because the unsupported-seed rc=2 path was filtered out client-side).**

**Fail-paths:**
- Malformed seed (canonicalization fails the 16-hex regex) → user-facing error; abort skill, do NOT capture.
- Seed canonicalizes but is unsupported → user prompt above; default abort; on opt-in proceed without the gate + with a loud caveat in Step 6's report.
- Hash mismatch on a supported seed (`fw replay` rc≠0 AFTER passing the supported-seed client gate) → **escalate to user**: real determinism break; capturing the broken replay would propagate the bug. Do NOT continue.

### Step 2 — Load the DotsViewer scene

```python
mcp__UnityMCP__manage_scene(
    action="load",
    path="Assets/Viewer/Adapters/Dots/Scenes/DotsViewer.unity"
)
```

### Step 3 — Set the match seed on the director

```python
mcp__UnityMCP__find_gameobjects(
    search_term="FinalWhistle.Viewer.Adapters.Dots.DotsMatchDirector",
    search_method="by_component"
)
# → returns instance ID for the director GameObject
mcp__UnityMCP__manage_components(
    action="set_property",
    target=<directorInstanceID>,
    component_type="FinalWhistle.Viewer.Adapters.Dots.DotsMatchDirector",
    property="matchSeedHex",
    value="<seed>"
)
```

The director's `OnValidate` will surface malformed seeds at inspector-edit time; the Awake throw is the load-bearing loud-fail.

### Step 4 — Enter Play mode + capture (state-driven; never wall-clock-only)

**Capture file names MUST reflect the actual canonical tick the screenshot was taken at.** A label of `tick-540` for a frame captured at tick 30 is false evidence — the artifact loses its diagnostic value + propagates the lie into PRs / devlogs / Codex review. The skill therefore polls `DotsMatchDirector.state.CurrentTick.Value` via UnityMCP `execute_code` BEFORE each capture; only fires `manage_camera` once `currentTick >= target_tick_marker`.

```python
mcp__UnityMCP__manage_editor(action="play")
# Brief settle for Awake + Start + first FixedUpdate to bootstrap state.

target_ticks = [30, 180, 540]   # 0.5s, 3s, 9s @60Hz; configurable
max_wait_seconds_per_tick = 30  # generous; sim runs at 60Hz so 540 ticks = ~9s wall-clock
                                # plus settle margin

for target in target_ticks:
    start_wall = time.time()
    while True:
        result = mcp__UnityMCP__execute_code(
            action="execute",
            code=(
                "var director = GameObject.Find(\"DotsMatchDirector\").GetComponent<"
                "FinalWhistle.Viewer.Adapters.Dots.DotsMatchDirector>();"
                "var stateField = typeof(FinalWhistle.Viewer.Adapters.Dots.DotsMatchDirector)"
                ".GetField(\"state\", System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);"
                "var state = stateField.GetValue(director);"
                "var tickProp = state.GetType().GetProperty(\"CurrentTick\");"
                "var tick = tickProp.GetValue(state);"
                "var valueProp = tick.GetType().GetProperty(\"Value\");"
                "return (long)valueProp.GetValue(tick);"
            )
        )
        current_tick = int(result.data.result)
        if current_tick >= target:
            break
        if time.time() - start_wall > max_wait_seconds_per_tick:
            # The sim is stuck / not advancing. ABORT — do NOT capture
            # a tick-N labelled frame that wasn't actually at tick N.
            mcp__UnityMCP__manage_editor(action="stop")
            raise SkillAbort(
                f"sim did not advance to tick {target} within "
                f"{max_wait_seconds_per_tick}s; aborting capture before "
                "producing false evidence")
        time.sleep(0.25)
    # Now currentTick >= target; capture with the ACTUAL tick in the filename.
    mcp__UnityMCP__manage_camera(
        action="screenshot",
        screenshot_file_name=f"replay-<seed>-tick-{current_tick:04d}.png"
    )

mcp__UnityMCP__manage_editor(action="stop")
```

Notes:
- **Filename uses the actual `current_tick`**, not `target`. If polling lag caused the read to land at tick 545 instead of exactly 540, the file is named `tick-0545` to match reality.
- **State access via reflection** because `DotsMatchDirector.state` is `private` — the skill is a diagnostic surface and shouldn't promote `state` to `public` just for this. If a Phase-4+ refactor exposes `CurrentTick` as an internal accessor, swap the reflection call for a direct read.
- **Polling cadence 250ms** — tight enough that overshoot is bounded to ~15 ticks, loose enough not to thrash the Editor.
- **Hard timeout per target** prevents a hung sim from looping forever.

Wall-clock: for target ticks `[30, 180, 540]` at 60Hz, the polling waits ~0.5s + 2.5s + 6s for the sim to reach each + ~1-2s per capture = total ~14-18s. The state-driven discipline costs a few seconds vs the prior wall-clock-only loop, but produces honest evidence.

### Step 5 — Move captures to `docs/screenshots/`

```sh
mkdir -p docs/screenshots
cp unity-project/Assets/Screenshots/replay-<seed>-tick-*.png docs/screenshots/
find unity-project/Assets/Screenshots -type f -delete  # clean tmp
rmdir unity-project/Assets/Screenshots 2>/dev/null
```

`unity-project/Assets/Screenshots/` is the auto-created tmp dump-zone from UnityMCP's screenshot path; never committed; always cleaned post-capture.

### Step 6 — Report

Return to user:
- The 3 capture paths under `docs/screenshots/`.
- The corpus-compare result from Step 1 (`PASS: hash matches expected sha256:...`).
- A one-line description of what's visible per tick (formation snapshot / mid-play / late-play).
- If anything failed (Editor not open, MCP timeout, etc.), report which step + the user-actionable fix.

## Failure modes + recovery

| Symptom | Cause | Fix |
|---|---|---|
| `fw replay --compare-corpus` exit non-zero **after passing the client-side supported-seed gate** | Real canonical determinism break (hash mismatch) — only reachable for the two Phase-3 supported seeds | DO NOT capture. Escalate to user. The replay would propagate the regression. |
| Canonicalized seed is not in the supported-set (`{0xdeadbeefdeadbeef, 0xfeedbeefcafefade}` at Phase 3) | User asked for a seed Phase-3 doesn't have a corpus fixture for | Skill prompts "capture without the determinism gate? [y/N]"; default abort. **Do NOT invoke `fw replay` here — it would rc=2 with a per-seed list and conflate with the genuine determinism-break row above.** Phase-6 corpus expansion lifts the restriction. |
| Canonicalization step rejects the seed as malformed | Seed not 16 hex digits (post-`0x`-strip) | User error — return the canonicalization error verbatim so they can correct the input. |
| State-poll via `execute_code` reflection returns null / throws | `DotsMatchDirector` not in the active scene OR field rename | Verify DotsViewer.unity is the active scene + `state` field still private+named that way. Fix the reflection or escalate if a refactor renamed the field. |
| Sim doesn't advance to target tick within timeout | Director paused / driveSim toggled off / Editor not playing | Abort capture (do NOT ship tick-NNN labelled frames at the wrong tick). Verify director's `driveSim` is true + Play mode is active. |
| UnityMCP `manage_scene` returns "scene not found" | DotsViewer scene unsaved or path drifted | Open Unity → verify scene path → save. |
| `manage_editor action=play` returns "compilation errors" | Compile broke since last verify | Run `scripts/fw verify` + read Unity console → fix → retry. |
| Screenshots written but appear blank/black | Editor window not focused OR scene not finished loading | Increase the per-target polling timeout. UnityMCP `manage_camera screenshot` is async — wait for the response before next call. |
| Camera shows tactical-wide only (no signature shots) | Smoke fixture (0xdeadbeefdeadbeef) doesn't fire signature events naturally (the chaotic 22-player pressing never satisfies trigger gates per C1 audit bb136866 + C4 closure). | **Use 0xfeedbeefcafefade instead** (C4 primed-for-LowCutback fixture, commit 10efbdf): signature fires tick 0, goal scored ~tick 50, commentary banner shows. Toggle `DotsMatchDirector.usePrimedFixture=true` in scene inspector. For seeds Phase-3 doesn't yet support, wait for Phase-6 corpus expansion. |

## Cross-references

- [SPEC line 152](../../../SPEC.md) — Phase-3 acceptance line for this skill.
- [`docs/plans/dots-adapter-blueprint.md`](../../../docs/plans/dots-adapter-blueprint.md) — the 7-slice ladder this skill renders the output of.
- [ADR-0009 dots-phase render adapter](../../../design/adr/adr-0009-dots-phase-render-adapter.md) — polish-bar criteria.
- [`design/specs/golden-replay-corpus.md`](../../../design/specs/golden-replay-corpus.md) — the corpus contract enforcement-skeleton ships against.
- [/duo-implement](../duo-implement/SKILL.md) — sibling skill for orchestrating Tier-2 coding tasks; not the same use case but shares the polling-+-escalation patterns.
- [`scripts/fw`](../../../scripts/fw) — the `replay <seed> --compare-corpus` subcommand this skill wraps.

## Phase-4+ follow-up (not in scope here)

- Headless batch-mode capture (`-batchmode -nographics` Unity invocation; no open Editor).
- Unity Recorder integration for proper video (MP4) capture across a full match.
- Multi-seed batch runs with side-by-side composition.
- `scripts/fw replay --regenerate-corpus` integration (regenerate the expected hash from a fresh run; gated on `golden-replay-corpus.md` schema-version protocol).
- Auto-detect signature-firing ticks from `state.SignatureRecipes` + capture at those specific moments.

## Sanity checklist before invoking

- [ ] Does the user-provided seed canonicalize to a 16-hex-digit `0x`-prefixed lowercase form (after `0x`/`0X` strip + lowercase + 16-hex regex)? If no → user error; return the validation message.
- [ ] Does the canonicalized seed match one of `{0xdeadbeefdeadbeef, 0xfeedbeefcafefade}` at Phase 3? Phase-6 corpus expansion adds more; until then any other seed gets the "soft-skip + capture-without-gate? [y/N]" prompt.
- [ ] Is `scripts/fw verify` currently green? If no → fix that first.
- [ ] Is the Unity Editor open with the project loaded? If no → CoplayDev UnityMCP can't reach the Editor.
- [ ] Have you cleaned old captures from `unity-project/Assets/Screenshots/`? If no → captures may collide.
- [ ] Does the skill's state-poll reflection still work? (`DotsMatchDirector.state` field name unchanged; `CurrentTick.Value` accessor intact.) A Phase-4+ refactor that renames either breaks the capture loop loudly per the failure-modes table.

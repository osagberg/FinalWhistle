---
name: unity-webgl-builder
description: Produce a WebGL build of the Unity project for browser QA, Steam page demo, dev-log GIF capture, or pre-release smoke testing. Uses unity-mcp when available, falls back to batchmode. Triggers on "webgl build", "build for web", "browser build", "make a demo build", "webgl regression", "run a webgl".
triggers:
  - webgl build
  - build webgl
  - browser build
  - web build
  - demo build
  - webgl regression
  - make a webgl
---

# Unity WebGL Builder — automated browser-playable builds

Produce a WebGL build artifact with deterministic settings. Not a polish pass — a build-system skill. Good output = a folder under `Build/WebGL/` with an `index.html`, written build report at `Library/WebGLBuildReport.json`, and a reported artifact size. Bad output = partial folder, silent failure, mystery size.

## When to invoke

| Situation | Trigger |
|---|---|
| `/done` on the Phase-3 "first WebGL build" task | Produce baseline artifact, record size + warnings. |
| Periodic regression (weekly-ish) | Detect size creep, new shader warnings, new exceptions. |
| `/release-checklist` demo build step | Feeds [github-pages-deploy](../github-pages-deploy/SKILL.md) if a public URL is needed. |
| `/hotfix` pre-push verification | Confirm no WebGL-only breakage before tagging a hotfix. |
| User asks for a browser-playable share | Output folder → zip → share, or chain into pages deploy. |

Skip for: pure design/doc tasks, non-runtime refactors, platform-specific (iOS/Android) work.

## Integration

- Pairs with [unity-check](../unity-check/SKILL.md) — L1/L2 should pass before a WebGL build is worth attempting.
- Pairs with [state-dump](../state-dump/SKILL.md) for post-build smoke: run the built game in a browser (via `python3 -m http.server`), `state-dump` can't reach it without hooks, so smoke is visual-only.
- Chains into [github-pages-deploy](../github-pages-deploy/SKILL.md) for public demo URL (respect project identity-firewall rules first).
- Uses batchmode per patterns in [/Users/vibelogic/dev/blueprint/unity/batchmode-gotchas.md](/Users/vibelogic/dev/blueprint/unity/batchmode-gotchas.md).

## Preconditions

1. Unity project exists at `unity-project/` (or bootstrap-configured path).
2. WebGL module installed for the active Unity version (`Unity Hub → Installs → Add Modules → WebGL Build Support`). If missing, batchmode fails with "Could not find WebGL".
3. Active render pipeline is URP with WebGL-compatible shaders. Built-in HDRP shaders will error. lilToon is WebGL-compatible with warnings.
4. At least one scene is in `Build Settings → Scenes In Build`. Empty list → build error `No scenes in build`.
5. `WebGLBuilder.cs` is present under `Assets/_Project/Editor/Build/` (see [templates/WebGLBuilder.cs](templates/WebGLBuilder.cs)). Copy at bootstrap.

## Procedure

### Step 1 — Handshake check

```
Bash: claude mcp list | grep unity-mcp
```

- Green → MCP path (Step 2a).
- Missing/red → batchmode fallback (Step 2b).

### Step 2a — MCP path (preferred, fast feedback)

1. `manage_editor(action="set_active_build_target", target="WebGL")` — switch platform if needed. First switch on a fresh project is slow (2–10 min recompile); subsequent switches are seconds.
2. `execute_menu_item(menu_path="{{PROJECT_NAME}}/Build/WebGL")` — invokes `WebGLBuilder.Build()`.
3. Wait for completion. Builds take 1–5 minutes depending on asset count and IL2CPP first-run.
4. Read `unity-project/Library/WebGLBuildReport.json`. Parse:
   - `result` — `Succeeded` / `Failed` / `Cancelled`
   - `totalSizeBytes` — final artifact size
   - `outputPath` — absolute path to folder with `index.html`
   - `warnings[]` / `errors[]` — deduped per-step messages
5. If `result != Succeeded` → report failure (see Failure modes).

### Step 2b — Batchmode fallback

Use when MCP is down or running in CI-like conditions.

```bash
UNITY_BIN="/Applications/Unity/Hub/Editor/6000.4.3f1/Unity.app/Contents/MacOS/Unity"
"$UNITY_BIN" \
  -batchmode -quit -nographics \
  -projectPath "$PROJECT_ROOT/unity-project" \
  -buildTarget WebGL \
  -executeMethod {{PROJECT_NAME}}.Editor.Build.WebGLBuilder.Build \
  -logFile /tmp/webgl-build.log
```

After exit:
1. Check exit code (0 ≠ success on its own — see batchmode-gotchas.md).
2. Read `Library/WebGLBuildReport.json` — source of truth for success.
3. Grep log for `error CS`, `Shader error`, `BuildFailedException`, `IL2CPP error` if report is missing.

### Step 3 — Verify artifact

1. `ls Build/WebGL/` — expect `index.html`, `Build/`, `TemplateData/` (default template) or equivalent.
2. Total size should be 15–50 MB for a minimal URP project, 50–200 MB with real content. Report exact `du -sh`.
3. If artifact is <5 MB → something stripped too aggressively; flag for investigation.

### Step 4 — Report

```
webgl-build PASS
  result      : Succeeded
  scenes      : 2 (Boot.unity, Main.unity)
  size        : 47.3 MB (compressed: Brotli)
  path        : /Users/.../unity-project/Build/WebGL
  duration    : 128.4s
  warnings    : 3 (shader keyword pressure on lilToon_Body)
  preview     : cd Build/WebGL && python3 -m http.server 8000  # http://localhost:8000
```

On failure:

```
webgl-build FAIL
  result      : Failed
  error       : Shader error in 'lilToon/Opaque': redefinition of '_MainTex'
  step        : "Compile shaders"
  report      : Library/WebGLBuildReport.json
  log         : /tmp/webgl-build.log (line 3421)
  remediation : see Failure modes § shader-keyword-count below
```

## Platform-specific considerations

### Compression

Default template config writes `WebGLCompressionFormat.Brotli`. Pros: 30–50% smaller than uncompressed; cons: requires server with `Content-Encoding: br` headers (GitHub Pages supports it; `python3 -m http.server` does NOT — you get a blank page + CORS-adjacent errors in browser console).

- Local preview with `python3 -m http.server` → pass `compression=disabled` to `BuildWithConfig` OR set `PlayerSettings.WebGL.compressionFormat = WebGLCompressionFormat.Disabled` before the run.
- GitHub Pages / real static host → Brotli (default) is fine.

### Memory size

`PlayerSettings.WebGL.memorySize` defaults to 256 MB in this template. Too low → `Out of memory` in browser. Too high → load failure on mobile. Rules of thumb:
- Desktop demo: 256–512 MB.
- Mobile-friendly: 128–256 MB (test on actual phone — chrome://crash not helpful).

### WebGL template

Use `Default` or `Minimal` template only. Third-party URP templates exist but often strip Unity's loader boilerplate in ways that break on mobile. First WebGL build → stay on `Default`.

### Exception support

`PlayerSettings.WebGL.exceptionSupport = WebGLExceptionSupport.ExplicitlyThrownExceptionsOnly` is the sensible default. Full exception support inflates size ~20%. None → silent failures in prod. Don't change unless you have a reason.

## Failure modes

### shader-keyword-count

Symptom: `Maximum number (256) of shader keywords exceeded`.
Cause: Too many shader variants — usually lilToon + URP combo + many materials.
Fix: Use Shader Stripping (Project Settings → Graphics → Shader Stripping → set Instancing Variants to Strip Unused; URP Asset → Shader Variant Log Level = Disabled). Re-run build.

### il2cpp-compile-error

Symptom: `IL2CPP error for type ... in assembly ...` after several minutes.
Cause: Reflection-using code hit IL2CPP's AOT limits (Newtonsoft.Json is a common source — fine for editor dump, but don't let it reach runtime WebGL paths) or forbidden API (`System.Threading.Thread`, `System.Net.Sockets`).
Fix: Wrap offending code in `#if !UNITY_WEBGL`. If third-party, check for WebGL-compatible fork.

### missing-webgl-module

Symptom: Batchmode exits with `Could not find WebGL ...` before `-executeMethod` runs.
Cause: Unity version is installed but WebGL Build Support module isn't.
Fix: `Unity Hub → Installs → <version> → Add Modules → WebGL Build Support`. ~500 MB download.

### empty-scenes-list

Symptom: Report says `BuildFailedException: No scenes in build`.
Cause: `EditorBuildSettings.scenes` is empty in the project.
Fix: Add at least one scene via Unity GUI or programmatically in `WebGLBuilder.Build` (the template pulls from `EditorBuildSettings.scenes` by default — populate it via MCP `manage_scene` if needed).

### blank-page-locally

Symptom: Build succeeds, `python3 -m http.server` serves it, browser shows blank page with console `Uncaught RangeError` or CORS-adjacent errors.
Cause: Compression enabled against a server that doesn't set `Content-Encoding`.
Fix: Rebuild with compression disabled, OR use a server that handles Brotli (e.g. GitHub Pages, or `npx serve` with a Brotli config).

### urp-webgl-2-vs-webgl-3

Symptom: Pink/magenta materials on every surface despite lilToon in editor.
Cause: WebGL 1.0 target + URP shader requiring GL ES 3.0. Rare on Unity 6 (WebGL 2.0 default) but easy to hit if `PlayerSettings.WebGL.useWasmLoader` or build-target overrides are stale.
Fix: `PlayerSettings.WebGL.linkerTarget = WebGLLinkerTarget.Wasm;` and `EditorUserBuildSettings.SetPlatformSettings("WebGL", "GraphicsApiType", "WebGL2")`.

## BuildWithConfig — ad-hoc overrides

For one-off builds with different settings (e.g. uncompressed for local preview, or a specific scene subset):

```bash
cat > /tmp/webgl-config.json <<'EOF'
{
  "outputPath": "Build/WebGL-uncompressed",
  "scenes": ["Assets/_Project/Scenes/Smoke.unity"],
  "compression": "disabled",
  "memorySizeMB": 256,
  "development": true
}
EOF
```

Then invoke via MCP:

```
execute_menu_item("{{PROJECT_NAME}}/Build/WebGL With Config")
```

The template reads `/tmp/webgl-config.json` (path hardcoded; edit the const in `WebGLBuilder.cs` for a different location). Missing config → falls back to defaults.

## Related

- [templates/WebGLBuilder.cs](templates/WebGLBuilder.cs) — the editor script that powers the menu items + batchmode entrypoint.
- [../unity-check/SKILL.md](../unity-check/SKILL.md) — run L2 before spending 2 minutes on a build.
- [../github-pages-deploy/SKILL.md](../github-pages-deploy/SKILL.md) — deploy the artifact to GitHub Pages.
- [/Users/vibelogic/dev/blueprint/unity/batchmode-gotchas.md](/Users/vibelogic/dev/blueprint/unity/batchmode-gotchas.md) — batchmode edge cases (exit-code lies, async completion races).

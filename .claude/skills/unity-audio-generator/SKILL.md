---
name: unity-audio-generator
description: Generate procedural placeholder audio (SFX + BGM) as .wav files directly from a C# Editor script — no external libraries, no Asset-Store purchase, no licensing. Useful for prototyping phases 3–5 when you need "a click", "a hit", "a loop" without going shopping. Triggers on "placeholder sfx", "generate audio", "procedural sound", "make a beep", "need a sound effect", "bgm placeholder".
triggers:
  - placeholder audio
  - generate sfx
  - procedural sound
  - generate beep
  - generate bgm
  - make a sound
  - placeholder music
  - generate chime
---

# Unity Audio Generator — procedural placeholder .wav

Single editor script. Menu items trigger generators. Output is 44.1 kHz / 16-bit / mono PCM .wav files dropped under `Assets/_Project/Audio/SFX/_placeholder/` or `Assets/_Project/Audio/BGM/_placeholder/`. Each generated file has a sidecar `.meta.json` recording parameters, so regenerating with the same params yields identical output and so you can tweak without guesswork.

Nothing here is final audio. It is "good enough that you can iterate on gameplay feel without being stuck." When a sound matters enough to keep, replace it and delete the `_placeholder/` version.

## When to invoke

| Situation | Trigger |
|---|---|
| Prototyping a new interaction (button, pickup, hit) and you need auditory feedback now | Beep / chime / impact. |
| Testing BGM triggers / audio state machines without real music | BGM placeholder (ambient/chiptune/dungeon). |
| Debugging audio routing / mixer setup — need a known-frequency test tone | Beep at specific Hz. |
| UI transitions / loading screens need a sweep without sourcing one | Sweep generator. |

Skip for: anything where placeholder risks shipping (Phase 7+, any build that leaves your machine). Skip for voice, ambience, or layered cinematic audio — none of those are in scope.

## Integration

- Pairs with the project's `AudioManager` (if one exists) — `AudioManager` typically `[SerializeField] AudioClip fooClip;` and expects an `AudioClip` asset. Generated .wav → Unity auto-imports as `AudioClip` on `AssetDatabase.Refresh()`.
- Optionally pairs with a `design/audio-design.md` spec doc if you keep one — this skill can read a YAML/JSON list of required SFX and batch-generate them (see "Spec-driven batch" below).
- Does NOT replace real audio design. This is for prototyping gates, not for polish.

## Preconditions

1. `AudioGenerator.cs` is present under `Assets/_Project/Editor/Audio/` (see [templates/AudioGenerator.cs](templates/AudioGenerator.cs)). Copy at bootstrap.
2. Unity Editor is open (MCP path) or available via batchmode (less common for this skill — interactive param entry is editor-only).
3. Folder conventions: the script auto-creates `Assets/_Project/Audio/SFX/_placeholder/` and `.../BGM/_placeholder/`. If your project uses different paths, edit the `OutputRoot` const in the template.

## Procedure

### Step 1 — Pick a generator

Match the need to a generator:

| Need | Generator | Typical params |
|---|---|---|
| Simple "beep" / tone | `GenerateBeep(freq, duration)` | 440 Hz, 0.15 s |
| Shhhhh / whoosh / fizz | `GenerateNoise(duration, color)` | 0.3 s, white / pink / brown |
| Hit / impact / bump | `GenerateImpact(intensity)` | 0.5–1.5 intensity |
| UI confirm / coin / chime | `GenerateChime(baseFreq, overtones)` | 880 Hz, 3 overtones |
| Transition / loading / sweep | `GenerateSweep(startFreq, endFreq, duration)` | 200 → 1200 Hz, 0.5 s |
| Background loop | `GenerateBGM(genre, duration)` | ambient / chiptune / dungeon, 8 s |

### Step 2 — Invoke via MCP

Option A — one-shot menu:
```
execute_menu_item("FinalWhistle/Audio/Generate Beep")
```

Each generator exposes a `[MenuItem]`. When fired, the editor script opens an `EditorWindow` for parameters (in interactive editor sessions) or uses default parameters (in batchmode / headless mode).

Option B — parameterized via public static API:
```
# via manage_components or an ad-hoc MenuItem like "Generate From Config"
# writes /tmp/audio-gen.json with { "generator": "Beep", "freq": 440, ... }
execute_menu_item("FinalWhistle/Audio/Generate From Config")
```

The `GenerateFromConfig` menu reads `/tmp/audio-gen.json`, dispatches to the right generator, and writes output + sidecar. Good for agentic batch use.

### Step 3 — Find the output

```
Assets/_Project/Audio/SFX/_placeholder/beep_440hz_0p15s.wav
Assets/_Project/Audio/SFX/_placeholder/beep_440hz_0p15s.meta.json
```

The sidecar JSON captures generator name + all parameters + timestamp + random seed (if any). Same inputs → same output.

### Step 4 — Assign to an AudioSource / AudioManager

Generated files auto-import as `AudioClip`. Claude can:
- Use `manage_components` to assign an `AudioClip` reference (via GUID or path).
- Open the scene in Unity and drag-drop manually (fast for small edits).

Or let the project's `AudioManager` auto-scan `_placeholder/` by filename convention (out of scope for this skill — depends on your audio architecture).

## Spec-driven batch (optional pattern)

If you maintain `design/audio-design.md` with a YAML block like:

```yaml
sfx:
  - { name: ui_confirm,  generator: chime,  freq: 880, overtones: 3, duration: 0.2 }
  - { name: ui_cancel,   generator: beep,   freq: 220, duration: 0.1 }
  - { name: door_open,   generator: sweep,  start: 400, end: 120, duration: 0.6 }
  - { name: footstep,    generator: impact, intensity: 0.6 }
bgm:
  - { name: dorm_ambient, generator: bgm, genre: ambient,  duration: 16 }
  - { name: classroom,    generator: bgm, genre: chiptune, duration: 12 }
```

…Claude can parse that block, write each entry to `/tmp/audio-gen.json`, and invoke `Generate From Config` once per entry. The sidecar makes reproducibility trivial.

## Metadata sidecar format

Every generated file gets `<filename>.meta.json`:

```json
{
  "generator": "Beep",
  "filename": "beep_440hz_0p15s.wav",
  "params": {
    "frequency": 440.0,
    "duration": 0.15,
    "volume": 0.8,
    "decay": 5.0
  },
  "sampleRate": 44100,
  "channels": 1,
  "bitsPerSample": 16,
  "generatedAtUtc": "2026-04-21T09:15:22Z",
  "generatorVersion": 1,
  "seed": null
}
```

- `generatorVersion` increments when the template changes in a way that would alter output for the same params.
- `seed` is set for any noise-based generator (impact, noise, BGM percussion). Null for deterministic pure-math generators (beep, chime, sweep).

## Generator catalog (template defaults)

| Generator | Signature | Defaults | Output size |
|---|---|---|---|
| Beep | `GenerateBeep(freq Hz, duration s)` | 440 Hz, 0.15 s | ~13 KB |
| Noise | `GenerateNoise(duration s, color)` | 0.3 s, pink | ~26 KB |
| Impact | `GenerateImpact(intensity 0..1)` | 0.8 intensity | ~35 KB |
| Chime | `GenerateChime(baseFreq Hz, overtones int)` | 880 Hz, 3 | ~35 KB |
| Sweep | `GenerateSweep(startFreq, endFreq, duration)` | 200→1200 Hz, 0.5 s | ~44 KB |
| BGM | `GenerateBGM(genre, duration s)` | ambient, 8 s | ~700 KB |

BGM genres: `ambient` (sine pad + slow bass), `chiptune` (square + arpeggio), `dungeon` (minor mode sine + low pulse).

## Failure modes

### menu-item-missing

Symptom: `execute_menu_item` returns "menu path not found".
Cause: `AudioGenerator.cs` not in `Assets/_Project/Editor/Audio/` or has compile errors (blocks MenuItem registration).
Fix: Run [unity-check L1](../unity-check/SKILL.md) — compile errors anywhere in Editor assembly suppress menu registration.

### clipping-audible-distortion

Symptom: Generated sound audibly distorts / clips.
Cause: Parameter choices (high volume + additive overtones) exceed 16-bit headroom.
Fix: Lower `volume` param (default 0.8 is safe for most; drop to 0.5 for chime / layered BGM).

### bgm-doesnt-loop-cleanly

Symptom: Audible click at loop boundary.
Cause: BGM duration doesn't align to an integer multiple of the beat grid.
Fix: Pass duration as an integer number of beats. Template has a `SnapToBeat=true` param (default true) that handles this.

### assetdatabase-not-refreshing

Symptom: File appears in Finder but not in Unity Project window.
Cause: Skill wrote outside `AssetDatabase.Refresh()` scope (shouldn't happen with template, but can if you invoke via `Bash` directly).
Fix: `AssetDatabase.Refresh()` from any menu item, or right-click `Assets/` → `Refresh`.

### output-is-silent

Symptom: File exists, size is correct, but plays silently.
Cause: Volume param was 0, or all samples clamped to same value (common when decay param is very high and duration is very long).
Fix: Check the `.meta.json` sidecar — the params are there. Rerun with saner values.

## Related

- [templates/AudioGenerator.cs](templates/AudioGenerator.cs) — full editor script.
- [../unity-check/SKILL.md](../unity-check/SKILL.md) — L1 to confirm no compile errors block MenuItem registration.
- [../state-dump/SKILL.md](../state-dump/SKILL.md) — if audio playback triggers state changes worth verifying.

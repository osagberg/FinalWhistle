# TECH_APPROACH.md — Final Whistle engineering blueprint

> Engineering decisions: engine, sim architecture, determinism discipline, AI content compiler, folder layout, library choices. Read after `CLAUDE.md` + `PROJECT_CONTEXT.md`.
>
> Authored 2026-04-22. Revise at each phase transition + whenever a major tech decision is logged in SPEC.md.

---

## 1. Stack summary

| Concern | Choice | Rationale |
|---|---|---|
| Engine | **Unity 6 LTS** (pin at Phase 3) | Solo-dev cross-platform; Mac Editor strong; URP fits 2D stylized; Steam deploys clean |
| Render pipeline | **URP 17.x** | Lightweight, customizable, 2D-friendly, Apple Silicon + Steam Deck friendly for post-EA |
| Render mode | **ForwardPlus** | Tile-based clustering; optimal for Apple Silicon TBDR and desktop |
| Color space | **Linear** | Required for stylized post-processing passes + colour-grading cinema |
| **Canonical match sim** | **`MatchSim.csproj` — pure C#, zero-Unity, fixed-point arithmetic** | Headless 10,000× speed balance harness; cross-platform deterministic replay; xUnit-testable |
| **Ball physics** | **Custom deterministic sim** (not Unity PhysX) | Rocket League lesson; lockstep with MatchSim; controlled Magnus force / air drag |
| Async | **UniTask** | Zero-allocation, editor-playmode safe |
| UI layer | **UI Toolkit (UXML/USS)** | Modern, data-bindable, hot-reload, type-safe bindings; avoids uGUI nested-prefab hell |
| Data | **ScriptableObjects** (content) + **YAML** (behavior-tree archetypes) + **content packs** (stable IDs, schema versions) | SO for Unity-authored content; YAML diffable + Claude-readable for BTs; content packs modding-ready |
| Loading | **Addressables** | Streaming, memory-clean, future-proofs content-pack hot-reload |
| Audio | **FMOD Studio** (free indie) | Dynamic crowd layers, match music stings, scored match moments |
| Steam | **Steamworks.NET** | Achievements, cloud saves, Workshop readiness |
| Animation (viewer) | **Animancer Pro** (trigger Phase 4) | Stateless clip play; avoids Mecanim state-graph breakdown with signature actions |
| Cloth (viewer, deferred) | **Magica Cloth 2** (owned) | Anime hair + kit flutter for post-EA 3D push; no cloth at 2D MVP |
| Shader (viewer) | **URP Shader Graph + custom HLSL passes** | Manga-broadcast stylization (screen-tone, motion-lines, impact frames, state-driven colour grade) |
| Editor UX | **Odin Inspector** (trigger Phase 3 SO-authoring pain) | SO authoring quality-of-life at scale |
| AI at runtime | **NONE** (intentional trap to avoid) | Inference cost breaks match flow; bake-time delivers same variety at zero runtime cost |
| AI at bake time | **Claude Opus 4.7 + prompt caching** via AI Content Compiler pipeline | Worldbuilding, player generation, match-report templates, press pools — all structured JSON with lints |
| Tactical AI | **Behavior trees + hand-authored YAML archetypes** (not ML-Agents) | Deterministic, debuggable, balance-harness reproducible |

**Ruled out:**
- **Unreal / Godot:** Unity's Mac Editor + URP + Mono-derived build pipeline fits solo workflow better
- **HDRP:** fights stylized 2D; overkill for viewer; no 3D at MVP to justify
- **Ink / Yarn Spinner:** narrative is event-sourced systemic, not scripted branching
- **VRoid / UniVRM:** no 3D characters at MVP; deferred indefinitely
- **Unity ML-Agents:** training cost + opacity + non-determinism; BTs win for this shape of game
- **Runtime local LLMs:** ~3-15s inference on mid-range GPUs April 2026; breaks match flow; 4-8GB distribution bloat
- **Unity Jobs/ECS speculation:** MatchSim stays pure-C# first; port to Jobs only if Phase 6 perf demands, not speculatively

---

## 2. Visual target

Stylized 2D manga-broadcast match viewer. Diagonal pitch compositions, player portrait cut-ins, panelized key moments, motion-line runs, impact frames for tackles and shots, state-driven colour grading. Camera grammar vocabulary of **7 shot types** (see `design/semantic-cinema.md`):

1. `tactical-wide` — classic broadcast-style overview
2. `diagonal-attack-lane` — pitch tilted, attacking run emphasized
3. `player-isolation` — close on one player's face/stance
4. `duel-panel` — split-panel 1v1 emphasis
5. `pass-shot-impact` — freeze-frame emphasis on contact moment
6. `crowd-reaction` — cutaway to stylized crowd response
7. `aftermath-freeze` — static panel after key event

Stakes (cup final vs friendly) and memory-state (is this player relevant to a prior ledger event?) modulate shot **intensity / colour / paneling / text overlay / timing** — not the vocabulary itself.

**Rationale:** Lowest-risk path to a visual identity that reads as "anime football" without needing 3D. Competes on a different axis than FM (who'll never leave 2D-dots baseline) and other manager games (who'll never leave drab realism).

**Non-goals at MVP:**
- 3D player models / stadiums / crowds
- Motion-capture or realistic player animation
- Free-cam / replay scrub UI (beyond internal dev tools)
- Signature-specific unique cinematics (they map into the 7-shot vocabulary; no per-signature authoring)

**References:** Aoashi manga (diagonal compositions), Captain Tsubasa Rise of New Champions (impact moments), Inazuma Eleven 2D portions (signature expressiveness), VA-11 Hall-A (typography discipline), Giant Killing manga (tactical-diagram aesthetic).

---

## 3. MatchSim architecture

### 3.1 Structural split

```
football/
├── MatchSim.csproj                  # pure C# class library, zero UnityEngine
│   ├── Sim/
│   │   ├── Ball.cs                  # custom deterministic physics
│   │   ├── Player.cs                # 22 agents with state machines
│   │   ├── Pitch.cs                 # fixed-point coordinate space
│   │   ├── Tick.cs                  # deterministic timestep loop
│   │   ├── Fixed.cs                 # fixed-point arithmetic type
│   │   └── Seed.cs                  # per-match + per-event replay seeds
│   ├── Tactics/
│   │   ├── BehaviorTree.cs          # runtime BT
│   │   ├── ArchetypeLoader.cs       # YAML → BT
│   │   └── Signatures/              # 24 signature implementations
│   └── Events/
│       ├── MatchEvent.cs            # structured emission for semantic cinema + memory
│       └── Salience.cs              # salience scoring
│
├── MatchSim.Tests.csproj            # xUnit tests, zero Unity
│
├── UnityProject/                    # Unity 6 + URP project
│   └── Assets/_Project/Scripts/
│       ├── Viewer/                  # 2D semantic-cinema renderer
│       ├── Management/              # club / season / transfer / squad UI
│       ├── Memory/                  # event ledger + readers
│       ├── AI/                      # behavior-tree Unity-side wiring
│       └── Pipeline/                # content pack loader, save migrations
```

### 3.2 Determinism discipline

- **Fixed-point canonical state.** All sim-relevant quantities (positions, velocities, forces, RNG streams) use a `Fixed` struct (Q16.16 or Q24.8 — pick at Phase 3). Floats are FORBIDDEN inside MatchSim except for non-canonical viewer interpolation.
- **Fixed timestep.** 60Hz logical tick. Viewer interpolates at framerate; never drives sim.
- **Replay seeds.** Every match carries a `match_seed: u64`; every in-match stochastic event carries an event seed derived from `(match_seed, tick, event_id)`. Replay re-runs the sim with identical outputs.
- **No Unity physics.** Unity's PhysX is NOT in the canonical path. Viewer may use Unity for decorative only.
- **CI matrix.** GitHub Actions runs MatchSim.Tests on Windows + Mac + Linux every push. Fails on any determinism drift (comparing canonical state hash between platforms).

### 3.3 Ball physics

Custom deterministic sim: position + velocity + spin in fixed-point, explicit Magnus force + air drag per tick. Playable, not realistic-first. Physically-grounded but allows "Tsubasa-curl" — the sim bias lets signature actions generate trajectories that real physics would reject, in controlled and readable ways. Spec in `design/match-engine.md`.

---

## 4. Content pipeline architecture

### 4.1 Content packs

All content ships in versioned packs with stable IDs. Every player, club, league, badge, kit, stadium, signature, event template is addressed by:

```
pack_id: "finalwhistle.core.v1"
schema_version: 3
entity: "player:fwh.core.v1:player_00042"
```

**Stable IDs** persist across regeneration. Regenerating content pack v1 with new prompt engineering MUST NOT change existing IDs; deltas ship as `finalwhistle.core.v1.patch.2` content packs loaded alongside.

**Schema versions** gate save migrations. Loader knows to run `migrate_v2_to_v3` before instantiating game state.

### 4.2 AI Content Compiler

Pipeline (see `design/player-generation.md` + `design/worldbuilding.md`):

```
spec + prompt + seed
    ↓
structured JSON draft (deterministic via prompt caching + stable seeds)
    ↓
validation (schema check — required fields, type correctness)
    ↓
lint (duplicate-name detection, legal-name lint, style-consistency, cultural-plausibility)
    ↓
sim sanity check (does this player crash MatchSim? does this club's finances load?)
    ↓
content pack version bump + commit
    ↓
import into Unity SO / ScriptableObject wrappers
```

**No runtime prose from LLM.** All press conferences, match reports, fan-sentiment text are rendered from bake-time templates with runtime slot-filling from event-ledger state.

### 4.3 Player Identity Compiler

Each generated player emits a stable identity packet (see `design/player-generation.md`):

- **Playing instincts** (positioning defaults)
- **Pressure response** curve
- **Development hooks** (latent-potential trigger conditions)
- **Signature candidates** (which of 24 signatures this player can unlock, with affinity weights)
- **Scout labels** (phenotype labels the scouting system surfaces: "Late Bloomer", "Composed Under Pressure", "Explosive First Step", "Set-Piece Natural")
- **Commentary handles** (preferred noun/verb tokens for match commentary templates)
- **Rivalry compatibility** (which other identity packets map into high-salience rivalries)

Generated players are data-driven but *feel authored* because the identity packet encodes specific playing-style coherence, not random stat rolls.

---

## 5. Event-sourced career memory

Architectural pattern for Long Memory (see `design/event-sourced-memory.md`).

Every meaningful event emits a structured record to the career ledger:

```
event_id, match_id, who, what, stakes, emotion, consequence, callback_eligibility, salience
```

Ledger is **append-only** at runtime. Memory readers query the ledger for surfacing decisions:

1. **Alumni DB reader** — "is this opponent a former player of ours?"
2. **Rival recall reader** — "has this scoreline context happened before with these parties?"
3. **Promise tracking reader** — "at contract talks, surface any past promises"
4. **Big-match scars reader** — "this is a cup final → recall the last cup final scar"
5. **Press/fan callback reader** — "journalist queries require state-referenced context"

**Compaction strategy** (50K players × 10yr × ~100 events/year ≈ 50M events): hot event log for recent seasons (full fidelity), compacted summary state for older careers (preserves callback eligibility but drops tick-level granularity). Compaction boundary configurable; default at 5 seasons.

---

## 6. Single-source-of-truth rules

1. `design/` → authoritative for intent
2. `ScriptableObjects/` + content packs → authoritative for runtime data
3. C# → behaviour only, never content
4. MatchSim → canonical simulation state
5. Viewer → presentation only, can never author canonical state
6. `CLAUDE.md` → conventions and contracts
7. Conflicts resolve: design > content > code

---

## 7. Anti-feature-creep / anti-mess architecture

### Assembly Definitions skeleton

One `.asmdef` per system folder under `Scripts/`. Dependency graph enforced. Editor-only code under `Editor/` with `includePlatforms: ["Editor"]`. Proposed layout:

- `MatchSim.Core` (references MatchSim.csproj)
- `Viewer.Core`, `Viewer.SemanticCinema`, `Viewer.UI`
- `Management.Core`, `Management.UI`, `Management.Screens`
- `Memory.Core`, `Memory.Readers`
- `AI.BehaviorTree`, `AI.Archetypes`
- `Pipeline.ContentPacks`, `Pipeline.SaveMigration`
- `Debug` (editor-only)

### No `Resources.Load` in runtime code

All asset loading through Addressables + content pack loader. Enforced by `/audit`.

### Addressables group ontology

Groups by content type: `Content/Clubs`, `Content/Players`, `Content/Signatures`, `UI/Screens`, `Viewer/SemanticCinema`, `Audio/Crowd`, `Audio/Music`, `Fonts`.

---

## 8. Pipeline skills (Claude-authored tooling)

Authored under `.claude/skills/` as the project grows:

- `match-replay` — given a match seed, re-run MatchSim headless + export 2D viewer capture (QA + trailer authoring)
- `balance-harness` — 10K-season sweep + distribution emit (Claude-assisted tuning)
- `content-compile` — run the AI Content Compiler end-to-end with validation lints
- `player-identity-compile` — generate identity packets from prompts + seeds
- `signature-authoring` — author/adjust one of the 24 signatures via YAML spec + preview
- `ledger-query` — read career memory ledger, filter by reader type, debug callback chains

Author each when the corresponding Phase triggers.

---

## 9. Phase progression (mirror of SPEC.md high-level)

```
PHASE 0 — KICKOFF (ACTIVE)
├─ pitch doc filled (done at bootstrap)
├─ 4-bucket scope split locked (done at bootstrap)
├─ 19 decisions logged (done at bootstrap)
├─ design docs scaffolded with open questions (done at bootstrap)
└─ gate: all design-doc open questions resolved; Month-3 slice spec reviewed

PHASE 1 — SETUP
├─ Unity 6 LTS pinned
├─ gh remote created + first push
├─ CI stub (GitHub Actions: build + MatchSim.Tests on 3 platforms)
└─ gate: `git log -3` shows setup commits; CI green on empty stub

PHASE 2 — DESIGN BIBLE
├─ all design/ open questions resolved + ADRs where systems lock
├─ SIGNATURE catalog: all 24 specified
├─ WORLDBUILDING: fictional nation + pyramid structure finalized
├─ EVENT-SOURCED-MEMORY ledger schema locked
└─ gate: engineering can begin without guessing

PHASE 3 — UNITY BOOTSTRAP + MATCHSIM PROTOTYPE
├─ MatchSim.csproj + xUnit tests skeleton
├─ Fixed-point arithmetic primitives
├─ 2 rival behavior-tree archetypes authored in YAML
├─ 22 players on a pitch, ball physics custom-deterministic
├─ unity-project/ created + URP configured
├─ 2D viewer with 3 shot types prototype (tactical-wide, diagonal-attack-lane, pass-shot-impact)
├─ devlog clips published Month 2-3
└─ MONTH-3 GATE: stranger watches match + understands drama

PHASE 4 — SCOUT DISAGREEMENT + FIRST SIGNATURES
├─ Scout Disagreement feel prototype (conditional MVP)
├─ MONTH-4 GATE: disagreement creates decisions or it's cut
├─ 3-6 signatures authored end-to-end (animation + trigger + sim bias + presentation recipe)
├─ closed itch build for trusted testers
└─ gate: Scout system decision + retention data from itch testers

PHASE 5 — VERTICAL SLICE
├─ full season playable
├─ all 7 shot types implemented
├─ event-sourced memory ledger operational with 3 readers
├─ Month-6 public demo
└─ gate: first full season plays end-to-end

PHASE 6 — CORE SYSTEMS
├─ signatures: all 24 authored
├─ content pack v1 compiled (~96 clubs, ~2000 players)
├─ manager archetypes: 20-30 authored
├─ save/load with schema migrations tested
├─ Month-8 Steam Next Fest (if first 10 mins are sharp)
└─ gate: systems architecturally complete

PHASE 7 — CONTENT SCALING + POLISH
├─ balance harness production passes
├─ narrative event templates (5-8/season)
├─ UI polish anti-FM26-regression pass
├─ localization hooks exercised
├─ accessibility pass
└─ gate: content-complete + polished for EA

PHASE 8 — EA LAUNCH (Month 12)
├─ Steamworks integration complete
├─ Steam page + screenshots + trailer
├─ EA launch checklist green
└─ gate: Steam release button pressed

PHASE 9 — POST-EA
├─ hotfix cadence
├─ community feedback triage
├─ 3D match engine R&D only now begins (conditional on audience signal)
├─ multi-nation expansion (post-1.0)
└─ ongoing
```

See `SPEC.md` for task-level detail.

---

## 10. Open engineering questions

- Fixed-point format: Q16.16 vs Q24.8 — lock at Phase 3 Week 1
- Behavior-tree runtime: authored from scratch vs lightweight open-source (BehaviorDesigner / NPBehave)
- UI Toolkit vs uGUI fallback: UI Toolkit first; fall back to uGUI only for any screen where UIT documented bugs block progress
- Content pack loader: JSON + schema lint first; evaluate msgpack for production size at Phase 6
- Save format: JSON at MVP for debuggability; consider versioned msgpack at Phase 6

---

*Authored 2026-04-22. Revise at each phase transition + whenever a major tech decision is logged in SPEC.md decisions log.*

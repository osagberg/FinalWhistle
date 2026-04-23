# Bootstrap reference profiles

> 5 reference profiles Claude uses during `/bootstrap` Phase B to inform customization decisions. NOT rigid templates — Claude composes custom when no profile fits well.

## How profiles work

After intake, Claude identifies the closest-match profile (or "custom" if distance from all profiles is large). The profile informs:
- Which MCPs + plugins to install
- Which Unity packages to queue for Phase 3
- Which template sections to keep / delete / customize
- Which SPEC.md tasks to keep / trim
- Which trigger-table entries to include in SETUP.md

**Claude announces the profile match during Phase B plan-announcement:**
> "Your intake matches the NARRATIVE-HEAVY profile (~80% confidence) with these modifications: <list>. Proceeding?"

---

## Profile 1 — narrative-heavy

**Fits:** Disco Elysium, Signalis, Pathologic, VA-11 HALL-A, Tyranny, Citizen Sleeper, Thousand Year Old Vampire-likes

### Intake signatures
- `genre_tags`: narrative / RPG / adventure / VN / mystery
- `narrative_weight`: `heavy`
- `character_count_estimate`: 6-12+
- Typical references: narrative-first games

### Recommended default scope
- **`rich`** — narrative-heavy needs creative-director + narrative-director in heavy rotation; 14-agent roster pays off immediately. If `context_window = 1M` AND currently at Phase 0 or 2, promote to **`research`** for breadth during design-bible authoring.

### Tech stack
Ships pre-installed: 14 core agents, 31 slash commands, 17 rules, unity-check / state-dump / unity-webgl-builder / github-pages-deploy / unity-audio-generator skills, 25+ patterns.

Project-specific adds:
- Unity 6 LTS + URP
- **Ink** (narrative engine — mandatory)
- Character pipeline: depends on 2D / 3D choice; 3D anime + VRoid is common
- Animancer Pro (Phase 4 trigger)
- FinalIK (Phase 4 trigger)
- Magica Cloth 2 (Phase 4-5 trigger)
- **Voice bible subagent pattern** (see `patterns/narrative-subagent-pattern.md`) — author per-character subagents in Phase 6

### MCPs
- Baseline (desktop-commander, context7, chrome, github)
- Unity MCP at Phase 3

### Plugins to queue
- feature-dev, pr-review-toolkit, hookify

### Template customizations
- **Agents in heavy rotation:** `creative-director`, `narrative-director`, `game-designer`, `art-director` — consult these often for tone / cast / scene work
- **Agents in light rotation:** `systems-designer`, `engine-programmer` — less narrative-relevant
- Keep all narrative-related sections in CLAUDE.md + TECH_APPROACH.md
- Keep Dialog asmdef in scripts-folder-structure
- Reference `templates/design-templates/game-design-document.md` + `game-pillars.md` at Phase 2
- Reference `templates/design-templates/ux-spec.md` + `hud-design.md` at Phase 5-6 for dialog UI
- SPEC.md Phase 2: emphasize cast.md, relationships.md, schedule.md (if calendar-sim mixed in)
- SPEC.md Phase 6: keep Dialog system task; point at `patterns/game-ai-runtime/dialogue-runtime.md` (studio-tier)

### design/ scaffolding
- setting.md, cast.md, relationships.md, stats_spec.md (if RPG), schedule.md (if calendar), content_policy.md, mechanics/
- Reference (don't copy): `templates/design-templates/game-concept.md`, `game-design-document.md`, `game-pillars.md`

### Scope note
Narrative-heavy games typically hit Phase 7 hardest (content grind on dialogue). Budget Phase 7 as 2-3x your initial estimate.

---

## Profile 2 — action-character

**Fits:** Hades, Hollow Knight, Dead Cells, Celeste, Ori and the Blind Forest, Katana ZERO

### Intake signatures
- `genre_tags`: action / platformer / roguelike / metroidvania / soulslike
- `narrative_weight`: `light` or `medium` (mostly mechanics-driven)
- `character_fidelity`: `2d` sprites OR `3d_stylized`

### Recommended default scope
- **`rich`** — gameplay-programmer + unity-specialist + game-designer are all in heavy rotation; action games benefit from the full 14-agent roster for feel-tuning. On Phase 5 (vertical slice) a 1M-tier user may temporarily `/deep-research` for feel-reference pass.

### Tech stack
Ships pre-installed: 14 core agents, 31 slash commands, 17 rules, unity-check / state-dump / unity-webgl-builder skills, 25+ patterns.

Project-specific adds:
- Unity 6 LTS + URP
- Animation Rigging + Animancer Pro (mandatory — animation-heavy)
- FinalIK (Phase 4 trigger if 3D)
- DOTween (free, for UI / effect tweens)
- Cinemachine (built-in, camera work)
- No cloth sim typically
- No Ink unless narrative-medium

### MCPs
- Baseline + unity-mcp

### Plugins to queue
- feature-dev, pr-review-toolkit, hookify

### Template customizations
- **Agents in heavy rotation:** `gameplay-programmer`, `game-designer`, `unity-specialist`, `engine-programmer` (performance critical), `qa-lead` (feel regression)
- **Agents in light rotation:** `narrative-director`, `creative-director` (narrative is thin)
- Remove narrative-heavy sections from TECH_APPROACH
- Reference `templates/design-templates/game-pillars.md` at Phase 0 (feel pillars lock)
- Reference `templates/design-templates/test-plan.md` at Phase 5+ (combat regression + feel gates)
- Reference `patterns/fsm.md`, `patterns/event-driven.md`, `patterns/object-pooling.md` at Phase 6 (combat/enemy systems)
- SPEC.md Phase 2: stats_spec, mechanics/, NO cast.md unless characters have voice
- SPEC.md Phase 6: emphasize combat + input systems; Dialog system only if narrative-medium
- Assembly Definitions: rename `CoreMechanic/` to match genre (e.g., `Platforming/`, `Roguelike/`, `Brawling/`) and add `Combat/` if combat is separate from the signature mechanic
- Keep Addressables groups but drop `Ink` group; add `Levels` + `Enemies` groups

### design/ scaffolding
- overview.md, combat.md, enemy-design.md, level-design.md, stats_spec.md, progression.md, accessibility.md
- Reference (don't copy): `templates/design-templates/game-concept.md`, `systems-index.md`, `test-plan.md`

### Scope note
Action games' Phase 5 (vertical slice) is MORE painful than narrative — you must prove the feel before scaling. Budget 8-16 weeks for Phase 5.

---

## Profile 3 — puzzle-minimal

**Fits:** Baba Is You, Patrick's Parabox, A Monster's Expedition, Stephen's Sausage Roll, The Witness

### Intake signatures
- `genre_tags`: puzzle / logic / sokoban / metapuzzle
- `narrative_weight`: `none` or `light`
- `character_fidelity`: `2d`, `none`, or `3d_lowpoly`
- `scope`: typically `tiny` or `small`

### Recommended default scope
- **`standard`** — puzzle games are intentionally lean; 14-agent roster is overkill. Core docs + 6 base slash commands is enough. User can `/expand-studio` later if scope grows (rarely needed).

### Tech stack
Ships pre-installed: 14 agents (on-demand only at `standard` scope), 31 slash commands, 17 rules, core skills, patterns library.

Project-specific adds:
- Unity 6 LTS + URP (or 2D Renderer if `2d`)
- Minimal package set
- No Ink, no cloth, no VRoid, no FinalIK
- DOTween + Odin Inspector (SO authoring for level data)

### MCPs
- Baseline (desktop-commander, context7, chrome)
- Skip github if truly private / single-developer
- Skip unity-mcp if project is small enough not to need MCP automation

### Plugins to queue
- feature-dev, hookify (skip pr-review-toolkit if solo + no PR workflow)

### Template customizations
- **Agents in heavy rotation (via Task tool on-demand):** `systems-designer`, `gameplay-programmer`, `game-designer` (level design is mechanic design)
- **Agents in light rotation:** `narrative-director`, `creative-director`, `art-director` (minimal visual scope)
- Drastically trim CLAUDE.md + TECH_APPROACH.md
- Remove character-pipeline section entirely
- Remove narrative sections
- Reference `templates/design-templates/game-concept.md`, `game-pillars.md` at Phase 0
- Reference `patterns/fsm.md`, `patterns/unit-testing.md` at Phase 3+
- SPEC.md: merge Phase 4 + Phase 5 if no characters
- Assembly Definitions: consolidate — maybe just `Core`, `Puzzle`, `UI`, `Debug`, `Editor`

### design/ scaffolding
- overview.md, mechanics/<each-puzzle-mechanic>.md, level-catalog.yaml (SO-backed)
- Reference (don't copy): `templates/design-templates/game-concept.md`, `systems-index.md`

### Scope note
Puzzle games succeed on content density. Phase 7 is ~100 well-designed levels, not 500 mediocre ones. Quality > quantity.

---

## Profile 4 — sim-management

**Fits:** Rimworld, Stardew Valley, Frostpunk, Oxygen Not Included, Cult of the Lamb, Slipways

### Intake signatures
- `genre_tags`: sim / management / colony / resource / economy / strategy
- `narrative_weight`: `light` (mostly ambient)
- `character_fidelity`: `2d` sprites or `3d_lowpoly` (small characters, many on screen)

### Recommended default scope
- **`studio`** — sim games have maximal systemic breadth (economy + progression + factions + balance + simulation-tick); studio's extended roster (ai-programmer, level-designer, writer for flavor text) is justified. Proactive-load of `phase-gate-workflow.md` + `budget-tier-trigger-table.md` helps the long Phase 6-7 grind.

### Tech stack
Ships pre-installed: 14 core agents (+ extended at studio scope), 31 slash commands, 17 rules, core skills, full patterns library.

Project-specific adds:
- Unity 6 LTS + URP
- **Odin Inspector** (Phase 2 trigger — SO-heavy data authoring)
- DOTween (UI tweens)
- No FinalIK (characters don't need deep IK)
- No cloth sim
- ECS / DOTS (consider if performance-critical; otherwise MonoBehaviour is fine for <500 entities)

### MCPs
- Baseline + github + unity-mcp

### Plugins to queue
- feature-dev, pr-review-toolkit, hookify

### Template customizations
- **Agents in heavy rotation:** `systems-designer`, `engine-programmer` (performance at scale), `technical-director` (ECS/DOTS architecture decisions), `producer` (Phase 6 systemic-depth grind)
- **Agents in heavy rotation at studio scope:** `ai-programmer` (sim AI), `level-designer` (tile/map systems), `technical-artist` (many-entities rendering)
- **Agents in light rotation:** `narrative-director`, `creative-director`
- Remove narrative-heavy sections
- Reference `templates/design-templates/systems-index.md` at Phase 2 (sim games have the largest systems-index by far)
- Reference `templates/design-templates/architecture-decision-record.md` at Phase 3+ for ECS/DOTS decision
- Reference `patterns/ecs-intro.md`, `patterns/spatial-partitioning.md`, `patterns/fsm.md`, `patterns/object-pooling.md`, `patterns/phase-gate-workflow.md`
- Reference `patterns/save-system/` subtree at Phase 6-7 (sim games need version-migration early)
- SPEC.md Phase 2: emphasize economy design, progression, balance mechanics
- SPEC.md Phase 6: add "simulation tick system" task
- Assembly Definitions: add `Simulation`, `Economy`, `Crafting`, `UI`; optionally remove `Combat` if peaceful sim
- Addressables groups: `Tiles`, `Entities`, `Buildings`, `UI`, `Audio`

### design/ scaffolding
- overview.md, economy.md, progression.md, faction-design.md (if factions), stats_spec.md, accessibility.md
- Reference (don't copy): `templates/design-templates/systems-index.md`, `game-design-document.md`, `architecture-decision-record.md`

### Scope note
Sim games win on systemic depth. Budget Phase 6 (core systems) as 4-6 months MINIMUM. Phase 7 content scaling is lighter than narrative games — systemic content emerges from depth, not hand-authoring.

---

## Profile 5 — vn-heavy (visual novel / dating sim)

**Fits:** Doki Doki Literature Club, VA-11 HALL-A, Hatoful Boyfriend, Danganronpa, Coffee Talk

### Intake signatures
- `genre_tags`: VN / visual-novel / dating / slice-of-life / romance
- `narrative_weight`: `heavy`
- `character_fidelity`: `2d` (sprite-based portraits + CG backgrounds) — OR `3d_anime` if making a 3D VN

### Recommended default scope
- **`rich`** — creative-director + narrative-director + art-director are all essential; dialogue-heavy projects benefit enormously from the 14-agent roster. At Phase 0 kickoff or Phase 2 design-bible authoring on 1M-tier, promote to `research` for the breadth of reference VN structure patterns.

### Tech stack
Ships pre-installed: 14 core agents, 31 slash commands, 17 rules, core skills, patterns library.

Project-specific adds:
- Unity 6 LTS + URP (or stick with 2D if no 3D elements)
- **Ink** (narrative engine — mandatory)
- Dialogue-UI asset (Fungus is free; or author custom — typically the latter for serious VNs)
- Animation: minimal; mostly portrait-state swapping
- CG pipeline (static images) — emphasis on external authoring (Clip Studio, Photoshop) not Unity

### MCPs
- Baseline + github
- Unity MCP at Phase 3 but lower emphasis (VN is less unity-native than most genres)

### Plugins to queue
- feature-dev, hookify
- anthropic-skills (for canvas-design skill — CG authoring)

### Template customizations
- **Agents in heavy rotation:** `narrative-director`, `creative-director`, `art-director` (CG art direction), `ui-programmer` / `unity-ui-specialist` (dialog UI is the game)
- **Agents in light rotation:** `gameplay-programmer`, `engine-programmer`, `systems-designer`
- Emphasize Dialog system in TECH_APPROACH
- Reference `templates/design-templates/game-pillars.md`, `game-design-document.md` at Phase 0-2
- Reference `templates/design-templates/ux-spec.md`, `hud-design.md` at Phase 5-6 (dialog UI is load-bearing)
- Reference `patterns/narrative-subagent-pattern.md` at Phase 6 (per-character voice bibles)
- Reference `patterns/game-ai-runtime/dialogue-runtime.md`, `safety-filters.md` (studio-tier) if adult-rated
- Reference `asset-pipelines/cg-render.md` + `2d-asset-gen.md` for CG pipeline
- Remove physics / cloth / IK sections
- SPEC.md Phase 4: focus on portrait authoring + UI rather than 3D character
- SPEC.md Phase 5 vertical slice: one full scene with routing + portrait swaps + CG reveal
- Voice-bible subagent pattern strongly recommended (Phase 6)

### design/ scaffolding
- overview.md, cast.md, relationships.md, content_policy.md, route-structure.md, CG-catalog.yaml
- Reference (don't copy): `templates/design-templates/game-design-document.md`, `ux-spec.md`, `hud-design.md`

### Scope note
VN scope scales with word count. 50K words = ~8 hours reading; 150K = ~25 hours. Budget writing time as **primary bottleneck**, not Unity work.

---

## Custom profile (no match)

When intake doesn't cluster around any single profile:

**Claude's approach:**
1. Don't force-fit — declare "custom" explicitly
2. Compose customizations per-intake-answer (see CUSTOMIZATION.md)
3. Default to narrative-heavy's template customizations if narrative_weight>=medium; default to action-character if mechanics-focused; default to minimal if tiny/puzzle
4. Flag the cherry-picked nature in the bootstrap summary: "Composed custom profile — reasoning: <brief>"

---

## Matching algorithm (informal)

Score each profile against intake:

```
narrative-heavy:
  +30 if narrative_weight == "heavy"
  +15 if narrative_weight == "medium"
  +20 if character_count_estimate >= 6
  +10 if "narrative" in genre_tags

action-character:
  +25 if "action" or "platformer" or "roguelike" in genre_tags
  +20 if character_fidelity in (2d, 3d_stylized)
  +15 if narrative_weight in (none, light)

puzzle-minimal:
  +30 if "puzzle" in genre_tags
  +20 if scope == "tiny"
  +15 if narrative_weight == "none"

sim-management:
  +25 if "sim" or "management" in genre_tags
  +20 if scope in (medium, large)
  +10 if Phase 6 expectation emphasizes systems

vn-heavy:
  +30 if "VN" or "visual-novel" or "dating-sim" in genre_tags
  +25 if narrative_weight == "heavy" AND character_fidelity == "2d"
```

Pick the highest score. If top-two are within 10 points: "composed custom drawing from X + Y."

---

## Adding new profiles

When you ship a project outside these profiles, consider adding a new profile file. Keep each profile to ~80-150 lines; longer profiles become bureaucratic and nobody reads them.

Good profile additions:
- Horror (Resident Evil-likes) — meaningfully different from narrative-heavy
- Racing — completely different tech stack
- MMO / multi-player — requires netcode addition not covered here
- Rhythm game — highly specialized

Bad profile additions:
- "RPG but with swords instead of guns" — just use narrative-heavy
- "My specific game" — profiles are shape-of-project, not specific games

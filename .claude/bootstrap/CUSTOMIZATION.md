# Bootstrap customization rules

> Read by Claude during `/bootstrap` Phase B + C. Maps intake answers → template file edits. Claude applies these rules to customize every template file.

## General principles

1. **Replace, don't just fill.** `{{PROJECT_NAME}}` → actual name. `<fill-in>` → real content. Don't leave placeholders.
2. **Delete irrelevant sections.** Non-narrative games shouldn't have dialogue-runner sections in TECH_APPROACH. Keep what matters.
3. **Add project-specific sections.** A roguelike needs a "run structure" design doc section that a linear narrative game doesn't.
4. **Be opinionated.** Fill with confident choices derived from intake answers. User can refine in Phase 0. Worse outcome: leave vague placeholders.
5. **Preserve structural patterns.** Don't rewrite the phase-gate workflow, decisions-log discipline, or hook wiring. Those are universal.

---

## File-by-file customization

### `CLAUDE.md`

**Section 1 — What this project is:**
- Replace all placeholders with intake values
- `<one-paragraph project pitch>` → 3-5 sentence pitch derived from reference games + genre + scope
- `<fill-in setting + mood summary>` → compose from pitch + reference games (2-3 sentences)
- Core loop — compose from intake genre + reference games
- Unique selling points — propose 3 based on pitch; flag as "Phase 0 refinement candidates"

**Section 3 — Tech stack:**
- Unity + URP rows — keep
- Cloth / IK / Animation / Narrative / Character-pipeline — set per intake:
  - `character_fidelity = none` or `2d` → delete all 3D-character rows (cloth, IK, VRoid, lilToon)
  - `character_fidelity = 3d_lowpoly` → keep Animation Rigging + Animancer; delete cloth unless `scope >= medium`
  - `character_fidelity = 3d_anime` → keep everything; VRoid pipeline; lilToon as toon shader
  - `character_fidelity = 3d_pbr` → keep FinalIK + Animation Rigging; delete lilToon (PBR uses URP Lit); delete VRoid (bespoke pipeline)
  - `narrative_weight = none` → delete Ink row + Dialog asmdef mentions
  - `narrative_weight >= medium` → keep Ink
- Budget row — set per `budget_tier`

**Section 5.6 — Git workflow:**
- Fill remote URL from studio + project name
- If `commercial_intent = private`, note private-by-default

**Pitfalls section:**
- Add `content_rating`-specific pitfall if `adult` rated — include explicit age-gate rule

### `PROJECT_CONTEXT.md`

**Section 1 — Pitch:** fill all 4 fields from intake (elevator, longer pitch, problem, user-targeted-pain)
**Section 2 — Comparables:** prefill with `reference_games` from intake; flag for user to complete table at Phase 0
**Section 3 — Target audience:** primary audience from genre + rating; non-targets from scope
**Section 4 — Commercial:** platforms, intent, launch target (set to 12-24 months out depending on scope)
**Section 5 — Setting & mood:** compose from intake + reference games
**Section 6 — Core loop:** propose from genre + references; flag for Phase 0 refinement
**Section 7 — Scope estimate:** derive from `scope` field:
  - `tiny` — 1-2 chars, 1-2 scenes, 2-4 hrs, ~5K words
  - `small` — 2-5 chars, 3-8 scenes, 4-8 hrs, ~15K words
  - `medium` — 5-12 chars, 8-20 scenes, 10-20 hrs, ~40K words
  - `large` — 12+ chars, 20+ scenes, 20+ hrs, ~80K+ words
**Section 8 — Non-goals:** propose 3 based on pitch (what you're explicitly NOT doing)
**Section 9 — Risk register:** pre-fill with scope-appropriate risks

### `TECH_APPROACH.md`

**Section 1 — Stack:** per `character_fidelity` + `narrative_weight`:
  - Fill in/out for every optional row
  - Remove "character pipeline" section entirely if `character_fidelity in (none, 2d)`
  - Remove "narrative" mentions if `narrative_weight = none`
**Section 2 — Visual target:** compose from intake references
**Section 3 — Character pipeline:** delete section entirely if `character_fidelity in (none, 2d)`; otherwise fill per `character_pipeline`
**Section 4 — Environment pipeline:** short note per genre (modular vs bespoke)
**Section 5 — Folder structure:** delete/rename subfolders per character + narrative choices:
  - No narrative: remove `Dialog/` + `Ink/` + `<Narrative>/` folder refs
  - No characters: remove `Characters/` + character-related Art subfolders
**Section 8 — Phase progression:** trim phase tasks per scope:
  - `tiny`: merge Phase 4+5 into single "First Playable"
  - `small`: keep standard phases
  - `medium`: keep standard phases
  - `large`: emphasize Phase 7 grind discipline

### `SPEC.md`

**Phase 0 tasks:** keep full list (applies to every project)
**Phase 1 tasks:**
- Account prereqs: add Steam Direct if `commercial_intent in (steam, multi)`
- Localization tooling mention: keep (Phase 7 task anyway, but note the early-planning benefit)
**Phase 2 tasks:** trim based on `narrative_weight`:
- `none`: remove setting.md, cast.md, mechanics narrative-specific
- `light`: keep overview + stats + mechanics; skip cast
- `medium`: keep all except relationships
- `heavy`: keep ALL including relationships, schedule
**Phase 3 — Unity Bootstrap tasks:** update package list per intake tech choices
**Phase 4 tasks:** delete entirely if `character_fidelity = none`; trim if `2d`
**Phase 5 tasks:** always keep (every game needs a vertical slice)
**Phase 6 tasks:**
- Combat system: delete if genre has no combat
- Dialog system: delete if `narrative_weight = none`
- Schedule/grid: delete if not a sim-calendar game
**Phase 7 tasks:** universal — keep
**Phase 8 tasks:**
- If `commercial_intent = itch` only: remove Steamworks tasks
- If `commercial_intent = private`: remove all marketing + Steam tasks
**Phase 9 tasks:** keep if shipping commercially

**Backlog:** add 3-5 ideas derived from genre (common design questions for that shape of game)

**Decisions log:** seed with initial bootstrap decision:
```
- **YYYY-MM-DD** — **Project bootstrapped from blueprint template**. Intake
  summary: {project_name} / {genre_tags} / {scope} / {commercial_intent} /
  {content_rating}. See `.blueprint-version` for full intake record.
```

### `STATUS.md`

- `Last updated` → today's date (hook will maintain)
- `Currently working on` → "Phase 0 — Kickoff. Running /next picks up pitch refinement."
- `Next action` → "Run `/next` to pick up first Phase 0 task."

### `CHANGELOG.md`

- Replace the placeholder first entry with a real bootstrap entry:
```
## YYYY-MM-DD (Project bootstrap ✅)

- Project forked from blueprint template at ~/dev/blueprint/
- Intake complete: <summary line>
- Customized CLAUDE.md, PROJECT_CONTEXT.md, TECH_APPROACH.md, SPEC.md per intake
- MCPs installed: <list>
- Plugin install queue: see `.claude/bootstrap/scripts/install-plugins.txt`
- Git initialized
- Ready for Phase 0 work via `/next`
```

### `SETUP.md`

**Section 1 — Accounts:** check/uncheck rows per intake:
- Steam Direct → marked required if `commercial_intent in (steam, multi)`
- Apple Dev → remove unless iOS/Mac App Store planned (rare)
- Adobe/Mixamo → keep if `character_fidelity in (3d_lowpoly, 3d_anime, 3d_pbr, mix)`
**Section 3 — Budget tiers:** annotate current tier per `budget_tier`
**Section 4 — Install list:** check rows per tech choices:
- VRoid Studio → install only if `character_pipeline = vroid`
- Blender → install if any 3D character fidelity
**Section 10 — Trigger table:** remove rows irrelevant to project (e.g., Magica Cloth 2 for a 2D game)

### `TOOLING.md`

**Section 1 — MCPs:** annotate which get installed at bootstrap (from INSTALLABLES.md selection)
**Section 2 — Plugins:** annotate which get installed at bootstrap
**Section 3 — Subagents:** remove the per-character-bible pointer if `narrative_weight in (none, light)`

### `.gitignore`

- Add `*.<genre-specific>` entries if applicable (rarely needed)
- Otherwise keep as-is

### `design/README.md`

Delete all rows in the "what goes here" table that don't apply:
- No cast.md row if `narrative_weight in (none, light)`
- No schedule.md unless calendar-sim
- etc.

Then optionally replace with project-specific scaffolding (empty per-mechanic files, etc.) — but don't create the design docs themselves. Phase 2 is where the user authors those.

---

## Context scope initialization

Every project ships with a 5-scope architecture (see `.claude/context-scopes.json`). Claude writes `.claude/.current-scope` with the chosen scope at bootstrap; session-start.sh reads it on every subsequent session to decide proactive-load manifest.

**Scope resolution rules:**

1. If intake answered `default_scope` explicitly → write that value
2. If intake left it `unknown`:
   - `context_window = 1M` → `rich`
   - `context_window = 200K` → `standard`
3. **Phase-sensitive override** — if the project's currently-active phase is 0, 2, or 8 AND `context_window = 1M`, recommend `research` during Phase B plan-announcement; override only if user declines
4. If `new-project.sh` already wrote `.claude/.current-scope = rich` (the script's default), overwrite only if intake demands a different value

**Write the scope:**
```bash
echo "<scope>" > .claude/.current-scope
```

**Announce in Phase D handoff:** "Starting scope: `<scope>`. Re-scope anytime via `/expand-studio`, `/deep-research`, or `/contract-scope <target>`."

---

## Agent roster initialization

The 14 core agents under `.claude/agents/` (creative-director, technical-director, producer, game-designer, lead-programmer, art-director, narrative-director, qa-lead, gameplay-programmer, engine-programmer, systems-designer, ui-programmer, unity-specialist, unity-ui-specialist) ship in every project's skeleton — they are Task-invocable regardless of scope.

**Behavior by scope:**
- `minimal` / `standard` — agents present on disk; invoked only when user explicitly summons via Task tool. Not announced in `/help`.
- `rich` / `studio` / `research` — agents proactively surfaced in `/help`; Claude may auto-delegate high-stakes decisions to the relevant director (e.g., architecture decision → technical-director).

**Per-profile agent emphasis** — see `PROFILES.md`. No roster editing needed; the profile's "Template customizations" section annotates which agents are in heavy rotation.

---

## Rules initialization

The 17 rule files under `.claude/rules/` ship pre-installed and lazy-load per-path via frontmatter matching. Claude does NOT prune these at bootstrap — they're low-cost (only loaded when a matching file is touched) and inert otherwise. Customization touches zero rule files.

---

## MCP install mapping

Pull from `INSTALLABLES.md` — which MCPs per project type:

| Project trait | MCPs to install |
|---|---|
| Always | desktop-commander, context7, chrome (user-scoped if not already there) |
| Git-remote-based | github (user-scoped) |
| Phase 3+ Unity | unity-mcp (project-scoped) |
| Steam commercial intent | — (no MCP; Steamworks via Unity package at Phase 8) |
| Character-heavy 3D | — (no MCP; VRoid is authoring-outside-of-Claude) |

For each MCP Claude installs, use the commands in INSTALLABLES.md. Check `claude mcp list` first to avoid duplicate installs at user scope.

## Plugin install queue

Pull from `INSTALLABLES.md` — which plugins per project type:

| Project trait | Plugins |
|---|---|
| Always | feature-dev, pr-review-toolkit, hookify |
| Narrative-heavy | — (subagent pattern sketched in patterns/, no plugin required) |
| Building Claude plugins later | plugin-dev |

Write these to `.claude/bootstrap/scripts/install-plugins.txt` as a list Claude shows at the end.

**Note:** with 31 slash commands shipping in the skeleton + 5 scopes, the plugin set itself is unchanged. Scope (not plugins) determines which commands are proactively visible in `/help`. No plugin changes are required based on scope choice.

## Unity package additions queued for Phase 3

In `SPEC.md` Phase 3, update the "Install Unity packages" task with the specific packages per project:

| Project trait | Unity packages |
|---|---|
| Always | UniTask, Addressables, Animation Rigging, Unity Recorder, Unity MCP (CoplayDev) |
| Narrative heavy | Ink-Unity-Integration |
| Character pipeline `vroid` | UniVRM, lilToon (git-URL UPM) |
| Cloth sim needed | (Asset Store) Magica Cloth 2 — Phase 3 Asset Store task |
| IK needed | (Asset Store) FinalIK — gated by Phase 4 pain-trigger |
| Localization at launch | Unity Localization |

Write the specific list into Phase 3's task description + the trigger table in SETUP.md.

---

## Edge cases

**Intake answer was `unknown` / vague:**
- Pick a sensible default based on other answers
- Flag in the file with `<TBD — unknown at bootstrap>` comment
- Note in SPEC.md decisions log that this was deferred

**Intake yielded a profile Claude has low confidence about:**
- Do best-guess customization
- Flag in the bootstrap summary: "I'm applying narrative-heavy profile with 60% confidence; revise CLAUDE.md / TECH_APPROACH.md in Phase 0 if it's wrong"

**Two intake answers conflict (e.g., adult rating + family-friendly genre):**
- Resolve toward the more conservative choice for content policy
- Flag the conflict in the summary before executing

---

## Final pass — sanity check

After all customization, Claude should grep across the FULL project tree (not just root docs — Wave B/C/D added content under many subdirs):
1. `{{PROJECT_NAME}}` / `{{STUDIO}}` / `{{GENRE}}` markers — include `templates/design-templates/*.md`, `asset-pipelines/*.md`, `patterns/**/*.md`, `unity/*.md`, `steam-release/*.md` in the grep scope
2. `<fill-in>` markers
3. `<placeholder>` text
4. `TODO:` / `FIXME:` that weren't intentional
5. Verify `.claude/.current-scope` exists and contains a valid scope name from `.claude/context-scopes.json`

If found, fix them before declaring bootstrap complete.

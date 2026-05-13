# openfootmanager — data model + Tauri layer

**Read on:** 2026-05-13
**Key files:** `src-tauri/crates/domain/src/player.rs` (655 LoC), `src-tauri/crates/engine/src/types.rs` (309 LoC), `src-tauri/src/lib.rs` (162 LoC, 49 Tauri commands), `src-tauri/crates/db/src/sql/v001_initial_schema.sql` (initial schema + 22 migrations through v023)

OFM splits its types across four crates: `domain` (canonical persisted shapes — `Player`, `Team`, `League`), `engine` (match-time `PlayerData` / `TeamData` projections), `db` (rusqlite + 23 schema migrations), `ofm_core` (game services, world generator, save IO). The frontend is React 19 + Zustand + Tailwind v4 + i18next — **no PixiJS, no canvas, no SVG pitch** (package.json:13–27).

## Player data model

Canonical `domain::player::Player` (player.rs:3–83) carries 80+ fields including identity, attributes, dynamic match values, traits, contract/transfer state, season stats, career history, and a `PlayerMoraleCore` substructure. IDs are `String` (UUID v4 serialized as strings — generator/generation.rs:163: `Uuid::new_v4().to_string()`). Position is an `enum Position` with **17 variants** (player.rs:85–105) including a four-element legacy group (`Goalkeeper / Defender / Midfielder / Forward`) plus 13 granular roles (`RightBack`, `AttackingMidfielder`, `LeftWinger`, …). `to_group_position()` (player.rs:115–135) collapses granular → legacy for the engine which only consumes the four-bucket version (engine/types.rs:8–13).

**Attribute count: 19 attributes, all `u8` 0–100** (player.rs:153–189). Split:
- **Physical (4):** pace, stamina, strength, agility
- **Technical (5):** passing, shooting, tackling, dribbling, defending
- **Mental (7):** positioning, vision, decisions, composure, aggression, teamwork, leadership
- **Goalkeeper (3):** handling, reflexes, aerial

No visible/hidden split — every attribute is rendered raw in the UI. No role-specific weights stored on the player; instead `engine/types.rs:142–180` defines `defense_rating()` / `midfield_rating()` / `attack_rating()` / `goalkeeper_rating()` as fixed weighted averages per bucket (e.g. defenders weight `defending + tackling + positioning + strength` evenly, then 0.7 def-line + 0.3 GK).

Dynamic per-match values live alongside attributes: `condition: u8` (0–100 short-term), `fitness: u8` (0–100 long-term shape, multiplies depletion — player.rs:38–40), `morale: u8` (player.rs:36), optional `injury: Option<Injury>` (player.rs:42, 203–207). Derived ratings `ovr: u8` and `potential: u8` (player.rs:53–57) are cached on the struct and recomputed by `ofm_core::player_rating::refresh_player_derived`. Traits (player.rs:408–437) are a `Vec<PlayerTrait>` enum of 21 variants derived from attribute thresholds via `compute_traits` (player.rs:440–514) — pure attribute-based, position-independent.

## Team + tactic shapes

`domain::team::Team` (team.rs:3–65) is similarly fat. Formation is a free-text `String` field (team.rs:33, default `"4-4-2"`), parsed at runtime by `parseFormationNeeds` on the frontend (PreMatchLineup.tsx:101). Starting XI is `Vec<String>` of player IDs (`starting_xi_ids`, team.rs:54) — auto-selected by OVR if empty.

Tactic is **a flat `enum PlayStyle`** (team.rs:148–156): `Balanced | Attacking | Defensive | Possession | Counter | HighPress`. No numeric dials. No "press intensity 0–20" — just six discrete buckets. Translated to per-zone modifiers via `play_style_modifier(...)` (engine/shared.rs, called from resolution.rs).

Set pieces + leadership: `MatchRoles { captain, vice_captain, penalty_taker, free_kick_taker, corner_taker }` — all `Option<String>` player IDs (team.rs:68–74). Training: `TrainingFocus`, `TrainingIntensity`, `TrainingSchedule` enums + a `Vec<TrainingGroup>` for per-subgroup focus overrides (team.rs:36–50, 76–140).

## Content authoring + storage

**Persistent storage: SQLite via `rusqlite 0.32.1` with `rusqlite_migration`** (db/Cargo.toml:11–12). One DB per save in `app_data_dir/saves/<id>/` (lib.rs:44). 23 versioned migrations (`v001_initial_schema.sql` through `v023_youth_scouting_search_profile.sql`) — forward-only, additive. Many "modern" struct fields are stored as TEXT JSON inside SQL columns: `attributes TEXT NOT NULL`, `traits TEXT NOT NULL DEFAULT '[]'`, `transfer_offers TEXT NOT NULL DEFAULT '[]'` (v001:62–76). This sidesteps schema explosion at the cost of giving up SQL-side querying of those fields.

**Content authoring: hard-coded Rust tables + optional JSON overrides** (generator/definitions.rs:67–95). `NATIONALITY_POOLS` and `TEAM_TEMPLATES` are baked into `data.rs` (666 LoC). `load_names_definition(path)` / `load_teams_definition(path)` allow user JSON files (`NamesDefinition`, `TeamsDefinition`) but the built-ins are the default. Players are **procedurally generated** at world creation — there are no fixture player files. Generation pulls from `NamePool` weighted 60% local nationality + 40% other (generation.rs:46–62).

## Tauri command surface

**49 commands registered** in `invoke_handler!` (src-tauri/src/lib.rs:84–159). All handlers are **synchronous `pub fn`** (no `async`) — Tauri spawns them on its own thread pool. State is shared via `tauri::State<'_, StateManager>` where `StateManager` holds `Mutex<Option<Game>>` + `Mutex<Option<LiveMatchSession>>` + `Mutex<Option<StatsState>>` (state.rs:42–47). One lock per concern, never one fat lock.

The live-match surface is six commands (live_match.rs:35–95):

```rust
#[tauri::command] fn start_live_match(state, fixture_index: usize, mode: String, allows_extra_time: bool) -> Result<MatchSnapshot, String>
#[tauri::command] fn step_live_match(state, minutes: u16) -> Result<Vec<MinuteResult>, String>
#[tauri::command] fn apply_match_command(state, command: MatchCommand) -> Result<MatchSnapshot, String>
#[tauri::command] fn get_match_snapshot(state) -> Result<MatchSnapshot, String>
#[tauri::command] fn finish_live_match(state) -> Result<FinishLiveMatchResponse, String>
#[tauri::command] fn apply_team_talk(state, tone: String, context: String) -> Result<Vec<Value>, String>
```

This is essentially Final Whistle's proposed `play_match` + `match_frames` pattern: start returns initial snapshot, step pulls N minutes of events, snapshot is read-only, finish drains and reports. The `step_live_match(minutes: u16)` knob unifies "tick one minute" and "fast-forward N minutes" — handy precedent for our `match_frames`.

Errors are `Result<T, String>` everywhere (no typed `IpcError`). Many strings are i18n keys (`"be.error.noActiveLiveMatch"`, application/live_match.rs:17). Some commands return `serde_json::Value` for ad-hoc payloads (live_match.rs:85, press conference) — pragmatic but loses TS type help.

## DTO / IPC boundary

**No DTO layer. Canonical types are serialized directly to the frontend.** `engine::MatchSnapshot` (live_match/mod.rs:121–146) is returned raw to TS, containing the full `TeamData` (309 LoC) with both squads' `Vec<PlayerData>`, the full event log `Vec<MatchEvent>`, `home_yellows: HashMap<String, u8>`, `sent_off: HashSet<String>`, etc. Every `step` already returns `Vec<MinuteResult>` and the UI then calls `get_match_snapshot` to refetch the full state (MatchLive.tsx:60–69) — round-trip per minute.

**Floats leak everywhere.** `MatchConfig` is all `f64` (types.rs:187–209: `home_advantage: 1.08`, `goal_conversion_base: 0.30`, `fatigue_per_minute: 0.20`). `PlayerData::overall()` and `effective_overall()` return `f64` (types.rs:93–112). The four team ratings are `f64` (types.rs:142–180). Possession percentages: `home_possession_pct: f64` (live_match/mod.rs:133). Per-minute condition tracking: `HashMap<String, f64>` (live_match/mod.rs:220). No fixed-point anywhere, no determinism harness — `simulate_with_rng` exists (engine/mod.rs:22) for tests but the live path uses `rand::rng()`.

Frontend TS types in `src/components/match/types.ts:5–98` are **hand-mirrored** by name — no `ts-rs` / `specta`. Drift risk is real (`EnginePlayerData` lists 19 attributes verbatim, types.ts:14–39).

## Frontend match rendering

**There is no pitch.** No PixiJS, no canvas, no SVG dots, no animation library. The match screen is `MatchLive.tsx` (60+ LoC) — a panel-based React layout: scoreline header, three tabs (`events | stats | lineups`), and a `SubPanel` flyout. `EventFeed`, `MatchStats`, `Lineups` are the three panels (MatchPanels.tsx:334 LoC). Lineups render formation as text (`{team.formation}`, MatchPanels.tsx:193). The pre-match `PreMatchLineup.tsx` shows `formationNeeds` as a `grid grid-cols-2` checklist (lines 201–230) — no positional dots.

The whole render loop is: `setInterval` → `invoke('step_live_match', { minutes: 1 })` → flush important events into a side list → `invoke('get_match_snapshot')` → repaint panels (MatchLive.tsx:60–95). Speed setting (`paused | slow | normal | fast | instant`) controls the interval (types.ts:152).

## What's worth adopting for Final Whistle T1-1 / T1-5

1. **Two-layer position enum.** Canonical 4-bucket group + 13 granular slots with a `to_group_position()` collapse. The engine consumes the cheap version; UI + transfer market use the rich version. Maps cleanly onto our `BehaviorArchetype` ↔ `PlayerTemplate`.
2. **The `(start, step(n), snapshot, apply_command, finish)` command quintet.** Exactly the shape we sketched. The `step_live_match(minutes: u16)` rather than `step_one_minute()` lets the same surface drive Live + Fast-forward + Instant — three modes, one IPC.
3. **`MatchCommand` enum for between-tick intents** (live_match/mod.rs:39–75). Substitution, formation change, set-piece taker, captain — all the same shape. Validated server-side via `apply_command -> Result<(), String>`. Matches our "UI enqueues intents, sim is sovereign" rule (`Tauri/RULES.md` §2).
4. **Pre-computed stoppage time at half boundaries** (live_match/mod.rs:213–217). Roll once, store, replay. Determinism-friendly.
5. **23 forward-only schema migrations** with TEXT-JSON columns for nested structs — pragmatic save evolution. Matches our `fw-save` schema-versioned bincode plan.
6. **Trait derivation from attribute thresholds** (compute_traits, player.rs:440–514). Cheap "breakthrough"-ish surface without a memory ledger. Even with our richer ledger, this is good prior art for the `Vec<PlayerTrait>` cache projection sent to UI.
7. **Per-player conditions tracked separately from the cached `condition: u8`** (live_match/mod.rs:220 holds `HashMap<String, f64>`). The float lives only inside the live engine state and is collapsed back to `u8` on snapshot.

## What's worth avoiding

1. **`String` IDs and `Uuid::new_v4().to_string()` everywhere.** 36-byte UUID strings as `HashMap` keys, `Option<String>` foreign keys, IDs in 30+ fields. Final Whistle's `u32` newtypes (`PlayerId`, `MatchId`) are 9× smaller and `Copy`. Stick to it.
2. **`f64` in everything labelled "engine".** `MatchConfig`, ratings, possession pct, per-minute conditions — none of this is fixed-point. There's no canonical-hash regression test in OFM. We can't follow this; Q32.32 stays mandatory in `fw-match-sim`.
3. **`HashMap<String, u8>` for yellows + `HashSet<String>` for sent-off** in canonical match state (engine/mod.rs:109–110, live_match/mod.rs:193–194). Non-deterministic iteration; would torch our cross-platform hash gate. Use `BTreeMap` / `BTreeSet`.
4. **No DTO layer.** Canonical engine state ships raw to the frontend with `serde::Serialize` — full `TeamData` (with `Vec<PlayerData>` × 2) per snapshot, every minute. We need a thin `MatchFrameDTO` projection.
5. **`Result<T, String>` with i18n keys.** Stringly-typed errors lose discrimination and force the frontend to string-match. Our `IpcError` enum (`Tauri/RULES.md` §4) is better.
6. **Two parallel engines (`simulate` + `LiveMatchState`).** Resolution logic is duplicated between `engine/mod.rs` and `live_match/simulation.rs`. One engine that the host drives at different speeds is the cleaner factoring.
7. **`HashMap` in domain types** (e.g. `NamesDefinition::pools: HashMap<String, NamePool>`, definitions.rs:18). Even outside the sim ring this hurts when those structs cross deterministic boundaries (world generation, save bytes).
8. **Free-text `formation: String`** (team.rs:33). Final Whistle should ship a typed `Formation` enum or struct (granular slots + relative positions) — string parsing is fragile and a banned-terms / typo magnet.
9. **No 2D pitch.** OFM's text-only match screen is austere; readers say so. Our PixiJS tactical board is one of our differentiators — don't drop it.

## Open questions

- OFM's `condition: f64` per-minute drain (live_match/mod.rs:220, helpers.rs:13–30) suggests our Q32 fatigue model needs to update each tick too — does our `Tick(u32)` cadence match the per-second resolution PixiJS needs, or do we project at lower frequency?
- OFM's `apply_match_command` runs **between minutes**, never within a tick (live_match/mod.rs:294–336). Our intent queue: do we apply at tick boundary only, or interleave with sub-tick actions?
- Their 17-position enum maps to 4-bucket engine consumption via `to_group_position()`. Should our 32-attribute model use a similar two-layer projection: full 32 in canonical state, 8 derived role-scores into the engine?
- They store player attributes as JSON inside a SQL TEXT column (v001:63). With our forward-migrated bincode saves + RON content packs, do we ever want SQLite at all, or is filesystem RON + bincode enough through T8?
- Their `MatchCommand` enum has zero validation beyond what `apply_command` returns — no command capability declarations. Our intents likely need explicit "which side can issue which command" rules.

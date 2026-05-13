# ADR-0004 — IPC command surface for live match

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** osagberg (+ Codex review at phase-T1 gate)

---

## Context

T1-5 lands the real Tauri command surface for the live match — the
boundary between the deterministic Rust sim and the SolidJS frontend.
Today's tree is Phase-0 scaffolding: `crates/fw-tauri/src/commands.rs`
ships a smoke `play_match(seed_hex, tick_count)`, and `src-tauri/src/commands.rs`
ships placeholder `#[tauri::command]` impls (`get_dummy_state`, `play_match`,
`get_league_standings`, `get_squad`, `list_fixtures`) flagged for deletion
by the comment block in `src-tauri/src/main.rs:14-27`. Codex's pre-T0
audit logged the src-tauri consolidation as Important #10
(`docs/postmortems/phase-T0.md:55`); this ADR resolves it.

The non-negotiables predate this ADR:

1. **Sim is sovereign** (`.claude/rules/Tauri/RULES.md` §2). UI reads
   canonical state and enqueues intents; never mutates.
2. **DTOs are projections** (`Tauri/RULES.md` §3). Q32 → f64 at the IPC
   boundary; f64 never returns to canonical state.
3. **Async lives only here** (`Tauri/RULES.md` §1). Sim crates stay sync.
4. **Typed errors** (`Tauri/RULES.md` §4). `Result<T, IpcError>`, never
   `Result<T, String>` (OFM's choice — `docs/research/existing-rust-sims/02-openfootmanager-data-and-tauri.md`
   §"Tauri command surface").

Two references were read end-to-end before drafting: **OpenFootManager**
(Rust + Tauri 2 + React; the live-match command shape we want) and
**ZOXEXIVO open-football** (PixiJS v8 reference for T1-2a; HTTP-chunk
post-match replay model not applicable to a live match).

This ADR locks the live-match command quintet, the `MatchCommand` intent
set, the snapshot + frame DTOs, the frame-streaming model, the
diagnostic payload, the async stance, the src-tauri consolidation
contract, and the TS-type-sync policy. It does not lock the BT-runner
contract (T1-2b), ball physics, the full `MatchEvent` taxonomy, or save
wire shapes — those belong elsewhere.

## Decision

We adopt **OFM's live-match command quintet, retyped for FW's determinism
+ DTO discipline**, and consolidate every `#[tauri::command]` in
`crates/fw-tauri/`. The shell binary at `src-tauri/` becomes a thin
re-export.

### §1. The command quintet

The shape mirrors OFM (`src-tauri/src/lib.rs:84-159` +
`application/live_match.rs:35-95`), narrowed: `step` takes a tick count
(our cadence is `Tick(u32)`), not minutes (OFM's coarse-minute model is
one of the things we are not adopting —
`docs/research/existing-rust-sims/01-openfootmanager-engine.md`
§"Tick structure"):

```rust
#[tauri::command]
pub async fn start_live_match(
    state: tauri::State<'_, AppState>,
    request: StartMatchRequest,
) -> Result<MatchHandle, IpcError>;

#[tauri::command]
pub async fn step_live_match(
    state: tauri::State<'_, AppState>,
    handle: MatchHandle,
    ticks: u32,
) -> Result<StepResult, IpcError>;

#[tauri::command]
pub async fn get_match_snapshot(
    state: tauri::State<'_, AppState>,
    handle: MatchHandle,
) -> Result<MatchSnapshot, IpcError>;

#[tauri::command]
pub async fn apply_match_command(
    state: tauri::State<'_, AppState>,
    handle: MatchHandle,
    command: MatchCommand,
) -> Result<(), IpcError>;

#[tauri::command]
pub async fn finish_live_match(
    state: tauri::State<'_, AppState>,
    handle: MatchHandle,
) -> Result<FinalMatchResult, IpcError>;
```

**`MatchHandle` carries the seed.** Concretely `MatchHandle { id: u32,
seed_hex: String }` — `id` keys into `AppState`'s `BTreeMap<u32,
LiveMatchSession>`; `seed_hex` is informational (replay links, bug
reports). Frontend treats the handle as opaque. Handle-based dispatch
(rather than OFM's "one current match" model in
`src-tauri/src/state.rs:42-47`) lets T2-5's season fast-forward run
multiple matches concurrently without an ADR addendum.

### §2. MatchCommand — between-tick intents

Modeled on OFM's `MatchCommand` (`live_match/mod.rs:39-75`); narrowed to
FW vocabulary and tagged-union for clean TS narrowing:

```rust
#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum MatchCommand {
    Substitute { player_in: PlayerId, player_out: PlayerId },
    ChangeFormation { formation: FormationId },
    ChangePressLevel { level: PressLevel },     // Low | Mid | High
    ChangeTempoBias { bias: TempoBias },        // Slow | Even | Fast
    SetCornerTaker { player: PlayerId },
    SetFreeKickTaker { player: PlayerId },
    SetPenaltyTaker { player: PlayerId },
    SetCaptain { player: PlayerId },
    TeamTalk { message_id: TeamTalkId },        // content-pack-qualified
}
```

The set is **closed**: new intents need an ADR addendum or a logged
decision. Intents apply at the next tick boundary, never mid-tick —
matches OFM (`live_match/mod.rs:294-336`). `PressLevel` and `TempoBias`
are typed enums (closed sets), not numeric sliders — avoids OFM's
`f64`-in-config trap and respects football-native vocabulary
(`Frontend/RULES.md` §9).

### §3. MatchSnapshot DTO

The fat read endpoint. Powers scoreboard, lineup, and event-feed panels.

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatchSnapshot {
    pub handle: MatchHandle,
    pub tick: u32,
    pub minute: u16,                          // tick / TICKS_PER_MINUTE
    pub phase: MatchPhase,                    // FirstHalf | HalfTime | ...
    pub score: ScoreDto,
    pub possession_pct: PossessionDto,
    pub ball_zone: BallZone,                  // 5-bucket (OFM)
    pub home_lineup: LineupDto,
    pub away_lineup: LineupDto,
    pub recent_events: Vec<MatchEventDto>,    // last 16 by default
    pub yellow_cards: BTreeMap<PlayerId, u8>,
    pub sent_off: BTreeSet<PlayerId>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerSnapshotDto {
    pub player_id: PlayerId,
    pub name: String,
    pub shirt_number: u8,
    pub position_slot: PositionSlot,
    pub pos_x: f64,                           // Q32 → f64
    pub pos_y: f64,
    pub condition_pct: u8,                    // collapsed from Q32 stamina
    pub yellow_cards: u8,
    pub sent_off: bool,
}
```

`BTreeMap` / `BTreeSet` at the DTO boundary: serde iterates them in key
order, making the JSON byte-stable across builds and platforms. The
brief allows `HashMap` in pure-display DTOs; we use `BTreeMap` anyway
to keep one rule across the codebase ("which DTO was canonical-feeding
again?" is a question we never want to answer).

### §4. Frame streaming for the tactical board (T1-2a)

`get_match_snapshot` cannot drive a 30Hz dot-renderer — it would
serialize lineup strings + event lists per frame. Decision: **Tauri
event streaming, not polling**. The sim emits compact `MatchFrameDTO`s
the frontend subscribes to with `listen('match:frame', ...)`:

```rust
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MatchFrameDTO {
    pub handle_id: u32,
    pub tick: u32,
    pub ball: [f64; 3],
    pub players: [PlayerPosDTO; 22],          // fixed-size, slot-indexed
}

#[derive(Debug, Serialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub struct PlayerPosDTO {
    pub x: f64,
    pub y: f64,
    pub facing_rad: f64,
}
```

Fixed-size `[_; 22]` array matches open-football's "dense parallel
arrays indexed by slot" guidance
(`docs/research/.../06-frontend-rendering.md`). Sim emits at ~10Hz; the
PixiJS renderer interpolates to 30fps using the
`(timestamp, x, y, z)`-pair pattern from open-football
(`index.html:388-404`). Backpressure: Tauri events are fire-and-forget;
canonical state is re-derivable from `(seed, tick)` so dropped frames
are recoverable.

Scoreboard + event-feed panels keep polling `get_match_snapshot` at
~1Hz — different cadence, different channel.

### §5. Diagnostic-mode payload

Gated by a build feature `fw-tauri/diagnostic` + a runtime toggle in
`AppState`. Parallel event channel `match:diagnostic-frame`:

```rust
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticFrameDTO {
    pub handle_id: u32,
    pub tick: u32,
    pub player_decisions: [PlayerDecisionDTO; 22],
    pub influence_map_summary: InfluenceMapDigestDTO,
    pub bt_trace_tail: Vec<BtTraceEntryDTO>,   // bounded, default 32
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerDecisionDTO {
    pub current_action: &'static str,         // tagged-decisions (ZOXEXIVO)
    pub reason: &'static str,
    pub utility_top3: [(&'static str, f64); 3],
}
```

`&'static str` tags are the ZOXEXIVO pattern: no allocations, greppable,
serde-friendly. Diagnostic emission paths + types are feature-gated out
of release builds.

### §6. Async vs sync handlers

**Uniform `async fn`** across the quintet. The sim itself stays sync
(`Sim/RULES.md` §5); handler bodies wrap the sync API.
`start_live_match` and `finish_live_match` do I/O (content packs, save
files); making every handler `async fn` is cleaner than mixed-stance and
the IPC layer pays the async cost regardless. OFM's sync-only stance
works for them because every load is sync sqlite — our save + content
load story is async-friendlier.

The non-negotiable: **async does not cross into sim crates**. Handler
wraps sim's sync API; sim does not become async.

### §7. src-tauri consolidation (Codex Imp #10)

T1-5 deletes the placeholder surface:

- All `#[tauri::command]` impls in `src-tauri/src/commands.rs` —
  **deleted**.
- All placeholder DTO types in `src-tauri/src/commands.rs` (`DummyState`,
  `MatchEvent`, `MatchResult`, `LeagueStanding`, `PlayerSummary`,
  `Fixture`) — **deleted**.
- The whole file `src-tauri/src/commands.rs` — **deleted** (no shim).
- `crates/fw-tauri/src/lib.rs` exports `pub fn generate_invoke_handler()`
  wrapping the `tauri::generate_handler!` macro list.
- `src-tauri/src/main.rs` becomes a one-line invoke-handler binding +
  the existing log plugin setup; the option-1 path the scaffold comment
  at `src-tauri/src/main.rs:14-27` already anticipates.
- `frontend/src/lib/types.ts` is regenerated by hand against
  `fw-tauri`'s real DTOs (see §8).

### §8. TS type sync — hand-mirrored through T4

`frontend/src/lib/types.ts` continues to mirror `fw-tauri` DTOs by hand.
A new test in `crates/fw-tauri/tests/dto_schema_smoke.rs` `insta`-snapshots
a canonical instance of each DTO; drift surfaces as a snapshot diff on
the Rust side, and the TS file is updated in the same commit.

**Re-evaluate `ts-rs` / `specta` at T4 kickoff.** Rationale for not
adopting now: DTO count at T1-5 is ~10 (tractable); a derive-macro stack
adds an unaudited dep tree; T4 brings ~30 more DTOs for the tabular
surfaces — that's the right inflection. OFM ships 49 hand-mirrored
commands with no public drift bugs — pattern is proven at our scale.

## Consequences

- **Positive:** IPC contract locked before T1-5 lets T1-2a (board) +
  T1-2b (BT runner) develop in parallel. Codex Imp #10 lands as a
  one-shot inside T1-5 rather than smearing across the phase.
  Handle-based dispatch unblocks T2-5 concurrent matches for free.
- **Negative:** Tauri event-emission has subtle threading rules not
  exercised in T0 — budget a couple of days for figure-out during T1-5.
  Closed `MatchCommand` set means new intents need an ADR addendum
  (right call for safety; friction if intents proliferate).
- **Neutral:** Hand-mirrored TS types continue for one more phase, with
  a known re-evaluation point at T4.

## Alternatives considered

- **Polling-only (no event streaming) for the tactical board.** Rejected
  per `docs/research/.../06-frontend-rendering.md` — full-snapshot pulls
  at 30Hz are the wrong shape; OFM gets away with it only because their
  event-feed is sub-Hz.
- **Single fat `tick_and_get` command.** Rejected — conflates "advance"
  with "read"; OFM cleanly separates them and we benefit from copying.
- **`Result<T, String>` errors (OFM's choice).** Rejected per
  `Tauri/RULES.md` §4. `IpcError` gives the frontend
  discriminated-union narrowing; strings don't.
- **Auto-generated TS types now via `ts-rs` / `specta`.** Rejected for
  T1; revisit at T4 per §8.
- **`HashMap` in DTOs (allowed by brief).** Rejected anyway — same rule
  everywhere removes a foot-gun. Cost is zero at our scale.
- **Per-command async stance (sync for reads, async for writes).**
  Rejected per §6 — uniform `async fn` is clearer.
- **Diagnostic data on the main frame channel.** Rejected — release
  builds would deserialize empty fields every frame. Separate channel +
  feature gate is the lower-coupling design.

## References

- `.claude/rules/Tauri/RULES.md` §1–§8
- `.claude/rules/Sim/RULES.md` §1, §2, §5, §6
- `.claude/rules/Frontend/RULES.md` §1, §4, §7
- `docs/research/existing-rust-sims/01-openfootmanager-engine.md`
  §"Tick structure"
- `docs/research/existing-rust-sims/02-openfootmanager-data-and-tauri.md`
  §"Tauri command surface", §"DTO / IPC boundary"
- `docs/research/existing-rust-sims/06-frontend-rendering.md`
  §"IPC + state-streaming patterns"
- `docs/MASTER_PLAN.md` rows T1-2a, T1-5
- `docs/postmortems/phase-T0.md:55` (Codex Imp #10 deferral)
- `crates/fw-tauri/src/{lib.rs,commands.rs}` (Phase-0 scaffold)
- `src-tauri/src/{commands.rs,main.rs}` (placeholders to be deleted)
- Prior ADRs: none — this is the first authored ADR in `docs/adr/`. The
  worked-example list in
  `templates/design-templates/architecture-decision-record.md` is
  illustrative, not historical.

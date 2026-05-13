---
description: Tauri IPC patterns. Async OK in command handlers. UI never drives canonical state.
applies_to:
  - crates/fw-tauri/**
  - src-tauri/**
auto_load: when_editing_matching_path
---

# Tauri IPC rules

## §1. Async is allowed (this crate only)

Tauri command handlers MAY be `async fn`. This is the only place in the Rust workspace where async lives.

```rust
#[tauri::command]
async fn get_squad(club_id: ClubId, state: tauri::State<'_, AppState>) -> Result<SquadDTO, IpcError> { ... }
```

`fw-match-sim` and `fw-memory` remain sync (see `Sim/RULES.md` §5).

## §2. UI never drives canonical state

- Tauri commands READ the sim's last committed state or ENQUEUE intents for the next tick.
- Tauri commands DO NOT mutate canonical state directly.
- Enqueued intents flow through the deterministic tick loop; the sim is sovereign.

## §3. DTOs are projections, not canonical types

- DTOs (`SquadDTO`, `MatchFrameDTO`, etc.) are **read-only projections** of canonical state for the frontend.
- DTOs use `f64` freely (UI doesn't need Q32) — that's the boundary translation. Convert `Q32` → `f64` in the DTO build.
- DTOs are NEVER serialized back into canonical state. One-way only.
- Use `#[serde(rename_all = "camelCase")]` on payloads so TS receives `playerName`, not `player_name`.

## §4. Error responses

- Command return type: `Result<T, IpcError>`.
- `IpcError` implements `serde::Serialize + thiserror::Error + Send`.
- Never panic in a handler. Map all errors to `IpcError` variants.
- Front-end shows error variants via a typed `IpcError` discriminator.

## §5. State management

- App state via `tauri::State<'_, AppState>`. `AppState` holds the sim's last committed snapshot + queues for pending intents.
- Mutex/RwLock OK in `fw-tauri` (this is outside the sim ring).
- Never expose `&mut SimWorld` through state — always work on a snapshot.

## §6. Capabilities (security)

- All capabilities declared in `src-tauri/capabilities/default.json`.
- Least-privilege: only enable the windowing / fs / IPC capabilities the app actually uses.
- No `allow-all` or wildcard capabilities.

## §7. TS type sync

- Frontend types in `frontend/src/lib/types.ts` mirror Rust DTO shapes.
- Manual sync for now; consider `ts-rs` or `specta` at Phase T4 if drift becomes painful.

## §8. Logging

- `tracing` crate for structured logs.
- Sensitive data NEVER logged (player save data, paths with usernames, etc.).
- `tracing::error!` for command failures; `info!` for command entry/exit at debug level.

## Cross-references

- `CLAUDE.md` §3 (Tauri 2 stack), §7 (UI never drives canonical state)
- `Sim/RULES.md` — what stays sync (sim crates)
- `Frontend/RULES.md` — TS side of the IPC boundary

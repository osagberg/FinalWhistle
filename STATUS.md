# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** T1-11 shipped: signatures wired into normal `tick_match` end-to-end (`MatchState::initial_with_content` + `tick_match(state, &sig_definitions)` + fw-tauri IPC threading + dev-board smoke). Audit P1 closed. The match-engine inner loop now narrates real football: T1-3.5 makes ball actions happen → T1-4a emits MatchEvents → T1-4b renders Tracery commentary → T1-11 makes signatures actually fire in the normal path. Next: T1-5 Tauri play_match + IPC consolidation + match_frames cap.

## Active task

(none — T1-11 closed across 1 implementation pass + 1 main-thread fix-pass for 1 P0 + 2 P1 self-review findings)

## Phase pointer

- **Just closed:** **T1-11** — `MatchState::initial_with_content(seed, &ContentStore)` constructor projects slot 7 (home AM) signature_candidates; `tick_match` signature change to accept `&BTreeMap<String, SignatureDefinition>`; dump_frames --content flag + smoke test asserting ≥1 SignatureFirstFired in 600 ticks; folded in Codex T1-4b P2 (MatchEvent::discriminant() returns typed enum + cross-crate alignment test). Fix-pass closed code-reviewer's P0 (fw-tauri IPC commands still passed BTreeMap::new) + 2 P1s.
- **Next:** **T1-5** — `fw-tauri::play_match` + IPC consolidation (delete src-tauri local stubs; fw-tauri is the only command surface) + match_frames unbounded-request cap (`MAX_FRAMES_PER_REQUEST` const + typed IpcError::TooManyFrames). **T1-5 scope should be amended to absorb the AppState ContentStore migration that T1-11's fix-pass deferred** — currently `fw-tauri::commands.rs` loads ContentStore inline on every IPC call (`load_content_for_command()` helper with FW_CONTENT_PATH env-var override); the proper fix lifts ContentStore into AppState constructed once at app startup.
- **Recommended /next order** (per audit triage): **T1-5** → T1-12 (content validation hardening) → T1-10 (LUT bake) → T1-13 (frontend Vitest + cargo audit) → T1-6 (frontend Match polish) → T1-7 (procgen) → T1-8 (replay corpus) → T1-9 (behavioral assertions).

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-11 + fix-pass: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `782fcde6…8c0f` + banned-terms + determinism-audit + `fw-content-baker validate`.

## Last canonical hash

`blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f` (60-tick smoke seed; UNCHANGED since T1-3.5; T1-4a/T1-4b/T1-11 all kept it stable — content side + new constructor paths don't reach the existing smoke seed which uses `MatchState::initial(seed)` with `&BTreeMap::new()` for sig_definitions).

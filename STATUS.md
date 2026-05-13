# STATUS — Final Whistle

**Last updated**: 2026-05-13

## Phase

**T1 — First Match** (active; T0 closed and Codex-approved at `4721fee6` + hardening `bad1a400`)

## Active task

(none — awaiting first `/next` on T1)

## Phase pointer

- **Just closed:** Phase T0 — Scaffold. All 13 rows DONE. T0-7b cross-OS canonical-hash agreement verified by CI matrix on `4721fee6`. Codex APPROVED 2026-05-13. Postmortem at `docs/postmortems/phase-T0.md`.
- **Now:** Phase T1 — First Match (8 rows). Two procedural teams play one match end-to-end with a text recap.
- **Critical path:** T1-1 → T1-2 → T1-4 → T1-5 → T1-6.

## Blockers

None.

## Last green verify

2026-05-13 — CI matrix green on `[macos-14, windows-latest, ubuntu-22.04]` at HEAD: `cargo fmt --check` + `cargo clippy --workspace --all-targets -- -D warnings` + `cargo test --workspace --release` + `pnpm typecheck` + `pnpm lint` + `pnpm build` + `determinism-audit` + `banned-terms` + canonical-hash regression. **First all-green CI matrix run in this codebase.**

## Last canonical hash

`blake3:d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49` (60-tick smoke seed; pinned T0-7; cross-OS-verified T0-7b).

## Recent commits

- `bad1a400` chore(ci): cache-bin: false on Swatinem invocations (preempt flake)
- `4721fee6` fix(ui): green up frontend typecheck + lint + build for CI matrix
- `a612e585` docs(audit): SKILL.md atomic Step 7/8 + stale-refs cleanup
- `89479063` fix(ci): unblock the matrix + workflow polish (Codex pre-merge audit)
- `a0b2e084` fix(ci): unblock GitHub Actions matrix — MSRV bump + pnpm lockfile + workflow path

## Next up

`/next` will pick **T1-1** — `fw-content` schema (`TeamTemplate` + `PlayerTemplate` + `BehaviorArchetype` + first RON fixtures under `content/sources/`).

Per `docs/MASTER_PLAN.md` T1 acceptance gate: two procedural teams play one match end-to-end; text recap surfaces with goals + score + key events.

## T1 risks (carried from postmortem)

- **T1-2 size** — ball physics + 22-player BT runner is XL (~1w). Largest single row on the plan; determinism cliffs concentrated there. Consider splitting into T1-2a (ball physics integrator) + T1-2b (BT runner) if early progress is slow.
- **f32 in TacticalArchetype.buildup_speed_factor** (Codex Imp #3 deferred) — needs conversion to u16 bps before BT-runner consumes it. T1-1 is the right phase to do that since it's the first row to touch TacticalArchetype materially.
- **src-tauri command consolidation** (Codex Imp #10) — placeholder commands shadow fw-tauri; T1-5 is the planned consolidation.
- **insta-snapshot baseline** — `smoke_seed_final_state_snapshot` still `#[ignore]`. T1-1 or T1-2 should unignore once there's real sim behavior to snapshot.

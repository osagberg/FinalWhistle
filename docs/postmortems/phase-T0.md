# Postmortem — Phase T0: Scaffold

**Author:** producer (Claude, main thread)
**Date:** 2026-05-13 (phase closed; Codex-approved)
**Phase duration:** 2026-05-13 single working session
**Codex verdict:** APPROVE at `4721fee6` + hardening `bad1a400`

---

## What shipped

The whole T0 scaffold + the audit fallout that closed the gate. 13 MASTER_PLAN rows: 12 DONE on dev box, T0-7b verified by CI matrix.

- `81fdeff` — Rust pivot scaffold (109 files; clean-slate from Unity+C# preserved at tag `v0-pre-pivot-2026-05-13`)
- `26f1ba0` — blueprint reconciliation (51 files; 14→7 agents, 35→6 commands, 14→5 hooks, 5→3 context scopes)
- `7dc510d` — Codex pre-T0 audit followup, quick wins (9 of 16; STATUS bootstrap; sanity hash test)
- `9eb184e` — Codex audit Q1+Q2 (Q32 panic-on-overflow + durable u32 IDs)
- `239594e` — T0-7 pin BLAKE3 canonical hash on macOS-14 dev box
- `73c1da0` — T0-12 unblock workspace verify (Tauri lib.rs `pub` bug + 4 scaffold gaps)
- `ea4e452` — /done phase-close ledger sync
- `a0b2e08` — CI unblockers (MSRV 1.85→1.95 for fixed@1.31; pnpm-lock at repo root)
- `89479063` — Codex pre-merge audit Critical 1+2 (icons unignored; Linux Tauri deps)
- `a612e585` — Codex Important 4+5 + Nice 8+9 (SKILL.md atomic Step 7/8; stale-refs)
- `4721fee6` — Frontend green-up (`@types/node`; defineConfig sync; Router children; `<For />`)
- `bad1a400` — `cache-bin: false` hardening (Swatinem flake class)

**Canonical-state hash:** `blake3:d6258107b2c90c84d2feeaa8633d1f5c159e10ccd2016623b52b41d3d96b1a49`. Cross-OS-verified.

## What went well

- **The blueprint reconciliation paid off.** The `/next` workflow proved out cleanly once T0-12 unblocked workspace verify. By T0's close the loop was: type `/next` → spec → dispatch → verify → commit → next. The 9-step pipeline didn't need restructuring mid-phase.
- **Codex's pre-T0 audit was high-signal.** 16 findings; 14 landed in tree, 2 were intentional deferrals with explicit MASTER_PLAN rows. Estimated ~6 weeks of T1 debugging avoided by getting Q32 panic-on-overflow + durable IDs locked before sim work began.
- **Determinism hardening had real teeth.** `scripts/determinism-audit.py` caught a latent `HashSet` in `fw-replay`'s own tests before it could drift the cross-OS hash. The hash itself agreed bit-for-bit across macOS-14 / windows-latest / ubuntu-22.04 on first matrix run after the toolchain was right.
- **One pivot, eight commits, T0 done.** From `81fdeff` (clean-slate) to Codex APPROVE in a single session, with five subprocess-deep `/next` cycles in between. Solo-dev cadence is real.

## What went poorly

- **Scaffold debt landed at the wrong layer.** The initial `81fdeff` pivot generated 109 files via parallel agents, several of which had pre-existing-but-untested issues (Tauri `pub` macro bug, fw-content-baker dead-code, frontend strict-typecheck failures, gitignored icons, missing `@types/node`, missing `pnpm-lock.yaml` at the workspace root the workflow pointed at, `frontend/dist` macro requirement, missing icon files for Tauri `generate_context!`, `Icon?` gitignore matching `icons` on case-insensitive macOS FS, MSRV mismatch with `fixed@1.31`). Every single one only surfaced when CI actually ran. Each was small, but they stacked: macOS dev-box green → first push → all CI red on a workflow path → fix → push → red on a config issue → fix → push → red on toolchain MSRV → fix → push → red on frontend → fix → push → green. **Six commits to close the gap between "green on dev box" and "green on CI matrix."**
- **Two of the workflow YAML files (`ci.yml`, `release.yml`) referenced `frontend/pnpm-lock.yaml`** as the cache path. The pnpm-workspace.yaml at repo root puts the lockfile at the repo root. Same kind of "scaffold authored without ever exercising it" bug. The path being wrong is trivial; the surface area meaning the wrong assumption nobody noticed is the lesson.
- **The agent-orchestrated reconciliation lost 10 of 10 agent's tool calls.** The `(Tools: All, tools)` permission spec on the project-scoped agents appears malformed; agents produced text but no Write calls. Caught only because I checked the filesystem after agent returns. **Trust-but-verify on subagent work is mandatory, especially for fresh project setups where the permission surface isn't proven.**

## What we learned

- **"Green on dev box" ≠ "green on CI."** Always assume the CI matrix has *less* than the dev box: no globally-installed types, no system deps the dev box happens to have, case-sensitive filesystems, fresh Rust install with no cached `~/.cargo/bin`. The dev box's environment leaks into local pass rates in ways that hide real bugs.
- **`pnpm typecheck` + `pnpm lint` + `pnpm build` need to run before the first push.** Locally these had been skipped. Six errors fell out across `vite.config.ts`, `main.tsx`, `Layout.tsx` the moment they ran on CI. Lesson: even when a verify suite *exists*, exercise every gate from a clean state before declaring the scaffold ready.
- **A pinned MSRV propagates further than expected.** `rust-version = "1.85"` in workspace Cargo.toml + `channel = "1.85"` in `rust-toolchain.toml` was set during scaffold based on "edition 2024 needs ≥1.85" reasoning. That was correct in isolation, but `fixed@1.31` (the Q32.32 backing crate — load-bearing) requires rustc 1.93. Lesson: when pinning the toolchain, also pin against the MSRV of the most demanding *load-bearing* dependency, not just edition baseline.
- **Codex review at phase boundaries works, but the pre-boundary audit was even higher signal.** The "look at my scaffold before I start running /next" review caught 4 Critical issues that would've corrupted the canonical hash or blocked /next outright. **Recommend a pre-phase audit for T1 too**, before T1-2 (ball physics + BT runner — the largest single row) lands.
- **Phase-close requires 6 commits not 1.** The mental model of "phase done = ship one PR" is wrong for a fresh codebase. T0 closed in 6 follow-up commits past the `/done` ceremony. That's normal for the first phase and shouldn't be normal for later phases — but plan for it.

## Action items for T1

- **Get a Codex pre-T1 audit before T1-2 starts.** Same model as pre-T0: read the scaffold, find what'll bite. Specifically have Codex stress-test the BT-runner contract + Q32 use in ball physics.
- **Author `docs/specs/save-migration-fixtures.md`** during T2 prep, before fw-save's V2 schema bump.
- **Convert `TacticalArchetype.buildup_speed_factor: f32` to `u16` basis points** as part of T1-1 — Codex Imp #3 deferred. Don't let T1-2 inherit it.
- **Resolve src-tauri command consolidation at T1-5** as planned. Drop the all-zero `canonical_hash` placeholder string (Codex Imp #7) — prefer `Option<String>` until real wiring lands.
- **Track CI-matrix run cost.** T0 burned ~6 CI runs at ~5 min each across 3 OSes = 90 OS-minutes of compute on debugging scaffold. T1 should burn that on real testing of behavior, not infra debugging.
- **Codex post-merge re-audit cadence.** Schedule one mid-phase + one pre-phase-gate.

## Stats

- Commits in phase: **12** (8 substantive + 4 audit-follow-ups)
- LoC delta: ~12,000 added (pivot + blueprint reconciliation dominate; subsequent fixes <500 lines each)
- Tests added: 19 test-runs across the workspace at phase close; insta snapshot baseline still pending T1
- Decisions logged in `docs/DECISIONS.md`: 0 (architecture choices flowed through Codex audit + STATUS open-questions and commit-body documentation; first formal DECISIONS entry expected at T1's BT-runner contract)
- Canonical-hash re-pins: 1 (the initial pin — by design)
- Phase-gate verdict: **PASS** (Codex approved 2026-05-13)

## Cross-references

- `CHANGELOG.md` — Phase T0 block
- `docs/BLUEPRINT_RECONCILE.md` — the 14→7 / 35→6 / 14→5 slimming audit
- `docs/MASTER_PLAN.md` — Phase T0 + Phase T1 rows
- `MEMORY.md` "Recently completed" — T0-7 + T0-12
- Codex pre-T0 audit findings: resolved inline across `7dc510d`, `9eb184e`, `239594e`, `73c1da0`, `89479063`, `a612e585` commit bodies
- Phase PR / discussion: solo-dev direct-to-main; no PR — Codex reviewed via filesystem against `origin/main` HEAD

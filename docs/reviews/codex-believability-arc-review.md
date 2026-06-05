# Codex review kickoff — the believability arc

Paste the block below into a fresh Codex CLI session at the repo root
(`/Users/vibelogic/dev/football`). We keep committing to `main` meanwhile;
the prompt anchors Codex to a frozen SHA so it isn't chasing a moving tree.

---

You are doing an independent, adversarial code review of the Final Whistle match-simulation engine. This is a deterministic procedural-fantasy football manager — a Rust workspace (`fw-core`, `fw-match-sim`, `fw-content`, `fw-memory`, `fw-replay`, `fw-save`, `fw-scouting`, `fw-tauri`) behind a Tauri 2 + SolidJS frontend. Read `CLAUDE.md` first (the project contract) and `.claude/rules/Sim/RULES.md` (the determinism non-negotiables) — those define the bar.

**Review target:** git HEAD = `fcb3943f` (tree is clean). We are still committing to `main` while you review, so anchor on this exact SHA — `git checkout fcb3943f` for a frozen tree.

**What to review — "the believability arc."** The last full external review was the post-T1 ultimate review (`docs/audits/post-t1-ultimate-review-2026-05-16.md`, mid-May 2026). Everything since is unreviewed externally. Walk `git log fcb3943f` back to that point. The substantive canonical-state work in that range:
- **Team defensive shape** — held zonal block (FUN-TS1). `crates/fw-match-sim/src/team_shape.rs`, `tactic_fsm.rs`, `bt/off_ball.rs`.
- **Realistic pass-kind mix** — short/long/cross/lay-off chosen by zone-conditional utility (FUN-TS3b). `bt/on_ball.rs`, `dispatch.rs`.
- **Shot model** — xG-gated shooting, dispersion (SIGMA), GK save model with floors. `lib.rs`, `dispatch.rs`.
- **Centre-back contesting / loose-ball handling** (FUN-CB1). `separation.rs`, ball physics.
- **Several AUTHORIZED canonical-hash rebaselines** (the pinned BLAKE3 state hashes in `crates/fw-replay`) that accompanied these behaviour changes.

**Focus your scrutiny, highest value first:**
1. **Determinism contract** (Sim/RULES §1–11). Any `f32`/`f64` in canonical state / `MatchEvent` / `MemoryEvent`; `HashMap`/`HashSet` in sim/memory/replay/save/content; `Instant`/`SystemTime`/`thread_rng` in sim; `async`/`tokio` in `fw-match-sim` or `fw-memory`; `saturating_*` on sim newtypes without a SAFETY justification; `debug_assert!` guarding a real canonical/gameplay invariant (must be `assert!`); non-BLAKE3 hashing; non-append-only `MatchEvent` discriminant changes; non-deterministic iteration order.
2. **Rebaseline integrity.** For each canonical-hash re-pin in this range, read the commit diff + body: was the re-pin driven by a legitimate intended behaviour change, or could it be masking an unintended regression? This is the highest-trust operation in the project and has had only internal review.
3. **Test / measurement integrity ("masking").** We caught several in-session attempts to fake green — loosened thresholds, regression seeds deleted or added for the wrong reason, widened tolerances, disabled / `let _ =`'d assertions, a physics overlap mislabeled as harmless. The question for you: did any masking survive into committed code? Check the proptest invariants, insta snapshots, and the pinned-hash tests specifically.
4. **Football-authenticity soundness of the shipped models** — the pass-mix bias math (zone multipliers, floors/ceilings, the cross-clamp), shot dispersion, and the save-model floors (`SAVE_BASE` >= 0.50/0.72). Are the formulas honest and the constants defensible, or is anything fudged?
5. **The IPC boundary / frontend contract.** `fw-tauri` DTOs + the frontend `MatchEventKind` union (`frontend/src/lib/runtime-validators.ts`, `Match.tsx`) — check for discriminant drift (a Rust `MatchEvent` variant the frontend doesn't handle, which silently rejects matches). Note: `f64` in `fw-tauri` DTOs is the SANCTIONED Q32→f64 boundary translation, not a violation.

**What to IGNORE (known / in-flight / deliberate — don't spend review budget here):**
- **Goal-production / "drift goals."** KNOWN issue (~29% of goals are uncontested balls crossing the line, not shots), fully documented in `docs/design/goal-production-drift-goals.md`, fix in flight right now. Do NOT re-report it. Only flag if you find something the doc doesn't cover.
- **FUN-TS4 shot-volume** — parked deliberately. The `docs/wip/*.patch` files are preserved work-in-progress, NOT live code — ignore them entirely.
- **FUN-PHYS-1** (ball-contact collision / goal-safe steering) — parked, documented.
- Any `docs/design/football-authenticity-gap-map.md` or `docs/design/visual-fidelity-*.md` — exploratory research landing in parallel, not commitments.
- **Process.** We deliberately moved off the strict phase/gate cadence to a dynamic best-game-first roadmap (no EA milestone). Don't bikeshed the workflow drift; review the code.
- Uncommitted working-tree changes if any appear — review committed `fcb3943f` only.

**Deliverable:** prioritized findings (P0 blocker / P1 should-fix / P2 nit), each with `file:line` and a concrete fix. End with: (a) a per-area verdict (Accept / Revise / Reject) for the five focus areas, and (b) a direct yes/no — "did any masking or any illegitimate canonical rebaseline slip through?" If yes, name the commit. Determinism note: review runs on the dev box; CI runs the macOS/Windows/Linux matrix, so flag anything platform-dependent.

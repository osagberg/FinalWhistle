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

`/next` will pick **T1-1** — `fw-content` schema (`TeamTemplate` + `PlayerTemplate` + `BehaviorArchetype` + first RON fixtures under `content/sources/`, folding in the Codex Imp #3 `f32 → u16 bps` conversion).

T1 was restructured 2026-05-13 to insert a developer-tier verification surface: see `docs/design/dev-verification.md`. Row count went 8 → 10 to add T1-2a (2D tactical board, pulled forward from T4) and T1-9 (behavioral proptest invariants). The XL T1-2 split into T1-2a (board, M) + T1-2b (BT runner, L). Rationale: without a visual debug surface, "is this football or random walking?" is unanswerable, and FW v1's worst sim bugs (static-ball, brain-dead pressing, GK-wanders-midfield) were only ever caught by eyeballing the dots viewer.

Per `MASTER_PLAN` T1 acceptance gate: two procedural teams play one match end-to-end; the 2D board renders it; a stranger watching 30s can identify formation shape + sides; 5 behavioral invariants hold over 100 random seeds; text recap surfaces goals + score + diagnostic commentary.

## Open T1 risks

- **T1-2b (BT runner) is still the largest row even after splitting.** L (5d). Determinism cliffs concentrate there. A Codex pre-T1-2 audit is recommended (same model as the pre-T0 audit that caught 14 real findings).
- **f32 in TacticalArchetype.buildup_speed_factor** (Codex Imp #3) — folded into T1-1 done-criteria.
- **src-tauri command consolidation** (Codex Imp #10) — folded into T1-5 done-criteria.
- **insta-snapshot baseline** — `smoke_seed_final_state_snapshot` still `#[ignore]`. T1-2b should unignore once real sim behavior exists to snapshot.
- **Sports-sim research complete** (this session) — 8 parallel agents covered FM, OOTP, FOF, EHM, PCM, Tennis Elbow, F1M; Dwarf Fortress / CK3 / RimWorld / Sims; AI techniques (BT/UAI/GOAP/HTN); football analytics (xG/xT/VAEP/pitch-control); verification/QA practices; player attribute systems; and a 55-link bibliography. All notes at `docs/research/sports-sims/`. Synthesis at `00-synthesis.md` — that's the doc T1-2b should be built against. Two structural findings worth flagging: our determinism floor exceeds every shipped competitor, and our "growth-via-ledger" pillar 3 is genuinely differentiated (every shipped sim has cinematic-bolt-on-XP). Composed architecture is a 7-layer stack (team tactic → BT → utility-with-bias-vector → influence maps → reactive interrupts → steering → Q32 locomotion) with xG/xT/pitch-control as the actual math.

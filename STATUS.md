# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** **The match-engine vertical is now complete end-to-end from sim through user-visible frontend.** T1-6 shipped: SolidJS Match page consumes T1-5's `MatchResult` IPC contract. Play button (configurable seed + ticks) runs `playMatch` IPC; final scoreline rendered prominently; commentary preview rendered with minute markers + goal-event emphasis; event list rendered as structured table with colored badges per kind; dev-board toggle lazy-imports the FrameSource-driven Pixi board behind a checkbox (opt-in). Browser-preview mode (running `pnpm dev` outside Tauri) returns a mock MatchResult so the surface renders without backend, gated by a visible "Preview mode" banner so a developer can't confuse it for a real sim run. Vitest substrate activated (`pnpm test` was previously exit-1 "no test files"; now exits 0 with 10/10 passing) — substrate-warming for T1-13's broader frontend test gate. The full vertical: T1-3.5 ball actions → T1-4a MatchEvents → T1-4b Tracery commentary → T1-11 signatures wired → T1-12 content validation hardened → T1-5 typed IPC contract → T1-6 user-visible Match page. Next: T1-10 (LUT bake — replace runtime f64 LUT generation with committed Q32 tables; 4th of 5 audit-triage P1 closures).

## Active task

(none — T1-6 closed across 1 implementation pass + 1 main-thread fix-pass for 3 P1 + 3 P2 + 1 P3 self-review findings; T1-10 starting next per audit-triage order)

## Phase pointer

- **Just closed:** **T1-6** — frontend Match page consuming T1-5 MatchResult. Match.tsx rewritten 109 → ~590 LoC (post fix-pass) with: header (seed hex input + ticks input + Play button + preview-mode banner gated on `!isTauri()`); scoreline `H – A` in display font text-4xl; two-column section (Commentary panel with minute markers + goal emphasis; Events panel with structured list + colored badges per `MatchEventKind`); typed `IpcError` narrowing via `isIpcError` guard + exhaustive switch with `_exhaustive: never` default (compile-error on new IpcError variants); dev-board toggle below recap (lazy-imports Dev/TacticalBoard, Pixi bundle only loads on toggle). Vitest substrate: new `vitest.config.ts` (jsdom + solid plugin) + `frontend/src/test-setup.ts` (jest-dom) + 10-test `Match.test.tsx`. Fix-pass closed: 3 P1 (tickCount silent-NaN-fallback → ticksValid hardening; `MatchEvent.kind: MatchEventKind | string` defeats exhaustiveness → dropped `| string` + typed `eventLabel`/`badgeClass` switches with `never` defaults; dev-board label drift → explicit caveat citing T4-1 deferral); 3 P2 ("HT" misnomer → "Seed:" prefix; `isIpcError` structurally too permissive → `KNOWN_IPC_ERROR_KINDS satisfies ReadonlySet<IpcError["kind"]>`; `lazy` imported twice from solid-js → merged); 1 P3 (preview-mode banner gated on `!isTauri()`).
- **Next:** **T1-10** per audit-triage order — `fw-core`: bake-time SIGMOID_LUT + EXP_LUT generation. Currently built at process startup via `f64::exp()` + quantized to Q32; cross-OS hash passes today but libm/platform-dependent by design (Codex 2026-05-16 audit P1 "Runtime f64 LUT bake is a determinism risk"). T1-10 commits the tables as source / committed data + makes runtime utility math pure Q32. After T1-10: T1-13 (frontend test gate + cargo audit) is the 5th + final audit-triage P1.
- **Recommended /next order** (updated post T1-6): **T1-10** → T1-13 (frontend Vitest broader gate + cargo audit) → T1-7 (procgen) → T1-8 (replay corpus) → T1-9 (behavioral assertions). All 4 of 5 audit-triage P1s are closed (T1-3.5 ball-actions P0 + T1-5 IPC split-brain + T1-5 match_frames unbounded + T1-11 signatures unreachable + T1-12 content validation); T1-10 + T1-13 are the remaining 2.

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-6 + fix-pass: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `782fcde6…8c0f` + banned-terms (frontend/src/ included) + determinism-audit + `fw-content-baker validate`. Frontend `pnpm typecheck` + `pnpm lint` + `pnpm test` (10/10) + `pnpm build` (Match chunk 10.11 kB gzipped 3.73 kB) all clean.

## Last canonical hash

`blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f` (60-tick smoke seed; UNCHANGED since T1-3.5; T1-4a/T1-4b/T1-11/T1-12/T1-5/T1-6 all kept it stable).

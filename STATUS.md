# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** **T1-3.6 shipped** — closes Codex's post-T1-7 adversarial multi-agent audit P0 (ball moved in 0/601 frames in 600-tick smoke seed runs because BT carrier routing returned `self` unconditionally — never moved the carrier into `InPossession`, so the on-ball BT branch was unreachable in production despite T1-3.5 having correctly authored ball-mutation code). Bundles the audit's 4 P1s in one /next cycle per user scope decision: (1) BT carrier routing in `role_states::evaluate_transitions` reads `state.possession`; (2) `MatchFrameDto.possession: Option<u8>` field exposed on dev-board IPC; (3) frontend per-DTO runtime shape guards (`isMatchResult` / `isMatchFrameDTO` / `isBackendHandshake`) + `safeInvoke<T>` wrapper that throws `IpcShapeError` on guard fail (closes the wire-shape-drift class Codex caught at Tier-2 post-T1-6 — for the consumer side this time); (4) canonical-hash unique-attribute test (1485 pair-swaps across all 55 `PlayerAttributes` fields with distinct raw bit values; catches the canonical-encoding hole where two structurally-different fixtures could hash-equal because baseline values were identical across fields); (5) stale-doc reconcile (MEMORY top-line + MASTER_PLAN snapshot). Self-review triple all ACCEPT (1 P1 carrier-routing duplication + 1 P1 pass_intent prop_assume gap + bounds-tightening folded in main-thread fix-pass; 2 P2/P3 deferred to follow-up rows). **Empirical reproduction of Codex's exact criterion now CLOSED**: ball moves in 595/601 frames (was 0/601 pre-fix); canonical hash REBASELINED `782fcde6…8c0f` → `ddccaf88…000b3` per ADR-0012 trigger #1. **T1 phase status: 8 of 11 numbered rows now DONE** (T1-3.6 added to the 7-of-11 baseline). The Codex 2026-05-16 audit triaged separately during this row (paste was against pre-T1-3.6 HEAD `537148f3`): every finding turned out to be already closed by the audit-triage sweep (T1-5 / T1-10 / T1-11 / T1-12 / T1-13) plus T1-3.6 itself. No new MASTER_PLAN rows added.

## Active task

(none — T1-3.6 closed across one main-thread implementation pass + one fix-pass for self-review P1 findings)

## Phase pointer

- **Just closed:** **T1-3.6** — BT carrier routing + MatchFrameDto.possession + frontend runtime shape guards + canonical unique-attribute test + stale-doc cleanup (Codex post-T1-7 adversarial multi-agent audit response — single-combined row per user scope decision). ~700 LoC source + tests across 4 crates + frontend; canonical hash REBASELINED to `ddccaf88…000b3`. The "match-engine vertical complete" claim from T1-6 / T1-7 is NOW ACTUALLY TRUE.
- **Next:** **T1-8** per MASTER_PLAN order — Replay corpus fixture #1 (smoke seed, 600 ticks, two-archetype matchup, pinned canonical hash on CI matrix). Creates `crates/fw-replay/fixtures/0xfeedbeefcafefade.ron`. OR **T1-9** (behavioral assertions — proptest invariants for 4 positional + PlayerSeparation + 3 pair-seed knob-isolation + events_chronological per ADR-0007).
- **Recommended /next order**: **T1-8** → T1-9 → `/done`. T1-8 is mostly mechanical fixture bake + CI wire-up; T1-9 is substantive proptest authoring; both can ship in either order since they're independent. After T1-9, T1 phase is ready for `/done` + Codex Tier-3 phase-boundary review.

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-3.6 + self-review fix-pass: cargo fmt + clippy + cargo test --workspace + pnpm test (56 frontend tests) + canonical-hash regression on the REBASELINED `ddccaf88…000b3` + banned-terms + determinism-audit + fw-content-baker validate + cargo audit + cargo deny check.

## Last canonical hash

`blake3:ddccaf88c94f328274d484ed1e14ced8638d1ccf63bb922ad64a4f28664000b3` (60-tick smoke seed; **REBASELINED 2026-05-16 at T1-3.6 per ADR-0012 trigger #1** — BT carrier routing produced actual ball motion in the smoke seed for the first time, which produced actual MatchEvents, which shifted `match_events` canonical encoding. Prior pin `782fcde6…8c0f` had stayed UNCHANGED across all 11 commits from T1-3.5 onward — which WAS the bug Codex's adversarial audit caught: the hash didn't change because no football was happening). The hash drift IS the proof T1-3.5 didn't actually work end-to-end before T1-3.6.

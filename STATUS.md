# STATUS — Final Whistle

**Last updated**: 2026-05-15

## Phase

**T1 — First Match** | **T1-2b SUB-PHASE FULLY CLOSED** (all 9 rows ship: i, ii, iii-a/b/c/d, T1-3, iv). Match-engine inner loop complete. Next: T1-4 MatchEvent emission. Mid-phase Codex audit recommended on the T1-2b accumulated diff.

## Active task

(none — T1-2b-iv closed; T1-2b sub-phase fully complete. `/next` picks T1-4.)

## Phase pointer

- **Just closed:** **T1-2b-iv signature dispatcher + 3 signatures end-to-end.** ADR-0011 §"Dispatch + softmax" + §"Cooldown" + §"Stacking policy" + §"Bias snapshot" all wired. New `signature/` module (mod / triggers / dispatcher / bias_apply / ledger). 3 trigger predicates (body_shield_pressure / long_range_strike / first_time_diagonal_switch) bound to SignatureIds in a build-time table; evaluate_signatures softmax-samples via SeedLayer::SignatureTrigger. 4 new canonical MatchState fields (signature_cooldowns / signature_firing / signature_first_fired_seen / signature_memory_events with #[serde(skip)] for the transient scratch buffer). Encoder VERSION 4→5. Hash rebaselined to `18f1776c…a5d048`. Self-review BLOCKED on initial impl due to vacuous AC tests (iii-c/iii-d pattern recurrence) + signature_memory_events lifecycle footgun + Idle→Cover wildcard bias bug. Fix pass closed 3 P0s + 6 P1s; AC tests rewritten 1:1 to MEMORY criteria via dispatch path; hash stayed unchanged through the fix pass.
- **Now:** T1-2b sub-phase fully closed. Phase T1 critical path: T1-2b-iv → **T1-4 (MatchEvent emission + commentary templates)** → T1-5 (Tauri play_match) → T1-6 (frontend Match route) → T1-7 (content procgen stub) → T1-8 (replay corpus #1) → T1-9 (behavioral assertions).
- **Next:** `T1-4` — `MatchEvent` enum (Goal / Shot / Pass / KickOff / FullTime) + ledger output struct + diagnostic commentary templates rich enough to spot brain-dead behavior from text alone (per ADR-0007 dev-verification §Layer 1). Reconciles the local `MemoryEvent::SignatureFirstFired` stub from T1-2b-iv into the real `MatchEvent` enum. Canonical hash REBASELINE expected (event stream joins canonical state per acceptance criterion). After T1-4 lands, T1-5 wires Tauri play_match → frontend can finally see a match flowing end-to-end. **Pre-T1-4 recommendation: mid-phase Codex audit on the T1-2b accumulated diff** (commits 7ae18f3 through this commit — 8 commits, ~5000 LoC sim + ~3000 LoC tests, 5 canonical-hash rebaselines, encoder VERSION 1→5; coherent "everything that makes a match tick deterministically" batch).

## Blockers

None. T1-2b-iv shipped clean with `scripts/fw verify` green.

## Last green verify

2026-05-15 — `scripts/fw verify` clean post-fix-pass: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `18f1776c…a5d048` + banned-terms + determinism-audit + `fw-content-baker validate`.

## Last canonical hash

`blake3:18f1776c2f77939d32849dc72e05909caf78b93bf6ce50a1222b28f9c6a5d048` (60-tick smoke seed; rebaselined T1-2b-iv per ADR-0012 trigger #1 — MatchState gained signature_cooldowns + signature_firing + signature_first_fired_seen; encoder VERSION 4→5 schema bump; signature_memory_events excluded via `#[serde(skip)]`; prior pin `1db6020c…59c798` was T1-2b-iii-d). The self-review fix pass left the hash unchanged (signature_memory_events was never in encoder; Idle→Neutral path requires sig_definitions which tick_match doesn't provide on smoke seed). Encoder progression: VERSION 1 (T0/T1-2b-i) → 2 (T1-2b-ii) → 3 (T1-2b-iii-a) → 4 (T1-2b-iii-b) → 5 (T1-2b-iv).

## T1-2b sub-phase commit chain

- `<this commit>` feat(sim): T1-2b-iv signature dispatcher + 3 signatures end-to-end (CLOSES T1-2b)
- `2831785` feat(content): T1-3 signature schema stub
- `7ae18f3` feat(sim): T1-2b-iii-d PlayerSeparation + visual playtest gate PASS
- `7840c1f` feat(sim): T1-2b-iii-c BT site bindings + personality bias + utility-scored leaves
- `d471892` feat(sim,core): T1-2b-iii-b utility math primitives + PlayerAttributes baseline
- `<earlier>` T1-2b-iii-a BT runner + per-role skeletons (and iii-b/c/d split MASTER_PLAN commits)
- T1-2b-ii tactic FSM + decision-cadence stagger
- T1-2b-i ball physics integrator

## Recent commits

(See T1-2b sub-phase commit chain above.)

## Next up

`/next` will pick **T1-4** — `MatchEvent` enum + ledger output struct + diagnostic commentary templates. After T1-4, T1-5 wires Tauri play_match and the frontend can finally see a match flow end-to-end.

**Recommendation before /next**: run mid-phase Codex audit on the T1-2b accumulated diff (8 commits since 7ae18f3 / 5000 LoC sim + 3000 LoC tests / 5 hash rebaselines / encoder VERSION 1→5). T1-2b is a coherent thematic batch ("everything that makes a match tick deterministically") — Codex's fresh eyes catch cross-row consistency, ADR drift, and architectural choices the per-row self-review can miss. Run via the existing `gh pr create` pattern (separate Codex CLI session).

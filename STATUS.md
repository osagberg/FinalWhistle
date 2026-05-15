# STATUS — Final Whistle

**Last updated**: 2026-05-15

## Phase

**T1 — First Match** (active; T1-2b-iii sub-phase **CLOSED**: T1-1 + T1-2a + T1-2b-i + T1-2b-ii + T1-2b-iii-a + T1-2b-iii-b + T1-2b-iii-c + **T1-2b-iii-d** all done; next: T1-2b-iv signature dispatcher; final T1-2b row before T1-3 onward)

## Active task

(none — T1-2b-iii-d closed; T1-2b-iii sub-phase fully complete. `/next` picks T1-2b-iv.)

## Phase pointer

- **Just closed:** **T1-2b-iii-d PlayerSeparation + visual playtest gate.** FW v1 `PlayerSeparation.cs` carry-forward as a deterministic Q32 pure function. 231-pair lex-order iteration; cordic sqrt; position-only adjustment so vel magnitude preserved trivially; zero-distance fallback deterministic by slot order; `tick_match` doc-comment enumerates 6 explicit steps. 7 unit tests + 7 proptests cover all 6 acceptance invariants 1:1 (test-coverage gaps caught + closed BEFORE the eyeball gate per the iii-c lesson). 600-tick smoke fixture generated cleanly. Canonical hash rebaselined to `1db6020c…59c798`. **Manual eyeball gate PASSED 2026-05-15** on the dev-board scrub: separation resolves overlaps within a few ticks; the residual back-and-forth movement is downstream of separation (skeletal-BT oscillation from identical mid_range_baseline attrs across all 22 players + no MatchEvent emission yet).
- **Now:** **T1-2b-iii sub-phase fully closed.** Phase T1 critical path: T1-2b-iii-d → **T1-2b-iv (signature dispatcher + first 3 signatures end-to-end)** → T1-3 (signature schema follow-up) → T1-4 (MatchEvent emission) → T1-5 (Tauri play_match command) → T1-6 (frontend Match route).
- **Next:** `T1-2b-iv` — partial implementation of ADR-0011 to validate the signature dispatcher path end-to-end. 3 representative signatures (one defensive, one attacking, one build-up) implement `TriggerPredicate` + `SimBiasSnapshot` + basic `PresentationRecipe`. Cooldown state added to canonical `MatchState`. Per-player `signature_candidates` schema landed at T1-3 (separate row); T1-2b-iv consumes it. `MemoryEvent::SignatureFirstFired` emitted. Softmax dispatch deterministic via `SeedLayer::SignatureTrigger`. Canonical hash REBASELINED (intentional).

## Blockers

None. T1-2b-iii-d shipped clean with `scripts/fw verify` green; 269+ unit tests + 33 proptest integrations.

## Last green verify

2026-05-15 — `scripts/fw verify` clean: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `1db6020c…59c798` + banned-terms + determinism-audit + `fw-content-baker validate`. Cross-OS matrix verification happens on the post-commit CI run.

## Last canonical hash

`blake3:1db6020c7ac3181fac9f73b2e30423708d9fdd55a846e38c8e81c8c7ab59c798` (60-tick smoke seed; rebaselined T1-2b-iii-d per ADR-0012 trigger #1 — PlayerSeparation pass added to `tick_match` at documented step 6; player positions now corrected after integration each tick; prior pin `235f6c5e…181288d` was T1-2b-iii-c baseline). VERSION constant unchanged at 4 (no schema bump; just behavior change). Another rebaseline expected at T1-2b-iv (signature cooldown state joins canonical MatchState).

## Recent commits

- `<this commit>` feat(sim): T1-2b-iii-d PlayerSeparation + manual eyeball PASS (ADR-0012 #1 rebaseline)
- `7840c1f` feat(sim): T1-2b-iii-c BT site bindings + personality bias + utility-scored leaves
- `d471892` feat(sim,core): T1-2b-iii-b utility math primitives + PlayerAttributes baseline
- `7786db0` docs(plan): further-split T1-2b-iii into iii-b/c/d
- `abebdf0` feat(sim): T1-2b-iii-a BT runner + per-role skeletons
- earlier — see CHANGELOG.

## Next up

`/next` will pick **T1-2b-iv** — the final T1-2b row. 3 representative signatures implementing ADR-0011 end-to-end (defensive `BodyShieldPressure` + attacking `LongRangeStrike` + build-up `FirstTimeDiagonalSwitch`). Cooldown state in canonical MatchState; softmax dispatch deterministic via `SeedLayer::SignatureTrigger`; `MemoryEvent::SignatureFirstFired` emitted. After this row closes, T1-2b is done and T1-3 onward (signatures schema, MatchEvent, Tauri play_match, frontend Match route, content procgen stub, replay corpus, behavioral assertions) carries Phase T1 to its acceptance gate.

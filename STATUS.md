# STATUS — Final Whistle

**Last updated**: 2026-05-15

## Phase

**T1 — First Match** (T1-2b-iii sub-phase CLOSED + **T1-3 DONE**; next: T1-2b-iv signature dispatcher; final T1-2b row)

## Active task

(none — T1-3 closed. `/next` picks T1-2b-iv.)

## Phase pointer

- **Just closed:** **T1-3 signature schema stub in fw-content.** Type-system-only row implementing ADR-0011 §"Mechanical shape" + §"Stacking policy" + §"Cooldown" + §"Per-player affinity". New `signature.rs` module ships `SignatureId` (with validated try_new constructor — dotted-pack-id format per Content/RULES.md §2 mod-pack carve-out), `SignatureCandidate::try_new` with affinity range validator, 8-variant `RoleFamily` + 4-variant `BiasCategory` enums (stable u8 discriminants), `SimBiasSnapshot` (5 Q32 multipliers collapsing ADR-0003 §5's 7 surfaces), `CooldownPolicy` (default 600 ticks), `StackingPolicy::Exclusive`, `SignatureTrigger::NoOpStub` (T1-2b-iv expands), `SignaturePresentationRecipe` stub, `SignatureDefinition`. `PlayerTemplate.signature_candidates: Vec<SignatureCandidate>` field with `#[serde(default)]`. `ContentStore` walks `content/sources/signatures/*.ron`; one no-op fixture lives at `content/sources/signatures/no-op-stub.ron`; one player fixture (`sample-am.ron`) references it. Canonical hash **UNCHANGED** at `1db6020c…59c798` (PlayerTemplate isn't in MatchState path). Data-only TDD-exempt; type-design self-review 2 P1s closed in-place (SignatureId validator tightening + SignatureCandidate::try_new).
- **Now:** Phase T1 critical path: T1-3 → **T1-2b-iv (signature dispatcher + first 3 signatures end-to-end)** → T1-4 (MatchEvent emission) → T1-5 (Tauri play_match) → T1-6 (frontend Match route) → T1-7 (content procgen stub) → T1-8 (replay corpus #1) → T1-9 (behavioral assertions).
- **Next:** `T1-2b-iv` — final T1-2b row. Partial ADR-0011 implementation: 3 representative signatures (one defensive, one attacking, one build-up) implementing `TriggerPredicate` + `SimBiasSnapshot` + basic `PresentationRecipe`. Cooldown state in canonical MatchState (`signature_cooldowns: BTreeMap<(PlayerId, SignatureId), Tick>`). Softmax dispatch deterministic via `SeedLayer::SignatureTrigger`. Bias snapshot multiplies into BT utility scoring. `MemoryEvent::SignatureFirstFired` emitted. Canonical hash REBASELINE intentional. After this row, T1-2b is fully done and T1-4 onward carries T1 to its acceptance gate.

## Blockers

None. T1-3 shipped clean with `scripts/fw verify` green.

## Last green verify

2026-05-15 — `scripts/fw verify` clean: fmt + clippy + `cargo test --workspace` (54 unit tests in fw-content + 32 across other crates + 33 proptest integrations) + release-mode canonical-hash regression UNCHANGED at `1db6020c…59c798` + banned-terms + determinism-audit + `fw-content-baker validate`.

## Last canonical hash

`blake3:1db6020c7ac3181fac9f73b2e30423708d9fdd55a846e38c8e81c8c7ab59c798` (60-tick smoke seed; pinned at T1-2b-iii-d; **UNCHANGED through T1-3** since fw-content additions don't touch MatchState bytes). Encoder VERSION 4. Next rebaseline expected at T1-2b-iv (signature cooldown state joins canonical MatchState; ADR-0012 trigger #1 schema bump).

## Recent commits

- `<this commit>` feat(content): T1-3 signature schema stub + per-player affinity field
- `7ae18f3` feat(sim): T1-2b-iii-d PlayerSeparation + visual playtest gate PASS
- `7840c1f` feat(sim): T1-2b-iii-c BT site bindings + personality bias + utility-scored leaves
- `d471892` feat(sim,core): T1-2b-iii-b utility math primitives + PlayerAttributes baseline
- earlier — see CHANGELOG.

## Next up

`/next` will pick **T1-2b-iv** — the final T1-2b row. 3 representative signatures (`BodyShieldPressure` defensive, `LongRangeStrike` attacking, `FirstTimeDiagonalSwitch` build-up) implement `TriggerPredicate` + `SimBiasSnapshot` + basic `PresentationRecipe`. Cooldown state joins canonical MatchState; softmax dispatch via `SeedLayer::SignatureTrigger`; bias snapshot multiplies into BT utility scoring. Canonical hash REBASELINE intentional. T1-2b closes after this row.

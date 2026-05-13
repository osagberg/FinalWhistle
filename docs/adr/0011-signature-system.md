# ADR-0011 — Signature system

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** Claude (Codex full-project audit Lane B + Lane D "carry-forward debt" driver) + Codex (pending pre-T1-3 audit)

---

## Context

**Pillar 5 — Signature identity** ("readable on-pitch moves, not stat lines"; DESIGN_DOC.md §3) is currently vapor at the implementation level. v1 had a working signature framework — `MatchSim/Sim/SignatureRules.cs`, `SignatureCooldownState.cs`, `SignaturePresentationRecipe.cs` + a 24-signature catalogue + `IdentityPacket.SignatureCandidates` per player. v2 has ADR mentions (ADR-0001 layer 1 references `SignatureKind`; ADR-0002 promises 24 signatures supported by the 38-visible-attribute model) but no actual spec.

The Codex full-project audit Lane B finding ("missing ADR — Signature system") + Lane D ("v1 had per-player signature affinity; v2 deferred to T1-3") both point at the same gap. T1-3 lands the signature stub; lock the architectural shape **before** stub code lands.

The choices the spec must resolve:
1. What IS a signature, mechanically? (Trigger predicate + bias snapshot + presentation recipe.)
2. Where do candidates live? (`PlayerTemplate.signature_candidates` per v1's pattern; Tranche 4 schema change.)
3. How do signatures fire? (Dispatcher at on-ball events + tactic-state-change moments; softmax-sampled when multiple candidates trigger.)
4. Cooldown + stacking. (Per-signature cooldown; multi-signature simultaneous firings allowed via stacking policy types.)
5. Counterplay. (Defensive signatures + cancellation predicates.)

## Decision

### Catalogue size: 24 signatures, 3 per role-family × 8 role families

Same shape as v1. **Initial scope is 24, not a cap** — the "24 signature" framing in DESIGN_DOC.md §3 is the launch baseline; mods + content packs can add more via `UnknownSignatureClass`.

Role families match the v1 `RoleFamily` enum (Goalkeeper, CentreBack, FullBack, DefensiveMidfielder, CentralMidfielder, AttackingMidfielder, Winger, Striker). The 24-signature catalogue itself is content-pack data — RON files under `content/sources/signatures/<id>.ron`, loaded by FW-VAL at content-load time.

### Mechanical shape

A signature is the tuple `(SignatureId, TriggerPredicate, SimBiasSnapshot, PresentationRecipe, CooldownPolicy)`:

```rust
// crates/fw-content/src/signature.rs (sketch; lands at T1-3)
pub struct SignatureDefinition {
    pub id: SignatureId,                          // content-pack-qualified
    pub display_name: String,
    pub role_family: RoleFamily,
    pub trigger: SignatureTrigger,                // predicate over MatchState + player + ball
    pub bias_snapshot: SimBiasSnapshot,           // bumps to BT-utility scoring while firing
    pub presentation: SignaturePresentationRecipe, // commentary line bank + camera framing hint
    pub cooldown: CooldownPolicy,
    pub schema_version: u32,
}
```

**Bias snapshot** is the "bump to decision-utility scoring while this signature is in flight" surface. ADR-0003 §5 personality-bias mapping multiplies in player baselines; `SimBiasSnapshot` multiplies *on top* for the duration of the signature window. E.g. a `LongRangeStrike` signature might apply `{ shoot_utility: ×1.4, dribble_utility: ×0.7 }` for the next 3 ticks after the trigger fires.

**Trigger** is a small DSL — pure predicate functions over canonical state. Authored in Rust (NOT in RON) because predicates that touch ball physics + opponent positioning need full type access and can't live as a string parser. The CATALOGUE binds `SignatureId → TriggerFn` in `fw-match-sim`; the per-signature parameters (radius thresholds, distance bounds) live in the RON.

### Per-player affinity

`fw_content::PlayerTemplate.signature_candidates: Vec<SignatureCandidate>` per v1's pattern. **This is the Tranche-4 + T1-3 schema change** — lands when the signature stub does:

```rust
pub struct SignatureCandidate {
    pub signature_id: SignatureId,                // qualified
    pub affinity: Q32,                            // [0, 1]; softmax input when multiple candidates fire
}
```

A player typically has 0–3 candidates. Affinity is a hand-authored gene-style attribute (procedurally derived for generated players in T2-4); it's NOT in `PlayerAttributes` (those are the 55 fields per ADR-0002).

### Dispatch + softmax

Multiple signatures can be eligible at the same tick (e.g. a winger has both `LowCutbackByline` and `InvertedRunCutback` candidates, both predicates fire). The dispatcher:

1. Evaluates trigger predicates for every candidate the player has.
2. For eligible candidates, samples one via **softmax over `affinity × event-class-fit`** with the standard `SeedLayer::SignatureTrigger` per ADR-0009.
3. Sets the player's `signature_firing` field to `Some(SignatureId + start_tick + duration)`.
4. Bias snapshot is applied to the player's utility-scoring on subsequent ticks while `signature_firing.is_some()`.

### Cooldown

Per-signature cooldown via `CooldownPolicy { ticks_since_last_fire: u32 }`. Default policy: 600-tick cooldown per signature per player (10 seconds at 60 Hz). Match-rare signatures (e.g. `OverheadKickFinish`) carry longer cooldowns via the `CooldownPolicy::PerMatchCount(n)` variant — they only fire `n` times per match.

State lives in `MatchState.signature_cooldowns: BTreeMap<(PlayerId, SignatureId), Tick>` — last-fire tick per player+signature pair. BTreeMap-keyed for determinism per `.claude/rules/Sim/RULES.md` §2.

### Stacking policy

Two signatures CAN be in flight simultaneously if they belong to different bias categories. Same-category stacking is forbidden (a player can't have two simultaneous `shoot_utility` bumps active). Categories:

- `Attacking` (shoot / dribble / through-ball)
- `Defensive` (press / cover / block)
- `Build-up` (pass / carry / lay-off)
- `Set-piece` (corner / free-kick / penalty)

`StackingPolicy::Exclusive { category }` enforces single-active-per-category. Concurrent cross-category is allowed because they bias non-overlapping utility-scoring lanes.

### Counterplay

The 24-signature catalogue includes **defensive signatures** with cancellation predicates (e.g. `BodyShieldPressure` cancels nearby `LowCutbackByline` triggers if the defender's positioning predicate fires within 2m). Cancellation = the dispatcher's softmax skips the cancelled signature when re-evaluating.

Cancellation is a real reactive force, not a stat-line counter — a defender with high `marking + positioning` will materially suppress opposing attacking signatures around them.

### Mod compatibility

`UnknownSignatureClass { id: SignatureId, payload: Vec<u8> }` — same pattern as ADR-0005's `UnknownEventClass`. The dispatcher routes unknown signatures through the trigger predicate hash and applies a generic-bias fallback; mods ship their own commentary + readers.

## Consequences

**Positive:**
- Pillar 5 has a concrete mechanical surface, not just a doc-level promise.
- Per-player affinity carries the v1 carry-forward debt (`SignatureCandidates`) into v2 — Codex Lane D finding resolved.
- The dispatcher is a single locus; new signatures land via content-pack RON (parameters) + Rust (predicate function) in tandem.
- Counterplay makes defending readable on the same dimension as attacking — defensive signatures are first-class, not bolt-on.

**Negative:**
- 24 trigger predicates is significant Rust to author + test. Each predicate is small (~50–100 LoC) but the catalogue takes real time. Scheduled as T1-3 + T2-4-adjacent work; full catalogue not required at T1-3 (the stub just lands 1-3 signatures end-to-end).
- The bias-snapshot tilts utility-scoring mid-decision, which means the canonical-hash regression captures the bias propagation. Re-baselining is harder.
- `SignatureCandidate` adds a field to `PlayerTemplate` — content-fixtures must declare them. Sample fixtures + bake-time generation absorb the cost.

**Neutral:**
- Signatures are NOT in `PlayerAttributes` (the 55-field model from ADR-0002). They're an independent surface. Some discussion-of-attributes-as-signature-axes is folded into ADR-0002 §"Choices" item 1 (mental subfield rationale), but the signatures themselves are separate.
- v1's `SignaturePresentationRecipe` carries forward in shape — the commentary line bank + camera framing hint live in content-pack RON.

**Rollback path:**
- If 24 signatures proves too tight, the catalogue extends without ADR amendment (content-pack addition). If 24 proves too generous and we ship < 24 at v1.0, the row done-criteria says "the catalogue file enumerates 24 entries; missing implementations are explicitly `not_yet_implemented: true` flags."
- If softmax dispatch produces unreadable simultaneous-firings, fall back to first-eligible-wins (deterministic by predicate order). Authoring effort to retune predicate ordering.

## Alternatives considered

- **No per-player affinity; signatures fire on attribute predicates only.** Rejected because it makes signatures stat-driven rather than identity-driven — kills Pillar 5's "readable identity" intent. The same set of attributes would always trigger the same set of signatures, regardless of player narrative.
- **Single signature in flight at a time (no stacking).** Simpler dispatcher but loses moments where, e.g., a `BurstingRun` (build-up signature) AND `LongRangeStrike` (attacking signature) fire on the same play. Rejected on richness grounds; stacking policy is the controlled middle ground.
- **All-Rust signature definitions (no content-pack RON).** Rejected on mod-compatibility + tuning-loop grounds — the per-signature parameters (radii, thresholds) need to live in tunable RON.
- **All-RON signature definitions (no Rust predicates).** Rejected — predicates over canonical state can't be expressed as RON without a DSL, and a DSL is more complex than letting Rust own the predicates.

## References

- DESIGN_DOC.md §3 Pillar 5 (the design promise)
- ADR-0001 layer 1 + 2 (where signatures dispatch from + bias the BT)
- ADR-0002 §"Choices" item 1 (the 38-visible-attribute model that triggers consult)
- ADR-0003 §5 (personality bias — composes multiplicatively with signature bias)
- ADR-0005 (memory ledger — `SignatureFirstFired` event class)
- ADR-0009 (RNG seed derivation — `SeedLayer::SignatureTrigger`)
- v1: `MatchSim/Sim/SignatureRules.cs`, `SignatureCooldownState.cs`, `SignaturePresentationRecipe.cs`, `design/signatures.md` (the carry-forward source)
- v1: `MatchSim/Content/IdentityPacket.cs` + `SignatureCandidate` record (the per-player affinity carry-forward)
- Codex full-project audit Lane B "missing ADRs" + Lane D carry-forward debt table
- `docs/MASTER_PLAN.md` T1-3 (the signature stub row)

//! Proptest invariants + deterministic unit tests for the T1-2b-iv signature
//! dispatcher. Covers all 6 acceptance criteria from the MEMORY task spec.
//!
//! ## AC → MEMORY criterion mapping
//!
//! AC-1a/b/c: MEMORY criterion 1 — each of the 3 real signatures fires via
//!            `dispatch_tick` when its trigger predicate is fully satisfied.
//! AC-2:      MEMORY criterion 2 — cooldown enforced: fired signature cannot
//!            re-fire while `signature_cooldowns[slot, id] > tick`.
//! AC-3:      MEMORY criterion 3 — softmax dispatch + stacking: at most one
//!            signature fires per category per slot per tick.
//! AC-4:      MEMORY criterion 4 — active signature bias changes selected intent
//!            (vel_x/vel_y differs between bias-on and bias-off runs).
//! AC-5:      MEMORY criterion 5 — `SignatureFirstFired` emitted exactly once per
//!            (player, signature) pair per match across 150 ticks.
//! AC-6:      MEMORY criterion 6 — canonical hash regression (determinism).

use fw_content::{
    BiasCategory, CooldownPolicy, RoleFamily, SignatureCandidate, SignatureDefinition, SignatureId,
    SignaturePresentationRecipe, SignatureTrigger, SimBiasSnapshot, StackingPolicy,
};
use fw_core::{Q32, Seed, Tick};
use fw_match_sim::{MatchState, dispatch, signature};
use proptest::prelude::*;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal `SignatureDefinition` with `EveryTicks(600)` cooldown.
fn make_def(id: &str, category: BiasCategory, bias: SimBiasSnapshot) -> SignatureDefinition {
    SignatureDefinition {
        schema_version: 1,
        id: SignatureId::try_new(id).expect("valid id"),
        display_name: id.to_string(),
        role_family: RoleFamily::CentralMidfielder,
        trigger: SignatureTrigger::NoOpStub,
        bias_snapshot: bias,
        presentation: SignaturePresentationRecipe {
            commentary_line_bank_id: "placeholder".to_string(),
            camera_framing_hint: "default".to_string(),
            schema_version: 1,
        },
        cooldown: CooldownPolicy::EveryTicks(600),
        stacking: StackingPolicy::Exclusive { category },
    }
}

fn no_op_bias() -> SimBiasSnapshot {
    SimBiasSnapshot::NO_OP
}

/// Bias snapshot with shoot_mul = 2.0 (raw Q32 for 2.0 = 2 × 2^32).
fn amplify_shoot_bias() -> SimBiasSnapshot {
    SimBiasSnapshot {
        shoot_mul: Q32::from_raw(8_589_934_592_i64), // 2.0
        pass_mul: Q32::ONE,
        dribble_mul: Q32::ONE,
        press_mul: Q32::ONE,
        cover_mul: Q32::ONE,
    }
}

fn arb_seed() -> impl Strategy<Value = u64> {
    any::<u64>()
}

/// Set trigger-relevant attributes to `Q32::ONE` for a given player slot.
/// This ensures all three trigger predicates can be satisfied regardless of
/// which trigger predicate fires for this slot's role.
fn set_attrs_above_threshold(state: &mut MatchState, slot: usize) {
    let attrs = state.players[slot].attributes_mut();
    // body-shield-pressure attributes
    attrs.technical.marking = Q32::ONE;
    attrs.physical.strength = Q32::ONE;
    attrs.personality.aggression = Q32::ONE;
    // long-range-strike attributes
    attrs.mental.composure = Q32::ONE;
    attrs.technical.long_shots = Q32::ONE;
    // first-time-diagonal-switch attributes
    attrs.mental.vision = Q32::ONE;
    attrs.technical.passing = Q32::ONE;
}

// ---------------------------------------------------------------------------
// AC-1a: long-range-strike fires via dispatch_tick when predicate satisfied
// ---------------------------------------------------------------------------

/// MEMORY criterion 1: `fwh.core:signature.long-range-strike` fires via
/// `dispatch_tick` (not direct state mutation) when the trigger predicate
/// is satisfied (composure >= 0.45, long_shots >= 0.45, slot is FWD or MID).
///
/// Slot 8 = home FWD (in_team = 8 % 11 = 8; is_attacker = 5..=10 ✓).
/// Attributes set to Q32::ONE (>> 0.45 threshold).
/// Runs up to 150 ticks to give the signature multiple decision opportunities.
#[test]
fn ac1a_long_range_strike_fires_via_dispatch_when_predicate_satisfied() {
    let lrs_id = "fwh.core:signature.long-range-strike";
    let id = SignatureId::try_new(lrs_id).unwrap();

    let mut state = MatchState::initial(Seed::from_u64(42));
    // slot 8 = home FWD; set attrs above threshold.
    set_attrs_above_threshold(&mut state, 8);
    // Register the signature candidate.
    state.players[8]
        .signature_candidates_mut()
        .push(SignatureCandidate::try_new(id.clone(), Q32::ONE).unwrap());

    let mut defs = BTreeMap::new();
    defs.insert(
        lrs_id.to_string(),
        make_def(lrs_id, BiasCategory::Attacking, amplify_shoot_bias()),
    );

    // long-range-strike is Attacking category (index 0 in signature_firing inner array)
    let attacking_idx = BiasCategory::Attacking as usize;
    let mut fired = false;
    for _ in 0..150 {
        state = dispatch::dispatch_tick(state, &defs);
        if state.signature_firing[8][attacking_idx].is_some()
            && state.signature_firing[8][attacking_idx]
                .as_ref()
                .unwrap()
                .id()
                == &id
        {
            fired = true;
            break;
        }
        // Also check if first_fired_seen has it (may have fired and expired).
        if state
            .signature_first_fired_seen
            .contains(&(8u8, id.clone()))
        {
            fired = true;
            break;
        }
        state.tick = state.tick.successor();
    }

    assert!(
        fired,
        "long-range-strike must fire via dispatch_tick when predicate fully satisfied at slot 8; \
         ran 150 ticks with composure=ONE, long_shots=ONE, slot is FWD"
    );
}

// ---------------------------------------------------------------------------
// AC-1b: body-shield-pressure fires via dispatch_tick when predicate satisfied
// ---------------------------------------------------------------------------

/// MEMORY criterion 1: `fwh.core:signature.body-shield-pressure` fires via
/// `dispatch_tick` when the trigger predicate is satisfied (marking >= 0.45,
/// strength >= 0.45, aggression >= 0.45, slot is DEF or MID).
///
/// Slot 1 = home DEF (in_team = 1; is_defender_or_mid = 1..=7 ✓).
#[test]
fn ac1b_body_shield_pressure_fires_via_dispatch_when_predicate_satisfied() {
    let bsp_id = "fwh.core:signature.body-shield-pressure";
    let id = SignatureId::try_new(bsp_id).unwrap();

    let mut state = MatchState::initial(Seed::from_u64(99));
    set_attrs_above_threshold(&mut state, 1);
    state.players[1]
        .signature_candidates_mut()
        .push(SignatureCandidate::try_new(id.clone(), Q32::ONE).unwrap());

    let mut defs = BTreeMap::new();
    defs.insert(
        bsp_id.to_string(),
        make_def(bsp_id, BiasCategory::Defensive, no_op_bias()),
    );

    // body-shield-pressure is Defensive category (index 1)
    let defensive_idx = BiasCategory::Defensive as usize;
    let mut fired = false;
    for _ in 0..150 {
        state = dispatch::dispatch_tick(state, &defs);
        if state.signature_firing[1][defensive_idx].is_some()
            && state.signature_firing[1][defensive_idx]
                .as_ref()
                .unwrap()
                .id()
                == &id
        {
            fired = true;
            break;
        }
        if state
            .signature_first_fired_seen
            .contains(&(1u8, id.clone()))
        {
            fired = true;
            break;
        }
        state.tick = state.tick.successor();
    }

    assert!(
        fired,
        "body-shield-pressure must fire via dispatch_tick when predicate satisfied at slot 1; \
         ran 150 ticks with marking=ONE, strength=ONE, aggression=ONE, slot is DEF"
    );
}

// ---------------------------------------------------------------------------
// AC-1c: first-time-diagonal-switch fires via dispatch_tick when predicate satisfied
// ---------------------------------------------------------------------------

/// MEMORY criterion 1: `fwh.core:signature.first-time-diagonal-switch` fires
/// via `dispatch_tick` when the trigger predicate is satisfied (vision >= 0.45,
/// passing >= 0.45, slot is MID).
///
/// Slot 5 = home MID (in_team = 5; is_midfielder = 5..=7 ✓).
#[test]
fn ac1c_first_time_diagonal_switch_fires_via_dispatch_when_predicate_satisfied() {
    let diag_id = "fwh.core:signature.first-time-diagonal-switch";
    let id = SignatureId::try_new(diag_id).unwrap();

    let mut state = MatchState::initial(Seed::from_u64(777));
    set_attrs_above_threshold(&mut state, 5);
    state.players[5]
        .signature_candidates_mut()
        .push(SignatureCandidate::try_new(id.clone(), Q32::ONE).unwrap());

    let mut defs = BTreeMap::new();
    defs.insert(
        diag_id.to_string(),
        make_def(diag_id, BiasCategory::BuildUp, no_op_bias()),
    );

    // first-time-diagonal-switch is BuildUp category (index 2)
    let buildup_idx = BiasCategory::BuildUp as usize;
    let mut fired = false;
    for _ in 0..150 {
        state = dispatch::dispatch_tick(state, &defs);
        if state.signature_firing[5][buildup_idx].is_some()
            && state.signature_firing[5][buildup_idx]
                .as_ref()
                .unwrap()
                .id()
                == &id
        {
            fired = true;
            break;
        }
        if state
            .signature_first_fired_seen
            .contains(&(5u8, id.clone()))
        {
            fired = true;
            break;
        }
        state.tick = state.tick.successor();
    }

    assert!(
        fired,
        "first-time-diagonal-switch must fire via dispatch_tick when predicate satisfied at slot 5; \
         ran 150 ticks with vision=ONE, passing=ONE, slot is MID"
    );
}

// ---------------------------------------------------------------------------
// AC-2: Cooldown blocks re-firing within the cooldown window
//
// **Codex Tier-2 re-audit round 4 (P1-8 final fix)**: round-3 attempted a
// non-vacuous rewrite by counting `signature_first_fired_seen` entries for
// (slot=8, lrs) across the cooldown window. But `signature_first_fired_seen`
// is a `BTreeSet<(PlayerSlot, SignatureId)>` — keys are structurally unique,
// so the count CAN NEVER exceed 1 regardless of dispatch behavior. A bad
// implementation that re-fires every tick inside the cooldown window would
// still pass that assertion (the second insert is a no-op set membership).
// Codex called this "wrong observable for cooldown enforcement."
//
// Round-4 observes the actual mutation a re-fire would cause:
//   - `signature_cooldowns[&key]` would be OVERWRITTEN to a later tick
//     (re-fire at tick T sets cooldown_end = T + 600). A correct impl that
//     blocks the re-fire leaves the original cooldown_end intact.
//   - `signature_firing[slot][cat]`, when Some, would carry a LATER start_tick
//     than the first firing. A correct impl either leaves the slot None
//     (after the 60-tick active window expires) or carries start_tick == 1.
//
// Both observables fail loudly on a re-fire bug; the prior set-membership
// observable did not.
// ---------------------------------------------------------------------------

#[test]
fn ac2_cooldown_blocks_refiring_within_window() {
    let lrs_id = "fwh.core:signature.long-range-strike";
    let id = SignatureId::try_new(lrs_id).unwrap();

    let seed = Seed::from_u64(42);
    let mut state = MatchState::initial(seed);
    set_attrs_above_threshold(&mut state, 8);
    state.players[8]
        .signature_candidates_mut()
        .push(SignatureCandidate::try_new(id.clone(), Q32::ONE).unwrap());

    // GUARANTEE firing: align decision_slots[8] = 1, then advance to tick=1
    // so should_decide(roster_slot=9, ...) returns true at the first
    // dispatch tick. (roster_slot is 1-indexed, so slot_idx 8 = roster_slot 9.)
    state.decision_slots[8] = 1;
    state.tick = Tick::ZERO.successor();

    let mut defs = BTreeMap::new();
    defs.insert(
        lrs_id.to_string(),
        make_def(lrs_id, BiasCategory::Attacking, amplify_shoot_bias()),
    );

    // Phase 1: first dispatch MUST fire the signature.
    state = dispatch::dispatch_tick(state, &defs);
    let key = (8u8, id.clone());
    assert!(
        state.signature_cooldowns.contains_key(&key),
        "AC-2 setup invariant violated: long_range_strike did not fire on the \
         guaranteed-firing tick. Attrs above threshold + decision_slots[8]=1 + \
         tick=1 should fire on the first dispatch_tick. \
         Check that ForwardState's role-state + the dispatch path are wired."
    );
    let fired_at = state.tick;
    // EveryTicks(600) cooldown → cooldown_end = fired_at + 600.
    let cooldown_end_first = state.signature_cooldowns[&key];
    assert!(
        cooldown_end_first > fired_at,
        "cooldown_end {:?} must be after fired_at {:?}",
        cooldown_end_first.to_raw(),
        fired_at.to_raw()
    );

    // Capture the start_tick of the first firing. LRS is Attacking (cat_idx=0).
    let cat_idx = BiasCategory::Attacking as usize;
    let first_firing = state.signature_firing[8][cat_idx]
        .as_ref()
        .expect("first fire must populate signature_firing[8][Attacking]");
    let first_start_tick = first_firing.start_tick();
    assert_eq!(
        first_start_tick, fired_at,
        "first firing's start_tick must equal the tick it fired on"
    );

    // Phase 2: run 500 more ticks (well within the 600-tick cooldown window).
    // Decision-cadence aligned re-fire opportunities land every 15 ticks
    // (tick % 15 == 1): tick=16, 31, 46, ..., 496 — ~33 attempts. A bad
    // implementation that ignores cooldown would re-fire at one of these.
    // OBSERVABLES that catch the re-fire bug:
    //   1. signature_cooldowns[&key] gets OVERWRITTEN to (re_fire_tick + 600).
    //   2. signature_firing[8][Attacking], if Some, carries a LATER start_tick.
    state.tick = state.tick.successor();
    for _ in 0..500 {
        state = dispatch::dispatch_tick(state, &defs);

        // Observable 1: cooldown_end MUST be unchanged. If the dispatcher
        // re-fired the signature, this value would be (re_fire_tick + 600),
        // strictly greater than the originally captured value.
        let current_cooldown_end = state.signature_cooldowns[&key];
        assert_eq!(
            current_cooldown_end,
            cooldown_end_first,
            "cooldown_end was overwritten at tick {:?} (cooldown_end {:?} -> {:?}); \
             this means the signature re-fired inside its cooldown window. \
             The dispatcher must skip candidates whose cooldown has not expired \
             (signature/dispatcher.rs §'Cooldown check').",
            state.tick.to_raw(),
            cooldown_end_first.to_raw(),
            current_cooldown_end.to_raw()
        );

        // Observable 2: if the firing slot is Some, its start_tick MUST equal
        // first_start_tick. A re-fire would mint a NEW SignatureFiring with
        // a later start_tick. After tick > first_start_tick + 60 the firing
        // window auto-clears to None (dispatch.rs §"Advance firing windows"),
        // and from then until cooldown_end it must stay None.
        if let Some(firing) = state.signature_firing[8][cat_idx].as_ref() {
            assert_eq!(
                firing.start_tick(),
                first_start_tick,
                "signature_firing[8][Attacking] carries start_tick {:?} at tick {:?}, \
                 but expected start_tick {:?} (the first firing). A different \
                 start_tick proves a re-fire happened inside the cooldown window.",
                firing.start_tick().to_raw(),
                state.tick.to_raw(),
                first_start_tick.to_raw()
            );
        }

        state.tick = state.tick.successor();
    }

    // Belt-and-suspenders: final state still shows the original cooldown end.
    assert_eq!(
        state.signature_cooldowns[&key], cooldown_end_first,
        "after 500 cooldown-window ticks, cooldown_end must remain pinned \
         at the originally-set value"
    );
}

// ---------------------------------------------------------------------------
// AC-3: Stacking — same-category signature cannot co-fire on the same tick
//
// **Codex Tier-2 re-audit round 3 (P1-8 non-vacuous rewrite)**: the prior
// AC-3 asserted `firing_count_slot5_buildup <= 1` where
// `firing_count = is_some() as usize`. Since `is_some()` returns 0 or 1,
// the assertion was structurally always-true regardless of stacking semantics.
// Codex called this "impossible structural condition." The rewrite forces
// BOTH same-category signatures to be simultaneously eligible at the same
// tick, then asserts EXACTLY ONE fires (count via first_fired_seen, which
// CAN reach 2 if stacking is broken).
// ---------------------------------------------------------------------------

#[test]
fn ac3_same_category_stacking_allows_at_most_one_signature_per_slot() {
    let sig_a_id = "fwh.core:signature.long-range-strike";
    let sig_b_id = "fwh.core:signature.first-time-diagonal-switch";

    let id_a = SignatureId::try_new(sig_a_id).unwrap();
    let id_b = SignatureId::try_new(sig_b_id).unwrap();

    // Both BuildUp category in THIS test fixture so they compete for the
    // same stacking lane. (Production signature definitions may put them
    // in different categories; we override here.)
    let mut defs = BTreeMap::new();
    defs.insert(
        sig_a_id.to_string(),
        make_def(sig_a_id, BiasCategory::BuildUp, no_op_bias()),
    );
    defs.insert(
        sig_b_id.to_string(),
        make_def(sig_b_id, BiasCategory::BuildUp, no_op_bias()),
    );

    // Slot 5 = home MID: eligible for both lrs (is_attacker includes 5..=10)
    // and first-time-diagonal-switch (is_midfielder = 5..=7).
    let mut state = MatchState::initial(Seed::from_u64(42));
    set_attrs_above_threshold(&mut state, 5);
    *state.players[5].signature_candidates_mut() = vec![
        SignatureCandidate::try_new(id_a.clone(), Q32::ONE).unwrap(),
        SignatureCandidate::try_new(id_b.clone(), Q32::ONE).unwrap(),
    ];

    // GUARANTEE firing on first tick: align decision_slots[5] = 1, advance to tick=1.
    state.decision_slots[5] = 1;
    state.tick = Tick::ZERO.successor();

    // One dispatch tick. Both signatures are eligible (predicates satisfied,
    // BuildUp lane empty, no cooldown). Stacking semantics: exactly one of
    // them fires + writes to signature_firing[5][BuildUp]. The other's trigger
    // either gets blocked at the dispatcher (preferred) or loses softmax to
    // the first (also acceptable). Either way: AT MOST one entry in
    // first_fired_seen for slot 5 after this single tick.
    state = dispatch::dispatch_tick(state, &defs);

    // Count first_fired_seen entries for slot 5 across BOTH candidate IDs.
    let slot5_first_fired: Vec<_> = state
        .signature_first_fired_seen
        .iter()
        .filter(|(slot, _)| *slot == 5u8)
        .collect();

    // **Load-bearing positive assertion**: at least one must have fired
    // (proves the test isn't vacuous). The dispatch path must have selected
    // one of the two via softmax + recorded it in first_fired_seen.
    assert!(
        !slot5_first_fired.is_empty(),
        "AC-3 setup invariant violated: with both same-category sigs eligible \
         + decision_slots[5]=1 + tick=1, dispatch must fire ONE of the two. \
         Neither fired — check trigger predicates + softmax tie-breaking."
    );

    // **Stacking assertion**: EXACTLY one (count == 1, not ≤ 1).
    assert_eq!(
        slot5_first_fired.len(),
        1,
        "slot 5 BuildUp lane stacking violated: expected exactly 1 signature \
         to fire on the same tick when both eligible; got {} (signatures: {:?})",
        slot5_first_fired.len(),
        slot5_first_fired
            .iter()
            .map(|(_, id)| id.as_str())
            .collect::<Vec<_>>()
    );

    // Confirm the fired one is in the BuildUp lane of signature_firing.
    let buildup_idx = BiasCategory::BuildUp as usize;
    assert!(
        state.signature_firing[5][buildup_idx].is_some(),
        "signature_firing[5][BuildUp] must have a Some entry after firing"
    );
}

// ---------------------------------------------------------------------------
// AC-4: Active signature bias changes the selected intent (vel update)
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// AC-4: Active signature bias changes the SELECTED INTENT (vel update)
//
// **Codex Tier-2 re-audit round 3 (P1-8 non-vacuous rewrite)**: prior AC-4
// compared `encode_canonical()` between state_with (signature_firing=Some)
// and state_without (signature_firing=None). The encodings ALWAYS differed
// because the Some/None field divergence ITSELF was encoded — regardless of
// whether the bias actually multiplied into utility. Codex called this
// "proves bytes differ because setup state differs, not because bias changed
// utility."
//
// The rewrite: BOTH states have the SAME signature_firing[8][Attacking] =
// Some(...) so the canonical setup is byte-identical EXCEPT for the bias
// snapshot in the SignatureDefinition map. Then compare players[8].vel_x +
// vel_y after dispatch — the selected intent's vel translation is the
// BIAS-driven behavioral output. If the bias is truly multiplied into
// utility, the softmax tilts toward different action classes (shoot vs
// pass) and vel differs.
// ---------------------------------------------------------------------------

#[test]
fn ac4_active_signature_bias_changes_selected_intent() {
    use fw_match_sim::signature::dispatcher::combine_active_biases;
    use fw_match_sim::subtree_library::select_outfield_intent;
    use fw_match_sim::{ForwardState, PlayerRoleState};
    use rand_chacha::ChaCha8Rng;
    use rand_chacha::rand_core::SeedableRng;

    let lrs_id = "fwh.core:signature.long-range-strike";
    let id = SignatureId::try_new(lrs_id).unwrap();

    // STRONG-shoot bias: shoot_mul=5.0; all others suppressed to 0.1.
    let strong_shoot_bias = SimBiasSnapshot {
        shoot_mul: Q32::from_raw(21_474_836_480_i64), // 5.0
        pass_mul: Q32::from_raw(429_496_730_i64),     // ≈0.1
        dribble_mul: Q32::from_raw(429_496_730_i64),
        press_mul: Q32::from_raw(429_496_730_i64),
        cover_mul: Q32::from_raw(429_496_730_i64),
    };
    let strong_pass_bias = SimBiasSnapshot {
        shoot_mul: Q32::from_raw(429_496_730_i64),
        pass_mul: Q32::from_raw(21_474_836_480_i64),
        dribble_mul: Q32::from_raw(429_496_730_i64),
        press_mul: Q32::from_raw(429_496_730_i64),
        cover_mul: Q32::from_raw(429_496_730_i64),
    };

    // Build state with active signature firing. Both bias maps see the same
    // state; the only difference is the SignatureDefinition's bias_snapshot.
    let mut state = MatchState::initial(Seed::from_u64(12345));
    state.players[8].role_state = PlayerRoleState::Forward(ForwardState::InPossession);
    let attacking_idx = BiasCategory::Attacking as usize;
    state.signature_firing[8][attacking_idx] = Some(signature::SignatureFiring::new(
        id.clone(),
        Tick::ZERO,
        1000,
    ));

    let mut defs_shoot = BTreeMap::new();
    defs_shoot.insert(
        lrs_id.to_string(),
        make_def(lrs_id, BiasCategory::Attacking, strong_shoot_bias),
    );
    let mut defs_pass = BTreeMap::new();
    defs_pass.insert(
        lrs_id.to_string(),
        make_def(lrs_id, BiasCategory::Attacking, strong_pass_bias),
    );

    // Compose biases (this is the dispatch logic's combine path).
    let composite_shoot = combine_active_biases(&state.signature_firing[8], &defs_shoot);
    let composite_pass = combine_active_biases(&state.signature_firing[8], &defs_pass);
    assert!(composite_shoot.is_some(), "shoot bias must compose");
    assert!(composite_pass.is_some(), "pass bias must compose");

    // **Pre-condition check**: the two composites differ (different bias
    // snapshots in the defs maps). If they were equal, the test would be
    // vacuous (no actual bias-driven divergence possible).
    assert_ne!(
        composite_shoot.as_ref().unwrap().shoot_mul,
        composite_pass.as_ref().unwrap().shoot_mul,
        "shoot_mul must differ between the two biases (pre-condition)"
    );

    // Run select_outfield_intent twice — once per bias — with identical
    // input state + identical RNG seed. The ONLY difference is the
    // active_bias parameter. If the bias is actually consumed by the
    // softmax (P0-3 + composite-fold fix), the SELECTED INTENT VARIANT
    // must differ.
    let mut rng_shoot = ChaCha8Rng::seed_from_u64(0);
    let mut rng_pass = ChaCha8Rng::seed_from_u64(0);
    let intent_shoot = select_outfield_intent(
        state.players[8].role_state,
        &state.players[8],
        9, // roster_slot is 1-indexed (slot_idx 8 = roster 9)
        &mut rng_shoot,
        composite_shoot.as_ref(),
    );
    let intent_pass = select_outfield_intent(
        state.players[8].role_state,
        &state.players[8],
        9,
        &mut rng_pass,
        composite_pass.as_ref(),
    );

    // **Behavioral assertion**: compare the PlayerIntent enum DISCRIMINANT.
    // If the bias is consumed, strong shoot_mul drives the softmax toward
    // AttemptShot; strong pass_mul drives it toward
    // AttemptPassShort/PassLong/LayOff. The variant must differ.
    //
    // (Comparing vel after apply_intent doesn't work because apply_intent
    // clamps every variant's vel to MAX_PLAYER_SPEED — different intents
    // can collapse to the same clamped vel. Comparing the intent variant
    // directly is the cleaner behavioral observable for "bias changed the
    // softmax outcome.")
    assert_ne!(
        std::mem::discriminant(&intent_shoot),
        std::mem::discriminant(&intent_pass),
        "active signature bias must change the selected intent variant. \
         shoot-biased picked {intent_shoot:?}; pass-biased picked {intent_pass:?}. \
         Same variant = softmax not actually consuming the bias snapshot."
    );
}

// ---------------------------------------------------------------------------
// AC-5: SignatureFirstFired emitted exactly once per (player, signature) pair
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// AC-5: SignatureFirstFired emitted EXACTLY once per (player, signature) pair
//
// **Codex Tier-2 re-audit round 3 (P1-8 non-vacuous rewrite)**: the prior
// AC-5 asserted `first_fired_count <= 1` — vacuously satisfied by count=0
// (the dominant case if the signature never fired in 150 ticks). Codex
// pinned: "AC-5 allows zero events." The rewrite GUARANTEES firing (same
// pattern as AC-1 + AC-2) then asserts count == 1 (not ≤ 1).
// ---------------------------------------------------------------------------

#[test]
fn ac5_signature_first_fired_emitted_exactly_once_per_player_per_sig() {
    use fw_match_sim::MatchEvent;

    let lrs_id = "fwh.core:signature.long-range-strike";
    let id = SignatureId::try_new(lrs_id).unwrap();

    let seed = Seed::from_u64(42);
    let mut state = MatchState::initial(seed);
    set_attrs_above_threshold(&mut state, 8);
    state.players[8]
        .signature_candidates_mut()
        .push(SignatureCandidate::try_new(id.clone(), Q32::ONE).unwrap());

    // GUARANTEE firing on first dispatch tick.
    state.decision_slots[8] = 1;
    state.tick = Tick::ZERO.successor();

    let mut defs = BTreeMap::new();
    defs.insert(
        lrs_id.to_string(),
        make_def(lrs_id, BiasCategory::Attacking, amplify_shoot_bias()),
    );

    // Run 150 ticks. T1-4a: SignatureFirstFired events now accumulate in
    // state.match_events (persistent canonical stream, not drained per tick).
    for _ in 0..150 {
        state = dispatch::dispatch_tick(state, &defs);
        state.tick = state.tick.successor();
    }

    // Count SignatureFirstFired events for slot 8 / LRS from match_events.
    let first_fired_count = state
        .match_events()
        .iter()
        .filter(|e| {
            matches!(
                e,
                MatchEvent::SignatureFirstFired {
                    player_slot: 8,
                    signature_id,
                    ..
                } if signature_id == &id
            )
        })
        .count();

    // **Load-bearing assertion**: EXACTLY 1, not ≤ 1. The prior "<=" was
    // vacuous on count=0. With guaranteed-firing setup, count MUST be 1.
    assert_eq!(
        first_fired_count, 1,
        "SignatureFirstFired for (slot=8, lrs) must be emitted exactly once \
         across 150 guaranteed-firing ticks. Got {first_fired_count}. \
         0 = setup-failed-to-fire (test is wrong). >1 = exactly-once-semantics \
         violated (production bug)."
    );

    // Cross-check: seen-set MUST contain the entry (firing-emission invariant).
    assert!(
        state
            .signature_first_fired_seen
            .contains(&(8u8, id.clone())),
        "first_fired_seen must contain (8, lrs) since exactly 1 \
         SignatureFirstFired was emitted"
    );
}

// ---------------------------------------------------------------------------
// Removed: vacuousness_check_acN companion tests (Codex Tier-2 re-audit
// round 3). They tested FAKE in-memory bad data (vec! literals of broken
// events; broken cooldown values via direct .insert()), not the real
// dispatch path. Codex's verdict: "test fake local bad data, not the real
// dispatch path." The companions added complexity without proving the AC
// tests would catch dispatch-path bugs.
//
// Instead, AC-2/3/4/5 are now non-vacuous BY CONSTRUCTION via the
// assert-positive-firing-then-assert-invariant pattern: each test
// guarantees a dispatch firing (decision_slots[8]=1 + tick=1 + attrs above
// threshold), asserts the firing happened (positive assertion that fails
// if dispatch is broken), then asserts the invariant on the post-firing
// state (count == N, not ≤ N). If the dispatch path or the invariant is
// broken, the AC test fails directly — no meta-test needed.
// ---------------------------------------------------------------------------

// (vacuousness_check companion tests removed; the assert-positive-then-
//  invariant pattern in AC-2/3/4/5 above is non-vacuous by construction.)

// ---------------------------------------------------------------------------
// T1-26 / AC-4 via dispatch_tick path: opposite-tilted bias maps → divergent
// vel_x/vel_y or different MatchEvent discriminants after one dispatch_tick.
//
// The prior AC-4 test called `select_outfield_intent` DIRECTLY (bypassing
// dispatch_tick). T1-26 exercises the full PRODUCTION composite-bias path:
//
//   initial state
//   → set signature_firing[slot][Attacking] = Some(active firing)
//   → two separate definition maps (strong-shoot vs strong-pass)
//   → call dispatch_tick (NOT select_outfield_intent)
//   → compare vel_x/vel_y of slot 8 between the two runs
//
// If the bias snapshot actually flows from the definition map through
// `combine_active_biases` → `apply_signature_bias` → `select_outfield_intent`
// → `apply_intent`, then the player's velocity will differ between the two runs.
//
// Pre-condition: both states must be byte-identical before dispatch_tick except
// for the definition map — same state, same RNG, different bias only.
// ---------------------------------------------------------------------------

#[test]
fn ac4_via_dispatch_tick_path_opposite_bias_diverges_vel() {
    use fw_match_sim::{ForwardState, PlayerRoleState};

    let lrs_id = "fwh.core:signature.long-range-strike";
    let id = SignatureId::try_new(lrs_id).unwrap();

    // Strong-shoot: shoot_mul = 50.0, all others = 0.05.
    // This extreme tilt ensures the softmax strongly favours AttemptShot.
    // Q32: 50.0 = 50 × 2^32 = 214748364800; 0.05 = 0.05 × 2^32 ≈ 214748365.
    let strong_shoot_bias = SimBiasSnapshot {
        shoot_mul: Q32::from_raw(214_748_364_800_i64), // 50.0
        pass_mul: Q32::from_raw(214_748_365_i64),      // ≈0.05
        dribble_mul: Q32::from_raw(214_748_365_i64),
        press_mul: Q32::from_raw(214_748_365_i64),
        cover_mul: Q32::from_raw(214_748_365_i64),
    };

    // Strong-pass: pass_mul = 50.0, all others = 0.05.
    let strong_pass_bias = SimBiasSnapshot {
        shoot_mul: Q32::from_raw(214_748_365_i64),
        pass_mul: Q32::from_raw(214_748_364_800_i64), // 50.0
        dribble_mul: Q32::from_raw(214_748_365_i64),
        press_mul: Q32::from_raw(214_748_365_i64),
        cover_mul: Q32::from_raw(214_748_365_i64),
    };

    // Build two definition maps — identical except for bias_snapshot.
    let mut defs_shoot = BTreeMap::new();
    defs_shoot.insert(
        lrs_id.to_string(),
        make_def(lrs_id, BiasCategory::Attacking, strong_shoot_bias),
    );
    let mut defs_pass = BTreeMap::new();
    defs_pass.insert(
        lrs_id.to_string(),
        make_def(lrs_id, BiasCategory::Attacking, strong_pass_bias),
    );

    // Build one canonical setup state (identical for both runs).
    // Slot 8 = home FWD (InPossession so on-ball intents fire).
    // Active signature firing ensures combine_active_biases returns Some.
    // We use with_last_touched_by(8) to establish slot 8 as the last ball
    // contact — dispatch_tick's carrier routing will recognise InPossession
    // on slot 8 and route it into on-ball BT candidates.
    let make_initial_state = || {
        let mut state = MatchState::initial(Seed::from_u64(0xBEEF_CAFE))
            .with_last_touched_by(8)
            .with_possession(8);
        state.players[8].role_state = PlayerRoleState::Forward(ForwardState::InPossession);
        // All relevant attrs above threshold for trigger predicates.
        set_attrs_above_threshold(&mut state, 8);
        // Give slot 8 the LRS candidate so the firing resolves to a real def.
        *state.players[8].signature_candidates_mut() =
            vec![SignatureCandidate::try_new(id.clone(), Q32::ONE).unwrap()];
        // Activate the Attacking lane for slot 8 — this is what makes
        // combine_active_biases return the bias snapshot.
        let attacking_idx = BiasCategory::Attacking as usize;
        state.signature_firing[8][attacking_idx] = Some(signature::SignatureFiring::new(
            id.clone(),
            fw_core::Tick::ZERO,
            5400, // active for the whole test
        ));
        // Force decision to fire this tick.
        state.decision_slots[8] = 1;
        state.tick = fw_core::Tick::ZERO.successor();
        state
    };

    let state_shoot_before = make_initial_state();
    let state_pass_before = make_initial_state();

    // Pre-condition: identical canonical bytes before dispatch.
    assert_eq!(
        state_shoot_before.encode_canonical(),
        state_pass_before.encode_canonical(),
        "T1-26 pre-condition: both states must be byte-identical before dispatch_tick"
    );

    // Run dispatch_tick once with each definition map.
    let state_shoot_after = dispatch::dispatch_tick(state_shoot_before, &defs_shoot);
    let state_pass_after = dispatch::dispatch_tick(state_pass_before, &defs_pass);

    // Behavioral assertion: vel_x AND vel_y for slot 8 must differ.
    //
    // With 50× shoot bias, the player fires AttemptShot (ball launched toward
    // goal, player vel set toward shot target). With 50× pass bias, the player
    // fires AttemptPassShort/Long (ball sent to a teammate, vel set toward
    // pass target — a different direction). The 50× tilt is extreme enough
    // that the softmax is deterministic on a single tick; divergent vel is
    // the genuine behavior signal that bias→intent selection is wired.
    //
    // The pre-condition assert above confirms input state is byte-identical,
    // so any vel divergence is caused ONLY by the differing bias snapshots.
    // We assert vel directly (not encode_canonical) because encode_canonical
    // would also diverge for encoding-incidental reasons (cooldown timestamps,
    // firing state) that are NOT evidence of bias→intent flow.
    let vel_shoot = (
        state_shoot_after.players[8].vel_x,
        state_shoot_after.players[8].vel_y,
    );
    let vel_pass = (
        state_pass_after.players[8].vel_x,
        state_pass_after.players[8].vel_y,
    );
    assert_ne!(
        vel_shoot, vel_pass,
        "T1-26: dispatch_tick with strong-shoot vs strong-pass bias must produce \
         divergent vel for slot 8. vel_shoot={vel_shoot:?}, vel_pass={vel_pass:?}. \
         Same vel = the bias snapshot is NOT flowing through the production \
         composite-bias path (combine_active_biases → apply_signature_bias → \
         select_outfield_intent → apply_intent)."
    );
}

// ---------------------------------------------------------------------------
// AC-6: Determinism — same seed + definitions → identical canonical output
// ---------------------------------------------------------------------------

// MEMORY criterion 6: two runs from the same seed and same definitions must
// produce byte-identical canonical output after 30 ticks.
//
// Uses the real trigger-eligible setup (attrs set above threshold, real
// candidate IDs, real definitions map) so the signature path is exercised
// — not just an empty-defs run.
proptest! {
    #[test]
    fn ac6_signature_dispatch_is_deterministic(seed_val in arb_seed()) {
        let lrs_id = "fwh.core:signature.long-range-strike";
        let diag_id = "fwh.core:signature.first-time-diagonal-switch";
        let bsp_id = "fwh.core:signature.body-shield-pressure";

        let id_lrs = SignatureId::try_new(lrs_id).unwrap();
        let id_diag = SignatureId::try_new(diag_id).unwrap();
        let id_bsp = SignatureId::try_new(bsp_id).unwrap();

        let mut defs = BTreeMap::new();
        defs.insert(lrs_id.to_string(), make_def(lrs_id, BiasCategory::Attacking, amplify_shoot_bias()));
        defs.insert(diag_id.to_string(), make_def(diag_id, BiasCategory::BuildUp, no_op_bias()));
        defs.insert(bsp_id.to_string(), make_def(bsp_id, BiasCategory::Defensive, no_op_bias()));

        let make_state = || {
            let seed = Seed::from_u64(seed_val);
            let mut state = MatchState::initial(seed);
            // Slot 8 (FWD): long-range-strike eligible.
            set_attrs_above_threshold(&mut state, 8);
            *state.players[8].signature_candidates_mut() = vec![
                SignatureCandidate::try_new(id_lrs.clone(), Q32::ONE).unwrap(),
            ];
            // Slot 5 (MID): first-time-diagonal-switch eligible.
            set_attrs_above_threshold(&mut state, 5);
            *state.players[5].signature_candidates_mut() = vec![
                SignatureCandidate::try_new(id_diag.clone(), Q32::ONE).unwrap(),
            ];
            // Slot 1 (DEF): body-shield-pressure eligible.
            set_attrs_above_threshold(&mut state, 1);
            *state.players[1].signature_candidates_mut() = vec![
                SignatureCandidate::try_new(id_bsp.clone(), Q32::ONE).unwrap(),
            ];
            state
        };

        let run_30 = |defs: &BTreeMap<String, SignatureDefinition>| {
            let mut state = make_state();
            for _ in 0..30 {
                state = dispatch::dispatch_tick(state, defs);
                state.tick = state.tick.successor();
            }
            state.encode_canonical()
        };

        let a = run_30(&defs);
        let b = run_30(&defs);

        prop_assert_eq!(a, b, "dispatch must be deterministic for seed 0x{:016x}", seed_val);
    }
}

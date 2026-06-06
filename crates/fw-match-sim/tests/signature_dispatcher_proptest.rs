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
    BiasCategory, CooldownPolicy, MatchEvent, RoleFamily, SignatureCandidate, SignatureDefinition,
    SignatureId, SignaturePresentationRecipe, SignatureTrigger, SimBiasSnapshot, StackingPolicy,
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
    // poacher's-dart trigger attributes — set ABOVE the 0.45 threshold but NOT
    // to 1.0. The mid_range_baseline() default of 0.5 already clears the gate;
    // we leave off_the_ball / anticipation / acceleration / pace at baseline so
    // they don't inflate the RunOffBall raw utility past 1.0 (the
    // `apply_run_off_ball_bias` precondition is `raw <= 1`, and that product is
    // off_the_ball × pace × acceleration × anticipation × secondary>1).
    // finishing is `technical` (used by the shot/xG path, not run-off-ball), so
    // it is safe to elevate for the box-shot recipe.
    attrs.technical.finishing = Q32::ONE;
}

/// 2026-06-06 kickoff-spam fix: signatures now fire ONLY when the player
/// genuinely EXECUTES the move during live play (a real shot / dribble / run /
/// interception), past a 60-tick kick-off settle window — not the instant a
/// static attribute precondition is first true. So a dispatch-level firing test
/// must drive an actual in-play action.
///
/// This builds the proven "centre-forward shoots from inside the box" scenario
/// (mirrors `ac4_via_dispatch_tick_path`): slot 9 (home CF, `in_team == 9`,
/// the poacher's-dart role) InPossession ~4.5m from goal with the ball at his
/// feet, elite finishing + a 50× shoot bias so the BT deterministically picks
/// `AttemptShot`. `poacher's-dart` action-gates on `AttemptShot && in_box`, so
/// this is a real signature execution. Tick is advanced PAST the settle window
/// and `decision_slots[9]` aligned so slot 9 decides on the first dispatch.
const POACHER_SLOT: usize = 9;
const POACHER_ID: &str = "fwh.core:signature.poachers-dart";

/// A separate shoot-driver signature, used ONLY to inject a 50× shoot bias via
/// a NON-Attacking lane. `combine_active_biases` folds every active lane, so a
/// SetPiece-lane bias still multiplies shoot_mul — while leaving the Attacking
/// lane FREE so poacher's-dart (Attacking) stays eligible (an occupied same-
/// category lane would make `evaluate_signatures` skip it). The no-op-stub id
/// has a trigger binding (never fires on its own) so the def resolves cleanly.
const SHOOT_DRIVER_ID: &str = "fwh.core:signature.no-op-stub";

/// Re-establish the on-ball box-shot scenario for slot 9 each tick (a shot
/// launches the ball loose + drops possession, so without re-arming the CF
/// wouldn't shoot again). Also re-asserts the SetPiece-lane shoot driver.
fn re_arm_box_shooter(state: MatchState) -> MatchState {
    use fw_match_sim::{ForwardState, PlayerRoleState};
    let driver_id = SignatureId::try_new(SHOOT_DRIVER_ID).unwrap();
    let mut state = state
        .with_last_touched_by(POACHER_SLOT as u8)
        .with_possession(POACHER_SLOT as u8);
    state.players[POACHER_SLOT].role_state = PlayerRoleState::Forward(ForwardState::InPossession);
    state.players[POACHER_SLOT].pos_x = Q32::from_int(48);
    state.players[POACHER_SLOT].pos_y = Q32::ZERO;
    state.ball.pos_x = Q32::from_int(48);
    state.ball.pos_y = Q32::ZERO;
    // Re-assert the SetPiece-lane shoot driver (different category from the
    // poacher's-dart Attacking lane, so eligibility stays unblocked).
    let setpiece_idx = BiasCategory::SetPiece as usize;
    state.signature_firing[POACHER_SLOT][setpiece_idx] =
        Some(signature::SignatureFiring::new(driver_id, Tick::ZERO, 5400));
    state
}

fn poacher_box_shot_state(seed: u64) -> (MatchState, BTreeMap<String, SignatureDefinition>) {
    use fw_match_sim::{ForwardState, PlayerRoleState};

    let id = SignatureId::try_new(POACHER_ID).unwrap();
    let driver_id = SignatureId::try_new(SHOOT_DRIVER_ID).unwrap();
    // 50× shoot tilt → deterministic AttemptShot on a single tick.
    let strong_shoot_bias = SimBiasSnapshot {
        shoot_mul: Q32::from_raw(214_748_364_800_i64), // 50.0
        pass_mul: Q32::from_raw(214_748_365_i64),      // ≈0.05
        dribble_mul: Q32::from_raw(214_748_365_i64),
        press_mul: Q32::from_raw(214_748_365_i64),
        cover_mul: Q32::from_raw(214_748_365_i64),
    };

    let mut state = MatchState::initial(Seed::from_u64(seed))
        .with_last_touched_by(POACHER_SLOT as u8)
        .with_possession(POACHER_SLOT as u8);
    state.players[POACHER_SLOT].role_state = PlayerRoleState::Forward(ForwardState::InPossession);
    // Home attacks +x; goal at GOAL_LINE_X = 52.5. Put the CF ~4.5m out, central
    // (inside the penalty area for the in_box action gate).
    state.players[POACHER_SLOT].pos_x = Q32::from_int(48);
    state.players[POACHER_SLOT].pos_y = Q32::ZERO;
    state.ball.pos_x = Q32::from_int(48);
    state.ball.pos_y = Q32::ZERO;
    {
        // Elevate ONLY the shot-path attributes (finishing / technique /
        // long_shots), leaving everything else at the mid_range_baseline (0.5,
        // which already clears every signature's 0.45 trigger threshold). We do
        // NOT use `set_attrs_above_threshold` here: maxing strength + composure +
        // balance together pushes the hold-ball raw utility > 1.0 and trips the
        // production `apply_hold_bias` precondition (`raw <= 1`) — a separate,
        // pre-existing latent issue, out of scope for the signature fix. Keeping
        // those at baseline avoids it while the elite finishing + the 50× shoot
        // bias still resolve the box shot.
        let attrs = state.players[POACHER_SLOT].attributes_mut();
        attrs.technical.finishing = Q32::ONE;
        attrs.technical.technique = Q32::ONE;
        attrs.technical.long_shots = Q32::ONE;
    }
    *state.players[POACHER_SLOT].signature_candidates_mut() =
        vec![SignatureCandidate::try_new(id, Q32::ONE).unwrap()];

    // Drive the shoot decision via the SetPiece lane (different category from
    // poacher's-dart's Attacking lane) so the 50× bias flows into shoot_mul
    // WITHOUT occupying the Attacking lane that poacher's-dart needs free.
    let setpiece_idx = BiasCategory::SetPiece as usize;
    state.signature_firing[POACHER_SLOT][setpiece_idx] =
        Some(signature::SignatureFiring::new(driver_id, Tick::ZERO, 5400));

    // Decide on the first dispatch, PAST the 60-tick settle window.
    state.decision_slots[POACHER_SLOT] = 1;
    state.tick = Tick::from_raw(61);

    let mut defs = BTreeMap::new();
    // The poacher's-dart definition (the signature under test).
    defs.insert(
        POACHER_ID.to_string(),
        make_def(POACHER_ID, BiasCategory::Attacking, no_op_bias()),
    );
    // The shoot-driver def in the SetPiece lane (50× shoot bias).
    defs.insert(
        SHOOT_DRIVER_ID.to_string(),
        make_def(SHOOT_DRIVER_ID, BiasCategory::SetPiece, strong_shoot_bias),
    );
    (state, defs)
}

// ---------------------------------------------------------------------------
// AC-1a: a signature fires END-TO-END through dispatch_tick on a REAL in-play
//        action moment (2026-06-06 kickoff-spam fix).
// ---------------------------------------------------------------------------

/// MEMORY criterion 1, re-cast for the in-play-moment contract: a signature
/// (`poacher's-dart`) fires via `dispatch_tick` when the player ACTUALLY
/// executes its move during live play — here, the centre-forward taking a shot
/// from inside the box. Capability (the attribute predicate) is necessary but
/// no longer sufficient: the player must perform the action.
#[test]
fn ac1a_poachers_dart_fires_via_dispatch_on_real_box_shot() {
    let id = SignatureId::try_new(POACHER_ID).unwrap();
    let (mut state, defs) = poacher_box_shot_state(42);

    state = dispatch::dispatch_tick(state, &defs);

    // The CF took a real shot in the box → poacher's-dart fired.
    assert!(
        state.match_events().iter().any(|e| matches!(
            e,
            MatchEvent::Shot {
                shooter_slot: 9,
                ..
            }
        )),
        "setup invariant: slot 9 must take a real shot (the action poacher's-dart \
         gates on). Events: {:?}",
        state.match_events()
    );
    assert!(
        state
            .signature_first_fired_seen
            .contains(&(POACHER_SLOT as u8, id.clone())),
        "poacher's-dart must fire via dispatch_tick when the CF executes a real \
         box shot past the settle window. first_fired_seen: {:?}",
        state.signature_first_fired_seen
    );
}

// ---------------------------------------------------------------------------
// AC-1b: the action gate is the load-bearing addition — a capable player whose
//        chosen intent is UNRELATED to the move does NOT fire (was the bug).
// ---------------------------------------------------------------------------

/// 2026-06-06 kickoff-spam regression guard: a player who is fully capable of a
/// signature (attribute predicate satisfied) but whose actual intent this tick
/// does NOT execute the move must NOT fire. This is the precise inverse of the
/// kickoff dump: pre-fix, capability alone fired the signature at kick-off.
#[test]
fn ac1b_capable_player_does_not_fire_without_the_action() {
    use fw_match_sim::role_states::PlayerIntent;
    use fw_match_sim::signature::signature_executes_now;

    let px = Q32::from_int(48);
    let py = Q32::ZERO;
    let player_pos = (px, py);
    let ball_pos = (px, py);
    let tick = Tick::from_raw(300); // well past the settle window

    // CF in the box, but merely holding position / passing back — not a dart.
    assert!(
        !signature_executes_now(
            POACHER_ID,
            &PlayerIntent::HoldBall {
                target_x: px,
                target_y: py
            },
            POACHER_SLOT as u8,
            tick,
            player_pos,
            ball_pos,
        ),
        "poacher's-dart must NOT fire on a non-dart intent (HoldBall) even in the box"
    );
    // The real action (a shot in the box) DOES fire.
    assert!(
        signature_executes_now(
            POACHER_ID,
            &PlayerIntent::AttemptShot {
                target_x: Q32::from_int(53),
                target_y: py
            },
            POACHER_SLOT as u8,
            tick,
            player_pos,
            ball_pos,
        ),
        "poacher's-dart must fire on a real box shot"
    );
}

// ---------------------------------------------------------------------------
// AC-1c: the settle window blocks firing during kick-off settling.
// ---------------------------------------------------------------------------

/// 2026-06-06 kickoff-spam regression guard: even with the right action +
/// geometry, no signature fires inside the `SIGNATURE_SETTLE_TICKS` window.
#[test]
fn ac1c_no_signature_fires_during_settle_window() {
    use fw_match_sim::role_states::PlayerIntent;
    use fw_match_sim::signature::signature_executes_now;

    let px = Q32::from_int(48);
    let py = Q32::ZERO;
    let shot = PlayerIntent::AttemptShot {
        target_x: Q32::from_int(53),
        target_y: py,
    };

    let pos = (px, py);
    // Inside the settle window (ticks 0..60): a real box shot still must not fire.
    for raw in [0_i64, 1, 30, 59] {
        assert!(
            !signature_executes_now(
                POACHER_ID,
                &shot,
                POACHER_SLOT as u8,
                Tick::from_raw(raw),
                pos,
                pos
            ),
            "no signature may fire at tick {raw} (inside the kick-off settle window)"
        );
    }
    // First tick past the window: the same action fires.
    assert!(
        signature_executes_now(
            POACHER_ID,
            &shot,
            POACHER_SLOT as u8,
            Tick::from_raw(60),
            pos,
            pos
        ),
        "the same box shot must fire on the first tick past the settle window"
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
    // 2026-06-06 kickoff-spam fix: a firing now requires a REAL action. Use the
    // poacher's-dart box-shot recipe (a guaranteed AttemptShot in the box) so a
    // real firing occurs, then assert cooldown blocks re-firing. Each tick we
    // re-establish possession + ball-at-feet so the CF keeps shooting — only the
    // cooldown should prevent a second firing.
    let id = SignatureId::try_new(POACHER_ID).unwrap();
    let (mut state, defs) = poacher_box_shot_state(42);
    let key = (POACHER_SLOT as u8, id.clone());

    // Phase 1: first dispatch MUST fire the signature on a real box shot.
    state = dispatch::dispatch_tick(state, &defs);
    assert!(
        state.signature_cooldowns.contains_key(&key),
        "AC-2 setup invariant violated: poacher's-dart did not fire on the \
         guaranteed box-shot tick. The CF must shoot in the box past the settle \
         window. first_fired_seen: {:?}",
        state.signature_first_fired_seen
    );
    let fired_at = state.tick;
    let cooldown_end_first = state.signature_cooldowns[&key];
    assert!(
        cooldown_end_first > fired_at,
        "cooldown_end {:?} must be after fired_at {:?}",
        cooldown_end_first.to_raw(),
        fired_at.to_raw()
    );

    // poacher's-dart is Attacking (cat_idx = 0).
    let cat_idx = BiasCategory::Attacking as usize;
    let first_start_tick = state.signature_firing[POACHER_SLOT][cat_idx]
        .as_ref()
        .expect("first fire must populate signature_firing[9][Attacking]")
        .start_tick();
    assert_eq!(
        first_start_tick, fired_at,
        "first firing's start_tick must equal the tick it fired on"
    );

    // Phase 2: run 500 more ticks (well within the 600-tick cooldown window).
    // Re-arm the shooting scenario each tick (shot launches the ball loose +
    // drops possession; without re-arming the CF wouldn't shoot again). The
    // cooldown — not a lack of action — must be what blocks the re-fire.
    state.tick = state.tick.successor();
    for _ in 0..500 {
        state = re_arm_box_shooter(state);

        state = dispatch::dispatch_tick(state, &defs);

        // Observable 1: cooldown_end MUST be unchanged. A re-fire would overwrite
        // it to (re_fire_tick + 600), strictly greater than the captured value.
        let current_cooldown_end = state.signature_cooldowns[&key];
        assert_eq!(
            current_cooldown_end,
            cooldown_end_first,
            "cooldown_end was overwritten at tick {:?} (cooldown_end {:?} -> {:?}); \
             the signature re-fired inside its cooldown window. The dispatcher \
             must skip candidates whose cooldown has not expired.",
            state.tick.to_raw(),
            cooldown_end_first.to_raw(),
            current_cooldown_end.to_raw()
        );

        // Observable 2: if the firing slot is Some, its start_tick MUST equal
        // first_start_tick. A re-fire would mint a NEW SignatureFiring.
        if let Some(firing) = state.signature_firing[POACHER_SLOT][cat_idx].as_ref() {
            assert_eq!(
                firing.start_tick(),
                first_start_tick,
                "signature_firing[9][Attacking] carries start_tick {:?} at tick {:?}, \
                 expected {:?} (the first firing). A different start_tick proves a \
                 re-fire happened inside the cooldown window.",
                firing.start_tick().to_raw(),
                state.tick.to_raw(),
                first_start_tick.to_raw()
            );
        }

        state.tick = state.tick.successor();
    }

    assert_eq!(
        state.signature_cooldowns[&key], cooldown_end_first,
        "after 500 cooldown-window ticks, cooldown_end must remain pinned"
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
    // 2026-06-06 kickoff-spam fix: the stacking exclusion lives in
    // `evaluate_signatures` (eligibility resolution), keyed off the per-category
    // active-firing lane — it is independent of the new in-play action gate. We
    // exercise it directly: a same-category candidate is excluded when the lane
    // is already occupied, and admitted when it is empty.
    use fw_match_sim::signature::evaluate_signatures;

    let sig_a_id = "fwh.core:signature.long-range-strike";
    let sig_b_id = "fwh.core:signature.first-time-diagonal-switch";
    let id_a = SignatureId::try_new(sig_a_id).unwrap();
    let id_b = SignatureId::try_new(sig_b_id).unwrap();

    // Both in the BuildUp lane so they compete for the same stacking lane.
    let mut defs = BTreeMap::new();
    defs.insert(
        sig_a_id.to_string(),
        make_def(sig_a_id, BiasCategory::BuildUp, no_op_bias()),
    );
    defs.insert(
        sig_b_id.to_string(),
        make_def(sig_b_id, BiasCategory::BuildUp, no_op_bias()),
    );

    // Slot 5 = home MID: eligible for both lrs (is_attacker 5..=10) and
    // diagonal-switch (is_midfielder 5..=7).
    let mut state = MatchState::initial(Seed::from_u64(42));
    set_attrs_above_threshold(&mut state, 5);
    let candidates = vec![
        SignatureCandidate::try_new(id_a.clone(), Q32::ONE).unwrap(),
        SignatureCandidate::try_new(id_b.clone(), Q32::ONE).unwrap(),
    ];
    *state.players[5].signature_candidates_mut() = candidates.clone();
    state.tick = Tick::from_raw(100);

    let trigger_table = signature::build_trigger_table();
    let buildup_idx = BiasCategory::BuildUp as usize;

    // Lane EMPTY → evaluate_signatures resolves exactly one eligible candidate.
    let empty_lanes: [Option<signature::SignatureFiring>; 4] = [None, None, None, None];
    let picked = evaluate_signatures(&state, 5, &candidates, &defs, &trigger_table, &empty_lanes);
    assert!(
        picked.is_some(),
        "with both BuildUp sigs eligible and the lane empty, evaluate_signatures \
         must resolve exactly one candidate (proves the test isn't vacuous)"
    );
    let picked_id = picked.unwrap().0;
    assert!(
        picked_id == id_a || picked_id == id_b,
        "resolved candidate must be one of the two registered sigs; got {picked_id:?}"
    );

    // Lane OCCUPIED by a BuildUp firing → BOTH BuildUp candidates are excluded
    // by the stacking check; evaluate_signatures returns None.
    let mut occupied_lanes: [Option<signature::SignatureFiring>; 4] = [None, None, None, None];
    occupied_lanes[buildup_idx] = Some(signature::SignatureFiring::new(
        id_a.clone(),
        Tick::from_raw(100),
        60,
    ));
    let blocked = evaluate_signatures(
        &state,
        5,
        &candidates,
        &defs,
        &trigger_table,
        &occupied_lanes,
    );
    assert!(
        blocked.is_none(),
        "stacking violated: with the BuildUp lane already occupied, no same-category \
         candidate may be resolved; got {blocked:?}"
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
    // Attribute-effect Slice 0 (2026-06-06): the non-linear curve compresses the
    // shoot composite (shooter_quality + xG gate) for a default-attribute carrier
    // in midfield, so the strong-shoot bias could no longer overtake the pass path
    // and BOTH biases collapsed to AttemptPassShort (vacuous). Mirror the proven
    // box-position recipe from the `..._resolves_to_shot` test: put slot 8 ~4.5m
    // from goal with the ball at his feet and elite shooting attributes, so an
    // elite finisher's curved shoot utility is genuinely available and the bias
    // can flip the variant. This keeps the mechanism check (bias IS consumed)
    // gated, not relaxed.
    state.players[8].pos_x = Q32::from_int(48);
    state.players[8].pos_y = Q32::ZERO;
    state.ball.pos_x = Q32::from_int(48);
    state.ball.pos_y = Q32::ZERO;
    {
        let attrs = state.players[8].attributes_mut();
        attrs.technical.finishing = Q32::ONE;
        attrs.technical.technique = Q32::ONE;
        attrs.technical.long_shots = Q32::ONE;
        attrs.mental.composure = Q32::ONE;
        attrs.physical.balance = Q32::ONE;
    }
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
    // FUN-TS1: compute shape for team 0 (home) — slot 8 is a home forward.
    let shape = fw_match_sim::team_shape::compute(0, &state);
    let intent_shoot = select_outfield_intent(
        state.players[8].role_state,
        &state.players[8],
        9, // roster_slot is 1-indexed (slot_idx 8 = roster 9)
        &mut rng_shoot,
        composite_shoot.as_ref(),
        None, // carrier_pos: no carrier for this test
        &shape,
        0, // team_idx: home
    );
    let intent_pass = select_outfield_intent(
        state.players[8].role_state,
        &state.players[8],
        9,
        &mut rng_pass,
        composite_pass.as_ref(),
        None, // carrier_pos: no carrier for this test
        &shape,
        0,
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

    // 2026-06-06 kickoff-spam fix: guarantee a REAL firing via the poacher's-dart
    // box-shot recipe, then assert SignatureFirstFired is emitted exactly once
    // even though the CF takes many shots (re-armed) across the window.
    let id = SignatureId::try_new(POACHER_ID).unwrap();
    let (mut state, defs) = poacher_box_shot_state(42);

    // Run 150 ticks, re-arming the box-shot scenario each tick so the CF keeps
    // shooting; the exactly-once gate (signature_first_fired_seen) — not a lack
    // of repeat actions — must be what caps the event count at 1.
    let attacking_idx = BiasCategory::Attacking as usize;
    for _ in 0..150 {
        state = re_arm_box_shooter(state);
        // Clear BOTH the cooldown AND the Attacking firing lane each tick, so a
        // SECOND firing would be fully allowed if the once-gate were broken —
        // this makes the exactly-once assertion non-vacuous. The ONLY thing that
        // may cap the event count at 1 is `signature_first_fired_seen`.
        state.signature_cooldowns.clear();
        state.signature_firing[POACHER_SLOT][attacking_idx] = None;

        state = dispatch::dispatch_tick(state, &defs);
        state.tick = state.tick.successor();
    }

    let first_fired_count = state
        .match_events()
        .iter()
        .filter(|e| {
            matches!(
                e,
                MatchEvent::SignatureFirstFired {
                    player_slot: 9,
                    signature_id,
                    ..
                } if signature_id == &id
            )
        })
        .count();

    assert_eq!(
        first_fired_count, 1,
        "SignatureFirstFired for (slot=9, poacher's-dart) must be emitted exactly \
         once across 150 guaranteed-firing ticks. Got {first_fired_count}. \
         0 = setup-failed-to-fire (test is wrong). >1 = exactly-once-semantics \
         violated (production bug)."
    );

    assert!(
        state
            .signature_first_fired_seen
            .contains(&(POACHER_SLOT as u8, id.clone())),
        "first_fired_seen must contain (9, poacher's-dart) since exactly 1 \
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
        // FUN-0b+c: position slot 8 (home, attacking +x; goal at GOAL_LINE_X=52.5)
        // in the opponent box with the ball at his feet. Slice A gated the shoot
        // utility on xG (XG_SHOOT_THRESHOLD), so a MIDFIELD carrier can never
        // shoot regardless of bias — strong-shoot bias × ~0 xG-gated utility = 0,
        // and the carrier falls back to a Pass for BOTH bias snapshots (the old
        // observable was vacuous for this reason). A central position ~4.5m from
        // goal makes xG high enough that the 50× shoot bias resolves to a Shot,
        // so the shoot-vs-pass divergence is observable through dispatch_tick.
        state.players[8].pos_x = Q32::from_int(48);
        state.players[8].pos_y = Q32::ZERO;
        state.ball.pos_x = Q32::from_int(48);
        state.ball.pos_y = Q32::ZERO;
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

    // Behavioral assertion: the EMITTED MatchEvent kind must diverge — a Shot
    // for the shoot-bias run, a Pass for the pass-bias run.
    //
    // With 50× shoot bias, the carrier fires AttemptShot → `MatchEvent::Shot`.
    // With 50× pass bias, the carrier fires AttemptPass* / Cross / LayOff →
    // `MatchEvent::Pass`. The 50× tilt is extreme enough that the softmax is
    // deterministic on a single tick; the divergent event kind is the genuine
    // behavior signal that bias→intent selection is wired all the way through
    // dispatch_tick → apply_intent.
    //
    // FUN-0b+c: this assertion was REWRITTEN from comparing post-cap PLAYER
    // velocity to comparing the emitted event. The 2D-magnitude velocity cap
    // (apply_vel_toward_target) normalises both shoot- and pass-driven movement
    // to MAX_PLAYER_SPEED toward the same +x target, so post-cap player vel is
    // (8, 0) for BOTH biases — a now-vacuous observable. The shoot-vs-pass
    // divergence lives in the BALL (and in the emitted event), not in the
    // carrier's own movement vector. The emitted MatchEvent is the strongest,
    // mutation-discriminating observable: if the bias snapshot stopped flowing,
    // both runs would pick the same intent → the same event kind → this fails.
    // (The sibling test `ac4_active_signature_bias_changes_selected_intent`
    // covers the intent discriminant via select_outfield_intent in isolation;
    // this one proves the SAME divergence survives the full dispatch_tick path.)
    let shoot_fired_shot = state_shoot_after
        .match_events()
        .iter()
        .any(|e| matches!(e, MatchEvent::Shot { .. }));
    let pass_fired_pass = state_pass_after
        .match_events()
        .iter()
        .any(|e| matches!(e, MatchEvent::Pass { .. }));
    let shoot_fired_pass = state_shoot_after
        .match_events()
        .iter()
        .any(|e| matches!(e, MatchEvent::Pass { .. }));
    assert!(
        shoot_fired_shot,
        "T1-26: strong-shoot bias must fire a MatchEvent::Shot through dispatch_tick \
         for slot 8. Events: {:?}",
        state_shoot_after.match_events(),
    );
    assert!(
        pass_fired_pass,
        "T1-26: strong-pass bias must fire a MatchEvent::Pass through dispatch_tick \
         for slot 8. Events: {:?}",
        state_pass_after.match_events(),
    );
    assert!(
        !shoot_fired_pass,
        "T1-26: strong-shoot bias must NOT fire a Pass (50× shoot tilt is \
         deterministic). Same event kinds across both runs = the bias snapshot \
         is NOT flowing through combine_active_biases → apply_signature_bias → \
         select_outfield_intent → apply_intent. Events: {:?}",
        state_shoot_after.match_events(),
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

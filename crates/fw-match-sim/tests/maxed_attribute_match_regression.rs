//! Regression: a match with maxed-attribute (elite procgen-career) players must
//! run to `FullTime` without panicking.
//!
//! ## The bug this guards against
//!
//! The `apply_*_bias` helpers in `bt/personality_bias.rs` used to assert
//! `raw >= 0 && raw <= 1`. But the BT utility functions multiply a `[0, 1]`
//! attribute product by personality bonus factors of the form `(1 + 0.1·attr)`,
//! so an elite player legitimately produces `raw > 1.0`:
//!   - `utility_hold_formation` → 1.1 at maxed attributes
//!   - `utility_run_off_ball`   → 1.21 at maxed attributes
//!
//! Per `.claude/rules/Sim/RULES.md` §11, `assert!` fires in RELEASE, so a
//! high-attribute player tripped the over-strict `raw <= 1` assert and PANICKED
//! the match step. `MatchState::initial`'s default players use
//! `mid_range_baseline()` (all 0.5) and never reach `raw > 1.0`, which is why
//! `dump_frames` and the canonical-hash seeds dodged the crash — but a real
//! career match with high-attribute players hit it, freezing the match.
//!
//! The fix relaxed the assert to `raw >= 0` (the upper bound was incorrect:
//! `raw > 1` is legitimate and desired by the Slice-0 elite skew). This test
//! drives a full match with all 22 players maxed to confirm it completes.

use std::collections::BTreeMap;

use fw_core::{PlayerAttributes, Seed};
use fw_match_sim::{MatchEvent, MatchState, SignatureDefinition, tick_match};

/// Run a full match (to `match_end_tick`) with all 22 players carrying
/// `max_baseline()` attributes. Must reach `FullTime` without panicking.
#[test]
fn maxed_attribute_roster_runs_to_full_time_without_panic() {
    let seed = Seed::from_u64(0xE11E_E11E_E11E_E11E);
    let empty_sigs: BTreeMap<String, SignatureDefinition> = BTreeMap::new();

    // An elite roster: every outfield + GK slot maxed. This is the high-attribute
    // case a procedurally-generated career club can produce and that the
    // mid-range default never exercises.
    let mut state = MatchState::initial(seed);
    for player in state.players.iter_mut() {
        *player.attributes_mut() = PlayerAttributes::max_baseline();
    }

    // Short budget keeps the test fast while still driving every BT/FSM path
    // (kick-off → in-play decisions → FullTime). 600 ticks = 10 in-sim minutes.
    state = state.with_match_end_tick(fw_core::Tick::from_raw(600));

    // Drive past match_end_tick; the freeze guard makes the trailing calls no-ops.
    // Before the fix this panics on the first tick where a maxed player evaluates
    // utility_hold_formation / utility_run_off_ball (raw > 1.0).
    for _ in 0..620 {
        state = tick_match(state, &empty_sigs);
    }

    let reached_full_time = matches!(
        state.match_events().last(),
        Some(MatchEvent::FullTime { .. })
    );
    assert!(
        reached_full_time,
        "maxed-attribute match must reach FullTime; last event = {:?}",
        state.match_events().last()
    );
}

/// The same elite roster routed through the real career match entry point
/// (`initial_with_content` → role-spread signature candidates), with attributes
/// maxed afterward, must also complete. This mirrors the production
/// `play_one_match` path more closely than `MatchState::initial`.
#[test]
fn maxed_attribute_roster_via_content_path_runs_to_full_time() {
    use std::path::PathBuf;

    let content_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content");
    let store = fw_content::ContentStore::load_sources(&content_root)
        .expect("ContentStore::load_sources failed");

    let seed = Seed::from_u64(0x600D_F00D_600D_F00D);
    let empty_sigs: BTreeMap<String, SignatureDefinition> = BTreeMap::new();

    let mut state = MatchState::initial_with_content(
        seed,
        &store,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
        fw_match_sim::DEFAULT_ARCHETYPE_ID,
    )
    .expect("initial_with_content failed");

    for player in state.players.iter_mut() {
        *player.attributes_mut() = PlayerAttributes::max_baseline();
    }
    state = state.with_match_end_tick(fw_core::Tick::from_raw(600));

    for _ in 0..620 {
        state = tick_match(state, &empty_sigs);
    }

    assert!(
        matches!(
            state.match_events().last(),
            Some(MatchEvent::FullTime { .. })
        ),
        "maxed content-path match must reach FullTime; last event = {:?}",
        state.match_events().last()
    );
}

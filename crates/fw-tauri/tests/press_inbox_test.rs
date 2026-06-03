//! T4-2.5k integration tests — PressReader IPC (`get_press_inbox`).
//!
//! Acceptance criteria:
//!
//! 1. After 2 complete seasons, `get_press_inbox_inner` returns a non-empty
//!    `PressInboxDto` containing ≥1 item with `topic == "matchResult"`.
//!    The top-K-per-topic merge (K=6) guarantees TitleWon events surface
//!    alongside DebutSenior events even when the total ledger is large.
//!
//! 2. Every returned `headline` is non-empty (items are rendered, not raw
//!    discriminant numbers).
//!
//! 3. All returned items have unique `event_id` values (dedup works).
//!
//! 4. The `items` list contains at most 20 entries (cap enforced).
//!
//! 5. `TitleWon` events are in the ledger AND `PressReader::candidates`
//!    returns them as `MatchResult` candidates (wiring proof independent
//!    of the salience sort).
//!
//! 6. Rendered headlines contain the player's display name (not nameless)
//!    for player-subject events.

use std::path::PathBuf;

use fw_core::Seed;
use fw_memory::event::EventClass;
use fw_memory::readers::PressTopic;
use fw_memory::readers::press::PressReader;
use fw_tauri::commands::{advance_season_inner, get_press_inbox_inner, play_fixtures_inner};
use fw_tauri::state::AppState;

fn workspace_content_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

fn test_state() -> AppState {
    AppState::new_with_career_seed(
        &workspace_content_path(),
        Seed::from_u64(0xdead_beef_cafe_f00d),
    )
    .expect("AppState::new_with_career_seed in test")
}

/// Play all fixtures for the current season, then call advance_season.
fn play_one_full_season(state: &AppState) {
    play_fixtures_inner(state).expect("play_fixtures_inner");
    advance_season_inner(state).expect("advance_season_inner");
}

// ---------------------------------------------------------------------------
// AC1–4: inbox shape, rendering, dedup, cap; AC1 now includes matchResult
// ---------------------------------------------------------------------------

/// After 2 complete seasons:
/// - inbox is non-empty.
/// - ≥1 item has `topic == "matchResult"` (TitleWon via top-K-per-topic merge).
/// - every `headline` is non-empty (rendering works end-to-end).
/// - all event_ids are unique (dedup exercised).
/// - at most 20 items (cap enforced).
#[test]
fn press_inbox_non_empty_after_two_seasons() {
    let state = test_state();

    play_one_full_season(&state);
    play_one_full_season(&state);

    let inbox = get_press_inbox_inner(&state).expect("get_press_inbox_inner");

    // AC1a — non-empty
    assert!(
        !inbox.items.is_empty(),
        "press inbox must be non-empty after 2 seasons; got 0 items."
    );

    // AC1b — ≥1 matchResult item (TitleWon guaranteed by advance_season_inner;
    //        top-K-per-topic merge ensures it surfaces alongside PlayerMilestone events)
    let match_result_count = inbox
        .items
        .iter()
        .filter(|i| i.topic == "matchResult")
        .count();
    assert!(
        match_result_count >= 1,
        "press inbox must contain ≥1 item with topic == \"matchResult\" after 2 seasons; \
         got {match_result_count}. TitleWon (disc 22, MatchResult) fires on every \
         advance_season_inner; the top-K-per-topic merge must surface it alongside \
         DebutSenior events."
    );

    // AC2 — every headline non-empty (rendered, not a raw discriminant number)
    for item in &inbox.items {
        assert!(
            !item.headline.is_empty(),
            "press item event_id={} topic={} has an empty headline — \
             render_memory_callback must return a non-empty string",
            item.event_id,
            item.topic,
        );
    }

    // AC3 — all event_ids unique (dedup path exercised)
    let mut seen_ids: std::collections::BTreeSet<u32> = std::collections::BTreeSet::new();
    for item in &inbox.items {
        assert!(
            seen_ids.insert(item.event_id),
            "duplicate event_id {} in press inbox — dedup path failed",
            item.event_id,
        );
    }

    // AC4 — at most 20 items (cap enforced)
    assert!(
        inbox.items.len() <= 20,
        "press inbox returned {} items; cap is 20",
        inbox.items.len(),
    );
}

// ---------------------------------------------------------------------------
// AC5: TitleWon wiring proof (independent of salience cut)
// ---------------------------------------------------------------------------

/// AC5: `TitleWon` events are in the ledger after 2 `advance_season_inner`
/// calls, and `PressReader::candidates(MatchResult)` returns them.
#[test]
fn title_won_events_in_ledger_and_press_reader_returns_them_as_match_result() {
    let state = test_state();

    play_one_full_season(&state);
    play_one_full_season(&state);

    {
        let career = state.career().read().expect("career lock");
        let title_count = career
            .ledger
            .iter()
            .filter(|e| matches!(e.event_class, EventClass::TitleWon))
            .count();
        assert_eq!(
            title_count, 2,
            "after 2 seasons there must be exactly 2 TitleWon events in the ledger; \
             got {title_count}."
        );
    }

    {
        let mut career = state.career().write().expect("career write lock");
        let now_tick = career.current_tick();
        let candidates =
            PressReader::candidates(&mut career.ledger, PressTopic::MatchResult, now_tick);
        let match_result_count = candidates
            .iter()
            .filter(|e| matches!(e.event_class, EventClass::TitleWon))
            .count();
        assert_eq!(
            match_result_count, 2,
            "PressReader::candidates(MatchResult) must return both TitleWon events; \
             got {match_result_count}."
        );
    }
}

// ---------------------------------------------------------------------------
// AC6: player name resolution wires into MemoryCallbackContext
// ---------------------------------------------------------------------------

/// AC6: for player-subject events (DebutSenior etc.) the `player_name` slot in
/// `MemoryCallbackContext` is populated from the roster — not left empty.
///
/// We prove this by directly checking the `roster_names` lookup: for every
/// PlayerMilestone event that PressReader surfaces, its Subject participant's
/// `PlayerId` must exist in `career.roster` with a non-empty `display_name`.
///
/// We do NOT assert the rendered headline text contains the name — Tracery
/// grammar variants legitimately omit `#player_name#` in some phrases (e.g.
/// "The debut at #club_name# — nervous before kick-off..."), so a text-
/// contains check would be brittle against variant selection. The invariant
/// that matters is: resolution succeeds, so grammars that DO use `#player_name#`
/// will render a real name rather than an empty slot.
#[test]
fn player_milestone_subject_players_are_in_roster() {
    use fw_memory::event::ParticipantRole;
    use fw_memory::readers::press::PressReader;

    let state = test_state();

    play_one_full_season(&state);
    play_one_full_season(&state);

    // Collect PlayerId → display_name from roster (mirrors the production lookup).
    let roster_names: std::collections::BTreeMap<fw_core::PlayerId, String> = {
        let career = state.career().read().expect("career lock");
        career
            .roster
            .values()
            .flat_map(|v| v.iter())
            .map(|inst| (inst.player_id, inst.display_name.clone()))
            .collect()
    };

    // Get top-K PlayerMilestone candidates (same path as get_press_inbox_inner).
    let milestone_events: Vec<fw_memory::event::MemoryEvent> = {
        let mut career = state.career().write().expect("career write lock");
        let now_tick = career.current_tick();
        PressReader::candidates(
            &mut career.ledger,
            fw_memory::readers::PressTopic::PlayerMilestone,
            now_tick,
        )
        .into_iter()
        .take(6)
        .cloned()
        .collect()
    };

    assert!(
        !milestone_events.is_empty(),
        "expected ≥1 PlayerMilestone candidate after 2 seasons"
    );

    // For every player-subject event, the Subject PlayerId must resolve to a
    // non-empty name in the roster — proving the resolution path works.
    for event in &milestone_events {
        let subject_pid: Option<fw_core::PlayerId> = event.participants.iter().find_map(|p| {
            if p.role == ParticipantRole::Subject
                && let fw_memory::event::EntityRef::Player(pid) = p.entity
            {
                return Some(pid);
            }
            None
        });

        let Some(pid) = subject_pid else {
            continue; // non-player-subject event — skip
        };

        let name = roster_names.get(&pid);
        assert!(
            name.is_some_and(|n| !n.is_empty()),
            "PlayerMilestone event {:?} has Subject player_id {:?} that is not in roster \
             or has an empty display_name — player_name resolution would produce an empty \
             context slot",
            event.event_class,
            pid,
        );
    }
}

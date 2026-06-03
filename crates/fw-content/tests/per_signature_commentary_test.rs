//! Tests for T4-2.5i: per-signature commentary sub-bank routing + slot→name.
//!
//! These are TDD-first tests — written before the implementation lands.
//! All four acceptance tests are listed in MEMORY.md §AC-to-test:
//!
//! 1. `per_signature_commentary_differs_and_names_player`
//! 2. `each_signature_bank_has_at_least_three_variants`
//! 3. `signature_commentary_is_deterministic`
//! 4. `nameless_signature_commentary_uses_positional_label`
//!
//! Loads real committed content via `ContentStore::load_sources` (mirrors
//! `commentary_render_test.rs` conventions).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use fw_content::{SignatureId, commentary::render_event, event::MatchEvent, runtime::ContentStore};
use fw_core::{PlayerSlot, Tick};

// ---------------------------------------------------------------------------
// Fixture helpers
// ---------------------------------------------------------------------------

fn content_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("content")
}

/// Cached ContentStore to avoid repeated disk I/O across tests in this file.
fn store() -> &'static ContentStore {
    static STORE: OnceLock<ContentStore> = OnceLock::new();
    STORE.get_or_init(|| {
        ContentStore::load_sources(&content_root()).expect("content load must succeed")
    })
}

fn sig_event_long_range(slot: PlayerSlot) -> MatchEvent {
    let id = SignatureId::try_new("fwh.core:signature.long-range-strike").expect("valid sig id");
    MatchEvent::SignatureFirstFired {
        player_slot: slot,
        signature_id: id,
        tick: Tick::from_raw(50),
    }
}

fn sig_event_diagonal_switch(slot: PlayerSlot) -> MatchEvent {
    let id = SignatureId::try_new("fwh.core:signature.first-time-diagonal-switch")
        .expect("valid sig id");
    MatchEvent::SignatureFirstFired {
        player_slot: slot,
        signature_id: id,
        tick: Tick::from_raw(30),
    }
}

fn names_map(slot: PlayerSlot, name: &str) -> BTreeMap<PlayerSlot, String> {
    let mut m = BTreeMap::new();
    m.insert(slot, name.to_string());
    m
}

// ---------------------------------------------------------------------------
// Test 1: two different signatures produce different commentary strings,
// each naming the player.
// ---------------------------------------------------------------------------

#[test]
fn per_signature_commentary_differs_and_names_player() {
    let s = store();
    let bank = &s.commentary_grammars;
    let slot: PlayerSlot = 9;
    let slot_names = names_map(slot, "Ada Brightwood");

    // Guard the routing premise: both sub-banks must actually be loaded, else
    // the divergence below would be proving nothing about routing.
    assert!(
        bank.signature_bank_origin_len("long-range-strike") >= 3
            && bank.signature_bank_origin_len("first-time-diagonal-switch") >= 3,
        "both per-signature sub-banks must be loaded for this routing test"
    );

    // CRITICAL (per T4-2.5i self-review P1): both events share the SAME tick,
    // slot, and match_seed. The commentary RNG seed is
    // `seed_fn(match_seed, tick, Commentary, (slot<<16)|disc)` — identical for
    // both events here, so the variant INDEX picked is identical. The ONLY
    // thing that can make the two rendered strings differ is which grammar
    // bank was selected. If routing regressed (both signatures fell through to
    // the generic bank), both calls would render byte-identical output and
    // `assert_ne!` would fail. An earlier draft used different ticks, which
    // made the divergence attributable to seed arithmetic rather than routing
    // — a vacuous test that survived a total routing failure.
    const SHARED_TICK: i64 = 40;
    let ev_lrs = MatchEvent::SignatureFirstFired {
        player_slot: slot,
        signature_id: SignatureId::try_new("fwh.core:signature.long-range-strike")
            .expect("valid sig id"),
        tick: Tick::from_raw(SHARED_TICK),
    };
    let ev_diag = MatchEvent::SignatureFirstFired {
        player_slot: slot,
        signature_id: SignatureId::try_new("fwh.core:signature.first-time-diagonal-switch")
            .expect("valid sig id"),
        tick: Tick::from_raw(SHARED_TICK),
    };

    let line_lrs = render_event(&ev_lrs, 0xDEAD_BEEF_u64, bank, &slot_names)
        .expect("long-range-strike render must succeed");
    let line_diag = render_event(&ev_diag, 0xDEAD_BEEF_u64, bank, &slot_names)
        .expect("first-time-diagonal-switch render must succeed");

    // Same seed/tick/slot → identical RNG → outputs can ONLY differ by which
    // sub-bank routing selected.
    assert_ne!(
        line_lrs, line_diag,
        "two different signature sub-banks must produce different commentary at \
         identical seed/tick/slot; got identical: {line_lrs:?}"
    );

    // Both must contain the player's name.
    assert!(
        line_lrs.contains("Ada Brightwood"),
        "long-range-strike commentary must contain player name 'Ada Brightwood'; \
         got: {line_lrs:?}"
    );
    assert!(
        line_diag.contains("Ada Brightwood"),
        "first-time-diagonal-switch commentary must contain player name 'Ada Brightwood'; \
         got: {line_diag:?}"
    );
}

// ---------------------------------------------------------------------------
// Test 2: each signature sub-bank has ≥3 origin variants.
// ---------------------------------------------------------------------------

#[test]
fn each_signature_bank_has_at_least_three_variants() {
    let s = store();
    let bank = &s.commentary_grammars;

    let slugs = ["long-range-strike", "first-time-diagonal-switch"];
    for slug in &slugs {
        let variant_count = bank.signature_bank_origin_len(slug);
        assert!(
            variant_count >= 3,
            "signature sub-bank for '{slug}' must have ≥3 origin variants; got {variant_count}"
        );
    }
}

// ---------------------------------------------------------------------------
// Test 3: determinism — same inputs → identical output twice.
// ---------------------------------------------------------------------------

#[test]
fn signature_commentary_is_deterministic() {
    let s = store();
    let slot: PlayerSlot = 9;
    let slot_names = names_map(slot, "Marcus Thornton");

    let ev_lrs = sig_event_long_range(slot);
    let ev_diag = sig_event_diagonal_switch(slot);

    for (ev, label) in [
        (&ev_lrs, "long-range-strike"),
        (&ev_diag, "diagonal-switch"),
    ] {
        for seed in [0u64, 0xCAFE_BABE, 0xDEAD_BEEF, 12345, 99999] {
            let a = render_event(ev, seed, &s.commentary_grammars, &slot_names)
                .expect("render must succeed");
            let b = render_event(ev, seed, &s.commentary_grammars, &slot_names)
                .expect("render must succeed");
            assert_eq!(
                a, b,
                "render_event({label}) is not deterministic for seed {seed}: {a:?} != {b:?}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Test 4: empty slot_names → positional label used, NOT a bare slot number.
// ---------------------------------------------------------------------------

#[test]
fn nameless_signature_commentary_uses_positional_label() {
    let s = store();
    let empty: BTreeMap<PlayerSlot, String> = BTreeMap::new();

    // Slot 9 is a forward (home). Should produce a label like "a forward",
    // not the bare string "9".
    let ev = sig_event_long_range(9);
    let line = render_event(&ev, 0xDEAD_BEEF_u64, &s.commentary_grammars, &empty)
        .expect("render must succeed with empty slot_names");

    assert!(!line.is_empty(), "render must produce non-empty output");

    // Must NOT contain the bare slot number as a stand-alone token.
    // (Checking "9" is present as a bare positional label.)
    assert!(
        !line.contains(" 9 ") && !line.starts_with("9 ") && !line.ends_with(" 9") && line != "9",
        "commentary must not contain bare slot number '9'; got: {line:?}"
    );

    // Must contain a football-native positional label.
    let has_positional_label = line.contains("forward")
        || line.contains("midfielder")
        || line.contains("defender")
        || line.contains("goalkeeper");
    assert!(
        has_positional_label,
        "commentary must contain a positional label (forward/midfielder/defender/goalkeeper); \
         got: {line:?}"
    );
}

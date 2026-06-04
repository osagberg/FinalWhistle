//! Drama metrics — M1 through M8 per `docs/design/drama-model.md`.
//!
//! Bin-local module: lives at `src/bin/drama.rs`, declared via `mod drama;`
//! in `drama_sweep.rs`. NOT part of the `fw-match-sim` lib — this keeps
//! float-using analysis code structurally outside canonical paths.
//!
//! `#![allow(clippy::float_arithmetic)]` is in `drama_sweep.rs` (the bin root)
//! and covers this module. The determinism-audit float exemption is scoped to
//! `src/bin/drama_sweep.rs` + `src/bin/drama.rs` per the `inspect_frames`
//! precedent (float rule only; HashMap/clock/RNG bans stay active).
//!
//! Pure functions over a `&[MatchEvent]` stream. No sim state mutation,
//! no floats in the metric inputs (goals/counts are integers), but floats
//! are PERMITTED here for aggregation / percentile arithmetic: this is
//! off-canonical-path tooling in the same spirit as the `calibrate` binary
//! (Sim/RULES.md §1 — only the sim itself + canonical state forbid floats;
//! off-sim-path bake/calibration/reporting tools opt in to f64 for
//! arithmetic). The `#[allow(clippy::float_arithmetic)]` below covers the
//! aggregation helpers.
//!
//! Every metric function is a pure function:
//!   `fn metric_foo(events: &[MatchEvent]) -> <T>` or
//!   `fn metric_foo(events: &[MatchEvent], match_end_tick: i64) -> <T>`
//!
//! The `match_end_tick` is read from `MatchEvent::FullTime::tick` by the
//! per-match runner; the metric functions accept it as a parameter so each
//! function is unit-testable on hand-crafted event sequences.
//!
//! ## Metric index
//!
//! | ID | Name                     | Class           |
//! |----|--------------------------|-----------------|
//! | M1 | Goals per match          | Realism guard   |
//! | M2 | Goal-timing distribution | Realism guard   |
//! | M3 | Competitive margin       | Drama target    |
//! | M4 | Lead changes + equaliser | Drama target    |
//! | M5 | Late drama rate          | Drama target    |
//! | M6 | Comeback magnitude       | Drama target    |
//! | M7 | Nervy finish rate        | Drama target    |
//! | M8 | Key-moment density       | Realism guard   |
//!
//! ## Realism guard bands (Phase-1 provisional)
//!
//! These mirror `docs/design/drama-model.md` §§M1, M2, M8 verbatim.
//! Any change to the numbers must be made in drama-model.md first, then here.
//!
//! M1: mean 2.3–3.2 goals/match, std dev 0.8–1.6, P95 ≤ 7, P5 ≥ 0.
//! M2: first-third goal% ≤ 55% → GUARD FAIL if > 55%.
//! M8: mean shots/match 9–18 (guard); mean signatures fired 0.5–4.0 (guard).

use fw_content::MatchEvent;
use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Side — identifies home or away team in anti-scripting metrics
// ---------------------------------------------------------------------------

/// Which side of the pitch a team occupies: Home or Away.
///
/// Replaces the raw `u8` convention (0=home, 1=away) used in the anti-scripting
/// fields. Defined here (drama.rs) so it travels with the metric types; re-exported
/// to main.rs via `use drama::*`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Side {
    Home,
    Away,
}

// ---------------------------------------------------------------------------
// M1 — Goals per match
// ---------------------------------------------------------------------------

/// Count total goals in a single match's event stream.
///
/// Source: `MatchEvent::Goal` variants only. If `FullTime` is present its
/// scores are a cross-check, but this function counts Goal events directly to
/// avoid an off-by-one if the last goal tick equals FullTime tick.
///
/// Returns 0 for a goalless match.
pub fn m1_goals(events: &[MatchEvent]) -> u32 {
    events
        .iter()
        .filter(|e| matches!(e, MatchEvent::Goal { .. }))
        .count() as u32
}

// ---------------------------------------------------------------------------
// M2 — Goal-timing distribution
// ---------------------------------------------------------------------------

/// The three temporal thirds of the match (0-33%, 33-66%, 66-100%).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GoalThirds {
    /// Number of goals in the first third (0 ≤ frac < 0.333).
    pub first: u32,
    /// Number of goals in the middle third (0.333 ≤ frac < 0.667).
    pub middle: u32,
    /// Number of goals in the final third (0.667 ≤ frac ≤ 1.0).
    pub final_third: u32,
}

impl GoalThirds {
    /// Total goals across all thirds.
    // Used in unit tests + first_fraction; not called from main.rs aggregation
    // (which uses raw .first count for pooled M2 computation).
    #[allow(dead_code)]
    pub fn total(&self) -> u32 {
        self.first + self.middle + self.final_third
    }

    /// Fraction of goals in the first third. Returns 0.0 for 0-goal matches.
    ///
    /// Used in unit tests. The main.rs aggregation uses raw `.first` counts
    /// for the pooled M2 corpus ratio instead of calling this per-match.
    #[allow(dead_code, clippy::float_arithmetic)]
    pub fn first_fraction(&self) -> f64 {
        let total = self.total();
        if total == 0 {
            return 0.0;
        }
        self.first as f64 / total as f64
    }
}

/// Distribute goals across match thirds using fractional tick position.
///
/// Each goal's `frac = goal_tick / match_end_tick`. The `match_end_tick`
/// is the raw i64 tick value from `MatchEvent::FullTime::tick` (or the
/// `FULL_MATCH_TICKS` constant if FullTime has not yet fired).
///
/// The drama-model spec uses fractional-tick notation so that short-budget
/// test matches (e.g. 60 ticks) produce the same fractions as a full 5400-tick
/// run.
///
/// Goals at exactly `frac == 0.333` fall into the middle third; goals at
/// exactly `frac == 0.667` fall into the final third (half-open intervals:
/// first: [0, 1/3), middle: [1/3, 2/3), final: [2/3, 1]).
#[allow(clippy::float_arithmetic)]
pub fn m2_goal_timing(events: &[MatchEvent], match_end_tick: i64) -> GoalThirds {
    let mut first = 0u32;
    let mut middle = 0u32;
    let mut final_third = 0u32;

    let end = match_end_tick.max(1) as f64;

    for e in events {
        if let MatchEvent::Goal { tick, .. } = e {
            let frac = tick.to_raw() as f64 / end;
            // Half-open intervals: [0, 1/3), [1/3, 2/3), [2/3, 1].
            if frac < 1.0 / 3.0 {
                first += 1;
            } else if frac < 2.0 / 3.0 {
                middle += 1;
            } else {
                final_third += 1;
            }
        }
    }

    GoalThirds {
        first,
        middle,
        final_third,
    }
}

// ---------------------------------------------------------------------------
// M3 — Competitive margin
// ---------------------------------------------------------------------------

/// Absolute goal margin at full time (`|home_score - away_score|`).
///
/// Source: `MatchEvent::FullTime`. Returns `None` if `FullTime` is absent
/// (match not complete — caller should skip incomplete matches).
pub fn m3_competitive_margin(events: &[MatchEvent]) -> Option<u32> {
    for e in events {
        if let MatchEvent::FullTime {
            home_score,
            away_score,
            ..
        } = e
        {
            let diff = (*home_score as i32 - *away_score as i32).unsigned_abs();
            return Some(diff);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// M4 — Lead changes + equalisers
// ---------------------------------------------------------------------------

/// Result of the M4 analysis for one match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LeadDrama {
    /// Number of times the leading team changed (not just equalisers —
    /// requires the lead to swap: e.g. home ahead → away ahead).
    pub lead_changes: u32,
    /// Number of goals that equalized the score (home and away tied at
    /// `n:n` after it was `n:m` with m < n, or vice versa).
    pub equalisers: u32,
}

/// Compute lead changes and equalisers from the ordered Goal event stream.
///
/// The running score is reconstructed from `Goal::score_home_after` /
/// `Goal::score_away_after` in tick-ascending order (chronological by
/// construction in the MatchEvent Vec).
///
/// Definitions per `drama-model.md §M4`:
/// - **Equaliser**: a goal that restores parity from a deficit (score becomes
///   `n:n` where it was previously `n:m`, `m < n`). The equalising team was
///   trailing before.
/// - **Lead change**: a goal that transfers the lead from one team to the
///   other. This counts whether the transfer goes directly (1-0 → 1-2) or
///   via level (1-0 → 1-1 → 1-2 produces one equaliser AND one lead change).
///   Tracking: `last_leader` = the team most recently in front. If the new
///   leader is the opposite team from `last_leader`, that is a lead change.
pub fn m4_lead_drama(events: &[MatchEvent]) -> LeadDrama {
    let mut lead_changes = 0u32;
    let mut equalisers = 0u32;

    // current_leader: None = level, Some(0) = home, Some(1) = away.
    let mut current_leader: Option<u8> = None;
    // last_leader_in_lead: the team most recently in front (ignores level
    // periods). Used to detect when leadership transfers to the other side.
    let mut last_leader_in_lead: Option<u8> = None;

    for e in events {
        if let MatchEvent::Goal {
            score_home_after,
            score_away_after,
            ..
        } = e
        {
            let h = *score_home_after;
            let a = *score_away_after;

            // Detect equaliser: score becomes level while someone was ahead.
            let now_level = h == a;
            if now_level && current_leader.is_some() {
                equalisers += 1;
            }

            let new_leader: Option<u8> = if h > a {
                Some(0)
            } else if a > h {
                Some(1)
            } else {
                None // level
            };

            // Detect lead change: leadership transfers to the other team.
            // This fires when:
            //   - A team goes from level (or from the trailing position) to
            //     taking the lead, AND the other team was previously the leader.
            if let Some(new_l) = new_leader {
                if let Some(last_l) = last_leader_in_lead
                    && new_l != last_l
                {
                    // Leadership transferred to the other team.
                    lead_changes += 1;
                }
                last_leader_in_lead = Some(new_l);
            }

            current_leader = new_leader;
        }
    }

    LeadDrama {
        lead_changes,
        equalisers,
    }
}

// ---------------------------------------------------------------------------
// M5 — Late drama rate
// ---------------------------------------------------------------------------

/// Whether a single match has late drama.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LateDrama {
    /// Whether any goal was scored in the final 15% of ticks
    /// (`goal_tick / match_end_tick > 0.85`).
    pub has_late_goal: bool,
    /// Whether any goal in the final 15% was a late winner or late
    /// equaliser (changed the result or restored parity).
    pub has_late_winner: bool,
    /// Which side scored the decisive late goal (the last one that changed
    /// the result or restored parity). `None` if no late decider occurred.
    ///
    /// This deduplicates the `late_decider_team` predicate that was previously
    /// in main.rs — both use the same five-branch `changed` expression.
    /// Keep in sync: any change to the `changed` expression here must be
    /// mirrored to the anti-scripting comeback check in main.rs `aggregate()`.
    pub late_decider_side: Option<Side>,
}

/// Detect late drama in a match.
///
/// "Late" = `tick / match_end_tick > 0.85`.
/// "Winner / equaliser" = the goal either broke a tie or swapped the lead.
///
/// `match_end_tick` is the raw i64 from `FullTime::tick`.
///
/// `late_decider_side` identifies which side scored the last decisive late
/// goal (changed result or restored parity). Used by the anti-scripting
/// metric in main.rs `aggregate()`; factored here to avoid a duplicate
/// `changed` predicate. The five-branch expression below is the single
/// source of truth — keep in sync with the anti-scripting comeback check.
#[allow(clippy::float_arithmetic)]
pub fn m5_late_drama(events: &[MatchEvent], match_end_tick: i64) -> LateDrama {
    let mut has_late_goal = false;
    let mut has_late_winner = false;
    let mut late_decider_side: Option<Side> = None;

    let end = match_end_tick.max(1) as f64;
    let late_threshold = end * 0.85;

    // Walk Goal events; track running score to detect result-changes.
    // (We reconstruct from Goal::score_home_after / score_away_after.)
    let mut prev_h = 0u16;
    let mut prev_a = 0u16;

    for e in events {
        if let MatchEvent::Goal {
            tick,
            score_home_after,
            score_away_after,
            ..
        } = e
        {
            let t = tick.to_raw() as f64;
            let is_late = t > late_threshold;

            if is_late {
                has_late_goal = true;

                // Was the result different before this goal?
                // Before: prev_h:prev_a. After: score_home_after:score_away_after.
                let prev_h_won = prev_h > prev_a;
                let prev_a_won = prev_a > prev_h;
                let prev_level = prev_h == prev_a;

                let h = *score_home_after;
                let a = *score_away_after;
                let now_h_won = h > a;
                let now_a_won = a > h;
                let now_level = h == a;

                // Late winner: result changed (from trailing or level → winning).
                // Late equaliser: was losing → now level.
                let changed = (prev_h_won && now_a_won)
                    || (prev_a_won && now_h_won)
                    || (prev_level && (now_h_won || now_a_won))
                    || (prev_h_won && now_level)
                    || (prev_a_won && now_level);

                if changed {
                    has_late_winner = true;
                    // Record which side scored the decisive goal.
                    late_decider_side = if h > prev_h {
                        Some(Side::Home)
                    } else {
                        Some(Side::Away)
                    };
                    // Keep scanning: a later goal may supersede this one.
                }
            }

            prev_h = *score_home_after;
            prev_a = *score_away_after;
        }
    }

    LateDrama {
        has_late_goal,
        has_late_winner,
        late_decider_side,
    }
}

// ---------------------------------------------------------------------------
// M6 — Comeback magnitude
// ---------------------------------------------------------------------------

/// The largest deficit overcome by any team in the match.
///
/// "Deficit overcome" means: team X was behind by N goals at some point AND
/// ended the match tied or winning. For draws, both teams' perspectives are
/// checked; for wins, only the eventual winner is checked.
///
/// Returns 0 if neither team came from behind.
pub fn m6_comeback_magnitude(events: &[MatchEvent]) -> u32 {
    // Reconstruct the score timeline.
    let mut scores: Vec<(u16, u16)> = vec![(0, 0)];
    for e in events {
        if let MatchEvent::Goal {
            score_home_after,
            score_away_after,
            ..
        } = e
        {
            scores.push((*score_home_after, *score_away_after));
        }
    }

    let (final_h, final_a) = *scores.last().unwrap_or(&(0, 0));
    let mut max_comeback = 0u32;

    // Check home team comeback: home was behind by D goals, ends winning or drawing.
    if final_h >= final_a {
        // Home didn't lose — check their max deficit.
        let mut max_deficit = 0u32;
        for &(h, a) in &scores {
            if a > h {
                let deficit = (a - h) as u32;
                if deficit > max_deficit {
                    max_deficit = deficit;
                }
            }
        }
        if max_deficit > max_comeback {
            max_comeback = max_deficit;
        }
    }

    // Check away team comeback: away was behind by D goals, ends winning or drawing.
    if final_a >= final_h {
        let mut max_deficit = 0u32;
        for &(h, a) in &scores {
            if h > a {
                let deficit = (h - a) as u32;
                if deficit > max_deficit {
                    max_deficit = deficit;
                }
            }
        }
        if max_deficit > max_comeback {
            max_comeback = max_deficit;
        }
    }

    max_comeback
}

// ---------------------------------------------------------------------------
// M7 — Nervy finish rate
// ---------------------------------------------------------------------------

/// Whether the result is "in doubt" (margin ≤ 1) at the 90% tick mark.
///
/// "In doubt" = the running score margin at `tick = 0.90 × match_end_tick`
/// is 0 (level) or 1 (one goal apart — either team can equalise or score
/// a late winner).
///
/// `match_end_tick` is the raw i64 from `FullTime::tick`.
#[allow(clippy::float_arithmetic)]
pub fn m7_nervy_finish(events: &[MatchEvent], match_end_tick: i64) -> bool {
    let end = match_end_tick.max(1) as f64;
    let threshold_raw = (end * 0.90) as i64;

    // Reconstruct score at threshold tick.
    let mut h = 0u16;
    let mut a = 0u16;

    for e in events {
        if let MatchEvent::Goal {
            tick,
            score_home_after,
            score_away_after,
            ..
        } = e
        {
            if tick.to_raw() <= threshold_raw {
                h = *score_home_after;
                a = *score_away_after;
            } else {
                break; // events are chronological; no need to continue
            }
        }
    }

    let margin = h.abs_diff(a);
    margin <= 1
}

// ---------------------------------------------------------------------------
// M8 — Key-moment density
// ---------------------------------------------------------------------------

/// Key-moment counts for one match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeyMoments {
    /// Total goals in the match.
    pub goals: u32,
    /// Total shots in the match (all shots; `on_target` sub-metric deferred
    /// to T2+ when `Shot::on_target` is reliable — drama-model open Q noted).
    pub shots: u32,
    /// Total `SignatureFirstFired` events (each player+signature fires once
    /// per match).
    pub signatures_fired: u32,
}

impl KeyMoments {
    /// Sum of all salient events.
    // Used in unit tests only; not called from drama_sweep aggregation.
    #[allow(dead_code)]
    pub fn total(&self) -> u32 {
        self.goals + self.shots + self.signatures_fired
    }
}

/// Count all salient events in a match: `Goal + Shot + SignatureFirstFired`.
///
/// Note on `Shot::on_target`: `Shot` events at T1 DO carry an `on_target`
/// field (derived from `target_y` vs `GOAL_HALF_WIDTH_M`), so reporting
/// on-target rate per match is feasible here. However the drama-model open Q
/// states "Shot::on_target is deferred to T2+" referring to REAL contest
/// physics. The raw `on_target` field available at T1 is a geometric
/// approximation only. We collect it here for the on-target rate diagnostic
/// but do NOT guard against the 30-55% band yet — that gate awaits T2+.
pub fn m8_key_moments(events: &[MatchEvent]) -> KeyMoments {
    let mut goals = 0u32;
    let mut shots = 0u32;
    let mut signatures_fired = 0u32;

    for e in events {
        match e {
            MatchEvent::Goal { .. } => goals += 1,
            MatchEvent::Shot { .. } => shots += 1,
            MatchEvent::SignatureFirstFired { .. } => signatures_fired += 1,
            _ => {}
        }
    }

    KeyMoments {
        goals,
        shots,
        signatures_fired,
    }
}

/// Count on-target shots in the event stream.
///
/// Returns `(on_target, total_shots)` for the on-target rate diagnostic.
/// Used by the drama-sweep report but not yet a guarded M8 band (T2+).
pub fn m8_on_target_count(events: &[MatchEvent]) -> (u32, u32) {
    let mut on_target = 0u32;
    let mut total = 0u32;
    for e in events {
        if let MatchEvent::Shot { on_target: ot, .. } = e {
            total += 1;
            if *ot {
                on_target += 1;
            }
        }
    }
    (on_target, total)
}

// ---------------------------------------------------------------------------
// Helper: extract match_end_tick from an event stream
// ---------------------------------------------------------------------------

/// Extract the `match_end_tick` (raw i64) from a `FullTime` event if present,
/// otherwise return `FULL_MATCH_TICKS` as the fallback.
///
/// Allows metric functions to operate on truncated event streams (e.g.
/// calibrate-run truncated at 600 ticks) by falling back to the full match
/// length. The drama-sweep binary always runs full 5400-tick matches so the
/// fallback fires only in tests.
pub fn match_end_tick_from_events(events: &[MatchEvent]) -> i64 {
    for e in events {
        if let MatchEvent::FullTime { tick, .. } = e {
            return tick.to_raw();
        }
    }
    fw_match_sim::FULL_MATCH_TICKS as i64
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_content::{MatchEvent, SignatureId};
    use fw_core::Tick;

    // Helper to make a Goal event.
    fn goal(tick_raw: i64, h: u16, a: u16) -> MatchEvent {
        MatchEvent::Goal {
            scorer_slot: 9,
            tick: Tick::from_raw(tick_raw),
            score_home_after: h,
            score_away_after: a,
        }
    }

    fn kickoff() -> MatchEvent {
        MatchEvent::KickOff {
            tick: Tick::ZERO,
            is_second_half: false,
        }
    }

    fn fulltime(tick_raw: i64, h: u16, a: u16) -> MatchEvent {
        MatchEvent::FullTime {
            tick: Tick::from_raw(tick_raw),
            home_score: h,
            away_score: a,
        }
    }

    fn shot(tick_raw: i64, on_target: bool) -> MatchEvent {
        MatchEvent::Shot {
            shooter_slot: 9,
            tick: Tick::from_raw(tick_raw),
            target_x: fw_core::Q32::ZERO,
            target_y: fw_core::Q32::ZERO,
            on_target,
        }
    }

    fn sig_fired(tick_raw: i64) -> MatchEvent {
        MatchEvent::SignatureFirstFired {
            player_slot: 7,
            signature_id: SignatureId::try_new("fwh.core:signature.test").unwrap(),
            tick: Tick::from_raw(tick_raw),
        }
    }

    // --- M1 tests ---

    #[test]
    fn m1_goals_empty_stream_is_zero() {
        let events: Vec<MatchEvent> = vec![];
        assert_eq!(m1_goals(&events), 0);
    }

    #[test]
    fn m1_goals_no_goals_returns_zero() {
        let events = vec![kickoff(), shot(100, true)];
        assert_eq!(m1_goals(&events), 0);
    }

    #[test]
    fn m1_goals_counts_only_goal_events() {
        // Three Goal events + some noise (KickOff, Shot, FullTime) → 3.
        let events = vec![
            kickoff(),
            goal(600, 1, 0),
            shot(1200, false),
            goal(2700, 1, 1),
            goal(4500, 2, 1),
            fulltime(5400, 2, 1),
        ];
        assert_eq!(m1_goals(&events), 3);
        // Verify this is wrong (fails the metric formula) if we count all events.
        assert_ne!(events.len() as u32, 3); // 6 total events — not the same
    }

    // --- M2 tests ---

    #[test]
    fn m2_goal_timing_no_goals_returns_zeros() {
        let events = vec![kickoff(), fulltime(5400, 0, 0)];
        let thirds = m2_goal_timing(&events, 5400);
        assert_eq!(thirds.first, 0);
        assert_eq!(thirds.middle, 0);
        assert_eq!(thirds.final_third, 0);
    }

    #[test]
    fn m2_goal_timing_first_third_boundary() {
        // Goal at tick 1799 (frac = 1799/5400 ≈ 0.333) → just under 1/3 → first.
        // Goal at tick 1800 (frac = 1800/5400 = 0.333...) → exactly 1/3 → middle.
        let events_first = vec![goal(1799, 1, 0)];
        let thirds_f = m2_goal_timing(&events_first, 5400);
        assert_eq!(thirds_f.first, 1, "tick 1799 should land in first third");
        assert_eq!(thirds_f.middle, 0);

        let events_middle = vec![goal(1800, 1, 0)];
        let thirds_m = m2_goal_timing(&events_middle, 5400);
        assert_eq!(thirds_m.first, 0);
        assert_eq!(thirds_m.middle, 1, "tick 1800 should land in middle third");
    }

    #[test]
    fn m2_goal_timing_spread_across_thirds() {
        // 2 goals in first, 3 in middle, 1 in final.
        let events = vec![
            goal(540, 1, 0),  // frac 0.1 → first
            goal(1080, 2, 0), // frac 0.2 → first
            goal(2160, 2, 1), // frac 0.4 → middle
            goal(2700, 2, 2), // frac 0.5 → middle
            goal(3240, 3, 2), // frac 0.6 → middle
            goal(4860, 4, 2), // frac 0.9 → final
        ];
        let thirds = m2_goal_timing(&events, 5400);
        assert_eq!(thirds.first, 2);
        assert_eq!(thirds.middle, 3);
        assert_eq!(thirds.final_third, 1);
    }

    #[test]
    fn m2_first_fraction_correct() {
        let events = vec![
            goal(540, 1, 0),  // first
            goal(1080, 2, 0), // first
            goal(2700, 2, 1), // middle
            goal(4860, 3, 1), // final
        ];
        let thirds = m2_goal_timing(&events, 5400);
        // 2/4 = 0.5 in first third
        let frac = thirds.first_fraction();
        assert!((frac - 0.5).abs() < 1e-9);
    }

    // --- M3 tests ---

    #[test]
    fn m3_competitive_margin_draw_is_zero() {
        let events = vec![goal(1000, 1, 0), goal(2000, 1, 1), fulltime(5400, 1, 1)];
        assert_eq!(m3_competitive_margin(&events), Some(0));
    }

    #[test]
    fn m3_competitive_margin_one_goal_win() {
        let events = vec![goal(3000, 1, 0), fulltime(5400, 1, 0)];
        assert_eq!(m3_competitive_margin(&events), Some(1));
    }

    #[test]
    fn m3_competitive_margin_three_goal_blowout() {
        let events = vec![
            goal(500, 1, 0),
            goal(1000, 2, 0),
            goal(1500, 3, 0),
            fulltime(5400, 3, 0),
        ];
        assert_eq!(m3_competitive_margin(&events), Some(3));
    }

    #[test]
    fn m3_competitive_margin_no_fulltime_returns_none() {
        let events = vec![goal(1000, 1, 0)];
        // No FullTime event → None (match not complete).
        assert_eq!(m3_competitive_margin(&events), None);
    }

    // --- M4 tests ---

    #[test]
    fn m4_lead_drama_no_goals_no_drama() {
        let events = vec![kickoff()];
        let ld = m4_lead_drama(&events);
        assert_eq!(ld.lead_changes, 0);
        assert_eq!(ld.equalisers, 0);
    }

    #[test]
    fn m4_lead_drama_one_team_scores_no_lead_change() {
        // Home scores 3-0. Three lead changes = 0 (home was always ahead).
        let events = vec![goal(1000, 1, 0), goal(2000, 2, 0), goal(3000, 3, 0)];
        let ld = m4_lead_drama(&events);
        assert_eq!(ld.lead_changes, 0);
        assert_eq!(ld.equalisers, 0);
    }

    #[test]
    fn m4_lead_drama_equaliser_and_lead_change() {
        // home 1-0 → away 1-1 (equaliser) → away 1-2 (lead change) → home 2-2 (equaliser).
        let events = vec![
            goal(1000, 1, 0), // home 1-0
            goal(2000, 1, 1), // equaliser
            goal(3000, 1, 2), // away take lead = lead change
            goal(4000, 2, 2), // equaliser
        ];
        let ld = m4_lead_drama(&events);
        assert_eq!(ld.equalisers, 2, "two equalisers: 1-1 and 2-2");
        assert_eq!(ld.lead_changes, 1, "one lead change: home→away");
    }

    #[test]
    fn m4_lead_drama_back_and_forth() {
        // 0-1 → 1-1 (eq) → 2-1 (lead change) → 2-2 (eq) → 2-3 (lead change)
        let events = vec![
            goal(500, 0, 1),  // away 1 (away ahead)
            goal(1000, 1, 1), // equaliser
            goal(2000, 2, 1), // home take lead = lead change
            goal(3000, 2, 2), // equaliser
            goal(4000, 2, 3), // away take lead = lead change
        ];
        let ld = m4_lead_drama(&events);
        assert_eq!(ld.equalisers, 2, "two equalisers");
        assert_eq!(
            ld.lead_changes, 2,
            "two lead changes: away→home and home→away"
        );
    }

    // --- M5 tests ---

    #[test]
    fn m5_late_drama_no_goals_no_drama() {
        let events = vec![kickoff()];
        let ld = m5_late_drama(&events, 5400);
        assert!(!ld.has_late_goal);
        assert!(!ld.has_late_winner);
    }

    #[test]
    fn m5_late_drama_early_goal_only() {
        // Goal at tick 1000 — well before the 85% threshold (4590).
        let events = vec![goal(1000, 1, 0)];
        let ld = m5_late_drama(&events, 5400);
        assert!(!ld.has_late_goal);
        assert!(!ld.has_late_winner);
    }

    #[test]
    fn m5_late_drama_late_goal_no_result_change() {
        // Home leads 3-0 at tick 4000; goal at tick 4700 makes it 4-0 — late
        // but NOT a result change (home was already winning convincingly).
        let events = vec![
            goal(1000, 1, 0),
            goal(2000, 2, 0),
            goal(3000, 3, 0),
            goal(4700, 4, 0), // late but no result change
        ];
        let ld = m5_late_drama(&events, 5400);
        assert!(ld.has_late_goal, "goal at tick 4700 > 85% of 5400 = 4590");
        assert!(!ld.has_late_winner, "4-0 after 3-0 is not a result change");
    }

    #[test]
    fn m5_late_drama_late_equaliser() {
        // Home leads 1-0; away equalises at tick 4800.
        let events = vec![
            goal(1000, 1, 0),
            goal(4800, 1, 1), // late equaliser
        ];
        let ld = m5_late_drama(&events, 5400);
        assert!(ld.has_late_goal);
        assert!(
            ld.has_late_winner,
            "1-1 after 1-0 is a result change (equaliser)"
        );
    }

    #[test]
    fn m5_late_drama_late_winner() {
        // Level 1-1 before last goal; home winner at tick 5200.
        let events = vec![
            goal(1000, 1, 0),
            goal(2000, 1, 1),
            goal(5200, 2, 1), // late winner
        ];
        let ld = m5_late_drama(&events, 5400);
        assert!(ld.has_late_goal);
        assert!(
            ld.has_late_winner,
            "2-1 after 1-1 is a result change (winner)"
        );
        // Pin: late_decider_side agrees with the scorer identity check.
        // Home scored (h went from 1 to 2) → Side::Home.
        assert_eq!(
            ld.late_decider_side,
            Some(Side::Home),
            "home scored the late winner at 5200"
        );
    }

    #[test]
    fn m5_late_drama_decider_side_away_equaliser() {
        // Home leads 1-0; away equalises at tick 4800 (late).
        // Away scored (a went from 0 to 1) → Side::Away.
        let events = vec![goal(1000, 1, 0), goal(4800, 1, 1)];
        let ld = m5_late_drama(&events, 5400);
        assert_eq!(ld.late_decider_side, Some(Side::Away));
    }

    #[test]
    fn m5_late_drama_no_late_decider_is_none() {
        // Goal at tick 4700 is late but NOT decisive (3-0 → 4-0).
        let events = vec![
            goal(1000, 1, 0),
            goal(2000, 2, 0),
            goal(3000, 3, 0),
            goal(4700, 4, 0),
        ];
        let ld = m5_late_drama(&events, 5400);
        assert_eq!(
            ld.late_decider_side, None,
            "non-decisive late goal must leave late_decider_side = None"
        );
    }

    // --- M6 tests ---

    #[test]
    fn m6_comeback_magnitude_no_comeback_returns_zero() {
        // Home leads from start, never behind.
        let events = vec![goal(1000, 1, 0), goal(2000, 2, 0)];
        assert_eq!(m6_comeback_magnitude(&events), 0);
    }

    #[test]
    fn m6_comeback_magnitude_one_goal_down_draws() {
        // Home scores to make it 0-1, then equalises to 1-1.
        let events = vec![goal(1000, 0, 1), goal(3000, 1, 1)];
        assert_eq!(m6_comeback_magnitude(&events), 1);
    }

    #[test]
    fn m6_comeback_magnitude_two_goals_down_wins() {
        // Away scores 0-1, 0-2; home fights back 1-2, 2-2, 3-2.
        let events = vec![
            goal(500, 0, 1),
            goal(1000, 0, 2),
            goal(2500, 1, 2),
            goal(3500, 2, 2),
            goal(4500, 3, 2),
        ];
        // Home was down 0-2 and wins — comeback of 2.
        assert_eq!(m6_comeback_magnitude(&events), 2);
    }

    #[test]
    fn m6_comeback_magnitude_loses_comeback_counts() {
        // Away was down 0-3 but "came back" to only lose 3-1 — however they
        // still lost. The eventual winner here is HOME. Check home only.
        // Home wins 3-1 from leading 3-0, but they were never behind → 0.
        let events = vec![
            goal(500, 1, 0),
            goal(1000, 2, 0),
            goal(1500, 3, 0),
            goal(3000, 3, 1),
        ];
        // Home never trailed; away trailed all match and still lost → 0.
        assert_eq!(m6_comeback_magnitude(&events), 0);
    }

    #[test]
    fn m6_comeback_magnitude_draw_both_teams_checked() {
        // Ends 2-2. Away was down 0-2; home was never behind.
        // → biggest comeback = 2 (away clawing back from 0-2 to draw 2-2).
        let events = vec![
            goal(500, 1, 0),
            goal(1000, 2, 0),
            goal(2500, 2, 1),
            goal(4000, 2, 2),
        ];
        assert_eq!(m6_comeback_magnitude(&events), 2);
    }

    // --- M7 tests ---

    #[test]
    fn m7_nervy_finish_level_at_90_pct() {
        // 1-1 at 90% tick mark (4860). Match ends 1-1 or 2-1 — doesn't matter,
        // what matters is the score AT the threshold.
        let events = vec![
            goal(1000, 1, 0),
            goal(3000, 1, 1), // 1-1 well before 90%
        ];
        assert!(m7_nervy_finish(&events, 5400));
    }

    #[test]
    fn m7_nervy_finish_one_goal_margin_at_90_pct() {
        // Home scores at tick 4800 (past 4860? — 4800 < 4860 → before threshold).
        // Score at threshold: 1-0. Margin = 1 → nervy.
        let events = vec![goal(4800, 1, 0)];
        // 90% of 5400 = 4860. Tick 4800 ≤ 4860, so included at threshold.
        assert!(m7_nervy_finish(&events, 5400));
    }

    #[test]
    fn m7_nervy_finish_blowout_not_nervy() {
        // Home leads 3-0 by tick 2000; no goals after that.
        // At 90% mark (4860) score is still 3-0 → margin 3 > 1 → not nervy.
        let events = vec![goal(500, 1, 0), goal(1000, 2, 0), goal(2000, 3, 0)];
        assert!(!m7_nervy_finish(&events, 5400));
    }

    #[test]
    fn m7_nervy_finish_scoreless_is_nervy() {
        // 0-0 at any tick → margin 0 → nervy.
        let events = vec![kickoff()];
        assert!(m7_nervy_finish(&events, 5400));
    }

    // --- M8 tests ---

    #[test]
    fn m8_key_moments_empty_stream() {
        let events: Vec<MatchEvent> = vec![];
        let km = m8_key_moments(&events);
        assert_eq!(km.goals, 0);
        assert_eq!(km.shots, 0);
        assert_eq!(km.signatures_fired, 0);
        assert_eq!(km.total(), 0);
    }

    #[test]
    fn m8_key_moments_counts_correctly() {
        // 2 goals, 3 shots, 1 signature.
        let events = vec![
            kickoff(),
            shot(500, true),
            goal(600, 1, 0),
            shot(1200, false),
            shot(2700, true),
            sig_fired(3000),
            goal(4500, 2, 0),
            fulltime(5400, 2, 0),
        ];
        let km = m8_key_moments(&events);
        assert_eq!(km.goals, 2);
        assert_eq!(km.shots, 3);
        assert_eq!(km.signatures_fired, 1);
        assert_eq!(km.total(), 6);
        // Verify: if formula is wrong (e.g. also counts KickOff/FullTime), total would be 8.
        assert_ne!(km.total(), events.len() as u32);
    }

    #[test]
    fn m8_on_target_count_correct() {
        let events = vec![
            shot(500, true),
            shot(1000, false),
            shot(2000, true),
            shot(3000, true),
            shot(4000, false),
        ];
        let (on_target, total) = m8_on_target_count(&events);
        assert_eq!(total, 5);
        assert_eq!(on_target, 3);
    }

    // --- match_end_tick_from_events ---

    #[test]
    fn match_end_tick_from_fulltime() {
        let events = vec![fulltime(5400, 2, 1)];
        assert_eq!(match_end_tick_from_events(&events), 5400);
    }

    #[test]
    fn match_end_tick_fallback_to_full_match_ticks() {
        // No FullTime → falls back to FULL_MATCH_TICKS.
        let events = vec![kickoff()];
        assert_eq!(
            match_end_tick_from_events(&events),
            fw_match_sim::FULL_MATCH_TICKS as i64
        );
    }
}

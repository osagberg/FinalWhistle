//! `MatchSnapshot` projection — `LiveMatchSession` → `MatchSnapshot` DTO.
//!
//! All projections here are read-only; they do not mutate the session.
//! `f64` arithmetic is allowed in this module — it sits inside `fw-tauri`,
//! which intentionally does NOT have `clippy::float_arithmetic = deny`.

use fw_content::MatchEvent;
use fw_match_sim::{PLAYERS_PER_TEAM, TOTAL_PLAYERS};

use super::session::LiveMatchSession;
use super::types::{
    BallZone, FinalMatchResult, LineupDto, MatchHandle, MatchPhase, MatchSnapshot, PossessionDto,
    SNAPSHOT_RECENT_EVENTS_CAP, ScoreDto,
};
use crate::result::MatchEventDto;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Ticks per game-minute. Matches the `TICKS_PER_GAME_MINUTE` const in
/// `crates/fw-tauri/src/result.rs` (both are 60). Defined here independently
/// to avoid a cross-module coupling; they must move together when T1-9
/// calibration pins the real conversion.
const TICKS_PER_GAME_MINUTE: u32 = 60;

/// Pitch half-length (metres from centre to goal-line). Q32 raw bits at
/// `GOAL_LINE_X` ≈ 52.5 m. We use `pos_x.to_bits()` (an `i64`) for zone
/// classification; the bucket boundaries below are `i64` fractions of a
/// Q32 metre. Zone math here is purely for DTO display — not canonical.
///
/// Q32.32 encoding: 1 metre = 1 << 32 raw bits = 4_294_967_296.
/// 21.0 m × 2^32 = 90_194_313_216. 7.0 m × 2^32 = 30_064_771_072.
///
/// These thresholds split the 105 m pitch into rough thirds for the 5-bucket
/// display model (ADR-0004 §3). The buckets from home's perspective:
///
/// | Zone                 | pos_x (m)           | raw bits (approx)          |
/// |----------------------|---------------------|----------------------------|
/// | OwnDefensiveThird    | x ≤ -21.0           | bits ≤ -90_194_313_216     |
/// | OwnMidThird          | -21.0 < x ≤ -7.0   | > -90_194 … ≤ -30_064 Gi  |
/// | Center               | -7.0 < x ≤  7.0    | > -30_064 … ≤  30_064 Gi  |
/// | OppMidThird          |  7.0 < x ≤  21.0   | >  30_064 … ≤  90_194 Gi  |
/// | OppAttackingThird    | x > 21.0            | bits >  90_194_313_216     |
const ZONE_BOUNDARY_INNER_BITS: i64 = 30_064_771_072;
const ZONE_BOUNDARY_OUTER_BITS: i64 = 90_194_313_216;

// ---------------------------------------------------------------------------
// Public projection entry points
// ---------------------------------------------------------------------------

/// Project `session` into a `MatchSnapshot` DTO.
///
/// Called from `get_match_snapshot_inner` under a read lock on
/// `AppState::live_matches`.
pub fn project_snapshot(session: &LiveMatchSession, handle: MatchHandle) -> MatchSnapshot {
    let tick = session.state.tick.to_raw().max(0) as u32;
    let minute = (tick / TICKS_PER_GAME_MINUTE).min(u16::MAX as u32) as u16;
    let phase = compute_phase(&session.state, session.state.match_events());
    let score = ScoreDto {
        home: session.state.home_score,
        away: session.state.away_score,
    };
    let possession_pct = compute_possession_pct(session.possession_ticks);
    let ball_zone = compute_ball_zone(session.state.ball.pos_x.to_bits());

    let home_lineup = LineupDto {
        players: (0..PLAYERS_PER_TEAM)
            .map(|s| session.state.players[s].slot as u32)
            .collect(),
    };
    let away_lineup = LineupDto {
        players: (PLAYERS_PER_TEAM..TOTAL_PLAYERS)
            .map(|s| session.state.players[s].slot as u32)
            .collect(),
    };

    // Recent events: last `SNAPSHOT_RECENT_EVENTS_CAP` in chronological order.
    let recent_events: Vec<MatchEventDto> = session
        .state
        .match_events()
        .iter()
        .rev()
        .take(SNAPSHOT_RECENT_EVENTS_CAP)
        .rev()
        .map(MatchEventDto::from_match_event)
        .collect();

    MatchSnapshot {
        handle,
        tick,
        minute,
        phase,
        score,
        possession_pct,
        ball_zone,
        home_lineup,
        away_lineup,
        recent_events,
        // T1: no card system.
        yellow_cards: std::collections::BTreeMap::new(),
        sent_off: std::collections::BTreeSet::new(),
    }
}

/// Project `session` into a `FinalMatchResult` DTO.
///
/// Called from `finish_live_match_inner` immediately before the session is
/// removed from `AppState::live_matches`.
pub fn project_final(session: &LiveMatchSession, handle: MatchHandle) -> FinalMatchResult {
    FinalMatchResult {
        handle,
        final_score: ScoreDto {
            home: session.state.home_score,
            away: session.state.away_score,
        },
        tick: session.state.tick.to_raw().max(0) as u32,
        total_events: session.state.match_events().len(),
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Derive `MatchPhase` from the accumulated event stream.
///
/// T1 match model: `FullTime` fires when `state.tick >= state.match_end_tick`.
/// `HalfTime` and `SecondHalf` transitions are not yet emitted by the T1 sim.
/// The phase bucketing degrades gracefully: if FullTime is in the events,
/// return `FullTime`; otherwise `FirstHalf`.
fn compute_phase(_state: &fw_match_sim::MatchState, events: &[MatchEvent]) -> MatchPhase {
    // Walk events in reverse (FullTime is late in the stream if present).
    let has_full_time = events
        .iter()
        .any(|e| matches!(e, MatchEvent::FullTime { .. }));
    if has_full_time {
        MatchPhase::FullTime
    } else {
        MatchPhase::FirstHalf
    }
}

/// Compute possession percentages from the running tally.
///
/// If no ticks have elapsed (both counters zero), return 50/50.
fn compute_possession_pct(ticks: [u32; 2]) -> PossessionDto {
    let total = ticks[0].saturating_add(ticks[1]);
    if total == 0 {
        return PossessionDto {
            home_pct: 50,
            away_pct: 50,
        };
    }
    // Integer percentage: avoid float by computing home as `home*100/total`
    // and away as the complement (avoids off-by-one rounding issues).
    let home_pct = ((ticks[0] as u64 * 100) / total as u64).min(100) as u8;
    let away_pct = 100 - home_pct;
    PossessionDto { home_pct, away_pct }
}

/// Classify ball `pos_x` raw bits into the 5-bucket `BallZone`.
///
/// Raw bits are signed Q32.32 representation. Positive X = attacking toward
/// away's goal; negative X = attacking toward home's goal. Zone boundaries
/// are documented at [`ZONE_BOUNDARY_INNER_BITS`] / [`ZONE_BOUNDARY_OUTER_BITS`].
fn compute_ball_zone(pos_x_bits: i64) -> BallZone {
    if pos_x_bits <= -ZONE_BOUNDARY_OUTER_BITS {
        BallZone::OwnDefensiveThird
    } else if pos_x_bits <= -ZONE_BOUNDARY_INNER_BITS {
        BallZone::OwnMidThird
    } else if pos_x_bits <= ZONE_BOUNDARY_INNER_BITS {
        BallZone::Center
    } else if pos_x_bits <= ZONE_BOUNDARY_OUTER_BITS {
        BallZone::OppMidThird
    } else {
        BallZone::OppAttackingThird
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn possession_pct_zero_ticks_returns_50_50() {
        let p = compute_possession_pct([0, 0]);
        assert_eq!(p.home_pct, 50);
        assert_eq!(p.away_pct, 50);
        assert_eq!(p.home_pct + p.away_pct, 100);
    }

    #[test]
    fn possession_pct_all_home_returns_100_0() {
        let p = compute_possession_pct([100, 0]);
        assert_eq!(p.home_pct, 100);
        assert_eq!(p.away_pct, 0);
    }

    #[test]
    fn possession_pct_all_away_returns_0_100() {
        let p = compute_possession_pct([0, 100]);
        assert_eq!(p.home_pct, 0);
        assert_eq!(p.away_pct, 100);
    }

    #[test]
    fn possession_pct_sums_to_100_for_arbitrary_split() {
        // 3:1 split → 75% home.
        let p = compute_possession_pct([75, 25]);
        assert_eq!(p.home_pct, 75);
        assert_eq!(p.away_pct, 25);
        assert_eq!(p.home_pct + p.away_pct, 100);
    }

    #[test]
    fn ball_zone_centre_returns_center() {
        assert_eq!(compute_ball_zone(0), BallZone::Center);
    }

    #[test]
    fn ball_zone_far_negative_returns_own_defensive_third() {
        // pos_x = -30.0 m → bits = -30 × 2^32 = -128_849_018_880 < -90_194_313_216
        let bits = -30_i64 * (1_i64 << 32);
        assert_eq!(compute_ball_zone(bits), BallZone::OwnDefensiveThird);
    }

    #[test]
    fn ball_zone_far_positive_returns_opp_attacking_third() {
        let bits = 30_i64 * (1_i64 << 32);
        assert_eq!(compute_ball_zone(bits), BallZone::OppAttackingThird);
    }

    #[test]
    fn ball_zone_boundary_inner_negative_returns_own_mid_third() {
        // Exactly at -21.0 m boundary.
        assert_eq!(
            compute_ball_zone(-ZONE_BOUNDARY_OUTER_BITS),
            BallZone::OwnDefensiveThird
        );
        // Just inside the OwnMidThird bucket.
        assert_eq!(
            compute_ball_zone(-ZONE_BOUNDARY_OUTER_BITS + 1),
            BallZone::OwnMidThird
        );
    }

    #[test]
    fn ball_zone_boundary_inner_positive_returns_opp_mid_third() {
        assert_eq!(
            compute_ball_zone(ZONE_BOUNDARY_OUTER_BITS),
            BallZone::OppMidThird
        );
        assert_eq!(
            compute_ball_zone(ZONE_BOUNDARY_OUTER_BITS + 1),
            BallZone::OppAttackingThird
        );
    }
}

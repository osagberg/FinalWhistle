// The detector helpers use f64 arithmetic for pitch-coordinate math (distances,
// segment projections). This binary is a VIEWER-SIDE analysis tool: its outputs
// are never fed back into canonical state. The crate-wide float_arithmetic deny
// is correct for the lib; here we allow it intentionally, same pattern as dto.rs.
#![allow(clippy::float_arithmetic)]
//! `inspect_frames` — deterministic match-quality glitch-detector.
//!
//! Reads a `dump_frames` JSON file (or stdin) and runs 7 glitch-detectors
//! over consecutive `MatchFrameDto` frames. Emits a structured JSON report
//! + human-readable summary to stdout/stderr.
//!
//! ## Usage
//!
//! ```sh
//! # From a file:
//! cargo run -p fw-match-sim --bin inspect_frames -- /tmp/dx2-frames.json
//!
//! # From stdin:
//! cargo run -p fw-match-sim --bin dump_frames -- --seed 0xfeedbeef --ticks 5400 \
//!     --content content | \
//!   cargo run -p fw-match-sim --bin inspect_frames
//!
//! # Compact JSON only:
//! cargo run -p fw-match-sim --bin inspect_frames -- /tmp/dx2.json --json-only
//! ```
//!
//! ## Detectors (v1)
//!
//! All thresholds are derived from engine physical caps (dt = 1/60 s)
//! unless marked as imported from canonical sim constants:
//!
//! 1. **BallTeleport** — `|Δball_pos| > MAX_BALL_TRAVEL_PER_TICK` (35 m/s peak shot
//!    speed × dt = 35/60 ≈ 0.583 m; threshold = 1.0 m for headroom).
//!    [derived from physical cap — no sim constant]
//! 2. **BallPhasingPlayer** — swept ball segment `[pos(t-1)→pos(t)]` within
//!    `PLAYER_RADIUS` (0.5 m) of a player, but no possession change on that tick.
//!    Possession-change is used as a coarse proxy: when possession changes,
//!    the possessor-pair slots are excluded from the check (likely legitimate
//!    contact); all other players are still checked. This is a **LOWER BOUND**
//!    — the DTO carries no touch/contact event data, so genuine phasing events
//!    that happen to coincide with a possession change are undercounted.
//!    [PLAYER_RADIUS = 0.5 m derived from physical reality]
//! 3. **PhantomGoal** — score changes at tick `t` but ball never crossed
//!    `GOAL_LINE_X` in a ±10-tick window.
//!    [GOAL_LINE_X imported from `fw_core::GOAL_LINE_X`]
//! 4. **PersistentPlayerOverlap** — two players closer than `MIN_PLAYER_DISTANCE`
//!    (0.4 m) for more than `K = 5` consecutive ticks.
//!    [MIN_PLAYER_DISTANCE imported from `fw_match_sim::separation::MIN_PLAYER_DISTANCE`]
//! 5. **ImpossiblePlayerVelocity** — player position delta `> MAX_PLAYER_TRAVEL_PER_TICK`
//!    (8 m/s × dt = 8/60 ≈ 0.133 m; threshold = 0.15 m for headroom).
//!    [derived from physical cap — no sim constant]
//! 6. **BallOffPitch** — ball outside `[±SIDELINE_Y, ±GOAL_LINE_X]` for > 5 ticks.
//!    [SIDELINE_Y, GOAL_LINE_X imported from `fw_core`]
//! 7. **Stall** — ball + all players move < `STALL_THRESHOLD` per entity per tick
//!    for `STALL_WINDOW = 60` consecutive ticks while in play (score not changing).
//!    [derived — no sim constant]
//!
//! ## Match-length facts (documented here, DX-2 deliverable)
//!
//! A full match is **5400 ticks** (90 minutes × 60 ticks/minute, dt = 1/60 s).
//! `tick_to_minute(t) = t / 60` — integer minutes, same as `t as f64 / 60.0`.
//! `FULL_MATCH_TICKS = 5400` is the canonical constant in `fw_match_sim::FULL_MATCH_TICKS`.
//! FullTime fires at `tick >= match_end_tick` (default 5400).
//!
//! ## Trust limitations
//!
//! - BallPhasingPlayer is a LOWER BOUND (see detector 2 above).
//! - An `evaluable_frame_pairs` field in the report records how many `windows(2)` were
//!   evaluated; a `warnings` list records structural problems (too few frames, wrong
//!   player count). Non-empty `warnings` means the report is INCOMPLETE — do not key
//!   automated decisions on a zero-flag result without verifying `warnings` is empty.

use std::io::{self, Read};
use std::path::PathBuf;

use clap::Parser;
use fw_match_sim::MatchFrameDto;
use serde::Serialize;

// ---------------------------------------------------------------------------
// CLI
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(
    name = "inspect_frames",
    about = "Run 7 glitch-detectors over a dump_frames JSON file and emit a structured report."
)]
struct Cli {
    /// Path to the dump_frames JSON file. If omitted, reads from stdin.
    input: Option<PathBuf>,

    /// Emit JSON only (no human-readable summary). Useful for piping.
    #[arg(long, default_value_t = false)]
    json_only: bool,

    /// Compact (non-pretty) JSON output.
    #[arg(long, default_value_t = false)]
    compact: bool,
}

// ---------------------------------------------------------------------------
// Constants: imported from canonical sources where available, derived from
// physical caps where no sim constant exists.
//
// Q32 → f64 projection: raw_bits / 2^32 (same formula as dto.rs::q32_to_f64).
// ---------------------------------------------------------------------------

const Q32_SCALE: f64 = 4_294_967_296.0; // 2^32

/// Min player-player distance. IMPORTED: fw_match_sim::separation::MIN_PLAYER_DISTANCE.
/// Q32 raw bits 1_717_986_918 → 1_717_986_918 / 2^32 ≈ 0.4 m.
const MIN_PLAYER_DISTANCE: f64 =
    fw_match_sim::separation::MIN_PLAYER_DISTANCE.to_bits() as f64 / Q32_SCALE;

/// Pitch half-length = GOAL_LINE_X. IMPORTED: fw_core::GOAL_LINE_X.
/// Q32 raw bits = PITCH_LENGTH_M.bits >> 1 = (105 << 32) >> 1 = 52.5 × 2^32.
const GOAL_LINE_X: f64 = fw_core::GOAL_LINE_X.to_bits() as f64 / Q32_SCALE;

/// Pitch half-width = SIDELINE_Y. IMPORTED: fw_core::SIDELINE_Y.
/// Q32 raw bits = PITCH_WIDTH_M.bits >> 1 = (68 << 32) >> 1 = 34 × 2^32.
const SIDELINE_Y: f64 = fw_core::SIDELINE_Y.to_bits() as f64 / Q32_SCALE;

/// Goal half-width. IMPORTED: fw_content::event::GOAL_HALF_WIDTH_M.
/// Q32 raw bits 15_720_299_520 → ≈ 3.66 m.
const GOAL_HALF_WIDTH: f64 = fw_content::event::GOAL_HALF_WIDTH_M.to_bits() as f64 / Q32_SCALE;

/// Full match ticks. IMPORTED: fw_match_sim::FULL_MATCH_TICKS.
const FULL_MATCH_TICKS: u32 = fw_match_sim::FULL_MATCH_TICKS;

// Physical-cap-derived thresholds — no sim constant exists for these.

/// Max ball travel per tick: peak shot speed (35 m/s) × dt (1/60 s) = 0.583 m.
/// Threshold is 1.0 m for headroom; 1.0/0.583 ≈ 1.7× over the physical cap.
const MAX_BALL_TRAVEL_PER_TICK: f64 = 1.0; // metres  [physical cap]

/// Player radius for phasing detection: approximate body radius for contact proxy.
/// 0.5 m is larger than a real body (~0.3 m) to catch near-misses as well.
const PLAYER_RADIUS: f64 = 0.5; // metres  [physical cap]

/// Max player travel per tick: max player speed (8 m/s) × dt (1/60 s) = 0.133 m.
/// Threshold is 0.15 m for headroom.
const MAX_PLAYER_TRAVEL_PER_TICK: f64 = 0.15; // metres  [physical cap]

/// Persistent-overlap trigger: flag after K consecutive ticks.
const OVERLAP_K_TICKS: u32 = 5;

/// Window (ticks) around a score-change to look for a goal-line crossing.
/// ±10 ticks = ±0.167 s around the scored tick.
const PHANTOM_GOAL_WINDOW: i64 = 10;

/// Ball-off-pitch persistence trigger (ticks before flagging).
const OFF_PITCH_K_TICKS: u32 = 5;

/// Stall detection window (ticks).  [physical cap]
const STALL_WINDOW: usize = 60;

/// Per-entity motion threshold for the stall detector.  [physical cap]
/// 0.01 m × 60 ticks = 0.6 m total motion per entity over the window.
const STALL_MOTION_THRESHOLD: f64 = 0.01; // metres per tick, per entity

// ---------------------------------------------------------------------------
// Flag types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Flag {
    pub tick: i64,
    pub detector: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DetectorResult {
    pub detector: String,
    pub flag_count: usize,
    /// First offending flag, if any.
    pub first_flag: Option<Flag>,
}

/// Status of the report — whether all frames were evaluable.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReportStatus {
    /// All frames evaluated; results are complete.
    Ok,
    /// Structural problems found; results are incomplete. Check `warnings`.
    Incomplete,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectReport {
    pub seed_hex: String,
    /// `"OK"` or `"INCOMPLETE"`. If INCOMPLETE, `warnings` is non-empty and
    /// any zero-flag result MUST NOT be interpreted as a clean match.
    pub status: ReportStatus,
    pub total_frames: usize,
    /// Number of consecutive-frame pairs evaluated (= `total_frames - 1` when
    /// the input is well-formed and dense). Detectors that use `windows(2)` run
    /// over exactly this many pairs; zero means no evaluation occurred.
    pub evaluable_frame_pairs: usize,
    /// Full match length in ticks. IMPORTED from `fw_match_sim::FULL_MATCH_TICKS`.
    pub full_match_ticks: u32,
    /// Tick → minute mapping: minute = tick / 60.
    pub tick_to_minute_formula: String,
    /// Structural warnings. Non-empty means `status = INCOMPLETE`.
    pub warnings: Vec<String>,
    pub detectors: Vec<DetectorResult>,
    /// Total flags across all detectors.
    pub total_flags: usize,
}

// ---------------------------------------------------------------------------
// Input validation
// ---------------------------------------------------------------------------

/// Validate the frame list for structural completeness. Returns a list of
/// warning strings; empty = evaluable. Also returns the evaluable_frame_pairs
/// count (how many windows(2) the detectors will see).
///
/// Checks:
/// 1. `total_frames < 2` → cannot run any windowed detector.
/// 2. Any frame has `players.len() != 22` → player-count invariant violated.
/// 3. Frames are dense and contiguous: `frames[i].tick == frames[0].tick + i as i64`.
///    If not dense, PhantomGoal's tick-window lookup and PersistentOverlap's
///    consecutive-tick counter are wrong. Fail loud rather than silently produce
///    misleading counts.
fn validate_frames(frames: &[MatchFrameDto]) -> (Vec<String>, usize) {
    let mut warnings = Vec::new();

    if frames.len() < 2 {
        warnings.push(format!(
            "only {} frame(s) — need ≥2 for any detector to run; \
             results are an all-clear that means nothing",
            frames.len()
        ));
        return (warnings, 0);
    }

    // Player-count check.
    for (i, f) in frames.iter().enumerate() {
        if f.players.len() != 22 {
            warnings.push(format!(
                "frame[{}] (tick {}) has {} players, expected 22; \
                 player-based detectors are unreliable",
                i,
                f.tick,
                f.players.len()
            ));
        }
    }

    // Dense/contiguous-tick check.
    let base_tick = frames[0].tick;
    for (i, f) in frames.iter().enumerate() {
        let expected = base_tick + i as i64;
        if f.tick != expected {
            warnings.push(format!(
                "frames are not dense/contiguous: frames[{}].tick={} expected {}; \
                 PhantomGoal and PersistentOverlap tick-window logic is unreliable on sparse input",
                i, f.tick, expected
            ));
            // One warning is enough to convey the problem — don't flood with one per frame.
            break;
        }
    }

    let pairs = frames.len() - 1;
    (warnings, pairs)
}

// ---------------------------------------------------------------------------
// Detector implementations (pure functions over frame slices)
// ---------------------------------------------------------------------------

/// Euclidean distance between two 2D points.
#[inline]
fn dist2d(ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = ax - bx;
    let dy = ay - by;
    (dx * dx + dy * dy).sqrt()
}

/// Point-to-segment distance (2D). Returns the minimum distance from point P
/// to the line segment [A, B].
fn point_to_segment_dist(px: f64, py: f64, ax: f64, ay: f64, bx: f64, by: f64) -> f64 {
    let dx = bx - ax;
    let dy = by - ay;
    let len_sq = dx * dx + dy * dy;
    if len_sq < 1e-12 {
        // Degenerate segment (A == B).
        return dist2d(px, py, ax, ay);
    }
    let t = ((px - ax) * dx + (py - ay) * dy) / len_sq;
    let t_clamped = t.clamp(0.0, 1.0);
    let closest_x = ax + t_clamped * dx;
    let closest_y = ay + t_clamped * dy;
    dist2d(px, py, closest_x, closest_y)
}

/// D1: Ball teleport — |Δpos| > MAX_BALL_TRAVEL_PER_TICK.
fn detect_ball_teleport(frames: &[MatchFrameDto]) -> Vec<Flag> {
    let mut flags = Vec::new();
    for w in frames.windows(2) {
        let (prev, curr) = (&w[0], &w[1]);
        let d = dist2d(
            curr.ball.pos_x,
            curr.ball.pos_y,
            prev.ball.pos_x,
            prev.ball.pos_y,
        );
        if d > MAX_BALL_TRAVEL_PER_TICK {
            flags.push(Flag {
                tick: curr.tick,
                detector: "BallTeleport".to_string(),
                detail: format!(
                    "ball jumped {d:.3}m in one tick (threshold={MAX_BALL_TRAVEL_PER_TICK}m); \
                     prev=({:.2},{:.2}) curr=({:.2},{:.2})",
                    prev.ball.pos_x, prev.ball.pos_y, curr.ball.pos_x, curr.ball.pos_y
                ),
            });
        }
    }
    flags
}

/// D2: Ball phasing through a player — swept ball segment passes within
/// PLAYER_RADIUS of a player, but that player was not involved in the
/// possession change this tick (if any).
///
/// Fix vs v1: previously skipped the ENTIRE frame when possession changed,
/// turning the detector blind exactly when the ball was near a player.
/// Now: when possession changes from slot A to slot B, we exclude only
/// those two slots from the check; all other ~20 players are still evaluated.
///
/// Limitation: possession delta is a coarse proxy for a contact event.
/// The DTO has no touch/interaction event data, so phasing events that
/// happen to coincide with a possession change are still undercounted.
/// This detector is a LOWER BOUND; the `warnings` field records this.
fn detect_ball_phasing(frames: &[MatchFrameDto]) -> Vec<Flag> {
    let mut flags = Vec::new();
    for w in frames.windows(2) {
        let (prev, curr) = (&w[0], &w[1]);

        // Build the set of slots excluded from phasing checks this tick.
        // If possession changed, the prev-possessor and curr-possessor are
        // the most likely legitimate-contact participants — exclude them.
        // All other players are still checked.
        let mut excluded: [bool; 22] = [false; 22];
        if prev.possession != curr.possession {
            if let Some(prev_slot) = prev.possession
                && (prev_slot as usize) < 22
            {
                excluded[prev_slot as usize] = true;
            }
            if let Some(curr_slot) = curr.possession
                && (curr_slot as usize) < 22
            {
                excluded[curr_slot as usize] = true;
            }
        }

        // Check each player that was not involved in the possession change.
        for player in &curr.players {
            let slot_idx = player.slot as usize;
            if slot_idx < 22 && excluded[slot_idx] {
                continue;
            }
            let d = point_to_segment_dist(
                player.pos_x,
                player.pos_y,
                prev.ball.pos_x,
                prev.ball.pos_y,
                curr.ball.pos_x,
                curr.ball.pos_y,
            );
            if d < PLAYER_RADIUS {
                flags.push(Flag {
                    tick: curr.tick,
                    detector: "BallPhasingPlayer".to_string(),
                    detail: format!(
                        "ball swept segment passed within {d:.3}m of slot {} \
                         (radius={PLAYER_RADIUS}m); \
                         possession change this tick: {:?}→{:?} (those slots excluded)",
                        player.slot, prev.possession, curr.possession
                    ),
                });
            }
        }
    }
    flags
}

/// D3: Phantom goal — score changes but ball never crossed goal-line in window.
///
/// Precondition: frames must be dense (validated up-front). The tick-window
/// lookup `frames.iter().filter(|f| f.tick >= window_start ...)` relies on
/// `frames[i].tick == i + base_tick` for correct results.
fn detect_phantom_goal(frames: &[MatchFrameDto]) -> Vec<Flag> {
    let mut flags = Vec::new();
    let n = frames.len() as i64;
    let base_tick = frames.first().map(|f| f.tick).unwrap_or(0);
    for w in frames.windows(2) {
        let (prev, curr) = (&w[0], &w[1]);
        let score_changed =
            (curr.home_score != prev.home_score) || (curr.away_score != prev.away_score);
        if !score_changed {
            continue;
        }
        // Window is symmetric around the scoring tick. Index by position since
        // we validated frames are dense, so tick - base_tick == array index.
        let window_start = (curr.tick - PHANTOM_GOAL_WINDOW).max(base_tick);
        let window_end = (curr.tick + PHANTOM_GOAL_WINDOW).min(base_tick + n - 1);
        let start_idx = (window_start - base_tick) as usize;
        let end_idx = ((window_end - base_tick) as usize).min(frames.len() - 1);

        let ball_crossed = frames[start_idx..=end_idx]
            .iter()
            .any(|f| f.ball.pos_x.abs() >= GOAL_LINE_X && f.ball.pos_y.abs() < GOAL_HALF_WIDTH);

        if !ball_crossed {
            flags.push(Flag {
                tick: curr.tick,
                detector: "PhantomGoal".to_string(),
                detail: format!(
                    "score changed {}-{} → {}-{} at tick {} \
                     but ball never crossed goal-line \
                     (|pos_x|≥{GOAL_LINE_X:.2}m ∧ |pos_y|<{GOAL_HALF_WIDTH:.2}m) \
                     in ±{PHANTOM_GOAL_WINDOW}-tick window; \
                     ball at ({:.2},{:.2}) at scoring tick",
                    prev.home_score,
                    prev.away_score,
                    curr.home_score,
                    curr.away_score,
                    curr.tick,
                    curr.ball.pos_x,
                    curr.ball.pos_y
                ),
            });
        }
    }
    flags
}

/// D4: Persistent player overlap — two players closer than MIN_PLAYER_DISTANCE
/// for more than K consecutive ticks.
///
/// The `reported` set is keyed on `(slot_i, slot_j)` (player slot numbers)
/// rather than array indices, so it is stable across frames even if player
/// ordering ever changes.
fn detect_persistent_overlap(frames: &[MatchFrameDto]) -> Vec<Flag> {
    let n_players = if frames.is_empty() {
        0
    } else {
        frames[0].players.len()
    };
    if n_players < 2 {
        return Vec::new();
    }
    // consec is keyed by (array-index-i, array-index-j). Because slot order is
    // stable within a match, this is equivalent to (slot_i, slot_j).
    let pair_count = n_players * (n_players - 1) / 2;
    let mut consec: Vec<u32> = vec![0; pair_count];
    let mut flags = Vec::new();
    // reported is keyed on SLOT numbers (not array indices) so it's semantically
    // stable even if player order were to shift.
    let mut reported: std::collections::BTreeSet<(u8, u8)> = Default::default();

    let pair_idx = |i: usize, j: usize| -> usize {
        // i < j guaranteed by caller.
        i * n_players - i * (i + 1) / 2 + j - i - 1
    };

    for frame in frames {
        for i in 0..n_players {
            for j in (i + 1)..n_players {
                let pi = &frame.players[i];
                let pj = &frame.players[j];
                let d = dist2d(pi.pos_x, pi.pos_y, pj.pos_x, pj.pos_y);
                let idx = pair_idx(i, j);
                // Key by slot numbers, not array indices.
                let slot_pair = (pi.slot.min(pj.slot), pi.slot.max(pj.slot));
                if d < MIN_PLAYER_DISTANCE {
                    consec[idx] += 1;
                    if consec[idx] == OVERLAP_K_TICKS && !reported.contains(&slot_pair) {
                        reported.insert(slot_pair);
                        flags.push(Flag {
                            tick: frame.tick,
                            detector: "PersistentPlayerOverlap".to_string(),
                            detail: format!(
                                "slots {} and {} closer than {MIN_PLAYER_DISTANCE:.3}m \
                                 for {OVERLAP_K_TICKS}+ consecutive ticks; \
                                 current dist={d:.3}m",
                                pi.slot, pj.slot
                            ),
                        });
                    }
                } else {
                    consec[idx] = 0;
                    // Allow re-reporting on a new overlap run.
                    reported.remove(&slot_pair);
                }
            }
        }
    }
    flags
}

/// D5: Impossible player velocity — position delta > MAX_PLAYER_TRAVEL_PER_TICK.
fn detect_impossible_player_velocity(frames: &[MatchFrameDto]) -> Vec<Flag> {
    let mut flags = Vec::new();
    for w in frames.windows(2) {
        let (prev, curr) = (&w[0], &w[1]);
        for (pp, cp) in prev.players.iter().zip(curr.players.iter()) {
            let d = dist2d(pp.pos_x, pp.pos_y, cp.pos_x, cp.pos_y);
            if d > MAX_PLAYER_TRAVEL_PER_TICK {
                flags.push(Flag {
                    tick: curr.tick,
                    detector: "ImpossiblePlayerVelocity".to_string(),
                    detail: format!(
                        "slot {} moved {d:.3}m in one tick \
                         (threshold={MAX_PLAYER_TRAVEL_PER_TICK}m = 8m/s×dt); \
                         prev=({:.2},{:.2}) curr=({:.2},{:.2})",
                        cp.slot, pp.pos_x, pp.pos_y, cp.pos_x, cp.pos_y
                    ),
                });
            }
        }
    }
    flags
}

/// D6: Ball off-pitch — ball outside pitch bounds for > K ticks.
fn detect_ball_off_pitch(frames: &[MatchFrameDto]) -> Vec<Flag> {
    let mut flags = Vec::new();
    let mut consec_oob: u32 = 0;
    let mut flagged_run = false;

    for frame in frames {
        let off = frame.ball.pos_x.abs() > GOAL_LINE_X || frame.ball.pos_y.abs() > SIDELINE_Y;
        if off {
            consec_oob += 1;
            if consec_oob == OFF_PITCH_K_TICKS && !flagged_run {
                flagged_run = true;
                flags.push(Flag {
                    tick: frame.tick,
                    detector: "BallOffPitch".to_string(),
                    detail: format!(
                        "ball off pitch for {OFF_PITCH_K_TICKS}+ consecutive ticks; \
                         pos=({:.2},{:.2}) \
                         pitch=[±{GOAL_LINE_X:.2}m × ±{SIDELINE_Y:.2}m]",
                        frame.ball.pos_x, frame.ball.pos_y
                    ),
                });
            }
        } else {
            consec_oob = 0;
            flagged_run = false;
        }
    }
    flags
}

/// D7: Stall — ball + players effectively frozen for STALL_WINDOW ticks.
fn detect_stall(frames: &[MatchFrameDto]) -> Vec<Flag> {
    if frames.len() < STALL_WINDOW + 1 {
        return Vec::new();
    }
    let mut flags = Vec::new();
    let mut i = 0;
    while i + STALL_WINDOW < frames.len() {
        let window = &frames[i..i + STALL_WINDOW + 1];
        let start_score = (window[0].home_score, window[0].away_score);
        let end_score = (
            window[STALL_WINDOW].home_score,
            window[STALL_WINDOW].away_score,
        );
        // Only flag stalls during live play (no score change in window).
        if start_score != end_score {
            i += 1;
            continue;
        }
        // Compute total ball motion over window.
        let ball_motion: f64 = window
            .windows(2)
            .map(|w| {
                dist2d(
                    w[1].ball.pos_x,
                    w[1].ball.pos_y,
                    w[0].ball.pos_x,
                    w[0].ball.pos_y,
                )
            })
            .sum::<f64>();
        let ball_avg = ball_motion / STALL_WINDOW as f64;

        // Compute average player motion (sum over all players / (n_players × window)).
        let n_players = window[0].players.len() as f64;
        let player_total_motion: f64 = window
            .windows(2)
            .map(|w| {
                w[0].players
                    .iter()
                    .zip(w[1].players.iter())
                    .map(|(pp, cp)| dist2d(pp.pos_x, pp.pos_y, cp.pos_x, cp.pos_y))
                    .sum::<f64>()
            })
            .sum::<f64>();
        let player_avg = player_total_motion / (n_players * STALL_WINDOW as f64);

        if ball_avg < STALL_MOTION_THRESHOLD && player_avg < STALL_MOTION_THRESHOLD {
            flags.push(Flag {
                tick: window[0].tick,
                detector: "Stall".to_string(),
                detail: format!(
                    "ball+players frozen for {STALL_WINDOW} ticks starting at tick {}; \
                     ball_avg_motion={ball_avg:.4}m/tick, \
                     player_avg_motion={player_avg:.4}m/tick \
                     (threshold={STALL_MOTION_THRESHOLD}m/tick)",
                    window[0].tick
                ),
            });
            i += STALL_WINDOW; // Skip the stall window to avoid re-flagging.
        } else {
            i += 1;
        }
    }
    flags
}

// ---------------------------------------------------------------------------
// Report assembly
// ---------------------------------------------------------------------------

fn run_all_detectors(frames: &[MatchFrameDto]) -> InspectReport {
    let seed_hex = frames
        .first()
        .map(|f| f.seed_hex.clone())
        .unwrap_or_else(|| "unknown".to_string());

    let (mut warnings, evaluable_frame_pairs) = validate_frames(frames);

    // Add the standing lower-bound caveat for BallPhasingPlayer so consumers
    // always know this number is an undercount.
    warnings.push(
        "BallPhasingPlayer is a LOWER BOUND: the DTO has no touch/contact events; \
         possession-delta is used as a coarse proxy. Phasing events that coincide \
         with a possession change are still undercounted even with the per-slot exclusion."
            .to_string(),
    );

    let status = if warnings
        .iter()
        .any(|w| w.contains("frame(s)") || w.contains("players") || w.contains("dense"))
    {
        ReportStatus::Incomplete
    } else {
        ReportStatus::Ok
    };

    let mut all_detectors = Vec::new();

    let run = |name: &str, flags: Vec<Flag>| -> DetectorResult {
        let first_flag = flags.first().cloned();
        DetectorResult {
            detector: name.to_string(),
            flag_count: flags.len(),
            first_flag,
        }
    };

    all_detectors.push(run("BallTeleport", detect_ball_teleport(frames)));
    all_detectors.push(run("BallPhasingPlayer", detect_ball_phasing(frames)));
    all_detectors.push(run("PhantomGoal", detect_phantom_goal(frames)));
    all_detectors.push(run(
        "PersistentPlayerOverlap",
        detect_persistent_overlap(frames),
    ));
    all_detectors.push(run(
        "ImpossiblePlayerVelocity",
        detect_impossible_player_velocity(frames),
    ));
    all_detectors.push(run("BallOffPitch", detect_ball_off_pitch(frames)));
    all_detectors.push(run("Stall", detect_stall(frames)));

    let total_flags: usize = all_detectors.iter().map(|d| d.flag_count).sum();

    InspectReport {
        seed_hex,
        status,
        total_frames: frames.len(),
        evaluable_frame_pairs,
        full_match_ticks: FULL_MATCH_TICKS,
        tick_to_minute_formula: "minute = tick / 60  (integer division; 5400 ticks = 90 min)"
            .to_string(),
        warnings,
        detectors: all_detectors,
        total_flags,
    }
}

fn print_human_summary(report: &InspectReport) {
    eprintln!("=== inspect_frames report ===");
    eprintln!(
        "Seed: {}  Frames: {}  Evaluable pairs: {}  Full-match ticks: {}",
        report.seed_hex, report.total_frames, report.evaluable_frame_pairs, report.full_match_ticks
    );
    eprintln!("Tick→minute: {}", report.tick_to_minute_formula);
    eprintln!("Status: {:?}", report.status);
    if !report.warnings.is_empty() {
        eprintln!("Warnings:");
        for w in &report.warnings {
            eprintln!("  ! {w}");
        }
    }
    eprintln!("Total flags: {}", report.total_flags);
    eprintln!();
    for d in &report.detectors {
        let status_str = if d.flag_count == 0 { "OK" } else { "FLAGGED" };
        eprintln!(
            "  [{status_str:7}] {:30}  count={}",
            d.detector, d.flag_count
        );
        if let Some(f) = &d.first_flag {
            eprintln!(
                "             first at tick {} (min {:.1}): {}",
                f.tick,
                f.tick as f64 / 60.0,
                f.detail
            );
        }
    }
    eprintln!();
}

// ---------------------------------------------------------------------------
// Entry point
// ---------------------------------------------------------------------------

fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match run(&cli) {
        Ok(report_status) => {
            if report_status == ReportStatus::Incomplete {
                // Non-zero exit when input was too short/malformed to evaluate.
                // Prevents automation from treating a truncated input as "clean".
                std::process::ExitCode::from(2)
            } else {
                std::process::ExitCode::SUCCESS
            }
        }
        Err(e) => {
            eprintln!("inspect_frames: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

fn run(cli: &Cli) -> Result<ReportStatus, String> {
    let json_str = if let Some(path) = &cli.input {
        std::fs::read_to_string(path).map_err(|e| format!("failed to read {:?}: {e}", path))?
    } else {
        let mut s = String::new();
        io::stdin()
            .read_to_string(&mut s)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        s
    };

    let frames: Vec<MatchFrameDto> =
        serde_json::from_str(&json_str).map_err(|e| format!("JSON parse error: {e}"))?;

    if frames.is_empty() {
        return Err("empty frame list — nothing to evaluate".to_string());
    }

    let report = run_all_detectors(&frames);
    let status = report.status.clone();

    if !cli.json_only {
        print_human_summary(&report);
    }

    let json = if cli.compact {
        serde_json::to_string(&report).map_err(|e| format!("JSON encode: {e}"))?
    } else {
        serde_json::to_string_pretty(&report).map_err(|e| format!("JSON encode: {e}"))?
    };
    println!("{json}");

    Ok(status)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use fw_match_sim::{BallFrameDto, MatchFrameDto, PlayerFrameDto};

    fn make_frame(tick: i64, bx: f64, by: f64, home_score: u8, away_score: u8) -> MatchFrameDto {
        MatchFrameDto {
            seed_hex: "0xtest".to_string(),
            tick,
            home_score,
            away_score,
            players: (0u8..22u8)
                .map(|slot| PlayerFrameDto {
                    slot,
                    pos_x: 0.0,
                    pos_y: slot as f64 * 2.0,
                    vel_x: 0.0,
                    vel_y: 0.0,
                })
                .collect(),
            ball: BallFrameDto {
                pos_x: bx,
                pos_y: by,
                pos_z: 0.0,
                vel_x: 0.0,
                vel_y: 0.0,
                vel_z: 0.0,
            },
            possession: Some(9),
        }
    }

    #[test]
    fn ball_teleport_fires_on_large_jump() {
        let frames = vec![
            make_frame(0, 0.0, 0.0, 0, 0),
            make_frame(1, 2.0, 0.0, 0, 0), // 2m jump > 1.0m threshold
        ];
        let flags = detect_ball_teleport(&frames);
        assert_eq!(flags.len(), 1);
        assert_eq!(flags[0].tick, 1);
    }

    #[test]
    fn ball_teleport_silent_on_normal_move() {
        let frames = vec![
            make_frame(0, 0.0, 0.0, 0, 0),
            make_frame(1, 0.4, 0.0, 0, 0), // 0.4m < 1.0m threshold
        ];
        let flags = detect_ball_teleport(&frames);
        assert!(flags.is_empty(), "should not flag normal ball movement");
    }

    #[test]
    fn phantom_goal_fires_when_score_changes_without_ball_crossing() {
        // Score changes at tick 5, but ball stays at (0,0) throughout.
        let frames: Vec<MatchFrameDto> = (0..20)
            .map(|t| make_frame(t, 0.0, 0.0, if t < 5 { 0 } else { 1 }, 0))
            .collect();
        let flags = detect_phantom_goal(&frames);
        assert!(!flags.is_empty(), "should flag phantom goal");
        assert_eq!(flags[0].detector, "PhantomGoal");
    }

    #[test]
    fn phantom_goal_silent_when_ball_crossed_line() {
        // Score changes at tick 5; ball is at pos_x = 53.0 at tick 5 (crossed).
        let frames: Vec<MatchFrameDto> = (0..20)
            .map(|t| {
                let bx = if (3..=7).contains(&t) { 53.0 } else { 0.0 };
                make_frame(t, bx, 0.0, if t < 5 { 0 } else { 1 }, 0)
            })
            .collect();
        let flags = detect_phantom_goal(&frames);
        assert!(
            flags.is_empty(),
            "should not flag when ball crossed goal-line: {flags:?}"
        );
    }

    #[test]
    fn impossible_velocity_fires_on_large_position_delta() {
        let f0 = make_frame(0, 0.0, 0.0, 0, 0);
        let mut f1 = make_frame(1, 0.0, 0.0, 0, 0);
        // Move player slot 5 by 0.5m (> 0.15m threshold).
        f1.players[5].pos_x = 0.5;
        let frames = vec![f0, f1];
        let flags = detect_impossible_player_velocity(&frames);
        assert!(!flags.is_empty(), "should flag impossible velocity");
        assert_eq!(flags[0].detector, "ImpossiblePlayerVelocity");
    }

    #[test]
    fn persistent_overlap_fires_after_k_ticks() {
        // Two players at the same position for OVERLAP_K_TICKS + 1 ticks.
        let frames: Vec<MatchFrameDto> = (0..10)
            .map(|t| {
                let mut f = make_frame(t, 0.0, 0.0, 0, 0);
                // Slot 0 and slot 1 both at (0, 0).
                f.players[0].pos_y = 0.0;
                f.players[1].pos_y = 0.0;
                f
            })
            .collect();
        let flags = detect_persistent_overlap(&frames);
        assert!(!flags.is_empty(), "should flag persistent overlap");
        assert_eq!(flags[0].detector, "PersistentPlayerOverlap");
    }

    #[test]
    fn ball_off_pitch_fires_after_k_ticks() {
        // Ball at x=60.0 (> GOAL_LINE_X) for 10 ticks.
        let frames: Vec<MatchFrameDto> = (0..10).map(|t| make_frame(t, 60.0, 0.0, 0, 0)).collect();
        let flags = detect_ball_off_pitch(&frames);
        assert!(!flags.is_empty(), "should flag ball off pitch");
    }

    #[test]
    fn stall_fires_on_frozen_frames() {
        // All entities at fixed positions for STALL_WINDOW + 1 ticks.
        let frames: Vec<MatchFrameDto> = (0..(STALL_WINDOW as i64 + 2))
            .map(|t| make_frame(t, 1.0, 1.0, 0, 0))
            .collect();
        let flags = detect_stall(&frames);
        assert!(!flags.is_empty(), "should flag stall on frozen frames");
    }

    #[test]
    fn run_all_detectors_produces_report_with_7_detectors() {
        let frames: Vec<MatchFrameDto> = (0..10).map(|t| make_frame(t, 0.0, 0.0, 0, 0)).collect();
        let report = run_all_detectors(&frames);
        assert_eq!(report.detectors.len(), 7);
        assert_eq!(report.full_match_ticks, FULL_MATCH_TICKS);
        assert_eq!(report.evaluable_frame_pairs, 9); // 10 frames → 9 pairs
    }

    // P1 fix: phasing still fires on an uninvolved player when possession changes.
    // Scenario: possession flips 9→5 (both slots excluded), but slot 3 is near the
    // ball path. The OLD code would skip the entire frame; the NEW code must flag slot 3.
    #[test]
    fn ball_phasing_fires_on_uninvolved_player_during_possession_change() {
        let mut f0 = make_frame(0, 0.0, 0.0, 0, 0);
        let mut f1 = make_frame(1, 0.4, 0.0, 0, 0); // ball moves 0.4m along x
        // Possession changes 9 → 5.
        f0.possession = Some(9);
        f1.possession = Some(5);
        // Place slot 3 at (0.2, 0.0) — on the ball path, well within PLAYER_RADIUS.
        f1.players[3].pos_x = 0.2;
        f1.players[3].pos_y = 0.0;
        let frames = vec![f0, f1];
        let flags = detect_ball_phasing(&frames);
        // Must flag slot 3 even though possession changed (9 and 5 are excluded, not 3).
        let slot3_flags: Vec<_> = flags
            .iter()
            .filter(|f| f.detail.contains("slot 3"))
            .collect();
        assert!(
            !slot3_flags.is_empty(),
            "slot 3 should be flagged for phasing even though possession changed 9→5; \
             got flags: {flags:?}"
        );
    }

    // P1 fix: when possession doesn't change, phasing on any near player still fires.
    #[test]
    fn ball_phasing_fires_when_no_possession_change() {
        let mut f0 = make_frame(0, 0.0, 0.0, 0, 0);
        let mut f1 = make_frame(1, 0.4, 0.0, 0, 0);
        // Same possession slot (no change), slot 3 near ball path.
        f0.possession = Some(9);
        f1.possession = Some(9);
        f1.players[3].pos_x = 0.2;
        f1.players[3].pos_y = 0.0;
        let frames = vec![f0, f1];
        let flags = detect_ball_phasing(&frames);
        let slot3_flags: Vec<_> = flags
            .iter()
            .filter(|f| f.detail.contains("slot 3"))
            .collect();
        assert!(
            !slot3_flags.is_empty(),
            "slot 3 should be flagged when no possession change; got: {flags:?}"
        );
    }

    // P1 fix: the possession-change pair (9→5) themselves are excluded.
    #[test]
    fn ball_phasing_excludes_possession_change_slots() {
        let mut f0 = make_frame(0, 0.0, 0.0, 0, 0);
        let mut f1 = make_frame(1, 0.4, 0.0, 0, 0);
        f0.possession = Some(9);
        f1.possession = Some(5);
        // Place both slot 9 and slot 5 exactly on the ball path.
        f1.players[9].pos_x = 0.2;
        f1.players[9].pos_y = 0.0;
        f1.players[5].pos_x = 0.2;
        f1.players[5].pos_y = 0.0;
        // All others well away from ball.
        for i in 0..22usize {
            if i != 5 && i != 9 {
                f1.players[i].pos_x = 50.0; // far away
            }
        }
        let frames = vec![f0, f1];
        let flags = detect_ball_phasing(&frames);
        // Neither slot 9 nor slot 5 should appear in flags (they were the possession pair).
        let involved_flags: Vec<_> = flags
            .iter()
            .filter(|f| f.detail.contains("slot 9") || f.detail.contains("slot 5"))
            .collect();
        assert!(
            involved_flags.is_empty(),
            "possession-change slots 9 and 5 should be excluded from phasing; \
             got: {involved_flags:?}"
        );
    }

    // P2 fix: a single-frame input must NOT silently report all-clear.
    #[test]
    fn single_frame_input_produces_incomplete_status_not_silent_allclear() {
        let frames = vec![make_frame(0, 0.0, 0.0, 0, 0)];
        let report = run_all_detectors(&frames);
        assert_eq!(
            report.status,
            ReportStatus::Incomplete,
            "single-frame report must be INCOMPLETE, not OK"
        );
        assert_eq!(
            report.evaluable_frame_pairs, 0,
            "zero evaluable pairs for single-frame input"
        );
        assert!(
            !report.warnings.is_empty(),
            "warnings must be non-empty for single-frame input"
        );
    }

    // P2 fix: a 2+-frame input with correct structure produces OK + correct pair count.
    #[test]
    fn two_frame_input_produces_ok_status_with_one_pair() {
        let frames: Vec<MatchFrameDto> = (0..2).map(|t| make_frame(t, 0.0, 0.0, 0, 0)).collect();
        let report = run_all_detectors(&frames);
        assert_eq!(report.evaluable_frame_pairs, 1);
        // Status may be OK (only the standing BallPhasing lower-bound warning is present,
        // which does not flip status to INCOMPLETE).
        assert_eq!(report.status, ReportStatus::Ok);
    }

    // P4 fix: validate_frames catches non-contiguous ticks.
    #[test]
    fn validate_frames_warns_on_non_contiguous_ticks() {
        let frames = vec![
            make_frame(0, 0.0, 0.0, 0, 0),
            make_frame(2, 0.0, 0.0, 0, 0), // gap: tick 1 missing
        ];
        let (warnings, _) = validate_frames(&frames);
        let gap_warning = warnings
            .iter()
            .any(|w| w.contains("dense") || w.contains("contiguous"));
        assert!(
            gap_warning,
            "should warn about non-contiguous ticks; got: {warnings:?}"
        );
    }

    // P2 fix: wrong player count is flagged.
    #[test]
    fn validate_frames_warns_on_wrong_player_count() {
        let mut f0 = make_frame(0, 0.0, 0.0, 0, 0);
        f0.players.truncate(10); // only 10 players instead of 22
        let f1 = make_frame(1, 0.0, 0.0, 0, 0);
        let (warnings, _) = validate_frames(&[f0, f1]);
        let count_warning = warnings.iter().any(|w| w.contains("10 players"));
        assert!(
            count_warning,
            "should warn about wrong player count; got: {warnings:?}"
        );
    }
}

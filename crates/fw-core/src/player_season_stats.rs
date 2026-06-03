//! `PlayerSeasonStats` — per-season performance accumulator for one player.
//!
//! Lives in `fw-core` so `fw-save` can reference it in `SavedPlayerInstance`
//! without depending on `fw-tauri`. Moved from `fw-tauri::roster` at T4-2.5g
//! when `SaveV4` gained a `roster` field that serialises these stats.
//!
//! ## Float-free discipline
//!
//! `average_rating_numerator` accumulates the raw Q32 integer bits for the
//! running sum of per-match ratings. Divide by `rating_sample_count` at DTO
//! projection time only — never store floats in canonical state.
//!
//! ## Field-order stability
//!
//! **Do not reorder fields.** bincode 2 is positional: changing the declaration
//! order changes the wire bytes and breaks the `SaveV4` round-trip contract.

use serde::{Deserialize, Serialize};

/// Per-season performance statistics for one player instance.
///
/// `average_rating_numerator` accumulates the sum of per-match Q32 ratings;
/// divide by `rating_sample_count` at DTO projection time to avoid floats in
/// canonical state (`Sim/RULES.md §1`).
///
/// Field order is stable for serde/bincode determinism. Do not reorder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct PlayerSeasonStats {
    pub appearances: u16,
    pub goals: u16,
    pub assists: u16,
    /// Total minutes played across all appearances this season.
    pub minutes_played: u32,
    /// Running sum of per-match Q32 ratings (numerator; divide by
    /// `rating_sample_count` to obtain the mean at DTO time).
    pub average_rating_numerator: i64,
    /// Number of match ratings recorded in `average_rating_numerator`.
    pub rating_sample_count: u16,
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// `PlayerSeasonStats::default()` produces all-zero fields — correct starting
    /// point for a fresh season or a migrated save that had no season stats.
    #[test]
    fn default_is_all_zero() {
        let stats = PlayerSeasonStats::default();
        assert_eq!(stats.appearances, 0);
        assert_eq!(stats.goals, 0);
        assert_eq!(stats.assists, 0);
        assert_eq!(stats.minutes_played, 0);
        assert_eq!(stats.average_rating_numerator, 0);
        assert_eq!(stats.rating_sample_count, 0);
    }

    /// bincode 1.x round-trip: encode → decode produces byte-identical output.
    /// This pins field order and serde behaviour — a field reorder or a
    /// `#[serde(skip)]` annotation change would break value equality.
    ///
    /// `fw-core` dev-deps include `bincode = "1"` for legacy Q32 tests; the
    /// production save path uses bincode 2 (covered by `fw-save` migration
    /// fixture tests which re-encode `SaveV4` from frozen bytes).
    #[test]
    fn bincode_round_trip_byte_identical() {
        let stats = PlayerSeasonStats {
            appearances: 38,
            goals: 12,
            assists: 7,
            minutes_played: 3240,
            average_rating_numerator: 274_877_906_944, // 64 * 2^32 in raw bits
            rating_sample_count: 38,
        };

        let encoded = bincode::serialize(&stats).expect("bincode 1 serialize");
        let decoded: PlayerSeasonStats =
            bincode::deserialize(&encoded).expect("bincode 1 deserialize");

        assert_eq!(decoded, stats, "round-trip must be value-identical");

        let re_encoded = bincode::serialize(&decoded).expect("bincode 1 re-serialize");
        assert_eq!(
            encoded, re_encoded,
            "encode(decode(encode(x))) must be byte-identical"
        );
    }

    /// Non-default values round-trip through serde_json (exercises the JSON
    /// path used by integration tests that serialise `AppState` as JSON).
    #[test]
    fn json_round_trip() {
        let stats = PlayerSeasonStats {
            appearances: 5,
            goals: 2,
            assists: 1,
            minutes_played: 450,
            average_rating_numerator: 0,
            rating_sample_count: 5,
        };
        let json = serde_json::to_string(&stats).expect("to_string");
        let back: PlayerSeasonStats = serde_json::from_str(&json).expect("from_str");
        assert_eq!(back, stats);
    }
}

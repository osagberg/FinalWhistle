//! Expected-threat (xT) delta — 16×12 Bellman-baked grid.
//!
//! The grid represents the expected probability of scoring within the next few
//! actions from each pitch zone, baked from Karun Singh's Bellman fixed-point
//! approach (16×12 resolution; hand-authored Phase-1 seeds per
//! `docs/design/xt-resolution.md`).
//!
//! Layout: index = x * 12 + y.
//!   - x: 0 = own-goal end, 15 = attacking-goal-line.
//!   - y: 0 = bottom touchline, 11 = top touchline.
//!
//! Usage: `xt_delta(src, dst)` gives the xT gain from moving the ball from
//! `src` to `dst`.  Positive = threat increased; negative = retreating.

use fw_core::Q32;

// -------------------------------------------------------------------------
// Grid
// -------------------------------------------------------------------------

/// 192-entry xT grid.  `XT_GRID[x * 12 + y]` is the xT for zone (x, y).
///
/// Values range from ~0.01 (own-goal end) to ~0.40 (attacking penalty area).
/// These are Phase-1 seeds hand-authored from Karun Singh's published 16×12
/// transition-matrix structure; re-calibration at T2-1 via the
/// `fw-content-baker` XT bake pass.
///
/// `pub(crate)` — external code reads via `PitchZone::xt()` / `xt_delta()`.
pub(crate) const XT_GRID: [Q32; 192] = [
    Q32::from_raw(42949673_i64),   // x=0,  y=0,  xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=1,  xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=2,  xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=3,  xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=4,  xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=5,  xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=6,  xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=7,  xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=8,  xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=9,  xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=10, xT=0.0100
    Q32::from_raw(42949673_i64),   // x=0,  y=11, xT=0.0100
    Q32::from_raw(46281484_i64),   // x=1,  y=0,  xT=0.0108
    Q32::from_raw(46523797_i64),   // x=1,  y=1,  xT=0.0108
    Q32::from_raw(46766111_i64),   // x=1,  y=2,  xT=0.0109
    Q32::from_raw(47008424_i64),   // x=1,  y=3,  xT=0.0109
    Q32::from_raw(47250738_i64),   // x=1,  y=4,  xT=0.0110
    Q32::from_raw(47493051_i64),   // x=1,  y=5,  xT=0.0111
    Q32::from_raw(47493051_i64),   // x=1,  y=6,  xT=0.0111
    Q32::from_raw(47250738_i64),   // x=1,  y=7,  xT=0.0110
    Q32::from_raw(47008424_i64),   // x=1,  y=8,  xT=0.0109
    Q32::from_raw(46766111_i64),   // x=1,  y=9,  xT=0.0109
    Q32::from_raw(46523797_i64),   // x=1,  y=10, xT=0.0108
    Q32::from_raw(46281484_i64),   // x=1,  y=11, xT=0.0108
    Q32::from_raw(58258656_i64),   // x=2,  y=0,  xT=0.0136
    Q32::from_raw(59372036_i64),   // x=2,  y=1,  xT=0.0138
    Q32::from_raw(60485417_i64),   // x=2,  y=2,  xT=0.0141
    Q32::from_raw(61598798_i64),   // x=2,  y=3,  xT=0.0143
    Q32::from_raw(62712178_i64),   // x=2,  y=4,  xT=0.0146
    Q32::from_raw(63825559_i64),   // x=2,  y=5,  xT=0.0149
    Q32::from_raw(63825559_i64),   // x=2,  y=6,  xT=0.0149
    Q32::from_raw(62712178_i64),   // x=2,  y=7,  xT=0.0146
    Q32::from_raw(61598798_i64),   // x=2,  y=8,  xT=0.0143
    Q32::from_raw(60485417_i64),   // x=2,  y=9,  xT=0.0141
    Q32::from_raw(59372036_i64),   // x=2,  y=10, xT=0.0138
    Q32::from_raw(58258656_i64),   // x=2,  y=11, xT=0.0136
    Q32::from_raw(80304532_i64),   // x=3,  y=0,  xT=0.0187
    Q32::from_raw(83021249_i64),   // x=3,  y=1,  xT=0.0193
    Q32::from_raw(85737966_i64),   // x=3,  y=2,  xT=0.0200
    Q32::from_raw(88454684_i64),   // x=3,  y=3,  xT=0.0206
    Q32::from_raw(91171401_i64),   // x=3,  y=4,  xT=0.0212
    Q32::from_raw(93888118_i64),   // x=3,  y=5,  xT=0.0219
    Q32::from_raw(93888118_i64),   // x=3,  y=6,  xT=0.0219
    Q32::from_raw(91171401_i64),   // x=3,  y=7,  xT=0.0212
    Q32::from_raw(88454684_i64),   // x=3,  y=8,  xT=0.0206
    Q32::from_raw(85737966_i64),   // x=3,  y=9,  xT=0.0200
    Q32::from_raw(83021249_i64),   // x=3,  y=10, xT=0.0193
    Q32::from_raw(80304532_i64),   // x=3,  y=11, xT=0.0187
    Q32::from_raw(113291287_i64),  // x=4,  y=0,  xT=0.0264
    Q32::from_raw(118407040_i64),  // x=4,  y=1,  xT=0.0276
    Q32::from_raw(123522794_i64),  // x=4,  y=2,  xT=0.0288
    Q32::from_raw(128638548_i64),  // x=4,  y=3,  xT=0.0300
    Q32::from_raw(133754302_i64),  // x=4,  y=4,  xT=0.0311
    Q32::from_raw(138870055_i64),  // x=4,  y=5,  xT=0.0323
    Q32::from_raw(138870055_i64),  // x=4,  y=6,  xT=0.0323
    Q32::from_raw(133754302_i64),  // x=4,  y=7,  xT=0.0311
    Q32::from_raw(128638548_i64),  // x=4,  y=8,  xT=0.0300
    Q32::from_raw(123522794_i64),  // x=4,  y=9,  xT=0.0288
    Q32::from_raw(118407040_i64),  // x=4,  y=10, xT=0.0276
    Q32::from_raw(113291287_i64),  // x=4,  y=11, xT=0.0264
    Q32::from_raw(157874631_i64),  // x=5,  y=0,  xT=0.0368
    Q32::from_raw(166232810_i64),  // x=5,  y=1,  xT=0.0387
    Q32::from_raw(174590989_i64),  // x=5,  y=2,  xT=0.0407
    Q32::from_raw(182949168_i64),  // x=5,  y=3,  xT=0.0426
    Q32::from_raw(191307347_i64),  // x=5,  y=4,  xT=0.0445
    Q32::from_raw(199665525_i64),  // x=5,  y=5,  xT=0.0465
    Q32::from_raw(199665525_i64),  // x=5,  y=6,  xT=0.0465
    Q32::from_raw(191307347_i64),  // x=5,  y=7,  xT=0.0445
    Q32::from_raw(182949168_i64),  // x=5,  y=8,  xT=0.0426
    Q32::from_raw(174590989_i64),  // x=5,  y=9,  xT=0.0407
    Q32::from_raw(166232810_i64),  // x=5,  y=10, xT=0.0387
    Q32::from_raw(157874631_i64),  // x=5,  y=11, xT=0.0368
    Q32::from_raw(214587535_i64),  // x=6,  y=0,  xT=0.0500
    Q32::from_raw(227070289_i64),  // x=6,  y=1,  xT=0.0529
    Q32::from_raw(239553043_i64),  // x=6,  y=2,  xT=0.0558
    Q32::from_raw(252035796_i64),  // x=6,  y=3,  xT=0.0587
    Q32::from_raw(264518550_i64),  // x=6,  y=4,  xT=0.0616
    Q32::from_raw(277001303_i64),  // x=6,  y=5,  xT=0.0645
    Q32::from_raw(277001303_i64),  // x=6,  y=6,  xT=0.0645
    Q32::from_raw(264518550_i64),  // x=6,  y=7,  xT=0.0616
    Q32::from_raw(252035796_i64),  // x=6,  y=8,  xT=0.0587
    Q32::from_raw(239553043_i64),  // x=6,  y=9,  xT=0.0558
    Q32::from_raw(227070289_i64),  // x=6,  y=10, xT=0.0529
    Q32::from_raw(214587535_i64),  // x=6,  y=11, xT=0.0500
    Q32::from_raw(283882532_i64),  // x=7,  y=0,  xT=0.0661
    Q32::from_raw(301404922_i64),  // x=7,  y=1,  xT=0.0702
    Q32::from_raw(318927311_i64),  // x=7,  y=2,  xT=0.0743
    Q32::from_raw(336449701_i64),  // x=7,  y=3,  xT=0.0783
    Q32::from_raw(353972091_i64),  // x=7,  y=4,  xT=0.0824
    Q32::from_raw(371494481_i64),  // x=7,  y=5,  xT=0.0865
    Q32::from_raw(371494481_i64),  // x=7,  y=6,  xT=0.0865
    Q32::from_raw(353972091_i64),  // x=7,  y=7,  xT=0.0824
    Q32::from_raw(336449701_i64),  // x=7,  y=8,  xT=0.0783
    Q32::from_raw(318927311_i64),  // x=7,  y=9,  xT=0.0743
    Q32::from_raw(301404922_i64),  // x=7,  y=10, xT=0.0702
    Q32::from_raw(283882532_i64),  // x=7,  y=11, xT=0.0661
    Q32::from_raw(366154857_i64),  // x=8,  y=0,  xT=0.0853
    Q32::from_raw(389660688_i64),  // x=8,  y=1,  xT=0.0907
    Q32::from_raw(413166520_i64),  // x=8,  y=2,  xT=0.0962
    Q32::from_raw(436672351_i64),  // x=8,  y=3,  xT=0.1017
    Q32::from_raw(460178183_i64),  // x=8,  y=4,  xT=0.1071
    Q32::from_raw(483684014_i64),  // x=8,  y=5,  xT=0.1126
    Q32::from_raw(483684014_i64),  // x=8,  y=6,  xT=0.1126
    Q32::from_raw(460178183_i64),  // x=8,  y=7,  xT=0.1071
    Q32::from_raw(436672351_i64),  // x=8,  y=8,  xT=0.1017
    Q32::from_raw(413166520_i64),  // x=8,  y=9,  xT=0.0962
    Q32::from_raw(389660688_i64),  // x=8,  y=10, xT=0.0907
    Q32::from_raw(366154857_i64),  // x=8,  y=11, xT=0.0853
    Q32::from_raw(461756610_i64),  // x=9,  y=0,  xT=0.1075
    Q32::from_raw(492215297_i64),  // x=9,  y=1,  xT=0.1146
    Q32::from_raw(522673983_i64),  // x=9,  y=2,  xT=0.1217
    Q32::from_raw(553132669_i64),  // x=9,  y=3,  xT=0.1288
    Q32::from_raw(583591356_i64),  // x=9,  y=4,  xT=0.1359
    Q32::from_raw(614050042_i64),  // x=9,  y=5,  xT=0.1430
    Q32::from_raw(614050042_i64),  // x=9,  y=6,  xT=0.1430
    Q32::from_raw(583591356_i64),  // x=9,  y=7,  xT=0.1359
    Q32::from_raw(553132669_i64),  // x=9,  y=8,  xT=0.1288
    Q32::from_raw(522673983_i64),  // x=9,  y=9,  xT=0.1217
    Q32::from_raw(492215297_i64),  // x=9,  y=10, xT=0.1146
    Q32::from_raw(461756610_i64),  // x=9,  y=11, xT=0.1075
    Q32::from_raw(571006116_i64),  // x=10, y=0,  xT=0.1329
    Q32::from_raw(609410221_i64),  // x=10, y=1,  xT=0.1419
    Q32::from_raw(647814326_i64),  // x=10, y=2,  xT=0.1508
    Q32::from_raw(686218431_i64),  // x=10, y=3,  xT=0.1598
    Q32::from_raw(724622536_i64),  // x=10, y=4,  xT=0.1687
    Q32::from_raw(763026641_i64),  // x=10, y=5,  xT=0.1777
    Q32::from_raw(763026641_i64),  // x=10, y=6,  xT=0.1777
    Q32::from_raw(724622536_i64),  // x=10, y=7,  xT=0.1687
    Q32::from_raw(686218431_i64),  // x=10, y=8,  xT=0.1598
    Q32::from_raw(647814326_i64),  // x=10, y=9,  xT=0.1508
    Q32::from_raw(609410221_i64),  // x=10, y=10, xT=0.1419
    Q32::from_raw(571006116_i64),  // x=10, y=11, xT=0.1329
    Q32::from_raw(694194450_i64),  // x=11, y=0,  xT=0.1616
    Q32::from_raw(741557707_i64),  // x=11, y=1,  xT=0.1727
    Q32::from_raw(788920963_i64),  // x=11, y=2,  xT=0.1837
    Q32::from_raw(836284220_i64),  // x=11, y=3,  xT=0.1947
    Q32::from_raw(883647476_i64),  // x=11, y=4,  xT=0.2057
    Q32::from_raw(931010733_i64),  // x=11, y=5,  xT=0.2168
    Q32::from_raw(931010733_i64),  // x=11, y=6,  xT=0.2168
    Q32::from_raw(883647476_i64),  // x=11, y=7,  xT=0.2057
    Q32::from_raw(836284220_i64),  // x=11, y=8,  xT=0.1947
    Q32::from_raw(788920963_i64),  // x=11, y=9,  xT=0.1837
    Q32::from_raw(741557707_i64),  // x=11, y=10, xT=0.1727
    Q32::from_raw(694194450_i64),  // x=11, y=11, xT=0.1616
    Q32::from_raw(831590193_i64),  // x=12, y=0,  xT=0.1936
    Q32::from_raw(913516411_i64),  // x=12, y=1,  xT=0.2127
    Q32::from_raw(981049538_i64),  // x=12, y=2,  xT=0.2284
    Q32::from_raw(1046214646_i64), // x=12, y=3,  xT=0.2436
    Q32::from_raw(1110153978_i64), // x=12, y=4,  xT=0.2585
    Q32::from_raw(1173309971_i64), // x=12, y=5,  xT=0.2732
    Q32::from_raw(1173309971_i64), // x=12, y=6,  xT=0.2732
    Q32::from_raw(1110153978_i64), // x=12, y=7,  xT=0.2585
    Q32::from_raw(1046214646_i64), // x=12, y=8,  xT=0.2436
    Q32::from_raw(981049538_i64),  // x=12, y=9,  xT=0.2284
    Q32::from_raw(913516411_i64),  // x=12, y=10, xT=0.2127
    Q32::from_raw(831590193_i64),  // x=12, y=11, xT=0.1936
    Q32::from_raw(983443006_i64),  // x=13, y=0,  xT=0.2290
    Q32::from_raw(1139417481_i64), // x=13, y=1,  xT=0.2653
    Q32::from_raw(1244091732_i64), // x=13, y=2,  xT=0.2897
    Q32::from_raw(1340325831_i64), // x=13, y=3,  xT=0.3121
    Q32::from_raw(1432190986_i64), // x=13, y=4,  xT=0.3335
    Q32::from_raw(1521264145_i64), // x=13, y=5,  xT=0.3542
    Q32::from_raw(1521264145_i64), // x=13, y=6,  xT=0.3542
    Q32::from_raw(1432190986_i64), // x=13, y=7,  xT=0.3335
    Q32::from_raw(1340325831_i64), // x=13, y=8,  xT=0.3121
    Q32::from_raw(1244091732_i64), // x=13, y=9,  xT=0.2897
    Q32::from_raw(1139417481_i64), // x=13, y=10, xT=0.2653
    Q32::from_raw(983443006_i64),  // x=13, y=11, xT=0.2290
    Q32::from_raw(1149986388_i64), // x=14, y=0,  xT=0.2678
    Q32::from_raw(1403011184_i64), // x=14, y=1,  xT=0.3267
    Q32::from_raw(1554980183_i64), // x=14, y=2,  xT=0.3620
    Q32::from_raw(1690323012_i64), // x=14, y=3,  xT=0.3936
    Q32::from_raw(1717986918_i64), // x=14, y=4,  xT=0.4000
    Q32::from_raw(1717986918_i64), // x=14, y=5,  xT=0.4000
    Q32::from_raw(1717986918_i64), // x=14, y=6,  xT=0.4000
    Q32::from_raw(1717986918_i64), // x=14, y=7,  xT=0.4000
    Q32::from_raw(1690323012_i64), // x=14, y=8,  xT=0.3936
    Q32::from_raw(1554980183_i64), // x=14, y=9,  xT=0.3620
    Q32::from_raw(1403011184_i64), // x=14, y=10, xT=0.3267
    Q32::from_raw(1149986388_i64), // x=14, y=11, xT=0.2678
    Q32::from_raw(1331439862_i64), // x=15, y=0,  xT=0.3100
    Q32::from_raw(1699855270_i64), // x=15, y=1,  xT=0.3958
    Q32::from_raw(1717986918_i64), // x=15, y=2,  xT=0.4000
    Q32::from_raw(1717986918_i64), // x=15, y=3,  xT=0.4000
    Q32::from_raw(1717986918_i64), // x=15, y=4,  xT=0.4000
    Q32::from_raw(1717986918_i64), // x=15, y=5,  xT=0.4000
    Q32::from_raw(1717986918_i64), // x=15, y=6,  xT=0.4000
    Q32::from_raw(1717986918_i64), // x=15, y=7,  xT=0.4000
    Q32::from_raw(1717986918_i64), // x=15, y=8,  xT=0.4000
    Q32::from_raw(1717986918_i64), // x=15, y=9,  xT=0.4000
    Q32::from_raw(1699855270_i64), // x=15, y=10, xT=0.3958
    Q32::from_raw(1331439862_i64), // x=15, y=11, xT=0.3100
];

// -------------------------------------------------------------------------
// Zone newtype
// -------------------------------------------------------------------------

/// A validated 16×12 pitch zone index.  `x` in [0, 15], `y` in [0, 11].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PitchZone {
    x: u8, // 0=own-goal end, 15=attacking-goal-line
    y: u8, // 0=bottom touchline, 11=top touchline
}

impl PitchZone {
    /// Construct from (x, y).  Returns `None` if out of range.
    #[inline]
    pub fn new(x: u8, y: u8) -> Option<Self> {
        if x < 16 && y < 12 {
            Some(PitchZone { x, y })
        } else {
            None
        }
    }

    /// x coordinate (0=own goal, 15=attacking goal line).
    #[inline]
    pub fn x(self) -> u8 {
        self.x
    }

    /// y coordinate (0=bottom, 11=top).
    #[inline]
    pub fn y(self) -> u8 {
        self.y
    }

    /// Flat index into `XT_GRID`.
    #[inline]
    pub fn flat_index(self) -> usize {
        (self.x as usize) * 12 + (self.y as usize)
    }

    /// xT value at this zone.
    #[inline]
    pub fn xt(self) -> Q32 {
        XT_GRID[self.flat_index()]
    }
}

// -------------------------------------------------------------------------
// Public API
// -------------------------------------------------------------------------

/// Expected-threat gain of moving the ball from `src` zone to `dst` zone.
///
/// Positive = threat increased (progressive pass / dribble).
/// Negative = threat decreased (back-pass / wide recycling).
///
/// The delta is computed purely from the `XT_GRID` constant — no allocation,
/// no RNG, fully deterministic.
#[inline]
pub fn xt_delta(src: PitchZone, dst: PitchZone) -> Q32 {
    // Both XT_GRID values are in [0, 0.40]; delta is in [-0.40, +0.40].
    // checked_sub is always safe here given the grid bounds.
    dst.xt().checked_sub(src.xt()).unwrap_or(Q32::ZERO)
}

// -------------------------------------------------------------------------
// Tests
// -------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(x: u8, y: u8) -> PitchZone {
        PitchZone::new(x, y).unwrap()
    }

    #[test]
    fn grid_has_correct_length() {
        assert_eq!(XT_GRID.len(), 192);
    }

    #[test]
    fn all_values_in_unit_range() {
        for (i, &v) in XT_GRID.iter().enumerate() {
            assert!(
                v >= Q32::ZERO && v <= Q32::ONE,
                "XT_GRID[{i}] out of [0,1]: raw={}",
                v.to_bits()
            );
        }
    }

    #[test]
    fn x_monotone_along_central_y() {
        // Central y=5 and y=6 should have monotonically non-decreasing xT as x increases.
        for y in [5u8, 6u8] {
            let mut prev = zone(0, y).xt();
            for x in 1u8..16 {
                let cur = zone(x, y).xt();
                assert!(
                    cur >= prev,
                    "xT should increase toward attacking end; x={x}, y={y}, prev={:?} cur={:?}",
                    prev,
                    cur
                );
                prev = cur;
            }
        }
    }

    #[test]
    fn y_symmetry() {
        // Left-right symmetry: xt(x, y) == xt(x, 11-y).
        for x in 0u8..16 {
            for y in 0u8..6 {
                let lo = zone(x, y).xt();
                let hi = zone(x, 11 - y).xt();
                assert_eq!(
                    lo,
                    hi,
                    "xT symmetry broken at x={x}: y={y} ({:?}) vs y={} ({:?})",
                    lo,
                    11 - y,
                    hi
                );
            }
        }
    }

    #[test]
    fn own_goal_end_is_low() {
        // All x=0 cells should have xT ≈ 0.01 (raw 42949673).
        for y in 0u8..12 {
            let v = zone(0, y).xt();
            assert!(
                v.to_bits() < 50_000_000,
                "x=0,y={y} xT too high: raw={}",
                v.to_bits()
            );
        }
    }

    #[test]
    fn attacking_third_is_high() {
        // Central cells at x=14-15 should have xT > 0.25 (raw > ~1_073_741_824).
        for x in 14u8..16 {
            for y in 4u8..8 {
                let v = zone(x, y).xt();
                assert!(
                    v.to_bits() > 1_073_741_824,
                    "x={x},y={y} xT should be > 0.25, raw={}",
                    v.to_bits()
                );
            }
        }
    }

    #[test]
    fn xt_delta_forward_is_positive() {
        let src = zone(4, 5);
        let dst = zone(12, 5);
        let delta = xt_delta(src, dst);
        assert!(delta > Q32::ZERO, "forward pass delta should be positive");
    }

    #[test]
    fn xt_delta_backward_is_negative() {
        let src = zone(12, 5);
        let dst = zone(4, 5);
        let delta = xt_delta(src, dst);
        assert!(delta < Q32::ZERO, "backward pass delta should be negative");
    }

    #[test]
    fn xt_delta_same_zone_is_zero() {
        let z = zone(7, 5);
        assert_eq!(xt_delta(z, z), Q32::ZERO);
    }

    #[test]
    fn pitch_zone_out_of_range_returns_none() {
        assert!(PitchZone::new(16, 0).is_none());
        assert!(PitchZone::new(0, 12).is_none());
        assert!(PitchZone::new(15, 11).is_some());
    }

    #[test]
    fn flat_index_corners() {
        assert_eq!(zone(0, 0).flat_index(), 0);
        assert_eq!(zone(0, 11).flat_index(), 11);
        assert_eq!(zone(15, 0).flat_index(), 180);
        assert_eq!(zone(15, 11).flat_index(), 191);
    }
}

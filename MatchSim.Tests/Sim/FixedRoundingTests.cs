using System;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Q32.32 rounding correctness — Floor / Ceiling / Truncate / Round.
/// Edge cases at zero, negative non-integers, and exact halves.
/// </summary>
public sealed class FixedRoundingTests
{
    private static Fixed Of(int integer, long fractionalRaw)
    {
        // Helper: construct a Fixed with explicit integer + fractional parts.
        // fractionalRaw is the raw bottom-32-bits value.
        long raw = ((long)integer << Fixed.FractionalBits) | (fractionalRaw & 0xFFFFFFFFL);
        return Fixed.FromRaw(raw);
    }

    #region Floor

    [Fact]
    public void Floor_PositiveExactInteger_ReturnsSelf()
    {
        Assert.Equal(Fixed.FromInt(3), Fixed.Floor(Fixed.FromInt(3)));
    }

    [Fact]
    public void Floor_PositiveFraction_RoundsDown()
    {
        // 3.5 → 3
        Fixed value = Fixed.FromInt(3) + Fixed.Half;
        Assert.Equal(Fixed.FromInt(3), Fixed.Floor(value));
    }

    [Fact]
    public void Floor_NegativeFraction_RoundsTowardNegativeInfinity()
    {
        // -3.5 → -4 (NOT -3)
        Fixed value = Fixed.FromInt(-3) - Fixed.Half;
        Assert.Equal(Fixed.FromInt(-4), Fixed.Floor(value));
    }

    [Fact]
    public void Floor_NegativeExactInteger_ReturnsSelf()
    {
        Assert.Equal(Fixed.FromInt(-5), Fixed.Floor(Fixed.FromInt(-5)));
    }

    [Fact]
    public void Floor_Zero_ReturnsZero()
    {
        Assert.Equal(Fixed.Zero, Fixed.Floor(Fixed.Zero));
    }

    #endregion

    #region Ceiling

    [Fact]
    public void Ceiling_PositiveFraction_RoundsUp()
    {
        // 3.5 → 4
        Fixed value = Fixed.FromInt(3) + Fixed.Half;
        Assert.Equal(Fixed.FromInt(4), Fixed.Ceiling(value));
    }

    [Fact]
    public void Ceiling_NegativeFraction_RoundsTowardZero()
    {
        // -3.5 → -3
        Fixed value = Fixed.FromInt(-3) - Fixed.Half;
        Assert.Equal(Fixed.FromInt(-3), Fixed.Ceiling(value));
    }

    [Fact]
    public void Ceiling_PositiveExactInteger_ReturnsSelf()
    {
        Assert.Equal(Fixed.FromInt(3), Fixed.Ceiling(Fixed.FromInt(3)));
    }

    [Fact]
    public void Ceiling_NegativeExactInteger_ReturnsSelf()
    {
        Assert.Equal(Fixed.FromInt(-5), Fixed.Ceiling(Fixed.FromInt(-5)));
    }

    #endregion

    #region Truncate

    [Fact]
    public void Truncate_PositiveFraction_RoundsTowardZero()
    {
        // 3.5 → 3
        Fixed value = Fixed.FromInt(3) + Fixed.Half;
        Assert.Equal(Fixed.FromInt(3), Fixed.Truncate(value));
    }

    [Fact]
    public void Truncate_NegativeFraction_RoundsTowardZero()
    {
        // -3.5 → -3 (different from Floor which gives -4)
        Fixed value = Fixed.FromInt(-3) - Fixed.Half;
        Assert.Equal(Fixed.FromInt(-3), Fixed.Truncate(value));
    }

    [Fact]
    public void Truncate_DoesNotEqualFloor_ForNegativeFractions()
    {
        Fixed value = Fixed.FromInt(-3) - Fixed.Half;
        Assert.NotEqual(Fixed.Floor(value), Fixed.Truncate(value));
    }

    [Fact]
    public void Truncate_DoesEqualFloor_ForPositiveFractions()
    {
        Fixed value = Fixed.FromInt(3) + Fixed.Half;
        Assert.Equal(Fixed.Floor(value), Fixed.Truncate(value));
    }

    #endregion

    #region Round (banker's rounding)

    [Theory]
    [InlineData(0, 0)]    // 0.5 → 0 (even)
    [InlineData(1, 2)]    // 1.5 → 2 (even)
    [InlineData(2, 2)]    // 2.5 → 2 (even)
    [InlineData(3, 4)]    // 3.5 → 4 (even)
    public void Round_PositiveExactHalf_GoesToEven(int intPart, int expected)
    {
        Fixed value = Fixed.FromInt(intPart) + Fixed.Half;
        Assert.Equal(Fixed.FromInt(expected), Fixed.Round(value));
    }

    [Theory]
    [InlineData(0, 0)]      // -0.5 → 0
    [InlineData(-1, -2)]    // -1.5 → -2
    [InlineData(-2, -2)]    // -2.5 → -2
    public void Round_NegativeExactHalf_GoesToEven(int intPart, int expected)
    {
        Fixed value = Fixed.FromInt(intPart) - Fixed.Half;
        Assert.Equal(Fixed.FromInt(expected), Fixed.Round(value));
    }

    [Fact]
    public void Round_BelowHalf_RoundsDown()
    {
        // 1.4 → 1
        Fixed value = Fixed.FromInt(1) + Of(0, 0x6666_6666L); // ~0.4
        Assert.Equal(Fixed.FromInt(1), Fixed.Round(value));
    }

    [Fact]
    public void Round_AboveHalf_RoundsUp()
    {
        // 1.6 → 2
        Fixed value = Fixed.FromInt(1) + Of(0, 0x9999_9999L); // ~0.6
        Assert.Equal(Fixed.FromInt(2), Fixed.Round(value));
    }

    [Fact]
    public void Round_ExactInteger_ReturnsSelf()
    {
        Assert.Equal(Fixed.FromInt(42), Fixed.Round(Fixed.FromInt(42)));
    }

    #endregion

    #region Upper-boundary overflow (regression — Codex audit P1-01)

    // Background: the C# left-shift operator is NOT subject to checked
    // arithmetic. Earlier Ceiling/Round implementations reconstructed the
    // raw long via `checked((intPart + 1L) << FractionalBits)`, which
    // silently wrapped to long.MinValue when intPart+1 = 2^31. Per
    // Codex audit 2026-04-28 (P1-01), the contract is now: any rounding
    // operation that would exceed Fixed.MaxValue must throw
    // OverflowException, never wrap.

    [Fact]
    public void Ceiling_AtMaxValueWithFraction_ThrowsOnRoundUp()
    {
        // Construct a value just below MaxValue with a non-zero fractional
        // part. Rounding up would produce 2^31 as an integer part, which
        // is NOT representable in Q32.32 (max integer part = 2^31 - 1).
        // Build raw = ((2^31 - 1) << 32) | half_fraction = MaxInt.5
        long maxIntegerPart = int.MaxValue;
        long halfFraction = 0x80000000L;
        long raw = (maxIntegerPart << 32) | halfFraction; // 2147483647.5
        Fixed value = Fixed.FromRaw(raw);

        Assert.Throws<OverflowException>(() => Fixed.Ceiling(value));
    }

    [Fact]
    public void Round_AtMaxValueExactHalf_BankersThrowsOnEvenSelectsRoundUp()
    {
        // 2147483647.5 → banker's rounding goes to 2147483648 (even neighbour),
        // which is NOT representable. Must throw, not wrap.
        long raw = ((long)int.MaxValue << 32) | 0x80000000L;
        Fixed value = Fixed.FromRaw(raw);

        Assert.Throws<OverflowException>(() => Fixed.Round(value));
    }

    [Fact]
    public void Round_AtMaxValueAboveHalf_ThrowsOnRoundUp()
    {
        // 2147483647.6 → round-up to 2147483648, not representable. Must throw.
        long raw = ((long)int.MaxValue << 32) | 0x9999_9999L; // ~.6
        Fixed value = Fixed.FromRaw(raw);

        Assert.Throws<OverflowException>(() => Fixed.Round(value));
    }

    [Fact]
    public void Round_AtMaxValueBelowHalf_DoesNotWrap()
    {
        // 2147483647.4 → round-down to 2147483647, which IS representable.
        // Critical regression: must NOT silently wrap to MinValue.
        long raw = ((long)int.MaxValue << 32) | 0x6666_6666L; // ~.4
        Fixed value = Fixed.FromRaw(raw);

        Fixed expected = Fixed.FromInt(int.MaxValue);
        Assert.Equal(expected, Fixed.Round(value));
    }

    [Fact]
    public void Ceiling_OneUlpBelowMaxIntegerPart_ReturnsMaxInt()
    {
        // (2147483647 - 1).5 = 2147483646.5 → ceiling = 2147483647.
        // This must succeed (in-range) — guards the regression test from
        // accidentally claiming throw is the only acceptable behavior.
        long raw = ((long)(int.MaxValue - 1) << 32) | 0x80000000L;
        Fixed value = Fixed.FromRaw(raw);

        Assert.Equal(Fixed.FromInt(int.MaxValue), Fixed.Ceiling(value));
    }

    [Fact]
    public void Round_OneUlpBelowMaxIntegerPartAboveHalf_ReturnsMaxInt()
    {
        // 2147483646.6 → 2147483647 (in-range; do not over-throw).
        long raw = ((long)(int.MaxValue - 1) << 32) | 0x9999_9999L;
        Fixed value = Fixed.FromRaw(raw);

        Assert.Equal(Fixed.FromInt(int.MaxValue), Fixed.Round(value));
    }

    #endregion
}

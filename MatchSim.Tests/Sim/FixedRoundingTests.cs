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
}

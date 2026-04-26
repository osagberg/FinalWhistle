using System;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Q32.32 arithmetic correctness — addition, subtraction, multiplication,
/// division, negation. Every overflow path tested. Sign correctness in all
/// four quadrants for multiplicative ops.
/// </summary>
public sealed class FixedArithmeticTests
{
    #region Addition + subtraction

    [Theory]
    [InlineData(0, 0, 0)]
    [InlineData(1, 0, 1)]
    [InlineData(2, 3, 5)]
    [InlineData(-1, 1, 0)]
    [InlineData(-3, -4, -7)]
    [InlineData(int.MaxValue / 2, int.MaxValue / 2, int.MaxValue - 1)]
    public void Add_IntegerCases(int a, int b, int expected)
    {
        Fixed result = Fixed.FromInt(a) + Fixed.FromInt(b);
        Assert.Equal(Fixed.FromInt(expected), result);
    }

    [Theory]
    [InlineData(0, 0, 0)]
    [InlineData(5, 3, 2)]
    [InlineData(0, 1, -1)]
    [InlineData(-5, -3, -2)]
    public void Subtract_IntegerCases(int a, int b, int expected)
    {
        Fixed result = Fixed.FromInt(a) - Fixed.FromInt(b);
        Assert.Equal(Fixed.FromInt(expected), result);
    }

    [Fact]
    public void Add_HalfPlusHalf_EqualsOne()
    {
        Assert.Equal(Fixed.One, Fixed.Half + Fixed.Half);
    }

    [Fact]
    public void Add_OverflowAtMaxValue_Throws()
    {
        Assert.Throws<OverflowException>(() => Fixed.MaxValue + Fixed.Epsilon);
    }

    [Fact]
    public void Subtract_UnderflowAtMinValue_Throws()
    {
        Assert.Throws<OverflowException>(() => Fixed.MinValue - Fixed.Epsilon);
    }

    [Fact]
    public void UnaryMinus_NegatesValue()
    {
        Assert.Equal(Fixed.MinusOne, -Fixed.One);
        Assert.Equal(Fixed.One, -Fixed.MinusOne);
        Assert.Equal(Fixed.Zero, -Fixed.Zero);
    }

    [Fact]
    public void UnaryMinus_OnMinValue_Throws()
    {
        // long.MinValue cannot be negated — its absolute value is 2^63
        // which exceeds long.MaxValue = 2^63 - 1.
        Assert.Throws<OverflowException>(() => -Fixed.MinValue);
    }

    [Fact]
    public void UnaryPlus_IsIdentity()
    {
        Assert.Equal(Fixed.One, +Fixed.One);
        Assert.Equal(Fixed.MinusOne, +Fixed.MinusOne);
    }

    #endregion

    #region Negation / Abs / Sign / Min / Max

    [Fact]
    public void Negate_FlipsSign()
    {
        Assert.Equal(Fixed.MinusOne, Fixed.Negate(Fixed.One));
        Assert.Equal(Fixed.One, Fixed.Negate(Fixed.MinusOne));
    }

    [Fact]
    public void Abs_ReturnsMagnitude()
    {
        Assert.Equal(Fixed.One, Fixed.Abs(Fixed.MinusOne));
        Assert.Equal(Fixed.One, Fixed.Abs(Fixed.One));
        Assert.Equal(Fixed.Zero, Fixed.Abs(Fixed.Zero));
    }

    [Fact]
    public void Abs_OnMinValue_Throws()
    {
        Assert.Throws<OverflowException>(() => Fixed.Abs(Fixed.MinValue));
    }

    [Theory]
    [InlineData(0, 0)]
    [InlineData(1, 1)]
    [InlineData(-1, -1)]
    [InlineData(42, 1)]
    [InlineData(-42, -1)]
    public void Sign_MatchesMathSignOnInt(int value, int expected)
    {
        Assert.Equal(expected, Fixed.Sign(Fixed.FromInt(value)));
    }

    [Fact]
    public void Min_SelectsSmaller()
    {
        Assert.Equal(Fixed.MinusOne, Fixed.Min(Fixed.One, Fixed.MinusOne));
        Assert.Equal(Fixed.Zero, Fixed.Min(Fixed.Zero, Fixed.One));
    }

    [Fact]
    public void Max_SelectsLarger()
    {
        Assert.Equal(Fixed.One, Fixed.Max(Fixed.One, Fixed.MinusOne));
        Assert.Equal(Fixed.One, Fixed.Max(Fixed.Zero, Fixed.One));
    }

    [Fact]
    public void Min_OnEqualValues_ReturnsFirst()
    {
        // Stable min/max for tie-break determinism.
        Assert.Equal(Fixed.One, Fixed.Min(Fixed.One, Fixed.One));
    }

    #endregion

    #region Multiplication — basic

    [Theory]
    [InlineData(0, 0, 0)]
    [InlineData(1, 0, 0)]
    [InlineData(0, 1, 0)]
    [InlineData(1, 1, 1)]
    [InlineData(2, 3, 6)]
    [InlineData(-1, 5, -5)]
    [InlineData(5, -1, -5)]
    [InlineData(-2, -3, 6)]
    [InlineData(7, -8, -56)]
    public void Multiply_IntegerCases(int a, int b, int expected)
    {
        Fixed result = Fixed.FromInt(a) * Fixed.FromInt(b);
        Assert.Equal(Fixed.FromInt(expected), result);
    }

    [Fact]
    public void Multiply_HalfTimesHalf_EqualsQuarter()
    {
        Fixed quarter = Fixed.Half * Fixed.Half;
        // 0.25 in Q32.32 is 2^30 = 0x4000_0000.
        Assert.Equal(0x4000_0000L, quarter.RawValue);
    }

    [Fact]
    public void Multiply_NegativeHalfTimesNegativeHalf_EqualsPositiveQuarter()
    {
        Fixed nh = -Fixed.Half;
        Fixed result = nh * nh;
        Assert.Equal(0x4000_0000L, result.RawValue);
    }

    [Fact]
    public void Multiply_NegativeHalfTimesHalf_EqualsNegativeQuarter()
    {
        Fixed result = (-Fixed.Half) * Fixed.Half;
        Assert.Equal(-0x4000_0000L, result.RawValue);
    }

    [Fact]
    public void Multiply_ByZero_IsZero()
    {
        Assert.Equal(Fixed.Zero, Fixed.MaxValue * Fixed.Zero);
        Assert.Equal(Fixed.Zero, Fixed.Zero * Fixed.MinValue);
    }

    [Fact]
    public void Multiply_ByOne_IsIdentity()
    {
        Fixed value = Fixed.FromInt(42);
        Assert.Equal(value, value * Fixed.One);
        Assert.Equal(value, Fixed.One * value);
    }

    #endregion

    #region Multiplication — overflow

    [Fact]
    public void Multiply_MaxValueTimesTwo_Overflows()
    {
        Assert.Throws<OverflowException>(() => Fixed.MaxValue * Fixed.FromInt(2));
    }

    [Fact]
    public void Multiply_LargeTimesLarge_Overflows()
    {
        Fixed large = Fixed.FromInt(int.MaxValue / 2 + 1);
        Assert.Throws<OverflowException>(() => large * Fixed.FromInt(4));
    }

    [Fact]
    public void Multiply_MinValueTimesOne_PreservesValue()
    {
        // MinValue raw is long.MinValue = -2^63; representing approximately
        // -2^31 in Q32.32 magnitude. * 1.0 = unchanged.
        Assert.Equal(Fixed.MinValue, Fixed.MinValue * Fixed.One);
    }

    [Fact]
    public void Multiply_MinValueTimesMinusOne_Overflows()
    {
        // Negating MinValue overflows since |MinValue| = 2^31 which is just
        // outside the +Q32.32 representable range (max is 2^31 - 2^-32).
        Assert.Throws<OverflowException>(() => Fixed.MinValue * Fixed.MinusOne);
    }

    [Fact]
    public void Multiply_MinValueByMinValue_Overflows()
    {
        Assert.Throws<OverflowException>(() => Fixed.MinValue * Fixed.MinValue);
    }

    #endregion

    #region Division — basic

    [Fact]
    public void Divide_OneByTwo_EqualsHalf()
    {
        Assert.Equal(Fixed.Half, Fixed.One / Fixed.FromInt(2));
    }

    [Theory]
    [InlineData(6, 2, 3)]
    [InlineData(10, -2, -5)]
    [InlineData(-12, 4, -3)]
    [InlineData(-15, -3, 5)]
    public void Divide_IntegerCases(int dividend, int divisor, int expected)
    {
        Fixed result = Fixed.FromInt(dividend) / Fixed.FromInt(divisor);
        Assert.Equal(Fixed.FromInt(expected), result);
    }

    [Fact]
    public void Divide_ByZero_Throws()
    {
        Assert.Throws<DivideByZeroException>(() => Fixed.One / Fixed.Zero);
    }

    [Fact]
    public void Divide_ZeroByNonzero_IsZero()
    {
        Assert.Equal(Fixed.Zero, Fixed.Zero / Fixed.MaxValue);
    }

    [Fact]
    public void Divide_OneByThree_RoundTripsViaMultiply()
    {
        Fixed third = Fixed.One / Fixed.FromInt(3);
        Fixed product = third * Fixed.FromInt(3);
        // Q32.32 is finite-precision; 1/3 * 3 != 1 exactly. The error is
        // bounded by a few ULPs because of integer-truncation rounding.
        long delta = Fixed.One.RawValue - product.RawValue;
        Assert.InRange(delta, 0L, 4L);
    }

    [Fact]
    public void Divide_NegativeBySmall_NegativeResult()
    {
        Fixed result = Fixed.MinusOne / Fixed.FromInt(4);
        // -0.25 in Q32.32
        Assert.Equal(-0x4000_0000L, result.RawValue);
    }

    #endregion
}

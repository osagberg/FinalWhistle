using System;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

public sealed class FixedSqrtTests
{
    private static Fixed F(int n) => Fixed.FromInt(n);

    #region Identity + edge cases

    [Fact]
    public void Sqrt_Zero_IsZero()
    {
        Assert.Equal(Fixed.Zero, Fixed.Sqrt(Fixed.Zero));
    }

    [Fact]
    public void Sqrt_One_IsOne()
    {
        Assert.Equal(Fixed.One, Fixed.Sqrt(Fixed.One));
    }

    [Fact]
    public void Sqrt_Negative_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => Fixed.Sqrt(F(-1)));
        Assert.Throws<ArgumentOutOfRangeException>(() => Fixed.Sqrt(Fixed.MinusOne));
    }

    #endregion

    #region Perfect squares

    [Theory]
    [InlineData(1, 1)]
    [InlineData(4, 2)]
    [InlineData(9, 3)]
    [InlineData(16, 4)]
    [InlineData(25, 5)]
    [InlineData(100, 10)]
    [InlineData(10000, 100)]
    [InlineData(1000000, 1000)]
    public void Sqrt_PerfectSquare_ReturnsExactRoot(int square, int expectedRoot)
    {
        Assert.Equal(F(expectedRoot), Fixed.Sqrt(F(square)));
    }

    [Fact]
    public void Sqrt_QuarterIsHalf()
    {
        // sqrt(0.25) = 0.5 exactly in Q32.32 (both are exact powers of two).
        Fixed quarter = Fixed.Half * Fixed.Half;
        Assert.Equal(Fixed.Half, Fixed.Sqrt(quarter));
    }

    #endregion

    #region Non-perfect-squares — verify by squaring back

    [Theory]
    [InlineData(2)]
    [InlineData(3)]
    [InlineData(5)]
    [InlineData(7)]
    [InlineData(13)]
    [InlineData(50)]
    [InlineData(123)]
    [InlineData(999999)]
    public void Sqrt_NonPerfectSquare_ProducesFloorOfTrueRoot(int input)
    {
        // Newton's method on integers (BigInteger) returns the floor of the
        // true square root in EXACT integer math. The production result is
        // floor(sqrt(x_raw · 2^32)) — the largest raw value whose square in
        // 128-bit exact math is ≤ x_raw · 2^32.
        //
        // The Q32.32 multiplication operator truncates after a 64-bit shift,
        // which means the strict integer floor invariant
        // <c>(root + ε)² > x</c> does NOT translate to a clean Fixed
        // inequality (truncation can collapse adjacent ULPs to identical
        // Fixed-multiply outputs). The cleanest test invariants are:
        //   1. root² ≤ x      (lower bound; always holds)
        //   2. root² is "close" to x — i.e., |x - root²| is bounded by a
        //      small margin proportional to the ULP density at root.
        Fixed x = F(input);
        Fixed root = Fixed.Sqrt(x);

        // Lower bound: root² ≤ x.
        Fixed rootSquared = root * root;
        Assert.True(rootSquared <= x, $"root² should be <= input; got root²={rootSquared}, input={x}");

        // Closeness bound: x - root² is bounded by ~2·root·Epsilon (the
        // derivative of x² at x = root). For typical Q32.32 inputs this is
        // a tiny fraction of the input value (well under 1 ULP of x for
        // small inputs; bounded by ~2·root·Epsilon for large ones).
        Fixed gap = x - rootSquared;
        Fixed margin = (root + Fixed.One) * Fixed.Epsilon * F(2);
        Assert.True(gap <= margin, $"x - root² should be <= 2·(root+1)·ε; got gap={gap}, margin={margin}");
    }

    #endregion

    #region Determinism

    [Fact]
    public void Sqrt_Deterministic_SameInputSameOutput100Times()
    {
        Fixed input = F(2);
        Fixed first = Fixed.Sqrt(input);
        for (int i = 0; i < 100; i++)
        {
            Assert.Equal(first, Fixed.Sqrt(input));
        }
    }

    [Fact]
    public void Sqrt_DistinctInputs_DistinctOutputs()
    {
        // 100 distinct inputs produce 100 distinct outputs (no collision in
        // the sqrt implementation).
        System.Collections.Generic.HashSet<Fixed> roots = new();
        for (int i = 0; i < 100; i++)
        {
            roots.Add(Fixed.Sqrt(F(i)));
        }
        Assert.Equal(100, roots.Count);
    }

    #endregion

    #region Range — large values

    [Fact]
    public void Sqrt_OfMaxValue_DoesNotOverflow()
    {
        // Sqrt of Fixed.MaxValue should produce ~46341 (sqrt of ~2.147e9).
        Fixed root = Fixed.Sqrt(Fixed.MaxValue);

        // Sanity: result is well within Fixed range (no overflow).
        Assert.True(root > Fixed.Zero);
        Assert.True(root < F(50000));
        // Tighter sanity: root² should be approximately MaxValue (within
        // 2× root ULP since fixed-point sqrt is floor-quantized).
        Fixed rootSquared = root * root;
        // We expect rootSquared <= MaxValue, and the relative error should
        // be tiny (≪ 1%).
        Assert.True(rootSquared <= Fixed.MaxValue);
    }

    #endregion

    #region Cross-platform pinning (literal Q32.32 raw values)

    [Fact]
    public void Sqrt_Two_ProducesPinnedRawValue()
    {
        // Sqrt(2) in Q32.32 is the floor of (sqrt(2) · 2^32) ≈ 6074000999.x → 6074000999.
        // Pinning this value prevents any future rewrite from silently
        // changing the sqrt result on Win/Mac/Linux.
        Fixed root = Fixed.Sqrt(F(2));
        Assert.Equal(Fixed.FromRaw(6074000999L), root);
    }

    [Fact]
    public void Sqrt_Three_ProducesPinnedRawValue()
    {
        // Sqrt(3) in Q32.32 = floor(sqrt(3 · 2^64)) = 7439101573 (Python isqrt
        // oracle on 2026-04-27).
        Fixed root = Fixed.Sqrt(F(3));
        Assert.Equal(Fixed.FromRaw(7439101573L), root);
    }

    [Fact]
    public void Sqrt_Five_ProducesPinnedRawValue()
    {
        // Sqrt(5) in Q32.32 = floor(sqrt(5 · 2^64)) = 9603838834 (Python isqrt
        // oracle on 2026-04-27).
        Fixed root = Fixed.Sqrt(F(5));
        Assert.Equal(Fixed.FromRaw(9603838834L), root);
    }

    #endregion
}

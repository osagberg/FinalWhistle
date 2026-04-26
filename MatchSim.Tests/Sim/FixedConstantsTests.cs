using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Locks the Q32.32 constant invariants. The exact raw values are part of
/// the determinism contract — any change here is a schema-bump-class
/// supersession per design/specs/save-migration-fixtures.md.
/// </summary>
public sealed class FixedConstantsTests
{
    [Fact]
    public void Zero_RawValue_IsZero()
    {
        Assert.Equal(0L, Fixed.Zero.RawValue);
    }

    [Fact]
    public void One_RawValue_IsExactlyTwoToThe32()
    {
        Assert.Equal(0x1_0000_0000L, Fixed.One.RawValue);
    }

    [Fact]
    public void MinusOne_RawValue_IsNegativeTwoToThe32()
    {
        Assert.Equal(-0x1_0000_0000L, Fixed.MinusOne.RawValue);
    }

    [Fact]
    public void Half_RawValue_IsTwoToThe31()
    {
        Assert.Equal(0x0_8000_0000L, Fixed.Half.RawValue);
    }

    [Fact]
    public void MaxValue_RawValue_IsLongMaxValue()
    {
        Assert.Equal(long.MaxValue, Fixed.MaxValue.RawValue);
    }

    [Fact]
    public void MinValue_RawValue_IsLongMinValue()
    {
        Assert.Equal(long.MinValue, Fixed.MinValue.RawValue);
    }

    [Fact]
    public void Epsilon_RawValue_IsOneULP()
    {
        Assert.Equal(1L, Fixed.Epsilon.RawValue);
    }

    [Fact]
    public void FractionalBits_LockedAt32()
    {
        // The Q32.32 split is part of the determinism contract — test asserts
        // the constant matches the locked decision (2026-04-23 SPEC entry).
        Assert.Equal(32, Fixed.FractionalBits);
    }

    [Fact]
    public void OneRaw_MatchesShiftLeft()
    {
        Assert.Equal(1L << Fixed.FractionalBits, Fixed.OneRaw);
    }

    [Fact]
    public void CanonicalFractionalDigits_LockedAt10()
    {
        // 10 digits = full Q32.32 precision (1 ULP ≈ 2.328e-10).
        // Changing this is a schema-bump for FW-VAL-A-018 + corpus / save fixtures.
        Assert.Equal(10, Fixed.CanonicalFractionalDigits);
    }

    [Fact]
    public void Default_EqualsZero()
    {
        Assert.Equal(Fixed.Zero, default(Fixed));
    }

    [Fact]
    public void FromInt_Zero_EqualsZeroConstant()
    {
        Assert.Equal(Fixed.Zero, Fixed.FromInt(0));
    }

    [Fact]
    public void FromInt_One_EqualsOneConstant()
    {
        Assert.Equal(Fixed.One, Fixed.FromInt(1));
    }

    [Fact]
    public void FromInt_MinusOne_EqualsMinusOneConstant()
    {
        Assert.Equal(Fixed.MinusOne, Fixed.FromInt(-1));
    }

    [Fact]
    public void FromInt_IntMaxValue_FitsInQ32Range()
    {
        // int.MaxValue = 2^31 - 1, which is exactly the upper bound of the
        // Q32.32 integer range minus 1 ULP.
        Fixed maxInt = Fixed.FromInt(int.MaxValue);
        Assert.Equal((long)int.MaxValue << 32, maxInt.RawValue);
    }

    [Fact]
    public void FromInt_IntMinValue_FitsInQ32Range()
    {
        Fixed minInt = Fixed.FromInt(int.MinValue);
        Assert.Equal((long)int.MinValue << 32, minInt.RawValue);
    }

    [Fact]
    public void FromLong_OutOfRange_Throws()
    {
        Assert.Throws<System.OverflowException>(() => Fixed.FromLong((long)int.MaxValue + 1));
        Assert.Throws<System.OverflowException>(() => Fixed.FromLong((long)int.MinValue - 1));
    }

    [Fact]
    public void FromRaw_RoundTripsRawValue()
    {
        long raw = 0x12345678_9ABCDEF0L;
        Assert.Equal(raw, Fixed.FromRaw(raw).RawValue);
    }
}

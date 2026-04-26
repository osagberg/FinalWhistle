using System.Globalization;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Canonical decimal-string serialization for Fixed. Matches FW-VAL-A-018
/// in design/specs/content-pack-validation-contract.md (sim-affecting values
/// serialize as Q32.32 fixed-point decimal strings, NOT float literals).
/// Round-trip stability + invariant-culture posture.
/// </summary>
public sealed class FixedSerializationTests
{
    #region ToString

    [Fact]
    public void ToString_Zero_HasZeroDotTenZeros()
    {
        Assert.Equal("0.0000000000", Fixed.Zero.ToString());
    }

    [Fact]
    public void ToString_One_HasOneDotTenZeros()
    {
        Assert.Equal("1.0000000000", Fixed.One.ToString());
    }

    [Fact]
    public void ToString_MinusOne_HasMinusOneDotTenZeros()
    {
        Assert.Equal("-1.0000000000", Fixed.MinusOne.ToString());
    }

    [Fact]
    public void ToString_Half_HasZeroPointFive()
    {
        Assert.Equal("0.5000000000", Fixed.Half.ToString());
    }

    [Fact]
    public void ToString_NegativeHalf_HasMinusZeroPointFive()
    {
        Assert.Equal("-0.5000000000", (-Fixed.Half).ToString());
    }

    [Fact]
    public void ToString_UsesInvariantCulture_DotDecimalSeparator()
    {
        Assert.Contains(".", Fixed.Half.ToString(), System.StringComparison.Ordinal);
        Assert.DoesNotContain(",", Fixed.Half.ToString(), System.StringComparison.Ordinal);
    }

    [Fact]
    public void ToString_HasExactlyTenFractionalDigits()
    {
        string s = Fixed.Half.ToString();
        int dotIndex = s.IndexOf('.', System.StringComparison.Ordinal);
        Assert.True(dotIndex >= 0);
        Assert.Equal(Fixed.CanonicalFractionalDigits, s.Length - dotIndex - 1);
    }

    [Fact]
    public void ToString_DeterministicAcrossCallSites()
    {
        // Same value → same string. Locks the formatting determinism contract.
        Fixed value = Fixed.Half * Fixed.Half;
        Assert.Equal(value.ToString(), value.ToString());
    }

    #endregion

    #region Parse

    [Theory]
    [InlineData("0", 0L)]
    [InlineData("1", 0x1_0000_0000L)]
    [InlineData("-1", -0x1_0000_0000L)]
    [InlineData("0.5", 0x0_8000_0000L)]
    [InlineData("-0.5", -0x0_8000_0000L)]
    [InlineData("0.25", 0x0_4000_0000L)]
    [InlineData("2", 0x2_0000_0000L)]
    public void Parse_ProducesExpectedRawValue(string s, long expectedRaw)
    {
        Fixed result = Fixed.Parse(s);
        Assert.Equal(expectedRaw, result.RawValue);
    }

    [Fact]
    public void Parse_RejectsNullArgument()
    {
        Assert.Throws<System.ArgumentNullException>(() => Fixed.Parse(null!));
    }

    [Fact]
    public void Parse_RejectsGarbage()
    {
        Assert.Throws<System.FormatException>(() => Fixed.Parse("not-a-number"));
    }

    [Fact]
    public void Parse_RejectsCommaDecimal_InInvariantCulture()
    {
        // Invariant culture uses '.' for decimal separator. A culture-specific
        // string with ',' is rejected.
        Assert.Throws<System.FormatException>(() => Fixed.Parse("0,5"));
    }

    [Fact]
    public void Parse_RejectsScientificNotation()
    {
        Assert.Throws<System.FormatException>(() => Fixed.Parse("1e-1"));
    }

    [Fact]
    public void Parse_AcceptsCommaDecimal_WithExplicitCulture()
    {
        var culture = new CultureInfo("de-DE");
        Fixed result = Fixed.Parse("0,5", culture);
        Assert.Equal(Fixed.Half, result);
    }

    #endregion

    #region TryParse

    [Fact]
    public void TryParse_ValidInput_ReturnsTrue()
    {
        bool ok = Fixed.TryParse("3.14", out Fixed result);
        Assert.True(ok);
        Assert.NotEqual(Fixed.Zero, result);
    }

    [Fact]
    public void TryParse_NullInput_ReturnsFalseWithDefaultResult()
    {
        bool ok = Fixed.TryParse(null, out Fixed result);
        Assert.False(ok);
        Assert.Equal(Fixed.Zero, result);
    }

    [Fact]
    public void TryParse_GarbageInput_ReturnsFalse()
    {
        bool ok = Fixed.TryParse("definitely-not-a-number", out Fixed result);
        Assert.False(ok);
        Assert.Equal(Fixed.Zero, result);
    }

    [Fact]
    public void TryParse_OutOfRangeInput_ReturnsFalse()
    {
        // Far beyond Q32.32's ±2.147e9 range.
        bool ok = Fixed.TryParse("99999999999", out Fixed _);
        Assert.False(ok);
    }

    [Fact]
    public void TryParse_HugeDecimalInput_ReturnsFalseWithoutThrowing()
    {
        bool ok = Fixed.TryParse("100000000000000000000", out Fixed _);
        Assert.False(ok);
    }

    #endregion

    #region Round-trip stability

    [Theory]
    [InlineData("0")]
    [InlineData("1")]
    [InlineData("-1")]
    [InlineData("0.5")]
    [InlineData("-0.5")]
    [InlineData("0.25")]
    [InlineData("3.14159")]
    [InlineData("-2.71828")]
    [InlineData("1234.5678")]
    public void Parse_Then_ToString_Then_Parse_IsStable(string s)
    {
        Fixed first = Fixed.Parse(s);
        string emitted = first.ToString();
        Fixed second = Fixed.Parse(emitted);
        Assert.Equal(first.RawValue, second.RawValue);
    }

    [Fact]
    public void RoundTrip_Stability_AcrossManyValues()
    {
        // Spot-check a deterministic spread of raw values for round-trip.
        // Deliberate choice of values that exercise sign, integer part,
        // and fractional precision boundaries.
        long[] rawValues =
        {
            0L,
            1L,
            -1L,
            Fixed.OneRaw,
            -Fixed.OneRaw,
            Fixed.OneRaw >> 1, // half
            -(Fixed.OneRaw >> 1),
            123456789L,
            -987654321L,
            (long)int.MaxValue << 16, // big positive
            (long)int.MinValue << 16, // big negative
            long.MaxValue,
            long.MinValue,
        };

        foreach (long raw in rawValues)
        {
            Fixed first = Fixed.FromRaw(raw);
            string emitted = first.ToString();
            Fixed second = Fixed.Parse(emitted);
            Assert.Equal(first.RawValue, second.RawValue);
        }
    }

    #endregion
}

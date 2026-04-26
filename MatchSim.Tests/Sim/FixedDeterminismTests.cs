using System.Collections.Generic;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Determinism + ordering invariants. Same inputs → same hash + same ordering
/// across runs. Cross-platform parity is implicitly guaranteed by integer-only
/// math; the Tier-A Linux smoke run validates one platform; Win/Mac/Linux
/// matrix activates per design/production-pipeline.md cost-discipline once
/// Tick + Seed land and there's a corpus of canonical operations to compare.
/// </summary>
public sealed class FixedDeterminismTests
{
    [Fact]
    public void Equality_OperatorAndMethod_Agree()
    {
        Fixed a = Fixed.FromInt(42);
        Fixed b = Fixed.FromInt(42);
        Assert.True(a == b);
        Assert.True(a.Equals(b));
        Assert.True(a.Equals((object)b));
        Assert.False(a != b);
    }

    [Fact]
    public void Inequality_OperatorAndMethod_Agree()
    {
        Fixed a = Fixed.FromInt(42);
        Fixed b = Fixed.FromInt(43);
        Assert.True(a != b);
        Assert.False(a == b);
        Assert.False(a.Equals(b));
    }

    [Fact]
    public void GetHashCode_StableForEqualValues()
    {
        // Same logical value → same hash. Required for Dictionary / HashSet
        // determinism in sim code.
        Fixed a = Fixed.FromInt(7);
        Fixed b = Fixed.FromInt(7);
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
    }

    [Fact]
    public void GetHashCode_DifferentForDifferentRawValues()
    {
        // Not strictly required by the contract (collisions are allowed), but
        // for our int64-backed implementation, distinct raws should yield
        // distinct hashes for typical small values.
        Fixed a = Fixed.FromInt(7);
        Fixed b = Fixed.FromInt(8);
        Assert.NotEqual(a.GetHashCode(), b.GetHashCode());
    }

    [Fact]
    public void Comparison_TotalOrdering()
    {
        Fixed[] sorted =
        {
            Fixed.MinValue,
            Fixed.MinusOne,
            -Fixed.Half,
            Fixed.Zero,
            Fixed.Epsilon,
            Fixed.Half,
            Fixed.One,
            Fixed.MaxValue,
        };

        for (int i = 0; i < sorted.Length - 1; i++)
        {
            Assert.True(sorted[i] < sorted[i + 1], $"sorted[{i}] should be less than sorted[{i + 1}]");
            Assert.True(sorted[i] <= sorted[i + 1]);
            Assert.False(sorted[i] > sorted[i + 1]);
            Assert.False(sorted[i] >= sorted[i + 1]);
        }
    }

    [Fact]
    public void CompareTo_MatchesOperator()
    {
        Fixed a = Fixed.FromInt(3);
        Fixed b = Fixed.FromInt(5);
        Assert.True(a.CompareTo(b) < 0);
        Assert.True(b.CompareTo(a) > 0);
        Assert.Equal(0, a.CompareTo(a));
    }

    [Fact]
    public void CompareTo_NonGeneric_HandlesNullAndNonFixed()
    {
        Fixed a = Fixed.FromInt(3);
        var comparable = (System.IComparable)a;
        Assert.Equal(1, comparable.CompareTo(null));
        Assert.Throws<System.ArgumentException>(() => comparable.CompareTo("not a Fixed"));
    }

    [Fact]
    public void Arithmetic_DeterministicAcrossRepeatedCalls()
    {
        // Given identical inputs, repeated arithmetic produces identical
        // outputs — bit-for-bit. This is the architectural floor for
        // golden-replay-corpus replay determinism.
        Fixed a = Fixed.FromInt(7);
        Fixed b = Fixed.FromInt(13);

        long mul1 = (a * b).RawValue;
        long mul2 = (a * b).RawValue;
        Assert.Equal(mul1, mul2);

        long div1 = (a / b).RawValue;
        long div2 = (a / b).RawValue;
        Assert.Equal(div1, div2);

        long add1 = (a + b).RawValue;
        long add2 = (a + b).RawValue;
        Assert.Equal(add1, add2);
    }

    [Fact]
    public void HashSet_DistinguishesValues()
    {
        // Collection determinism — distinct Fixed values are distinct
        // dictionary/set entries.
        var set = new HashSet<Fixed>
        {
            Fixed.Zero,
            Fixed.One,
            Fixed.MinusOne,
            Fixed.Half,
        };

        Assert.Equal(4, set.Count);
        Assert.Contains(Fixed.Zero, set);
        Assert.Contains(Fixed.One, set);
        Assert.DoesNotContain(Fixed.FromInt(2), set);
    }
}

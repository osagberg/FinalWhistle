using System;
using System.Collections.Generic;
using System.Numerics;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Seed — deterministic 64-bit sim seed. Per ADR-0001 forbidden-nondeterminism
/// + ADR-0008 ViewerEvent.Seed + TECH_APPROACH §3.2: same
/// (matchSeed, tick, eventId) triple → same Seed, across runs and platforms.
/// Different triples → avalanching outputs.
/// </summary>
public sealed class SeedTests
{
    #region Constants + Construction

    [Fact]
    public void Zero_HasZeroValue()
    {
        Assert.Equal(0UL, Seed.Zero.Value);
    }

    [Fact]
    public void Default_EqualsZero()
    {
        Assert.Equal(Seed.Zero, default(Seed));
    }

    [Fact]
    public void Constructor_StoresValue()
    {
        var seed = new Seed(0xDEADBEEFCAFEBABEUL);
        Assert.Equal(0xDEADBEEFCAFEBABEUL, seed.Value);
    }

    [Fact]
    public void FromUInt64_StoresValue()
    {
        var seed = Seed.FromUInt64(0x1234567890ABCDEFUL);
        Assert.Equal(0x1234567890ABCDEFUL, seed.Value);
    }

    [Fact]
    public void FromUInt64_AcceptsZero()
    {
        Assert.Equal(Seed.Zero, Seed.FromUInt64(0UL));
    }

    [Fact]
    public void FromUInt64_AcceptsMaxValue()
    {
        Assert.Equal(ulong.MaxValue, Seed.FromUInt64(ulong.MaxValue).Value);
    }

    #endregion

    #region Derive — determinism

    [Fact]
    public void Derive_SameTriple_SameSeed()
    {
        // Locks the cross-run determinism contract: same inputs always
        // produce the same seed. Failure here = replay determinism is broken.
        Seed a = Seed.Derive(0x1234567890ABCDEFUL, Tick.FromSeconds(42), 7UL);
        Seed b = Seed.Derive(0x1234567890ABCDEFUL, Tick.FromSeconds(42), 7UL);
        Assert.Equal(a, b);
    }

    [Fact]
    public void Derive_PinnedTriple_LocksKnownSeedValue()
    {
        // Lock the SplitMix64-based derivation against future regression:
        // changing the mixer or the composition order without an explicit
        // SPEC supersession trips this test. The pinned seed value is
        // computed from (matchSeed=0xdeadbeefdeadbeef, tick=0, eventId=0)
        // — the corpus-spec smoke seed authoring it produces a stable value.
        Seed derived = Seed.Derive(0xDEADBEEFDEADBEEFUL, Tick.Zero, 0UL);
        // First-run-derived value; if this changes, mixer or composition
        // semantics changed — bump corpus + save fixtures via SPEC entry.
        ulong locked = ComputeReferenceSplitMix64Triple(0xDEADBEEFDEADBEEFUL, 0UL, 0UL);
        Assert.Equal(locked, derived.Value);
    }

    private static ulong ComputeReferenceSplitMix64Triple(ulong matchSeed, ulong tickValue, ulong eventId)
    {
        ulong h = matchSeed;
        h = ReferenceSplitMix64(h ^ tickValue);
        h = ReferenceSplitMix64(h ^ eventId);
        return h;
    }

    private static ulong ReferenceSplitMix64(ulong x)
    {
        unchecked
        {
            x = (x ^ (x >> 30)) * 0xBF58476D1CE4E5B9UL;
            x = (x ^ (x >> 27)) * 0x94D049BB133111EBUL;
            x = x ^ (x >> 31);
        }
        return x;
    }

    [Fact]
    public void Derive_DeterministicAcrossRepeatedCalls()
    {
        // Same inputs called 100 times produce the same output 100 times.
        Seed first = Seed.Derive(42UL, Tick.FromSeconds(7), 99UL);
        for (int i = 0; i < 100; i++)
        {
            Seed again = Seed.Derive(42UL, Tick.FromSeconds(7), 99UL);
            Assert.Equal(first, again);
        }
    }

    #endregion

    #region Derive — sensitivity

    [Fact]
    public void Derive_DifferentMatchSeed_DifferentSeed()
    {
        Seed a = Seed.Derive(1UL, Tick.FromSeconds(42), 7UL);
        Seed b = Seed.Derive(2UL, Tick.FromSeconds(42), 7UL);
        Assert.NotEqual(a, b);
    }

    [Fact]
    public void Derive_DifferentTick_DifferentSeed()
    {
        Seed a = Seed.Derive(0xDEADUL, Tick.FromSeconds(1), 7UL);
        Seed b = Seed.Derive(0xDEADUL, Tick.FromSeconds(2), 7UL);
        Assert.NotEqual(a, b);
    }

    [Fact]
    public void Derive_DifferentEventId_DifferentSeed()
    {
        Seed a = Seed.Derive(0xDEADUL, Tick.FromSeconds(42), 7UL);
        Seed b = Seed.Derive(0xDEADUL, Tick.FromSeconds(42), 8UL);
        Assert.NotEqual(a, b);
    }

    [Fact]
    public void Derive_OrderMatters()
    {
        // Composition is non-commutative — swapping matchSeed and eventId
        // produces a different seed. This is the right behavior because
        // matchSeed and eventId are conceptually distinct identifiers.
        Seed forward = Seed.Derive(matchSeed: 1UL, Tick.FromSeconds(42), eventId: 2UL);
        Seed reversed = Seed.Derive(matchSeed: 2UL, Tick.FromSeconds(42), eventId: 1UL);
        Assert.NotEqual(forward, reversed);
    }

    #endregion

    #region Derive — avalanche

    [Fact]
    public void Derive_OneBitMatchSeedFlip_AvalanchesOutput()
    {
        // SplitMix64 has good avalanche — flipping a single bit in any
        // input should flip roughly half the output bits. We don't assert
        // 32/64 exactly (statistical noise), but we require the Hamming
        // distance be at least 16 to catch outright passthrough or
        // weak-mixer regressions.
        Seed baseline = Seed.Derive(0x0UL, Tick.FromSeconds(42), 7UL);
        Seed flipped = Seed.Derive(0x1UL, Tick.FromSeconds(42), 7UL); // flip bit 0

        int hammingDistance = PopCount(baseline.Value ^ flipped.Value);
        Assert.True(
            hammingDistance >= 16,
            $"Expected ≥16 bit-flips on 1-bit input change, got {hammingDistance}");
    }

    [Fact]
    public void Derive_OneBitTickFlip_AvalanchesOutput()
    {
        Seed baseline = Seed.Derive(0xDEADUL, new Tick(0L), 7UL);
        Seed flipped = Seed.Derive(0xDEADUL, new Tick(1L), 7UL);
        int hammingDistance = PopCount(baseline.Value ^ flipped.Value);
        Assert.True(
            hammingDistance >= 16,
            $"Expected ≥16 bit-flips on 1-bit tick change, got {hammingDistance}");
    }

    [Fact]
    public void Derive_OneBitEventIdFlip_AvalanchesOutput()
    {
        Seed baseline = Seed.Derive(0xDEADUL, Tick.FromSeconds(42), 0UL);
        Seed flipped = Seed.Derive(0xDEADUL, Tick.FromSeconds(42), 1UL);
        int hammingDistance = PopCount(baseline.Value ^ flipped.Value);
        Assert.True(
            hammingDistance >= 16,
            $"Expected ≥16 bit-flips on 1-bit eventId change, got {hammingDistance}");
    }

    [Fact]
    public void Derive_DistributionAcrossSequentialEventIds_HasHighDistinctness()
    {
        // Across N consecutive eventIds with the same (matchSeed, tick),
        // we expect (effectively) N distinct seeds. Birthday-paradox
        // collisions on 64-bit space are vanishingly rare for small N.
        const int N = 1000;
        var seeds = new HashSet<Seed>(capacity: N);
        for (int i = 0; i < N; i++)
        {
            seeds.Add(Seed.Derive(0xCAFEUL, Tick.FromSeconds(42), (ulong)i));
        }
        Assert.Equal(N, seeds.Count);
    }

    private static int PopCount(ulong value)
    {
        // Manual popcount (BitOperations.PopCount is .NET-version-gated;
        // explicit implementation guarantees behavior across target frameworks).
        int count = 0;
        while (value != 0UL)
        {
            count += (int)(value & 1UL);
            value >>= 1;
        }
        return count;
    }

    #endregion

    #region Equality + Comparison

    [Fact]
    public void Equality_SameValue()
    {
        var a = new Seed(0xDEADBEEFUL);
        var b = new Seed(0xDEADBEEFUL);
        Assert.True(a == b);
        Assert.True(a.Equals(b));
        Assert.True(a.Equals((object)b));
        Assert.False(a != b);
    }

    [Fact]
    public void Equality_DifferentValues()
    {
        var a = new Seed(1UL);
        var b = new Seed(2UL);
        Assert.False(a == b);
        Assert.True(a != b);
        Assert.False(a.Equals(b));
    }

    [Fact]
    public void HashCode_StableForEqualValues()
    {
        var a = new Seed(0xCAFEBABEUL);
        var b = new Seed(0xCAFEBABEUL);
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
    }

    [Fact]
    public void Comparison_TotalOrdering()
    {
        Seed[] sorted =
        {
            Seed.Zero,
            new(1UL),
            new(0xCAFEUL),
            new(0xDEADBEEFUL),
            new(ulong.MaxValue),
        };
        for (int i = 0; i < sorted.Length - 1; i++)
        {
            Assert.True(sorted[i] < sorted[i + 1]);
            Assert.True(sorted[i] <= sorted[i + 1]);
            Assert.False(sorted[i] > sorted[i + 1]);
            Assert.False(sorted[i] >= sorted[i + 1]);
        }
    }

    [Fact]
    public void CompareTo_NonGeneric_HandlesNullAndNonSeed()
    {
        var seed = new Seed(42UL);
        var comparable = (System.IComparable)seed;
        Assert.Equal(1, comparable.CompareTo(null));
        Assert.Throws<ArgumentException>(() => comparable.CompareTo("not a Seed"));
    }

    #endregion

    #region ToString / Parse

    [Fact]
    public void ToString_Zero_IsZeroPrefixedSixteenHexZeros()
    {
        Assert.Equal("0x0000000000000000", Seed.Zero.ToString());
    }

    [Fact]
    public void ToString_CorpusSmokeSeed_MatchesCorpusSpec()
    {
        // golden-replay-corpus.md pins Tier-A smoke seed 0xdeadbeefdeadbeef.
        // Our ToString must emit that canonical lowercase form.
        var seed = new Seed(0xDEADBEEFDEADBEEFUL);
        Assert.Equal("0xdeadbeefdeadbeef", seed.ToString());
    }

    [Fact]
    public void ToString_AlwaysSixteenHexDigits()
    {
        // Even small values get padded to 16 digits — ensures the canonical
        // form is fixed-width and visually scannable in fixture diffs.
        var seed = new Seed(0x1UL);
        Assert.Equal("0x0000000000000001", seed.ToString());
    }

    [Fact]
    public void ToString_UsesLowercaseHex()
    {
        var seed = new Seed(0xABCDEFUL);
        string emitted = seed.ToString();
        // Must contain only lowercase hex digits after the 0x prefix.
        Assert.DoesNotContain("A", emitted, StringComparison.Ordinal);
        Assert.DoesNotContain("F", emitted, StringComparison.Ordinal);
        Assert.Contains("a", emitted, StringComparison.Ordinal);
        Assert.Contains("f", emitted, StringComparison.Ordinal);
    }

    [Theory]
    [InlineData("0x0", 0x0UL)]
    [InlineData("0x1", 0x1UL)]
    [InlineData("0xdeadbeefdeadbeef", 0xDEADBEEFDEADBEEFUL)]
    [InlineData("0xDEADBEEFDEADBEEF", 0xDEADBEEFDEADBEEFUL)] // case-insensitive parse
    [InlineData("0X42", 0x42UL)] // uppercase prefix
    [InlineData("ff", 0xFFUL)] // missing prefix is allowed for forgiveness
    [InlineData("0000000000000001", 0x1UL)] // padded no-prefix
    [InlineData("ffffffffffffffff", ulong.MaxValue)]
    public void Parse_AcceptedForms(string input, ulong expected)
    {
        Assert.Equal(expected, Seed.Parse(input).Value);
    }

    [Fact]
    public void Parse_RejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() => Seed.Parse(null!));
    }

    [Theory]
    [InlineData("")]
    [InlineData("0x")]
    [InlineData("not-a-hex-string")]
    [InlineData("0x10000000000000000")] // 17 hex digits — too long
    [InlineData("0x.5")]
    [InlineData("-0x1")]
    public void Parse_RejectsGarbage(string input)
    {
        Assert.Throws<FormatException>(() => Seed.Parse(input));
    }

    [Fact]
    public void TryParse_NullInput_ReturnsFalse()
    {
        Assert.False(Seed.TryParse(null, out Seed result));
        Assert.Equal(Seed.Zero, result);
    }

    [Fact]
    public void TryParse_EmptyInput_ReturnsFalse()
    {
        Assert.False(Seed.TryParse(string.Empty, out Seed result));
        Assert.Equal(Seed.Zero, result);
    }

    [Fact]
    public void TryParse_GarbageInput_ReturnsFalseWithoutThrowing()
    {
        Assert.False(Seed.TryParse("definitely-not-hex", out Seed _));
    }

    #endregion

    #region Round-trip stability

    [Fact]
    public void RoundTrip_AcrossSpreadOfValues()
    {
        ulong[] values =
        {
            0UL,
            1UL,
            0xCAFEUL,
            0xDEADBEEFUL,
            0xDEADBEEFCAFEBABEUL,
            0xFFFFFFFFFFFFFFFFUL,
            ulong.MaxValue / 2,
        };
        foreach (ulong v in values)
        {
            var first = new Seed(v);
            string emitted = first.ToString();
            Seed second = Seed.Parse(emitted);
            Assert.Equal(first, second);
        }
    }

    #endregion
}

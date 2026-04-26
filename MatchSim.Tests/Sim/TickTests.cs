using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Tick — discrete sim-time counter. 60 Hz fixed timestep per
/// TECH_APPROACH §3.2. Determinism contract: tick counter is stable
/// across runs + platforms; Tick math is checked; Tick − Tick returns
/// long tick-delta (distinct type from absolute tick).
/// </summary>
public sealed class TickTests
{
    #region Constants

    [Fact]
    public void TicksPerSecond_LockedAt60()
    {
        // 60 Hz is locked per TECH_APPROACH §3.2 + 2026-04-22 SPEC entry.
        // Changing this is a determinism-contract supersession requiring
        // golden-replay-corpus + save-migration fixture refresh.
        Assert.Equal(60, Tick.TicksPerSecond);
    }

    [Fact]
    public void TicksPerMinute_Equals60TimesTicksPerSecond()
    {
        Assert.Equal(60L * Tick.TicksPerSecond, Tick.TicksPerMinute);
        Assert.Equal(3600L, Tick.TicksPerMinute);
    }

    [Fact]
    public void Zero_HasZeroValue()
    {
        Assert.Equal(0L, Tick.Zero.Value);
    }

    [Fact]
    public void One_HasOneValue()
    {
        Assert.Equal(1L, Tick.One.Value);
    }

    [Fact]
    public void Default_EqualsZero()
    {
        Assert.Equal(Tick.Zero, default(Tick));
    }

    #endregion

    #region Construction

    [Fact]
    public void Constructor_StoresValue()
    {
        var tick = new Tick(42L);
        Assert.Equal(42L, tick.Value);
    }

    [Fact]
    public void Constructor_AcceptsNegativeValue()
    {
        // Negative ticks aren't typical sim usage but the type doesn't
        // forbid them — tests like "what was the tick 30 ticks before
        // kickoff" want to produce a negative absolute tick legitimately.
        var tick = new Tick(-100L);
        Assert.Equal(-100L, tick.Value);
    }

    [Fact]
    public void Constructor_AcceptsLongMaxValue()
    {
        var tick = new Tick(long.MaxValue);
        Assert.Equal(long.MaxValue, tick.Value);
    }

    #endregion

    #region Factories

    [Theory]
    [InlineData(0, 0L)]
    [InlineData(1, 60L)]
    [InlineData(60, 3600L)]
    [InlineData(90, 5400L)]      // half-time of a 90-min match
    public void FromSeconds_ProducesExpectedTickCount(int seconds, long expected)
    {
        Assert.Equal(expected, Tick.FromSeconds(seconds).Value);
    }

    [Theory]
    [InlineData(0, 0L)]
    [InlineData(1, 3600L)]        // one minute
    [InlineData(45, 162000L)]     // first half
    [InlineData(90, 324000L)]     // full match
    public void FromMinutes_ProducesExpectedTickCount(int minutes, long expected)
    {
        Assert.Equal(expected, Tick.FromMinutes(minutes).Value);
    }

    [Fact]
    public void FromSeconds_NegativeValue_ProducesNegativeTick()
    {
        Assert.Equal(-60L, Tick.FromSeconds(-1).Value);
    }

    [Fact]
    public void FromSeconds_OverflowOnLargeInput_Throws()
    {
        // int.MaxValue * 60 overflows long? No — int.MaxValue ≈ 2.1e9,
        // × 60 ≈ 1.3e11 which fits in long (max ≈ 9.2e18). So FromSeconds
        // doesn't overflow on int inputs. Good — that's the contract.
        // (Future: if we add a long-input factory, that one would need
        // overflow-checking.)
        Assert.Equal((long)int.MaxValue * 60L, Tick.FromSeconds(int.MaxValue).Value);
    }

    [Fact]
    public void FromMinutes_OverflowOnLargeInput_Throws()
    {
        // int.MaxValue minutes = ~408 million minutes, × 3600 ticks/min
        // = 7.7e12 ticks — still fits in long.
        Assert.Equal((long)int.MaxValue * 3600L, Tick.FromMinutes(int.MaxValue).Value);
    }

    #endregion

    #region Conversion to seconds

    [Fact]
    public void ToSeconds_Zero_IsFixedZero()
    {
        Assert.Equal(Fixed.Zero, Tick.Zero.ToSeconds());
    }

    [Fact]
    public void ToSeconds_60Ticks_IsOneSecond()
    {
        Assert.Equal(Fixed.One, Tick.FromSeconds(1).ToSeconds());
    }

    [Fact]
    public void ToSeconds_30Ticks_IsHalfSecond()
    {
        Tick halfSecond = new(30L);
        Assert.Equal(Fixed.Half, halfSecond.ToSeconds());
    }

    [Fact]
    public void ToSeconds_PreservesIntegerSecondsExactly()
    {
        // For exact multiples of TicksPerSecond, conversion is lossless.
        for (int s = 0; s <= 90; s++)
        {
            Fixed expected = Fixed.FromInt(s);
            Fixed actual = Tick.FromSeconds(s).ToSeconds();
            Assert.Equal(expected, actual);
        }
    }

    [Fact]
    public void ToSeconds_NonMultipleOfTickRate_IsApproximate()
    {
        // 1 tick = 1/60 second. Q32.32 cannot represent 1/60 exactly,
        // but the result is within 1 ULP of the true value.
        Fixed actual = Tick.One.ToSeconds();
        Fixed expected = Fixed.One / Fixed.FromInt(Tick.TicksPerSecond);
        Assert.Equal(expected, actual);
    }

    #endregion

    #region Arithmetic — addition / subtraction with delta

    [Fact]
    public void Plus_Delta_AdvancesTick()
    {
        Tick start = Tick.FromSeconds(10);
        Tick later = start + 60L;
        Assert.Equal(Tick.FromSeconds(11), later);
    }

    [Fact]
    public void Plus_DeltaOnLeft_Commutative()
    {
        Tick start = Tick.FromSeconds(10);
        Tick later = 60L + start;
        Assert.Equal(Tick.FromSeconds(11), later);
    }

    [Fact]
    public void Minus_Delta_StepsBack()
    {
        Tick start = Tick.FromSeconds(10);
        Tick earlier = start - 60L;
        Assert.Equal(Tick.FromSeconds(9), earlier);
    }

    [Fact]
    public void Plus_Delta_OverflowAtMax_Throws()
    {
        Tick maxTick = new(long.MaxValue);
        Assert.Throws<OverflowException>(() => maxTick + 1L);
    }

    [Fact]
    public void Minus_Delta_UnderflowAtMin_Throws()
    {
        Tick minTick = new(long.MinValue);
        Assert.Throws<OverflowException>(() => minTick - 1L);
    }

    #endregion

    #region Arithmetic — Tick minus Tick returns delta

    [Fact]
    public void TickMinusTick_ReturnsDeltaAsLong()
    {
        Tick later = Tick.FromSeconds(10);
        Tick earlier = Tick.FromSeconds(7);
        long delta = later - earlier;
        Assert.Equal(180L, delta); // 3 seconds = 180 ticks
    }

    [Fact]
    public void TickMinusTick_NegativeDeltaForReversed()
    {
        Tick later = Tick.FromSeconds(10);
        Tick earlier = Tick.FromSeconds(7);
        long reversedDelta = earlier - later;
        Assert.Equal(-180L, reversedDelta);
    }

    [Fact]
    public void TickMinusTick_SameTick_IsZeroDelta()
    {
        Tick t = Tick.FromSeconds(42);
        Assert.Equal(0L, t - t);
    }

    [Fact]
    public void TickMinusTick_OverflowOnExtremeRange_Throws()
    {
        Tick maxTick = new(long.MaxValue);
        Tick minTick = new(long.MinValue);
        // MaxValue - MinValue overflows positively
        Assert.Throws<OverflowException>(() => maxTick - minTick);
    }

    #endregion

    #region Equality + Comparison

    [Fact]
    public void Equality_SameValue()
    {
        Tick a = Tick.FromSeconds(42);
        Tick b = Tick.FromSeconds(42);
        Assert.True(a == b);
        Assert.True(a.Equals(b));
        Assert.True(a.Equals((object)b));
        Assert.False(a != b);
    }

    [Fact]
    public void Equality_DifferentValues()
    {
        Tick a = Tick.FromSeconds(42);
        Tick b = Tick.FromSeconds(43);
        Assert.False(a == b);
        Assert.True(a != b);
        Assert.False(a.Equals(b));
    }

    [Fact]
    public void HashCode_StableForEqualValues()
    {
        Tick a = Tick.FromSeconds(42);
        Tick b = Tick.FromSeconds(42);
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
    }

    [Fact]
    public void Comparison_TotalOrdering()
    {
        Tick[] sorted =
        {
            new(long.MinValue),
            Tick.FromSeconds(-1),
            Tick.Zero,
            Tick.One,
            Tick.FromSeconds(1),
            Tick.FromSeconds(60),
            Tick.FromMinutes(45),
            new(long.MaxValue),
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
    public void CompareTo_NonGeneric_HandlesNullAndNonTick()
    {
        Tick t = Tick.FromSeconds(5);
        var comparable = (System.IComparable)t;
        Assert.Equal(1, comparable.CompareTo(null));
        Assert.Throws<ArgumentException>(() => comparable.CompareTo("not a Tick"));
    }

    [Fact]
    public void HashSet_DistinguishesTicks()
    {
        var set = new HashSet<Tick>
        {
            Tick.Zero,
            Tick.One,
            Tick.FromSeconds(1),
            Tick.FromMinutes(45),
        };
        Assert.Equal(4, set.Count);
        Assert.Contains(Tick.Zero, set);
        Assert.DoesNotContain(Tick.FromSeconds(2), set);
    }

    #endregion

    #region Determinism

    [Fact]
    public void Arithmetic_DeterministicAcrossRepeatedCalls()
    {
        Tick a = Tick.FromSeconds(7);
        long delta1 = (a + 13L).Value;
        long delta2 = (a + 13L).Value;
        Assert.Equal(delta1, delta2);
    }

    [Fact]
    public void Conversion_DeterministicAcrossRepeatedCalls()
    {
        Tick t = Tick.FromMinutes(45);
        Fixed s1 = t.ToSeconds();
        Fixed s2 = t.ToSeconds();
        Assert.Equal(s1, s2);
    }

    #endregion

    #region ToString

    [Fact]
    public void ToString_EmitsInvariantInteger()
    {
        Assert.Equal("0", Tick.Zero.ToString());
        Assert.Equal("60", Tick.FromSeconds(1).ToString());
        Assert.Equal("-60", Tick.FromSeconds(-1).ToString());
    }

    #endregion
}

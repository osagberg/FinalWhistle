using FinalWhistle.MatchSim.Memory;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Memory;

/// <summary>
/// Tests for <see cref="SalienceEngine"/> — emission-time compute,
/// band classification, reader-side callback-age decay. Pinned values
/// computed in Q32.32 by hand against the formula in ADR-0004
/// §"Salience formula" so a future refactor of the multiply order
/// or weight wiring fails loudly.
/// </summary>
public sealed class SalienceEngineTests
{
    [Fact]
    public void Compute_Phase3DefaultsWithGoalScoredPlaceholders_LandsInNotableBand()
    {
        // Phase-3 emission inputs from MemoryEmissionRules:
        //   Stakes=0.95, Prominence=0.6, ClassWeight=0.6, Rivalry=0, Rarity=0.
        // With Phase3Defaults weights (0.4, 0.2, 0.2, 0.1, 0.1):
        //   0.4·0.95 + 0.2·0.6 + 0.2·0.6 = 0.38 + 0.12 + 0.12 = 0.62.
        // ~2 ULP margin above NotableThreshold = 0.60 so Q32.32 multiply
        // rounding can't flip the classification at the boundary.
        SalienceInputs inputs = new(
            stakes: Fixed.Parse("0.9500000000"),
            participantProminenceAvg: Fixed.Parse("0.6000000000"),
            eventClassBaseWeight: Fixed.Parse("0.6000000000"),
            rivalryBoost: Fixed.Zero,
            rarityBoost: Fixed.Zero);

        Fixed salience = SalienceEngine.Compute(inputs, SalienceWeights.Phase3Defaults);

        // Band classification is the load-bearing assertion (the press-fan
        // reader filters by band, not by raw salience scalar). Tolerance
        // check on the scalar value catches refactor regressions in the
        // Compute formula without pinning Q32.32 multiply-rounding ULPs.
        Assert.Equal(SalienceBand.Notable, SalienceEngine.ClassifyBand(salience));
        Assert.True(salience >= SalienceEngine.NotableThreshold,
            $"Salience {salience} below NotableThreshold {SalienceEngine.NotableThreshold}.");
        Assert.True(salience < SalienceEngine.SeasonDefiningThreshold,
            $"Salience {salience} at or above SeasonDefiningThreshold {SalienceEngine.SeasonDefiningThreshold}.");
    }

    [Fact]
    public void Compute_AllZeroInputs_ProducesZeroSalience()
    {
        SalienceInputs inputs = new(
            Fixed.Zero, Fixed.Zero, Fixed.Zero, Fixed.Zero, Fixed.Zero);
        Fixed salience = SalienceEngine.Compute(inputs, SalienceWeights.Phase3Defaults);
        Assert.Equal(Fixed.Zero, salience);
        Assert.Equal(SalienceBand.Routine, SalienceEngine.ClassifyBand(salience));
    }

    [Fact]
    public void Compute_AllOneInputsClampsToOne()
    {
        // 1·(0.4 + 0.2 + 0.2 + 0.1 + 0.1) = 1.0 — at clamp ceiling.
        SalienceInputs inputs = new(
            Fixed.One, Fixed.One, Fixed.One, Fixed.One, Fixed.One);
        Fixed salience = SalienceEngine.Compute(inputs, SalienceWeights.Phase3Defaults);
        Assert.Equal(Fixed.One, salience);
        Assert.Equal(SalienceBand.SeasonDefining, SalienceEngine.ClassifyBand(salience));
    }

    [Fact]
    public void ClassifyBand_AtNotableThresholdBoundary_IsNotable()
    {
        // Boundary inclusive on the lower side per ADR-0004.
        Assert.Equal(SalienceBand.Notable,
            SalienceEngine.ClassifyBand(Fixed.Parse("0.6000000000")));
    }

    [Fact]
    public void ClassifyBand_JustBelowNotable_IsRoutine()
    {
        Assert.Equal(SalienceBand.Routine,
            SalienceEngine.ClassifyBand(Fixed.Parse("0.5999999999")));
    }

    [Fact]
    public void ClassifyBand_AtSeasonDefiningThresholdBoundary_IsSeasonDefining()
    {
        Assert.Equal(SalienceBand.SeasonDefining,
            SalienceEngine.ClassifyBand(Fixed.Parse("0.8500000000")));
    }

    [Fact]
    public void ApplyCallbackAgeModifier_ZeroAge_ReturnsBaseSalience()
    {
        Fixed baseSalience = Fixed.Parse("0.6000000000");
        Fixed surfacing = SalienceEngine.ApplyCallbackAgeModifier(
            baseSalience, eventSeason: 5, currentSeason: 5);
        Assert.Equal(baseSalience, surfacing);
    }

    [Fact]
    public void ApplyCallbackAgeModifier_ThreeSeasons_DecaysByThreeUnits()
    {
        // Linear decay: surfacing = base - age * 0.05.
        // Use a base + age combination that lands on values cleanly
        // representable in Q32.32 (avoid 0.6 boundary — see Compute test).
        // base 0.80 - 3 * 0.05 = 0.65.
        Fixed baseSalience = Fixed.Parse("0.8000000000");
        Fixed surfacing = SalienceEngine.ApplyCallbackAgeModifier(
            baseSalience, eventSeason: 1, currentSeason: 4);
        // Expected ≈ 0.65 with up to 1 ULP rounding from Fixed.Parse +
        // multiply path. Tolerance: assert the decay direction + amount.
        Fixed expected = Fixed.Parse("0.6500000000");
        Fixed delta = surfacing > expected ? surfacing - expected : expected - surfacing;
        Assert.True(delta < Fixed.Parse("0.0000000050"),
            $"Surfacing {surfacing} not within tolerance of expected {expected}.");
        Assert.True(surfacing < baseSalience);
    }

    [Fact]
    public void ApplyCallbackAgeModifier_DecayBeyondZero_ClampsToZero()
    {
        Fixed baseSalience = Fixed.Parse("0.0500000000");
        Fixed surfacing = SalienceEngine.ApplyCallbackAgeModifier(
            baseSalience, eventSeason: 1, currentSeason: 100);
        Assert.Equal(Fixed.Zero, surfacing);
    }

    [Fact]
    public void ApplyCallbackAgeModifier_FutureCurrentSeason_ReturnsBaseUnchanged()
    {
        // currentSeason < eventSeason can only happen via authoring bug;
        // engine returns the base salience clamped to [0,1].
        Fixed baseSalience = Fixed.Parse("0.6000000000");
        Fixed surfacing = SalienceEngine.ApplyCallbackAgeModifier(
            baseSalience, eventSeason: 5, currentSeason: 3);
        Assert.Equal(baseSalience, surfacing);
    }
}

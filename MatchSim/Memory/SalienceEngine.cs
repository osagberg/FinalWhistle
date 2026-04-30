using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Memory;

/// <summary>
/// Pure-static salience compute engine per ADR-0004 §"Salience formula"
/// + §"Reader-side surfacing modifier." All arithmetic in
/// <see cref="Fixed"/> Q32.32 — no <c>double</c>, no platform-dependent
/// IEEE-754 corners. Reader-side modifiers (callback-age decay) are
/// recomputed per-surface; the persisted <see cref="MemoryEvent.Salience"/>
/// stays immutable.
/// </summary>
public static class SalienceEngine
{
    /// <summary>
    /// Phase-6 tuning seed for the Notable cutoff per
    /// <c>design/event-sourced-memory.md</c>. Salience &gt;= 0.60 routes
    /// to Notable band.
    /// </summary>
    public static readonly Fixed NotableThreshold = Fixed.Parse("0.6000000000");

    /// <summary>
    /// Phase-6 tuning seed for the SeasonDefining cutoff. Salience &gt;= 0.85
    /// routes to SeasonDefining band — eligible for season-ending
    /// narrative beats per ADR-0004 §"Compaction" hard-preserve rule.
    /// </summary>
    public static readonly Fixed SeasonDefiningThreshold = Fixed.Parse("0.8500000000");

    /// <summary>
    /// Per-season decay applied to surfacing salience reader-side. 5%
    /// decay per season elapsed since the event was emitted. Phase-6
    /// tuning target; lives here as the named constant rather than
    /// scattered across reader implementations.
    /// </summary>
    public static readonly Fixed CallbackAgeDecayPerSeason = Fixed.Parse("0.0500000000");

    /// <summary>
    /// Compute emission-time salience per ADR-0004 formula:
    /// <c>clamp(w_stakes·stakes + w_prominence·prominence +
    /// w_event_class·classWeight + w_rivalry·rivalryBoost +
    /// w_rarity·rarityBoost, 0, 1)</c>.
    /// </summary>
    public static Fixed Compute(SalienceInputs inputs, SalienceWeights weights)
    {
        Fixed sum = weights.WStakes * inputs.Stakes
            + weights.WProminence * inputs.ParticipantProminenceAvg
            + weights.WEventClass * inputs.EventClassBaseWeight
            + weights.WRivalry * inputs.RivalryBoost
            + weights.WRarity * inputs.RarityBoost;
        return Clamp01(sum);
    }

    /// <summary>
    /// Route a Q32.32 salience scalar to its <see cref="SalienceBand"/>
    /// per ADR-0004 §"Compaction" three-tier rule. <c>Routine</c> at
    /// salience &lt; 0.60; <c>Notable</c> at 0.60 &lt;= s &lt; 0.85;
    /// <c>SeasonDefining</c> at s &gt;= 0.85. Boundary inclusive on the
    /// lower side (matches the design-doc &gt;=).
    /// </summary>
    public static SalienceBand ClassifyBand(Fixed salience)
    {
        if (salience >= SeasonDefiningThreshold) return SalienceBand.SeasonDefining;
        if (salience >= NotableThreshold) return SalienceBand.Notable;
        return SalienceBand.Routine;
    }

    /// <summary>
    /// Apply the reader-side callback-age modifier to a base salience
    /// per ADR-0004 §"Reader `callback_age` modifier." Subtracts
    /// <c>(currentSeason - eventSeason) · CallbackAgeDecayPerSeason</c>
    /// from the base salience, clamped to <c>[0, 1]</c>. Result is
    /// surfacing-salience only — never persisted.
    /// </summary>
    /// <remarks>
    /// Two cases short-circuit through the <c>currentSeason &lt;= eventSeason</c>
    /// guard:
    /// <list type="bullet">
    ///   <item><description><c>currentSeason == eventSeason</c> (age 0):
    ///       same-season query; the arithmetic path would compute
    ///       <c>decay = 0 · 0.05 = 0</c> and produce the same result, so
    ///       the early return is purely an optimization.</description></item>
    ///   <item><description><c>currentSeason &lt; eventSeason</c>: querying
    ///       "the future" of an event — only possible in an authoring
    ///       bug. Returns base unchanged rather than applying negative
    ///       decay. Phase-3 readers never produce that case.</description></item>
    /// </list>
    /// Per pr-review-toolkit round-1 finding I2: the prior comment said
    /// only the <c>&lt;</c> case applied; corrected to describe both.
    /// </remarks>
    public static Fixed ApplyCallbackAgeModifier(
        Fixed baseSalience, ushort eventSeason, ushort currentSeason)
    {
        if (currentSeason <= eventSeason) return Clamp01(baseSalience);
        int age = currentSeason - eventSeason;
        Fixed decay = CallbackAgeDecayPerSeason * Fixed.FromInt(age);
        Fixed surfacing = baseSalience - decay;
        return Clamp01(surfacing);
    }

    private static Fixed Clamp01(Fixed value)
    {
        if (value < Fixed.Zero) return Fixed.Zero;
        if (value > Fixed.One) return Fixed.One;
        return value;
    }
}

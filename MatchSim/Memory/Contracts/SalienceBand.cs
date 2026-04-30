namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Salience bands per ADR-0004 §"Compaction — three tiers + per-season
/// quota" routing rule. Used by <c>SalienceEngine.ClassifyBand</c> + by
/// <see cref="ReaderQuery.MinBand"/> filtering. Numeric cutoffs (0.30 /
/// 0.60 / 0.85) are Phase-6 tuning seeds — they live in
/// <c>SalienceEngine</c>, not on this enum.
///
/// <para>
/// <strong>Ordering matters.</strong> Routine=0 &lt; Notable=1 &lt;
/// SeasonDefining=2 — readers filter via <c>band &gt;= MinBand</c>, so a
/// SeasonDefining event passes a Notable filter (more salient events are
/// always eligible for lower-threshold readers).
/// </para>
/// </summary>
public enum SalienceBand : byte
{
    /// <summary>Salience &lt; 0.60. Aggregated-only after compaction.</summary>
    Routine = 0,

    /// <summary>0.60 &lt;= salience &lt; 0.85. Compact-preserve after compaction.</summary>
    Notable = 1,

    /// <summary>Salience &gt;= 0.85. Hard-preserve after compaction; eligible for season-ending narrative beats.</summary>
    SeasonDefining = 2,
}

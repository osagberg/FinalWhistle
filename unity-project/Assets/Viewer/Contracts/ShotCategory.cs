namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// 7-shot vocabulary per <c>design/semantic-cinema.md</c> + ADR-0001
    /// §"ShotTypeSO schema" / ADR-0008 §"ShotTypeDefinition." Each
    /// <see cref="ShotTypeDefinition"/> belongs to exactly one category;
    /// the category is what the adapter modulates against (camera framing,
    /// duration envelope, transition rules).
    ///
    /// <para>
    /// <strong>Pinned numeric values; never reuse.</strong> Adding a
    /// category requires updating the design doc + the dots adapter +
    /// (when Phase-4+ ShotTypeSO loading lands) the validator.
    /// </para>
    /// </summary>
    public enum ShotCategory : byte
    {
        /// <summary>Sentinel — never valid in a constructed ShotTypeDefinition.</summary>
        None = 0,

        /// <summary>Wide framing showing tactical formation + space; low modulation.</summary>
        TacticalWide = 1,

        /// <summary>Diagonal cut following an attacking lane; medium modulation.</summary>
        DiagonalAttackLane = 2,

        /// <summary>Single-player focus; high focal-subject modulation.</summary>
        PlayerIsolation = 3,

        /// <summary>Two-player duel framing; medium-high modulation.</summary>
        DuelPanel = 4,

        /// <summary>Pass-to-shot impact framing; pinned to the shot beat.</summary>
        PassShotImpact = 5,

        /// <summary>Crowd-reaction cutaway after a high-stakes beat.</summary>
        CrowdReaction = 6,

        /// <summary>Aftermath freeze with overlay text; longest-duration framing.</summary>
        AftermathFreeze = 7,
    }
}

namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Emotional valence of a <see cref="MemoryEvent"/> per ADR-0004
/// §"`MemoryEvent` schema" emotion field. Emission-time choice; never
/// recomputed reader-side. Phase-3 only emits <see cref="Triumph"/> for
/// <see cref="EventClass.GoalScored"/>; Phase-4+ adds the negative
/// emotions (Shame on a conceded goal, Anger on a broken promise).
/// </summary>
public enum Emotion : byte
{
    /// <summary>Sentinel — never valid in a constructed MemoryEvent.</summary>
    None = 0,

    /// <summary>Goal scored, title won, big upset achieved. Positive register.</summary>
    Triumph = 1,

    /// <summary>Goal conceded, derby lost, relegation suffered. Negative register.</summary>
    Shame = 2,

    /// <summary>Surviving a tight match, escaping relegation. Tension-release register.</summary>
    Relief = 3,

    /// <summary>Promise broken, manager fired unjustly. Combative register.</summary>
    Anger = 4,

    /// <summary>Promotion attempt, signing of a star player. Anticipation register.</summary>
    Hope = 5,
}

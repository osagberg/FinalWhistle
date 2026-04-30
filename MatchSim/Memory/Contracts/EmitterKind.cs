namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Source-system kind for <see cref="EventEmitter"/>. Catalogued per
/// ADR-0004 §"`MemoryEvent` schema" emitter vocabulary. Phase-3 only
/// emits <see cref="Match"/>; the rest are Phase-4+ slots reserved so
/// the enum's pinned int values lock now.
/// </summary>
public enum EmitterKind : byte
{
    /// <summary>Sentinel — never valid in a constructed EventEmitter.</summary>
    None = 0,

    /// <summary>MatchSim runtime emission (goals, restarts, signatures).</summary>
    Match = 1,

    /// <summary>Contract negotiation events (PromiseMade / Broken / Kept). Phase-4+.</summary>
    Contract = 2,

    /// <summary>Press-conference + media events (PressConferenceWin / Loss). Phase-4+.</summary>
    Press = 3,

    /// <summary>Board-confidence shifts (BoardUltimatum / BoardConfidenceShift). Phase-4+.</summary>
    Board = 4,
}

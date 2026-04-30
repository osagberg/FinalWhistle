namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// MemoryEvent class enum per ADR-0004 §"Event class catalog". Stable
/// integer IDs for fast-path dictionary lookup; PascalCase names match
/// the design-doc enum exactly (cross-doc exact-match discipline). NEVER
/// reuse a pinned int value — schema bumps add new entries; conditional
/// drops migrate via `MigrationChain` per ADR-0004 §"Load-time forward
/// migration."
///
/// <para>
/// <strong>Phase-3 minimum subset</strong>: only <see cref="GoalScored"/>
/// is exercised by <c>MemoryEmissionRules</c> + <c>PressFanReader</c>.
/// The full ~42-entry starter catalog (per ADR-0004 + design/event-sourced
/// -memory.md §Event-class-catalog) lands at Phase-4+ when more emission
/// sources come online (signature-execution events, scout reports,
/// promise-tracking, etc.). Cross-doc exact-match constraint applies the
/// moment <c>SignatureAwakened</c> / <c>SignatureExecuted</c> are added.
/// </para>
/// </summary>
public enum EventClass
{
    /// <summary>Sentinel — never emitted. Catches default-init.</summary>
    None = 0,

    /// <summary>
    /// Ball crossed the goal-mouth plane and the score incremented for
    /// the team mapped to <c>KeyEvent.Side</c>. Phase-3 bridge maps
    /// <c>KeyEventKind.Goal</c> → this class.
    /// </summary>
    GoalScored = 1,
}

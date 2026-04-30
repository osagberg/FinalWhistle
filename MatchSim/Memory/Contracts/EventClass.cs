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

    /// <summary>
    /// Phase-3 minimum persistent-development event per SPEC line 145 +
    /// <c>design/breakthrough-moments.md</c> §"Trigger kinds" Kind 1.
    /// Fires when a player records their final-allowed signature fire
    /// of the match (firedCount reaches per-match cap). Phase-3 bridge
    /// maps <c>KeyEventKind.SignatureBreakthrough</c> → this class.
    /// Phase-3 salience compute (Stakes=1.0, Prominence=0.6, ClassWeight
    /// =0.9, rivalry+rarity=0) lands at 0.70 — Notable band. Permanence
    /// comes from the breakthrough tag's <c>ExpiryPolicy.Never</c>
    /// setting, NOT the band scalar; SeasonDefining requires Phase-4+
    /// rivalry/rarity wiring for contextually-relevant breakthroughs.
    ///
    /// <para>
    /// <strong>Not <c>SignatureAwakened</c>.</strong> ADR-0004's
    /// catalog reserves <c>SignatureAwakened</c> for the Phase-4+
    /// readiness-accumulator awakening lifecycle. Phase-3's
    /// cap-reach trigger is a deterministic fixture-driven minimum
    /// that demonstrates the persistent-development pattern without
    /// pulling forward Phase-4 lifecycle work. Phase 4 will add
    /// <c>SignatureAwakened = 3</c> as a separate value; the
    /// Phase-3 <see cref="SignatureBreakthrough"/> stays as the
    /// fixture-driven cap-reach event.
    /// </para>
    /// </summary>
    SignatureBreakthrough = 2,
}

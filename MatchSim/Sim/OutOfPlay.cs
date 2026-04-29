namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Phase-3 minimum out-of-play state per SPEC 2026-04-28 PitchRules
/// decisions-log entry. Tracks WHY play stopped + what the next restart
/// kind is. <see cref="InPlay"/> is the default-init value (zero) so a
/// fresh <see cref="MatchSimulationState"/> starts in-play without any
/// extra wiring.
///
/// <para>
/// <strong>No <c>KickOff</c> / <c>CenterRestart</c> value</strong> per the
/// 2026-04-28 decisions-log entry: goal restarts are immediate respawn at
/// center within the same tick the goal is detected. The "celebration tick
/// range" doesn't exist in Phase 3. If observers find that gamey, a future
/// SPEC decision adds the value (and runners would honor the transient
/// state for some number of ticks).
/// </para>
///
/// <para>
/// <strong>byte storage</strong> (1 byte canonical-state cost) per
/// <see cref="TeamSide"/> precedent. Default 0 = <see cref="InPlay"/>.
/// </para>
/// </summary>
public enum OutOfPlay : byte
{
    /// <summary>
    /// Ball is in play; players move freely; no restart pending.
    /// Default-init value.
    /// </summary>
    InPlay = 0,

    /// <summary>
    /// Ball crossed the goal line outside the goal mouth on the defending
    /// side; play resumes with a goal kick from the goal area corner. Phase-3
    /// simplification: emitted whenever the goal line is crossed without a
    /// goal — proper GoalKick / CornerKick disambiguation requires last-
    /// touched-by tracking which is Phase 4+ scope.
    /// </summary>
    GoalKick = 1,

    /// <summary>
    /// Ball crossed a touchline; play resumes with a throw-in at the
    /// crossing point on the touchline. Opposite-of-last-touched team
    /// gets the throw-in (Phase 4+ — Phase 3 omits last-touched tracking).
    /// </summary>
    ThrowIn = 2,

    /// <summary>
    /// Ball crossed the goal line outside the goal mouth on the attacking
    /// side; play resumes with a corner kick. Phase 4+ activates this
    /// distinct from <see cref="GoalKick"/> once last-touched-by tracking
    /// lands; Phase 3 always classifies a non-goal goal-line crossing as
    /// <see cref="GoalKick"/>.
    /// </summary>
    CornerKick = 3,
}

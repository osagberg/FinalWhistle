namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Discriminator for entries in <see cref="MatchSimulationState.KeyEvents"/>.
/// Per SPEC 2026-04-28 PitchRules decisions-log entry: append-only stream
/// of significant match events that the golden-replay-corpus spec's
/// <c>key_event_hashes</c> field hashes for replay verification. Phase-3
/// vocabulary covers goals + the three out-of-play restart kinds.
///
/// <para>
/// <strong>Numeric values are pinned; never reuse.</strong> Like the
/// EventClass enum policy from ADR-0004 §Versioning, Phase 4+ additions
/// (kickoff celebration, foul, card, sub) take new values; existing values
/// stay stable so corpus fixtures encoded today remain decodable.
/// </para>
///
/// <para>
/// Default-init <see cref="None"/> = 0 is reserved as a sentinel — a
/// <see cref="KeyEvent"/> emitted with <see cref="None"/> would indicate a
/// bug in the runner (forgot to set the kind). <see cref="KeyEvent"/>
/// constructor rejects it.
/// </para>
/// </summary>
public enum KeyEventKind : byte
{
    /// <summary>Sentinel — never emitted. Default-init catches uninitialized state.</summary>
    None = 0,

    /// <summary>Ball crossed the goal line within the goal mouth; score incremented; ball respawns at center for the immediate restart.</summary>
    Goal = 1,

    /// <summary>Ball crossed the goal line on the defending side outside the goal mouth; goal-kick restart. Phase-3 also covers what would be CornerKick once last-touched tracking lands Phase 4+.</summary>
    GoalKickRestart = 2,

    /// <summary>Ball crossed a touchline; throw-in restart at the crossing point.</summary>
    ThrowInRestart = 3,

    /// <summary>Ball crossed the goal line on the attacking side outside the goal mouth; corner-kick restart. Phase-4 activation; Phase-3 emits <see cref="GoalKickRestart"/> for all non-goal goal-line crossings.</summary>
    CornerKickRestart = 4,

    /// <summary>
    /// Phase-3 active signature #20 fired: <em>Low cutback from the byline</em>.
    /// Carrier (Winger role-family) drives near the byline + cuts the ball
    /// back. <see cref="KeyEvent.JerseyNumber"/> = the carrier's jersey;
    /// <see cref="KeyEvent.Position"/> = the carrier's pitch position at
    /// fire-time. <see cref="KeyEvent.Side"/> = the carrier's team. Pairs
    /// with the parallel <see cref="MatchSimulationState.SignatureRecipes"/>
    /// stream that carries the presentation-recipe metadata for the
    /// dots-adapter consumption via <c>Viewer.EventBridge</c> (Phase-3
    /// next semantic-slice item).
    /// </summary>
    SignatureExecuted_LowCutback = 5,

    /// <summary>
    /// Phase-3 active signature #22 fired: <em>Blind-side near-post run</em>.
    /// Carrier (Striker role-family) curves run off defender's shoulder to
    /// near post during a wide ball-delivery moment.
    /// </summary>
    SignatureExecuted_BlindSideNearPostRun = 6,

    /// <summary>
    /// Phase-3 active signature #13 fired: <em>First-time diagonal switch</em>.
    /// Carrier (CentralMidfielder role-family) plays a one-touch
    /// cross-field switch from the middle third while the ball is moving.
    /// </summary>
    SignatureExecuted_FirstTimeDiagonalSwitch = 7,
}

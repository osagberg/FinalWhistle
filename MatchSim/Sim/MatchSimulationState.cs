using System;
using System.Collections.Generic;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Mutable production state for the Phase-3 deterministic match loop. The
/// arrays are intentionally mutable because the runner overwrites player
/// snapshots in place each tick; callers must preserve roster order.
///
/// <para>
/// <strong>Phase-3 PitchRules extensions</strong> per SPEC 2026-04-28
/// PitchRules decisions-log entry: <see cref="HomeScore"/> /
/// <see cref="AwayScore"/> (byte; capacity for any realistic Phase-3 match),
/// <see cref="OutOfPlay"/> (per-tick transient flag), and
/// <see cref="KeyEvents"/> (append-only stream of significant events
/// canonically encoded for replay-corpus hashing). All four fields are
/// canonical state — see <c>MatchCanonicalState.Write</c>.
/// </para>
/// </summary>
public sealed class MatchSimulationState
{
    /// <summary>Current absolute simulation tick.</summary>
    public Tick CurrentTick { get; set; }

    /// <summary>Current canonical ball state.</summary>
    public BallState Ball { get; set; }

    /// <summary>Home players in stable roster order, length 11.</summary>
    public PlayerState[] HomeTeam { get; }

    /// <summary>Away players in stable roster order, length 11.</summary>
    public PlayerState[] AwayTeam { get; }

    /// <summary>
    /// Home team score (number of goals scored). byte capacity = 0-255;
    /// the realistic Phase-3 ceiling is ~10-15 per side. <see cref="MatchRules.Step"/>
    /// throws if this would overflow rather than silently wrap.
    /// </summary>
    public byte HomeScore { get; set; }

    /// <summary>Away team score. Same byte-capacity / overflow contract as <see cref="HomeScore"/>.</summary>
    public byte AwayScore { get; set; }

    /// <summary>
    /// Per-tick transient flag set by <see cref="MatchRules.Step"/> when an
    /// out-of-play event fires THIS tick. Reset to <see cref="OutOfPlay.InPlay"/>
    /// at the start of every <see cref="MatchRules.Step"/> call. The
    /// persistent record of "what restarts have happened" lives in
    /// <see cref="KeyEvents"/>; this flag exists for tick-local consumers
    /// (the dots viewer adapter overlays the restart marker on the tick the
    /// event fires).
    /// </summary>
    public OutOfPlay OutOfPlay { get; set; }

    /// <summary>
    /// Append-only stream of significant match events: goals + restart
    /// emissions + Phase-3 signature executions. Entries are written in
    /// canonical order during <see cref="MatchRules.Step"/> +
    /// <see cref="SignatureRules.Step"/>; never removed or reordered. The
    /// golden-replay-corpus spec's <c>key_event_hashes</c> field hashes the
    /// canonical encoding of this list per replay seed for Tier-A
    /// verification.
    /// </summary>
    public List<KeyEvent> KeyEvents { get; }

    /// <summary>
    /// Parallel append-only stream of presentation-recipe metadata for
    /// signature-execution <see cref="KeyEvent"/>s. Read by
    /// <c>Viewer.EventBridge</c> (Phase-3 next semantic-slice item) which
    /// translates each entry to a <c>ViewerEvent</c> per ADR-0008.
    ///
    /// <para>
    /// <strong>Not canonical state.</strong> Excluded from
    /// <c>MatchCanonicalState.Write</c>. Coupling presentation
    /// metadata to the canonical hash would mean any overlay-text or
    /// shot-recipe change invalidates the corpus fixture — wrong axis
    /// (canonical = gameplay outcomes; recipes = derived display data).
    /// Each entry's <c>KeyEventIndex</c> points back into
    /// <see cref="KeyEvents"/> so the bridge can correlate.
    /// </para>
    /// </summary>
    public List<SignatureExecution> SignatureRecipes { get; }

    /// <summary>
    /// Per-match signature cooldown + fire-count tracker. Persisted on
    /// <see cref="MatchSimulationState"/> (allocated once at construction)
    /// so that chunked-tick callers — viewer/replay loops that drive
    /// <c>MatchSimulationRunner.RunTicks</c> in small batches — keep
    /// per-match cooldown windows + max-fire caps across calls. Re-allocating
    /// inside each <c>RunTicks</c> invocation would silently let signatures
    /// fire past their documented per-match cap (Codex round-9 P1).
    ///
    /// <para>
    /// <strong>Not canonical state.</strong> Excluded from
    /// <c>MatchCanonicalState.Write</c> for the same reason
    /// <see cref="SignatureRecipes"/> is excluded — it's a derived
    /// runtime-only tracker, reconstructible from the canonical KeyEvents
    /// stream + signature config. Pinned-hash determinism is unaffected.
    /// </para>
    /// </summary>
    public SignatureCooldownState SignatureCooldown { get; }

    public MatchSimulationState(
        Tick currentTick,
        BallState ball,
        PlayerState[] homeTeam,
        PlayerState[] awayTeam)
    {
        HomeTeam = CopyAndValidateTeam(homeTeam, nameof(homeTeam));
        AwayTeam = CopyAndValidateTeam(awayTeam, nameof(awayTeam));
        CurrentTick = currentTick;
        Ball = ball;
        HomeScore = 0;
        AwayScore = 0;
        OutOfPlay = OutOfPlay.InPlay;
        KeyEvents = new List<KeyEvent>();
        SignatureRecipes = new List<SignatureExecution>();
        SignatureCooldown = new SignatureCooldownState();
    }

    /// <summary>
    /// Build the canonical Month-3 smoke fixture initial state: 22 players at
    /// archetype formation positions, ball supplied by caller, tick supplied
    /// by caller.
    /// </summary>
    public static MatchSimulationState FromArchetypeFormations(
        Tick currentTick,
        BallState ball,
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype)
    {
        if (homeArchetype is null)
        {
            throw new ArgumentNullException(nameof(homeArchetype));
        }
        if (awayArchetype is null)
        {
            throw new ArgumentNullException(nameof(awayArchetype));
        }

        PlayerState[] homeTeam = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        PlayerState[] awayTeam = new PlayerState[MatchCanonicalState.PlayersPerTeam];

        foreach (FormationSlot homeSlot in homeArchetype.Formation)
        {
            int rosterIndex = homeSlot.RosterSlot - 1;
            homeTeam[rosterIndex] = new PlayerState(
                position: homeSlot.HomeBasePosition,
                velocity: Vector3Fixed.Zero,
                jerseyNumber: homeSlot.RosterSlot,
                side: TeamSide.Home);
        }

        foreach (FormationSlot awaySlot in awayArchetype.Formation)
        {
            int rosterIndex = awaySlot.RosterSlot - 1;
            awayTeam[rosterIndex] = new PlayerState(
                position: awaySlot.AwayBasePosition(),
                velocity: Vector3Fixed.Zero,
                jerseyNumber: awaySlot.RosterSlot,
                side: TeamSide.Away);
        }

        return new MatchSimulationState(currentTick, ball, homeTeam, awayTeam);
    }

    /// <summary>
    /// C4 Phase-3 polish (2026-05-11): construct an initial state pre-arranged
    /// to trigger the LowCutback signature on tick 0. Home roster slot 7
    /// (Winger per direct-pressing.yaml + low-block-counter.yaml; LowCutback
    /// affinity in the Phase-3 IdentityPacket fixtures) is placed at the
    /// attacking byline (X = ~50m, just inside the goal line at ±52.5m) in
    /// a wide channel (Z = 22m, beyond the 20m wide-channel threshold)
    /// with lateral velocity (Z-axis 2 m/s — above the 1 m/s minimum
    /// lateral-speed gate). Ball placed coincident so the carrier-on-ball
    /// possession check passes. Other 21 players at formation positions.
    ///
    /// <para>
    /// Closes the C1 audit finding (bb136866) that natural smoke-fixture
    /// play produces SignatureRecipes.Count=0: Slice-7 visual surfaces
    /// (selection rings / motion lines / pressure tint / Tension cadence)
    /// all depend on signature events, so without a fixture that satisfies
    /// the spatial+role+velocity gates, those visuals never fire naturally.
    /// This fixture sits alongside the smoke fixture (0xdeadbeefdeadbeef);
    /// the seed convention is 0xfeedbeefcafefade ("primed-for-low-cutback").
    /// </para>
    ///
    /// <para>
    /// Trigger geometry (matching SignatureConfig.Phase3Defaults):
    /// <list type="bullet">
    ///   <item><description>BylineProximityMetres = 3 m → winger X = 50 m, distance
    ///     to GoalLineX (52.5) = 2.5 m. PASS.</description></item>
    ///   <item><description>WideChannelZThreshold = 20 m → winger Z = 22 m. PASS.</description></item>
    ///   <item><description>MinLateralSpeed = 1 m/s → winger Velocity.Z = -2 m/s. PASS.</description></item>
    ///   <item><description>IsCarrier check → ball at winger's position, within
    ///     PlayerKinematics.Radius (0.5 m). PASS.</description></item>
    /// </list>
    /// </para>
    /// </summary>
    public static MatchSimulationState FromLowCutbackPrimedFixture(
        Tick currentTick,
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype)
    {
        if (homeArchetype is null) throw new ArgumentNullException(nameof(homeArchetype));
        if (awayArchetype is null) throw new ArgumentNullException(nameof(awayArchetype));

        // Locked Phase-3 winger position for the primed fixture. Numbers
        // chosen to clear SignatureConfig.Phase3Defaults gates with margin
        // (X 2.5m inside byline; Z 2m inside wide channel; vel.Z 1 m/s above
        // minimum) so the LowCutback gate passes deterministically + any
        // future gate-coefficient nudge has slack before tripping this.
        var primedWingerPosition = new Vector3Fixed(
            Fixed.FromInt(50), Fixed.Zero, Fixed.FromInt(22));
        var primedWingerVelocity = new Vector3Fixed(
            Fixed.Zero, Fixed.Zero, -Fixed.FromInt(2));

        PlayerState[] homeTeam = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        PlayerState[] awayTeam = new PlayerState[MatchCanonicalState.PlayersPerTeam];

        foreach (FormationSlot homeSlot in homeArchetype.Formation)
        {
            int rosterIndex = homeSlot.RosterSlot - 1;
            // Slot 6 = RM (Winger) per direct-pressing 4-4-2: Jonas Pielke,
            // RoleFamily=Winger, IdentityPacket affinity for
            // fwh.core:signature.low-cutback-from-byline. Pre-place at the
            // byline with lateral velocity. (Slot 7 is RCM/CentralMidfielder
            // and has FirstTimeDiagonalSwitch affinity — different gate.)
            if (homeSlot.RosterSlot == 6)
            {
                homeTeam[rosterIndex] = new PlayerState(
                    position: primedWingerPosition,
                    velocity: primedWingerVelocity,
                    jerseyNumber: 6,
                    side: TeamSide.Home);
            }
            else
            {
                homeTeam[rosterIndex] = new PlayerState(
                    position: homeSlot.HomeBasePosition,
                    velocity: Vector3Fixed.Zero,
                    jerseyNumber: homeSlot.RosterSlot,
                    side: TeamSide.Home);
            }
        }

        foreach (FormationSlot awaySlot in awayArchetype.Formation)
        {
            int rosterIndex = awaySlot.RosterSlot - 1;
            awayTeam[rosterIndex] = new PlayerState(
                position: awaySlot.AwayBasePosition(),
                velocity: Vector3Fixed.Zero,
                jerseyNumber: awaySlot.RosterSlot,
                side: TeamSide.Away);
        }

        // Ball coincident with the winger so the carrier-on-ball check passes.
        // Zero velocity: the LowCutback gate doesn't require ball motion (it's
        // a cutback intent gate driven by the carrier's lateral velocity, not
        // the ball's). The pass-the-ball CarrierKickGate (2 m/s ball velocity)
        // is satisfied because ball.Velocity = 0 < gate, but the LowCutback
        // emit fires BEFORE BehaviorTreeRunner can run any pass logic (BTs
        // run before signatures? actually after — see MatchSimulationRunner
        // step order: BT.Tick × 2 → Actuator × 22 → ApplyKick → BallPhysics
        // → MatchRules → SignatureRules → Tick+1). At tick 0 the carrier may
        // emit a kick AT THE SAME TICK as the signature fires; that's fine
        // because SignatureRules reads the CURRENT player position (which
        // hasn't moved yet) + the CURRENT ball velocity (which would have
        // just been kicked but the signature gate is on the carrier's
        // velocity, not the ball's). Cooldown=180 ticks for LowCutback so
        // re-fires are gated; max 3 fires per match.
        var ball = new BallState(primedWingerPosition, Vector3Fixed.Zero, Vector3Fixed.Zero);

        return new MatchSimulationState(currentTick, ball, homeTeam, awayTeam);
    }

    private static PlayerState[] CopyAndValidateTeam(PlayerState[] team, string paramName)
    {
        if (team is null)
        {
            throw new ArgumentNullException(paramName);
        }
        if (team.Length != MatchCanonicalState.PlayersPerTeam)
        {
            throw new ArgumentException(
                $"{paramName} must contain exactly {MatchCanonicalState.PlayersPerTeam} players; got {team.Length}.",
                paramName);
        }

        PlayerState[] copy = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        Array.Copy(team, copy, MatchCanonicalState.PlayersPerTeam);
        return copy;
    }
}

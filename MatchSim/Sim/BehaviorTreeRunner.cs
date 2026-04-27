using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Pure-deterministic per-tick runner that converts a
/// <see cref="BehaviorTreeArchetype"/> + match snapshot into a
/// <see cref="PlayerCommand"/> per own-team player.
///
/// <para>The Month-3 heuristic has three modes:</para>
///
/// <list type="bullet">
///   <item><description><strong>Press</strong> — if the opponents have
///     possession AND an own-team player is within
///     <see cref="BehaviorTreeArchetype.PressRadiusMetres"/> of the ball,
///     that player heads toward the ball at <c>MaxSpeed</c>.</description></item>
///   <item><description><strong>Build-up</strong> — if our team has
///     possession, the player nearest the ball heads toward the opponent
///     goal at <c>MaxSpeed * BuildupSpeedFactor</c>; other players hold
///     their formation base position.</description></item>
///   <item><description><strong>Hold shape</strong> — otherwise, the
///     player heads to their formation base position (mirrored if Away)
///     at <c>MaxSpeed * 0.5</c> (jog).</description></item>
/// </list>
///
/// <para>
/// <strong>Determinism:</strong> the runner is order-stable + sqrt-free
/// where possible. Reads the inputs but does not allocate; commands are
/// written into the caller-provided buffer.
/// </para>
///
/// <para>
/// <strong>Possession resolution:</strong> "team in possession" is the
/// team whose nearest player is strictly closer to the ball. Ties resolve
/// to the opponent (defensive default; conservative). Match-loop layers
/// may eventually supersede with a more nuanced contest-resolution policy.
/// </para>
/// </summary>
public static class BehaviorTreeRunner
{
    /// <summary>
    /// Tick the BT for one team. Writes 11 <see cref="PlayerCommand"/>s
    /// into <paramref name="commandsOut"/> in the same order as
    /// <paramref name="ownTeam"/>.
    /// </summary>
    /// <param name="ball">Current ball state.</param>
    /// <param name="ownTeam">The team this archetype is being run for. Length must be 11.</param>
    /// <param name="opponents">The opposing team. Length must be 11.</param>
    /// <param name="side">Which side <paramref name="ownTeam"/> is — drives X-axis mirroring of formation positions.</param>
    /// <param name="archetype">The tactical archetype to apply.</param>
    /// <param name="kinematics">Per-player kinematic parameters. (Currently homogeneous; per-player tuning is Phase 4.)</param>
    /// <param name="commandsOut">Output buffer; must have length ≥ 11. Written in own-team-index order.</param>
    public static void Tick(
        BallState ball,
        ReadOnlySpan<PlayerState> ownTeam,
        ReadOnlySpan<PlayerState> opponents,
        TeamSide side,
        BehaviorTreeArchetype archetype,
        PlayerKinematics kinematics,
        Span<PlayerCommand> commandsOut)
    {
        if (archetype is null)
        {
            throw new ArgumentNullException(nameof(archetype));
        }
        if (ownTeam.Length != 11)
        {
            throw new ArgumentException($"ownTeam must have exactly 11 players; got {ownTeam.Length}.", nameof(ownTeam));
        }
        if (opponents.Length != 11)
        {
            throw new ArgumentException($"opponents must have exactly 11 players; got {opponents.Length}.", nameof(opponents));
        }
        if (commandsOut.Length < 11)
        {
            throw new ArgumentException($"commandsOut must hold at least 11 commands; got {commandsOut.Length}.", nameof(commandsOut));
        }
        if (side is not (TeamSide.Home or TeamSide.Away))
        {
            throw new ArgumentOutOfRangeException(nameof(side), side, "TeamSide must be Home or Away.");
        }

        // Resolve possession: which team's nearest player is closer to the ball.
        Fixed pressRadiusSq = archetype.PressRadiusMetres * archetype.PressRadiusMetres;
        bool ownTeamInPossession = ResolveOwnTeamInPossession(ball, ownTeam, opponents);

        Vector3Fixed opponentGoal = OpponentGoalPosition(side);

        // Identify our own-team player nearest the ball — used for the
        // build-up "head to opponent goal" command.
        int nearestToBallIndex = NearestPlayerIndex(ball.Position, ownTeam);

        for (int i = 0; i < 11; i++)
        {
            PlayerState player = ownTeam[i];
            FormationSlot slot = FormationSlotForRosterIndex(archetype, (byte)(i + 1));
            Vector3Fixed basePosition = side == TeamSide.Home ? slot.HomeBasePosition : slot.AwayBasePosition();

            // Press check: any opponent within PressRadius of this player AND
            // opponents have possession → press the ball.
            Fixed distFromPlayerToBallSq = Vector3Fixed.DistanceSquared(player.Position, ball.Position);
            bool inPressRange = distFromPlayerToBallSq <= pressRadiusSq;

            if (!ownTeamInPossession && inPressRange)
            {
                // Press: head to ball at full speed.
                commandsOut[i] = new PlayerCommand(ball.Position, kinematics.MaxSpeed);
                continue;
            }

            if (ownTeamInPossession && i == nearestToBallIndex)
            {
                // Build-up: ball-carrier heads toward opponent goal at the
                // archetype's build-up speed.
                commandsOut[i] = new PlayerCommand(opponentGoal, kinematics.MaxSpeed * archetype.BuildupSpeedFactor);
                continue;
            }

            // Hold shape: head toward base position at jog (half speed).
            commandsOut[i] = new PlayerCommand(basePosition, kinematics.MaxSpeed * Fixed.Half);
        }
    }

    /// <summary>
    /// Return <c>true</c> iff our team's nearest player is strictly closer
    /// to the ball than the opponents'. Ties resolve to opponent (defensive
    /// default); the match-loop layer may eventually replace this with a
    /// more nuanced contest-resolution policy.
    /// </summary>
    private static bool ResolveOwnTeamInPossession(
        BallState ball,
        ReadOnlySpan<PlayerState> ownTeam,
        ReadOnlySpan<PlayerState> opponents)
    {
        Fixed ownNearestSq = NearestDistanceSquared(ball.Position, ownTeam);
        Fixed oppNearestSq = NearestDistanceSquared(ball.Position, opponents);
        return ownNearestSq < oppNearestSq;
    }

    /// <summary>Smallest squared-distance from <paramref name="ballPosition"/> to any player in the team.</summary>
    private static Fixed NearestDistanceSquared(Vector3Fixed ballPosition, ReadOnlySpan<PlayerState> team)
    {
        Fixed nearestSq = Fixed.MaxValue;
        foreach (PlayerState player in team)
        {
            Fixed distSq = Vector3Fixed.DistanceSquared(ballPosition, player.Position);
            if (distSq < nearestSq)
            {
                nearestSq = distSq;
            }
        }
        return nearestSq;
    }

    /// <summary>
    /// Index of the player in <paramref name="team"/> nearest to
    /// <paramref name="ballPosition"/>. Ties resolve to the lower index
    /// (deterministic; iteration is in-order).
    /// </summary>
    private static int NearestPlayerIndex(Vector3Fixed ballPosition, ReadOnlySpan<PlayerState> team)
    {
        int nearestIndex = 0;
        Fixed nearestSq = Vector3Fixed.DistanceSquared(ballPosition, team[0].Position);
        for (int i = 1; i < team.Length; i++)
        {
            Fixed distSq = Vector3Fixed.DistanceSquared(ballPosition, team[i].Position);
            if (distSq < nearestSq)
            {
                nearestSq = distSq;
                nearestIndex = i;
            }
        }
        return nearestIndex;
    }

    /// <summary>
    /// Find the formation slot for a given roster index (1-11). Linear scan
    /// — formation has 11 entries; cost is bounded.
    /// </summary>
    private static FormationSlot FormationSlotForRosterIndex(BehaviorTreeArchetype archetype, byte rosterSlot)
    {
        foreach (FormationSlot slot in archetype.Formation)
        {
            if (slot.RosterSlot == rosterSlot)
            {
                return slot;
            }
        }
        throw new InvalidOperationException($"Archetype '{archetype.Name}' is missing roster slot {rosterSlot}; this should have been caught at parse time.");
    }

    /// <summary>
    /// Opponent goal position. Pitch is 105m × 68m centred on origin; goal
    /// lines at X = ±52.5. Home defends -X (so opponent goal is at +X);
    /// Away defends +X (opponent goal is at -X).
    /// </summary>
    private static Vector3Fixed OpponentGoalPosition(TeamSide side)
    {
        // 52.5m as Fixed: 105/2.
        Fixed goalLine = Fixed.FromInt(105) / Fixed.FromInt(2);
        Fixed x = side == TeamSide.Home ? goalLine : -goalLine;
        return new Vector3Fixed(x, Fixed.Zero, Fixed.Zero);
    }
}

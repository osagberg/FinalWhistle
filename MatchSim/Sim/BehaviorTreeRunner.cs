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
    // Phase-3 pass-the-ball coefficients. Hard-coded (NOT inspector-tunable)
    // because they're canonical-state-bearing — a Phase-4+ refactor that
    // makes these per-archetype YAML values must re-pin the corpus hash.
    // Documented in the per-field comments so SPEC closure prose can
    // reference the source.

    /// <summary>Ball-velocity-magnitude (m/s) below which the carrier may
    /// emit a fresh kick. Suppresses "kick the already-rolling ball"
    /// double-fires; lets the ball settle for ~30 ticks after each kick
    /// before the next kick is allowed.</summary>
    private static readonly Fixed CarrierKickGateMetresPerSecond = Fixed.FromInt(2);

    /// <summary>Squared form of the above for sqrt-free comparison against
    /// <see cref="Vector3Fixed.LengthSquared"/>.</summary>
    private static readonly Fixed CarrierKickGateMetresPerSecondSquared =
        CarrierKickGateMetresPerSecond * CarrierKickGateMetresPerSecond;

    /// <summary>Carrier-to-ball squared-distance below which the carrier is
    /// considered "on the ball" + eligible to kick. Matches the
    /// PlayerKinematics.Radius (default 0.5m) squared — same surface
    /// PlayerActuator.HasPossession uses.</summary>
    private static Fixed CarrierPossessionRadiusSquared(PlayerKinematics kinematics)
        => kinematics.Radius * kinematics.Radius;

    /// <summary>Minimum pass-target distance (metres). Below this the
    /// teammate is too close — kicking would skip past them; the carrier
    /// keeps the ball + looks further forward.</summary>
    private static readonly Fixed MinPassDistanceMetres = Fixed.FromInt(8);

    /// <summary>Maximum pass-target distance (metres). Above this the ball
    /// won't reach with Phase-3 kick speeds before drag kills it.</summary>
    private static readonly Fixed MaxPassDistanceMetres = Fixed.FromInt(35);

    /// <summary>Squared bounds for sqrt-free range checks.</summary>
    private static readonly Fixed MinPassDistanceSquared =
        MinPassDistanceMetres * MinPassDistanceMetres;
    private static readonly Fixed MaxPassDistanceSquared =
        MaxPassDistanceMetres * MaxPassDistanceMetres;

    /// <summary>Pass kick speed (m/s). Calibrated so a 20m pass arrives in
    /// ~1.5s with Phase-3 drag coefficients — fast enough to look like
    /// football, slow enough to be intercept-able.</summary>
    private static readonly Fixed PassSpeedMetresPerSecond = Fixed.FromInt(14);

    /// <summary>Long-ball kick speed (m/s). Used when no eligible pass
    /// target exists — carrier hoists toward opponent goal.</summary>
    private static readonly Fixed LongBallSpeedMetresPerSecond = Fixed.FromInt(18);

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

        Vector3Fixed ballPitchPosition = ProjectToPitchPlane(ball.Position);

        // Resolve possession: which team's nearest player is closer to the ball.
        Fixed pressRadiusSq = archetype.PressRadiusMetres * archetype.PressRadiusMetres;
        bool ownTeamInPossession = ResolveOwnTeamInPossession(ballPitchPosition, ownTeam, opponents);

        Vector3Fixed opponentGoal = OpponentGoalPosition(side);

        // Identify our own-team player nearest the ball — used for the
        // build-up "head to opponent goal" command.
        int nearestToBallIndex = NearestPlayerIndex(ballPitchPosition, ownTeam);

        for (int i = 0; i < 11; i++)
        {
            PlayerState player = ownTeam[i];
            FormationSlot slot = FormationSlotForRosterIndex(archetype, (byte)(i + 1));
            Vector3Fixed basePosition = side == TeamSide.Home ? slot.HomeBasePosition : slot.AwayBasePosition();

            // Press check: any opponent within PressRadius of this player AND
            // opponents have possession → press the ball.
            Fixed distFromPlayerToBallSq = Vector3Fixed.DistanceSquared(player.Position, ballPitchPosition);
            bool inPressRange = distFromPlayerToBallSq <= pressRadiusSq;

            if (!ownTeamInPossession && inPressRange)
            {
                // Press: head to ball at full speed.
                commandsOut[i] = new PlayerCommand(ballPitchPosition, kinematics.MaxSpeed);
                continue;
            }

            if (ownTeamInPossession && i == nearestToBallIndex)
            {
                // Build-up: ball-carrier heads toward opponent goal at the
                // archetype's build-up speed.
                PlayerCommand carrierCommand = new(opponentGoal, kinematics.MaxSpeed * archetype.BuildupSpeedFactor);

                // Phase-3 pass-the-ball: if the carrier is "on the ball"
                // (within possession radius) AND the ball is moving slowly
                // (below the kick gate), emit a KickIntent aimed at the
                // forward-most eligible teammate. Falls back to long-ball
                // toward the opponent goal if no teammate is in pass range.
                // Without this emission the BT only moves players toward
                // the ball but never APPLIES VELOCITY to the ball, so the
                // ball stays glued to its starting spot — the Slice-7
                // static-ball regression that blueprint patch 8e2dc1b
                // flagged.
                Fixed carrierToBallSq = distFromPlayerToBallSq;
                Fixed possessionRadiusSq = CarrierPossessionRadiusSquared(kinematics);
                bool carrierOnBall = carrierToBallSq <= possessionRadiusSq;
                bool ballSettled = ball.Velocity.LengthSquared() <= CarrierKickGateMetresPerSecondSquared;

                if (carrierOnBall && ballSettled)
                {
                    KickIntent kick = ChoosePassKick(
                        ball, ownTeam, i, side, opponentGoal);
                    carrierCommand = carrierCommand.WithKick(kick);
                }

                commandsOut[i] = carrierCommand;
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
        Vector3Fixed ballPosition,
        ReadOnlySpan<PlayerState> ownTeam,
        ReadOnlySpan<PlayerState> opponents)
    {
        Fixed ownNearestSq = NearestDistanceSquared(ballPosition, ownTeam);
        Fixed oppNearestSq = NearestDistanceSquared(ballPosition, opponents);
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

    private static Vector3Fixed ProjectToPitchPlane(Vector3Fixed position)
        => new(position.X, Fixed.Zero, position.Z);

    /// <summary>
    /// Phase-3 pass-the-ball aim heuristic. Picks the FORWARD-MOST teammate
    /// within pass-distance range whose X is strictly ahead of the carrier
    /// in the attacking direction; falls back to a long-ball at the
    /// opponent goal if no eligible teammate is in range. Pitch-plane kick
    /// (Y=0) per the Magnus-stub policy.
    ///
    /// <para>
    /// Deterministic + sqrt-free until the final normalize: distance bounds
    /// are compared via <see cref="Vector3Fixed.LengthSquared"/>;
    /// forward-most ranking is on raw X · forwardSign. Ties on X (same
    /// forwardness) resolve to the lower roster index per the iteration
    /// order — same determinism discipline as
    /// <see cref="NearestPlayerIndex"/>.
    /// </para>
    ///
    /// <para>
    /// <strong>Phase-4+ follow-up</strong>: this aim doesn't model opponent
    /// pressure (a forward teammate marked tight by 2 opponents should
    /// score lower than a free teammate slightly further back), nor pass
    /// accuracy (perfect aim now; gene-driven error variance later), nor
    /// risk-vs-reward (long-ball-to-goal is currently the fallback, but a
    /// "risk-it-when-trailing" tuning is reasonable Phase-4+). Logged here
    /// so the heuristic doesn't ossify silently.
    /// </para>
    /// </summary>
    private static KickIntent ChoosePassKick(
        BallState ball,
        ReadOnlySpan<PlayerState> ownTeam,
        int carrierIndex,
        TeamSide side,
        Vector3Fixed opponentGoal)
    {
        Vector3Fixed carrierPos = ProjectToPitchPlane(ownTeam[carrierIndex].Position);

        // Home attacks +X; Away attacks -X. forwardSign multiplies into the
        // "is this teammate ahead of carrier" comparison so the test is
        // sign-invariant across sides.
        Fixed forwardSign = side == TeamSide.Home ? Fixed.One : -Fixed.One;

        int bestTeammateIndex = -1;
        // Highest "forwardness score" seen so far: tpos.X * forwardSign.
        // Initialize to Fixed.MinValue so any candidate beats it.
        Fixed bestForwardness = Fixed.MinValue;

        for (int t = 0; t < ownTeam.Length; t++)
        {
            if (t == carrierIndex) continue;
            Vector3Fixed teammatePos = ProjectToPitchPlane(ownTeam[t].Position);
            Fixed distSq = Vector3Fixed.DistanceSquared(carrierPos, teammatePos);
            if (distSq < MinPassDistanceSquared) continue;
            if (distSq > MaxPassDistanceSquared) continue;

            // Must be strictly ahead of carrier in attacking direction.
            Fixed carrierForwardX = carrierPos.X * forwardSign;
            Fixed teammateForwardX = teammatePos.X * forwardSign;
            if (teammateForwardX <= carrierForwardX) continue;

            // Among eligible teammates, prefer the one furthest forward.
            if (teammateForwardX > bestForwardness)
            {
                bestForwardness = teammateForwardX;
                bestTeammateIndex = t;
            }
        }

        Vector3Fixed kickTarget;
        Fixed kickSpeed;
        if (bestTeammateIndex >= 0)
        {
            kickTarget = ProjectToPitchPlane(ownTeam[bestTeammateIndex].Position);
            kickSpeed = PassSpeedMetresPerSecond;
        }
        else
        {
            // No eligible pass target — long-ball at the opponent goal.
            // opponentGoal is already pitch-plane (Y=0) from caller.
            kickTarget = opponentGoal;
            kickSpeed = LongBallSpeedMetresPerSecond;
        }

        Vector3Fixed delta = kickTarget - carrierPos;
        Fixed deltaLenSq = delta.LengthSquared();
        if (deltaLenSq == Fixed.Zero)
        {
            // Degenerate — kick target coincident with carrier. Hoist
            // forward at the long-ball speed so the ball doesn't sit
            // stuck on the carrier dot. Forward direction = (forwardSign, 0, 0).
            Vector3Fixed forwardDir = new(forwardSign, Fixed.Zero, Fixed.Zero);
            return KickIntent.Ground(forwardDir * LongBallSpeedMetresPerSecond);
        }

        Vector3Fixed direction = delta.Normalize();
        Vector3Fixed kickVelocity = direction * kickSpeed;
        return KickIntent.Ground(kickVelocity);
    }
}

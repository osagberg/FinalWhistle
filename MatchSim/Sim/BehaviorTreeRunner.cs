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

    // Phase-3 polish-pass round 3 #3 (2026-05-12) variable pass speed.
    // Before round 3 #3, every pass used a constant 14 m/s regardless of
    // distance. Short passes (8-15m) looked heavy; long passes (25-35m)
    // were underpowered (drag killed before arrival). Tier the speed by
    // squared-distance zones (sqrt-free) so passes look more deliberate.

    /// <summary>Short-pass speed (m/s). 8-15m passes — quick, easy control.</summary>
    private static readonly Fixed ShortPassSpeedMetresPerSecond = Fixed.FromInt(10);

    /// <summary>Medium-pass speed (m/s). 15-25m passes — the prior
    /// Phase-3 constant. Calibrated so a 20m pass arrives in ~1.5s with
    /// Phase-3 drag coefficients — fast enough to look like football,
    /// slow enough to be intercept-able.</summary>
    private static readonly Fixed MediumPassSpeedMetresPerSecond = Fixed.FromInt(14);

    /// <summary>Long-pass speed (m/s). 25-35m passes — lobby driven
    /// delivery so drag doesn't kill it before arrival.</summary>
    private static readonly Fixed LongPassSpeedMetresPerSecond = Fixed.FromInt(18);

    /// <summary>Short-zone upper bound, squared (m²). 15² = 225.</summary>
    private static readonly Fixed ShortPassZoneUpperBoundSquared =
        Fixed.FromInt(15) * Fixed.FromInt(15);

    /// <summary>Medium-zone upper bound, squared (m²). 25² = 625.</summary>
    private static readonly Fixed MediumPassZoneUpperBoundSquared =
        Fixed.FromInt(25) * Fixed.FromInt(25);

    /// <summary>Long-ball kick speed (m/s). Used when no eligible pass
    /// target exists — carrier hoists toward opponent goal. Same numeric
    /// value as long-pass speed by design (both are "drive it forward"
    /// shots), but kept as a separate symbol so a Phase-4 tweak to one
    /// doesn't silently move the other.</summary>
    private static readonly Fixed LongBallSpeedMetresPerSecond = Fixed.FromInt(18);

    // Phase-3 Option-2 (2026-05-11) goalkeeper-specialization constants.
    // The "goalkeeper" detection is purely structural: roster slot 1 is the
    // GK by both authored YAML archetypes (direct-pressing + low-block-
    // counter). A Phase-4+ role-system refactor that decouples GK from
    // RosterSlot==1 must re-route this branch.

    /// <summary>FIFA-spec penalty-area depth (16.5m). Used to gate the GK's
    /// "charge at the ball" behaviour — outside this depth the GK holds
    /// formation rather than chasing.</summary>
    private static readonly Fixed PenaltyAreaDepthMetres =
        Fixed.FromInt(16) + Fixed.FromInt(1) / Fixed.FromInt(2);

    /// <summary>The roster slot reserved for goalkeeper by both shipped
    /// archetype YAMLs. Hardcoded for Month-3 / Phase-3; Phase-4 role-
    /// system will replace this with a typed role enum.</summary>
    private const byte GoalkeeperRosterSlot = 1;

    // Phase-3 Option-3 (2026-05-11) off-ball formation-translation constants.
    // Closes user-caught polish-pass review symptom "Just straight line running
    // for the most part." Prior to Option-3, hold-shape branch commanded
    // STATIC formation base positions — outfield players never shifted with
    // the ball, so the team played as 11 disconnected pucks. Football reality:
    // formation SHAPE preserved, CENTROID shifts toward ball position.

    /// <summary>Axial (X) formation shift factor. Translates the formation's
    /// X centre by `factor × ball.X`. 0.5 means the team shifts halfway
    /// toward the ball's longitudinal position — preserves formation depth
    /// while making the team compact toward the action.</summary>
    private static readonly Fixed FormationBallShiftFactor =
        Fixed.FromInt(1) / Fixed.FromInt(2);

    /// <summary>Lateral (Z) formation shift factor. Smaller than axial
    /// because the pitch is narrower (68m) than long (105m); a full lateral
    /// shift would push wide players off the touchline. 0.3 preserves
    /// formation width while letting the team drift toward the ball's
    /// lateral position.</summary>
    private static readonly Fixed LateralBallShiftFactor =
        Fixed.FromInt(3) / Fixed.FromInt(10);

    /// <summary>Pitch axial half-extent (X). 105m / 2 = 52.5m FIFA spec.
    /// Translated formation positions are clamped to ±this so a
    /// ball-relative shift cannot push players off the pitch.</summary>
    private static readonly Fixed PitchAxialHalfExtentMetres =
        Fixed.FromInt(105) / Fixed.FromInt(2);

    /// <summary>Pitch lateral half-extent (Z). 68m / 2 = 34m FIFA spec.
    /// </summary>
    private static readonly Fixed PitchLateralHalfExtentMetres =
        Fixed.FromInt(34);

    // Phase-3 polish-pass round 3 #1 (2026-05-11) coordinated press cap.
    // Before round 3 #1, EVERY outfield player within PressRadiusMetres of
    // the ball would sprint at it — at the 25m default radius this is often
    // 5-7 players swarming. Reads as a "puck swarm" not football. Real
    // football: 2-3 players close down, others mark space.

    /// <summary>Cap on simultaneous pressers per own team. Outfield (non-GK)
    /// players within `PressRadiusMetres` of the ball compete for the press
    /// slots; only the K nearest engage; the rest fall through to hold-shape
    /// (Option-3 ball-translated formation). Hardcoded for Phase-3; Phase-4
    /// may move to per-archetype YAML for tactical-style differentiation
    /// (low block: 1-2 pressers; high press: 4-5).</summary>
    private const int MaxPressersPerTeam = 3;

    // Phase-3 polish-pass round 3 #2 (2026-05-11) pass-target opponent-
    // pressure scoring. Before round 3 #2, ChoosePassKick picked the
    // FORWARD-MOST eligible teammate within [8, 35]m — carriers threaded
    // passes into traffic if the forward teammate was marked tight. After:
    // scoring penalises each opponent within `OpponentPressureRadiusMetres`
    // of the candidate teammate by `OpponentPressurePenalty` metres of
    // effective forwardness. Less-forward-but-free teammates outscore
    // forward-but-marked ones.

    /// <summary>Radius (metres) around a pass-candidate teammate within which
    /// opponents count as "marking" the candidate. 3m matches the radius
    /// SignatureRules uses for "tight contest" gates.</summary>
    private static readonly Fixed OpponentPressureRadiusMetres = Fixed.FromInt(3);

    /// <summary>Squared form for sqrt-free distance comparison.</summary>
    private static readonly Fixed OpponentPressureRadiusSquared =
        OpponentPressureRadiusMetres * OpponentPressureRadiusMetres;

    /// <summary>Per-marker penalty (metres of effective forwardness) deducted
    /// from a teammate's score for each opponent within
    /// `OpponentPressureRadiusMetres` of them. 4m means a teammate marked by
    /// 1 opponent loses ~4m of "advance value"; marked by 2 loses 8m. Tuning
    /// dial — Phase-4 archetype/personality params may differentiate
    /// risk-tolerant vs cautious tactical styles.</summary>
    private static readonly Fixed OpponentPressurePenalty = Fixed.FromInt(4);

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

        // Own goal line for GK gating (Phase-3 Option-2). Home defends -X,
        // Away defends +X. Pitch is 105m on X centred on origin; goal lines
        // at X = ±52.5.
        Fixed ownGoalLine = side == TeamSide.Home
            ? -(Fixed.FromInt(105) / Fixed.FromInt(2))
            : Fixed.FromInt(105) / Fixed.FromInt(2);

        // Phase-3 polish-pass round 3 #1 (2026-05-11): pre-compute the
        // K-nearest outfield pressers. Only these slots actually press; the
        // rest of the in-range outfield falls through to hold-shape.
        // Skipped for own-team-in-possession (no press fires in that case).
        // Stack-allocated 10-slot scratch (max 10 outfield candidates).
        Span<int> topPresserIndices = stackalloc int[MaxPressersPerTeam];
        int topPresserCount = 0;
        if (!ownTeamInPossession)
        {
            topPresserCount = ResolveTopKPressers(
                ballPitchPosition, ownTeam, archetype, pressRadiusSq, topPresserIndices);
        }

        for (int i = 0; i < 11; i++)
        {
            PlayerState player = ownTeam[i];
            FormationSlot slot = FormationSlotForRosterIndex(archetype, (byte)(i + 1));
            Vector3Fixed basePosition = side == TeamSide.Home ? slot.HomeBasePosition : slot.AwayBasePosition();

            // Distance from this player to the ball — used by the GK branch
            // (for ChoosePassKick possession-radius check) AND a few downstream
            // pieces of logic. The press eligibility check itself now lives in
            // the K-nearest pre-computation above (round 3 #1), so a per-player
            // `inPressRange` boolean is no longer enough — a player may be in
            // range but NOT in the top-K and therefore not press.
            Fixed distFromPlayerToBallSq = Vector3Fixed.DistanceSquared(player.Position, ballPitchPosition);
            bool inPressTopK = IndexIsInTopK(i, topPresserIndices, topPresserCount);

            // Phase-3 Option-2 (2026-05-11): goalkeeper branch fires BEFORE
            // press / build-up / hold-shape. Closes the user-caught polish-
            // pass review symptom "Goalkeepers running through all other
            // players with the ball" — the prior tree treated roster slot 1
            // identically to outfield, so the GK would sprint upfield with
            // possession or chase a ball at midfield.
            //
            // GK behaviours:
            //   1. In possession (own team + GK is nearest-to-ball):
            //      emit long-ball KickIntent toward forward-most eligible
            //      teammate (or opponent goal fallback). Move command =
            //      basePosition. GK never sprints upfield with the ball.
            //   2. Without possession, ball inside own penalty area
            //      (|ball.X - ownGoalLine| <= 16.5m): charge at the ball at
            //      MaxSpeed.
            //   3. Otherwise: hold formation (basePosition at half speed).
            if (slot.RosterSlot == GoalkeeperRosterSlot)
            {
                Fixed ballAxialDistanceFromGoal = Fixed.Abs(ballPitchPosition.X - ownGoalLine);
                bool ballInOwnPenaltyArea = ballAxialDistanceFromGoal <= PenaltyAreaDepthMetres;

                if (ownTeamInPossession && i == nearestToBallIndex)
                {
                    // GK with the ball: kick long, hold goal-line.
                    PlayerCommand gkCommand = new(basePosition, kinematics.MaxSpeed * Fixed.Half);

                    Fixed possessionRadiusSq = CarrierPossessionRadiusSquared(kinematics);
                    bool carrierOnBall = distFromPlayerToBallSq <= possessionRadiusSq;
                    bool ballSettled = ball.Velocity.LengthSquared() <= CarrierKickGateMetresPerSecondSquared;
                    if (carrierOnBall && ballSettled)
                    {
                        KickIntent kick = ChoosePassKick(ball, ownTeam, opponents, i, side, opponentGoal);
                        gkCommand = gkCommand.WithKick(kick);
                    }

                    commandsOut[i] = gkCommand;
                    continue;
                }

                if (!ownTeamInPossession && ballInOwnPenaltyArea)
                {
                    // Ball in our box, we don't have it: GK charges.
                    commandsOut[i] = new PlayerCommand(ballPitchPosition, kinematics.MaxSpeed);
                    continue;
                }

                // Default GK: hold formation slot at jog. Never chase a
                // ball that's outside our penalty area.
                commandsOut[i] = new PlayerCommand(basePosition, kinematics.MaxSpeed * Fixed.Half);
                continue;
            }

            if (!ownTeamInPossession && inPressTopK)
            {
                // Press: head to ball at full speed. Round 3 #1: gated by
                // K-nearest selection so the team caps at MaxPressersPerTeam
                // simultaneous pressers (no 5-7 player puck swarm).
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
                        ball, ownTeam, opponents, i, side, opponentGoal);
                    carrierCommand = carrierCommand.WithKick(kick);
                }

                commandsOut[i] = carrierCommand;
                continue;
            }

            // Hold shape: head toward BALL-TRANSLATED base position at jog
            // (half speed). Option-3 (2026-05-11): the formation centroid
            // shifts toward the ball's longitudinal + lateral position so
            // the team plays as a compact block instead of 11 disconnected
            // pucks. Translation factors live in FormationBallShiftFactor +
            // LateralBallShiftFactor; clamped to pitch boundaries.
            Vector3Fixed translatedBase = BallTranslatedBasePosition(basePosition, ballPitchPosition);
            commandsOut[i] = new PlayerCommand(translatedBase, kinematics.MaxSpeed * Fixed.Half);
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

    /// <summary>
    /// Phase-3 polish-pass round 3 #1 (2026-05-11) coordinated press cap.
    /// Returns the count of own-team OUTFIELD (non-GK) players eligible to
    /// press (within <paramref name="pressRadiusSq"/> of the ball), filling
    /// <paramref name="topKIndicesOut"/> with the indices of the
    /// <see cref="MaxPressersPerTeam"/> nearest of them.
    ///
    /// <para>Deterministic: ties on squared distance break by lower roster
    /// index. Hot-path allocation-free (stackalloc candidate buffer +
    /// in-place partial-sort).</para>
    ///
    /// <para>Returns 0 if no candidates exist (no outfield in range).
    /// Returned count is min(candidates, MaxPressersPerTeam) and indexes
    /// into <paramref name="ownTeam"/>. GK (slot 1, index 0) is skipped:
    /// GK has its own branch in <see cref="Tick"/> per Option-2 and never
    /// enters the press code path.</para>
    /// </summary>
    private static int ResolveTopKPressers(
        Vector3Fixed ballPitchPosition,
        ReadOnlySpan<PlayerState> ownTeam,
        BehaviorTreeArchetype archetype,
        Fixed pressRadiusSq,
        Span<int> topKIndicesOut)
    {
        // Candidate buffer: (playerIndex, distSq) pairs for outfield players
        // within press radius. Stackalloc bounds: 10 outfield slots max.
        Span<int> candidateIndices = stackalloc int[10];
        Span<long> candidateDistSqRaw = stackalloc long[10];
        int candidateCount = 0;

        for (int i = 0; i < ownTeam.Length; i++)
        {
            FormationSlot slot = FormationSlotForRosterIndex(archetype, (byte)(i + 1));
            if (slot.RosterSlot == GoalkeeperRosterSlot) continue;

            Fixed distSq = Vector3Fixed.DistanceSquared(ownTeam[i].Position, ballPitchPosition);
            if (distSq > pressRadiusSq) continue;

            candidateIndices[candidateCount] = i;
            candidateDistSqRaw[candidateCount] = distSq.RawValue;
            candidateCount++;
        }

        // Select top-K nearest. With K <= 3 and candidates <= 10, a
        // selection-sort pass is bounded at 30 comparisons — cheaper than
        // a heap or full sort + the result is naturally deterministic.
        int k = candidateCount < MaxPressersPerTeam ? candidateCount : MaxPressersPerTeam;
        for (int slot = 0; slot < k; slot++)
        {
            int bestPos = slot;
            long bestDistSq = candidateDistSqRaw[slot];
            int bestIndex = candidateIndices[slot];
            for (int scan = slot + 1; scan < candidateCount; scan++)
            {
                long scanDistSq = candidateDistSqRaw[scan];
                int scanIndex = candidateIndices[scan];
                // Strictly closer wins. Equal-distance ties: lower roster
                // index wins (deterministic tie-break consistent with
                // NearestPlayerIndex).
                if (scanDistSq < bestDistSq ||
                    (scanDistSq == bestDistSq && scanIndex < bestIndex))
                {
                    bestPos = scan;
                    bestDistSq = scanDistSq;
                    bestIndex = scanIndex;
                }
            }
            // Swap best into position `slot`.
            if (bestPos != slot)
            {
                (candidateIndices[slot], candidateIndices[bestPos]) =
                    (candidateIndices[bestPos], candidateIndices[slot]);
                (candidateDistSqRaw[slot], candidateDistSqRaw[bestPos]) =
                    (candidateDistSqRaw[bestPos], candidateDistSqRaw[slot]);
            }
            topKIndicesOut[slot] = candidateIndices[slot];
        }
        return k;
    }

    /// <summary>Linear scan of the top-K presser indices for membership.
    /// Bounded at K (= 3) comparisons; cheaper than any container for this
    /// size.</summary>
    private static bool IndexIsInTopK(int index, ReadOnlySpan<int> topK, int count)
    {
        for (int i = 0; i < count; i++)
        {
            if (topK[i] == index) return true;
        }
        return false;
    }

    /// <summary>
    /// Phase-3 polish-pass round 3 #3 (2026-05-12) variable pass speed.
    /// Three discrete tiers based on squared distance:
    /// <list type="bullet">
    ///   <item><description>distSq ≤ 225 (≤ 15m): short pass at 10 m/s.</description></item>
    ///   <item><description>225 &lt; distSq ≤ 625 (15-25m): medium at 14 m/s.</description></item>
    ///   <item><description>distSq &gt; 625 (&gt; 25m): long pass at 18 m/s.</description></item>
    /// </list>
    /// Sqrt-free (compares squared distance against squared thresholds).
    /// Caller filters by MinPassDistanceSquared (8²=64) + MaxPassDistanceSquared
    /// (35²=1225) before invoking, so distSq is always in [64, 1225].
    /// </summary>
    private static Fixed PassSpeedForDistanceSquared(Fixed distSq)
    {
        if (distSq.RawValue <= ShortPassZoneUpperBoundSquared.RawValue)
        {
            return ShortPassSpeedMetresPerSecond;
        }
        if (distSq.RawValue <= MediumPassZoneUpperBoundSquared.RawValue)
        {
            return MediumPassSpeedMetresPerSecond;
        }
        return LongPassSpeedMetresPerSecond;
    }

    /// <summary>
    /// Phase-3 polish-pass round 3 #2 (2026-05-11) opponent-pressure
    /// counter. Returns the number of opponents whose position is within
    /// <paramref name="radiusSq"/> of <paramref name="referencePos"/>.
    /// Used by <see cref="ChoosePassKick"/> to penalise pass candidates
    /// that are marked tight by multiple opponents. Bounded at 11
    /// comparisons; allocation-free.
    /// </summary>
    private static int CountOpponentsWithinRadius(
        Vector3Fixed referencePos, ReadOnlySpan<PlayerState> opponents, Fixed radiusSq)
    {
        int count = 0;
        for (int i = 0; i < opponents.Length; i++)
        {
            Fixed distSq = Vector3Fixed.DistanceSquared(referencePos, opponents[i].Position);
            if (distSq <= radiusSq) count++;
        }
        return count;
    }

    /// <summary>
    /// Phase-3 Option-3 (2026-05-11) off-ball formation translation. Returns
    /// <paramref name="basePosition"/> shifted by
    /// <c>(ball.X × FormationBallShiftFactor, 0, ball.Z × LateralBallShiftFactor)</c>,
    /// clamped to pitch boundaries (±52.5 X, ±34 Z) so a ball-relative shift
    /// cannot push players off the touchline / goal-line.
    ///
    /// <para>Effect: when the ball is at midfield centre, translation is
    /// zero — basePosition unchanged. When the ball moves to +40 X
    /// (attacking third), the formation centroid shifts +20 X — back-four
    /// at -30 X effectively becomes -10 X (pushing up); strikers at +20 X
    /// become +40 X (advancing into attack). Symmetric for AWAY since
    /// <see cref="FormationSlot.AwayBasePosition"/> already mirrors X.</para>
    /// </summary>
    private static Vector3Fixed BallTranslatedBasePosition(
        Vector3Fixed basePosition, Vector3Fixed ballPitchPosition)
    {
        Fixed shiftedX = basePosition.X + ballPitchPosition.X * FormationBallShiftFactor;
        Fixed shiftedZ = basePosition.Z + ballPitchPosition.Z * LateralBallShiftFactor;

        Fixed clampedX = Clamp(shiftedX, -PitchAxialHalfExtentMetres, PitchAxialHalfExtentMetres);
        Fixed clampedZ = Clamp(shiftedZ, -PitchLateralHalfExtentMetres, PitchLateralHalfExtentMetres);

        return new Vector3Fixed(clampedX, Fixed.Zero, clampedZ);
    }

    private static Fixed Clamp(Fixed value, Fixed min, Fixed max)
    {
        if (value.RawValue < min.RawValue) return min;
        if (value.RawValue > max.RawValue) return max;
        return value;
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
        ReadOnlySpan<PlayerState> opponents,
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
        // Highest "pass score" seen so far. Score = teammateForwardX -
        // (markerCount × OpponentPressurePenalty). Phase-3 round 3 #2:
        // free-but-behind teammates can outscore marked-but-forward ones.
        // A candidate must score STRICTLY POSITIVE to be picked — if the
        // sole eligible teammate is heavily marked enough to score ≤ 0,
        // the carrier falls back to long-ball at opponent goal. This is
        // the "risk threshold" pin per the task-spec: don't thread a pass
        // into hopeless traffic just because the teammate is marginally
        // forward. Phase-4 may parameterise the threshold per archetype
        // (risk-tolerant attackers may accept zero/negative scores).
        Fixed bestScore = Fixed.Zero;

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

            // Round 3 #2 (2026-05-11): subtract opponent-pressure penalty.
            // Each opponent within OpponentPressureRadiusMetres of the
            // candidate teammate counts as a marker; each marker deducts
            // OpponentPressurePenalty (= 4m by default) from the score.
            int markers = CountOpponentsWithinRadius(
                teammatePos, opponents, OpponentPressureRadiusSquared);
            Fixed score = teammateForwardX -
                (Fixed.FromInt(markers) * OpponentPressurePenalty);

            // Candidate must score STRICTLY POSITIVE to be considered. If
            // bestScore is the Fixed.Zero floor (no candidate found yet),
            // we require score > 0. If a positive-scoring candidate is
            // already in bestScore, we require strictly higher (or equal
            // with lower roster index for the deterministic tie-break).
            bool firstCandidate = bestTeammateIndex < 0;
            bool strictlyBetter = score.RawValue > bestScore.RawValue;
            bool tiedWithLowerIndex =
                score.RawValue == bestScore.RawValue &&
                !firstCandidate && t < bestTeammateIndex;

            if ((firstCandidate && score.RawValue > Fixed.Zero.RawValue) ||
                strictlyBetter || tiedWithLowerIndex)
            {
                bestScore = score;
                bestTeammateIndex = t;
            }
        }

        Vector3Fixed kickTarget;
        Fixed kickSpeed;
        if (bestTeammateIndex >= 0)
        {
            kickTarget = ProjectToPitchPlane(ownTeam[bestTeammateIndex].Position);
            // Round-3 #3 (2026-05-12): pass speed varies with target
            // distance instead of a constant 14 m/s. Short passes ease
            // off; long passes get extra zip so drag doesn't kill them.
            Fixed passDistSq = Vector3Fixed.DistanceSquared(carrierPos, kickTarget);
            kickSpeed = PassSpeedForDistanceSquared(passDistSq);
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

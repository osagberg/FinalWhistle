using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Pure-deterministic player kinematic actuator per
/// <c>design/match-engine.md §Q3</c>. The behavior tree (BT) outputs
/// <c>desired_position</c> + <c>desired_speed</c> per tick; the actuator
/// applies acceleration / max-speed caps toward that target. One
/// authority over a player's movement per tick — dual-authoritative
/// movement (BT force-integration alongside actuator caps) is forbidden
/// per the design-doc exit clause.
///
/// <para>
/// <strong>Coordinate convention</strong> matches <see cref="Vector3Fixed"/>
/// + <see cref="BallPhysics"/>: X+Z = pitch plane, Y = altitude. Players
/// stay at <c>Y = 0</c> in the Month-3 slice (no jumping headers yet);
/// the actuator does not enforce this invariant — caller supplies
/// <c>desired_position</c> in the pitch plane.
/// </para>
///
/// <para>
/// <strong>Algorithm</strong> (semi-implicit Euler, matches <see cref="BallPhysics"/>):
/// </para>
/// <list type="number">
///   <item><description>Compute <c>toTarget = desiredPosition - currentPosition</c>.</description></item>
///   <item><description>If <c>toTarget</c> is exactly zero OR <c>desiredSpeed</c> is non-positive: target velocity is <see cref="Vector3Fixed.Zero"/> (player wants to stop / is at target).</description></item>
///   <item><description>Else: target velocity = unit-direction(toTarget) × <c>min(desiredSpeed, MaxSpeed)</c>.</description></item>
///   <item><description>Velocity-delta = target velocity - current velocity. Clamp magnitude of velocity-delta to <c>MaxAcceleration · dt</c> (this naturally caps turn rate without an explicit angular cap — at high speed, direction changes need many ticks).</description></item>
///   <item><description>New velocity = current velocity + velocity-delta. Clamp magnitude to <c>MaxSpeed</c> (defensive; the per-step cap above usually covers this, but rounding can drift).</description></item>
///   <item><description>New position = current position + new velocity × dt.</description></item>
/// </list>
/// </summary>
public static class PlayerActuator
{
    /// <summary>Same fixed timestep as <see cref="BallPhysics"/> — 1/60 s.</summary>
    private static readonly Fixed Dt = Fixed.FromInt(1) / Fixed.FromInt(Tick.TicksPerSecond);

    /// <summary>
    /// Step the player one 60Hz tick forward. Pure function: same input ⇒
    /// same output across runs and platforms.
    /// </summary>
    public static PlayerState Step(
        PlayerState state,
        Vector3Fixed desiredPosition,
        Fixed desiredSpeed,
        PlayerKinematics kinematics)
    {
        Vector3Fixed currentPosition = ProjectToPitchPlane(state.Position);
        Vector3Fixed currentVelocity = ProjectToPitchPlane(state.Velocity);
        Vector3Fixed targetPosition = ProjectToPitchPlane(desiredPosition);

        // 1. Direction toward target (sqrt-free LengthSquared first to
        //    handle the at-target case without any Sqrt cost).
        Vector3Fixed toTarget = targetPosition - currentPosition;
        Fixed distSq = toTarget.LengthSquared();

        Vector3Fixed targetVelocity;
        if (distSq == Fixed.Zero || desiredSpeed <= Fixed.Zero)
        {
            // At target, or BT asked to halt. Target velocity is zero.
            targetVelocity = Vector3Fixed.Zero;
        }
        else
        {
            // Cap requested speed by the player's max-speed kinematic limit.
            Fixed cappedSpeed = Fixed.Min(desiredSpeed, kinematics.MaxSpeed);

            // Direction = toTarget / |toTarget|. Sqrt cost is paid here.
            Vector3Fixed direction = toTarget.Normalize();
            targetVelocity = direction * cappedSpeed;
        }

        // 2. Velocity-delta toward target, clamped by MaxAcceleration · dt.
        Vector3Fixed delta = targetVelocity - currentVelocity;
        Fixed maxDeltaMag = kinematics.MaxAcceleration * Dt;
        Vector3Fixed clampedDelta = ClampMagnitude(delta, maxDeltaMag);

        Vector3Fixed newVelocity = currentVelocity + clampedDelta;

        // 3. Defensive max-speed clamp. The per-step delta cap above should
        //    keep us under, but accumulated rounding could nudge over by a
        //    few ULP. Keeps the kinematic invariant tight.
        newVelocity = ClampMagnitude(newVelocity, kinematics.MaxSpeed);

        // 4. Position update (semi-implicit Euler; matches BallPhysics step
        //    so player + ball trajectories advance with the same convention).
        Vector3Fixed newPosition = currentPosition + newVelocity * Dt;

        return new PlayerState(newPosition, newVelocity, state.JerseyNumber, state.Side);
    }

    /// <summary>
    /// Returns <c>true</c> iff the ball is within the player's possession
    /// radius. Sqrt-free: compares squared distance against squared radius.
    /// Both team-mates and opponents can independently report possession of
    /// the same ball — the match loop is responsible for resolving contests.
    /// </summary>
    public static bool HasPossession(PlayerState state, BallState ball, PlayerKinematics kinematics)
    {
        Fixed distSq = Vector3Fixed.DistanceSquared(state.Position, ball.Position);
        Fixed radiusSq = kinematics.Radius * kinematics.Radius;
        return distSq <= radiusSq;
    }

    /// <summary>
    /// Clamp a vector's magnitude to at most <paramref name="maxMagnitude"/>.
    /// If the input magnitude is already within the cap, returns the input
    /// unchanged (no Sqrt). Else, returns <c>v · (maxMagnitude / |v|)</c>
    /// (one Sqrt + one Fixed division).
    /// </summary>
    private static Vector3Fixed ClampMagnitude(Vector3Fixed v, Fixed maxMagnitude)
    {
        Fixed lenSq = v.LengthSquared();
        Fixed maxSq = maxMagnitude * maxMagnitude;
        if (lenSq <= maxSq)
        {
            return v;
        }
        Fixed len = Fixed.Sqrt(lenSq);
        Fixed scale = maxMagnitude / len;
        return v * scale;
    }

    private static Vector3Fixed ProjectToPitchPlane(Vector3Fixed v)
        => new(v.X, Fixed.Zero, v.Z);
}

/// <summary>
/// Kinematic tuning for a single player. <c>MaxSpeed</c> + <c>MaxAcceleration</c>
/// drive the steering-target actuator; <c>Radius</c> drives possession
/// detection. Per-player values vary with the player's physical attributes
/// (Phase 4+ — gene-driven kinematics); the static
/// <see cref="Phase3Defaults"/> factory is the placeholder used by the
/// Month-3 slice's homogeneous 22-player roster.
/// </summary>
public readonly struct PlayerKinematics
{
    /// <summary>Maximum sustained speed (m/s). Phase-3 default = 7 m/s (reasonable for an outfield player at sustained sprint).</summary>
    public readonly Fixed MaxSpeed;

    /// <summary>Maximum acceleration (m/s²). Caps how fast the player can change velocity per tick. Phase-3 default = 6 m/s².</summary>
    public readonly Fixed MaxAcceleration;

    /// <summary>Possession radius (m). Phase-3 default = 0.5 m (a player has the ball if it's within half a metre of their position).</summary>
    public readonly Fixed Radius;

    /// <summary>Construct from explicit values. Use <see cref="Phase3Defaults"/> for Month-3 placeholder values.</summary>
    public PlayerKinematics(Fixed maxSpeed, Fixed maxAcceleration, Fixed radius)
    {
        if (maxSpeed < Fixed.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(maxSpeed), maxSpeed, "MaxSpeed must be non-negative.");
        }
        if (maxAcceleration < Fixed.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(maxAcceleration), maxAcceleration, "MaxAcceleration must be non-negative.");
        }
        if (radius < Fixed.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(radius), radius, "Radius must be non-negative.");
        }

        MaxSpeed = maxSpeed;
        MaxAcceleration = maxAcceleration;
        Radius = radius;
    }

    /// <summary>
    /// Phase-3 placeholder kinematics — homogeneous across all 22 players in
    /// the Month-3 slice. Real per-player tuning lands at Phase 4 with the
    /// gene-driven physical-attribute model.
    /// </summary>
    public static PlayerKinematics Phase3Defaults => Phase3DefaultValues;

    private static readonly PlayerKinematics Phase3DefaultValues = new(
        maxSpeed:        Fixed.FromInt(7),
        maxAcceleration: Fixed.FromInt(6),
        radius:          Fixed.FromInt(1) / Fixed.FromInt(2)  // 0.5 m
    );
}

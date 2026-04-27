using System.Collections.Generic;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

public sealed class PlayerActuatorTests
{
    private static Fixed F(int n) => Fixed.FromInt(n);
    private static Vector3Fixed V3(int x, int y, int z) => new(F(x), F(y), F(z));

    private static PlayerState AtRest(Vector3Fixed pos)
        => new(pos, Vector3Fixed.Zero, jerseyNumber: 9, side: TeamSide.Home);

    #region PlayerState basics

    [Fact]
    public void PlayerState_Construct_PreservesAllFields()
    {
        Vector3Fixed pos = V3(10, 0, 5);
        Vector3Fixed vel = V3(2, 0, -1);
        PlayerState p = new(pos, vel, jerseyNumber: 7, side: TeamSide.Away);

        Assert.Equal(pos, p.Position);
        Assert.Equal(vel, p.Velocity);
        Assert.Equal((byte)7, p.JerseyNumber);
        Assert.Equal(TeamSide.Away, p.Side);
    }

    [Fact]
    public void PlayerState_Equality_FieldwiseAndOrderInsensitive()
    {
        PlayerState a = new(V3(1, 0, 2), V3(3, 0, 4), 9, TeamSide.Home);
        PlayerState b = new(V3(1, 0, 2), V3(3, 0, 4), 9, TeamSide.Home);
        PlayerState c = new(V3(1, 0, 2), V3(3, 0, 4), 9, TeamSide.Away);

        Assert.Equal(a, b);
        Assert.True(a == b);
        Assert.NotEqual(a, c);
        Assert.True(a != c);
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
    }

    [Fact]
    public void PlayerState_WriteCanonical_Writes50Bytes()
    {
        PlayerState p = new(V3(1, 0, 2), V3(3, 0, 4), 9, TeamSide.Home);
        CanonicalEncoder enc = new();
        p.WriteCanonical(enc);

        // 6× Fixed (8 bytes) + 2× byte = 50 bytes.
        Assert.Equal(50, enc.WrittenCount);
    }

    [Fact]
    public void PlayerState_WriteCanonical_OrderIsPositionVelocityJerseySide()
    {
        // Lock the on-disk order: P.X / P.Y / P.Z / V.X / V.Y / V.Z / JerseyNumber / Side.
        PlayerState p = new(V3(1, 0, 2), V3(3, 0, 4), jerseyNumber: 9, side: TeamSide.Home);
        CanonicalEncoder helper = new();
        p.WriteCanonical(helper);

        CanonicalEncoder manual = new();
        manual.WriteVector3Fixed(V3(1, 0, 2));
        manual.WriteVector3Fixed(V3(3, 0, 4));
        manual.WriteByte(9);
        manual.WriteByte((byte)TeamSide.Home);

        Assert.Equal(manual.WrittenSpan.ToArray(), helper.WrittenSpan.ToArray());
    }

    [Fact]
    public void TeamSide_HomeAndAway_DistinctByteValues()
    {
        // Wire-format pinning: Home = 1, Away = 2.
        Assert.Equal((byte)1, (byte)TeamSide.Home);
        Assert.Equal((byte)2, (byte)TeamSide.Away);
        // Default(byte) = 0, intentionally not a valid TeamSide so an
        // uninitialized byte is detectable as "unset".
        Assert.NotEqual(default(TeamSide), TeamSide.Home);
        Assert.NotEqual(default(TeamSide), TeamSide.Away);
    }

    #endregion

    #region PlayerKinematics defaults

    [Fact]
    public void Phase3Defaults_AreNonZero()
    {
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;

        Assert.NotEqual(Fixed.Zero, k.MaxSpeed);
        Assert.NotEqual(Fixed.Zero, k.MaxAcceleration);
        Assert.NotEqual(Fixed.Zero, k.Radius);
    }

    [Fact]
    public void Phase3Defaults_MatchAuthoredValues()
    {
        // Pin Phase-3 placeholder values; if observers find players too slow
        // or too sticky to the ball, this is the test that gets updated
        // alongside the design tuning.
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;

        Assert.Equal(F(7), k.MaxSpeed);                                   // 7 m/s sustained sprint
        Assert.Equal(F(6), k.MaxAcceleration);                            // 6 m/s²
        Assert.Equal(Fixed.One / F(2), k.Radius);                         // 0.5 m possession radius
    }

    #endregion

    #region PlayerActuator.Step — determinism

    [Fact]
    public void Step_Deterministic_SameInputProducesSameOutput100Times()
    {
        PlayerState start = AtRest(V3(0, 0, 0));
        Vector3Fixed target = V3(10, 0, 5);
        Fixed speed = F(7);
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;

        HashSet<PlayerState> distinct = new();
        for (int i = 0; i < 100; i++)
        {
            distinct.Add(PlayerActuator.Step(start, target, speed, k));
        }
        Assert.Single(distinct);
    }

    #endregion

    #region PlayerActuator.Step — at-target stops

    [Fact]
    public void Step_AtTarget_VelocityDecaysTowardZero()
    {
        // Player has nonzero velocity but is exactly at target. Target velocity
        // is zero, so velocity-delta is -current-velocity, capped by max-accel.
        // Velocity should decrease in magnitude.
        Vector3Fixed pos = V3(5, 0, 5);
        PlayerState start = new(pos, V3(3, 0, 0), jerseyNumber: 9, side: TeamSide.Home);
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;

        PlayerState after = PlayerActuator.Step(start, pos, F(0), k);

        Assert.True(after.Velocity.LengthSquared() < start.Velocity.LengthSquared(),
            $"Velocity should decay; was {start.Velocity}, became {after.Velocity}");
    }

    [Fact]
    public void Step_DesiredSpeedZero_StopsTheTakeoff()
    {
        // BT asks for desired_speed = 0 even though target is non-trivially
        // distant. Player should decelerate (target velocity = 0, not "head
        // toward target slowly").
        PlayerState start = new(V3(0, 0, 0), V3(5, 0, 0), 9, TeamSide.Home);
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;

        PlayerState after = PlayerActuator.Step(start, V3(100, 0, 0), Fixed.Zero, k);

        // X velocity should be reduced (target velocity is zero; current is +5).
        Assert.True(after.Velocity.X < F(5));
    }

    #endregion

    #region PlayerActuator.Step — accelerates toward target

    [Fact]
    public void Step_FromRest_AcceleratesTowardTarget()
    {
        // Player at origin, target at +X, desired speed = max. The per-step
        // velocity gain is bounded by MaxAcceleration · dt; the production
        // path passes through Sqrt + division (ClampMagnitude) which has its
        // own ULP-rounding profile, so the resulting velocity won't equal
        // `MaxAcceleration * dt` exactly. Test structural properties:
        //   - velocity is along +X (target direction)
        //   - magnitude is close to MaxAcceleration · dt (within tolerance)
        //   - magnitude does NOT exceed MaxAcceleration · dt (the cap holds)
        PlayerState start = AtRest(V3(0, 0, 0));
        Vector3Fixed target = V3(10, 0, 0);
        Fixed speed = F(7);
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;

        PlayerState after = PlayerActuator.Step(start, target, speed, k);

        Fixed dt = Fixed.One / F(60);
        Fixed maxStepDelta = k.MaxAcceleration * dt;

        // Direction: along +X only.
        Assert.True(after.Velocity.X > Fixed.Zero, "Velocity should head toward target (+X).");
        Assert.Equal(Fixed.Zero, after.Velocity.Y);
        Assert.Equal(Fixed.Zero, after.Velocity.Z);

        // Magnitude bounded by MaxAcceleration · dt (with tiny ULP slack
        // from ClampMagnitude's sqrt + division path). Allow 2 ULP drift.
        Assert.True(after.Velocity.X <= maxStepDelta + Fixed.Epsilon * F(2),
            $"Velocity X = {after.Velocity.X} exceeds maxStepDelta {maxStepDelta}");

        // Magnitude is close to maxStepDelta (well within 1% — the
        // ClampMagnitude rounding can drift a few ULP, but not 1%).
        Fixed onePercent = maxStepDelta / F(100);
        Fixed gap = maxStepDelta - after.Velocity.X;
        Assert.True(gap < onePercent,
            $"Velocity X = {after.Velocity.X} is too far below maxStepDelta {maxStepDelta} (gap {gap})");

        // Position advanced by new velocity × dt (semi-implicit Euler).
        // Use production-equivalent expression for exact-byte comparison.
        Assert.Equal(after.Velocity.X * dt, after.Position.X);
    }

    [Fact]
    public void Step_PerStepVelocityGain_NeverExceedsMaxAccelerationTimesDt()
    {
        // Run many ticks chasing a far-away target; verify each step's
        // velocity-magnitude growth is bounded by the per-step accel cap.
        PlayerState s = AtRest(V3(0, 0, 0));
        Vector3Fixed target = V3(1000, 0, 0);
        Fixed speed = F(100);  // ridiculous, but capped to MaxSpeed inside
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;
        Fixed dt = Fixed.One / F(60);
        Fixed maxDelta = k.MaxAcceleration * dt;

        for (int i = 0; i < 30; i++)
        {
            PlayerState next = PlayerActuator.Step(s, target, speed, k);
            // Per-step velocity-change magnitude (squared, sqrt-free).
            Fixed deltaMagSq = (next.Velocity - s.Velocity).LengthSquared();
            // Allow a tiny ULP slack: maxDelta² + small Fixed.Epsilon.
            Fixed maxDeltaSq = maxDelta * maxDelta;
            // Fixed-point rounding can let deltaMagSq be a few ULP above
            // maxDeltaSq when ClampMagnitude divides + multiplies; allow
            // 1 ULP of drift.
            Fixed slack = Fixed.Epsilon;
            Assert.True(deltaMagSq <= maxDeltaSq + slack,
                $"Per-step velocity delta exceeds max-accel·dt: |Δv|²={deltaMagSq}, max={maxDeltaSq}");
            s = next;
        }
    }

    [Fact]
    public void Step_NeverExceedsMaxSpeed_EvenWithFarawayTarget()
    {
        // After many ticks of acceleration, velocity magnitude should not
        // exceed MaxSpeed (the post-step clamp catches accumulated rounding).
        PlayerState s = AtRest(V3(0, 0, 0));
        Vector3Fixed target = V3(1000, 0, 0);
        Fixed speed = F(100);  // requested, capped to MaxSpeed
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;

        for (int i = 0; i < 600; i++)  // 10 seconds of running
        {
            s = PlayerActuator.Step(s, target, speed, k);
            // Allow 2 ULP slack on speed-cap.
            Fixed maxSq = k.MaxSpeed * k.MaxSpeed;
            Assert.True(s.Velocity.LengthSquared() <= maxSq + Fixed.Epsilon * F(2),
                $"Tick {i}: velocity exceeds MaxSpeed: |v|²={s.Velocity.LengthSquared()}, max={maxSq}");
        }
    }

    [Fact]
    public void Step_DiagonalTarget_HeadingMatchesDirection()
    {
        // Player at origin, target diagonally at (3, 0, 4) — 3-4-5 triangle.
        // Heading should match unit (3, 0, 4)/5 = (0.6, 0, 0.8).
        PlayerState start = AtRest(V3(0, 0, 0));
        Vector3Fixed target = V3(3, 0, 4);
        Fixed speed = F(5);  // exactly distance — at MaxSpeed (7), this is fine
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;

        PlayerState after = PlayerActuator.Step(start, target, speed, k);

        // After one step from rest with max-accel cap, velocity is along the
        // direction (3, 0, 4)/5 — so velocity.Z / velocity.X should equal 4/3.
        // (Both X and Z components are non-zero and proportional.)
        Assert.True(after.Velocity.X > Fixed.Zero);
        Assert.True(after.Velocity.Z > Fixed.Zero);
        Assert.Equal(Fixed.Zero, after.Velocity.Y);
        // Ratio: v.Z / v.X ≈ 4/3 ≈ 1.333.
        Fixed ratio = after.Velocity.Z / after.Velocity.X;
        Fixed expected = F(4) / F(3);
        Fixed err = Fixed.Abs(ratio - expected);
        Fixed tolerance = Fixed.One / F(1000);  // 0.001
        Assert.True(err < tolerance, $"Heading ratio off; got {ratio}, expected ≈ {expected}");
    }

    #endregion

    #region PlayerActuator.HasPossession

    [Fact]
    public void HasPossession_BallExactlyAtPlayer_True()
    {
        PlayerState p = AtRest(V3(5, 0, 5));
        BallState ball = new(V3(5, 0, 5), Vector3Fixed.Zero, Vector3Fixed.Zero);

        Assert.True(PlayerActuator.HasPossession(p, ball, PlayerKinematics.Phase3Defaults));
    }

    [Fact]
    public void HasPossession_BallJustOutsideRadius_False()
    {
        // Phase-3 default radius is 0.5 m. Place ball 1 m away — outside.
        PlayerState p = AtRest(V3(0, 0, 0));
        BallState ball = new(V3(1, 0, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);

        Assert.False(PlayerActuator.HasPossession(p, ball, PlayerKinematics.Phase3Defaults));
    }

    [Fact]
    public void HasPossession_BallExactlyAtRadius_True()
    {
        // Distance == radius → counted as possession (boundary inclusive).
        PlayerState p = AtRest(V3(0, 0, 0));
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;
        BallState ball = new(new Vector3Fixed(k.Radius, Fixed.Zero, Fixed.Zero), Vector3Fixed.Zero, Vector3Fixed.Zero);

        Assert.True(PlayerActuator.HasPossession(p, ball, k));
    }

    [Fact]
    public void HasPossession_DifferentPlayers_BothCanReportPossession()
    {
        // ADR-0008-style: both team-mates and opponents independently report
        // possession. Match loop is responsible for resolving contests.
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;
        PlayerState a = new(V3(0, 0, 0), Vector3Fixed.Zero, 9, TeamSide.Home);
        PlayerState b = new(new Vector3Fixed(k.Radius, Fixed.Zero, Fixed.Zero), Vector3Fixed.Zero, 9, TeamSide.Away);
        // Place ball between them: ball within both radii.
        BallState ball = new(new Vector3Fixed(k.Radius / F(2), Fixed.Zero, Fixed.Zero), Vector3Fixed.Zero, Vector3Fixed.Zero);

        Assert.True(PlayerActuator.HasPossession(a, ball, k));
        Assert.True(PlayerActuator.HasPossession(b, ball, k));
    }

    #endregion

    #region Integration: player runs to ball, kicks it

    [Fact]
    public void Integration_PlayerRunsToBall_GainsPossession_KicksBallAway()
    {
        // Set up: ball stationary at (10, 0, 0); player at (0, 0, 0). Player
        // runs toward ball at MaxSpeed for as many ticks as it takes to reach
        // possession-radius. Then "kicks" the ball by setting a new BallState.
        BallState ball = new(V3(10, 0, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        PlayerState player = AtRest(V3(0, 0, 0));
        PlayerKinematics pk = PlayerKinematics.Phase3Defaults;
        BallPhysicsCoefficients bk = BallPhysicsCoefficients.Phase3Seeds;
        Fixed speed = pk.MaxSpeed;

        bool hadPossession = false;
        for (int tick = 0; tick < 600 && !hadPossession; tick++)
        {
            player = PlayerActuator.Step(player, ball.Position, speed, pk);
            ball = BallPhysics.Step(ball, bk);  // ball just sits (no v, no spin → no gravity below ground)
            hadPossession = PlayerActuator.HasPossession(player, ball, pk);
        }

        Assert.True(hadPossession, "Player should reach possession of stationary ball within 10 seconds.");

        // Kick: at the moment of possession, set ball velocity to (0, 0, 10)
        // (lateral kick along +Z). No need for a separate Kick API — kick is
        // just composing a new BallState.
        ball = new BallState(ball.Position, V3(0, 0, 10), Vector3Fixed.Zero);

        // Verify ball begins moving.
        Assert.Equal(F(10), ball.Velocity.Z);
        // Step ball one tick; ball should have moved in +Z.
        BallState after = BallPhysics.Step(ball, bk);
        Assert.True(after.Position.Z > Fixed.Zero, "Ball should have moved in +Z after kick.");
    }

    #endregion
}

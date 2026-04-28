using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

public sealed class BallPhysicsTests
{
    private static Fixed F(int n) => Fixed.FromInt(n);
    private static Fixed Half => Fixed.Half;

    /// <summary>"All forces off" coefficients — useful for testing one force in isolation.</summary>
    private static BallPhysicsCoefficients Off => new(
        gravity:         Fixed.Zero,
        linearDrag:      Fixed.Zero,
        magnusCoupling:  Fixed.Zero,
        bounceRetention: Fixed.Zero,
        rollingFriction: Fixed.Zero
    );

    private static Vector3Fixed V3(int x, int y, int z) => new(F(x), F(y), F(z));

    #region Determinism

    [Fact]
    public void Step_PureFunction_SameInputProducesSameOutput100Times()
    {
        BallState start = new(
            position: V3(1, 10, 2),
            velocity: V3(5, 8, -3),
            spin:     V3(0, 1, 0)
        );
        BallPhysicsCoefficients k = BallPhysicsCoefficients.Phase3Seeds;

        // Run 100 fresh independent steps; verify single resulting state.
        HashSet<BallState> distinct = new();
        for (int i = 0; i < 100; i++)
        {
            distinct.Add(BallPhysics.Step(start, k));
        }

        Assert.Single(distinct);
    }

    [Fact]
    public void Step_SequentialN_Ticks_Deterministic_AcrossRepeatedRuns()
    {
        BallState start = new(
            position: V3(0, 5, 0),
            velocity: V3(10, 5, 0),
            spin:     V3(0, 1, 0)
        );
        BallPhysicsCoefficients k = BallPhysicsCoefficients.Phase3Seeds;

        // Run a 60-tick simulation twice; final state must be identical.
        BallState a = start;
        BallState b = start;
        for (int i = 0; i < 60; i++)
        {
            a = BallPhysics.Step(a, k);
            b = BallPhysics.Step(b, k);
        }

        Assert.Equal(a, b);
    }

    #endregion

    #region Gravity-only fall

    [Fact]
    public void GravityOnly_BallAtRest_AcceleratesDownwardAtGTimesDt()
    {
        // With drag/magnus/bounce/rolling all zero and only gravity active,
        // semi-implicit Euler gives v.y_new = v.y - g*dt; p.y_new = p.y + v.y_new * dt.
        //
        // Expected values are computed via production-equivalent expressions
        // (g * Dt, NOT a single g/60 division). Fixed-point arithmetic is
        // sensitive to operation order — different formulations of the same
        // mathematical expression can differ by 1-5 ULP after rounding. The
        // test pins THIS arithmetic path; deterministic across platforms is
        // the contract.
        Fixed gravity = F(981) / F(100);  // 9.81 m/s²
        Fixed dt = F(1) / F(60);
        BallPhysicsCoefficients k = new(
            gravity:         gravity,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.Zero,
            rollingFriction: Fixed.Zero
        );
        BallState start = new(V3(0, 100, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        // v.y_new = 0 - gravity * dt (semi-implicit Euler, gravity-only).
        Fixed expectedVelY = Fixed.Zero - gravity * dt;
        Assert.Equal(expectedVelY, after.Velocity.Y);

        // p.y_new = p.y_old + v.y_new * dt.
        Fixed expectedPosY = F(100) + expectedVelY * dt;
        Assert.Equal(expectedPosY, after.Position.Y);

        // Lateral components untouched.
        Assert.Equal(Fixed.Zero, after.Position.X);
        Assert.Equal(Fixed.Zero, after.Position.Z);
        Assert.Equal(Fixed.Zero, after.Velocity.X);
        Assert.Equal(Fixed.Zero, after.Velocity.Z);

        // Sanity: gravity-only should reduce altitude (independent oracle).
        Assert.True(after.Position.Y < F(100), "Ball should have fallen.");
        Assert.True(after.Velocity.Y < Fixed.Zero, "Velocity should point downward.");
    }

    [Fact]
    public void GravityOnly_VelocityAccumulatesLinearlyOverNTicks()
    {
        // After N ticks with only gravity active: v.y = -N * (g * dt).
        // Semi-implicit Euler is exact for constant acceleration applied to
        // velocity, so v.y after N ticks is -N times the per-step gravity
        // delta — verified via production-equivalent fixed-point math.
        Fixed gravity = F(981) / F(100);
        Fixed dt = F(1) / F(60);
        BallPhysicsCoefficients k = new(
            gravity:         gravity,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.Zero,
            rollingFriction: Fixed.Zero
        );
        BallState s = new(V3(0, 1000, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);

        const int N = 30;
        for (int i = 0; i < N; i++)
        {
            s = BallPhysics.Step(s, k);
        }

        // After N ticks: v.y = N * (-gravity*dt). Use production-equivalent
        // arithmetic: subtract g*dt N times to match accumulated rounding.
        Fixed expected = Fixed.Zero;
        for (int i = 0; i < N; i++)
        {
            expected -= gravity * dt;
        }
        Assert.Equal(expected, s.Velocity.Y);

        // Independent sanity (no Fixed-arithmetic dependence): velocity is
        // negative + monotonically decreased over the run (we ran N steps,
        // velocity should be far below zero, well below -1 m/s after 0.5 s).
        Assert.True(s.Velocity.Y < F(-4), $"After 30 ticks with g=9.81, v.y should be ≪ -4 m/s; got {s.Velocity.Y}");
        Assert.True(s.Velocity.Y > F(-6), $"v.y should be > -6 m/s; got {s.Velocity.Y}");
    }

    #endregion

    #region Drag-only velocity decay

    [Fact]
    public void DragOnly_HorizontalVelocity_DecaysGeometricallyEachStep()
    {
        // With drag=0.02 per step and all other forces off:
        //   v_new = v * (1 - LinearDrag).
        // Production computes the retention factor as `Fixed.One - LinearDrag`
        // — match that path here (NOT a separate F(98)/F(100), which rounds
        // to a different Q32.32 raw).
        Fixed drag = F(2) / F(100);
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      drag,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.Zero,
            rollingFriction: Fixed.Zero
        );
        BallState start = new(V3(0, 100, 0), V3(10, 0, 0), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        // v_new.x = 10 * (1 - drag) — using production-equivalent expression.
        Assert.Equal(F(10) * (Fixed.One - drag), after.Velocity.X);
        Assert.Equal(Fixed.Zero, after.Velocity.Y);
        Assert.Equal(Fixed.Zero, after.Velocity.Z);

        // Independent sanity: drag reduces magnitude.
        Assert.True(after.Velocity.X < F(10), "Drag must reduce speed.");
        Assert.True(after.Velocity.X > F(9), "Drag should be small per step.");
    }

    [Fact]
    public void DragOnly_DecaysAllComponentsTogether()
    {
        Fixed drag = F(2) / F(100);
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      drag,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.Zero,
            rollingFriction: Fixed.Zero
        );
        BallState start = new(V3(0, 100, 0), V3(10, 5, 8), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        // All three components scaled identically by the production-equivalent
        // retention factor (Fixed.One - drag).
        Fixed retention = Fixed.One - drag;
        Assert.Equal(F(10) * retention, after.Velocity.X);
        Assert.Equal(F(5) * retention, after.Velocity.Y);
        Assert.Equal(F(8) * retention, after.Velocity.Z);
    }

    #endregion

    #region Magnus stub (zero spin → off; non-zero spin → curve)

    [Fact]
    public void Magnus_ZeroSpin_NoLateralChange()
    {
        // Ball moving along +X with zero spin and only Magnus enabled (gravity
        // / drag / bounce / rolling all zero). Velocity should pass through
        // unchanged — the Magnus skip-when-spin-zero branch is exercised.
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  F(4) / F(10000),  // 0.0004 per step
            bounceRetention: Fixed.Zero,
            rollingFriction: Fixed.Zero
        );
        BallState start = new(V3(0, 100, 0), V3(10, 0, 0), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        // Velocity unchanged: spin=0 → no Magnus; no other forces.
        Assert.Equal(V3(10, 0, 0), after.Velocity);
    }

    [Fact]
    public void Magnus_NonZeroSpin_CurvesPerpendicularToVelocity()
    {
        // Ball moving along +X with spin around +Y axis. Magnus = C_m·(spin × v).
        // (0,1,0) × (10,0,0) = (0, 0, -10).
        // Per step: v.z += MagnusCoupling * -10.
        Fixed magnus = F(4) / F(10000);
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  magnus,
            bounceRetention: Fixed.Zero,
            rollingFriction: Fixed.Zero
        );
        BallState start = new(V3(0, 100, 0), V3(10, 0, 0), V3(0, 1, 0));

        BallState after = BallPhysics.Step(start, k);

        // X velocity unchanged (Magnus result is along -Z; perpendicular to v).
        Assert.Equal(F(10), after.Velocity.X);
        // Y velocity unchanged (cross product has no Y component here).
        Assert.Equal(Fixed.Zero, after.Velocity.Y);
        // Z velocity gets the Magnus nudge: production-equivalent expression.
        Fixed expectedVelZ = magnus * F(-10);
        Assert.Equal(expectedVelZ, after.Velocity.Z);

        // Independent sanity: Magnus produces lateral motion (Z deflection).
        Assert.NotEqual(Fixed.Zero, after.Velocity.Z);
        // Direction: spin around +Y plus velocity along +X yields -Z deflection
        // (right-handed cross product). Catches sign-error regressions.
        Assert.True(after.Velocity.Z < Fixed.Zero, $"Expected -Z deflection; got Z = {after.Velocity.Z}");

        // Spin is invariant — preserved across the step (this stub does not
        // model spin decay; that's a Phase 4+ concern per match-engine.md).
        Assert.Equal(V3(0, 1, 0), after.Spin);
    }

    [Fact]
    public void Magnus_CoefficientZero_NoCurveEvenWithSpin()
    {
        // Stub policy from design doc §Q2: Magnus structure stays even if
        // observers find the curve noisy at Month-3 — operators may zero the
        // coefficient. Verify zero-coefficient produces zero Magnus effect.
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,  // explicitly zeroed
            bounceRetention: Fixed.Zero,
            rollingFriction: Fixed.Zero
        );
        BallState start = new(V3(0, 100, 0), V3(10, 0, 0), V3(0, 1, 0));

        BallState after = BallPhysics.Step(start, k);

        Assert.Equal(V3(10, 0, 0), after.Velocity);
    }

    #endregion

    #region Ground bounce

    [Fact]
    public void Bounce_BallFallingHitsGround_FlipsVerticalVelocityScaledByRetention()
    {
        // Ball low + falling fast: in one step it penetrates the ground; bounce
        // fires. With e=0.5 and v.y=-10, post-bounce v.y = -e * -10 = +5.
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: F(5) / F(10),  // 0.5
            rollingFriction: Fixed.Zero
        );
        // Position low enough that one step puts y < 0.
        // p.y_new = 0.05 + (-10) * (1/60) = 0.05 - 0.1667 ≈ -0.1167.
        Fixed startY = F(5) / F(100);  // 0.05
        BallState start = new(new Vector3Fixed(Fixed.Zero, startY, Fixed.Zero), V3(0, -10, 0), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        // Position clamped to ground.
        Assert.Equal(Fixed.Zero, after.Position.Y);
        // Vertical velocity flipped + scaled by retention.
        Assert.Equal(F(5), after.Velocity.Y);
    }

    [Fact]
    public void Bounce_BallNotFalling_NoBounce_PositionClampedOnly()
    {
        // Edge case: ball already at y=0 with v.y=0 (settled). No bounce should
        // fire; rolling friction may apply (but rolling is also zero here).
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: F(5) / F(10),
            rollingFriction: Fixed.Zero
        );
        BallState start = new(Vector3Fixed.Zero, V3(0, 0, 0), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        Assert.Equal(Fixed.Zero, after.Position.Y);
        Assert.Equal(Fixed.Zero, after.Velocity.Y);
    }

    [Fact]
    public void Bounce_GroundedAtRestWithGravity_DoesNotBounce()
    {
        BallState after = BallPhysics.Step(BallState.AtRest, BallPhysicsCoefficients.Phase3Seeds);

        Assert.Equal(Vector3Fixed.Zero, after.Position);
        Assert.Equal(Vector3Fixed.Zero, after.Velocity);
    }

    [Fact]
    public void Bounce_GroundedRollingWithGravity_DoesNotCreateVerticalBounce()
    {
        BallState start = new(Vector3Fixed.Zero, V3(8, 0, 0), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, BallPhysicsCoefficients.Phase3Seeds);

        Assert.Equal(Fixed.Zero, after.Position.Y);
        Assert.Equal(Fixed.Zero, after.Velocity.Y);
        Assert.True(after.Velocity.X > Fixed.Zero, $"Rolling ball should still move forward; X velocity = {after.Velocity.X}");
        Assert.True(after.Velocity.X < F(8), $"Drag + rolling friction should reduce X velocity; X velocity = {after.Velocity.X}");
    }

    [Fact]
    public void Bounce_PerfectlyElastic_PreservesVerticalSpeedExactly()
    {
        // e=1.0 → no energy loss. v.y=-10 → +10 after bounce.
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.One,
            rollingFriction: Fixed.Zero
        );
        BallState start = new(new Vector3Fixed(Fixed.Zero, F(5) / F(100), Fixed.Zero), V3(0, -10, 0), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        Assert.Equal(F(10), after.Velocity.Y);
    }

    [Fact]
    public void Bounce_PerfectlyInelastic_DropsVerticalVelocityToZero()
    {
        // e=0 → no rebound. v.y=-10 → 0 after bounce.
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.Zero,
            rollingFriction: Fixed.Zero
        );
        BallState start = new(new Vector3Fixed(Fixed.Zero, F(5) / F(100), Fixed.Zero), V3(0, -10, 0), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        Assert.Equal(Fixed.Zero, after.Velocity.Y);
    }

    #endregion

    #region Rolling friction

    [Fact]
    public void Rolling_BallOnGround_HorizontalVelocityDecaysByRetention()
    {
        // μ=0.25 per step; v.x=10 on ground → post-step v.x = 10 * 0.75 = 7.5.
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.Zero,
            rollingFriction: F(25) / F(100)  // 0.25
        );
        BallState start = new(Vector3Fixed.Zero, V3(10, 0, 0), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        // v.x_new = 10 * (1 - 0.25) = 7.5.
        Assert.Equal(F(75) / F(10), after.Velocity.X);
        // Y still 0; Z still 0.
        Assert.Equal(Fixed.Zero, after.Velocity.Y);
        Assert.Equal(Fixed.Zero, after.Velocity.Z);
    }

    [Fact]
    public void Rolling_DecaysBothHorizontalComponents()
    {
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.Zero,
            rollingFriction: F(25) / F(100)
        );
        BallState start = new(Vector3Fixed.Zero, V3(8, 0, 4), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        Fixed retention = F(75) / F(100);
        Assert.Equal(F(8) * retention, after.Velocity.X);
        Assert.Equal(F(4) * retention, after.Velocity.Z);
    }

    [Fact]
    public void Rolling_BallAirborne_NoFrictionApplied()
    {
        // Ball at altitude (Y=10) moving along +X. Rolling friction must NOT
        // fire; only ground-contact triggers it.
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.Zero,
            rollingFriction: F(25) / F(100)
        );
        BallState start = new(V3(0, 10, 0), V3(10, 0, 0), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        Assert.Equal(F(10), after.Velocity.X);
    }

    [Fact]
    public void Rolling_AfterBounce_OnlyAppliesIfBallSettlesNotIfReboundsUp()
    {
        // Ball drops at ground edge with high bounce coefficient (e=1.0);
        // post-bounce v.y is positive (upward), so rolling friction MUST NOT
        // apply this step (ball is now airborne).
        BallPhysicsCoefficients k = new(
            gravity:         Fixed.Zero,
            linearDrag:      Fixed.Zero,
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.One,
            rollingFriction: F(50) / F(100)  // 0.50, would be very visible if applied
        );
        BallState start = new(new Vector3Fixed(Fixed.Zero, F(5) / F(100), Fixed.Zero), V3(8, -10, 0), Vector3Fixed.Zero);

        BallState after = BallPhysics.Step(start, k);

        // X velocity preserved (no rolling friction applied because the ball
        // bounced upward and is now airborne).
        Assert.Equal(F(8), after.Velocity.X);
        // Y velocity = +10 (perfect bounce).
        Assert.Equal(F(10), after.Velocity.Y);
    }

    #endregion

    #region Combined: kicked-ball trajectory (sanity gate)

    [Fact]
    public void KickedBall_LandsBackOnGround_AfterReasonableTimeWithPhase3Seeds()
    {
        // Ball kicked at 45° with initial speed 20 m/s — physically plausible
        // football kick. With Phase-3 seeds (gravity 9.81 + drag 0.02 + no
        // spin), the ball should rise, peak, fall, bounce, eventually settle.
        // Sanity gate: after ~5 game-seconds (300 ticks) the ball is ON the
        // ground (Y=0) and has moved forward in +X (didn't fly backward).
        BallPhysicsCoefficients k = BallPhysicsCoefficients.Phase3Seeds;

        // Initial velocity: (sqrt(2)/2)*20 ≈ 14.14 along X, same on Y.
        // Approximate via 14: close enough for a sanity test.
        BallState s = new(Vector3Fixed.Zero, V3(14, 14, 0), Vector3Fixed.Zero);

        for (int i = 0; i < 300; i++)
        {
            s = BallPhysics.Step(s, k);
        }

        Assert.Equal(Fixed.Zero, s.Position.Y);  // settled on ground
        Assert.True(s.Position.X > Fixed.Zero, $"Ball should have moved forward; X = {s.Position.X}");
        // Forward distance should be reasonable (<100 m to filter "flew off").
        Assert.True(s.Position.X < F(100), $"Ball traveled implausibly far; X = {s.Position.X}");
    }

    [Fact]
    public void DroppedBall_AtRest_FallsBouncesAndEventuallySettles()
    {
        // Ball dropped from 5 m, no horizontal velocity, no spin. With
        // realistic gravity + bounce e=0.55, ball should bounce several times
        // and eventually settle (Y=0, |v.y| small) within a few seconds.
        BallPhysicsCoefficients k = BallPhysicsCoefficients.Phase3Seeds;
        BallState s = new(V3(0, 5, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);

        // Step 600 ticks (10 game-seconds) — well past settling time.
        for (int i = 0; i < 600; i++)
        {
            s = BallPhysics.Step(s, k);
        }

        // On ground.
        Assert.Equal(Fixed.Zero, s.Position.Y);
        // Vertical velocity small (the bounces have damped to ≤ 1 m/s; we
        // accept any value because exact damping rate depends on coefficient
        // tuning, but ball must NOT still be in mid-air with ascending Y vel).
        Assert.True(s.Velocity.Y <= Fixed.One, $"Ball still bouncing high; v.y = {s.Velocity.Y}");
    }

    #endregion

    #region Coefficient sanity

    [Fact]
    public void Phase3Seeds_AreDistinctFromZero()
    {
        // Sanity that Phase3Seeds didn't accidentally default everything to 0.
        BallPhysicsCoefficients k = BallPhysicsCoefficients.Phase3Seeds;

        Assert.NotEqual(Fixed.Zero, k.Gravity);
        Assert.NotEqual(Fixed.Zero, k.LinearDrag);
        Assert.NotEqual(Fixed.Zero, k.MagnusCoupling);
        Assert.NotEqual(Fixed.Zero, k.BounceRetention);
        Assert.NotEqual(Fixed.Zero, k.RollingFriction);
    }

    [Fact]
    public void Phase3Seeds_MatchDesignDocumentValues()
    {
        // Lock the seeds to design/match-engine.md §Q2 values. If the design
        // doc tunes these, this test gets updated alongside.
        BallPhysicsCoefficients k = BallPhysicsCoefficients.Phase3Seeds;

        Assert.Equal(F(981) / F(100), k.Gravity);                   // 9.81
        Assert.Equal(F(2) / F(100), k.LinearDrag);                  // 0.02
        Assert.Equal(F(4) / F(10000), k.MagnusCoupling);            // 0.0004
        Assert.Equal(F(55) / F(100), k.BounceRetention);            // 0.55
        Assert.Equal(F(25) / F(100), k.RollingFriction);            // 0.25
    }

    #endregion

    #region Constructor validation (Codex audit P3-01)

    // Tuning-seed bounds are enforced at construction so nonphysical values
    // can't silently destabilize the integrator. Phase-3 stub policy:
    // gravity ≥ 0; per-step retention coefficients in [0, 1]; Magnus ≥ 0.

    [Fact]
    public void Constructor_NegativeGravity_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BallPhysicsCoefficients(
            gravity:         Fixed.FromInt(-1),
            linearDrag:      F(2) / F(100),
            magnusCoupling:  Fixed.Zero,
            bounceRetention: F(55) / F(100),
            rollingFriction: F(25) / F(100)));
    }

    [Fact]
    public void Constructor_LinearDragBelowZero_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BallPhysicsCoefficients(
            gravity:         F(981) / F(100),
            linearDrag:      Fixed.FromInt(-1) / Fixed.FromInt(100),  // -0.01
            magnusCoupling:  Fixed.Zero,
            bounceRetention: F(55) / F(100),
            rollingFriction: F(25) / F(100)));
    }

    [Fact]
    public void Constructor_LinearDragAboveOne_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BallPhysicsCoefficients(
            gravity:         F(981) / F(100),
            linearDrag:      Fixed.FromInt(2),                        // 2.0 — silently inverts velocity
            magnusCoupling:  Fixed.Zero,
            bounceRetention: F(55) / F(100),
            rollingFriction: F(25) / F(100)));
    }

    [Fact]
    public void Constructor_NegativeMagnusCoupling_Throws()
    {
        // Phase-3 stub policy: zero allowed (disables Magnus); negative is not.
        Assert.Throws<ArgumentOutOfRangeException>(() => new BallPhysicsCoefficients(
            gravity:         F(981) / F(100),
            linearDrag:      F(2) / F(100),
            magnusCoupling:  Fixed.FromInt(-1) / Fixed.FromInt(10000),
            bounceRetention: F(55) / F(100),
            rollingFriction: F(25) / F(100)));
    }

    [Fact]
    public void Constructor_BounceRetentionAboveOne_Throws()
    {
        // > 1 means the ball gains energy on every bounce — silent escalation.
        Assert.Throws<ArgumentOutOfRangeException>(() => new BallPhysicsCoefficients(
            gravity:         F(981) / F(100),
            linearDrag:      F(2) / F(100),
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.FromInt(2),
            rollingFriction: F(25) / F(100)));
    }

    [Fact]
    public void Constructor_BounceRetentionBelowZero_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BallPhysicsCoefficients(
            gravity:         F(981) / F(100),
            linearDrag:      F(2) / F(100),
            magnusCoupling:  Fixed.Zero,
            bounceRetention: Fixed.FromInt(-1) / Fixed.FromInt(100),
            rollingFriction: F(25) / F(100)));
    }

    [Fact]
    public void Constructor_RollingFrictionAboveOne_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BallPhysicsCoefficients(
            gravity:         F(981) / F(100),
            linearDrag:      F(2) / F(100),
            magnusCoupling:  Fixed.Zero,
            bounceRetention: F(55) / F(100),
            rollingFriction: Fixed.FromInt(2)));
    }

    [Fact]
    public void Constructor_RollingFrictionBelowZero_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BallPhysicsCoefficients(
            gravity:         F(981) / F(100),
            linearDrag:      F(2) / F(100),
            magnusCoupling:  Fixed.Zero,
            bounceRetention: F(55) / F(100),
            rollingFriction: Fixed.FromInt(-1) / Fixed.FromInt(100)));
    }

    [Fact]
    public void Constructor_BoundaryValues_DoNotThrow()
    {
        // Edge cases: 0 and 1 are both valid for retention coefficients.
        // Critical regression: must NOT over-throw legitimate boundary values.
        var ex = Record.Exception(() => new BallPhysicsCoefficients(
            gravity:         Fixed.Zero,         // 0 gravity = floating-ball world; valid
            linearDrag:      Fixed.Zero,         // 0 = no drag
            magnusCoupling:  Fixed.Zero,         // 0 = Magnus disabled (stub policy)
            bounceRetention: Fixed.One,          // 1 = elastic bounce
            rollingFriction: Fixed.One));        // 1 = rolling stops instantly each tick
        Assert.Null(ex);
    }

    [Fact]
    public void Phase3Seeds_AlwaysConstructible()
    {
        // Sanity: the cached Phase3Seeds singleton initialized successfully —
        // proves the design-doc seed values pass validation.
        var ex = Record.Exception(() => _ = BallPhysicsCoefficients.Phase3Seeds);
        Assert.Null(ex);
    }

    #endregion
}

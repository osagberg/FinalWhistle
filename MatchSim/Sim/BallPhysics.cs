using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Pure-deterministic ball physics integrator per
/// <c>design/match-engine.md §Q2</c>. Semi-implicit Euler at fixed
/// <see cref="Tick.TicksPerSecond"/> (60Hz) step. Forces per step:
/// gravity, linear air drag, Magnus (when spin is non-zero). Collisions:
/// ground bounce + rolling friction. No PhysX, no <c>Time.deltaTime</c>,
/// no platform RNG — Q32.32 fixed-point integer math throughout.
///
/// <para>
/// <strong>Stub policy</strong> (Magnus, design doc §Q2): the Magnus term
/// is structurally present from Month 3 but the coefficient may be zeroed
/// for the gate build if observers find curve-driven moments noisy. This
/// integrator honors that — pass <see cref="BallPhysicsCoefficients.MagnusCoupling"/>
/// = <see cref="Fixed.Zero"/> to disable Magnus while keeping the structure.
/// </para>
///
/// <para>
/// <strong>Coordinate convention:</strong> X + Z form the pitch plane,
/// Y is altitude. Gravity acts on -Y. Ground = <c>Y &lt;= 0</c>; ball
/// position is clamped to <c>Y = 0</c> on ground contact.
/// </para>
/// </summary>
public static class BallPhysics
{
    /// <summary>
    /// Fixed simulation timestep (1/60 s). Pre-computed so the per-step hot
    /// path doesn't pay BigInteger division each call (Fixed division is
    /// expensive per <c>Fixed.cs</c> internals).
    /// </summary>
    private static readonly Fixed Dt = Fixed.FromInt(1) / Fixed.FromInt(Tick.TicksPerSecond);

    /// <summary>
    /// Step the ball one 60Hz tick forward. Pure function: same input ⇒
    /// same output across runs and platforms (the determinism floor).
    /// </summary>
    /// <param name="state">Pre-step ball state.</param>
    /// <param name="coefficients">Physics tuning seeds (gravity / drag / Magnus / bounce / rolling).</param>
    /// <returns>Post-step ball state.</returns>
    public static BallState Step(BallState state, BallPhysicsCoefficients coefficients)
    {
        Vector3Fixed v = state.Velocity;
        bool startedOnGround = state.Position.Y <= Fixed.Zero;

        // 1. Gravity. F = (0, -g, 0); per-step delta on velocity = -g * dt.
        //    g is in m/s² (continuous SI units per design doc), so dt scaling matters.
        v = new Vector3Fixed(v.X, v.Y - coefficients.Gravity * Dt, v.Z);

        // 2. Linear air drag. Per-step coefficient (NOT continuous SI):
        //    v_new = v * (1 - C_d).
        //    C_d already absorbs dt per design doc §Q2 coefficient table.
        Fixed dragRetention = Fixed.One - coefficients.LinearDrag;
        v = v * dragRetention;

        // 3. Magnus. F_m = C_m · (spin × v); per-step coefficient (already absorbs dt).
        //    Skip the cross product if spin is exactly zero — common case for
        //    untouched ball, and avoids spurious Q32.32 multiplications.
        if (!state.Spin.Equals(Vector3Fixed.Zero))
        {
            Vector3Fixed magnus = coefficients.MagnusCoupling * Vector3Fixed.Cross(state.Spin, v);
            v = v + magnus;
        }

        // 4. Position update (semi-implicit: uses NEW velocity).
        //    p_new = p + v * dt.
        Vector3Fixed p = state.Position + v * Dt;

        // 5. Ground collision. Y <= 0 means ball is on or below the pitch.
        if (p.Y <= Fixed.Zero)
        {
            bool crossedIntoGroundFromAir = !startedOnGround && v.Y < Fixed.Zero;

            // Clamp position to exact ground plane. Avoids subterranean drift
            // that would compound over season-long replays.
            p = new Vector3Fixed(p.X, Fixed.Zero, p.Z);

            if (crossedIntoGroundFromAir)
            {
                // Bounce: vertical velocity flipped + scaled by retention.
                // For e=0.55, a -10 m/s downward becomes +5.5 m/s upward.
                v = new Vector3Fixed(v.X, -coefficients.BounceRetention * v.Y, v.Z);
            }
            else if (v.Y < Fixed.Zero)
            {
                // Gravity is applied before contact resolution. A ball that
                // started grounded must stay grounded, not gain a tiny rebound.
                v = new Vector3Fixed(v.X, Fixed.Zero, v.Z);
            }

            // Rolling friction applies only when the ball is "settled" on
            // the ground — i.e., NOT bouncing back upward. Apply when
            // post-bounce v.Y is still <= 0 (ball at rest or moving down,
            // post-clamp).
            if (v.Y <= Fixed.Zero)
            {
                Fixed rollRetention = Fixed.One - coefficients.RollingFriction;
                v = new Vector3Fixed(v.X * rollRetention, v.Y, v.Z * rollRetention);
            }
        }

        return new BallState(p, v, state.Spin);
    }
}

/// <summary>
/// Ball-physics tuning coefficients per <c>design/match-engine.md §Q2</c>.
/// Mix of continuous-time SI quantities (<see cref="Gravity"/> in m/s²) and
/// per-step coefficients (<see cref="LinearDrag"/> / <see cref="MagnusCoupling"/>
/// / <see cref="RollingFriction"/> already absorb dt). The design doc is
/// explicit that these are tuning seeds, not physical truth — expect to
/// re-tune in Phase 3 once the first match is watchable, and again at the
/// Month-3 gate.
/// </summary>
public readonly struct BallPhysicsCoefficients
{
    /// <summary>Gravitational acceleration (m/s²). Continuous SI; dt-scaled per step inside <see cref="BallPhysics.Step"/>.</summary>
    public readonly Fixed Gravity;

    /// <summary>Linear air drag, per-step dimensionless coefficient. <c>v_new = v * (1 - LinearDrag)</c>.</summary>
    public readonly Fixed LinearDrag;

    /// <summary>Magnus coupling, per-step dimensionless coefficient. <c>v_new += MagnusCoupling · (spin × v)</c>.</summary>
    public readonly Fixed MagnusCoupling;

    /// <summary>Vertical bounce retention <c>e ∈ [0, 1]</c>. Post-bounce <c>v.Y = -BounceRetention * v.Y</c>.</summary>
    public readonly Fixed BounceRetention;

    /// <summary>Rolling friction, per-step coefficient applied when ball is on ground. <c>v.{X,Z}_new = v.{X,Z} * (1 - RollingFriction)</c>.</summary>
    public readonly Fixed RollingFriction;

    /// <summary>Construct from explicit values. Use <see cref="Phase3Seeds"/> for the design-doc defaults.</summary>
    public BallPhysicsCoefficients(
        Fixed gravity,
        Fixed linearDrag,
        Fixed magnusCoupling,
        Fixed bounceRetention,
        Fixed rollingFriction)
    {
        Gravity = gravity;
        LinearDrag = linearDrag;
        MagnusCoupling = magnusCoupling;
        BounceRetention = bounceRetention;
        RollingFriction = rollingFriction;
    }

    /// <summary>
    /// Phase-3 starting tuning seeds per <c>design/match-engine.md §Q2</c>:
    /// <c>g=9.81</c> m/s², <c>C_d=0.02</c>/step, <c>C_m=0.0004</c>/step,
    /// <c>e=0.55</c>, <c>μ_step=0.25</c>/step. Expected to re-tune through
    /// Phase 3 visual playtests; Magnus may be zeroed for Month-3 gate per
    /// stub policy.
    /// </summary>
    public static BallPhysicsCoefficients Phase3Seeds => Phase3SeedValues;

    private static readonly BallPhysicsCoefficients Phase3SeedValues = new(
        gravity:         FixedRatio(981, 100),       // 9.81
        linearDrag:      FixedRatio(2, 100),         // 0.02
        magnusCoupling:  FixedRatio(4, 10000),       // 0.0004
        bounceRetention: FixedRatio(55, 100),        // 0.55
        rollingFriction: FixedRatio(25, 100)         // 0.25
    );

    /// <summary>Helper: build a Fixed ratio without parsing decimal strings (which would be Phase-3 tuning-knob friction).</summary>
    private static Fixed FixedRatio(int numerator, int denominator)
        => Fixed.FromInt(numerator) / Fixed.FromInt(denominator);
}

using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Per-tick kick command emitted by <see cref="BehaviorTreeRunner.Tick"/>
/// for a player in possession of a stationary-or-slow ball. Consumed by
/// <see cref="MatchSimulationRunner"/>, which writes <see cref="Velocity"/>
/// into <see cref="BallState.Velocity"/> + (Phase-4+) <see cref="Spin"/>
/// into <see cref="BallState.Spin"/> before the next
/// <see cref="BallPhysics.Step"/>.
///
/// <para>
/// <strong>Phase-3 minimum</strong>: Spin is always <see cref="Vector3Fixed.Zero"/>
/// per the Magnus-stub policy (Ball physics keeps the field shape for
/// forward-compat but doesn't model curl until Phase 4+ when per-player
/// gene-driven kick variance lands). Velocity is the ground-plane kick
/// direction × pass-or-long-ball speed; canonical pitch-plane convention
/// (Y stays Zero for ground rolls).
/// </para>
///
/// <para>
/// <strong>Determinism</strong>: Q32.32 throughout. Constructor allows any
/// finite Vector3Fixed values; the kick-emission heuristic upstream
/// guarantees magnitudes within Phase-3 bounds (Pass 8-35m / LongBall to
/// opponent goal).
/// </para>
/// </summary>
public readonly struct KickIntent : IEquatable<KickIntent>
{
    public readonly Vector3Fixed Velocity;
    public readonly Vector3Fixed Spin;

    public KickIntent(Vector3Fixed velocity, Vector3Fixed spin)
    {
        Velocity = velocity;
        Spin = spin;
    }

    /// <summary>Kick with zero spin (Phase-3 default per Magnus stub).</summary>
    public static KickIntent Ground(Vector3Fixed velocity) => new(velocity, Vector3Fixed.Zero);

    public bool Equals(KickIntent other)
        => Velocity.Equals(other.Velocity) && Spin.Equals(other.Spin);

    public override bool Equals(object? obj) => obj is KickIntent other && Equals(other);

    public override int GetHashCode()
    {
        unchecked
        {
            int h = 17;
            h = h * 31 + Velocity.GetHashCode();
            h = h * 31 + Spin.GetHashCode();
            return h;
        }
    }

    public static bool operator ==(KickIntent left, KickIntent right) => left.Equals(right);
    public static bool operator !=(KickIntent left, KickIntent right) => !left.Equals(right);

    public override string ToString() => $"KickIntent(v={Velocity}, spin={Spin})";
}

using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Immutable canonical ball state. Pure data — no behavior; all evolution
/// happens through <see cref="BallPhysics.Step"/>. Per
/// <c>design/match-engine.md §Q2</c> the structure is locked at v1:
/// <c>position</c> + <c>velocity</c> + <c>spin</c>, all <see cref="Vector3Fixed"/>.
///
/// <para>
/// Coordinates: <c>X</c> + <c>Z</c> = pitch plane; <c>Y</c> = altitude.
/// Position in metres; velocity in m/s; spin in rad/s.
/// </para>
///
/// <para>
/// <strong>Canonical serialization order:</strong> Position.X, Position.Y,
/// Position.Z, Velocity.X, Velocity.Y, Velocity.Z, Spin.X, Spin.Y, Spin.Z —
/// 9 × 8 bytes = 72 bytes. Locked at v1 by <see cref="WriteCanonical"/>.
/// Changing this order = silent corpus-fixture invalidation.
/// </para>
/// </summary>
public readonly struct BallState : IEquatable<BallState>
{
    /// <summary>Ball position in pitch coordinates (metres).</summary>
    public readonly Vector3Fixed Position;

    /// <summary>Ball linear velocity (m/s).</summary>
    public readonly Vector3Fixed Velocity;

    /// <summary>Ball angular velocity / spin (rad/s). Zero unless a signature or kick imparts spin.</summary>
    public readonly Vector3Fixed Spin;

    /// <summary>Construct from explicit components.</summary>
    public BallState(Vector3Fixed position, Vector3Fixed velocity, Vector3Fixed spin)
    {
        Position = position;
        Velocity = velocity;
        Spin = spin;
    }

    /// <summary>Ball at rest at origin (kick-off start state, before any kick).</summary>
    public static BallState AtRest => default;

    /// <summary>
    /// Write this ball state to a <see cref="CanonicalEncoder"/> in the
    /// locked v1 order (Position / Velocity / Spin, each as Vector3Fixed
    /// X / Y / Z). 72 bytes total. Caller is responsible for encoder
    /// lifecycle.
    /// </summary>
    public void WriteCanonical(CanonicalEncoder encoder)
    {
        if (encoder is null)
        {
            throw new ArgumentNullException(nameof(encoder));
        }
        encoder.WriteVector3Fixed(Position);
        encoder.WriteVector3Fixed(Velocity);
        encoder.WriteVector3Fixed(Spin);
    }

    #region Equality + ToString

    /// <inheritdoc />
    public bool Equals(BallState other)
        => Position.Equals(other.Position) && Velocity.Equals(other.Velocity) && Spin.Equals(other.Spin);

    /// <inheritdoc />
    public override bool Equals(object? obj) => obj is BallState other && Equals(other);

    /// <inheritdoc />
    public override int GetHashCode()
    {
        unchecked
        {
            int h = 17;
            h = h * 31 + Position.GetHashCode();
            h = h * 31 + Velocity.GetHashCode();
            h = h * 31 + Spin.GetHashCode();
            return h;
        }
    }

    public static bool operator ==(BallState left, BallState right) => left.Equals(right);

    public static bool operator !=(BallState left, BallState right) => !left.Equals(right);

    /// <inheritdoc />
    public override string ToString()
        => $"BallState(P={Position}, V={Velocity}, S={Spin})";

    #endregion
}

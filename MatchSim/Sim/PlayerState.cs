using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Immutable canonical player state. 22 of these per match — one per
/// player on the pitch. Position + Velocity are <see cref="Vector3Fixed"/>
/// for forward-compatibility with jumping headers / set-piece flight, but
/// the Y component is invariant <c>Fixed.Zero</c> for ground players in
/// the Month-3 slice (no jumping yet per <c>match-engine.md §Q4</c>).
///
/// <para>
/// <strong>Canonical serialization order:</strong> Position.X, Position.Y,
/// Position.Z, Velocity.X, Velocity.Y, Velocity.Z, JerseyNumber (1 byte),
/// Side (1 byte) — 6×8 + 1 + 1 = 50 bytes. Locked at v1 by
/// <see cref="WriteCanonical"/>. Adding a field = corpus-fixture invalidation
/// (handle via SerializationContract version bump).
/// </para>
///
/// <para>
/// <strong>JerseyNumber:</strong> 1-99 inclusive per football convention;
/// the encoder writes it as a single byte, so 0 is technically representable
/// in the wire format but never legal at runtime (uninitialized sentinel).
/// </para>
/// </summary>
public readonly struct PlayerState : IEquatable<PlayerState>
{
    /// <summary>Player position in pitch coordinates (metres). <c>Y = 0</c> for ground players (Month-3 invariant).</summary>
    public readonly Vector3Fixed Position;

    /// <summary>Player linear velocity (m/s). <c>Y = 0</c> for ground players (Month-3 invariant).</summary>
    public readonly Vector3Fixed Velocity;

    /// <summary>Jersey number 1-99. Display + identity overlay; never used as a stable cross-match identity (use roster-side player ids for that).</summary>
    public readonly byte JerseyNumber;

    /// <summary>Which team this player belongs to.</summary>
    public readonly TeamSide Side;

    /// <summary>Construct from explicit components.</summary>
    public PlayerState(Vector3Fixed position, Vector3Fixed velocity, byte jerseyNumber, TeamSide side)
    {
        ValidateJerseyNumber(jerseyNumber);
        ValidateTeamSide(side);

        Position = position;
        Velocity = velocity;
        JerseyNumber = jerseyNumber;
        Side = side;
    }

    /// <summary>
    /// Write this player state to a <see cref="CanonicalEncoder"/> in the
    /// locked v1 order. 50 bytes total.
    /// </summary>
    public void WriteCanonical(CanonicalEncoder encoder)
    {
        if (encoder is null)
        {
            throw new ArgumentNullException(nameof(encoder));
        }
        if (!IsValidJerseyNumber(JerseyNumber) || !IsValidTeamSide(Side))
        {
            throw new InvalidOperationException("PlayerState contains invalid identity fields and cannot be serialized canonically.");
        }
        encoder.WriteVector3Fixed(Position);
        encoder.WriteVector3Fixed(Velocity);
        encoder.WriteByte(JerseyNumber);
        encoder.WriteByte((byte)Side);
    }

    private static void ValidateJerseyNumber(byte jerseyNumber)
    {
        if (!IsValidJerseyNumber(jerseyNumber))
        {
            throw new ArgumentOutOfRangeException(nameof(jerseyNumber), jerseyNumber, "JerseyNumber must be in [1, 99].");
        }
    }

    private static bool IsValidJerseyNumber(byte jerseyNumber)
        => jerseyNumber is >= 1 and <= 99;

    private static void ValidateTeamSide(TeamSide side)
    {
        if (!IsValidTeamSide(side))
        {
            throw new ArgumentOutOfRangeException(nameof(side), side, "TeamSide must be Home or Away.");
        }
    }

    private static bool IsValidTeamSide(TeamSide side)
        => side is TeamSide.Home or TeamSide.Away;

    #region Equality + ToString

    /// <inheritdoc />
    public bool Equals(PlayerState other)
        => Position.Equals(other.Position)
        && Velocity.Equals(other.Velocity)
        && JerseyNumber == other.JerseyNumber
        && Side == other.Side;

    /// <inheritdoc />
    public override bool Equals(object? obj) => obj is PlayerState other && Equals(other);

    /// <inheritdoc />
    public override int GetHashCode()
    {
        unchecked
        {
            int h = 17;
            h = h * 31 + Position.GetHashCode();
            h = h * 31 + Velocity.GetHashCode();
            h = h * 31 + JerseyNumber.GetHashCode();
            h = h * 31 + (int)Side;
            return h;
        }
    }

    public static bool operator ==(PlayerState left, PlayerState right) => left.Equals(right);

    public static bool operator !=(PlayerState left, PlayerState right) => !left.Equals(right);

    /// <inheritdoc />
    public override string ToString()
        => $"PlayerState(#{JerseyNumber} {Side}, P={Position}, V={Velocity})";

    #endregion
}

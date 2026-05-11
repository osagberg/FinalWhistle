using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// What a behavior-tree archetype emits per player per tick. The
/// <see cref="PlayerActuator"/> consumes <see cref="DesiredPosition"/> +
/// <see cref="DesiredSpeed"/> and applies kinematic caps to produce the
/// player's actual <see cref="PlayerState"/> step. Per
/// <c>design/match-engine.md §Q3</c> exit clause: <c>desired_position</c>
/// + <c>desired_speed</c> are the locked BT-output shape; switching to
/// continuous force integration requires a new ADR.
///
/// <para>
/// <strong>Phase-3 pass-the-ball extension</strong>: an optional
/// <see cref="Kick"/> field carries the carrier's kick intent. When set,
/// <see cref="MatchSimulationRunner"/> applies the kick to
/// <see cref="BallState.Velocity"/> before the next
/// <see cref="BallPhysics.Step"/>. Nullable so existing callers that
/// don't issue kicks remain byte-compatible — and so the per-tick BT
/// emission only allocates a struct when a kick actually fires.
/// </para>
/// </summary>
public readonly struct PlayerCommand : IEquatable<PlayerCommand>
{
    /// <summary>Where the player should head, in pitch coordinates (metres). Y is invariant zero for ground players.</summary>
    public readonly Vector3Fixed DesiredPosition;

    /// <summary>How fast the player should move toward <see cref="DesiredPosition"/>. Capped to <see cref="PlayerKinematics.MaxSpeed"/> by the actuator.</summary>
    public readonly Fixed DesiredSpeed;

    /// <summary>
    /// Optional kick intent for the player in possession. Null on every
    /// player who is NOT the in-possession carrier this tick. When set,
    /// the match runner writes <see cref="KickIntent.Velocity"/> into
    /// <see cref="BallState.Velocity"/> + (Phase-4+) Spin into the ball's
    /// spin field. Phase-3 carriers emit kicks only when ball velocity
    /// magnitude is below <c>CarrierKickGateMetresPerSecond</c> (default
    /// 2 m/s) — so "kick the rolling ball" weirdness doesn't fire.
    /// </summary>
    public readonly KickIntent? Kick;

    /// <summary>Construct without a kick (the common case — only the carrier kicks per tick).</summary>
    public PlayerCommand(Vector3Fixed desiredPosition, Fixed desiredSpeed)
        : this(desiredPosition, desiredSpeed, null)
    {
    }

    /// <summary>Construct with explicit kick intent.</summary>
    public PlayerCommand(Vector3Fixed desiredPosition, Fixed desiredSpeed, KickIntent? kick)
    {
        DesiredPosition = desiredPosition;
        DesiredSpeed = desiredSpeed;
        Kick = kick;
    }

    /// <summary>"Stand still" sentinel — desired position = current player position; desired speed = zero. Used when no tactical decision applies.</summary>
    public static PlayerCommand Halt(Vector3Fixed atPosition) => new(atPosition, Fixed.Zero);

    /// <summary>Return a copy of this command with the given <see cref="KickIntent"/> attached.</summary>
    public PlayerCommand WithKick(KickIntent kick) => new(DesiredPosition, DesiredSpeed, kick);

    /// <inheritdoc />
    public bool Equals(PlayerCommand other)
        => DesiredPosition.Equals(other.DesiredPosition)
        && DesiredSpeed.Equals(other.DesiredSpeed)
        && Nullable.Equals(Kick, other.Kick);

    /// <inheritdoc />
    public override bool Equals(object? obj) => obj is PlayerCommand other && Equals(other);

    /// <inheritdoc />
    public override int GetHashCode()
    {
        unchecked
        {
            int h = 17;
            h = h * 31 + DesiredPosition.GetHashCode();
            h = h * 31 + DesiredSpeed.GetHashCode();
            h = h * 31 + (Kick.HasValue ? Kick.Value.GetHashCode() : 0);
            return h;
        }
    }

    public static bool operator ==(PlayerCommand left, PlayerCommand right) => left.Equals(right);

    public static bool operator !=(PlayerCommand left, PlayerCommand right) => !left.Equals(right);

    /// <inheritdoc />
    public override string ToString()
        => Kick.HasValue
            ? $"PlayerCommand(target={DesiredPosition}, speed={DesiredSpeed}, kick={Kick.Value})"
            : $"PlayerCommand(target={DesiredPosition}, speed={DesiredSpeed})";
}

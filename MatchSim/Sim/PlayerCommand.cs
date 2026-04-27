using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// What a behavior-tree archetype emits per player per tick. The
/// <see cref="PlayerActuator"/> consumes these and applies kinematic caps
/// to produce the player's actual <see cref="PlayerState"/> step. Per
/// <c>design/match-engine.md §Q3</c> exit clause: <c>desired_position</c>
/// + <c>desired_speed</c> are the locked BT-output shape; switching to
/// continuous force integration requires a new ADR.
/// </summary>
public readonly struct PlayerCommand : IEquatable<PlayerCommand>
{
    /// <summary>Where the player should head, in pitch coordinates (metres). Y is invariant zero for ground players.</summary>
    public readonly Vector3Fixed DesiredPosition;

    /// <summary>How fast the player should move toward <see cref="DesiredPosition"/>. Capped to <see cref="PlayerKinematics.MaxSpeed"/> by the actuator.</summary>
    public readonly Fixed DesiredSpeed;

    /// <summary>Construct from explicit components.</summary>
    public PlayerCommand(Vector3Fixed desiredPosition, Fixed desiredSpeed)
    {
        DesiredPosition = desiredPosition;
        DesiredSpeed = desiredSpeed;
    }

    /// <summary>"Stand still" sentinel — desired position = current player position; desired speed = zero. Used when no tactical decision applies.</summary>
    public static PlayerCommand Halt(Vector3Fixed atPosition) => new(atPosition, Fixed.Zero);

    /// <inheritdoc />
    public bool Equals(PlayerCommand other)
        => DesiredPosition.Equals(other.DesiredPosition) && DesiredSpeed.Equals(other.DesiredSpeed);

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
            return h;
        }
    }

    public static bool operator ==(PlayerCommand left, PlayerCommand right) => left.Equals(right);

    public static bool operator !=(PlayerCommand left, PlayerCommand right) => !left.Equals(right);

    /// <inheritdoc />
    public override string ToString() => $"PlayerCommand(target={DesiredPosition}, speed={DesiredSpeed})";
}

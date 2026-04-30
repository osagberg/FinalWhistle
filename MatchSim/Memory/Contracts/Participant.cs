using System;

namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Per-event participant descriptor per ADR-0004 §"`MemoryEvent`
/// schema" participants array. Pairs a role string ("scorer" / "assist"
/// / "opponent" / "fan_base" / etc.) with a content-pack-qualified
/// entity ID (player ID / club ID / etc.). Phase-3 minimum exercises
/// no participant tracking on emission (Goal events ship with empty
/// Participants array because the canonical <c>KeyEvent</c> stream
/// doesn't carry scorer attribution yet — that ships Phase 4).
///
/// <para>
/// Role is a string (not an enum) to allow content-pack extension
/// without a schema bump; Phase-4+ validator lints against a registered
/// role-name set per the modding constraints in <c>design/modding.md</c>.
/// </para>
/// </summary>
public readonly struct Participant : IEquatable<Participant>
{
    public string Role { get; }
    public string EntityId { get; }

    public Participant(string role, string entityId)
    {
        if (string.IsNullOrEmpty(role))
        {
            throw new ArgumentException(
                "Role must be non-empty.", nameof(role));
        }
        if (string.IsNullOrEmpty(entityId))
        {
            throw new ArgumentException(
                "EntityId must be non-empty.", nameof(entityId));
        }
        Role = role;
        EntityId = entityId;
    }

    public bool Equals(Participant other) =>
        Role == other.Role && EntityId == other.EntityId;

    public override bool Equals(object? obj) => obj is Participant other && Equals(other);

    public override int GetHashCode() => HashCode.Combine(Role, EntityId);

    public static bool operator ==(Participant left, Participant right) => left.Equals(right);
    public static bool operator !=(Participant left, Participant right) => !left.Equals(right);

    public override string ToString() => $"{Role}={EntityId}";
}

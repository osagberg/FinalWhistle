using System;

namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Source-system descriptor for a <see cref="MemoryEvent"/> per ADR-0004
/// §"`MemoryEvent` schema" emitter field. Pairs the
/// <see cref="EmitterKind"/> with a stable <c>SourceId</c> string — for
/// Phase-3 match emissions, <c>SourceId</c> is the match-id string used
/// throughout the corpus replay path.
/// </summary>
public readonly struct EventEmitter : IEquatable<EventEmitter>
{
    public EmitterKind Kind { get; }
    public string SourceId { get; }

    public EventEmitter(EmitterKind kind, string sourceId)
    {
        if (kind == EmitterKind.None)
        {
            throw new ArgumentException(
                "EmitterKind.None is the sentinel; never construct an EventEmitter with it.",
                nameof(kind));
        }
        if (string.IsNullOrEmpty(sourceId))
        {
            throw new ArgumentException(
                "SourceId must be non-empty.", nameof(sourceId));
        }
        Kind = kind;
        SourceId = sourceId;
    }

    public bool Equals(EventEmitter other) =>
        Kind == other.Kind && SourceId == other.SourceId;

    public override bool Equals(object? obj) => obj is EventEmitter other && Equals(other);

    public override int GetHashCode() => HashCode.Combine((byte)Kind, SourceId);

    public static bool operator ==(EventEmitter left, EventEmitter right) => left.Equals(right);
    public static bool operator !=(EventEmitter left, EventEmitter right) => !left.Equals(right);

    public override string ToString() => $"{Kind}:{SourceId}";
}

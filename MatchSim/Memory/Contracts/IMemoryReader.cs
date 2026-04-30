using System;
using System.Collections.Generic;

namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Stable identifier for a <see cref="IMemoryReader"/> implementation.
/// Wraps a string so call sites cannot accidentally swap a tag-id for a
/// reader-id at the type system level. Phase-3 has exactly one reader
/// (<c>press-fan-reader</c>); Phase-4+ adds the other 4 from the
/// design/event-sourced-memory.md §Five-readers spec.
/// </summary>
public readonly struct ReaderId : IEquatable<ReaderId>
{
    public string Value { get; }

    public ReaderId(string value)
    {
        if (string.IsNullOrEmpty(value))
        {
            throw new ArgumentException(
                "ReaderId must be non-empty.", nameof(value));
        }
        Value = value;
    }

    public bool Equals(ReaderId other) => Value == other.Value;
    public override bool Equals(object? obj) => obj is ReaderId other && Equals(other);
    public override int GetHashCode() => Value.GetHashCode();
    public static bool operator ==(ReaderId left, ReaderId right) => left.Equals(right);
    public static bool operator !=(ReaderId left, ReaderId right) => !left.Equals(right);
    public override string ToString() => Value;
}

/// <summary>
/// Reader-side query interface per ADR-0004 §"Key Interfaces" /
/// <c>IMemoryReader</c>. Each reader subscribes to the <c>Ledger</c>
/// + filters by tag + band + season-window + per-reader logic, returns
/// surfacing decisions as <see cref="CallbackCandidate"/> values. Phase-3
/// minimum: one implementation (<c>PressFanReader</c>).
/// </summary>
public interface IMemoryReader
{
    /// <summary>Stable reader identifier matching the consuming-readers
    /// metadata on relevant <see cref="CallbackTag"/> entries.</summary>
    ReaderId Id { get; }

    /// <summary>
    /// Run the query against the reader's underlying ledger view + return
    /// surfacing candidates. Order is implementation-defined (the
    /// PressFanReader sorts by surfacing-salience descending). Empty
    /// result is valid + means "nothing surfaces this opportunity" —
    /// callers should handle empty enumerables.
    /// </summary>
    IEnumerable<CallbackCandidate> Query(ReaderQuery q);
}

using System;
using System.Collections.Generic;
using System.Linq;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Append-only career-memory record per ADR-0004 §"`MemoryEvent` schema."
/// Pure value type — zero logic, zero Unity refs, zero reflection.
/// Constructor enforces every documented invariant up-front so a malformed
/// event cannot reach <see cref="Ledger.Emit"/>.
///
/// <para>
/// <strong>Phase-3 minimum subset</strong> per the architect blueprint:
/// drops the <c>Consequences</c> array (Phase-4+ when contracts /
/// attribute-deltas land) + the per-event <c>CallbackEligibility</c>
/// override (Phase-4+ when tags need per-event recall_after / expires_after
/// overrides). Tag defaults from <see cref="CallbackTag"/> registry suffice
/// for the Phase-3 single-reader demo.
/// </para>
///
/// <para>
/// <strong>Determinism</strong>: all numeric fields are Q32.32 / integer
/// types; <see cref="Id"/> is a deterministic-from-inputs string; no
/// floats, no <see cref="DateTime"/>, no <see cref="Guid"/>. Salience is
/// computed at emission via <c>SalienceEngine.Compute</c> + frozen on the
/// record (append-only invariant); <see cref="SalienceInputs"/> is
/// persisted alongside as the audit trail.
/// </para>
/// </summary>
public readonly struct MemoryEvent : IEquatable<MemoryEvent>
{
    /// <summary>
    /// Schema version of the <see cref="MemoryEvent"/> shape.
    /// Increment on any field-level change; the bump triggers the
    /// 4-test save-migration discipline per
    /// <c>design/specs/save-migration-fixtures.md</c>.
    /// </summary>
    public const ushort CurrentSchemaVersion = 1;

    public string Id { get; }
    public string? MatchId { get; }
    public ushort Season { get; }
    public uint? Tick { get; }
    public CareerDate CareerDate { get; }
    public EventEmitter Emitter { get; }
    /// <summary>
    /// Read-only list of participants. Backed by a <see cref="Participant"/>[]
    /// at construction; <see cref="IReadOnlyList{T}"/> chosen over
    /// <c>ImmutableArray</c> for the same reason as
    /// <see cref="CallbackTag.ConsumingReaders"/> — Unity 6 Mono doesn't
    /// ship <c>System.Collections.Immutable</c> inboxed. Treat as
    /// immutable; never cast back to the underlying array + mutate.
    /// Empty array is valid (Phase-3 GoalScored events ship without
    /// participant attribution); null is rejected by the constructor.
    /// </summary>
    public IReadOnlyList<Participant> Participants { get; }
    public EventClass What { get; }
    public Fixed Stakes { get; }
    public Emotion Emotion { get; }
    public Fixed Salience { get; }
    public SalienceInputs SalienceInputs { get; }
    public ushort SalienceModelVersion { get; }
    public ushort SchemaVersion { get; }

    public MemoryEvent(
        string id,
        string? matchId,
        ushort season,
        uint? tick,
        CareerDate careerDate,
        EventEmitter emitter,
        IReadOnlyList<Participant> participants,
        EventClass what,
        Fixed stakes,
        Emotion emotion,
        Fixed salience,
        SalienceInputs salienceInputs,
        ushort salienceModelVersion,
        ushort schemaVersion)
    {
        if (string.IsNullOrEmpty(id))
        {
            throw new ArgumentException("Id must be non-empty.", nameof(id));
        }
        if (what == EventClass.None)
        {
            throw new ArgumentException(
                "EventClass.None is the sentinel; never construct a MemoryEvent with it.",
                nameof(what));
        }
        if (emotion == Emotion.None)
        {
            throw new ArgumentException(
                "Emotion.None is the sentinel; never construct a MemoryEvent with it.",
                nameof(emotion));
        }
        if (stakes < Fixed.Zero || stakes > Fixed.One)
        {
            throw new ArgumentOutOfRangeException(nameof(stakes), stakes,
                "Stakes must lie in [0, 1].");
        }
        if (salience < Fixed.Zero || salience > Fixed.One)
        {
            throw new ArgumentOutOfRangeException(nameof(salience), salience,
                "Salience must lie in [0, 1].");
        }
        if (participants is null)
        {
            throw new ArgumentNullException(nameof(participants),
                "Participants must not be null. Use Array.Empty<Participant>() for " +
                "events without participant attribution.");
        }
        if (schemaVersion != CurrentSchemaVersion)
        {
            throw new ArgumentException(
                $"SchemaVersion {schemaVersion} does not match CurrentSchemaVersion " +
                $"({CurrentSchemaVersion}). Future-version events must be migrated via " +
                "MigrationChain (Phase-4+) before Emit is called.",
                nameof(schemaVersion));
        }

        Id = id;
        MatchId = matchId;
        Season = season;
        Tick = tick;
        CareerDate = careerDate;
        Emitter = emitter;
        // Defensive copy per pr-review-toolkit:type-design-analyzer round-1
        // finding 1: callers passing a List<Participant> retained a mutation
        // handle to ledger state. ToArray gives the readonly struct an
        // immutable backing T[] regardless of caller's underlying type. One
        // allocation per emission — negligible vs the ledger's lifetime
        // (Phase-3 ledger volume: hundreds of events per season at most).
        Participants = participants.ToArray();
        What = what;
        Stakes = stakes;
        Emotion = emotion;
        Salience = salience;
        SalienceInputs = salienceInputs;
        SalienceModelVersion = salienceModelVersion;
        SchemaVersion = schemaVersion;
    }

    public bool Equals(MemoryEvent other) =>
        Id == other.Id
        && MatchId == other.MatchId
        && Season == other.Season
        && Tick == other.Tick
        && CareerDate == other.CareerDate
        && Emitter == other.Emitter
        && ParticipantsEqual(Participants, other.Participants)
        && What == other.What
        && Stakes == other.Stakes
        && Emotion == other.Emotion
        && Salience == other.Salience
        && SalienceInputs == other.SalienceInputs
        && SalienceModelVersion == other.SalienceModelVersion
        && SchemaVersion == other.SchemaVersion;

    public override bool Equals(object? obj) => obj is MemoryEvent other && Equals(other);

    public override int GetHashCode() => HashCode.Combine(
        Id, Season, What, Salience, SchemaVersion);

    public static bool operator ==(MemoryEvent left, MemoryEvent right) => left.Equals(right);
    public static bool operator !=(MemoryEvent left, MemoryEvent right) => !left.Equals(right);

    private static bool ParticipantsEqual(IReadOnlyList<Participant> a, IReadOnlyList<Participant> b)
    {
        if (ReferenceEquals(a, b)) return true;
        if (a.Count != b.Count) return false;
        for (int i = 0; i < a.Count; i++)
        {
            if (!a[i].Equals(b[i])) return false;
        }
        return true;
    }
}

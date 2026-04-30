using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Memory;

/// <summary>
/// Phase-3 minimum append-only career-memory ledger per ADR-0004
/// §"Architecture Sketch" / <c>LedgerAppender</c>. Holds emitted
/// <see cref="MemoryEvent"/>s in insertion order; supports tag/season
/// queries by readers. Compaction + serialization + migration are
/// deliberately deferred to Phase 6 (compaction) / Phase 4+ (save
/// migration once schema-v2 lands).
///
/// <para>
/// <strong>Append-only.</strong> No deletion, no in-place mutation —
/// the only public mutator is <see cref="Emit"/>.
/// </para>
///
/// <para>
/// <strong>Salience-precomputed contract.</strong> The caller must use
/// <see cref="SalienceEngine.Compute"/> to compute Salience BEFORE
/// constructing the <see cref="MemoryEvent"/>. <see cref="Emit"/>
/// validates that Salience is in <c>[0, 1]</c> (matches the
/// <see cref="MemoryEvent"/> constructor invariant) but does NOT
/// recompute. Hidden recompute would surprise tests + couple
/// <c>Ledger</c> to the weight-table version, which is the wrong axis.
/// </para>
/// </summary>
public sealed class Ledger
{
    private readonly List<MemoryEvent> _events = new();

    /// <summary>
    /// Append a pre-built event to the ledger. Validates schema version
    /// + Salience bounds + non-default Participants + recognized
    /// SalienceModelVersion. Caller is responsible for computing Salience
    /// via <see cref="SalienceEngine.Compute"/>.
    /// </summary>
    public void Emit(MemoryEvent memoryEvent)
    {
        if (memoryEvent.SchemaVersion != MemoryEvent.CurrentSchemaVersion)
        {
            throw new ArgumentException(
                $"MemoryEvent.SchemaVersion {memoryEvent.SchemaVersion} != current " +
                $"{MemoryEvent.CurrentSchemaVersion}. Migrate via MigrationChain (Phase-4+) " +
                "before Emit.",
                nameof(memoryEvent));
        }
        if (memoryEvent.What == EventClass.None)
        {
            throw new ArgumentException(
                "Cannot emit MemoryEvent with EventClass.None.", nameof(memoryEvent));
        }
        if (memoryEvent.Salience < Fixed.Zero || memoryEvent.Salience > Fixed.One)
        {
            throw new ArgumentOutOfRangeException(
                nameof(memoryEvent), memoryEvent.Salience,
                "MemoryEvent.Salience must lie in [0, 1].");
        }
        if (memoryEvent.Participants is null)
        {
            throw new ArgumentException(
                "MemoryEvent.Participants must not be null.",
                nameof(memoryEvent));
        }
        if (memoryEvent.SalienceModelVersion != SalienceWeights.Phase3ModelVersion)
        {
            throw new ArgumentException(
                $"Unknown SalienceModelVersion {memoryEvent.SalienceModelVersion}. " +
                $"Phase-3 only recognises version {SalienceWeights.Phase3ModelVersion}.",
                nameof(memoryEvent));
        }
        _events.Add(memoryEvent);
    }

    /// <summary>
    /// Return events at-or-after the given career season (inclusive).
    /// Phase-3 minimum: linear scan over the in-memory list. Phase-6+
    /// hot path swaps in a season-indexed structure once the
    /// balance-harness 20-year synthetic save proves the linear path
    /// hot. Returned list is a snapshot copy; callers may iterate
    /// without seeing later <see cref="Emit"/> calls.
    /// </summary>
    public IReadOnlyList<MemoryEvent> SinceSeason(ushort season)
    {
        List<MemoryEvent> result = new();
        for (int i = 0; i < _events.Count; i++)
        {
            if (_events[i].Season >= season)
            {
                result.Add(_events[i]);
            }
        }
        return result;
    }

    /// <summary>
    /// All events in insertion order, returned as a snapshot copy.
    /// Snapshot semantics match <see cref="SinceSeason"/> per
    /// pr-review-toolkit round-1 finding I1 — the previous
    /// implementation returned the live backing list, which was
    /// inconsistent with <see cref="SinceSeason"/> and would corrupt
    /// reader iteration if any Phase-4+ caller emitted while a query
    /// was active. One allocation per call; ledger sizes are O(events
    /// per season) at Phase-3 — tens to hundreds — well under any
    /// budget.
    /// </summary>
    public IReadOnlyList<MemoryEvent> All
    {
        get
        {
            MemoryEvent[] snapshot = new MemoryEvent[_events.Count];
            for (int i = 0; i < _events.Count; i++)
            {
                snapshot[i] = _events[i];
            }
            return snapshot;
        }
    }

    /// <summary>Number of events appended.</summary>
    public int Count => _events.Count;
}

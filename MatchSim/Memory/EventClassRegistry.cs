using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Memory;

/// <summary>
/// Per-<see cref="EventClass"/> static registry: emission-time base
/// weights (<see cref="BaseWeightFor"/>) + tag attachments
/// (<see cref="TagsFor"/>). Phase-3 minimum: only
/// <see cref="EventClass.GoalScored"/> populated. Phase-4+ adds entries
/// for the rest of the ~42-class catalog as their emission sources land.
///
/// <para>
/// Pinned base-weight values are Phase-6 tuning seeds (live in
/// <c>design/event-sourced-memory.md</c>, not SPEC-locked) — adjustable
/// without an ADR change.
/// </para>
/// </summary>
public static class EventClassRegistry
{
    private static readonly Dictionary<EventClass, Fixed> _baseWeights;
    private static readonly Dictionary<EventClass, IReadOnlyList<string>> _tags;

    /// <summary>Phase-3 base weight for <see cref="EventClass.GoalScored"/>: 0.6 — a goal is notable by default.</summary>
    public static readonly Fixed GoalScoredBaseWeight = Fixed.Parse("0.6000000000");

    static EventClassRegistry()
    {
        _baseWeights = new Dictionary<EventClass, Fixed>
        {
            [EventClass.GoalScored] = GoalScoredBaseWeight,
        };
        // ReadOnlyCollection wrap per pr-review-toolkit round-1 P2:
        // a raw T[] cast back to string[] would let consumers mutate
        // the registry tag-attachments at runtime.
        _tags = new Dictionary<EventClass, IReadOnlyList<string>>
        {
            [EventClass.GoalScored] = new ReadOnlyCollection<string>(new[]
            {
                CallbackTagRegistry.PressFanId,
                CallbackTagRegistry.ScoringMilestoneId,
            }),
        };
    }

    /// <summary>
    /// Emission-time base weight for the given event class. Throws on
    /// <see cref="EventClass.None"/> or any unregistered Phase-4+ class
    /// — a missing entry is a content-pack authoring bug, not a runtime
    /// fallback condition.
    /// </summary>
    public static Fixed BaseWeightFor(EventClass eventClass)
    {
        if (!_baseWeights.TryGetValue(eventClass, out Fixed weight))
        {
            throw new ArgumentOutOfRangeException(nameof(eventClass), eventClass,
                $"No base weight registered for EventClass.{eventClass}. " +
                "Phase-3 only registers GoalScored; expand the registry as new event classes land.");
        }
        return weight;
    }

    /// <summary>
    /// Callback-tag IDs attached to the given event class. Returns
    /// content-pack-qualified tag IDs (<c>fwh.core:tag.&lt;slug&gt;</c>)
    /// per ADR-0006 ID format. Throws on unregistered classes for the
    /// same reason as <see cref="BaseWeightFor"/>.
    /// </summary>
    public static IReadOnlyList<string> TagsFor(EventClass eventClass)
    {
        if (!_tags.TryGetValue(eventClass, out IReadOnlyList<string>? tags))
        {
            throw new ArgumentOutOfRangeException(nameof(eventClass), eventClass,
                $"No tag attachments registered for EventClass.{eventClass}.");
        }
        return tags;
    }
}

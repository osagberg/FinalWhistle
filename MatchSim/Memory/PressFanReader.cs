using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Memory;

/// <summary>
/// Phase-3 reader implementation per ADR-0004 §"Five readers" / Reader 5
/// — press / fan callbacks. Filters the underlying <see cref="Ledger"/>
/// by:
/// <list type="bullet">
///   <item><description>Tag = <see cref="CallbackTagRegistry.PressFanId"/>
///       (event class must be registered with <c>press-fan</c> tag).</description></item>
///   <item><description>Salience band &gt;= <see cref="ReaderQuery.MinBand"/>
///       (default Notable per <see cref="CallbackTagRegistry.PressFan"/>).</description></item>
///   <item><description>Season window <c>[FromSeason, ToSeason]</c>
///       inclusive.</description></item>
/// </list>
/// Returns <see cref="CallbackCandidate"/> values sorted by surfacing
/// salience descending; up to <see cref="MaxResults"/> entries.
///
/// <para>
/// <strong>Surfacing salience</strong> is the persisted
/// <see cref="MemoryEvent.Salience"/> with
/// <see cref="SalienceEngine.ApplyCallbackAgeModifier"/> applied
/// reader-side per ADR-0004 §"Reader `callback_age` modifier" — never
/// persisted; recomputed per query.
/// </para>
///
/// <para>
/// <strong>Phase-3 minimum scope</strong>: Phase-3 has exactly one
/// template per event class. The <see cref="EventClass.GoalScored"/>
/// → <c>fwh.core:callback_template.goal_press_fan_milestone</c> mapping
/// is hard-coded; Phase-4+ swaps in a per-reader template-family
/// selector with prior-event matching for the boyhood-club / answers-
/// heartbreak / first-in-N-months flavors per the narrative-director's
/// Phase-3 authoring pass.
/// </para>
/// </summary>
public sealed class PressFanReader : IMemoryReader
{
    public const int DefaultMaxResults = 5;
    public const int DefaultSeasonWindow = 3;

    /// <summary>
    /// Phase-3 default template ID per the narrative-director's
    /// authoring pass. Boyhood-club flavor surfaced as the most legible
    /// memory-callback for the Month-3 stranger-watching-3-minutes
    /// rubric; multi-template selection is Phase-4+.
    /// </summary>
    public const string GoalScoredTemplateId =
        "fwh.core:callback_template.goal_press_fan_milestone";

    private readonly Ledger _ledger;

    public PressFanReader(Ledger ledger, int maxResults = DefaultMaxResults)
    {
        if (ledger is null) throw new ArgumentNullException(nameof(ledger));
        if (maxResults < 1)
        {
            throw new ArgumentOutOfRangeException(
                nameof(maxResults), maxResults, "MaxResults must be at least 1.");
        }
        _ledger = ledger;
        MaxResults = maxResults;
    }

    public int MaxResults { get; }

    public ReaderId Id => CallbackTagRegistry.PressFanReaderId;

    /// <summary>
    /// Convenience overload: build a default <see cref="ReaderQuery"/>
    /// for the press-fan tag centred on <paramref name="currentSeason"/>
    /// (window = <see cref="DefaultSeasonWindow"/> seasons back; band
    /// floor = press-fan tag's <see cref="CallbackTag.MinBand"/>).
    /// </summary>
    public IEnumerable<CallbackCandidate> QueryForSeason(ushort currentSeason)
    {
        ushort fromSeason = currentSeason >= DefaultSeasonWindow
            ? (ushort)(currentSeason - DefaultSeasonWindow)
            : (ushort)0;
        ReaderQuery query = new(
            tagId: CallbackTagRegistry.PressFanId,
            fromSeason: fromSeason,
            toSeason: currentSeason,
            currentSeason: currentSeason,
            minBand: CallbackTagRegistry.PressFan.MinBand);
        return Query(query);
    }

    public IEnumerable<CallbackCandidate> Query(ReaderQuery q)
    {
        // Filter into a temporary list so we can sort + truncate; the
        // ledger's All view stays untouched.
        List<CallbackCandidate> candidates = new();
        IReadOnlyList<MemoryEvent> source = _ledger.All;
        for (int i = 0; i < source.Count; i++)
        {
            MemoryEvent ev = source[i];

            // Tag-attachment filter: press-fan only fires on event
            // classes that register the press-fan tag in
            // EventClassRegistry.TagsFor.
            IReadOnlyList<string> tags = EventClassRegistry.TagsFor(ev.What);
            if (!ContainsTag(tags, q.TagId))
            {
                continue;
            }

            // Season-window filter (inclusive).
            if (ev.Season < q.FromSeason || ev.Season > q.ToSeason)
            {
                continue;
            }

            // Band floor filter — uses enum order: Routine=0 < Notable=1
            // < SeasonDefining=2. SeasonDefining events pass a Notable
            // floor (more salient events are always eligible for
            // lower-threshold readers).
            if (SalienceEngine.ClassifyBand(ev.Salience) < q.MinBand)
            {
                continue;
            }

            // Reader-side surfacing modifier — callback-age decay.
            // Use q.CurrentSeason (NOT q.ToSeason) per pr-review-toolkit
            // round-1 finding C1: the window upper bound is decoupled from
            // the in-world current season so historical-window queries
            // compute decay against the real present, not the window edge.
            Fixed surfacing = SalienceEngine.ApplyCallbackAgeModifier(
                ev.Salience, ev.Season, q.CurrentSeason);

            string template = TemplateFor(ev.What);
            candidates.Add(new CallbackCandidate(ev, surfacing, template));
        }

        // Sort by surfacing salience descending. Stable sort isn't
        // required — emission-order ties break by insertion-order which
        // is acceptable for Phase-3.
        candidates.Sort(static (a, b) => b.SurfacingSalience.CompareTo(a.SurfacingSalience));
        if (candidates.Count > MaxResults)
        {
            candidates.RemoveRange(MaxResults, candidates.Count - MaxResults);
        }
        return candidates;
    }

    private static bool ContainsTag(IReadOnlyList<string> tags, string tagId)
    {
        for (int i = 0; i < tags.Count; i++)
        {
            if (tags[i] == tagId) return true;
        }
        return false;
    }

    private static string TemplateFor(EventClass eventClass)
    {
        return eventClass switch
        {
            EventClass.GoalScored => GoalScoredTemplateId,
            // Phase-4+ event classes added here as they ship readers.
            _ => throw new ArgumentOutOfRangeException(
                nameof(eventClass), eventClass,
                $"PressFanReader has no Phase-3 template for EventClass.{eventClass}. " +
                "Phase-3 only registers GoalScored; expand TemplateFor as new event " +
                "classes attach the press-fan tag."),
        };
    }
}

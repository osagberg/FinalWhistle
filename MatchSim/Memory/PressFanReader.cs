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
        // Registry-boundary enforcement per Codex round-1 P1: the prior
        // implementation only checked tag-attachment on EventClass; it
        // never verified that the queried tag actually lists THIS reader
        // as a consumer, never enforced the tag's MinBand floor, and never
        // enforced the tag's Expiry. A buggy / malicious caller could
        // query ScoringMilestoneId via PressFanReader and silently get
        // press-fan templates back. Resolve the tag at entry, refuse
        // queries from non-consumer readers, and apply tag-level
        // invariants in addition to the query-level ones.
        if (!CallbackTagRegistry.TryGet(q.TagId, out CallbackTag tag))
        {
            throw new ArgumentException(
                $"Unknown tag id '{q.TagId}'. PressFanReader only accepts tags " +
                "in CallbackTagRegistry; query a registered tag.",
                nameof(q));
        }
        if (!TagListsThisReader(tag))
        {
            // ADR-0004 §"CallbackTag registry" mandates "every tag must
            // declare which readers may filter on it." Returning empty
            // would silently mask the contract violation; fail loud at
            // the registry boundary instead.
            throw new InvalidOperationException(
                $"PressFanReader (Id={Id}) is not a registered consumer of tag " +
                $"'{tag.Id}' (consumers: {string.Join(", ", TagConsumerIds(tag))}). " +
                "Construct a query whose TagId references a tag this reader is " +
                "registered for, or use the appropriate reader for this tag.");
        }

        // Effective MinBand is the stricter of the query's MinBand AND
        // the tag's MinBand floor — a caller can tighten the band but
        // cannot relax it past the tag's documented minimum.
        SalienceBand effectiveMinBand =
            q.MinBand >= tag.MinBand ? q.MinBand : tag.MinBand;

        // Filter into a temporary list. Capture the original ledger
        // index alongside each candidate so we can apply a deterministic
        // tiebreaker on equal surfacing salience (closes round-1 P2:
        // List<T>.Sort is not stable, and Phase-3 placeholder values
        // produce many same-salience candidates).
        List<(CallbackCandidate Candidate, int LedgerIndex)> filtered = new();
        IReadOnlyList<MemoryEvent> source = _ledger.All;
        for (int i = 0; i < source.Count; i++)
        {
            MemoryEvent ev = source[i];

            // Tag-attachment filter: only events whose EventClass is
            // registered with q.TagId in EventClassRegistry are eligible.
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

            // Tag-expiry enforcement per Codex round-1 P1.
            if (IsTagExpiredForEvent(tag.Expiry, ev, q.CurrentSeason))
            {
                continue;
            }

            // Effective band floor: max(query, tag) per Codex round-1 P1.
            if (SalienceEngine.ClassifyBand(ev.Salience) < effectiveMinBand)
            {
                continue;
            }

            // Reader-side surfacing modifier — callback-age decay.
            // q.CurrentSeason (NOT q.ToSeason) per pr-review-toolkit
            // round-1 finding C1.
            Fixed surfacing = SalienceEngine.ApplyCallbackAgeModifier(
                ev.Salience, ev.Season, q.CurrentSeason);

            string template = TemplateFor(ev.What);
            filtered.Add((new CallbackCandidate(ev, surfacing, template), i));
        }

        // Sort by surfacing salience descending; deterministic tiebreaker
        // by ascending ledger insertion index per Codex round-1 P2.
        // List<T>.Sort is not stable; an explicit secondary key keeps
        // ordering reproducible across platforms / .NET versions / sort-
        // impl changes — important because Phase-3 GoalScored events all
        // share the same placeholder salience scalar.
        filtered.Sort(static (a, b) =>
        {
            int byScore = b.Candidate.SurfacingSalience.CompareTo(a.Candidate.SurfacingSalience);
            return byScore != 0 ? byScore : a.LedgerIndex.CompareTo(b.LedgerIndex);
        });

        int resultCount = filtered.Count > MaxResults ? MaxResults : filtered.Count;
        CallbackCandidate[] result = new CallbackCandidate[resultCount];
        for (int i = 0; i < resultCount; i++)
        {
            result[i] = filtered[i].Candidate;
        }
        return result;
    }

    private bool TagListsThisReader(CallbackTag tag)
    {
        for (int i = 0; i < tag.ConsumingReaders.Count; i++)
        {
            if (tag.ConsumingReaders[i].Equals(Id)) return true;
        }
        return false;
    }

    private static IEnumerable<string> TagConsumerIds(CallbackTag tag)
    {
        for (int i = 0; i < tag.ConsumingReaders.Count; i++)
        {
            yield return tag.ConsumingReaders[i].Value;
        }
    }

    /// <summary>
    /// Per ADR-0004 <see cref="ExpiryPolicy"/> semantics. <c>Never</c>:
    /// always eligible. <c>Seasons(N)</c>: eligible for N seasons after
    /// emission (event expired if <c>currentSeason &gt; eventSeason + N</c>).
    /// <c>OnEvent</c>: Phase-4+ — would require scanning the ledger for
    /// the trigger event class between event.Season and currentSeason.
    /// Phase-3 throws <see cref="System.NotSupportedException"/> to fail
    /// loud rather than silently passing "expired" events through.
    /// </summary>
    private static bool IsTagExpiredForEvent(
        ExpiryPolicy expiry, MemoryEvent ev, ushort currentSeason)
    {
        switch (expiry)
        {
            case ExpiryPolicy.Never:
                return false;
            case ExpiryPolicy.Seasons seasons:
                return currentSeason > ev.Season + seasons.Count;
            case ExpiryPolicy.OnEvent onEvent:
                throw new System.NotSupportedException(
                    $"ExpiryPolicy.OnEvent({onEvent.TriggerClass}) enforcement is " +
                    "Phase-4+ — would require ledger scan for the trigger event class " +
                    "between event.Season and currentSeason. No Phase-3 tag uses this " +
                    "policy; fail loud if a content pack adds one before the Phase-4 " +
                    "implementation lands.");
            default:
                throw new System.InvalidOperationException(
                    $"Unknown ExpiryPolicy subtype {expiry?.GetType().FullName ?? "<null>"}.");
        }
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

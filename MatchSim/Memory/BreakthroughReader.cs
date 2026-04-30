using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Memory;

// Phase-4+ refactor anchor (per pr-review-toolkit:type-design-analyzer
// 2026-04-30 finding #2): once the 5-reader matrix from ADR-0004
// §"Five readers" lands (Alumni DB / Rival recall / Promise tracking /
// Big-match scars / Press-fan + this Breakthrough makes 6 once Phase-4+
// adds SignatureAwakened lifecycle), extract a `ReaderBase` capturing the
// shared registry-boundary enforcement + stable-sort tiebreaker +
// callback-age-decay surface. The duplicated private helpers
// (TagListsThisReader / IsTagExpiredForEvent / ContainsTag) + the Query
// body skeleton are the obvious candidates. Extracting at n=2 readers
// would be premature; the right factoring needs n>=5 visible.

/// <summary>
/// Phase-3 minimum reader implementation per SPEC line 145 — surfaces
/// <see cref="EventClass.SignatureBreakthrough"/> events tagged with
/// <see cref="CallbackTagRegistry.SignatureBreakthroughId"/> as
/// <see cref="CallbackCandidate"/>s for the Phase-3 persistent-
/// development-event surface (the eventual breakthrough-cinema panel
/// beat per <c>design/breakthrough-moments.md</c>).
///
/// <para>
/// Architecturally a sister to <see cref="PressFanReader"/>: same
/// registry-boundary enforcement (consumer-check + tag's MinBand floor
/// + Expiry), same stable-sort tiebreaker (ledger insertion index for
/// equal-salience events), same callback-age-decay reader-side
/// modifier. Differs in the tag it serves (signature-breakthrough vs
/// press-fan) and the template ID it resolves to.
/// </para>
///
/// <para>
/// <strong>Phase-3 minimum scope</strong>: one template per event class
/// (<see cref="EventClass.SignatureBreakthrough"/> →
/// <see cref="BreakthroughTemplateId"/>). Phase-4+ swaps in a per-
/// reader template family with prior-event matching for the
/// breakthrough-cinema two-tier text pattern (Tier 1 quiet observational
/// phrase / Tier 2 match-specific follow-up per
/// <c>design/breakthrough-moments.md</c> §Q2).
/// </para>
/// </summary>
public sealed class BreakthroughReader : IMemoryReader
{
    public const int DefaultMaxResults = 5;
    public const int DefaultSeasonWindow = 3;

    /// <summary>
    /// Phase-3 default template ID per <c>design/breakthrough-moments.md</c>
    /// §"Trigger kinds" Kind 1: a <c>player-isolation</c> shot →
    /// <c>aftermath-freeze</c> overlay sequence per ADR-0008 §7-shot
    /// vocabulary. The renderer (Phase-3 Viewer.EventBridge) resolves
    /// this template ID to the actual cinema beat composition.
    /// Multi-template selection is Phase-4+.
    /// </summary>
    public const string BreakthroughTemplateId =
        "fwh.core:callback_template.signature_breakthrough_panel";

    private readonly Ledger _ledger;

    public BreakthroughReader(Ledger ledger, int maxResults = DefaultMaxResults)
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

    public ReaderId Id => CallbackTagRegistry.BreakthroughReaderId;

    /// <summary>
    /// Convenience overload: build a default <see cref="ReaderQuery"/>
    /// for the signature-breakthrough tag centred on
    /// <paramref name="currentSeason"/>. Uses <see cref="DefaultSeasonWindow"/>
    /// + the tag's <see cref="CallbackTag.MinBand"/> floor.
    /// </summary>
    public IEnumerable<CallbackCandidate> QueryForSeason(ushort currentSeason)
    {
        ushort fromSeason = currentSeason >= DefaultSeasonWindow
            ? (ushort)(currentSeason - DefaultSeasonWindow)
            : (ushort)0;
        ReaderQuery query = new(
            tagId: CallbackTagRegistry.SignatureBreakthroughId,
            fromSeason: fromSeason,
            toSeason: currentSeason,
            currentSeason: currentSeason,
            minBand: CallbackTagRegistry.SignatureBreakthrough.MinBand);
        return Query(query);
    }

    public IEnumerable<CallbackCandidate> Query(ReaderQuery q)
    {
        // Registry-boundary enforcement (same posture as PressFanReader
        // post Codex round-1 P1).
        if (!CallbackTagRegistry.TryGet(q.TagId, out CallbackTag tag))
        {
            throw new ArgumentException(
                $"Unknown tag id '{q.TagId}'. BreakthroughReader only accepts tags " +
                "in CallbackTagRegistry.",
                nameof(q));
        }
        if (!TagListsThisReader(tag))
        {
            throw new InvalidOperationException(
                $"BreakthroughReader (Id={Id}) is not a registered consumer of tag " +
                $"'{tag.Id}' (consumers: {string.Join(", ", TagConsumerIds(tag))}).");
        }

        SalienceBand effectiveMinBand =
            q.MinBand >= tag.MinBand ? q.MinBand : tag.MinBand;

        List<(CallbackCandidate Candidate, int LedgerIndex)> filtered = new();
        IReadOnlyList<MemoryEvent> source = _ledger.All;
        for (int i = 0; i < source.Count; i++)
        {
            MemoryEvent ev = source[i];

            IReadOnlyList<string> tags = EventClassRegistry.TagsFor(ev.What);
            if (!ContainsTag(tags, q.TagId))
            {
                continue;
            }
            if (ev.Season < q.FromSeason || ev.Season > q.ToSeason)
            {
                continue;
            }
            if (IsTagExpiredForEvent(tag.Expiry, ev, q.CurrentSeason))
            {
                continue;
            }
            if (SalienceEngine.ClassifyBand(ev.Salience) < effectiveMinBand)
            {
                continue;
            }

            Fixed surfacing = SalienceEngine.ApplyCallbackAgeModifier(
                ev.Salience, ev.Season, q.CurrentSeason);
            string template = TemplateFor(ev.What);
            filtered.Add((new CallbackCandidate(ev, surfacing, template), i));
        }

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
                    "Phase-4+.");
            default:
                throw new InvalidOperationException(
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
            EventClass.SignatureBreakthrough => BreakthroughTemplateId,
            _ => throw new ArgumentOutOfRangeException(
                nameof(eventClass), eventClass,
                $"BreakthroughReader has no Phase-3 template for EventClass.{eventClass}."),
        };
    }
}

using System.Collections.Generic;
using FinalWhistle.MatchSim.Memory.Contracts;

namespace FinalWhistle.MatchSim.Memory;

/// <summary>
/// Static <see cref="CallbackTag"/> registry per ADR-0004 §"`CallbackTag`
/// registry." Phase-3 starter set — only the <see cref="PressFan"/> tag
/// has a Phase-3 reader (<c>PressFanReader</c>); the
/// <see cref="ScoringMilestone"/> tag is registered with a placeholder
/// reader-id so it passes the validator's <c>ConsumingReaders.Length &gt;= 1</c>
/// rule, but no Phase-3 reader filters on it (Phase-4+ adds the
/// scoring-milestone reader from the design/event-sourced-memory.md
/// §Five-readers list).
///
/// <para>
/// Per the narrative-director Phase-3 callback authoring pass:
/// <c>press-fan</c> tag uses <see cref="SalienceBand.Notable"/> floor
/// (every-routine-event press-coverage is noise) + <c>Seasons(3)</c>
/// expiry (matches typical British football press-narrative carry).
/// SeasonDefining-band events warrant a 5-season override per the
/// narrative-director's Phase-4 sharpening note — deferred for now.
/// </para>
/// </summary>
public static class CallbackTagRegistry
{
    public const string PressFanId = "fwh.core:tag.press-fan";
    public const string ScoringMilestoneId = "fwh.core:tag.scoring-milestone";

    /// <summary>
    /// Phase-3 minimum persistent-development tag per SPEC line 145.
    /// Consumed by <see cref="BreakthroughReader"/>. Expiry =
    /// <see cref="ExpiryPolicy.Never"/> per
    /// <c>design/breakthrough-moments.md</c> "Permanent. Awakenings are
    /// irreversible." MinBand = Notable: the salience formula's max
    /// achievable value with rivalry+rarity=0 is 0.80 (under
    /// SeasonDefining's 0.85 cutoff), so Phase-3 breakthroughs naturally
    /// land in Notable band even at maxed Stakes/Prominence/ClassWeight.
    /// Phase-4+ rivalry-boost / rarity-boost wiring lifts breakthroughs
    /// to SeasonDefining where appropriate; the tag-level expiry-Never
    /// + reader-level consumer-check enforce the "permanent player-
    /// development" semantic regardless of band.
    /// </summary>
    public const string SignatureBreakthroughId = "fwh.core:tag.signature-breakthrough";

    public static readonly ReaderId PressFanReaderId = new("press-fan-reader");
    public static readonly ReaderId ScoringMilestoneReaderId = new("scoring-milestone-reader");
    public static readonly ReaderId BreakthroughReaderId = new("breakthrough-reader");

    public static readonly CallbackTag PressFan;
    public static readonly CallbackTag ScoringMilestone;
    public static readonly CallbackTag SignatureBreakthrough;

    private static readonly Dictionary<string, CallbackTag> _byId;

    static CallbackTagRegistry()
    {
        // Constructor-validating per pr-review-toolkit round-1 finding 2;
        // no opt-in Validate() call needed.
        PressFan = new CallbackTag(
            id: PressFanId,
            consumingReaders: new[] { PressFanReaderId },
            minBand: SalienceBand.Notable,
            expiry: new ExpiryPolicy.Seasons(3));

        ScoringMilestone = new CallbackTag(
            id: ScoringMilestoneId,
            consumingReaders: new[] { ScoringMilestoneReaderId },
            minBand: SalienceBand.Notable,
            expiry: ExpiryPolicy.Never.Instance);

        SignatureBreakthrough = new CallbackTag(
            id: SignatureBreakthroughId,
            consumingReaders: new[] { BreakthroughReaderId },
            minBand: SalienceBand.Notable,
            expiry: ExpiryPolicy.Never.Instance);

        _byId = new Dictionary<string, CallbackTag>
        {
            [PressFan.Id] = PressFan,
            [ScoringMilestone.Id] = ScoringMilestone,
            [SignatureBreakthrough.Id] = SignatureBreakthrough,
        };
    }

    /// <summary>All registered tags (read-only view).</summary>
    public static IReadOnlyCollection<CallbackTag> All => _byId.Values;

    public static CallbackTag Get(string tagId)
    {
        if (!_byId.TryGetValue(tagId, out CallbackTag? tag))
        {
            // Derive the registered-IDs list from _byId so the diagnostic
            // stays in sync as new tags land (closes pr-review-toolkit:
            // feature-dev:code-reviewer 2026-04-30 finding #2 — prior
            // hardcoded "PressFanId, ScoringMilestoneId" string was stale
            // after the round-2 SignatureBreakthroughId addition).
            throw new KeyNotFoundException(
                $"Unknown callback-tag ID: '{tagId}'. Registry contains: " +
                $"{string.Join(", ", _byId.Keys)}.");
        }
        return tag;
    }

    public static bool TryGet(string tagId, out CallbackTag tag)
    {
        if (_byId.TryGetValue(tagId, out CallbackTag? found))
        {
            tag = found;
            return true;
        }
        tag = default!;
        return false;
    }
}

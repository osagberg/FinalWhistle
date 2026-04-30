using System;

namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Filter parameters for <see cref="IMemoryReader.Query"/>. Specifies a
/// <see cref="CallbackTag"/> ID + season window (inclusive) + minimum
/// salience band + the in-world <see cref="CurrentSeason"/> against
/// which reader-side modifiers (callback-age decay) are computed.
/// Phase-3 readers all use simple tag-band-season filtering; Phase-4+
/// may add per-reader fields (alumni-DB needs "former club" filter,
/// promise-tracking needs participant-id).
///
/// <para>
/// <strong><see cref="CurrentSeason"/></strong> per pr-review-toolkit
/// 2026-04-30 round-1 finding C1 — kept distinct from
/// <see cref="ToSeason"/> so historical-window queries (e.g.
/// <c>fromSeason=1, toSeason=3</c> while currently in season 8) compute
/// callback-age against the real in-world season, not the window upper
/// bound. Conflating the two silently capped decay age at the window
/// width.
/// </para>
/// </summary>
public readonly struct ReaderQuery
{
    public string TagId { get; }
    public ushort FromSeason { get; }
    public ushort ToSeason { get; }

    /// <summary>
    /// In-world current season. Reader-side modifiers (callback-age
    /// decay) compute <c>age = CurrentSeason - event.Season</c>. Must
    /// be &gt;= <see cref="ToSeason"/> (you cannot query a future-of-now
    /// window without driving the comparison through pre-emptive
    /// authoring; <see cref="SalienceEngine.ApplyCallbackAgeModifier"/>
    /// handles the <c>currentSeason &lt; eventSeason</c> case
    /// gracefully, but a query whose <c>CurrentSeason &lt; ToSeason</c>
    /// is a structural authoring bug — the window claims to extend past
    /// the present, which has no in-game meaning).
    /// </summary>
    public ushort CurrentSeason { get; }

    public SalienceBand MinBand { get; }

    public ReaderQuery(
        string tagId, ushort fromSeason, ushort toSeason,
        ushort currentSeason, SalienceBand minBand)
    {
        if (string.IsNullOrEmpty(tagId))
        {
            throw new ArgumentException(
                "TagId must be non-empty.", nameof(tagId));
        }
        if (fromSeason > toSeason)
        {
            throw new ArgumentException(
                $"FromSeason ({fromSeason}) must not exceed ToSeason ({toSeason}).",
                nameof(fromSeason));
        }
        if (currentSeason < toSeason)
        {
            throw new ArgumentException(
                $"CurrentSeason ({currentSeason}) must not be less than ToSeason " +
                $"({toSeason}); the query window cannot extend past the present.",
                nameof(currentSeason));
        }
        TagId = tagId;
        FromSeason = fromSeason;
        ToSeason = toSeason;
        CurrentSeason = currentSeason;
        MinBand = minBand;
    }
}

using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;

namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Tag expiry policy per ADR-0004 §"`CallbackTag` registry" — a closed
/// discriminated union expressed as a sealed abstract base + 3 concrete
/// nested types. Pattern matches existing MatchSim discriminated-union
/// usage (see <c>KeyEvent</c> + <c>TeamSide</c> patterns) without
/// requiring an external union library.
/// </summary>
public abstract class ExpiryPolicy
{
    private ExpiryPolicy() { }

    /// <summary>Tag never expires; eligible forever.</summary>
    public sealed class Never : ExpiryPolicy
    {
        public static readonly Never Instance = new();
        private Never() { }
        public override string ToString() => "Never";
    }

    /// <summary>Tag expires after <see cref="Count"/> seasons.</summary>
    public sealed class Seasons : ExpiryPolicy
    {
        public byte Count { get; }
        public Seasons(byte count)
        {
            if (count == 0)
            {
                throw new ArgumentOutOfRangeException(nameof(count), count,
                    "ExpiryPolicy.Seasons.Count must be at least 1; use ExpiryPolicy.Never for forever.");
            }
            Count = count;
        }
        public override string ToString() => $"Seasons({Count})";
    }

    /// <summary>Tag expires the moment the trigger event class fires.</summary>
    public sealed class OnEvent : ExpiryPolicy
    {
        public EventClass TriggerClass { get; }
        public OnEvent(EventClass triggerClass)
        {
            if (triggerClass == EventClass.None)
            {
                throw new ArgumentException(
                    "OnEvent trigger cannot be EventClass.None.", nameof(triggerClass));
            }
            TriggerClass = triggerClass;
        }
        public override string ToString() => $"OnEvent({TriggerClass})";
    }
}

/// <summary>
/// Callback-tag registry record per ADR-0004 §"`CallbackTag` registry."
/// Constructor-enforced invariants (per pr-review-toolkit:type-design-analyzer
/// round-1 finding 2 — close the opt-in <c>Validate()</c> footgun):
/// <list type="bullet">
///   <item><description><see cref="Id"/> is content-pack-qualified per
///       ADR-0006 ID format (<c>fwh.core:tag.&lt;slug&gt;</c>); non-empty.</description></item>
///   <item><description><see cref="ConsumingReaders"/> length &gt;= 1 —
///       tags without consumers accumulate; the lint catches them
///       at content-pack compile time.</description></item>
///   <item><description><see cref="Expiry"/> is non-null.</description></item>
/// </list>
/// Sealed class with parameterized constructor — record syntax + <c>init</c>
/// setters were the prior shape but allowed
/// <c>new CallbackTag { Id = "" }</c> to bypass validation. Constructor
/// pattern matches the rest of the Memory.Contracts directory.
/// </summary>
public sealed class CallbackTag
{
    public string Id { get; }

    /// <summary>
    /// Read-only list of consuming reader IDs. Backed by a defensive
    /// <see cref="ReaderId"/>[] copy at construction; <see cref="IReadOnlyList{T}"/>
    /// chosen over <c>ImmutableArray</c> to avoid pulling
    /// <c>System.Collections.Immutable</c> as a NuGet dep — Unity 6's
    /// Mono runtime doesn't ship it inboxed, same constraint that drove
    /// the Codex round-7 STJ-removal refactor on
    /// <c>IdentityPacketParser</c>.
    /// </summary>
    public IReadOnlyList<ReaderId> ConsumingReaders { get; }

    public SalienceBand MinBand { get; }
    public ExpiryPolicy Expiry { get; }

    public CallbackTag(
        string id,
        IReadOnlyList<ReaderId> consumingReaders,
        SalienceBand minBand,
        ExpiryPolicy expiry)
    {
        if (string.IsNullOrEmpty(id))
        {
            throw new ArgumentException("CallbackTag.Id must be non-empty.", nameof(id));
        }
        if (consumingReaders is null)
        {
            throw new ArgumentNullException(nameof(consumingReaders));
        }
        if (consumingReaders.Count == 0)
        {
            throw new ArgumentException(
                $"CallbackTag {id} has no consuming readers. Every tag must declare " +
                "at least one reader that may filter on it (per ADR-0004 §CallbackTag registry).",
                nameof(consumingReaders));
        }
        if (expiry is null)
        {
            throw new ArgumentNullException(nameof(expiry));
        }

        Id = id;
        // Defensive copy + ReadOnlyCollection wrap per pr-review-toolkit
        // round-1 P2: a raw T[] cast back to ReaderId[] would let any
        // consumer mutate registry tags after static-init. ReadOnlyCollection
        // forbids the cast-back path.
        ReaderId[] copy = new ReaderId[consumingReaders.Count];
        for (int i = 0; i < consumingReaders.Count; i++)
        {
            copy[i] = consumingReaders[i];
        }
        ConsumingReaders = new ReadOnlyCollection<ReaderId>(copy);
        MinBand = minBand;
        Expiry = expiry;
    }
}

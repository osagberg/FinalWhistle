using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// Memory-callback descriptor attached to a <see cref="ViewerEvent"/>
    /// per ADR-0008 §"ViewerEvent schema" / <see cref="MemoryHit"/>.
    /// Carries a callback-line ID (locale-resolved at adapter render time)
    /// + structured slot values that the adapter slot-fills before
    /// presenting. Phase-3 minimum: <see cref="EventBridge"/> derives
    /// MemoryHits from the
    /// <see cref="FinalWhistle.MatchSim.Memory.IMemoryReader"/> output of
    /// the relevant memory readers; bridge-side prose generation is
    /// forbidden — only deterministic slot lookup.
    ///
    /// <para>
    /// <strong>Determinism contract</strong>: <see cref="Slots"/> are
    /// sorted ordinal-ascending by <see cref="CallbackSlotValue.SlotName"/>
    /// at bridge emission; adapters never re-sort. <see cref="CallbackLineId"/>
    /// is included in the pass-activation trace; the rendered prose is
    /// excluded (per ADR-0008 §"Pass-activation trace fields") because
    /// locale-rendered text drifts across builds.
    /// </para>
    /// </summary>
    public readonly struct MemoryHit : IEquatable<MemoryHit>
    {
        /// <summary>
        /// Content-pack-qualified ID of the participating entity (player,
        /// club, etc.) the callback is about.
        /// </summary>
        public string ParticipantId { get; }

        /// <summary>
        /// ADR-0004 callback-tag ID per
        /// <c>FinalWhistle.MatchSim.Memory.CallbackTagRegistry</c>. Stable
        /// across runs.
        /// </summary>
        public string Tag { get; }

        /// <summary>Q32.32 surfacing salience in <c>[0, 1]</c>.</summary>
        public Fixed Salience { get; }

        /// <summary>
        /// Localized + lint-scanned line asset ID per ADR-0008
        /// §"ViewerEvent schema." Adapter resolves to localized text at
        /// render time; bridge never bakes in locale-specific bytes.
        /// </summary>
        public string CallbackLineId { get; }

        /// <summary>
        /// Deterministic slot values for the callback line. Backed by a
        /// defensive <see cref="ReadOnlyCollection{T}"/> wrap (per slice-#3
        /// round-1 P2 cast-back-prevention pattern); bridge sorts
        /// ordinal-ascending by <see cref="CallbackSlotValue.SlotName"/>
        /// before storing.
        /// </summary>
        public IReadOnlyList<CallbackSlotValue> Slots { get; }

        public MemoryHit(
            string participantId,
            string tag,
            Fixed salience,
            string callbackLineId,
            IReadOnlyList<CallbackSlotValue> slots)
        {
            if (string.IsNullOrEmpty(participantId))
            {
                throw new ArgumentException("ParticipantId must be non-empty.", nameof(participantId));
            }
            if (string.IsNullOrEmpty(tag))
            {
                throw new ArgumentException("Tag must be non-empty.", nameof(tag));
            }
            if (salience < Fixed.Zero || salience > Fixed.One)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(salience), salience, "Salience must lie in [0, 1].");
            }
            if (string.IsNullOrEmpty(callbackLineId))
            {
                throw new ArgumentException("CallbackLineId must be non-empty.", nameof(callbackLineId));
            }
            if (slots is null)
            {
                throw new ArgumentNullException(nameof(slots),
                    "Slots must not be null. Use Array.Empty<CallbackSlotValue>() for hits without slots.");
            }

            ParticipantId = participantId;
            Tag = tag;
            Salience = salience;
            CallbackLineId = callbackLineId;

            // Defensive copy + ordinal-ascending sort by SlotName per
            // ADR-0008 §"Determinism contract" so the adapter never has
            // to re-sort + a future authoring drift in the caller's
            // ordering doesn't affect the pass-activation trace bytes.
            CallbackSlotValue[] sortedCopy = new CallbackSlotValue[slots.Count];
            for (int i = 0; i < slots.Count; i++)
            {
                sortedCopy[i] = slots[i];
            }
            Array.Sort(sortedCopy, static (a, b) =>
                string.CompareOrdinal(a.SlotName, b.SlotName));
            Slots = new ReadOnlyCollection<CallbackSlotValue>(sortedCopy);
        }

        public bool Equals(MemoryHit other)
        {
            if (ParticipantId != other.ParticipantId
                || Tag != other.Tag
                || Salience != other.Salience
                || CallbackLineId != other.CallbackLineId
                || Slots.Count != other.Slots.Count)
            {
                return false;
            }
            for (int i = 0; i < Slots.Count; i++)
            {
                if (!Slots[i].Equals(other.Slots[i])) return false;
            }
            return true;
        }

        public override bool Equals(object? obj) => obj is MemoryHit other && Equals(other);
        public override int GetHashCode() => HashCode.Combine(ParticipantId, Tag, Salience, CallbackLineId, Slots.Count);
        public static bool operator ==(MemoryHit left, MemoryHit right) => left.Equals(right);
        public static bool operator !=(MemoryHit left, MemoryHit right) => !left.Equals(right);
    }
}

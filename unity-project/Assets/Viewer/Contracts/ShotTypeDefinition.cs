using System;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// Pure-C# DTO projection of an ADR-0001 <c>ShotTypeSO</c> per ADR-0008
    /// §"ShotTypeDefinition." Renderer-agnostic — adapters consume the
    /// <see cref="Id"/> and <see cref="Category"/> + the
    /// <see cref="DurationTicks"/> envelope to render their adapter-specific
    /// presentation. Phase-4+ projection from real <c>ShotTypeSO</c>
    /// ScriptableObject assets adds <c>FramingParams</c> /
    /// <c>ChainRules</c> / <c>OverlayTemplates</c> per ADR-0008's full
    /// schema; Phase-3 minimum is the fields below + a hard-coded
    /// <see cref="ShotTypeCatalog"/> in <c>Viewer.Core</c>.
    ///
    /// <para>
    /// <strong>Determinism floor:</strong> all numeric fields are
    /// integer / Q32.32; no <see cref="double"/>, no UnityEngine refs.
    /// Same <see cref="Id"/> + same effect on a viewer event = same
    /// pass-activation trace bytes per ADR-0008 §"Determinism contract."
    /// </para>
    /// </summary>
    public sealed class ShotTypeDefinition : IEquatable<ShotTypeDefinition>
    {
        /// <summary>
        /// Content-pack-qualified identifier per ADR-0006 ID format
        /// (<c>fwh.core:shot.&lt;slug&gt;</c>). Stable across runs +
        /// platforms; ordinal-comparable.
        /// </summary>
        public string Id { get; }

        public ShotCategory Category { get; }

        /// <summary>
        /// Default tick-duration envelope from <see cref="ViewerEvent.StartTick"/>
        /// to <see cref="ViewerEvent.EndTick"/> at canonical 60 Hz. Phase-3
        /// minimum: pinned per category (see <c>ShotTypeCatalog</c>).
        /// Phase-4+ ShotTypeSO authoring exposes per-asset duration override
        /// + chain-aware variable durations.
        /// </summary>
        public int DurationTicks { get; }

        /// <summary>
        /// Optional reduce-motion variant Id substituted by
        /// <c>EventBridge</c> at emission time per ADR-0008 §"Reduce-motion
        /// adapter-awareness." Null = no substitution (the shot already
        /// honors reduce-motion or has no motion-heavy presentation).
        /// </summary>
        public string? ReduceMotionVariantId { get; }

        public ShotTypeDefinition(
            string id,
            ShotCategory category,
            int durationTicks,
            string? reduceMotionVariantId = null)
        {
            if (string.IsNullOrEmpty(id))
            {
                throw new ArgumentException("ShotTypeDefinition.Id must be non-empty.", nameof(id));
            }
            if (category == ShotCategory.None)
            {
                throw new ArgumentException(
                    "ShotCategory.None is the sentinel; never construct a " +
                    "ShotTypeDefinition with it.",
                    nameof(category));
            }
            if (durationTicks <= 0)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(durationTicks), durationTicks,
                    "DurationTicks must be strictly positive (the shot has " +
                    "non-zero on-screen presence).");
            }
            // 5 seconds at 60Hz = 300 ticks. design/breakthrough-moments.md
            // §Q1 locks the cinema-beat range at 3-5s; the shot envelope
            // upper bound matches that band so a future authoring drift
            // (e.g., 8s on a routine shot) trips at construction.
            if (durationTicks > Tick.TicksPerSecond * 5)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(durationTicks), durationTicks,
                    $"DurationTicks must not exceed {Tick.TicksPerSecond * 5} " +
                    "(5 seconds at 60Hz canonical) per design/breakthrough-moments.md " +
                    "§Q1 cinema-beat duration upper bound. 8s+ shots dropped from " +
                    "consideration there; this guard prevents stealthy reintroduction.");
            }
            // ReduceMotionVariantId is optional but, when present, must be
            // non-empty (empty-string sentinel would silently disable
            // substitution at the bridge).
            if (reduceMotionVariantId is not null && reduceMotionVariantId.Length == 0)
            {
                throw new ArgumentException(
                    "ReduceMotionVariantId must be null OR non-empty; empty-string " +
                    "is rejected to keep the substitution flag unambiguous.",
                    nameof(reduceMotionVariantId));
            }
            Id = id;
            Category = category;
            DurationTicks = durationTicks;
            ReduceMotionVariantId = reduceMotionVariantId;
        }

        public bool Equals(ShotTypeDefinition? other)
        {
            if (other is null) return false;
            if (ReferenceEquals(this, other)) return true;
            return Id == other.Id
                && Category == other.Category
                && DurationTicks == other.DurationTicks
                && ReduceMotionVariantId == other.ReduceMotionVariantId;
        }

        public override bool Equals(object? obj) => obj is ShotTypeDefinition other && Equals(other);
        public override int GetHashCode() => HashCode.Combine(Id, (byte)Category, DurationTicks, ReduceMotionVariantId);
        public override string ToString() => $"ShotTypeDefinition({Id}, {Category}, {DurationTicks} ticks)";
    }
}

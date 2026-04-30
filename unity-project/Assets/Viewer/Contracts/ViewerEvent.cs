using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// Bridge-derived presentation event consumed by viewer adapters per
    /// ADR-0008 §"ViewerEvent schema." Same canonical sim event stream
    /// produces the same <see cref="ViewerEvent"/> stream byte-for-byte
    /// across platforms — adapters then render that stream in their
    /// own style (dots / 3D / future) per ADR-0008 §"Adapter selection."
    /// MatchSim never sees this type; it lives entirely in the
    /// <c>Viewer.Contracts</c> renderer-agnostic layer.
    ///
    /// <para>
    /// <strong>Phase-3 minimum scope</strong>: implements the v1 contract
    /// fields locked in ADR-0008 §"ViewerEvent schema" for
    /// <see cref="EventClass.GoalScored"/> +
    /// <see cref="EventClass.SignatureBreakthrough"/> + the three Phase-3
    /// signature-execution KeyEvents. The <c>PitchView</c> +
    /// <c>ActiveViewerEvent</c> + <c>IShotPresentationAdapter</c>
    /// runtime-rendering surfaces from ADR-0008 §"Adapter interface" land
    /// alongside the dots adapter (SPEC line 149 — Phase-3 next item).
    /// </para>
    ///
    /// <para>
    /// <strong>Determinism contract</strong>: stream order is
    /// <c>(StartTick ascending, ViewerEventId ascending)</c>;
    /// <see cref="ViewerEventId"/> is bridge-assigned monotonic per match;
    /// <see cref="ParticipantPlayerIds"/> + <see cref="MemoryHits"/> are
    /// frozen at construction via defensive
    /// <see cref="ReadOnlyCollection{T}"/> wraps. Adapters MUST iterate
    /// in supplied order; they MUST NOT re-sort.
    /// </para>
    /// </summary>
    public sealed class ViewerEvent : IEquatable<ViewerEvent>
    {
        /// <summary>
        /// Phase-3 minimum shot-duration floor — 30 ticks = 0.5 seconds at
        /// 60Hz canonical. Per pr-review-toolkit:type-design-analyzer
        /// 2026-04-30 finding #1: a 1-tick window would silently break
        /// downstream replay-corpus expectations + would never legitimately
        /// be produced by <see cref="ShotTypeCatalog"/>'s 180/240/300-tick
        /// envelopes. The lower bound mirrors
        /// <see cref="ShotTypeDefinition.DurationTicks"/>'s upper bound
        /// (5s = 300 ticks) so the cinema-beat range is symmetrically
        /// pinned at construction.
        /// </summary>
        public const int MinShotDurationTicks = 30;

        // ------------------------------------------------------------
        // Stable identity + ordering (ADR-0008 §"ViewerEvent schema")
        // ------------------------------------------------------------

        /// <summary>Bridge-assigned monotonic ID per match.</summary>
        public ulong ViewerEventId { get; }

        /// <summary>
        /// Source MatchSim canonical event index per ADR-0004. Phase-3
        /// uses the position into <c>MatchSimulationState.KeyEvents</c>
        /// because there's no ulong event-id surface yet; Phase-4+ adds a
        /// stable ulong event-id and this field carries it.
        ///
        /// <para>
        /// <strong>Note per pr-review-toolkit:feature-dev:code-reviewer
        /// 2026-04-30 finding #1</strong>: this value will be larger than
        /// <see cref="ViewerEventId"/> for events that follow a skipped
        /// restart KeyEvent in the canonical stream
        /// (<see cref="ViewerEventId"/> is monotonic + contiguous over
        /// emitted ViewerEvents; <see cref="SourceEventId"/> is the raw
        /// position into <c>MatchSimulationState.KeyEvents</c> including
        /// skipped restarts). Do NOT treat the two fields as
        /// equivalent — use <see cref="ViewerEventId"/> for ordering
        /// + counting, <see cref="SourceEventId"/> for tracing back to
        /// the underlying canonical KeyEvent.
        /// </para>
        /// </summary>
        public ulong SourceEventId { get; }

        /// <summary>
        /// Index of this <see cref="ViewerEvent"/> within the source
        /// event's emission burst (0 if the source event maps 1:1 to
        /// exactly one ViewerEvent).
        /// </summary>
        public int SourceEventOrdinal { get; }

        // ------------------------------------------------------------
        // Shot identity (ADR-0008 §"BaseShotTypeId / EffectiveShotTypeId")
        // ------------------------------------------------------------

        /// <summary>
        /// Pre-substitution shot identity per ADR-0001 ShotTypeSO.Id.
        /// Provenance only — visible in pass-activation trace + debug
        /// overlay; adapters do NOT render this directly.
        /// </summary>
        public string BaseShotTypeId { get; }

        /// <summary>
        /// Post-substitution shot identity. Adapters render THIS one.
        /// Equal to <see cref="BaseShotTypeId"/> when no reduce-motion
        /// substitution applied; otherwise the
        /// <see cref="ShotTypeDefinition.ReduceMotionVariantId"/> from the
        /// base shot.
        /// </summary>
        public string EffectiveShotTypeId { get; }

        /// <summary>
        /// True iff <see cref="EventBridge"/> substituted the shot's
        /// <see cref="ShotTypeDefinition.ReduceMotionVariantId"/>.
        /// Adapters consult this to drive their adapter-specific feature
        /// toggles per ADR-0008 §"Reduce-motion adapter-awareness."
        /// </summary>
        public bool ReduceMotionApplied { get; }

        // ------------------------------------------------------------
        // Temporal window (ADR-0008 §"Temporal window")
        // ------------------------------------------------------------

        public Tick StartTick { get; }
        public Tick EndTick { get; }
        public Seed Seed { get; }

        // ------------------------------------------------------------
        // Modulation parameters (ADR-0008 §"Modulation parameters")
        // ------------------------------------------------------------

        /// <summary>Q32.32 in <c>[0, 1]</c>.</summary>
        public Fixed StakesNormalized { get; }

        /// <summary>Q32.32 in <c>[0, 1]</c>.</summary>
        public Fixed MemoryRelevance { get; }

        /// <summary>
        /// Explicit focal player. Adapters use this for selection-ring +
        /// camera-target hints. Null if no single focal subject (e.g.
        /// <see cref="ShotCategory.TacticalWide"/>).
        /// </summary>
        public string? FocalSubject { get; }

        /// <summary>
        /// Player IDs participating in this event. Backed by a
        /// <see cref="ReadOnlyCollection{T}"/>; never default-init.
        /// </summary>
        public IReadOnlyList<string> ParticipantPlayerIds { get; }

        /// <summary>
        /// Structured callback hits the adapter may surface. Backed by a
        /// <see cref="ReadOnlyCollection{T}"/>; never default-init.
        /// </summary>
        public IReadOnlyList<MemoryHit> MemoryHits { get; }

        // ------------------------------------------------------------
        // Provenance (ADR-0008 §"Event provenance")
        // ------------------------------------------------------------

        public EventClass SourceEventClass { get; }

        /// <summary>
        /// If a player / signature triggered the event, the entity ID;
        /// null otherwise.
        /// </summary>
        public string? SourceEntityId { get; }

        public ViewerEvent(
            ulong viewerEventId,
            ulong sourceEventId,
            int sourceEventOrdinal,
            string baseShotTypeId,
            string effectiveShotTypeId,
            bool reduceMotionApplied,
            Tick startTick,
            Tick endTick,
            Seed seed,
            Fixed stakesNormalized,
            Fixed memoryRelevance,
            string? focalSubject,
            IReadOnlyList<string> participantPlayerIds,
            IReadOnlyList<MemoryHit> memoryHits,
            EventClass sourceEventClass,
            string? sourceEntityId)
        {
            if (sourceEventOrdinal < 0)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(sourceEventOrdinal), sourceEventOrdinal,
                    "SourceEventOrdinal must be non-negative.");
            }
            if (string.IsNullOrEmpty(baseShotTypeId))
            {
                throw new ArgumentException("BaseShotTypeId must be non-empty.", nameof(baseShotTypeId));
            }
            if (string.IsNullOrEmpty(effectiveShotTypeId))
            {
                throw new ArgumentException("EffectiveShotTypeId must be non-empty.", nameof(effectiveShotTypeId));
            }
            if (endTick.Value <= startTick.Value)
            {
                throw new ArgumentException(
                    $"EndTick ({endTick.Value}) must be strictly greater than StartTick ({startTick.Value}).",
                    nameof(endTick));
            }
            // Min-window guard per pr-review-toolkit:type-design-analyzer
            // 2026-04-30 finding #1: 1-tick events would silently break
            // downstream replay-corpus expectations.
            if (endTick.Value - startTick.Value < MinShotDurationTicks)
            {
                throw new ArgumentException(
                    $"ViewerEvent duration ({endTick.Value - startTick.Value} ticks) is below " +
                    $"MinShotDurationTicks ({MinShotDurationTicks} = 0.5s at 60Hz canonical). " +
                    "ShotTypeCatalog envelopes are 180/240/300 ticks; this guard catches a " +
                    "future bridge regression that desynchronizes EndTick from the shot definition.",
                    nameof(endTick));
            }
            if (stakesNormalized < Fixed.Zero || stakesNormalized > Fixed.One)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(stakesNormalized), stakesNormalized, "StakesNormalized must lie in [0, 1].");
            }
            if (memoryRelevance < Fixed.Zero || memoryRelevance > Fixed.One)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(memoryRelevance), memoryRelevance, "MemoryRelevance must lie in [0, 1].");
            }
            if (focalSubject is not null && focalSubject.Length == 0)
            {
                throw new ArgumentException(
                    "FocalSubject must be null OR non-empty.", nameof(focalSubject));
            }
            if (participantPlayerIds is null)
            {
                throw new ArgumentNullException(nameof(participantPlayerIds),
                    "Use Array.Empty<string>() for events without participant attribution.");
            }
            if (memoryHits is null)
            {
                throw new ArgumentNullException(nameof(memoryHits),
                    "Use Array.Empty<MemoryHit>() for events without memory hits.");
            }
            if (sourceEventClass == EventClass.None)
            {
                throw new ArgumentException(
                    "EventClass.None is the sentinel; never construct a ViewerEvent with it.",
                    nameof(sourceEventClass));
            }
            if (sourceEntityId is not null && sourceEntityId.Length == 0)
            {
                throw new ArgumentException(
                    "SourceEntityId must be null OR non-empty.", nameof(sourceEntityId));
            }
            // Reduce-motion-applied and the BaseShotTypeId vs EffectiveShotTypeId
            // pair must agree: if ReduceMotionApplied is true, the IDs must
            // differ (substitution actually happened); if false, they must
            // match (no substitution).
            bool idsDiffer = !string.Equals(baseShotTypeId, effectiveShotTypeId, StringComparison.Ordinal);
            if (reduceMotionApplied != idsDiffer)
            {
                throw new ArgumentException(
                    $"ReduceMotionApplied={reduceMotionApplied} contradicts the " +
                    $"BaseShotTypeId={baseShotTypeId} vs EffectiveShotTypeId={effectiveShotTypeId} " +
                    "comparison. ReduceMotionApplied must be true iff the IDs differ.",
                    nameof(reduceMotionApplied));
            }

            ViewerEventId = viewerEventId;
            SourceEventId = sourceEventId;
            SourceEventOrdinal = sourceEventOrdinal;
            BaseShotTypeId = baseShotTypeId;
            EffectiveShotTypeId = effectiveShotTypeId;
            ReduceMotionApplied = reduceMotionApplied;
            StartTick = startTick;
            EndTick = endTick;
            Seed = seed;
            StakesNormalized = stakesNormalized;
            MemoryRelevance = memoryRelevance;
            FocalSubject = focalSubject;
            // Defensive copy + ReadOnlyCollection wrap per slice-#3 round-1
            // P2 cast-back-prevention pattern: a raw T[] cast back to
            // string[] / MemoryHit[] would let any consumer mutate the
            // event after emission.
            string[] participantsCopy = new string[participantPlayerIds.Count];
            for (int i = 0; i < participantPlayerIds.Count; i++)
            {
                if (string.IsNullOrEmpty(participantPlayerIds[i]))
                {
                    throw new ArgumentException(
                        $"ParticipantPlayerIds[{i}] is null or empty.",
                        nameof(participantPlayerIds));
                }
                participantsCopy[i] = participantPlayerIds[i];
            }
            ParticipantPlayerIds = new ReadOnlyCollection<string>(participantsCopy);
            MemoryHit[] memoryHitsCopy = new MemoryHit[memoryHits.Count];
            for (int i = 0; i < memoryHits.Count; i++)
            {
                memoryHitsCopy[i] = memoryHits[i];
            }
            MemoryHits = new ReadOnlyCollection<MemoryHit>(memoryHitsCopy);
            SourceEventClass = sourceEventClass;
            SourceEntityId = sourceEntityId;
        }

        public bool Equals(ViewerEvent? other)
        {
            if (other is null) return false;
            if (ReferenceEquals(this, other)) return true;
            if (ViewerEventId != other.ViewerEventId
                || SourceEventId != other.SourceEventId
                || SourceEventOrdinal != other.SourceEventOrdinal
                || BaseShotTypeId != other.BaseShotTypeId
                || EffectiveShotTypeId != other.EffectiveShotTypeId
                || ReduceMotionApplied != other.ReduceMotionApplied
                || StartTick != other.StartTick
                || EndTick != other.EndTick
                || Seed != other.Seed
                || StakesNormalized != other.StakesNormalized
                || MemoryRelevance != other.MemoryRelevance
                || FocalSubject != other.FocalSubject
                || SourceEventClass != other.SourceEventClass
                || SourceEntityId != other.SourceEntityId
                || ParticipantPlayerIds.Count != other.ParticipantPlayerIds.Count
                || MemoryHits.Count != other.MemoryHits.Count)
            {
                return false;
            }
            for (int i = 0; i < ParticipantPlayerIds.Count; i++)
            {
                if (ParticipantPlayerIds[i] != other.ParticipantPlayerIds[i]) return false;
            }
            for (int i = 0; i < MemoryHits.Count; i++)
            {
                if (!MemoryHits[i].Equals(other.MemoryHits[i])) return false;
            }
            return true;
        }

        public override bool Equals(object? obj) => obj is ViewerEvent other && Equals(other);
        public override int GetHashCode() => HashCode.Combine(
            ViewerEventId, SourceEventId, BaseShotTypeId, EffectiveShotTypeId, StartTick, SourceEventClass);
        public override string ToString() =>
            $"ViewerEvent(id={ViewerEventId}, shot={EffectiveShotTypeId}, tick={StartTick.Value}-{EndTick.Value}, class={SourceEventClass})";
    }
}

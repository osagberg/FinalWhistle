using System;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Contracts;

namespace FinalWhistle.Viewer.Core
{
    /// <summary>
    /// Runtime wrapper around <see cref="ViewerEvent"/> per the Phase-3
    /// dots-adapter blueprint at <c>docs/plans/dots-adapter-blueprint.md</c>
    /// §A. Resolves adapter-needed projections at construction time so
    /// adapters never re-look-up <see cref="ShotTypeCatalog"/> or
    /// re-compute the Stakes-as-float per render frame — the scene
    /// director resolves once when the event enters the active window and
    /// hands the wrapper to <see cref="IShotPresentationAdapter.PresentShot"/>.
    ///
    /// <para>
    /// <strong>Determinism note:</strong> the only float in this DTO is
    /// <see cref="StakesFloat"/>, derived from the canonical
    /// <see cref="ViewerEvent.StakesNormalized"/> (Q32.32 in [0, 1]) for
    /// shader-uniform consumption. The cast happens once at construction
    /// and is reused for every shader update — adapters never re-project.
    /// </para>
    ///
    /// <para>
    /// <strong>ElapsedTicks ownership:</strong> the director updates the
    /// elapsed-ticks counter as the active event ages; if an adapter
    /// needs a fresh value mid-shot (e.g., for a multi-stage shader uniform)
    /// the director constructs a new <see cref="ActiveViewerEvent"/> with
    /// the current value and re-calls
    /// <see cref="IShotPresentationAdapter.PresentShot"/>. Phase-3 keeps
    /// the wrapper immutable so debugging snapshots are byte-identical
    /// across render frames within a single tick.
    /// </para>
    /// </summary>
    public sealed class ActiveViewerEvent
    {
        /// <summary>The bridge-derived event payload; immutable.</summary>
        public ViewerEvent Event { get; }

        /// <summary>Resolved category from <see cref="ShotTypeCatalog.Get"/>.</summary>
        public ShotCategory ShotCategory { get; }

        /// <summary>Resolved shot type definition from <see cref="ShotTypeCatalog.Get"/>.</summary>
        public ShotTypeDefinition ShotDef { get; }

        /// <summary>
        /// <see cref="ViewerEvent.StakesNormalized"/> projected to
        /// <see cref="float"/> for shader-uniform consumption. Range
        /// [0, 1]. Derived once at construction.
        /// </summary>
        public float StakesFloat { get; }

        /// <summary>
        /// Ticks elapsed since <see cref="ViewerEvent.StartTick"/>. Always
        /// non-negative (constructor rejects negatives).
        ///
        /// <para>
        /// <strong>Director-owned upper bound (Codex round-1 follow-up
        /// against <c>b8d400f</c> — P3):</strong> the scene director (Slice
        /// 3 <c>DotsMatchDirector</c>) is responsible for retiring the
        /// active event when <c>ElapsedTicks</c> reaches
        /// <see cref="ShotDef"/>.<c>DurationTicks</c>. <c>ActiveViewerEvent</c>
        /// does NOT clamp or reject values above the duration at construction
        /// — the director's retirement boundary is the single source of truth
        /// so the contract stays renderer-agnostic (a 3D adapter may want
        /// different overrun behaviour than the dots adapter). Slice 3 ships
        /// a director test pinning the saturation contract directly. If
        /// adapter-side progress derivations land before Slice 3, treat
        /// <c>ElapsedTicks &gt; DurationTicks</c> as a director bug and surface
        /// loudly rather than silently clamping.
        /// </para>
        /// </summary>
        public int ElapsedTicks { get; }

        /// <exception cref="ArgumentNullException">Thrown when <paramref name="event"/> is null.</exception>
        /// <exception cref="ArgumentOutOfRangeException">Thrown when <paramref name="elapsedTicks"/> is negative.</exception>
        /// <exception cref="System.Collections.Generic.KeyNotFoundException">
        /// Thrown when <see cref="ViewerEvent.EffectiveShotTypeId"/> is not registered in
        /// <see cref="ShotTypeCatalog"/>. This is intentional loud-fail discipline per
        /// pr-review-toolkit:silent-failure-hunter 2026-04-30 P2 against the slice-1 first
        /// draft — a missing shot type is a content-pack-validation failure that must be
        /// surfaced at the boundary, not silently substituted with a fallback shot.
        /// </exception>
        public ActiveViewerEvent(ViewerEvent @event, int elapsedTicks)
        {
            if (@event is null)
            {
                throw new ArgumentNullException(nameof(@event));
            }
            if (elapsedTicks < 0)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(elapsedTicks), elapsedTicks,
                    "ElapsedTicks must be non-negative; the active window starts at 0.");
            }

            Event = @event;
            // ShotTypeCatalog.Get throws KeyNotFoundException on unknown id;
            // surface that to the director rather than swallowing it (a
            // missing shot type is a content-pack-validation failure that
            // must be loud, not silent — silent fallback to TacticalWide
            // would mask Phase-4+ ScriptableObject authoring drift).
            ShotDef = ShotTypeCatalog.Get(@event.EffectiveShotTypeId);
            ShotCategory = ShotDef.Category;

            // Q32.32 → float with a double-precision intermediate; matches
            // PitchView.FixedToWorld's discipline. StakesNormalized is
            // pre-clamped to [0, 1] at the bridge, so direct cast is safe;
            // the double avoids the float-mantissa cliff at high raw magnitudes.
            const double oneRaw = (double)Fixed.OneRaw;
            StakesFloat = (float)(@event.StakesNormalized.RawValue / oneRaw);

            ElapsedTicks = elapsedTicks;
        }
    }
}

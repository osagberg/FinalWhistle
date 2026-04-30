using FinalWhistle.Viewer.Contracts;

namespace FinalWhistle.Viewer.Core
{
    /// <summary>
    /// Adapter contract per ADR-0008 §"Adapter interface" + ADR-0009 §"Adapter
    /// scope" — every renderer adapter (dots-phase per ADR-0009; conditional
    /// cel-shaded 3D per ADR-0010 if the Phase-5/6 production-feasibility
    /// spike succeeds; future Workshop-shipped trusted-build adapters)
    /// implements this interface. Phase-3 lock per SPEC 2026-04-30
    /// dots-adapter blueprint Decision 1: 4-method shape lives in
    /// <c>Viewer.Core</c> (not <c>Viewer.Contracts</c>) because
    /// <see cref="PitchView"/> depends on <c>UnityEngine.Vector3</c>; the
    /// asmdef-level <c>noEngineReferences: true</c> on Contracts forbids
    /// engine deps there. A future headless-test adapter would need a
    /// FixedToWorld seam-split — Phase-5+ refactor if/when that demand
    /// emerges.
    /// </summary>
    public interface IShotPresentationAdapter
    {
        /// <summary>
        /// Code-owned identifier per the closed <see cref="AdapterId"/>
        /// registry. Used for adapter-keyed pass-activation hashes per
        /// <c>design/specs/golden-replay-corpus.md</c> v1 (the dots adapter's
        /// pass-activation hash differs from the 3D adapter's even on the
        /// same canonical seed because their rendering choices diverge —
        /// the corpus key disambiguates them).
        /// </summary>
        AdapterId AdapterId { get; }

        /// <summary>
        /// Bind the adapter to the active scene's pitch geometry. Called
        /// once per match-viewer scene load. The adapter caches whatever
        /// derived geometry it needs (sprite scales, camera bounds, etc.)
        /// from the supplied <see cref="PitchView"/> — Phase-3 dots adapter
        /// uses it for the <see cref="PitchView.FixedToWorld"/> conversion
        /// applied per dot per tick.
        /// </summary>
        void Initialize(PitchView pitch);

        /// <summary>
        /// Drive presentation for a single <see cref="ActiveViewerEvent"/>.
        /// Called by the scene director when a new <see cref="ViewerEvent"/>
        /// enters the active window or its frame timer advances.
        ///
        /// <para>
        /// <strong>Adapter MUST NOT mutate canonical sim state</strong> per
        /// ADR-0008 §"Determinism contract" + ADR-0009 §"Adapter is
        /// presentation-only." The adapter consumes the bridge-resolved
        /// <see cref="ViewerEvent.EffectiveShotTypeId"/> and never
        /// re-substitutes reduce-motion variants — that boundary is
        /// locked at the bridge per ADR-0008 §"Reduce-motion
        /// adapter-awareness."
        /// </para>
        /// </summary>
        void PresentShot(ActiveViewerEvent active);

        /// <summary>
        /// Release adapter-owned resources (sprite pools, render features,
        /// UIDocument lifetimes). Called once on scene unload or adapter
        /// swap (e.g., user toggles between dots and 3D adapters at
        /// runtime — Phase-5+ feature gated on the 3D-spike outcome).
        /// </summary>
        void Teardown();
    }
}

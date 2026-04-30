using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Contracts;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Phase-3 dots-adapter <c>ShotTypeSO</c> — data-driven framing
    /// parameters per the Slice-4 blueprint at
    /// <c>docs/plans/dots-adapter-blueprint.md</c> §B Slice 4. One asset
    /// per Phase-3 shot type (<c>tactical-wide</c>,
    /// <c>diagonal-attack-lane</c>, <c>pass-shot-impact</c>);
    /// <see cref="ShotCamera"/> reads the framing on <see cref="DotsAdapterRoot.PresentShot"/>
    /// and Lerps <see cref="UnityEngine.Camera.orthographicSize"/> +
    /// <see cref="Transform.rotation"/> over <see cref="TransitionTicks"/>.
    ///
    /// <para>
    /// <strong>Lives in <c>Viewer.Adapters.Dots</c></strong>: the SO is
    /// adapter-specific (the orthographic-camera + Y-tilt model is the
    /// dots adapter's framing language; the conditional 3D adapter would
    /// ship its own ShotTypeSO asset family with full-3D camera params).
    /// The <see cref="ShotTypeCatalog"/>-side <see cref="ShotTypeDefinition"/>
    /// stays renderer-agnostic per ADR-0008 §"Contract package boundary";
    /// this SO carries the dots-only framing layer.
    /// </para>
    ///
    /// <para>
    /// <strong>Phase-3 framing table</strong> per blueprint §B Slice 4:
    /// </para>
    /// <list type="table">
    ///   <listheader><description>Shot</description><description>orthographicSize</description><description>tilt (°)</description><description>target anchor</description></listheader>
    ///   <item><description>tactical-wide</description><description>38</description><description>90</description><description>ball XZ</description></item>
    ///   <item><description>diagonal-attack-lane</description><description>20</description><description>75</description><description>ball ↔ focal-subject midpoint</description></item>
    ///   <item><description>pass-shot-impact</description><description>12</description><description>80</description><description>focal-subject (null → ball)</description></item>
    /// </list>
    /// </summary>
    [CreateAssetMenu(
        menuName = "Final Whistle/Viewer/Dots/Shot Type",
        fileName = "ShotType")]
    public sealed class ShotTypeSO : ScriptableObject
    {
        /// <summary>
        /// Default transition ticks per blueprint §B Slice 4: 12 ticks at
        /// 60Hz canonical = 0.2s smooth-Lerp window from current framing
        /// to this one. Phase-4+ ScriptableObject authoring may tune
        /// per-shot.
        /// </summary>
        public const int DefaultTransitionTicks = 12;

        [Tooltip("Content-pack-qualified shot id (e.g., fwh.core:shot.tactical-wide). Must resolve in ShotTypeCatalog.")]
        [SerializeField] private string shotTypeId;

        [Tooltip("Orthographic half-height in metres at this shot's framing.")]
        [SerializeField] private float orthographicSize = 38f;

        [Tooltip("Camera Euler X rotation in degrees. 90° = pure top-down; <90° tilts the camera so its forward direction has positive Z.")]
        [SerializeField, Range(60f, 90f)] private float tiltDegrees = 90f;

        [Tooltip("Where the camera frames its target. BallXZ = follow ball; FocalSubject = focal player (null falls back to ball); BallFocalMidpoint = average of ball + focal.")]
        [SerializeField] private TargetAnchor targetAnchor = TargetAnchor.BallXZ;

        [Tooltip("Ticks (60Hz) to Lerp from the current framing to this shot's framing. Default 12 = 0.2s.")]
        [SerializeField, Min(1)] private int transitionTicks = DefaultTransitionTicks;

        public string ShotTypeId => shotTypeId;
        public float OrthographicSize => orthographicSize;
        public float TiltDegrees => tiltDegrees;
        public TargetAnchor Target => targetAnchor;
        public int TransitionTicks => transitionTicks;

        /// <summary>
        /// Transition window in real-time seconds, derived from
        /// <see cref="TransitionTicks"/> at the canonical
        /// <see cref="Tick.TicksPerSecond"/> rate. <c>TransitionTicks=12</c>
        /// → 0.2s. Centralised here per pr-review-toolkit
        /// type-design-analyzer Slice-4 P2 so the unit conversion lives
        /// in one place; <see cref="ShotCamera"/> consumes the seconds
        /// value directly. <strong>Note:</strong> at
        /// <c>TransitionTicks == 1</c> the duration is ~16.67ms which is
        /// effectively a hard cut at any framerate above 60fps; values
        /// in [2, ∞) yield a perceptible Lerp.
        /// </summary>
        public float TransitionDurationSeconds =>
            transitionTicks / (float)Tick.TicksPerSecond;

        /// <summary>
        /// Where the camera centres its frame for a given shot. Adapter-
        /// resolved at <see cref="DotsAdapterRoot.PresentShot"/> time
        /// against the active <see cref="ActiveViewerEvent.Event"/>'s
        /// <c>FocalSubject</c>.
        /// </summary>
        public enum TargetAnchor : byte
        {
            /// <summary>Ball position projected to pitch plane (Y=0). Default for tactical-wide.</summary>
            BallXZ = 0,

            /// <summary>Midpoint of ball + focal-subject; falls back to BallXZ when focal-subject is null.</summary>
            BallFocalMidpoint = 1,

            /// <summary>Focal-subject's position; falls back to ball when focal-subject is null.</summary>
            FocalSubject = 2,
        }

#if UNITY_EDITOR
        // OnValidate guards for Phase-3 authoring drift: a misauthored shot
        // id that doesn't resolve in ShotTypeCatalog or a tilt outside the
        // [60, 90] band makes the framing math malformed at runtime. Surface
        // at inspector edit time rather than at PresentShot time.
        private void OnValidate()
        {
            if (!string.IsNullOrWhiteSpace(shotTypeId)
                && !ShotTypeCatalog.TryGet(shotTypeId, out _))
            {
                Debug.LogError(
                    $"{nameof(ShotTypeSO)}.{nameof(shotTypeId)} '{shotTypeId}' does not resolve " +
                    $"in {nameof(ShotTypeCatalog)}. Use one of: " +
                    $"{ShotTypeCatalog.ShotTacticalWide} / " +
                    $"{ShotTypeCatalog.ShotDiagonalAttackLane} / " +
                    $"{ShotTypeCatalog.ShotPlayerIsolation} / " +
                    $"{ShotTypeCatalog.ShotPassShotImpact} / " +
                    $"{ShotTypeCatalog.ShotAftermathFreeze}.",
                    this);
            }
            if (orthographicSize <= 0f)
            {
                Debug.LogError(
                    $"{nameof(ShotTypeSO)}.{nameof(orthographicSize)} must be strictly positive; got {orthographicSize}.",
                    this);
            }
            // Tilt is [Range(60, 90)] in the inspector; OnValidate doesn't
            // need a redundant guard. transitionTicks has [Min(1)].
            _ = Tick.TicksPerSecond; // touch the Tick API so refactors that
                                     // delete TicksPerSecond surface here.
        }
#endif
    }
}

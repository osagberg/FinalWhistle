using System;
using FinalWhistle.Viewer.Contracts;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Focal-subject highlighter per the Phase-3 dots-adapter blueprint
    /// §B Slice 7. Spawns a single child <see cref="SpriteRenderer"/>
    /// rendering <see cref="ringSprite"/> (a 64×64 transparent PNG with
    /// a white outline) and positions it over the focal-player dot
    /// during shots whose <see cref="ShotCategory"/> is
    /// <see cref="ShotCategory.PlayerIsolation"/> or
    /// <see cref="ShotCategory.PassShotImpact"/> when a non-null
    /// <see cref="ViewerEvent.FocalSubject"/> is present.
    ///
    /// <para>
    /// <strong>One ring, one focal subject.</strong> Phase-3 events
    /// surface a single focal subject at a time; multi-focal cases
    /// (e.g. press-trap closing on the ball-carrier with two pressers
    /// shown in-frame) are Phase-4+. Calling
    /// <see cref="Engage(string)"/> while already engaged just updates
    /// the focal-subject lookup; <see cref="Disengage"/> hides the
    /// ring.
    /// </para>
    ///
    /// <para>
    /// <strong>LateUpdate position sync:</strong> the ring re-reads the
    /// focal dot's <c>transform.position</c> every frame so it tracks
    /// <see cref="DotPool"/>'s sub-tick interpolation. Reading
    /// position-only keeps this presentation-only — output is the ring's
    /// transform, which never feeds back into MatchSim state. ADR-0008
    /// determinism contract preserved.
    /// </para>
    ///
    /// <para>
    /// <strong>ReduceMotion:</strong> the blueprint reduces motion by
    /// stripping motion-line bursts (Phase-3 hazard surface) but keeps
    /// the selection ring — the ring is the static identity cue that
    /// makes the focal player legible without animated motion. So this
    /// component does NOT consume <see cref="ViewerEvent.ReduceMotionApplied"/>.
    /// </para>
    /// </summary>
    public sealed class SelectionRing : MonoBehaviour
    {
        // Hover the ring just above the dot's Y-lift so it composes
        // cleanly without z-fighting at top-down framings. Dot Y-lift is
        // 0.05f (DotPool.DotYLift); ring lifts to 0.06f.
        private const float RingYLift = 0.06f;

        // Visual scale of the ring relative to the dot's metres-diameter.
        // 1.6x outfield diameter gives a halo ~0.6m wider than the dot —
        // legible at tactical-wide zoom (orthoSize=38) without dominating
        // the dot itself.
        private const float RingDiameterMultiplier = 1.6f;
        private const float OutfieldDiameterMetres = 1.4f;

        [SerializeField] private Sprite ringSprite;
        [SerializeField] private Color ringColor = new(1f, 1f, 1f, 0.85f);

        private DotPool dotPool;
        private PitchView pitchView;
        private SpriteRenderer ringRenderer;
        private int currentFocalDotIndex = -1;

        /// <summary>
        /// Wire the ring to its <see cref="DotPool"/> + <see cref="PitchView"/>
        /// + sprite source. Idempotent: subsequent calls re-bind
        /// references but reuse the existing renderer.
        /// </summary>
        public void Initialize(DotPool dotPoolArg, PitchView pitchViewArg)
        {
            if (dotPoolArg == null) throw new ArgumentNullException(nameof(dotPoolArg));
            if (pitchViewArg is null) throw new ArgumentNullException(nameof(pitchViewArg));
            if (ringSprite == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(SelectionRing)}.{nameof(ringSprite)} reference missing; assign " +
                    "selection_ring.png in the scene inspector.");
            }

            dotPool = dotPoolArg;
            pitchView = pitchViewArg;

            if (ringRenderer == null)
            {
                GameObject ringObj = new("Ring");
                ringObj.transform.SetParent(transform, worldPositionStays: false);
                ringObj.transform.localRotation = Quaternion.Euler(90f, 0f, 0f);
                ringRenderer = ringObj.AddComponent<SpriteRenderer>();
                ringRenderer.sprite = ringSprite;
                ringRenderer.color = ringColor;
                ringRenderer.sortingOrder = -1; // behind dots
                float diameter = OutfieldDiameterMetres * RingDiameterMultiplier
                                 * pitchView.WorldUnitsPerMeter;
                ringObj.transform.localScale = new Vector3(diameter, diameter, diameter);
            }
            ringRenderer.enabled = false;
            currentFocalDotIndex = -1;
        }

        /// <summary>
        /// Engage the ring on the named focal subject. No-op (and
        /// disengages the ring) when <paramref name="focalSubject"/>
        /// is null, empty, or cannot be resolved by
        /// <see cref="DotPool.IndexForFocalSubject"/>.
        /// </summary>
        public void Engage(string focalSubject)
        {
            EnsureInitialized();
            int idx = dotPool.IndexForFocalSubject(focalSubject);
            if (idx < 0)
            {
                Disengage();
                return;
            }
            currentFocalDotIndex = idx;
            ringRenderer.enabled = true;
            SyncRingTransform();
        }

        /// <summary>Hide the ring + clear the focal-subject lookup.</summary>
        public void Disengage()
        {
            if (ringRenderer != null)
            {
                ringRenderer.enabled = false;
            }
            currentFocalDotIndex = -1;
        }

        /// <summary>
        /// Test/diagnostic surface: returns the resolved focal-dot
        /// index, or -1 when not engaged.
        /// </summary>
        internal int CurrentFocalDotIndex => currentFocalDotIndex;

        internal bool IsEngaged => ringRenderer != null && ringRenderer.enabled;

        private void LateUpdate()
        {
            if (currentFocalDotIndex < 0 || ringRenderer == null || !ringRenderer.enabled)
            {
                return;
            }
            SyncRingTransform();
        }

        private void SyncRingTransform()
        {
            // The focal dot's transform.position is the interpolated
            // frame-time position (DotPool.Update wrote it earlier this
            // frame). Reading it here gives the ring the same per-frame
            // position the player dot is rendered at — no lag.
            Transform dotTransform = dotPool.transform.GetChild(currentFocalDotIndex);
            Vector3 dotPos = dotTransform.position;
            Transform ringT = ringRenderer.transform;
            ringT.position = new Vector3(dotPos.x, RingYLift, dotPos.z);
        }

        private void EnsureInitialized()
        {
            if (dotPool == null || pitchView == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(SelectionRing)}.{nameof(Initialize)} must be called before " +
                    $"{nameof(Engage)} / {nameof(Disengage)}.");
            }
        }
    }
}

using System;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Contracts;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Drives the orthographic top-down <see cref="UnityEngine.Camera"/>
    /// per the Phase-3 dots-adapter blueprint §B Slice 4. On a new shot
    /// (<see cref="BeginShot"/> or <see cref="BeginAdapterShot"/>) the
    /// current framing is captured as the Lerp <c>from</c>-state and the
    /// SO's framing becomes the <c>to</c>-state; <see cref="LateUpdate"/>
    /// advances a single Lerp parameter <c>t</c> from 0 to 1 over
    /// <see cref="ShotTypeSO.TransitionDurationSeconds"/> + samples
    /// <c>Mathf.Lerp(from, to, t)</c> to get the current framing. This
    /// is the textbook constant-velocity transition shape (per
    /// pr-review-toolkit type-design-analyzer Slice-4 P1a closure of the
    /// initial draft's exponential-decay drift).
    ///
    /// <para>
    /// <strong>Hand-rolled, no Cinemachine</strong> per blueprint Decision
    /// 2 (Cinemachine 3.1.6 is installed but premature for a locked
    /// orthographic top-down view; the conditional 3D adapter ADR-0010 is
    /// the natural Cinemachine consumer if/when the Phase-5 spike
    /// greenlights). Phase-3 keeps the camera-rig footprint small.
    /// </para>
    ///
    /// <para>
    /// <strong>Position derivation:</strong> for an orthographic camera at
    /// height <c>H</c> looking down with Euler X rotation <c>θ</c>, the
    /// frame centre on the pitch plane (Y=0) is offset from the camera
    /// position along <c>+Z</c> by <c>H * tan(90° - θ)</c>. Inverting:
    /// to frame a target <c>T = (tx, 0, tz)</c>, position the camera at
    /// <c>(tx, H, tz - H * tan(90° - θ))</c>. For <c>θ = 90°</c> (pure
    /// top-down), the offset is zero; for <c>θ = 75°</c>, the offset is
    /// <c>H * tan(15°) ≈ 0.268 * H</c>. Height stays constant across
    /// shots so the dots scale stays uniform.
    /// </para>
    ///
    /// <para>
    /// <strong>Determinism note:</strong> the Lerp uses real-time
    /// <see cref="Time.deltaTime"/> for transition smoothness. This is
    /// presentation-only — output is camera transform + projection, never
    /// feeds back to canonical state.
    /// </para>
    /// </summary>
    public sealed class ShotCamera : MonoBehaviour
    {
        /// <summary>
        /// Camera height (Y) above pitch plane. Locked across all shots
        /// so the dots scale stays uniform and the orthographic view
        /// volume framing matches the height-derived position offset.
        /// </summary>
        public const float CameraHeightMetres = 50f;

        [SerializeField] private Camera targetCamera;
        [SerializeField] private DotPool dotPool;

        // Default framing applied on Initialize + auto-restored after
        // an active shot's duration envelope expires. Phase-3 default =
        // tactical-wide per blueprint acceptance criterion.
        [SerializeField] private ShotTypeSO defaultShot;

        private PitchView pitchView;

        // Lerp from-state: captured at BeginShot/BeginAdapterShot time.
        private float fromOrthoSize;
        private float fromTiltDegrees;
        private Vector3 fromTargetWorld;

        // Lerp to-state: where the camera is heading.
        private ShotTypeSO targetShot;
        private float targetOrthoSize;
        private float targetTiltDegrees;
        private string targetFocalSubject;

        // Lerp current state — what the camera renders this frame.
        private float currentOrthoSize;
        private float currentTiltDegrees;
        private Vector3 currentTargetWorld;

        // Transition smoothing — Lerp from→to over TransitionTicks ticks.
        // No field initializer (per pr-review-toolkit feature-dev:code-reviewer
        // Slice-4 P2.5: the prior `0.2f` initializer was unreachable because
        // every code path that reads it overwrites it first).
        private float transitionDurationSeconds;
        private float transitionElapsedSeconds;
        private bool transitioning;

        // Active shot expiry — when the active event's StartTick +
        // EffectiveShot.DurationTicks elapses, return to defaultShot.
        // EndTick is exclusive: the shot is visible for ticks
        // [StartTick, EndTick); auto-return fires at currentSimTick == EndTick.
        // This matches the ActiveViewerEvent.ElapsedTicks contract that
        // says elapsed ∈ [0, DurationTicks) is the visible window. Last-
        // writer-wins on overlapping events at Phase-3 (Phase-4+ may add
        // cinematics-priority arbitration).
        private long activeShotEndTick;
        private bool hasActiveShot;

        // Telemetry: one-shot warning the first time BeginAdapterShot bails
        // because hasActiveShot is true. Per pr-review-toolkit
        // silent-failure-hunter Slice-4 P2-B: distinguishes "heuristic
        // never fires because conditions never met" from "heuristic fires
        // but is suppressed by an active event-driven shot."
        private bool warnedAdapterShotSuppressed;

        public void Initialize(PitchView pitchViewArg)
        {
            if (pitchViewArg is null)
            {
                throw new ArgumentNullException(nameof(pitchViewArg));
            }
            if (targetCamera == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(ShotCamera)}.{nameof(targetCamera)} reference missing; " +
                    "assign the Main Camera in the scene inspector.");
            }
            if (dotPool == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(ShotCamera)}.{nameof(dotPool)} reference missing; " +
                    "assign the DotPool in the scene inspector.");
            }
            if (defaultShot == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(ShotCamera)}.{nameof(defaultShot)} reference missing; " +
                    "assign tactical-wide.asset (or another default ShotTypeSO) in the inspector.");
            }
            ValidateShotResolves(defaultShot);

            pitchView = pitchViewArg;

            currentOrthoSize = defaultShot.OrthographicSize;
            currentTiltDegrees = defaultShot.TiltDegrees;
            currentTargetWorld = pitchView.Origin;

            ApplyFraming(currentOrthoSize, currentTiltDegrees, currentTargetWorld);

            targetShot = defaultShot;
            targetOrthoSize = currentOrthoSize;
            targetTiltDegrees = currentTiltDegrees;
            targetFocalSubject = null;
            fromOrthoSize = currentOrthoSize;
            fromTiltDegrees = currentTiltDegrees;
            fromTargetWorld = currentTargetWorld;
            transitioning = false;
            transitionDurationSeconds = 0f;
            transitionElapsedSeconds = 0f;
            hasActiveShot = false;
            warnedAdapterShotSuppressed = false;
        }

        /// <summary>
        /// Begin a transition toward the framing in <paramref name="shot"/>.
        /// Called by <see cref="DotsAdapterRoot.PresentShot"/> when a new
        /// <see cref="ActiveViewerEvent"/> arrives.
        /// </summary>
        public void BeginShot(ShotTypeSO shot, ActiveViewerEvent active)
        {
            EnsureInitialized(nameof(BeginShot));
            if (shot == null)
            {
                throw new ArgumentNullException(nameof(shot));
            }
            if (active is null)
            {
                throw new ArgumentNullException(nameof(active));
            }
            ValidateShotResolves(shot);

            CaptureFromCurrent();
            targetShot = shot;
            targetOrthoSize = shot.OrthographicSize;
            targetTiltDegrees = shot.TiltDegrees;
            targetFocalSubject = active.Event.FocalSubject;
            transitionDurationSeconds = shot.TransitionDurationSeconds;
            transitionElapsedSeconds = 0f;
            transitioning = true;

            activeShotEndTick = active.Event.EndTick.Value;
            hasActiveShot = true;
        }

        /// <summary>
        /// Adapter-local <c>diagonal-attack-lane</c> heuristic per blueprint
        /// Decision 3: callers (e.g. <see cref="DotsMatchDirector"/>'s
        /// per-tick check) invoke this when ball Z-velocity exceeds the
        /// threshold AND no event-driven shot is currently active. Frames
        /// the ball-as-target without an event lifetime so the auto-return
        /// to default happens naturally as soon as the heuristic stops
        /// firing.
        /// </summary>
        public void BeginAdapterShot(ShotTypeSO shot)
        {
            EnsureInitialized(nameof(BeginAdapterShot));
            if (shot == null)
            {
                throw new ArgumentNullException(nameof(shot));
            }
            if (hasActiveShot)
            {
                // Bridge-emitted events take precedence over adapter-local
                // heuristics — don't override an in-flight event. Telemetry
                // (one-shot) per pr-review-toolkit silent-failure-hunter
                // Slice-4 P2-B: Slice-7 observers reporting "heuristic never
                // fires" need to distinguish "conditions never met" from
                // "suppressed by active event."
                if (!warnedAdapterShotSuppressed)
                {
                    Debug.Log(
                        $"{nameof(ShotCamera)}: adapter-local heuristic shot '{shot.ShotTypeId}' " +
                        $"suppressed by an active event-driven shot. This message logs once " +
                        "per session (subsequent suppressions are silent).",
                        this);
                    warnedAdapterShotSuppressed = true;
                }
                return;
            }
            ValidateShotResolves(shot);

            CaptureFromCurrent();
            targetShot = shot;
            targetOrthoSize = shot.OrthographicSize;
            targetTiltDegrees = shot.TiltDegrees;
            targetFocalSubject = null;
            transitionDurationSeconds = shot.TransitionDurationSeconds;
            transitionElapsedSeconds = 0f;
            transitioning = true;
        }

        /// <summary>
        /// Per-FixedUpdate hook from <see cref="DotsMatchDirector"/>.
        /// Checks whether the active shot has expired (by canonical tick
        /// count) and snaps the target framing back to <see cref="defaultShot"/>.
        /// </summary>
        public void OnSimTick(Tick currentSimTick)
        {
            if (pitchView == null || !hasActiveShot)
            {
                return;
            }
            if (currentSimTick.Value >= activeShotEndTick)
            {
                hasActiveShot = false;
                if (targetShot != defaultShot)
                {
                    CaptureFromCurrent();
                    targetShot = defaultShot;
                    targetOrthoSize = defaultShot.OrthographicSize;
                    targetTiltDegrees = defaultShot.TiltDegrees;
                    targetFocalSubject = null;
                    transitionDurationSeconds = defaultShot.TransitionDurationSeconds;
                    transitionElapsedSeconds = 0f;
                    transitioning = true;
                }
            }
        }

        // LateUpdate per blueprint: target tracked AFTER DotPool.Update has
        // synced dot positions for this frame, so the focal-subject lookup
        // reads the freshly-interpolated position via DotPool.BallWorldPosition
        // (which now reads transform.position so Slice-4 P1.1 closure
        // returns the interpolated dot, not the post-tick snapshot).
        private void LateUpdate()
        {
            if (pitchView == null)
            {
                return;
            }

            // Target world is recomputed every frame regardless of
            // transition state — the underlying ball / focal-subject
            // moves continuously via DotPool's sub-tick interpolation.
            Vector3 toTargetWorld = ResolveTarget(targetShot, targetFocalSubject);

            float t;
            if (!transitioning)
            {
                t = 1f;
            }
            else
            {
                transitionElapsedSeconds += Time.deltaTime;
                t = transitionDurationSeconds > 0f
                    ? Mathf.Clamp01(transitionElapsedSeconds / transitionDurationSeconds)
                    : 1f;
                if (t >= 1f)
                {
                    transitioning = false;
                }
            }

            // CONSTANT-VELOCITY Lerp from CAPTURED from-state to current
            // to-state per pr-review-toolkit type-design-analyzer Slice-4
            // P1a fix. The prior draft used Lerp(current, target, t) which
            // produced exponential decay (each frame moves t-fraction of
            // the REMAINING gap, not t-fraction of the ORIGINAL gap).
            currentOrthoSize = Mathf.Lerp(fromOrthoSize, targetOrthoSize, t);
            currentTiltDegrees = Mathf.Lerp(fromTiltDegrees, targetTiltDegrees, t);
            currentTargetWorld = Vector3.Lerp(fromTargetWorld, toTargetWorld, t);

            ApplyFraming(currentOrthoSize, currentTiltDegrees, currentTargetWorld);
        }

        private void ApplyFraming(float orthoSize, float tiltDegrees, Vector3 targetWorld)
        {
            targetCamera.orthographic = true;
            targetCamera.orthographicSize = Mathf.Max(0.1f, orthoSize);
            // Compute the camera position offset from the target along -Z
            // to keep the target at frame centre given the tilt.
            // tilt=90° → offset=0 (pure top-down). tilt<90° → camera moves
            // back along -Z by H * tan(90° - tilt).
            float tiltFromVertical = 90f - tiltDegrees;
            float zOffset = CameraHeightMetres * Mathf.Tan(tiltFromVertical * Mathf.Deg2Rad);
            Vector3 cameraPos = new(
                targetWorld.x,
                targetWorld.y + CameraHeightMetres,
                targetWorld.z - zOffset);
            targetCamera.transform.position = cameraPos;
            targetCamera.transform.rotation = Quaternion.Euler(tiltDegrees, 0f, 0f);
        }

        private Vector3 ResolveTarget(ShotTypeSO shot, string focalSubject)
        {
            // BallWorldPosition now returns the INTERPOLATED ball position
            // (the rendered transform.position) per pr-review-toolkit
            // feature-dev:code-reviewer Slice-4 P1.1 closure — Update
            // ran first in the frame's lifecycle and wrote the lerped
            // ball position to transform; reading it here gives the
            // camera the same frame-time position the dot is rendered at.
            Vector3 ballWorld = dotPool.BallWorldPosition;
            switch (shot.Target)
            {
                case ShotTypeSO.TargetAnchor.BallXZ:
                    return new Vector3(ballWorld.x, 0f, ballWorld.z);
                case ShotTypeSO.TargetAnchor.FocalSubject:
                {
                    if (TryGetFocalWorld(focalSubject, out Vector3 focalWorld))
                    {
                        return new Vector3(focalWorld.x, 0f, focalWorld.z);
                    }
                    return new Vector3(ballWorld.x, 0f, ballWorld.z);
                }
                case ShotTypeSO.TargetAnchor.BallFocalMidpoint:
                {
                    if (TryGetFocalWorld(focalSubject, out Vector3 focalWorld))
                    {
                        Vector3 mid = (ballWorld + focalWorld) * 0.5f;
                        return new Vector3(mid.x, 0f, mid.z);
                    }
                    return new Vector3(ballWorld.x, 0f, ballWorld.z);
                }
                default:
                    return new Vector3(ballWorld.x, 0f, ballWorld.z);
            }
        }

        private bool TryGetFocalWorld(string focalSubject, out Vector3 worldPos)
        {
            if (string.IsNullOrEmpty(focalSubject))
            {
                worldPos = default;
                return false;
            }
            return dotPool.TryGetFocalWorldPosition(focalSubject, out worldPos);
        }

        private void CaptureFromCurrent()
        {
            // Snapshot whatever the camera is rendering RIGHT NOW as the
            // Lerp from-state. This gives a constant-velocity transition
            // even when a new shot interrupts an in-progress transition
            // (the camera doesn't "snap back" to the prior shot's start
            // before transitioning to the new one — it Lerps from where
            // it is now to the new target).
            fromOrthoSize = currentOrthoSize;
            fromTiltDegrees = currentTiltDegrees;
            fromTargetWorld = currentTargetWorld;
        }

        private void EnsureInitialized(string callerName)
        {
            if (pitchView == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(ShotCamera)}.{nameof(Initialize)} must be called before {callerName}.");
            }
        }

        // Runtime gate per pr-review-toolkit silent-failure-hunter Slice-4
        // P2-A: ShotTypeSO.OnValidate's Debug.LogError doesn't block save,
        // so a misauthored shotTypeId can ship. Validate at the runtime
        // consumer (BeginShot / BeginAdapterShot / Initialize) so the
        // failure surfaces loud at scene-load + per-shot rather than
        // silently rendering a default-fallback framing.
        private static void ValidateShotResolves(ShotTypeSO shot)
        {
            if (string.IsNullOrEmpty(shot.ShotTypeId))
            {
                throw new InvalidOperationException(
                    $"ShotTypeSO {shot.name} has empty ShotTypeId; populate the inspector field.");
            }
            if (!ShotTypeCatalog.TryGet(shot.ShotTypeId, out _))
            {
                throw new InvalidOperationException(
                    $"ShotTypeSO {shot.name} references unknown shot id '{shot.ShotTypeId}'; " +
                    $"register in {nameof(ShotTypeCatalog)} or fix the inspector value.");
            }
        }

        /// <summary>Test/diagnostic surface — current framing snapshot.</summary>
        public CurrentFraming Sample() => new(currentOrthoSize, currentTiltDegrees, currentTargetWorld, targetCamera != null ? targetCamera.transform.position : Vector3.zero);

        public readonly struct CurrentFraming
        {
            public CurrentFraming(float orthoSize, float tiltDegrees, Vector3 targetWorld, Vector3 cameraWorld)
            {
                OrthoSize = orthoSize;
                TiltDegrees = tiltDegrees;
                TargetWorld = targetWorld;
                CameraWorld = cameraWorld;
            }
            public float OrthoSize { get; }
            public float TiltDegrees { get; }
            public Vector3 TargetWorld { get; }
            public Vector3 CameraWorld { get; }
        }
    }
}

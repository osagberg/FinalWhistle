using FinalWhistle.Viewer.Core;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Camera-rhythm cadence tier per the Phase-3 dots-adapter blueprint
    /// §B Slice 7. Resolved deterministically from
    /// <see cref="ActiveViewerEvent.StakesNormalized"/>; modulates
    /// <c>ShotCamera</c>'s per-shot transition duration so that
    /// low-stakes shots breathe (slower transitions, more "calm camera")
    /// and high-stakes shots punch (faster transitions, more
    /// "responsive camera"). Adapter-internal — NOT in
    /// <c>Viewer.Contracts</c> — because the tier projection is a
    /// renderer-side rhythm choice, not part of the deterministic
    /// MatchSim → ViewerEvent contract.
    /// </summary>
    /// <remarks>
    /// Thresholds chosen to evenly divide the stakes scale into thirds
    /// (Calm: [0, 0.33), Standard: [0.33, 0.67), Tension: [0.67, 1.0]).
    /// Tick counts at 60Hz: 24 = 0.4s, 12 = 0.2s, 6 = 0.1s.
    /// </remarks>
    public enum CadenceTier
    {
        /// <summary>
        /// Low-stakes (<c>StakesNormalized &lt; 0.33</c>). 24-tick transition
        /// (0.4s @60Hz). Slower transitions give the camera time to
        /// breathe between routine events.
        /// </summary>
        Calm = 0,

        /// <summary>
        /// Medium-stakes (<c>0.33 ≤ StakesNormalized &lt; 0.67</c>).
        /// 12-tick transition (0.2s @60Hz). The Phase-3 default cadence —
        /// matches the prior fixed <c>ShotTypeSO.TransitionDurationSeconds</c>
        /// authoring (most shot SOs ship a 0.2s transition).
        /// </summary>
        Standard = 1,

        /// <summary>
        /// High-stakes (<c>StakesNormalized ≥ 0.67</c>). 6-tick transition
        /// (0.1s @60Hz). Faster transitions read as the camera "snapping"
        /// to the action, signalling momentum to the viewer.
        /// </summary>
        Tension = 2,
    }

    /// <summary>
    /// Pure helpers for resolving <see cref="CadenceTier"/> from a stakes
    /// value + projecting the tier onto a transition duration in seconds.
    /// Separated from <c>ShotCamera</c> so the math is testable in EditMode
    /// without instantiating a <see cref="UnityEngine.Camera"/>.
    /// </summary>
    public static class CadenceTierResolver
    {
        // Locked thresholds + tick counts per blueprint §B Slice 7.
        public const float CalmThreshold = 0.33f;
        public const float StandardThreshold = 0.67f;
        public const int CalmTransitionTicks = 24;
        public const int StandardTransitionTicks = 12;
        public const int TensionTransitionTicks = 6;
        public const float TicksPerSecond = 60f;

        /// <summary>
        /// Resolve the cadence tier for a normalized-stakes value. Strict
        /// half-open intervals per blueprint: Calm in [0, 0.33), Standard
        /// in [0.33, 0.67), Tension in [0.67, 1.0]. Values outside [0, 1]
        /// clamp at the endpoints; NaN resolves to Standard (the safe
        /// Phase-3 default — see remarks).
        /// </summary>
        /// <remarks>
        /// NaN handling: <see cref="ActiveViewerEvent.StakesNormalized"/>
        /// is constructed from <c>(float)Fixed.ToDouble(StakesNormalized)</c>
        /// and the ViewerEvent constructor rejects NaN sources upstream;
        /// but a future adapter that constructs <c>ActiveViewerEvent</c>
        /// outside the bridge could leak NaN. Treating NaN as Standard
        /// keeps the camera rolling (the Phase-3 default cadence) rather
        /// than throwing — a NaN-stakes camera bug should surface as a
        /// warn-once in <see cref="ActiveViewerEvent"/>, not a hard
        /// failure in the camera.
        /// </remarks>
        public static CadenceTier Resolve(float stakesNormalized)
        {
            if (float.IsNaN(stakesNormalized))
            {
                return CadenceTier.Standard;
            }
            if (stakesNormalized < CalmThreshold)
            {
                return CadenceTier.Calm;
            }
            if (stakesNormalized < StandardThreshold)
            {
                return CadenceTier.Standard;
            }
            return CadenceTier.Tension;
        }

        /// <summary>
        /// Project a tier onto a transition duration in seconds at the
        /// 60Hz canonical tick rate. 24 / 12 / 6 ticks per the blueprint.
        /// </summary>
        public static float TransitionSecondsForCadence(CadenceTier tier)
        {
            int ticks = tier switch
            {
                CadenceTier.Calm => CalmTransitionTicks,
                CadenceTier.Tension => TensionTransitionTicks,
                _ => StandardTransitionTicks,
            };
            return ticks / TicksPerSecond;
        }
    }
}

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Pure-C# helpers + canonical shader-uniform names for the Phase-3
    /// anime-presentation budget surfaces (impact-frame flash + diagonal
    /// screen-tone) per the dots-adapter blueprint §B Slice 5. Lives in
    /// <c>Viewer.Adapters.Dots</c> alongside the renderer features that
    /// read these uniforms.
    ///
    /// <para>
    /// <strong>Why a static helper class?</strong> Decay + strength math
    /// must be pure-C# so it's L1-testable in EditMode without spinning up
    /// the Unity render thread. The renderer features themselves are hard
    /// to exercise in EditMode (no real camera + render path); the
    /// arithmetic that determines the per-frame uniform values is the
    /// load-bearing logic and lives here so a unit test can pin it.
    /// </para>
    ///
    /// <para>
    /// <strong>Determinism contract</strong>: every input is a canonical
    /// <see cref="MatchSim.Sim.Tick"/> count; no <c>Time.time</c>,
    /// <c>Time.deltaTime</c>, or shader <c>_Time</c> reads. The shaders
    /// that consume these uniforms are <c>_Time</c>-clean per
    /// <c>.claude/rules/Scripts/Viewer/RULES.md</c>.
    /// </para>
    /// </summary>
    internal static class AnimePresentationUniforms
    {
        // Global shader-uniform names. FW_ prefix avoids collision with
        // any third-party global. UnityEngine.Shader.PropertyToID hashes
        // these once at first use; we cache the ids in the director.

        // internal const (not public) per pr-review-toolkit
        // type-design-analyzer Slice-5 P3 closure: the enclosing class
        // is `internal static`, so `public` was effectively `internal`
        // already; tightening the modifier removes the visibility
        // ambiguity for future readers.
        internal const string FlashIntensityName = "_FW_FlashIntensity";
        internal const string ScreenToneStrengthName = "_FW_ScreenToneStrength";
        internal const string ElapsedTicksName = "_FW_ElapsedTicks";

        /// <summary>
        /// Compute current flash intensity given a flash-start tick + a
        /// decay window length. Linear decay from 1.0 at start to 0.0
        /// at <c>startTick + decayTicks</c>. After expiry: 0.
        ///
        /// <para>
        /// Returns <c>0</c> when no flash is active (<paramref name="startTick"/>
        /// is negative). Returns <c>0</c> when the decay window is
        /// non-positive (defensive — caller should pass a positive
        /// <paramref name="decayTicks"/>).
        /// </para>
        ///
        /// <para>
        /// Linear decay was chosen over exponential to keep the math
        /// tick-quantised and trivially testable. Anime impact-frame
        /// references favour a sharp on-set + fast linear fade-out
        /// rather than an exponential tail.
        /// </para>
        /// </summary>
        public static float ComputeFlashIntensity(long currentTick, long startTick, int decayTicks)
        {
            if (startTick < 0)
            {
                return 0f;
            }
            if (decayTicks <= 0)
            {
                return 0f;
            }
            long elapsed = currentTick - startTick;
            if (elapsed < 0)
            {
                return 0f;
            }
            if (elapsed >= decayTicks)
            {
                return 0f;
            }
            float t = (float)elapsed / decayTicks;
            return 1f - t;
        }

        /// <summary>
        /// Compute current screen-tone overlay strength given an active
        /// window <c>[startTick, endTick)</c>, a base strength derived
        /// from <see cref="ViewerEvent.StakesNormalized"/> at trigger
        /// time, and a 6-tick fade-in / fade-out envelope. Returns
        /// <c>0</c> outside the window. The fade is symmetric.
        ///
        /// <para>
        /// Symmetric fade-in/out hides the seam between an active
        /// aftermath-freeze tone and the underlying scene; without it
        /// the screen would pop on/off at the canonical tick boundary
        /// and read as a glitch rather than a manga screen-tone.
        /// </para>
        /// </summary>
        public static float ComputeScreenToneStrength(
            long currentTick, long startTick, long endTick, float baseStrength, int fadeTicks)
        {
            if (startTick < 0 || endTick <= startTick)
            {
                return 0f;
            }
            if (currentTick < startTick || currentTick >= endTick)
            {
                return 0f;
            }
            if (fadeTicks <= 0 || baseStrength <= 0f)
            {
                return baseStrength <= 0f ? 0f : Clamp01(baseStrength);
            }

            long elapsed = currentTick - startTick;
            long remaining = endTick - currentTick;
            // Both elapsed and remaining are >0 inside the open
            // (startTick, endTick) interval and form the symmetric ramp.
            int fadeWindow = fadeTicks;
            float fadeIn = elapsed >= fadeWindow ? 1f : (float)elapsed / fadeWindow;
            float fadeOut = remaining >= fadeWindow ? 1f : (float)remaining / fadeWindow;
            float envelope = fadeIn < fadeOut ? fadeIn : fadeOut;
            return Clamp01(baseStrength * envelope);
        }

        private static float Clamp01(float value) =>
            value < 0f ? 0f : value > 1f ? 1f : value;
    }
}

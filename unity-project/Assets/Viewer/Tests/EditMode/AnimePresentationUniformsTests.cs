using FinalWhistle.Viewer.Adapters.Dots;
using NUnit.Framework;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Slice-5 tests for the pure-C# decay + strength math feeding the
    /// impact-frame and screen-tone shader globals. The renderer features
    /// themselves are hard to L1-test (need a real URP renderer + camera);
    /// these tests cover the load-bearing arithmetic that determines what
    /// the shaders see each frame.
    ///
    /// <para>
    /// Determinism contract per <c>.claude/rules/Scripts/Viewer/RULES.md</c>:
    /// every input is a canonical <see cref="MatchSim.Sim.Tick"/> count;
    /// no <c>Time.time</c> / <c>Time.deltaTime</c> / shader <c>_Time</c>
    /// reads. Same canonical state in → same global uniforms out, byte-
    /// identical across platforms + frame-rates.
    /// </para>
    /// </summary>
    public sealed class AnimePresentationUniformsTests
    {
        // ----- Flash intensity (linear decay) -----

        [Test]
        public void FlashIntensity_BeforeStartOrInactive_ReturnsZero()
        {
            // Sentinel start tick = -1 means "no flash active."
            float i = AnimePresentationUniforms.ComputeFlashIntensity(
                currentTick: 100, startTick: -1, decayTicks: 12);
            Assert.That(i, Is.EqualTo(0f));
        }

        [Test]
        public void FlashIntensity_AtStart_ReturnsOne()
        {
            float i = AnimePresentationUniforms.ComputeFlashIntensity(
                currentTick: 100, startTick: 100, decayTicks: 12);
            Assert.That(i, Is.EqualTo(1f).Within(1e-6f));
        }

        [Test]
        public void FlashIntensity_AtMidpoint_ReturnsHalf()
        {
            // 6 ticks into a 12-tick window → 1 - (6/12) = 0.5.
            float i = AnimePresentationUniforms.ComputeFlashIntensity(
                currentTick: 106, startTick: 100, decayTicks: 12);
            Assert.That(i, Is.EqualTo(0.5f).Within(1e-6f));
        }

        [Test]
        public void FlashIntensity_AtOrBeyondDecayWindow_ReturnsZero()
        {
            float atEnd = AnimePresentationUniforms.ComputeFlashIntensity(
                currentTick: 112, startTick: 100, decayTicks: 12);
            float beyondEnd = AnimePresentationUniforms.ComputeFlashIntensity(
                currentTick: 120, startTick: 100, decayTicks: 12);
            Assert.That(atEnd, Is.EqualTo(0f));
            Assert.That(beyondEnd, Is.EqualTo(0f));
        }

        [Test]
        public void FlashIntensity_NegativeElapsed_ReturnsZero()
        {
            // A future-dated start tick (e.g., bug in trigger code) must
            // not produce a flash; intensity stays zero until the tick
            // catches up.
            float i = AnimePresentationUniforms.ComputeFlashIntensity(
                currentTick: 100, startTick: 200, decayTicks: 12);
            Assert.That(i, Is.EqualTo(0f));
        }

        [Test]
        public void FlashIntensity_NonPositiveDecay_ReturnsZero()
        {
            // Defensive: a 0 or negative decay window is a config bug;
            // never produce a flash that lasts forever.
            float zero = AnimePresentationUniforms.ComputeFlashIntensity(
                currentTick: 105, startTick: 100, decayTicks: 0);
            float neg = AnimePresentationUniforms.ComputeFlashIntensity(
                currentTick: 105, startTick: 100, decayTicks: -5);
            Assert.That(zero, Is.EqualTo(0f));
            Assert.That(neg, Is.EqualTo(0f));
        }

        // ----- Screen-tone strength (symmetric fade envelope) -----

        [Test]
        public void ScreenToneStrength_OutsideWindow_ReturnsZero()
        {
            float before = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 50, startTick: 100, endTick: 160,
                baseStrength: 0.8f, fadeTicks: 6);
            float at = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 160, startTick: 100, endTick: 160,
                baseStrength: 0.8f, fadeTicks: 6);
            float after = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 200, startTick: 100, endTick: 160,
                baseStrength: 0.8f, fadeTicks: 6);
            Assert.That(before, Is.EqualTo(0f));
            Assert.That(at, Is.EqualTo(0f),
                "EndTick is exclusive — currentTick == endTick is outside the window.");
            Assert.That(after, Is.EqualTo(0f));
        }

        [Test]
        public void ScreenToneStrength_AtStart_RampsFromZero()
        {
            // tick == start: elapsed = 0, fadeIn = 0/6 = 0 → envelope = 0.
            float i = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 100, startTick: 100, endTick: 160,
                baseStrength: 0.8f, fadeTicks: 6);
            Assert.That(i, Is.EqualTo(0f).Within(1e-6f));
        }

        [Test]
        public void ScreenToneStrength_AtMidWindow_ReachesBaseStrength()
        {
            // 30 ticks into a 60-tick window with 6-tick fades: well past
            // fade-in end + well before fade-out start → envelope = 1.
            float i = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 130, startTick: 100, endTick: 160,
                baseStrength: 0.8f, fadeTicks: 6);
            Assert.That(i, Is.EqualTo(0.8f).Within(1e-6f));
        }

        [Test]
        public void ScreenToneStrength_DuringFadeIn_RampsLinear()
        {
            // 3 ticks into a 6-tick fade-in: elapsed=3, fadeIn = 3/6 = 0.5.
            // base 1.0 → strength 0.5.
            float i = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 103, startTick: 100, endTick: 160,
                baseStrength: 1f, fadeTicks: 6);
            Assert.That(i, Is.EqualTo(0.5f).Within(1e-6f));
        }

        [Test]
        public void ScreenToneStrength_DuringFadeOut_RampsLinear()
        {
            // tick = 156, end = 160, fadeTicks = 6 → remaining = 4,
            // fadeOut = 4/6 ≈ 0.6667. base 1.0 → ≈0.6667.
            float i = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 156, startTick: 100, endTick: 160,
                baseStrength: 1f, fadeTicks: 6);
            Assert.That(i, Is.EqualTo(4f / 6f).Within(1e-6f));
        }

        [Test]
        public void ScreenToneStrength_BaseZero_ReturnsZero()
        {
            // Reduce-motion path: base strength clamped to 0 by the
            // director should produce 0 regardless of where in the
            // window we sample.
            float i = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 130, startTick: 100, endTick: 160,
                baseStrength: 0f, fadeTicks: 6);
            Assert.That(i, Is.EqualTo(0f));
        }

        [Test]
        public void ScreenToneStrength_OverUnitBase_ClampsToOne()
        {
            // base > 1 (caller bug) must not produce a >1 strength —
            // the shader treats > 1 as full overlay regardless, but the
            // contract says [0, 1].
            float i = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 130, startTick: 100, endTick: 160,
                baseStrength: 1.5f, fadeTicks: 6);
            Assert.That(i, Is.EqualTo(1f).Within(1e-6f));
        }

        [Test]
        public void ScreenToneStrength_InvalidWindow_ReturnsZero()
        {
            // start < 0 sentinel = "no tone active."
            float noActive = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 130, startTick: -1, endTick: 160,
                baseStrength: 0.8f, fadeTicks: 6);
            // end <= start is malformed.
            float malformed = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 130, startTick: 160, endTick: 100,
                baseStrength: 0.8f, fadeTicks: 6);
            Assert.That(noActive, Is.EqualTo(0f));
            Assert.That(malformed, Is.EqualTo(0f));
        }

        [Test]
        public void ScreenToneStrength_ZeroFadeTicks_HoldsBaseAcrossWindow()
        {
            // No-fade configuration: enter the window at full base
            // strength, stay there until exit.
            float i = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick: 100, startTick: 100, endTick: 160,
                baseStrength: 0.7f, fadeTicks: 0);
            Assert.That(i, Is.EqualTo(0.7f).Within(1e-6f));
        }

        // ----- Uniform-name contract (catches accidental rename) -----

        [Test]
        public void UniformNames_AreFW_PrefixedAndStable()
        {
            // The shaders read these literal global-uniform names; a
            // rename here without updating the shader produces a silent
            // never-fires bug.
            Assert.That(AnimePresentationUniforms.FlashIntensityName, Is.EqualTo("_FW_FlashIntensity"));
            Assert.That(AnimePresentationUniforms.ScreenToneStrengthName, Is.EqualTo("_FW_ScreenToneStrength"));
            Assert.That(AnimePresentationUniforms.ElapsedTicksName, Is.EqualTo("_FW_ElapsedTicks"));
        }
    }
}

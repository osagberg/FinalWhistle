using FinalWhistle.Viewer.Adapters.Dots;
using NUnit.Framework;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Slice-7 EditMode tests for <see cref="CadenceTierResolver"/>. Pin
    /// the threshold boundaries + tick-count projections per blueprint
    /// §B Slice 7. These are pure-static helpers; no GameObject / no SO.
    /// </summary>
    public sealed class CadenceTierTests
    {
        // ----- Resolve thresholds (half-open intervals) -----

        [TestCase(0.00f, CadenceTier.Calm)]
        [TestCase(0.10f, CadenceTier.Calm)]
        [TestCase(0.32f, CadenceTier.Calm)]
        [TestCase(0.329999f, CadenceTier.Calm)]
        [TestCase(0.33f, CadenceTier.Standard)]
        [TestCase(0.50f, CadenceTier.Standard)]
        [TestCase(0.66f, CadenceTier.Standard)]
        [TestCase(0.669999f, CadenceTier.Standard)]
        [TestCase(0.67f, CadenceTier.Tension)]
        [TestCase(0.95f, CadenceTier.Tension)]
        [TestCase(1.00f, CadenceTier.Tension)]
        public void Resolve_StakesInBand_ReturnsExpectedTier(float stakes, CadenceTier expected)
        {
            Assert.AreEqual(expected, CadenceTierResolver.Resolve(stakes));
        }

        // ----- Out-of-range + NaN -----

        [Test]
        public void Resolve_NaN_ReturnsStandard()
        {
            Assert.AreEqual(CadenceTier.Standard, CadenceTierResolver.Resolve(float.NaN));
        }

        [Test]
        public void Resolve_Negative_ResolvesToCalm()
        {
            // -0.5 < CalmThreshold → Calm. Clamping behavior is implicit:
            // we don't normalize, we just compare. Useful when a future
            // adapter passes stakes from a model that hasn't clamped yet.
            Assert.AreEqual(CadenceTier.Calm, CadenceTierResolver.Resolve(-0.5f));
        }

        [Test]
        public void Resolve_Above1_ResolvesToTension()
        {
            Assert.AreEqual(CadenceTier.Tension, CadenceTierResolver.Resolve(2.0f));
        }

        // ----- TransitionSecondsForCadence (24/12/6 ticks @60Hz) -----

        [TestCase(CadenceTier.Calm, 24f / 60f)]
        [TestCase(CadenceTier.Standard, 12f / 60f)]
        [TestCase(CadenceTier.Tension, 6f / 60f)]
        public void TransitionSecondsForCadence_ExpectedTickCount_AtSixtyHz(CadenceTier tier, float expectedSeconds)
        {
            float actual = CadenceTierResolver.TransitionSecondsForCadence(tier);
            Assert.AreEqual(expectedSeconds, actual, 1e-6f);
        }

        [Test]
        public void TransitionSecondsForCadence_CalmGreaterThanStandardGreaterThanTension()
        {
            // Sanity invariant: Calm transitions slowest, Tension fastest.
            // Pinning the rank-ordering catches any future tier-coefficient
            // re-tune that accidentally inverts the perceptual contract.
            float calm = CadenceTierResolver.TransitionSecondsForCadence(CadenceTier.Calm);
            float std = CadenceTierResolver.TransitionSecondsForCadence(CadenceTier.Standard);
            float ten = CadenceTierResolver.TransitionSecondsForCadence(CadenceTier.Tension);
            Assert.That(calm, Is.GreaterThan(std));
            Assert.That(std, Is.GreaterThan(ten));
        }

        [Test]
        public void Constants_BlueprintPinned_24_12_6_Ticks()
        {
            // Direct pin of the locked tick counts so any future blueprint
            // re-tune trips this test (the constants live alongside the
            // SPEC entry, not in code-comments alone).
            Assert.AreEqual(24, CadenceTierResolver.CalmTransitionTicks);
            Assert.AreEqual(12, CadenceTierResolver.StandardTransitionTicks);
            Assert.AreEqual(6, CadenceTierResolver.TensionTransitionTicks);
            Assert.AreEqual(0.33f, CadenceTierResolver.CalmThreshold, 1e-6f);
            Assert.AreEqual(0.67f, CadenceTierResolver.StandardThreshold, 1e-6f);
            Assert.AreEqual(60f, CadenceTierResolver.TicksPerSecond, 1e-6f);
        }
    }
}

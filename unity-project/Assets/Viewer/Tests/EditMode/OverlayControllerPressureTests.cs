using FinalWhistle.Viewer.Adapters.Dots;
using NUnit.Framework;
using UnityEngine;
using UnityEngine.UI;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Slice-7 EditMode tests for <see cref="OverlayController.SetPressureTint"/>.
    /// Pins the alpha-only Lerp + clamping + NaN-safe behaviour. UGUI Image
    /// component is spun up via <c>AddComponent</c> on a Canvas
    /// GameObject; the rest of the OverlayController's wiring is left
    /// null because SetPressureTint short-circuits on null pressureIndicatorTint.
    ///
    /// <para>
    /// Note: SetPressureTint short-circuits when no Image is wired, so
    /// the unwired test doesn't actually call SetPressureTint to verify
    /// "no crash" — it just constructs the component on a bare GameObject
    /// and confirms HasPressureIndicator is false.
    /// </para>
    /// </summary>
    public sealed class OverlayControllerPressureTests
    {
        private GameObject host;
        private OverlayController controller;
        private Image tintImage;

        [SetUp]
        public void SetUp()
        {
            host = new GameObject("OverlayControllerPressureTestHost");
            // Canvas → Image so the UI sub-tree is well-formed even
            // though we don't actually render.
            var canvas = host.AddComponent<Canvas>();
            canvas.renderMode = RenderMode.ScreenSpaceOverlay;

            var imgObj = new GameObject("PressureTint");
            imgObj.transform.SetParent(host.transform, worldPositionStays: false);
            tintImage = imgObj.AddComponent<Image>();

            controller = host.AddComponent<OverlayController>();
            // Use reflection to wire the private serialized field —
            // we don't want to expose the inspector field publicly just
            // for tests, and the test layer here is the right place to
            // poke a private SerializeField.
            var fld = typeof(OverlayController).GetField(
                "pressureIndicatorTint",
                System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            fld.SetValue(controller, tintImage);
        }

        [TearDown]
        public void TearDown()
        {
            if (host != null)
            {
                Object.DestroyImmediate(host);
                host = null;
                controller = null;
                tintImage = null;
            }
        }

        // ----- Stakes endpoints -----

        [Test]
        public void SetPressureTint_Zero_AlphaZero()
        {
            controller.SetPressureTint(0f);
            Assert.AreEqual(0f, tintImage.color.a, 1e-4f);
        }

        [Test]
        public void SetPressureTint_One_AlphaMatchesHighStakesColor()
        {
            controller.SetPressureTint(1f);
            // Default pressureTintHighStakes alpha is 80/255 per the
            // serialized default.
            Assert.AreEqual(80f / 255f, tintImage.color.a, 1e-3f);
        }

        // ----- Mid-range Lerp -----

        [Test]
        public void SetPressureTint_Half_AlphaLerpsToMiddle()
        {
            controller.SetPressureTint(0.5f);
            float expected = 0.5f * (80f / 255f);
            Assert.AreEqual(expected, tintImage.color.a, 1e-3f);
        }

        // ----- Clamping -----

        [Test]
        public void SetPressureTint_AboveOne_Clamps()
        {
            controller.SetPressureTint(2.5f);
            Assert.AreEqual(80f / 255f, tintImage.color.a, 1e-3f);
        }

        [Test]
        public void SetPressureTint_Negative_Clamps()
        {
            controller.SetPressureTint(-0.5f);
            Assert.AreEqual(0f, tintImage.color.a, 1e-4f);
        }

        [Test]
        public void SetPressureTint_NaN_TreatsAsZero()
        {
            controller.SetPressureTint(float.NaN);
            Assert.AreEqual(0f, tintImage.color.a, 1e-4f);
        }

        // ----- Hue invariance: only alpha changes -----

        [Test]
        public void SetPressureTint_RGBConstantAcrossStakesValues()
        {
            controller.SetPressureTint(0.2f);
            Color low = tintImage.color;
            controller.SetPressureTint(0.8f);
            Color high = tintImage.color;
            Assert.AreEqual(low.r, high.r, 1e-4f);
            Assert.AreEqual(low.g, high.g, 1e-4f);
            Assert.AreEqual(low.b, high.b, 1e-4f);
            Assert.That(high.a, Is.GreaterThan(low.a));
        }
    }
}

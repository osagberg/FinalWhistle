using System;
using System.Reflection;
using FinalWhistle.Viewer.Adapters.Dots;
using FinalWhistle.Viewer.Contracts;
using FinalWhistle.Viewer.Core;
using NUnit.Framework;
using UnityEngine;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Slice-4 Codex round-1 finding 2 closure: bridge-emitted
    /// <see cref="ShotCategory"/>s that aren't catalog-wired must throw
    /// loudly rather than silently falling back to TacticalWide. Also
    /// pins the existing Initialize-time TacticalWide-required gate.
    /// </summary>
    public sealed class DotsAdapterRootTests
    {
        private GameObject host;
        private DotsAdapterRoot root;
        private ShotTypeSO tacticalWide;

        [SetUp]
        public void SetUp()
        {
            host = new GameObject("test-adapter-root");
            root = host.AddComponent<DotsAdapterRoot>();
            tacticalWide = CreateShot(ShotTypeCatalog.ShotTacticalWide);
        }

        [TearDown]
        public void TearDown()
        {
            if (host != null) UnityEngine.Object.DestroyImmediate(host);
            if (tacticalWide != null) UnityEngine.Object.DestroyImmediate(tacticalWide);
        }

        [Test]
        public void ResolveShot_UnregisteredBridgeEmittedCategory_ThrowsLoudly()
        {
            // Catalog wires only TacticalWide. EventBridge emits
            // ShotCategory.PlayerIsolation for KeyEventKind.SignatureExecuted_LowCutback;
            // before the fix the call silently fell back to TacticalWide.
            // After the fix it throws + names the missing category.
            SetField(root, "shotCatalog", new[] { tacticalWide });
            root.Initialize(new PitchView());

            InvalidOperationException ex = Assert.Throws<InvalidOperationException>(
                () => root.ResolveShot(ShotCategory.PlayerIsolation));
            Assert.That(ex.Message, Does.Contain("PlayerIsolation"));
        }

        [Test]
        public void ResolveShot_UnregisteredAftermathFreeze_ThrowsLoudly()
        {
            // EventBridge also emits ShotCategory.AftermathFreeze for
            // KeyEventKind.SignatureBreakthrough — Codex called out
            // PlayerIsolation but the same silent-fallback hole applied
            // to AftermathFreeze. Pin both.
            SetField(root, "shotCatalog", new[] { tacticalWide });
            root.Initialize(new PitchView());

            InvalidOperationException ex = Assert.Throws<InvalidOperationException>(
                () => root.ResolveShot(ShotCategory.AftermathFreeze));
            Assert.That(ex.Message, Does.Contain("AftermathFreeze"));
        }

        [Test]
        public void Initialize_MissingTacticalWide_ThrowsLoudly()
        {
            // Existing baseline gate stays green: TacticalWide is the
            // documented baseline + must be present at Initialize time.
            ShotTypeSO diagonal = CreateShot(ShotTypeCatalog.ShotDiagonalAttackLane);
            try
            {
                SetField(root, "shotCatalog", new[] { diagonal });
                Assert.Throws<InvalidOperationException>(
                    () => root.Initialize(new PitchView()));
            }
            finally
            {
                UnityEngine.Object.DestroyImmediate(diagonal);
            }
        }

        [Test]
        public void ResolveShot_RegisteredCategory_ReturnsAuthoredAsset()
        {
            // Sanity: the happy path still works after the silent-fallback
            // removal — registered categories resolve to their authored SO.
            SetField(root, "shotCatalog", new[] { tacticalWide });
            root.Initialize(new PitchView());

            ShotTypeSO resolved = root.ResolveShot(ShotCategory.TacticalWide);
            Assert.That(resolved, Is.SameAs(tacticalWide));
        }

        // ----- Helpers -----

        private static ShotTypeSO CreateShot(string id)
        {
            var so = ScriptableObject.CreateInstance<ShotTypeSO>();
            SetField(so, "shotTypeId", id);
            SetField(so, "orthographicSize", 38f);
            SetField(so, "tiltDegrees", 90f);
            SetField(so, "transitionTicks", 12);
            return so;
        }

        private static void SetField(object target, string name, object value)
        {
            FieldInfo field = target.GetType().GetField(name,
                BindingFlags.Instance | BindingFlags.NonPublic | BindingFlags.Public);
            if (field == null)
            {
                throw new InvalidOperationException(
                    $"Field '{name}' not found on {target.GetType().Name}.");
            }
            field.SetValue(target, value);
        }
    }
}

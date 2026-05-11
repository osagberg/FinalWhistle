using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Contracts;
using FinalWhistle.Viewer.Core;
using NUnit.Framework;
using UnityEngine;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Slice-7 EditMode tests for
    /// <see cref="FinalWhistle.Viewer.Adapters.Dots.MotionLineEmitter"/>.
    /// Pin burst-count + reduce-motion-skip + fade behaviour via the
    /// internal <c>ActiveLineCount</c> diagnostic surface. Construction
    /// requires a <c>DotPool</c> + <c>PitchView</c>; the pool is
    /// minimally initialized through reflection so EmitBurst's
    /// IndexForFocalSubject + GetChild lookups resolve to valid
    /// transforms.
    /// </summary>
    public sealed class MotionLineEmitterTests
    {
        private GameObject host;
        private FinalWhistle.Viewer.Adapters.Dots.MotionLineEmitter emitter;
        private FinalWhistle.Viewer.Adapters.Dots.DotPool pool;
        private PitchView pitch;
        private Sprite stubSprite;

        [SetUp]
        public void SetUp()
        {
            host = new GameObject("MotionLineEmitterTestHost");

            // Sprite stub: 2×16 white texture matching motion_line.png shape.
            var tex = new Texture2D(2, 16, TextureFormat.RGBA32, false);
            stubSprite = Sprite.Create(tex, new Rect(0, 0, 2, 16), new Vector2(0.5f, 0.5f), 16f);

            // DotPool stub: needs to be initialized + have child dots.
            var poolObj = new GameObject("DotPool");
            poolObj.transform.SetParent(host.transform);
            pool = poolObj.AddComponent<FinalWhistle.Viewer.Adapters.Dots.DotPool>();
            // Spawn 23 child placeholders so transform.GetChild(idx)
            // resolves.
            for (int i = 0; i < FinalWhistle.Viewer.Adapters.Dots.DotPool.TotalDots; i++)
            {
                var dot = new GameObject($"Dot_{i}");
                dot.transform.SetParent(pool.transform, worldPositionStays: false);
                dot.transform.position = new Vector3(i, 0f, 0f);
            }
            // Force the pool's internal SpriteRenderer[] dots field to
            // non-null so DotPool.IndexForFocalSubject's contracts don't
            // need to be ensured — but IndexForFocalSubject is pure string
            // parsing + does NOT check initialization, so we don't actually
            // need to fake it. EmitBurst uses dotPool.IndexForFocalSubject
            // (pure) + dotPool.transform.GetChild (the children we created
            // above).

            pitch = new PitchView(pitchLengthMeters: 105f, pitchWidthMeters: 68f, origin: Vector3.zero);

            emitter = host.AddComponent<FinalWhistle.Viewer.Adapters.Dots.MotionLineEmitter>();
            var lineSpriteFld = typeof(FinalWhistle.Viewer.Adapters.Dots.MotionLineEmitter)
                .GetField("lineSprite",
                          System.Reflection.BindingFlags.NonPublic | System.Reflection.BindingFlags.Instance);
            lineSpriteFld.SetValue(emitter, stubSprite);

            emitter.Initialize(pool, pitch);
        }

        [TearDown]
        public void TearDown()
        {
            if (host != null)
            {
                Object.DestroyImmediate(host);
                host = null;
                emitter = null;
                pool = null;
                pitch = null;
            }
            if (stubSprite != null)
            {
                Object.DestroyImmediate(stubSprite.texture);
                Object.DestroyImmediate(stubSprite);
                stubSprite = null;
            }
        }

        private ActiveViewerEvent BuildActiveEvent(string focalSubject, bool reduceMotion)
        {
            // Use player-isolation for both base + effective so the
            // reduceMotionApplied flag drives our reduce-motion test
            // (otherwise constructor rejects mismatched id pairs).
            string shotId = reduceMotion
                ? ShotTypeCatalog.ShotPlayerIsolationReduceMotion
                : ShotTypeCatalog.ShotPlayerIsolation;
            var evt = new ViewerEvent(
                viewerEventId: 1,
                sourceEventId: 1,
                sourceEventOrdinal: 0,
                baseShotTypeId: ShotTypeCatalog.ShotPlayerIsolation,
                effectiveShotTypeId: shotId,
                reduceMotionApplied: reduceMotion,
                startTick: new Tick(100),
                endTick: new Tick(130),
                seed: Seed.FromUInt64(1),
                stakesNormalized: Fixed.One,
                memoryRelevance: Fixed.Zero,
                focalSubject: focalSubject,
                participantPlayerIds: System.Array.Empty<string>(),
                memoryHits: System.Array.Empty<MemoryHit>(),
                sourceEventClass: EventClass.GoalScored,
                sourceEntityId: null);
            return new ActiveViewerEvent(evt, elapsedTicks: 0);
        }

        // ----- EmitBurst happy path -----

        [Test]
        public void EmitBurst_ValidFocal_ActivatesLinesPerBurst()
        {
            ActiveViewerEvent active = BuildActiveEvent("viewer.focal:home.6", reduceMotion: false);
            emitter.EmitBurst(active, currentTick: 0);
            Assert.AreEqual(
                FinalWhistle.Viewer.Adapters.Dots.MotionLineEmitter.LinesPerBurst,
                emitter.ActiveLineCount);
        }

        // ----- Reduce-motion skip -----

        [Test]
        public void EmitBurst_ReduceMotionApplied_SkipsEmission()
        {
            ActiveViewerEvent active = BuildActiveEvent("viewer.focal:home.6", reduceMotion: true);
            emitter.EmitBurst(active, currentTick: 0);
            Assert.AreEqual(0, emitter.ActiveLineCount);
        }

        // ----- Invalid focal -----

        [Test]
        public void EmitBurst_NullFocal_SkipsEmission()
        {
            ActiveViewerEvent active = BuildActiveEvent(null, reduceMotion: false);
            emitter.EmitBurst(active, currentTick: 0);
            Assert.AreEqual(0, emitter.ActiveLineCount);
        }

        [Test]
        public void EmitBurst_MalformedFocal_SkipsEmission()
        {
            ActiveViewerEvent active = BuildActiveEvent("garbage", reduceMotion: false);
            emitter.EmitBurst(active, currentTick: 0);
            Assert.AreEqual(0, emitter.ActiveLineCount);
        }

        // ----- Fade lifecycle -----

        [Test]
        public void Tick_AfterFadeTicks_AllLinesRetire()
        {
            ActiveViewerEvent active = BuildActiveEvent("viewer.focal:home.6", reduceMotion: false);
            emitter.EmitBurst(active, currentTick: 0);
            Assert.AreEqual(
                FinalWhistle.Viewer.Adapters.Dots.MotionLineEmitter.LinesPerBurst,
                emitter.ActiveLineCount);

            // FadeTicks=18; advance just past it.
            emitter.Tick(FinalWhistle.Viewer.Adapters.Dots.MotionLineEmitter.FadeTicks);
            Assert.AreEqual(0, emitter.ActiveLineCount,
                "All burst lines should retire at FadeTicks elapsed.");
        }

        [Test]
        public void Tick_MidFade_LinesStillActive()
        {
            ActiveViewerEvent active = BuildActiveEvent("viewer.focal:home.6", reduceMotion: false);
            emitter.EmitBurst(active, currentTick: 0);
            emitter.Tick(currentTick: 5);
            Assert.AreEqual(
                FinalWhistle.Viewer.Adapters.Dots.MotionLineEmitter.LinesPerBurst,
                emitter.ActiveLineCount,
                "Lines remain visible until FadeTicks elapsed.");
        }

        [Test]
        public void EmitBurst_NullActive_Throws()
        {
            Assert.Throws<System.ArgumentNullException>(() => emitter.EmitBurst(null, 0));
        }
    }
}

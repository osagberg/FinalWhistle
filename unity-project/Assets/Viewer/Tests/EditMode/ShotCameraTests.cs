using System;
using System.Reflection;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Adapters.Dots;
using FinalWhistle.Viewer.Contracts;
using FinalWhistle.Viewer.Core;
using NUnit.Framework;
using UnityEngine;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Slice-4 Codex round-1 finding 1 closure: ShotCamera adapter-shot
    /// API must be idempotent (same-shot calls don't restart the Lerp)
    /// and must return-to-default when the heuristic stops asking for an
    /// adapter shot. Also pins active-shot suppression behaviour so a
    /// sustained heuristic survives a transient event-driven interruption.
    /// </summary>
    public sealed class ShotCameraTests
    {
        private const string TacticalWideId = ShotTypeCatalog.ShotTacticalWide;
        private const string DiagonalLaneId = ShotTypeCatalog.ShotDiagonalAttackLane;
        private const string PassShotImpactId = ShotTypeCatalog.ShotPassShotImpact;

        private GameObject host;
        private Camera camera;
        private DotPool pool;
        private ShotCamera shotCamera;
        private ShotTypeSO tacticalWide;
        private ShotTypeSO diagonalLane;
        private ShotTypeSO passShotImpact;
        private PitchView pitch;

        [SetUp]
        public void SetUp()
        {
            host = new GameObject("test-shotcam");
            camera = host.AddComponent<Camera>();
            // DotPool is a MonoBehaviour requirement of ShotCamera but
            // none of the public methods exercised by these tests drive
            // LateUpdate / BallWorldPosition, so an uninitialised DotPool
            // satisfies the not-null gate without further setup.
            pool = host.AddComponent<DotPool>();
            shotCamera = host.AddComponent<ShotCamera>();

            tacticalWide = CreateShot(TacticalWideId, ortho: 38f, tilt: 90f);
            diagonalLane = CreateShot(DiagonalLaneId, ortho: 20f, tilt: 75f);
            passShotImpact = CreateShot(PassShotImpactId, ortho: 12f, tilt: 80f);

            SetField(shotCamera, "targetCamera", camera);
            SetField(shotCamera, "dotPool", pool);
            SetField(shotCamera, "defaultShot", tacticalWide);

            pitch = new PitchView();
            shotCamera.Initialize(pitch);
        }

        [TearDown]
        public void TearDown()
        {
            if (host != null) UnityEngine.Object.DestroyImmediate(host);
            if (tacticalWide != null) UnityEngine.Object.DestroyImmediate(tacticalWide);
            if (diagonalLane != null) UnityEngine.Object.DestroyImmediate(diagonalLane);
            if (passShotImpact != null) UnityEngine.Object.DestroyImmediate(passShotImpact);
        }

        // ----- Idempotency contract (Codex round-1 Slice-4 finding 1) -----

        [Test]
        public void SetAdapterShot_SameShotRepeated_DoesNotRestartTransition()
        {
            // First call captures from-state + starts a transition. The
            // next 59 calls with the same shot must be no-ops: this is the
            // sustained-over-threshold regression that a real diagonal-
            // attack-lane heuristic produces every FixedUpdate tick.
            for (int i = 0; i < 60; i++)
            {
                shotCamera.SetAdapterShot(diagonalLane);
            }
            Assert.That(shotCamera.TransitionStartCount, Is.EqualTo(1));
            Assert.That(shotCamera.TargetShot, Is.SameAs(diagonalLane));
            Assert.That(shotCamera.CurrentAdapterShot, Is.SameAs(diagonalLane));
        }

        [Test]
        public void SetAdapterShot_OverThresholdThenBelow_ReturnsToDefault()
        {
            // Codex's explicit regression shape: sustained >8m/s motion
            // followed by below-threshold motion. Two transitions only:
            // tactical-wide → diagonal-attack-lane (entry) and
            // diagonal-attack-lane → tactical-wide (exit).
            for (int i = 0; i < 30; i++)
            {
                shotCamera.SetAdapterShot(diagonalLane);
            }
            for (int i = 0; i < 30; i++)
            {
                shotCamera.SetAdapterShot(null);
            }
            Assert.That(shotCamera.TransitionStartCount, Is.EqualTo(2));
            Assert.That(shotCamera.TargetShot, Is.SameAs(tacticalWide));
            Assert.That(shotCamera.CurrentAdapterShot, Is.Null);
        }

        [Test]
        public void SetAdapterShot_NullWhenAlreadyClear_IsNoop()
        {
            // Initialize leaves currentAdapterShot=null. SetAdapterShot(null)
            // immediately after Initialize must NOT count as a transition.
            shotCamera.SetAdapterShot(null);
            Assert.That(shotCamera.TransitionStartCount, Is.EqualTo(0));
            Assert.That(shotCamera.TargetShot, Is.SameAs(tacticalWide));
        }

        // ----- Active-shot suppression survives event interruption -----

        [Test]
        public void SetAdapterShot_DuringEventDrivenShot_RecordsButDoesNotTransition()
        {
            // BeginShot starts a transition (count=1). SetAdapterShot is
            // suppressed (count stays 1) but currentAdapterShot is recorded
            // so OnSimTick can resume to it on event-end.
            ActiveViewerEvent active = BuildActiveEvent(passShotImpact, startTickValue: 100, endTickValue: 160);
            shotCamera.BeginShot(passShotImpact, active);
            Assert.That(shotCamera.TransitionStartCount, Is.EqualTo(1));
            Assert.That(shotCamera.TargetShot, Is.SameAs(passShotImpact));

            shotCamera.SetAdapterShot(diagonalLane);
            Assert.That(shotCamera.TransitionStartCount, Is.EqualTo(1),
                "SetAdapterShot must not start a transition while an event-driven shot is active.");
            Assert.That(shotCamera.TargetShot, Is.SameAs(passShotImpact),
                "Active event-driven shot must keep rendering during suppression.");
            Assert.That(shotCamera.CurrentAdapterShot, Is.SameAs(diagonalLane),
                "Suppressed adapter shot must be recorded for resume on event-end.");
        }

        [Test]
        public void OnSimTick_AfterEventEnd_ResumesToRecordedAdapterShot()
        {
            ActiveViewerEvent active = BuildActiveEvent(passShotImpact, startTickValue: 100, endTickValue: 160);
            shotCamera.BeginShot(passShotImpact, active);
            shotCamera.SetAdapterShot(diagonalLane);

            // Tick advances past EndTick: OnSimTick must transition back to
            // the recorded adapter shot, NOT to the default.
            shotCamera.OnSimTick(Tick.Zero + 160L);
            Assert.That(shotCamera.TargetShot, Is.SameAs(diagonalLane));
            Assert.That(shotCamera.TransitionStartCount, Is.EqualTo(2),
                "Resume to recorded adapter shot must count as a transition.");
        }

        [Test]
        public void OnSimTick_AfterEventEnd_ResumesToDefaultWhenNoAdapterShotRecorded()
        {
            ActiveViewerEvent active = BuildActiveEvent(passShotImpact, startTickValue: 100, endTickValue: 160);
            shotCamera.BeginShot(passShotImpact, active);
            shotCamera.OnSimTick(Tick.Zero + 160L);
            Assert.That(shotCamera.TargetShot, Is.SameAs(tacticalWide));
            Assert.That(shotCamera.TransitionStartCount, Is.EqualTo(2));
        }

        // ----- Slice-7 finding #3: ShotEnded event fires on canonical retire -----

        [Test]
        public void OnSimTick_AfterEventEnd_FiresShotEndedEvent()
        {
            // Codex 2026-05-11 finding #3 regression: SelectionRing needs
            // a hook to disengage when the active event-driven shot retires
            // on tick expiry. The ShotEnded event is that hook.
            int firedCount = 0;
            shotCamera.ShotEnded += () => firedCount++;

            ActiveViewerEvent active = BuildActiveEvent(passShotImpact, startTickValue: 100, endTickValue: 160);
            shotCamera.BeginShot(passShotImpact, active);
            Assert.AreEqual(0, firedCount, "Begin must not raise ShotEnded.");

            // Pre-expiry tick: still active, no event.
            shotCamera.OnSimTick(Tick.Zero + 150L);
            Assert.AreEqual(0, firedCount, "Pre-expiry OnSimTick must not raise ShotEnded.");

            // Expiry: ShotEnded fires exactly once.
            shotCamera.OnSimTick(Tick.Zero + 160L);
            Assert.AreEqual(1, firedCount, "Expiry must raise ShotEnded exactly once.");

            // Post-expiry: no further fires.
            shotCamera.OnSimTick(Tick.Zero + 161L);
            Assert.AreEqual(1, firedCount, "Post-expiry OnSimTick must not re-raise.");
        }

        // ----- Slice-7 cadence tier wiring: BeginShot resolves tier from stakes -----

        [Test]
        public void BeginShot_LowStakes_ResolvesCalmCadence()
        {
            ActiveViewerEvent active = BuildActiveEventWithStakes(passShotImpact, stakesRaw: Fixed.OneRaw / 5L);  // ~0.2 → Calm
            shotCamera.BeginShot(passShotImpact, active);
            Assert.AreEqual(CadenceTier.Calm, shotCamera.LastResolvedCadenceTier);
        }

        [Test]
        public void BeginShot_MidStakes_ResolvesStandardCadence()
        {
            ActiveViewerEvent active = BuildActiveEventWithStakes(passShotImpact, stakesRaw: Fixed.OneRaw / 2L);  // 0.5 → Standard
            shotCamera.BeginShot(passShotImpact, active);
            Assert.AreEqual(CadenceTier.Standard, shotCamera.LastResolvedCadenceTier);
        }

        [Test]
        public void BeginShot_HighStakes_ResolvesTensionCadence()
        {
            ActiveViewerEvent active = BuildActiveEventWithStakes(passShotImpact, stakesRaw: Fixed.OneRaw * 9L / 10L);  // 0.9 → Tension
            shotCamera.BeginShot(passShotImpact, active);
            Assert.AreEqual(CadenceTier.Tension, shotCamera.LastResolvedCadenceTier);
        }

        private static ActiveViewerEvent BuildActiveEventWithStakes(ShotTypeSO shot, long stakesRaw)
        {
            ViewerEvent ev = new(
                viewerEventId: 1UL,
                sourceEventId: 1UL,
                sourceEventOrdinal: 0,
                baseShotTypeId: shot.ShotTypeId,
                effectiveShotTypeId: shot.ShotTypeId,
                reduceMotionApplied: false,
                startTick: Tick.Zero + 100L,
                endTick: Tick.Zero + 160L,
                seed: Seed.Zero,
                stakesNormalized: Fixed.FromRaw(stakesRaw),
                memoryRelevance: Fixed.Zero,
                focalSubject: null,
                participantPlayerIds: Array.Empty<string>(),
                memoryHits: Array.Empty<MemoryHit>(),
                sourceEventClass: EventClass.GoalScored,
                sourceEntityId: null,
                signatureMetadata: null);
            return new ActiveViewerEvent(ev, elapsedTicks: 0);
        }

        // ----- Helpers -----

        private static ShotTypeSO CreateShot(string id, float ortho, float tilt)
        {
            var so = ScriptableObject.CreateInstance<ShotTypeSO>();
            SetField(so, "shotTypeId", id);
            SetField(so, "orthographicSize", ortho);
            SetField(so, "tiltDegrees", tilt);
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

        private static ActiveViewerEvent BuildActiveEvent(ShotTypeSO shot, long startTickValue, long endTickValue)
        {
            ViewerEvent ev = new(
                viewerEventId: 1UL,
                sourceEventId: 1UL,
                sourceEventOrdinal: 0,
                baseShotTypeId: shot.ShotTypeId,
                effectiveShotTypeId: shot.ShotTypeId,
                reduceMotionApplied: false,
                startTick: Tick.Zero + startTickValue,
                endTick: Tick.Zero + endTickValue,
                seed: Seed.Zero,
                stakesNormalized: Fixed.Zero,
                memoryRelevance: Fixed.Zero,
                focalSubject: null,
                participantPlayerIds: Array.Empty<string>(),
                memoryHits: Array.Empty<MemoryHit>(),
                sourceEventClass: EventClass.GoalScored,
                sourceEntityId: null,
                signatureMetadata: null);
            return new ActiveViewerEvent(ev, elapsedTicks: 0);
        }
    }
}

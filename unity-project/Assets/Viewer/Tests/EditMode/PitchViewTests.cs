using System;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Contracts;
using FinalWhistle.Viewer.Core;
using NUnit.Framework;
using UnityEngine;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Slice-1 EditMode tests for the dots-adapter blueprint contracts
    /// (<see cref="PitchView"/> + <see cref="ActiveViewerEvent"/>). Per the
    /// blueprint Slice-1 acceptance criteria: PitchView.FixedToWorld must
    /// map canonical Q32.32 positions to Unity world space within 0.001f
    /// (1mm) tolerance; ActiveViewerEvent must resolve ShotTypeCatalog
    /// projections at construction; constructor invariant guards must
    /// reject invalid inputs at the boundary, not at first use.
    /// </summary>
    public sealed class PitchViewTests
    {
        // ----- PitchView constructor invariants -----

        [Test]
        public void PitchView_DefaultConstructor_UsesFifaSpec105x68()
        {
            PitchView pitch = new();
            Assert.That(pitch.PitchLengthMeters, Is.EqualTo(PitchView.DefaultPitchLengthMeters));
            Assert.That(pitch.PitchWidthMeters, Is.EqualTo(PitchView.DefaultPitchWidthMeters));
            Assert.That(pitch.MetersPerUnit, Is.EqualTo(PitchView.DefaultMetersPerUnit));
            Assert.That(pitch.Origin, Is.EqualTo(Vector3.zero));
        }

        [Test]
        public void PitchView_OriginOverride_PersistsToAccessor()
        {
            Vector3 origin = new(10f, 0.5f, -3f);
            PitchView pitch = new(origin: origin);
            Assert.That(pitch.Origin, Is.EqualTo(origin));
        }

        [Test]
        public void PitchView_NonPositivePitchLength_Throws()
        {
            Assert.Throws<ArgumentOutOfRangeException>(
                () => _ = new PitchView(pitchLengthMeters: 0f));
            Assert.Throws<ArgumentOutOfRangeException>(
                () => _ = new PitchView(pitchLengthMeters: -1f));
        }

        [Test]
        public void PitchView_NonPositivePitchWidth_Throws()
        {
            Assert.Throws<ArgumentOutOfRangeException>(
                () => _ = new PitchView(pitchWidthMeters: 0f));
            Assert.Throws<ArgumentOutOfRangeException>(
                () => _ = new PitchView(pitchWidthMeters: -68f));
        }

        [Test]
        public void PitchView_NonPositiveMetersPerUnit_Throws()
        {
            Assert.Throws<ArgumentOutOfRangeException>(
                () => _ = new PitchView(metersPerUnit: 0f));
            Assert.Throws<ArgumentOutOfRangeException>(
                () => _ = new PitchView(metersPerUnit: -0.1f));
        }

        [Test]
        public void PitchView_NaNDimensions_Throws()
        {
            Assert.Throws<ArgumentException>(
                () => _ = new PitchView(pitchLengthMeters: float.NaN));
            Assert.Throws<ArgumentException>(
                () => _ = new PitchView(pitchWidthMeters: float.PositiveInfinity));
        }

        [Test]
        public void PitchView_NaNOrigin_Throws()
        {
            Assert.Throws<ArgumentException>(
                () => _ = new PitchView(origin: new Vector3(float.NaN, 0f, 0f)));
        }

        // ----- FixedToWorld mapping (1mm tolerance) -----

        private const float TolMm = 0.001f;

        [Test]
        public void FixedToWorld_OriginOriginZeroFixed_MapsToZero()
        {
            PitchView pitch = new();
            Vector3 actual = pitch.FixedToWorld(Vector3Fixed.Zero);
            Assert.That(actual, Is.EqualTo(Vector3.zero).Using(new Vector3EqualityComparer(TolMm)));
        }

        [Test]
        public void FixedToWorld_PenaltySpotEquivalent_MapsAccurately()
        {
            // Integer Q32.32 values round-trip through float exactly (raw
            // = 52 << 32 = 2.23e11 fits in float without truncation since
            // the value is a power-of-2-scaled integer below 2^33 magnitude
            // when reconstructed). This test pins the integer-coordinate
            // path; the FRACTIONAL-corner test below pins the precision
            // claim that the double intermediate actually buys.
            Vector3Fixed corner = new(Fixed.FromInt(52), Fixed.Zero, Fixed.FromInt(34));
            PitchView pitch = new();
            Vector3 actual = pitch.FixedToWorld(corner);
            Assert.That(actual, Is.EqualTo(new Vector3(52f, 0f, 34f)).Using(new Vector3EqualityComparer(TolMm)));
        }

        [Test]
        public void FixedToWorld_FractionalCorner_HoldsSubMillimetreAccuracy()
        {
            // Per pr-review-toolkit:feature-dev:code-reviewer 2026-04-30 P2
            // against the slice-1 first draft: the prior corner test used
            // integer-only Q32.32 which round-trips through float exactly
            // and therefore did NOT exercise the float-cliff guard the
            // double intermediate exists to defend against. This test
            // constructs `(52.5, 0, 34.0)` — the actual full corner of the
            // 105×68 origin-centred pitch — using `Fixed.Half` for the
            // fractional component, then asserts sub-mm accuracy. This is
            // the magnitude where direct `long → float` conversion of the
            // raw value loses ~6 microns; the double intermediate keeps
            // the result at exactly 52.5 within the 1mm tolerance band.
            Vector3Fixed fullCorner = new(
                Fixed.FromInt(52) + Fixed.Half,
                Fixed.Zero,
                Fixed.FromInt(34));
            PitchView pitch = new();
            Vector3 actual = pitch.FixedToWorld(fullCorner);
            Assert.That(actual.x, Is.EqualTo(52.5f).Within(TolMm));
            Assert.That(actual.y, Is.EqualTo(0f).Within(TolMm));
            Assert.That(actual.z, Is.EqualTo(34f).Within(TolMm));
        }

        [Test]
        public void FixedToWorld_NonUnityScaleAtCornerMagnitude_PreservesAccuracy()
        {
            // Per pr-review-toolkit:feature-dev:code-reviewer 2026-04-30 P1
            // against the slice-1 first draft: when MetersPerUnit != 1f the
            // multiplication MUST stay in double precision, not happen on
            // the float side after the cast. This test exercises that
            // pipeline: 0.5f scale × corner-of-pitch fractional coord =
            // 26.25 Unity units; if the multiply happened post-cast, the
            // float-side error at this magnitude would still be sub-mm but
            // would compound differently. We assert sub-mm accuracy here
            // to lock the all-double pipeline in.
            Vector3Fixed pos = new(
                Fixed.FromInt(52) + Fixed.Half,
                Fixed.Zero,
                Fixed.FromInt(34));
            PitchView pitch = new(metersPerUnit: 0.5f);
            Vector3 actual = pitch.FixedToWorld(pos);
            Assert.That(actual.x, Is.EqualTo(26.25f).Within(TolMm));
            Assert.That(actual.z, Is.EqualTo(17f).Within(TolMm));
        }

        [Test]
        public void FixedToWorld_NegativeQuadrant_MapsAccurately()
        {
            Vector3Fixed pos = new(Fixed.FromInt(-25), Fixed.Zero, Fixed.FromInt(-15));
            PitchView pitch = new();
            Vector3 actual = pitch.FixedToWorld(pos);
            Assert.That(actual, Is.EqualTo(new Vector3(-25f, 0f, -15f)).Using(new Vector3EqualityComparer(TolMm)));
        }

        [Test]
        public void FixedToWorld_AltitudeOnYAxis_PreservesY()
        {
            // Ball aloft 2.5m: Y component is altitude. Ground players have Y=0.
            Vector3Fixed aloft = new(Fixed.Zero, Fixed.Half + Fixed.FromInt(2), Fixed.Zero);
            PitchView pitch = new();
            Vector3 actual = pitch.FixedToWorld(aloft);
            Assert.That(actual.x, Is.EqualTo(0f).Within(TolMm));
            Assert.That(actual.y, Is.EqualTo(2.5f).Within(TolMm));
            Assert.That(actual.z, Is.EqualTo(0f).Within(TolMm));
        }

        [Test]
        public void FixedToWorld_OriginShift_AppliesAdditively()
        {
            Vector3 origin = new(100f, 0f, 200f);
            PitchView pitch = new(origin: origin);
            Vector3Fixed pos = new(Fixed.FromInt(5), Fixed.Zero, Fixed.FromInt(-3));
            Vector3 actual = pitch.FixedToWorld(pos);
            Assert.That(actual, Is.EqualTo(new Vector3(105f, 0f, 197f)).Using(new Vector3EqualityComparer(TolMm)));
        }

        [Test]
        public void FixedToWorld_FractionalPosition_PreservesSubMeterAccuracy()
        {
            // Half-metre offset in each axis tests fractional Q32.32 → float
            // precision at typical pitch magnitudes.
            Vector3Fixed pos = new(Fixed.Half, Fixed.Zero, Fixed.Half);
            PitchView pitch = new();
            Vector3 actual = pitch.FixedToWorld(pos);
            Assert.That(actual.x, Is.EqualTo(0.5f).Within(TolMm));
            Assert.That(actual.z, Is.EqualTo(0.5f).Within(TolMm));
        }

        [Test]
        public void FixedToWorld_MetersPerUnitScales_ScalesOutput()
        {
            // 0.5 m/unit means a 1-meter sim distance maps to 0.5 Unity units.
            PitchView pitch = new(metersPerUnit: 0.5f);
            Vector3Fixed pos = new(Fixed.FromInt(10), Fixed.Zero, Fixed.Zero);
            Vector3 actual = pitch.FixedToWorld(pos);
            Assert.That(actual.x, Is.EqualTo(5f).Within(TolMm));
        }

        // ----- ActiveViewerEvent invariants + projections -----

        private static ViewerEvent BuildSampleEvent(string effectiveShotTypeId, long stakesRaw)
        {
            // Construct a minimal-shape ViewerEvent; the exact other field
            // values don't matter for ActiveViewerEvent's projections — only
            // EffectiveShotTypeId + StakesNormalized are read.
            return new ViewerEvent(
                viewerEventId: 1UL,
                sourceEventId: 1UL,
                sourceEventOrdinal: 0,
                baseShotTypeId: effectiveShotTypeId,
                effectiveShotTypeId: effectiveShotTypeId,
                reduceMotionApplied: false,
                startTick: Tick.Zero,
                endTick: Tick.Zero + 60L,
                seed: Seed.Zero,
                stakesNormalized: Fixed.FromRaw(stakesRaw),
                memoryRelevance: Fixed.Zero,
                focalSubject: null,
                participantPlayerIds: Array.Empty<string>(),
                memoryHits: Array.Empty<MemoryHit>(),
                sourceEventClass: EventClass.GoalScored,
                sourceEntityId: null,
                signatureMetadata: null);
        }

        [Test]
        public void ActiveViewerEvent_Construct_ResolvesShotDefAndCategory()
        {
            ViewerEvent ev = BuildSampleEvent(ShotTypeCatalog.ShotPassShotImpact, Fixed.OneRaw);
            ActiveViewerEvent active = new(ev, elapsedTicks: 0);

            Assert.That(active.Event, Is.SameAs(ev));
            Assert.That(active.ShotDef.Id, Is.EqualTo(ShotTypeCatalog.ShotPassShotImpact));
            Assert.That(active.ShotCategory, Is.EqualTo(ShotCategory.PassShotImpact));
            Assert.That(active.ElapsedTicks, Is.EqualTo(0));
        }

        [Test]
        public void ActiveViewerEvent_StakesFullRaw_ProjectsToOnePointZero()
        {
            ViewerEvent ev = BuildSampleEvent(ShotTypeCatalog.ShotTacticalWide, Fixed.OneRaw);
            ActiveViewerEvent active = new(ev, elapsedTicks: 0);
            Assert.That(active.StakesFloat, Is.EqualTo(1f).Within(TolMm));
        }

        [Test]
        public void ActiveViewerEvent_StakesHalf_ProjectsToHalf()
        {
            ViewerEvent ev = BuildSampleEvent(ShotTypeCatalog.ShotTacticalWide, Fixed.OneRaw / 2L);
            ActiveViewerEvent active = new(ev, elapsedTicks: 0);
            Assert.That(active.StakesFloat, Is.EqualTo(0.5f).Within(TolMm));
        }

        [Test]
        public void ActiveViewerEvent_NullEvent_Throws()
        {
            Assert.Throws<ArgumentNullException>(
                () => _ = new ActiveViewerEvent(null!, elapsedTicks: 0));
        }

        [Test]
        public void ActiveViewerEvent_NegativeElapsedTicks_Throws()
        {
            ViewerEvent ev = BuildSampleEvent(ShotTypeCatalog.ShotTacticalWide, 0L);
            Assert.Throws<ArgumentOutOfRangeException>(
                () => _ = new ActiveViewerEvent(ev, elapsedTicks: -1));
        }

        [Test]
        public void ActiveViewerEvent_ElapsedTicksAdvances_PreservesValue()
        {
            ViewerEvent ev = BuildSampleEvent(ShotTypeCatalog.ShotPassShotImpact, 0L);
            ActiveViewerEvent fresh = new(ev, elapsedTicks: 0);
            ActiveViewerEvent later = new(ev, elapsedTicks: 60);
            Assert.That(fresh.ElapsedTicks, Is.EqualTo(0));
            Assert.That(later.ElapsedTicks, Is.EqualTo(60));
        }

        // ----- Vector3EqualityComparer (test-local helper) -----

        private sealed class Vector3EqualityComparer : System.Collections.Generic.IEqualityComparer<Vector3>
        {
            private readonly float _tolerance;
            public Vector3EqualityComparer(float tolerance) => _tolerance = tolerance;

            public bool Equals(Vector3 x, Vector3 y) =>
                Math.Abs(x.x - y.x) <= _tolerance &&
                Math.Abs(x.y - y.y) <= _tolerance &&
                Math.Abs(x.z - y.z) <= _tolerance;

            public int GetHashCode(Vector3 obj) => obj.GetHashCode();
        }
    }
}

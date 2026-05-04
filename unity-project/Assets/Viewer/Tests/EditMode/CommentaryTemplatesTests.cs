using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Adapters.Dots;
using FinalWhistle.Viewer.Contracts;
using NUnit.Framework;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Slice-6 tests for the pure-C# commentary + signature display-name
    /// matrix in <see cref="CommentaryTemplates"/>. The renderer (UXML +
    /// USS + UIDocument) is L2-screenshot territory; this fixture pins
    /// the load-bearing arithmetic + invariants:
    ///
    /// <list type="bullet">
    ///   <item>Deterministic <see cref="CommentaryTemplates.TryPickCommentary"/>
    ///     across replays of the same canonical state (replay-pin guarantee).</item>
    ///   <item>Matrix completeness: every Phase-3 bridge-emittable
    ///     <c>(EventClass, ShotCategory)</c> tuple has a registered pool
    ///     (Codex Slice-6 P2-2 closure).</item>
    ///   <item>Pool invariants: 5 strings each, no empty / whitespace-
    ///     only entries, no duplicates within a pool (Codex P3 closure).</item>
    ///   <item>Signature display-name registry: 3 known signatures
    ///     resolve; unknown IDs throw loudly (Codex Slice-6 P3 closure).</item>
    ///   <item>Title-card eligibility: Phase-3 fires on the 3 signature
    ///     execution tuples only — Goal + SignatureBreakthrough are
    ///     explicit no-ops per the matrix.</item>
    /// </list>
    /// </summary>
    public sealed class CommentaryTemplatesTests
    {
        private const int Phase3PoolSize = 5;

        // -------------------------------------------------------------
        // Matrix completeness — every bridge-emittable tuple has a pool
        // -------------------------------------------------------------

        [Test]
        public void MatrixKeys_AreExactlyTheFiveBridgeEmittableTuples()
        {
            // The bridge currently emits exactly five (EventClass,
            // ShotCategory) combinations per design/anime-presentation-
            // budget.md surface mapping + EventBridge.Derive Phase-3
            // scope:
            //   GoalScored             + PassShotImpact
            //   SignatureExecuted      + PlayerIsolation   (LowCutback)
            //   SignatureExecuted      + PassShotImpact    (BlindSideNearPostRun)
            //   SignatureExecuted      + TacticalWide      (FirstTimeDiagonalSwitch)
            //   SignatureBreakthrough  + AftermathFreeze
            // Adding a sixth without updating CommentaryTemplates is a
            // silent-fall-through risk — Codex P2-2 explicitly required
            // the matrix be pinned. This test fails the moment a Phase-4
            // bridge addition lands without a matching commentary entry.
            (EventClass, ShotCategory)[] expected =
            {
                (EventClass.GoalScored, ShotCategory.PassShotImpact),
                (EventClass.SignatureExecuted, ShotCategory.PlayerIsolation),
                (EventClass.SignatureExecuted, ShotCategory.PassShotImpact),
                (EventClass.SignatureExecuted, ShotCategory.TacticalWide),
                (EventClass.SignatureBreakthrough, ShotCategory.AftermathFreeze),
            };
            Assert.That(CommentaryTemplates.CommentaryMatrixKeys,
                Is.EquivalentTo(expected));
        }

        // -------------------------------------------------------------
        // Pool invariants — 5 strings each, non-empty, no duplicates
        // -------------------------------------------------------------

        [Test]
        public void EveryMatrixKey_HasExactlyFiveStrings()
        {
            foreach ((EventClass ec, ShotCategory sc) in CommentaryTemplates.CommentaryMatrixKeys)
            {
                IReadOnlyList<string>? pool = CommentaryTemplates.PoolForTest(ec, sc);
                Assert.That(pool, Is.Not.Null,
                    $"Pool missing for ({ec}, {sc}) — matrix key registered but PoolFor returned null.");
                Assert.That(pool!.Count, Is.EqualTo(Phase3PoolSize),
                    $"Pool ({ec}, {sc}) has {pool.Count} entries; expected {Phase3PoolSize}.");
            }
        }

        [Test]
        public void EveryCommentaryString_IsNonEmpty()
        {
            foreach ((EventClass ec, ShotCategory sc) in CommentaryTemplates.CommentaryMatrixKeys)
            {
                IReadOnlyList<string> pool = CommentaryTemplates.PoolForTest(ec, sc)!;
                for (int i = 0; i < pool.Count; i++)
                {
                    Assert.That(pool[i], Is.Not.Null.And.Not.Empty,
                        $"Pool ({ec}, {sc}) entry [{i}] is null/empty.");
                    Assert.That(pool[i].Trim(), Is.Not.Empty,
                        $"Pool ({ec}, {sc}) entry [{i}] is whitespace-only.");
                }
            }
        }

        [Test]
        public void EveryPool_HasNoDuplicateStrings()
        {
            // A duplicate within a pool reduces the effective variety by
            // 1/N — easy to introduce by copy-paste, hard to spot in a
            // 25-entry table. Pin the invariant here.
            foreach ((EventClass ec, ShotCategory sc) in CommentaryTemplates.CommentaryMatrixKeys)
            {
                IReadOnlyList<string> pool = CommentaryTemplates.PoolForTest(ec, sc)!;
                var unique = new HashSet<string>(pool, StringComparer.Ordinal);
                Assert.That(unique.Count, Is.EqualTo(pool.Count),
                    $"Pool ({ec}, {sc}) contains duplicate strings.");
            }
        }

        // -------------------------------------------------------------
        // Determinism — same canonical state → same pick across replays
        // -------------------------------------------------------------

        [Test]
        public void TryPickCommentary_GoalEvent_IsDeterministic()
        {
            ViewerEvent ev = BuildSampleEvent(
                viewerEventId: 7UL,
                shotTypeId: ShotTypeCatalog.ShotPassShotImpact,
                eventClass: EventClass.GoalScored,
                seedRaw: 0xdeadbeefdeadbeefUL);

            Assert.IsTrue(CommentaryTemplates.TryPickCommentary(ev, out string firstPick));
            Assert.IsTrue(CommentaryTemplates.TryPickCommentary(ev, out string secondPick));
            Assert.That(secondPick, Is.EqualTo(firstPick),
                "Same ViewerEvent must yield the same commentary line — replay-pin guarantee.");
        }

        [Test]
        public void TryPickCommentary_DistinctSeeds_ProduceVariety()
        {
            // 5 distinct seeds covering the index-modulo space — at
            // least 2 distinct strings should surface (the modulo
            // distribution may collide on N=5, but 5 distinct seeds with
            // different mod-5 residues guarantees full coverage).
            ulong[] seeds = { 0UL, 1UL, 2UL, 3UL, 4UL };
            var picks = new HashSet<string>(StringComparer.Ordinal);
            foreach (ulong seedRaw in seeds)
            {
                ViewerEvent ev = BuildSampleEvent(
                    viewerEventId: seedRaw,
                    shotTypeId: ShotTypeCatalog.ShotPassShotImpact,
                    eventClass: EventClass.GoalScored,
                    seedRaw: seedRaw);
                Assert.IsTrue(CommentaryTemplates.TryPickCommentary(ev, out string line));
                picks.Add(line);
            }
            Assert.That(picks.Count, Is.EqualTo(Phase3PoolSize),
                "5 seeds with distinct mod-5 residues must produce all 5 pool strings.");
        }

        [Test]
        public void TryPickCommentary_RestartEvent_ReturnsFalse()
        {
            // Phase-3 bridge does NOT emit ViewerEvents for restart
            // KeyEventKinds — but if a future bridge adds them with a
            // category outside the matrix, the trigger path must
            // silently no-op, not throw. This test pins that contract.
            ViewerEvent ev = BuildSampleEvent(
                viewerEventId: 99UL,
                shotTypeId: ShotTypeCatalog.ShotTacticalWide,
                eventClass: EventClass.GoalScored,  // tuple (GoalScored, TacticalWide) is NOT in the matrix
                seedRaw: 1UL);
            Assert.IsFalse(CommentaryTemplates.TryPickCommentary(ev, out string line));
            Assert.That(line, Is.Empty);
        }

        [Test]
        public void TryPickCommentary_NullEvent_ReturnsFalse()
        {
            Assert.IsFalse(CommentaryTemplates.TryPickCommentary(null!, out string line));
            Assert.That(line, Is.Empty);
        }

        // -------------------------------------------------------------
        // Signature display names
        // -------------------------------------------------------------

        [Test]
        public void GetSignatureDisplayName_KnownSignatures_ReturnFootballNativeText()
        {
            Assert.That(
                CommentaryTemplates.GetSignatureDisplayName("fwh.core:signature.first-time-diagonal-switch"),
                Is.EqualTo("First-time diagonal switch"));
            Assert.That(
                CommentaryTemplates.GetSignatureDisplayName("fwh.core:signature.low-cutback-from-byline"),
                Is.EqualTo("Low cutback from the byline"));
            Assert.That(
                CommentaryTemplates.GetSignatureDisplayName("fwh.core:signature.blind-side-near-post-run"),
                Is.EqualTo("Blind-side near-post run"));
        }

        [Test]
        public void GetSignatureDisplayName_UnknownId_ThrowsLoudly()
        {
            // Loud-fail discipline: a content-pack drift in
            // SignatureMetadata.SignatureId must surface, not silently
            // render the raw ID string in the title-card.
            Assert.Throws<KeyNotFoundException>(
                () => CommentaryTemplates.GetSignatureDisplayName("fwh.core:signature.unknown-future-signature"));
        }

        [Test]
        public void GetSignatureDisplayName_EmptyId_ThrowsArgumentException()
        {
            Assert.Throws<ArgumentException>(
                () => CommentaryTemplates.GetSignatureDisplayName(string.Empty));
        }

        [Test]
        public void EverySignatureDisplayName_IsNonEmpty()
        {
            foreach (string id in CommentaryTemplates.AllSignatureIds)
            {
                string displayName = CommentaryTemplates.GetSignatureDisplayName(id);
                Assert.That(displayName, Is.Not.Null.And.Not.Empty,
                    $"Signature display name for '{id}' is null/empty.");
            }
        }

        // -------------------------------------------------------------
        // Title-card eligibility per matrix
        // -------------------------------------------------------------

        [Test]
        public void ShouldShowTitleCard_OnlyForSignatureExecutionTuples()
        {
            // The 3 signature-execution tuples → YES.
            Assert.IsTrue(CommentaryTemplates.ShouldShowTitleCard(
                EventClass.SignatureExecuted, ShotCategory.PlayerIsolation));
            Assert.IsTrue(CommentaryTemplates.ShouldShowTitleCard(
                EventClass.SignatureExecuted, ShotCategory.PassShotImpact));
            Assert.IsTrue(CommentaryTemplates.ShouldShowTitleCard(
                EventClass.SignatureExecuted, ShotCategory.TacticalWide));

            // Goal + SignatureBreakthrough → NO (explicit matrix entries).
            Assert.IsFalse(CommentaryTemplates.ShouldShowTitleCard(
                EventClass.GoalScored, ShotCategory.PassShotImpact));
            Assert.IsFalse(CommentaryTemplates.ShouldShowTitleCard(
                EventClass.SignatureBreakthrough, ShotCategory.AftermathFreeze));

            // None / unknown → NO.
            Assert.IsFalse(CommentaryTemplates.ShouldShowTitleCard(
                EventClass.None, ShotCategory.None));
        }

        // -------------------------------------------------------------
        // Helpers
        // -------------------------------------------------------------

        private static ViewerEvent BuildSampleEvent(
            ulong viewerEventId, string shotTypeId, EventClass eventClass, ulong seedRaw)
        {
            // SignatureMetadata is required for SignatureExecuted (per
            // ViewerEvent ctor invariant). Fill it for those event class
            // values; null otherwise.
            SignatureRecipeMetadata? metadata = eventClass == EventClass.SignatureExecuted
                ? new SignatureRecipeMetadata(
                    signatureId: "fwh.core:signature.first-time-diagonal-switch",
                    recipeKey: "tactical-wide",
                    simBiasFieldId: "carry_quality",
                    simBiasDeltaRawQ32: 0L)
                : null;
            return new ViewerEvent(
                viewerEventId: viewerEventId,
                sourceEventId: viewerEventId,
                sourceEventOrdinal: 0,
                baseShotTypeId: shotTypeId,
                effectiveShotTypeId: shotTypeId,
                reduceMotionApplied: false,
                startTick: Tick.Zero + 100L,
                endTick: Tick.Zero + 280L,
                seed: Seed.FromUInt64(seedRaw),
                stakesNormalized: Fixed.Zero,
                memoryRelevance: Fixed.Zero,
                focalSubject: null,
                participantPlayerIds: Array.Empty<string>(),
                memoryHits: Array.Empty<MemoryHit>(),
                sourceEventClass: eventClass,
                sourceEntityId: null,
                signatureMetadata: metadata);
        }
    }
}

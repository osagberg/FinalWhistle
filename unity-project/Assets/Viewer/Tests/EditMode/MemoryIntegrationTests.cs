using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Memory;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Adapters.Dots;
using NUnit.Framework;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// C1a EditMode tests for <see cref="DotsMatchDirector.EmitMemoryEventsCore"/>.
    /// Pins the integration that Codex P1 counter (commit-proposal edcac998 →
    /// counter dde7e934) flagged as un-tested. Critical invariants:
    /// <list type="bullet">
    ///   <item><description>No-op when no new KeyEvents — counter unchanged, ledger
    ///     unchanged.</description></item>
    ///   <item><description>Goal KeyEvent → MemoryEvent emitted + appended to ledger;
    ///     counter advances to total.</description></item>
    ///   <item><description>Double-call with no new KeyEvents → no double-emit.</description></item>
    ///   <item><description>Incremental batches: emit 1 → emit 2 more → ledger has 3,
    ///     no duplicates.</description></item>
    ///   <item><description><strong>Retry semantics on emission failure</strong>:
    ///     when MemoryEmissionRules throws, the counter STAYS at the prior value
    ///     so the next FixedUpdate retries. This was the eager-counter-advance bug
    ///     Codex caught.</description></item>
    /// </list>
    /// </summary>
    public sealed class MemoryIntegrationTests
    {
        private const string TestMatchId = "test-match-id";

        // Minimal "goal" KeyEvent. Phase-3 emits goals with JerseyUnspecified
        // (no scorer attribution); MemoryEmissionRules.EmitForKeyEvents handles
        // that path without requiring IdentityPacket lookup.
        private static KeyEvent BuildGoalKeyEvent(long tickValue, TeamSide scoringSide)
        {
            return new KeyEvent(
                tick: Tick.Zero + tickValue,
                kind: KeyEventKind.Goal,
                side: scoringSide,
                jerseyNumber: KeyEvent.JerseyUnspecified,
                position: Vector3Fixed.Zero);
        }

        private static IdentityPacket[] EmptyPackets() => Array.Empty<IdentityPacket>();

        // ----- No-op cases -----

        [Test]
        public void EmitMemoryEventsCore_EmptyKeyEvents_NoOp()
        {
            var ledger = new Ledger();
            var keyEvents = new List<KeyEvent>();
            int result = DotsMatchDirector.EmitMemoryEventsCore(
                keyEvents, previouslyEmittedCount: 0,
                matchId: TestMatchId, ledger: ledger,
                homePackets: EmptyPackets(), awayPackets: EmptyPackets());
            Assert.AreEqual(0, result, "Counter must stay at 0 when keyEvents is empty.");
            Assert.AreEqual(0, ledger.Count, "Ledger must remain empty.");
        }

        [Test]
        public void EmitMemoryEventsCore_PreviouslyEqualsTotal_NoOp()
        {
            var ledger = new Ledger();
            var keyEvents = new List<KeyEvent> { BuildGoalKeyEvent(100, TeamSide.Home) };
            int result = DotsMatchDirector.EmitMemoryEventsCore(
                keyEvents, previouslyEmittedCount: 1,
                matchId: TestMatchId, ledger: ledger,
                homePackets: EmptyPackets(), awayPackets: EmptyPackets());
            Assert.AreEqual(1, result, "Counter stays at 1 when already caught up.");
            Assert.AreEqual(0, ledger.Count, "Ledger must NOT gain an entry on no-op.");
        }

        // ----- Single emission -----

        [Test]
        public void EmitMemoryEventsCore_SingleGoalKeyEvent_EmitsAndAdvances()
        {
            var ledger = new Ledger();
            var keyEvents = new List<KeyEvent> { BuildGoalKeyEvent(100, TeamSide.Home) };
            int result = DotsMatchDirector.EmitMemoryEventsCore(
                keyEvents, previouslyEmittedCount: 0,
                matchId: TestMatchId, ledger: ledger,
                homePackets: EmptyPackets(), awayPackets: EmptyPackets());
            Assert.AreEqual(1, result, "Counter must advance to total (1).");
            Assert.AreEqual(1, ledger.Count, "Ledger must gain exactly one entry.");
            // Verify the emitted MemoryEvent is the right shape.
            var events = ledger.All;
            Assert.AreEqual(EventClass.GoalScored, events[0].What);
        }

        // ----- Double-call no-duplicate -----

        [Test]
        public void EmitMemoryEventsCore_DoubleCallSameInput_DoesNotDoubleEmit()
        {
            var ledger = new Ledger();
            var keyEvents = new List<KeyEvent> { BuildGoalKeyEvent(100, TeamSide.Home) };

            int first = DotsMatchDirector.EmitMemoryEventsCore(
                keyEvents, previouslyEmittedCount: 0,
                matchId: TestMatchId, ledger: ledger,
                homePackets: EmptyPackets(), awayPackets: EmptyPackets());
            Assert.AreEqual(1, first);
            Assert.AreEqual(1, ledger.Count);

            // Second call passes the counter from the first — should no-op.
            int second = DotsMatchDirector.EmitMemoryEventsCore(
                keyEvents, previouslyEmittedCount: first,
                matchId: TestMatchId, ledger: ledger,
                homePackets: EmptyPackets(), awayPackets: EmptyPackets());
            Assert.AreEqual(1, second, "Counter must remain at 1 on no-new-keyEvents call.");
            Assert.AreEqual(1, ledger.Count, "Ledger MUST NOT gain a duplicate entry.");
        }

        // ----- Incremental batches -----

        [Test]
        public void EmitMemoryEventsCore_IncrementalBatches_EmitsOnlyNew()
        {
            var ledger = new Ledger();
            var keyEvents = new List<KeyEvent>();

            // Batch 1: 1 goal
            keyEvents.Add(BuildGoalKeyEvent(100, TeamSide.Home));
            int after1 = DotsMatchDirector.EmitMemoryEventsCore(
                keyEvents, previouslyEmittedCount: 0,
                matchId: TestMatchId, ledger: ledger,
                homePackets: EmptyPackets(), awayPackets: EmptyPackets());
            Assert.AreEqual(1, after1);
            Assert.AreEqual(1, ledger.Count);

            // Batch 2: 2 more goals appended
            keyEvents.Add(BuildGoalKeyEvent(200, TeamSide.Away));
            keyEvents.Add(BuildGoalKeyEvent(300, TeamSide.Home));
            int after2 = DotsMatchDirector.EmitMemoryEventsCore(
                keyEvents, previouslyEmittedCount: after1,
                matchId: TestMatchId, ledger: ledger,
                homePackets: EmptyPackets(), awayPackets: EmptyPackets());
            Assert.AreEqual(3, after2, "Counter advances by 2 (1+2) to total 3.");
            Assert.AreEqual(3, ledger.Count, "Ledger gains exactly 2 new entries.");
        }

        // ----- Retry semantics on emission failure (THE BUG CODEX CAUGHT) -----

        [Test]
        public void EmitMemoryEventsCore_EmissionThrows_CounterPreservedForRetry()
        {
            var ledger = new Ledger();
            // SignatureBreakthrough requires non-empty IdentityPackets per
            // MemoryEmissionRules.cs:99-106 — passing empty packets makes
            // the emission throw. This simulates a content-pack drift in
            // Phase-4+ that breaks emission mid-match.
            var keyEvents = new List<KeyEvent>
            {
                new KeyEvent(
                    tick: Tick.Zero + 100,
                    kind: KeyEventKind.SignatureBreakthrough,
                    side: TeamSide.Home,
                    jerseyNumber: 7,
                    position: Vector3Fixed.Zero)
            };

            Exception capturedException = null;
            int result = DotsMatchDirector.EmitMemoryEventsCore(
                keyEvents, previouslyEmittedCount: 0,
                matchId: TestMatchId, ledger: ledger,
                homePackets: EmptyPackets(),  // empty → SignatureBreakthrough emit throws
                awayPackets: EmptyPackets(),
                onError: ex => capturedException = ex);

            Assert.IsNotNull(capturedException,
                "onError must have been invoked with the caught ArgumentException.");
            Assert.AreEqual(0, result,
                "Counter MUST stay at 0 on exception so next call retries this batch. " +
                "If this asserts 1, the eager-counter-advance bug (Codex P1 on edcac998) has regressed.");
            Assert.AreEqual(0, ledger.Count,
                "Ledger MUST NOT gain partial entries when emit throws.");
        }

        [Test]
        public void EmitMemoryEventsCore_OnEmittedCallback_FiresOncePerMemoryEvent()
        {
            var ledger = new Ledger();
            var keyEvents = new List<KeyEvent>
            {
                BuildGoalKeyEvent(100, TeamSide.Home),
                BuildGoalKeyEvent(200, TeamSide.Away),
            };
            int callbackCount = 0;
            DotsMatchDirector.EmitMemoryEventsCore(
                keyEvents, previouslyEmittedCount: 0,
                matchId: TestMatchId, ledger: ledger,
                homePackets: EmptyPackets(), awayPackets: EmptyPackets(),
                onEmitted: _ => callbackCount++);
            Assert.AreEqual(2, callbackCount,
                "onEmitted callback must fire exactly once per emitted MemoryEvent.");
        }

        /// <summary>
        /// Codex 2026-05-11 P2 round-3 regression pin: the instance method's
        /// Debug.Log uses <c>ledger.Count</c> to render "ledger size N",
        /// so onEmitted MUST observe the ledger AFTER the append. The
        /// pre-fix order (onEmitted before Emit) made the first log read
        /// "ledger size 0" instead of "ledger size 1". This test asserts
        /// the post-append observation contract so any future re-inversion
        /// trips the regression.
        /// </summary>
        [Test]
        public void EmitMemoryEventsCore_OnEmittedCallback_ObservesPostAppendLedgerCount()
        {
            var ledger = new Ledger();
            var keyEvents = new List<KeyEvent>
            {
                BuildGoalKeyEvent(100, TeamSide.Home),
                BuildGoalKeyEvent(200, TeamSide.Away),
                BuildGoalKeyEvent(300, TeamSide.Home),
            };
            var observedCountsAtCallback = new List<int>();
            DotsMatchDirector.EmitMemoryEventsCore(
                keyEvents, previouslyEmittedCount: 0,
                matchId: TestMatchId, ledger: ledger,
                homePackets: EmptyPackets(), awayPackets: EmptyPackets(),
                onEmitted: _ => observedCountsAtCallback.Add(ledger.Count));

            Assert.AreEqual(3, observedCountsAtCallback.Count,
                "Three KeyEvents → three callback fires.");
            Assert.AreEqual(1, observedCountsAtCallback[0],
                "First callback fires AFTER the first ledger.Emit — ledger.Count must be 1, not 0.");
            Assert.AreEqual(2, observedCountsAtCallback[1],
                "Second callback fires AFTER the second ledger.Emit — ledger.Count must be 2.");
            Assert.AreEqual(3, observedCountsAtCallback[2],
                "Third callback fires AFTER the third ledger.Emit — ledger.Count must be 3.");
        }
    }
}

using System.Collections.Generic;
using System.Linq;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Contracts;
using NUnit.Framework;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Unity Test Framework EditMode tests for the Phase-3 minimum
    /// <see cref="EventBridge"/> per SPEC line 148 + ADR-0008
    /// §"Validation criteria." Synthetic-fixture path: hand-built
    /// <c>MatchSimulationState</c> with synthetic KeyEvents flows through
    /// <see cref="EventBridge.Derive"/> and produces a deterministic
    /// <see cref="ViewerEvent"/> stream covering Phase-3's 5 active
    /// KeyEventKind translations + restart-event skip.
    /// </summary>
    public sealed class EventBridgeTests
    {
        private static MatchSimulationState BuildEmptyState()
        {
            // Construct a minimal state with all 22 players initialized so
            // MatchSimulationState's constructor invariants pass. The bridge
            // reads only state.KeyEvents — players + ball stay default.
            PlayerState[] home = new PlayerState[MatchCanonicalState.PlayersPerTeam];
            PlayerState[] away = new PlayerState[MatchCanonicalState.PlayersPerTeam];
            for (byte i = 1; i <= MatchCanonicalState.PlayersPerTeam; i++)
            {
                home[i - 1] = new PlayerState(Vector3Fixed.Zero, Vector3Fixed.Zero, i, TeamSide.Home);
                away[i - 1] = new PlayerState(Vector3Fixed.Zero, Vector3Fixed.Zero, i, TeamSide.Away);
            }
            return new MatchSimulationState(Tick.Zero, BallState.AtRest, home, away);
        }

        private static KeyEvent BuildKeyEvent(long tickValue, KeyEventKind kind, TeamSide side, byte jersey)
        {
            return new KeyEvent(
                tick: new Tick(tickValue),
                kind: kind,
                side: side,
                jerseyNumber: jersey,
                position: Vector3Fixed.Zero);
        }

        // ============================================================
        // SPEC line 148 acceptance: synthetic-fixture path produces
        // deterministic ViewerEvent stream
        // ============================================================

        [Test]
        public void Derive_SyntheticGoalKeyEvent_ProducesPassShotImpactViewerEvent()
        {
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(1200, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(
                state, matchSeed: Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(1, events.Count);
            ViewerEvent ev = events[0];
            Assert.AreEqual(ShotTypeCatalog.ShotPassShotImpact, ev.BaseShotTypeId);
            Assert.AreEqual(ShotTypeCatalog.ShotPassShotImpact, ev.EffectiveShotTypeId);
            Assert.IsFalse(ev.ReduceMotionApplied);
            Assert.AreEqual(1200L, ev.StartTick.Value);
            // Pass-shot-impact is 4s = 240 ticks per ShotTypeCatalog.
            Assert.AreEqual(1200L + 240L, ev.EndTick.Value);
            Assert.AreEqual(EventClass.GoalScored, ev.SourceEventClass);
            // Phase-3 goals don't carry scorer attribution.
            Assert.IsNull(ev.FocalSubject);
            Assert.AreEqual(0, ev.ParticipantPlayerIds.Count);
        }

        [Test]
        public void Derive_LowCutbackKeyEvent_ProducesPlayerIsolationViewerEvent()
        {
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(900, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(1, events.Count);
            ViewerEvent ev = events[0];
            Assert.AreEqual(ShotTypeCatalog.ShotPlayerIsolation, ev.BaseShotTypeId);
            Assert.AreEqual(ShotTypeCatalog.ShotPlayerIsolation, ev.EffectiveShotTypeId);
            Assert.AreEqual("viewer.focal:home.06", ev.FocalSubject);
            Assert.AreEqual(1, ev.ParticipantPlayerIds.Count);
            Assert.AreEqual("viewer.focal:home.06", ev.ParticipantPlayerIds[0]);
        }

        [Test]
        public void Derive_DiagonalSwitchKeyEvent_ProducesTacticalWideViewerEvent()
        {
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(1500, KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch, TeamSide.Home, jersey: 7));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(1, events.Count);
            Assert.AreEqual(ShotTypeCatalog.ShotTacticalWide, events[0].BaseShotTypeId);
            Assert.AreEqual("viewer.focal:home.07", events[0].FocalSubject);
        }

        [Test]
        public void Derive_BlindSideRunKeyEvent_ProducesPassShotImpactViewerEvent()
        {
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(1800, KeyEventKind.SignatureExecuted_BlindSideNearPostRun, TeamSide.Away, jersey: 11));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(1, events.Count);
            Assert.AreEqual(ShotTypeCatalog.ShotPassShotImpact, events[0].BaseShotTypeId);
            Assert.AreEqual("viewer.focal:away.11", events[0].FocalSubject);
        }

        [Test]
        public void Derive_SignatureBreakthroughKeyEvent_ProducesAftermathFreezeViewerEvent()
        {
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(2100, KeyEventKind.SignatureBreakthrough, TeamSide.Home, jersey: 6));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(1, events.Count);
            ViewerEvent ev = events[0];
            Assert.AreEqual(ShotTypeCatalog.ShotAftermathFreeze, ev.BaseShotTypeId);
            // Aftermath-freeze is 5s = 300 ticks (the design-doc upper-bound
            // for genuinely high-stakes beats).
            Assert.AreEqual(2100L + 300L, ev.EndTick.Value);
            Assert.AreEqual(EventClass.SignatureBreakthrough, ev.SourceEventClass);
            Assert.AreEqual("viewer.focal:home.06", ev.FocalSubject);
        }

        [Test]
        public void Derive_RestartKeyEvents_AreSkipped()
        {
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(100, KeyEventKind.GoalKickRestart, TeamSide.Home, KeyEvent.JerseyUnspecified));
            state.KeyEvents.Add(BuildKeyEvent(200, KeyEventKind.ThrowInRestart, TeamSide.Away, KeyEvent.JerseyUnspecified));
            state.KeyEvents.Add(BuildKeyEvent(300, KeyEventKind.CornerKickRestart, TeamSide.Home, KeyEvent.JerseyUnspecified));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            // Restart events stay as routine-band match telemetry per
            // ADR-0004; the bridge skips them at Phase 3.
            Assert.AreEqual(0, events.Count);
        }

        // ============================================================
        // ADR-0008 §Determinism contract: ordering + ID monotonicity
        // ============================================================

        [Test]
        public void Derive_MultipleKeyEvents_AssignsMonotonicViewerEventId()
        {
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(100, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));
            state.KeyEvents.Add(BuildKeyEvent(200, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6));
            state.KeyEvents.Add(BuildKeyEvent(300, KeyEventKind.SignatureBreakthrough, TeamSide.Home, jersey: 6));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(3, events.Count);
            // Monotonic assignment: 0, 1, 2.
            Assert.AreEqual(0UL, events[0].ViewerEventId);
            Assert.AreEqual(1UL, events[1].ViewerEventId);
            Assert.AreEqual(2UL, events[2].ViewerEventId);
            // SourceEventId points back into KeyEvents by index.
            Assert.AreEqual(0UL, events[0].SourceEventId);
            Assert.AreEqual(1UL, events[1].SourceEventId);
            Assert.AreEqual(2UL, events[2].SourceEventId);
        }

        [Test]
        public void Derive_RestartsBetweenSurfacedEvents_AreSkippedButViewerEventIdStaysContiguous()
        {
            // Mixed stream: Goal, GoalKickRestart, LowCutback. Restart is
            // skipped; ViewerEventIds 0 + 1 stay contiguous.
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(100, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));
            state.KeyEvents.Add(BuildKeyEvent(150, KeyEventKind.GoalKickRestart, TeamSide.Away, KeyEvent.JerseyUnspecified));
            state.KeyEvents.Add(BuildKeyEvent(300, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(2, events.Count);
            Assert.AreEqual(0UL, events[0].ViewerEventId);
            Assert.AreEqual(1UL, events[1].ViewerEventId);
            // SourceEventId reflects the underlying KeyEvents index — Goal
            // at index 0, LowCutback at index 2 (restart at index 1
            // skipped).
            Assert.AreEqual(0UL, events[0].SourceEventId);
            Assert.AreEqual(2UL, events[1].SourceEventId);
        }

        [Test]
        public void Derive_SameInputTwice_ProducesByteIdenticalStream()
        {
            // ADR-0008 §"Determinism contract": same canonical input
            // produces same ViewerEvent stream byte-for-byte.
            MatchSimulationState a = BuildEmptyState();
            MatchSimulationState b = BuildEmptyState();
            a.KeyEvents.Add(BuildKeyEvent(100, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));
            a.KeyEvents.Add(BuildKeyEvent(500, KeyEventKind.SignatureBreakthrough, TeamSide.Home, jersey: 6));
            b.KeyEvents.Add(BuildKeyEvent(100, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));
            b.KeyEvents.Add(BuildKeyEvent(500, KeyEventKind.SignatureBreakthrough, TeamSide.Home, jersey: 6));

            IReadOnlyList<ViewerEvent> eventsA = EventBridge.Derive(a, Seed.FromUInt64(0xdeadbeefdeadbeefUL));
            IReadOnlyList<ViewerEvent> eventsB = EventBridge.Derive(b, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(eventsA.Count, eventsB.Count);
            for (int i = 0; i < eventsA.Count; i++)
            {
                Assert.AreEqual(eventsA[i], eventsB[i]);
            }
        }

        // ============================================================
        // Reduce-motion substitution at the bridge boundary
        // ============================================================

        [Test]
        public void Derive_LowCutbackWithReduceMotion_SubstitutesEffectiveShot()
        {
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(900, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(
                state, matchSeed: Seed.FromUInt64(0xdeadbeefdeadbeefUL),
                reduceMotionEnabled: true);

            Assert.AreEqual(1, events.Count);
            ViewerEvent ev = events[0];
            Assert.AreEqual(ShotTypeCatalog.ShotPlayerIsolation, ev.BaseShotTypeId);
            Assert.AreEqual(ShotTypeCatalog.ShotPlayerIsolationReduceMotion, ev.EffectiveShotTypeId);
            Assert.IsTrue(ev.ReduceMotionApplied);
        }

        [Test]
        public void Derive_GoalWithReduceMotion_NoVariantSoNoSubstitution()
        {
            // pass-shot-impact has no reduce_motion_variant defined in the
            // Phase-3 catalog. The flag stays false even with reduce-motion
            // requested.
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(1200, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(
                state, Seed.FromUInt64(0xdeadbeefdeadbeefUL), reduceMotionEnabled: true);

            Assert.AreEqual(1, events.Count);
            Assert.AreEqual(events[0].BaseShotTypeId, events[0].EffectiveShotTypeId);
            Assert.IsFalse(events[0].ReduceMotionApplied);
        }

        // ============================================================
        // Argument validation
        // ============================================================

        [Test]
        public void Derive_NullState_Throws()
        {
            Assert.Throws<System.ArgumentNullException>(() =>
                EventBridge.Derive(null!, Seed.FromUInt64(0xdeadbeefdeadbeefUL)));
        }

        // ============================================================
        // ShotTypeCatalog smoke tests
        // ============================================================

        [Test]
        public void ShotTypeCatalog_PlayerIsolationHasReduceMotionVariant()
        {
            ShotTypeDefinition shot = ShotTypeCatalog.Get(ShotTypeCatalog.ShotPlayerIsolation);
            Assert.AreEqual(ShotCategory.PlayerIsolation, shot.Category);
            Assert.AreEqual(ShotTypeCatalog.ShotPlayerIsolationReduceMotion, shot.ReduceMotionVariantId);
        }

        [Test]
        public void ShotTypeCatalog_AftermathFreezeIsFiveSecondsLong()
        {
            ShotTypeDefinition shot = ShotTypeCatalog.Get(ShotTypeCatalog.ShotAftermathFreeze);
            // 5 seconds at 60Hz = 300 ticks. Per design/breakthrough-moments.md
            // §Q1, this is the longest envelope reserved for high-stakes
            // beats (breakthroughs).
            Assert.AreEqual(300, shot.DurationTicks);
        }

        [Test]
        public void ShotTypeCatalog_UnknownId_Throws()
        {
            Assert.Throws<System.Collections.Generic.KeyNotFoundException>(
                () => ShotTypeCatalog.Get("fwh.core:shot.does-not-exist"));
        }

        [Test]
        public void ShotTypeDefinition_DurationOverFiveSeconds_Throws()
        {
            // Per design/breakthrough-moments.md §Q1: 8s+ shots dropped
            // from consideration. Constructor pins the upper bound at 5s.
            Assert.Throws<System.ArgumentOutOfRangeException>(
                () => new ShotTypeDefinition(
                    id: "fwh.core:shot.test-too-long",
                    category: ShotCategory.AftermathFreeze,
                    durationTicks: 480));  // 8s
        }

        // ============================================================
        // ViewerEvent invariant guards
        // ============================================================

        [Test]
        public void ViewerEvent_ReduceMotionAppliedTrueButShotIdsMatch_Throws()
        {
            // ReduceMotionApplied must be true iff BaseShotTypeId !=
            // EffectiveShotTypeId. Constructor catches the inconsistency.
            Assert.Throws<System.ArgumentException>(() => new ViewerEvent(
                viewerEventId: 0,
                sourceEventId: 0,
                sourceEventOrdinal: 0,
                baseShotTypeId: ShotTypeCatalog.ShotPassShotImpact,
                effectiveShotTypeId: ShotTypeCatalog.ShotPassShotImpact,
                reduceMotionApplied: true,  // contradicts equal IDs
                startTick: new Tick(100),
                endTick: new Tick(200),
                seed: Seed.FromUInt64(0),
                stakesNormalized: Fixed.Zero,
                memoryRelevance: Fixed.Zero,
                focalSubject: null,
                participantPlayerIds: System.Array.Empty<string>(),
                memoryHits: System.Array.Empty<MemoryHit>(),
                sourceEventClass: EventClass.GoalScored,
                sourceEntityId: null));
        }

        [Test]
        public void ViewerEvent_EndTickBeforeStartTick_Throws()
        {
            Assert.Throws<System.ArgumentException>(() => new ViewerEvent(
                viewerEventId: 0,
                sourceEventId: 0,
                sourceEventOrdinal: 0,
                baseShotTypeId: ShotTypeCatalog.ShotPassShotImpact,
                effectiveShotTypeId: ShotTypeCatalog.ShotPassShotImpact,
                reduceMotionApplied: false,
                startTick: new Tick(200),
                endTick: new Tick(100),  // before start
                seed: Seed.FromUInt64(0),
                stakesNormalized: Fixed.Zero,
                memoryRelevance: Fixed.Zero,
                focalSubject: null,
                participantPlayerIds: System.Array.Empty<string>(),
                memoryHits: System.Array.Empty<MemoryHit>(),
                sourceEventClass: EventClass.GoalScored,
                sourceEntityId: null));
        }

        // ============================================================
        // CallbackSlotValue + MemoryHit invariants
        // ============================================================

        [Test]
        public void CallbackSlotValue_BothNull_Throws()
        {
            Assert.Throws<System.ArgumentException>(() =>
                new CallbackSlotValue("test", entityId: null, literalValue: null));
        }

        [Test]
        public void CallbackSlotValue_BothSet_Throws()
        {
            Assert.Throws<System.ArgumentException>(() =>
                new CallbackSlotValue("test", entityId: "fwh.core:player_00006", literalValue: "12"));
        }

        [Test]
        public void MemoryHit_SortsSlotsOrdinalAscendingBySlotName()
        {
            // ADR-0008 §"Determinism contract" ordering rule:
            // MemoryHit.Slots sorted by SlotName ordinal ascending.
            // Constructor enforces this so adapters never re-sort.
            CallbackSlotValue[] unsortedSlots = new[]
            {
                CallbackSlotValue.ForLiteral("zebra", "z"),
                CallbackSlotValue.ForLiteral("alpha", "a"),
                CallbackSlotValue.ForLiteral("gamma", "g"),
            };
            MemoryHit hit = new(
                participantId: "fwh.core:player_00006",
                tag: "fwh.core:tag.press-fan",
                salience: Fixed.Parse("0.5000000000"),
                callbackLineId: "fwh.core:callback_line.test",
                slots: unsortedSlots);
            Assert.AreEqual(3, hit.Slots.Count);
            Assert.AreEqual("alpha", hit.Slots[0].SlotName);
            Assert.AreEqual("gamma", hit.Slots[1].SlotName);
            Assert.AreEqual("zebra", hit.Slots[2].SlotName);
        }
    }
}

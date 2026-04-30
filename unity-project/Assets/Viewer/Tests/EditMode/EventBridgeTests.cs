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

        /// <summary>
        /// Append a signature-execution KeyEvent + the matching
        /// SignatureExecution recipe so the bridge's symmetric validation
        /// (every signature-execution KeyEvent has exactly one recipe;
        /// every recipe maps to a real signature-execution KeyEvent)
        /// passes. Recipe-key matches the signature kind per the
        /// SignatureRules production wiring (#20 → "player-isolation",
        /// #22 → "pass-shot-impact", #13 → "tactical-wide").
        /// </summary>
        private static void AppendSignatureExecution(
            MatchSimulationState state, long tickValue, KeyEventKind kind,
            TeamSide side, byte jersey)
        {
            state.KeyEvents.Add(BuildKeyEvent(tickValue, kind, side, jersey));
            int keyEventIndex = state.KeyEvents.Count - 1;
            (string sigId, string recipeKey, string biasField) = kind switch
            {
                KeyEventKind.SignatureExecuted_LowCutback =>
                    ("fwh.core:signature.low-cutback-from-byline", "player-isolation", "cutback_xAssist"),
                KeyEventKind.SignatureExecuted_BlindSideNearPostRun =>
                    ("fwh.core:signature.blind-side-near-post-run", "pass-shot-impact", "near_post_xG"),
                KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch =>
                    ("fwh.core:signature.first-time-diagonal-switch", "tactical-wide", "diagonal_switch_trigger"),
                _ => throw new System.ArgumentOutOfRangeException(nameof(kind), kind, "Not a signature-execution kind."),
            };
            SignaturePresentationRecipe recipe = new(
                signatureId: sigId,
                recipeKey: recipeKey,
                simBiasFieldId: biasField,
                simBiasDeltaRawQ32: Fixed.OneRaw / 5L);
            state.SignatureRecipes.Add(new SignatureExecution(keyEventIndex, recipe));
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
            AppendSignatureExecution(state, 900, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6);

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(1, events.Count);
            ViewerEvent ev = events[0];
            Assert.AreEqual(ShotTypeCatalog.ShotPlayerIsolation, ev.BaseShotTypeId);
            Assert.AreEqual(ShotTypeCatalog.ShotPlayerIsolation, ev.EffectiveShotTypeId);
            Assert.AreEqual("viewer.focal:home.06", ev.FocalSubject);
            Assert.AreEqual(1, ev.ParticipantPlayerIds.Count);
            Assert.AreEqual("viewer.focal:home.06", ev.ParticipantPlayerIds[0]);
            // Per Codex round-1 P1 against 40159bd: signature executions
            // are NOT breakthroughs — distinct EventClass + moderate stakes.
            Assert.AreEqual(EventClass.SignatureExecuted, ev.SourceEventClass);
            Assert.AreEqual(Fixed.Parse("0.7000000000"), ev.StakesNormalized);
        }

        [Test]
        public void Derive_DiagonalSwitchKeyEvent_ProducesTacticalWideViewerEvent()
        {
            MatchSimulationState state = BuildEmptyState();
            AppendSignatureExecution(state, 1500, KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch, TeamSide.Home, jersey: 7);

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(1, events.Count);
            Assert.AreEqual(ShotTypeCatalog.ShotTacticalWide, events[0].BaseShotTypeId);
            Assert.AreEqual("viewer.focal:home.07", events[0].FocalSubject);
            Assert.AreEqual(EventClass.SignatureExecuted, events[0].SourceEventClass);
        }

        [Test]
        public void Derive_BlindSideRunKeyEvent_ProducesPassShotImpactViewerEvent()
        {
            MatchSimulationState state = BuildEmptyState();
            AppendSignatureExecution(state, 1800, KeyEventKind.SignatureExecuted_BlindSideNearPostRun, TeamSide.Away, jersey: 11);

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(1, events.Count);
            Assert.AreEqual(ShotTypeCatalog.ShotPassShotImpact, events[0].BaseShotTypeId);
            Assert.AreEqual("viewer.focal:away.11", events[0].FocalSubject);
            Assert.AreEqual(EventClass.SignatureExecuted, events[0].SourceEventClass);
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
            AppendSignatureExecution(state, 200, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6);
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
            AppendSignatureExecution(state, 300, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6);

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
            AppendSignatureExecution(state, 900, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6);

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

        // ============================================================
        // Codex round-1 P1 against 40159bd: signature executions are
        // not breakthroughs (distinct EventClass + distinct stakes)
        // ============================================================

        [Test]
        public void Derive_SignatureExecution_IsNotClassifiedAsBreakthrough()
        {
            // Codex round-1 P1: a routine signature fire MUST surface as
            // EventClass.SignatureExecuted with moderate stakes (~0.7),
            // distinct from EventClass.SignatureBreakthrough at Stakes=1.0
            // which is reserved for the cap-reach permanent-development
            // event. Conflating them silently corrupts the dots adapter's
            // modulation logic + the replay-corpus's distinction.
            MatchSimulationState state = BuildEmptyState();
            AppendSignatureExecution(state, 900, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6);
            state.KeyEvents.Add(BuildKeyEvent(1500, KeyEventKind.SignatureBreakthrough, TeamSide.Home, jersey: 6));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(2, events.Count);
            // Signature execution: distinct class + distinct stakes.
            Assert.AreEqual(EventClass.SignatureExecuted, events[0].SourceEventClass);
            Assert.AreEqual(Fixed.Parse("0.7000000000"), events[0].StakesNormalized);
            Assert.AreNotEqual(Fixed.One, events[0].StakesNormalized);
            // Breakthrough still classifies + stakes correctly.
            Assert.AreEqual(EventClass.SignatureBreakthrough, events[1].SourceEventClass);
            Assert.AreEqual(Fixed.One, events[1].StakesNormalized);
        }

        // ============================================================
        // Codex round-1 P1 against 40159bd: bridge consumes
        // SignatureRecipes — derives shot from Recipe.RecipeKey
        // ============================================================

        [Test]
        public void Derive_SignatureExecution_DerivesShotFromRecipeKey()
        {
            // The bridge must derive shot ID from Recipe.RecipeKey, not
            // hard-code by KeyEventKind. Substituting a recipe with a
            // different RecipeKey changes the resulting shot.
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(900, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6));
            // Authored an unusual recipe: LowCutback signature with
            // tactical-wide RecipeKey (a hypothetical content-pack
            // override). Bridge must follow the recipe, not the kind.
            SignaturePresentationRecipe overrideRecipe = new(
                signatureId: "fwh.core:signature.low-cutback-from-byline",
                recipeKey: "tactical-wide",
                simBiasFieldId: "cutback_xAssist",
                simBiasDeltaRawQ32: Fixed.OneRaw / 5L);
            state.SignatureRecipes.Add(new SignatureExecution(
                keyEventIndex: 0, recipe: overrideRecipe));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(1, events.Count);
            // Bridge followed the recipe-key override, not the static
            // KeyEventKind→shot map.
            Assert.AreEqual(ShotTypeCatalog.ShotTacticalWide, events[0].BaseShotTypeId);
        }

        [Test]
        public void Derive_SignatureExecution_MissingRecipe_Throws()
        {
            // Bridge enforces "every signature-execution KeyEvent has
            // exactly one matching SignatureRecipes entry" — a missing
            // recipe is a runner-wiring bug, not a recoverable runtime
            // condition.
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(900, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6));
            // No SignatureRecipes entry added.

            Assert.Throws<System.InvalidOperationException>(() =>
                EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL)));
        }

        [Test]
        public void Derive_SignatureRecipe_MismatchedKeyEventIndex_Throws()
        {
            // A recipe pointing at a non-signature-execution KeyEvent
            // (here: a Goal) is a bridge-contract violation.
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(900, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));
            state.SignatureRecipes.Add(new SignatureExecution(
                keyEventIndex: 0,  // points at Goal, not a signature execution
                recipe: new SignaturePresentationRecipe(
                    "fwh.core:signature.test", "tactical-wide", "test_field", 0L)));

            Assert.Throws<System.InvalidOperationException>(() =>
                EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL)));
        }

        // ============================================================
        // Codex round-1 P2 against 40159bd: Derive returns immutable
        // wrapper — cast-back to List<T> impossible
        // ============================================================

        [Test]
        public void Derive_Result_CannotBeCastToMutableList()
        {
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(100, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            // Per Codex round-1 P2: a bare List<ViewerEvent> would let
            // consumers cast back + reorder/append after the bridge has
            // emitted a deterministic stream. ReadOnlyCollection wrap
            // returns null on the cast-back probe.
            List<ViewerEvent>? leakedList = events as List<ViewerEvent>;
            Assert.IsNull(leakedList);
        }

        // ============================================================
        // Codex round-2 P1 against 24767c0: SignatureRecipeMetadata
        // is exposed on ViewerEvent for adapter consumption
        // ============================================================

        [Test]
        public void Derive_SignatureExecution_PopulatesSignatureMetadataFromRecipe()
        {
            // Codex round-2 P1: ViewerEvent must carry the recipe's
            // SignatureId / SimBiasFieldId / SimBiasDeltaRawQ32 onto
            // the contract surface — the dots adapter consumes them.
            MatchSimulationState state = BuildEmptyState();
            AppendSignatureExecution(state, 900, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6);

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(1, events.Count);
            Assert.IsTrue(events[0].SignatureMetadata.HasValue);
            SignatureRecipeMetadata md = events[0].SignatureMetadata!.Value;
            Assert.AreEqual("fwh.core:signature.low-cutback-from-byline", md.SignatureId);
            Assert.AreEqual("player-isolation", md.RecipeKey);
            Assert.AreEqual("cutback_xAssist", md.SimBiasFieldId);
            // AppendSignatureExecution helper uses Fixed.OneRaw / 5L for the delta.
            Assert.AreEqual(Fixed.OneRaw / 5L, md.SimBiasDeltaRawQ32);
        }

        [Test]
        public void Derive_GoalEvent_HasNullSignatureMetadata()
        {
            // Goals are not signature executions; SignatureMetadata MUST
            // be null so the cross-field invariant on ViewerEvent fires
            // at construction if a future bridge change wrongly populates.
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(1200, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.IsFalse(events[0].SignatureMetadata.HasValue);
        }

        [Test]
        public void Derive_BreakthroughEvent_HasNullSignatureMetadata()
        {
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(1500, KeyEventKind.SignatureBreakthrough, TeamSide.Home, jersey: 6));

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.IsFalse(events[0].SignatureMetadata.HasValue);
        }

        [Test]
        public void ViewerEvent_Construction_SignatureExecutedWithoutMetadata_Throws()
        {
            // Cross-field invariant: SignatureExecuted REQUIRES non-null metadata.
            Assert.Throws<System.ArgumentException>(() => new ViewerEvent(
                viewerEventId: 0,
                sourceEventId: 0,
                sourceEventOrdinal: 0,
                baseShotTypeId: ShotTypeCatalog.ShotPlayerIsolation,
                effectiveShotTypeId: ShotTypeCatalog.ShotPlayerIsolation,
                reduceMotionApplied: false,
                startTick: new Tick(100),
                endTick: new Tick(280),
                seed: Seed.FromUInt64(0),
                stakesNormalized: Fixed.Parse("0.7000000000"),
                memoryRelevance: Fixed.Zero,
                focalSubject: "viewer.focal:home.06",
                participantPlayerIds: new[] { "viewer.focal:home.06" },
                memoryHits: System.Array.Empty<MemoryHit>(),
                sourceEventClass: EventClass.SignatureExecuted,
                sourceEntityId: "viewer.focal:home.06",
                signatureMetadata: null));  // missing — must throw
        }

        [Test]
        public void ViewerEvent_Construction_GoalWithSignatureMetadata_Throws()
        {
            // Cross-field invariant: non-SignatureExecuted REJECTS metadata.
            SignatureRecipeMetadata md = new(
                "fwh.core:signature.low-cutback-from-byline", "player-isolation",
                "cutback_xAssist", Fixed.OneRaw / 5L);
            Assert.Throws<System.ArgumentException>(() => new ViewerEvent(
                viewerEventId: 0,
                sourceEventId: 0,
                sourceEventOrdinal: 0,
                baseShotTypeId: ShotTypeCatalog.ShotPassShotImpact,
                effectiveShotTypeId: ShotTypeCatalog.ShotPassShotImpact,
                reduceMotionApplied: false,
                startTick: new Tick(100),
                endTick: new Tick(340),
                seed: Seed.FromUInt64(0),
                stakesNormalized: Fixed.Parse("0.9500000000"),
                memoryRelevance: Fixed.Zero,
                focalSubject: null,
                participantPlayerIds: System.Array.Empty<string>(),
                memoryHits: System.Array.Empty<MemoryHit>(),
                sourceEventClass: EventClass.GoalScored,
                sourceEntityId: null,
                signatureMetadata: md));  // wrong — must throw
        }

        [Test]
        public void SignatureRecipeMetadata_Equality_RoundTripsFromRecipe()
        {
            // Round-trip through SignatureRecipeMetadata.FromRecipe.
            SignaturePresentationRecipe recipe = new(
                "fwh.core:signature.test", "player-isolation",
                "cutback_xAssist", Fixed.OneRaw / 5L);
            SignatureRecipeMetadata a = SignatureRecipeMetadata.FromRecipe(recipe);
            SignatureRecipeMetadata b = SignatureRecipeMetadata.FromRecipe(recipe);

            Assert.AreEqual(a, b);
            Assert.AreEqual(a.GetHashCode(), b.GetHashCode());
            Assert.IsTrue(a == b);
        }

        // ============================================================
        // Codex round-2 P2 against 24767c0: Recipe.SignatureId must
        // match the specific KeyEventKind
        // ============================================================

        [Test]
        public void Derive_RecipeSignatureIdMismatchesKeyEventKind_Throws()
        {
            // Codex round-2 P2: a LowCutback KeyEvent with a
            // blind-side-near-post-run-keyed recipe must throw at the
            // bridge boundary, not silently emit a wrong-signature
            // ViewerEvent.
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(900, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6));
            // Recipe carries the WRONG SignatureId for this kind:
            SignaturePresentationRecipe wrongRecipe = new(
                signatureId: "fwh.core:signature.blind-side-near-post-run",  // wrong: kind expects low-cutback
                recipeKey: "player-isolation",
                simBiasFieldId: "near_post_xG",
                simBiasDeltaRawQ32: Fixed.OneRaw / 4L);
            state.SignatureRecipes.Add(new SignatureExecution(0, wrongRecipe));

            Assert.Throws<System.InvalidOperationException>(() =>
                EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL)));
        }

        // ============================================================
        // Codex round-1 P2 against 40159bd: KeyEvents must be
        // StartTick-non-decreasing
        // ============================================================

        [Test]
        public void Derive_OutOfOrderKeyEvents_Throws()
        {
            // Per ADR-0008 §Determinism contract: stream order is
            // (StartTick, ViewerEventId). Bridge requires the canonical
            // KeyEvents stream to already be StartTick-non-decreasing
            // (the MatchSim runtime invariant). A hand-built or future
            // orchestration state with [tick=300, tick=100] must throw
            // at the bridge boundary, not silently emit out-of-order.
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(300, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));
            state.KeyEvents.Add(BuildKeyEvent(100, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));

            Assert.Throws<System.ArgumentException>(() =>
                EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL)));
        }

        [Test]
        public void Derive_EqualStartTickKeyEvents_AreAllowed()
        {
            // Equal-tick is allowed (multiple events on same tick →
            // ordering by ViewerEventId per ADR-0008). Only strict-
            // decreasing throws.
            MatchSimulationState state = BuildEmptyState();
            state.KeyEvents.Add(BuildKeyEvent(100, KeyEventKind.Goal, TeamSide.Home, KeyEvent.JerseyUnspecified));
            AppendSignatureExecution(state, 100, KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home, jersey: 6);

            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(state, Seed.FromUInt64(0xdeadbeefdeadbeefUL));

            Assert.AreEqual(2, events.Count);
            // Both at tick 100; ViewerEventId monotonic.
            Assert.AreEqual(100L, events[0].StartTick.Value);
            Assert.AreEqual(100L, events[1].StartTick.Value);
            Assert.AreEqual(0UL, events[0].ViewerEventId);
            Assert.AreEqual(1UL, events[1].ViewerEventId);
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

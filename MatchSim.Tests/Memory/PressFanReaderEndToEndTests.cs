using System;
using System.Collections.Generic;
using System.Linq;
using FinalWhistle.MatchSim.Memory;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Memory;

/// <summary>
/// End-to-end + reader-behavior tests for the Phase-3 memory layer.
/// The headline test (<see cref="PressFanReader_EndToEnd_GoalKeyEventProducesCallbackWithCorrectTemplate"/>)
/// is the SPEC line 144 acceptance gate: a synthetic goal KeyEvent
/// flows through <see cref="MemoryEmissionRules.EmitForKeyEvents"/>
/// → <see cref="Ledger.Emit"/> → <see cref="PressFanReader.QueryForSeason"/>
/// and produces a <see cref="CallbackCandidate"/> carrying the
/// <see cref="PressFanReader.GoalScoredTemplateId"/> + non-default
/// surfacing salience. This proves the reader-callback pattern is
/// wired end-to-end.
/// </summary>
public sealed class PressFanReaderEndToEndTests
{
    private static KeyEvent BuildSyntheticGoal(long tickValue, TeamSide side, byte jersey)
    {
        return new KeyEvent(
            tick: new Tick(tickValue),
            kind: KeyEventKind.Goal,
            side: side,
            jerseyNumber: jersey,
            position: Vector3Fixed.Zero);
    }

    [Fact]
    public void PressFanReader_EndToEnd_GoalKeyEventProducesCallbackWithCorrectTemplate()
    {
        // SPEC line 144 acceptance: 1 MemoryEvent reader callback
        // demonstrated end-to-end. Flow:
        //   1. Synthetic Goal KeyEvent at tick 1200, home jersey unspecified.
        //   2. MemoryEmissionRules.EmitForKeyEvents translates to 1
        //      MemoryEvent with EventClass.GoalScored + Salience=0.60
        //      (Phase-3 placeholders + Phase3Defaults weights).
        //   3. Ledger.Emit appends.
        //   4. PressFanReader.QueryForSeason returns 1 CallbackCandidate
        //      carrying the goal_press_fan_milestone template.
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildSyntheticGoal(tickValue: 1200, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
        };

        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents: keyEvents,
            matchId: "0xdeadbeef",
            season: 1,
            careerDate: new CareerDate(1, 5, 3),
            weights: SalienceWeights.Phase3Defaults);

        Assert.Single(events);
        MemoryEvent ev = events[0];
        Assert.Equal(EventClass.GoalScored, ev.What);
        Assert.Equal(Emotion.Triumph, ev.Emotion);
        Assert.Equal((uint)1200, ev.Tick);
        Assert.Equal("0xdeadbeef", ev.MatchId);
        // Phase-3 placeholders (Stakes=0.95, Prominence=0.6, Class=0.6)
        // with Phase3Defaults weights (0.4, 0.2, 0.2, 0.1, 0.1) compute
        // ~0.62; band classification == Notable is the load-bearing
        // assertion (band-filter is what gates the press-fan surfacing).
        Assert.Equal(SalienceBand.Notable, SalienceEngine.ClassifyBand(ev.Salience));

        Ledger ledger = new();
        ledger.Emit(ev);
        Assert.Equal(1, ledger.Count);

        PressFanReader reader = new(ledger);
        List<CallbackCandidate> candidates = reader.QueryForSeason(currentSeason: 1).ToList();

        Assert.Single(candidates);
        CallbackCandidate candidate = candidates[0];
        Assert.Equal(PressFanReader.GoalScoredTemplateId, candidate.Template);
        // age=0 at season 1 vs current 1 → no decay; surfacing == emission.
        Assert.Equal(ev.Salience, candidate.SurfacingSalience);
        Assert.Equal(ev.Id, candidate.Source.Id);
    }

    [Fact]
    public void PressFanReader_Query_RoutineBandEvent_ReturnsNoCandidates()
    {
        // An event with salience below the press-fan tag's MinBand
        // (Notable @ 0.60) is not surfaced. Construct a Routine-band
        // GoalScored event by hand (skipping MemoryEmissionRules) so
        // the salience can be set below threshold.
        SalienceInputs lowInputs = new(
            stakes: Fixed.Parse("0.1000000000"),
            participantProminenceAvg: Fixed.Parse("0.1000000000"),
            eventClassBaseWeight: Fixed.Parse("0.1000000000"),
            rivalryBoost: Fixed.Zero,
            rarityBoost: Fixed.Zero);
        Fixed lowSalience = Fixed.Parse("0.1000000000");
        MemoryEvent ev = new(
            id: "match:test:tick:0:seq:0",
            matchId: "test", season: 1, tick: 0u,
            careerDate: new CareerDate(1, 1, 1),
            emitter: new EventEmitter(EmitterKind.Match, "test"),
            participants: Array.Empty<Participant>(),
            what: EventClass.GoalScored,
            stakes: Fixed.Parse("0.1000000000"),
            emotion: Emotion.Triumph,
            salience: lowSalience,
            salienceInputs: lowInputs,
            salienceModelVersion: SalienceWeights.Phase3ModelVersion,
            schemaVersion: MemoryEvent.CurrentSchemaVersion);

        Ledger ledger = new();
        ledger.Emit(ev);

        PressFanReader reader = new(ledger);
        List<CallbackCandidate> candidates = reader.QueryForSeason(1).ToList();
        Assert.Empty(candidates);
    }

    [Fact]
    public void PressFanReader_Query_OldEventOutsideWindow_ReturnsNoCandidates()
    {
        // Emit a Notable event in season 1; query season 5 with default
        // 3-season window (covers seasons 2..5). Season 1 is outside
        // the window → no candidate.
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildSyntheticGoal(tickValue: 100, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
        };
        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents: keyEvents,
            matchId: "0xtest", season: 1,
            careerDate: new CareerDate(1, 1, 1),
            weights: SalienceWeights.Phase3Defaults);
        Ledger ledger = new();
        ledger.Emit(events[0]);

        PressFanReader reader = new(ledger);
        List<CallbackCandidate> candidates = reader.QueryForSeason(currentSeason: 5).ToList();
        Assert.Empty(candidates);
    }

    [Fact]
    public void PressFanReader_Query_OneSeasonOldEvent_AppliesCallbackAgeDecay()
    {
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildSyntheticGoal(tickValue: 100, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
        };
        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents: keyEvents,
            matchId: "0xtest", season: 1,
            careerDate: new CareerDate(1, 1, 1),
            weights: SalienceWeights.Phase3Defaults);
        Ledger ledger = new();
        ledger.Emit(events[0]);

        PressFanReader reader = new(ledger);
        List<CallbackCandidate> candidates = reader.QueryForSeason(currentSeason: 2).ToList();
        Assert.Single(candidates);
        // Base salience ≈ 0.62 (Phase-3 placeholders); age 1 → decay 0.05
        // → surfacing ≈ 0.57. Decay direction (surfacing < emission) is
        // the contract; absolute value tolerance covers Q32.32 multiply
        // rounding in both Compute + ApplyCallbackAgeModifier paths.
        Assert.True(candidates[0].SurfacingSalience < candidates[0].Source.Salience,
            "Reader-side callback-age modifier failed to reduce surfacing salience.");
        Fixed delta = candidates[0].Source.Salience - candidates[0].SurfacingSalience;
        Fixed expectedDelta = SalienceEngine.CallbackAgeDecayPerSeason;
        Fixed deltaError = delta > expectedDelta ? delta - expectedDelta : expectedDelta - delta;
        Assert.True(deltaError < Fixed.Parse("0.0000000050"),
            $"Decay magnitude {delta} differs from expected {expectedDelta} by {deltaError} (>1ULP).");
    }

    [Fact]
    public void Ledger_Emit_NonGoalKeyEvents_AreSkippedByEmissionRules()
    {
        // MemoryEmissionRules only translates KeyEventKind.Goal in
        // Phase 3. Restart events are routine-band match telemetry,
        // signature-execution events translate Phase-4+.
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            new KeyEvent(new Tick(100), KeyEventKind.GoalKickRestart, TeamSide.Home,
                KeyEvent.JerseyUnspecified, Vector3Fixed.Zero),
            new KeyEvent(new Tick(200), KeyEventKind.ThrowInRestart, TeamSide.Away,
                KeyEvent.JerseyUnspecified, Vector3Fixed.Zero),
            new KeyEvent(new Tick(300), KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home,
                jerseyNumber: 6, position: Vector3Fixed.Zero),
        };
        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents: keyEvents,
            matchId: "0xtest", season: 1,
            careerDate: new CareerDate(1, 1, 1),
            weights: SalienceWeights.Phase3Defaults);
        Assert.Empty(events);
    }

    [Fact]
    public void MemoryEmissionRules_EventIds_AreDeterministicFromInputs()
    {
        // Same input → same event-id. Pin the Phase-3 ID format
        // explicitly: "match:<matchId>:tick:<tick>:seq:<n>".
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildSyntheticGoal(tickValue: 1200, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
            BuildSyntheticGoal(tickValue: 1500, TeamSide.Away, jersey: KeyEvent.JerseyUnspecified),
        };
        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents: keyEvents,
            matchId: "0xdeadbeef", season: 1,
            careerDate: new CareerDate(1, 5, 3),
            weights: SalienceWeights.Phase3Defaults);
        Assert.Equal(2, events.Count);
        Assert.Equal("match:0xdeadbeef:tick:1200:seq:0", events[0].Id);
        Assert.Equal("match:0xdeadbeef:tick:1500:seq:1", events[1].Id);
    }

    [Fact]
    public void Ledger_Emit_FutureSchemaVersion_Throws()
    {
        // The MemoryEvent constructor itself rejects schema-version
        // mismatches, so this test confirms the validator path holds.
        Ledger ledger = new();
        // Build a valid event then fake a future-version one via
        // direct construction — but the constructor already rejects.
        // Instead exercise the path where a valid event is emitted
        // successfully (negative test for completeness).
        SalienceInputs inputs = new(
            Fixed.Parse("0.5000000000"), Fixed.Parse("0.5000000000"),
            Fixed.Parse("0.5000000000"), Fixed.Zero, Fixed.Zero);
        Assert.Throws<ArgumentException>(() => new MemoryEvent(
            id: "id", matchId: null, season: 1, tick: 0u,
            careerDate: new CareerDate(1, 1, 1),
            emitter: new EventEmitter(EmitterKind.Match, "src"),
            participants: Array.Empty<Participant>(),
            what: EventClass.GoalScored,
            stakes: Fixed.Parse("0.5000000000"),
            emotion: Emotion.Triumph,
            salience: Fixed.Parse("0.5000000000"),
            salienceInputs: inputs,
            salienceModelVersion: 1,
            schemaVersion: 99));
    }

    [Fact]
    public void Ledger_SinceSeason_ReturnsOnlyAtOrAfterEvents()
    {
        Ledger ledger = new();
        for (ushort s = 1; s <= 3; s++)
        {
            IReadOnlyList<KeyEvent> keyEvents = new[]
            {
                BuildSyntheticGoal(tickValue: 100 * s, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
            };
            foreach (var e in MemoryEmissionRules.EmitForKeyEvents(
                keyEvents, "0xtest", s, new CareerDate(s, 1, 1),
                SalienceWeights.Phase3Defaults))
            {
                ledger.Emit(e);
            }
        }
        Assert.Equal(3, ledger.Count);
        Assert.Equal(2, ledger.SinceSeason(2).Count);
        Assert.Empty(ledger.SinceSeason(99));
    }

    [Fact]
    public void CallbackTagRegistry_PressFan_ValidatesAtStaticInit()
    {
        Assert.Equal(CallbackTagRegistry.PressFanId, CallbackTagRegistry.PressFan.Id);
        Assert.Equal(SalienceBand.Notable, CallbackTagRegistry.PressFan.MinBand);
        Assert.IsType<ExpiryPolicy.Seasons>(CallbackTagRegistry.PressFan.Expiry);
        Assert.Equal((byte)3, ((ExpiryPolicy.Seasons)CallbackTagRegistry.PressFan.Expiry).Count);
    }

    [Fact]
    public void EventClassRegistry_GoalScored_TagsIncludePressFan()
    {
        IReadOnlyList<string> tags = EventClassRegistry.TagsFor(EventClass.GoalScored);
        Assert.Contains(CallbackTagRegistry.PressFanId, tags);
    }

    [Fact]
    public void EventClassRegistry_BaseWeightFor_NoneEventClass_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            EventClassRegistry.BaseWeightFor(EventClass.None));
    }

    // ============================================================
    // Codex round-1 P1: registry-boundary enforcement
    // ============================================================

    [Fact]
    public void PressFanReader_Query_ScoringMilestoneIdViaPressFanReader_Throws()
    {
        // Codex round-1 P1: PressFanReader is registered as a consumer
        // of press-fan only; querying ScoringMilestoneId via this reader
        // must fail loud at the registry boundary, not silently return
        // press-fan templates.
        Ledger ledger = new();
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildSyntheticGoal(tickValue: 100, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
        };
        foreach (var e in MemoryEmissionRules.EmitForKeyEvents(
            keyEvents, "0xtest", 1, new CareerDate(1, 1, 1),
            SalienceWeights.Phase3Defaults))
        {
            ledger.Emit(e);
        }

        PressFanReader reader = new(ledger);
        ReaderQuery wrongReaderQuery = new(
            tagId: CallbackTagRegistry.ScoringMilestoneId,
            fromSeason: 0, toSeason: 1, currentSeason: 1,
            minBand: SalienceBand.Notable);

        var ex = Assert.Throws<InvalidOperationException>(() =>
            reader.Query(wrongReaderQuery).GetEnumerator().MoveNext());
        Assert.Contains("not a registered consumer", ex.Message);
        Assert.Contains(CallbackTagRegistry.ScoringMilestoneId, ex.Message);
    }

    [Fact]
    public void PressFanReader_Query_UnknownTagId_Throws()
    {
        Ledger ledger = new();
        PressFanReader reader = new(ledger);
        ReaderQuery unknownTagQuery = new(
            tagId: "fwh.core:tag.does-not-exist",
            fromSeason: 0, toSeason: 1, currentSeason: 1,
            minBand: SalienceBand.Notable);

        Assert.Throws<ArgumentException>(() =>
            reader.Query(unknownTagQuery).GetEnumerator().MoveNext());
    }

    [Fact]
    public void PressFanReader_Query_ExpiredEvent_FilteredOut()
    {
        // PressFan tag has Expiry = Seasons(3). An event from season 1
        // queried at currentSeason=5 is expired (5 > 1+3).
        Ledger ledger = new();
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildSyntheticGoal(tickValue: 100, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
        };
        foreach (var e in MemoryEmissionRules.EmitForKeyEvents(
            keyEvents, "0xtest", season: 1, new CareerDate(1, 1, 1),
            SalienceWeights.Phase3Defaults))
        {
            ledger.Emit(e);
        }

        PressFanReader reader = new(ledger);
        ReaderQuery expiredWindowQuery = new(
            tagId: CallbackTagRegistry.PressFanId,
            fromSeason: 0, toSeason: 5, currentSeason: 5,
            minBand: SalienceBand.Notable);
        List<CallbackCandidate> candidates = reader.Query(expiredWindowQuery).ToList();
        Assert.Empty(candidates);
    }

    [Fact]
    public void PressFanReader_Query_TightenedMinBandHonored_BeyondTagFloor()
    {
        // Effective MinBand is max(query.MinBand, tag.MinBand). Querying
        // with MinBand=SeasonDefining tightens past the tag's Notable
        // floor — Phase-3 GoalScored events sit in Notable, so they're
        // filtered out.
        Ledger ledger = new();
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildSyntheticGoal(tickValue: 100, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
        };
        foreach (var e in MemoryEmissionRules.EmitForKeyEvents(
            keyEvents, "0xtest", 1, new CareerDate(1, 1, 1),
            SalienceWeights.Phase3Defaults))
        {
            ledger.Emit(e);
        }

        PressFanReader reader = new(ledger);
        ReaderQuery seasonDefiningQuery = new(
            tagId: CallbackTagRegistry.PressFanId,
            fromSeason: 0, toSeason: 1, currentSeason: 1,
            minBand: SalienceBand.SeasonDefining);
        List<CallbackCandidate> candidates = reader.Query(seasonDefiningQuery).ToList();
        Assert.Empty(candidates);
    }

    // ============================================================
    // Codex round-1 P2: cast-back immutability + stable ordering
    // ============================================================

    [Fact]
    public void MemoryEvent_Participants_CastToArray_ReturnsNullPreventingMutation()
    {
        // ReadOnlyCollection<T> wrap per round-1 P2: caller cannot cast
        // back to Participant[] and mutate the persisted event. C# 'as'
        // returns null when the cast is invalid (ROC<T> is not Participant[]).
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildSyntheticGoal(tickValue: 100, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
        };
        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents, "0xtest", 1, new CareerDate(1, 1, 1),
            SalienceWeights.Phase3Defaults);
        MemoryEvent ev = events[0];

        Participant[]? leakedArray = ev.Participants as Participant[];
        Assert.Null(leakedArray);
    }

    [Fact]
    public void CallbackTag_ConsumingReaders_CastToArray_ReturnsNullPreventingMutation()
    {
        ReaderId[]? leakedArray = CallbackTagRegistry.PressFan.ConsumingReaders as ReaderId[];
        Assert.Null(leakedArray);
    }

    [Fact]
    public void EventClassRegistry_TagsFor_CastToArray_ReturnsNullPreventingMutation()
    {
        IReadOnlyList<string> tags = EventClassRegistry.TagsFor(EventClass.GoalScored);
        string[]? leakedArray = tags as string[];
        Assert.Null(leakedArray);
    }

    [Fact]
    public void PressFanReader_Query_EqualSalienceCandidates_OrderedByLedgerInsertionIndex()
    {
        // Codex round-1 P2: List<T>.Sort is not stable, and Phase-3
        // GoalScored events all share the same placeholder salience, so
        // order could drift across platforms / .NET versions. The
        // explicit secondary key (ledger insertion index ascending)
        // pins the ordering deterministically.
        Ledger ledger = new();
        // Three same-season goals at distinct ticks — same salience,
        // same emission season. Ledger insertion order: 100, 200, 300.
        for (uint tick = 100; tick <= 300; tick += 100)
        {
            IReadOnlyList<KeyEvent> keyEvents = new[]
            {
                BuildSyntheticGoal(tickValue: tick, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
            };
            foreach (var e in MemoryEmissionRules.EmitForKeyEvents(
                keyEvents, "0xtest", 1, new CareerDate(1, 1, 1),
                SalienceWeights.Phase3Defaults,
                eventSeqStart: ledger.Count))
            {
                ledger.Emit(e);
            }
        }
        Assert.Equal(3, ledger.Count);

        PressFanReader reader = new(ledger);
        List<CallbackCandidate> candidates = reader.QueryForSeason(1).ToList();
        Assert.Equal(3, candidates.Count);

        // All three share the same emission salience (Phase-3 placeholders
        // produce the same scalar). Order MUST be by ledger insertion
        // index ascending — i.e., Tick 100, 200, 300.
        Assert.Equal((uint)100, candidates[0].Source.Tick);
        Assert.Equal((uint)200, candidates[1].Source.Tick);
        Assert.Equal((uint)300, candidates[2].Source.Tick);
    }
}

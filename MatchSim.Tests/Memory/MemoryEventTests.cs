using System;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Memory;

/// <summary>
/// Schema + invariant tests for <see cref="MemoryEvent"/> + supporting
/// value types. Per ADR-0004 §"`MemoryEvent` schema": every field has a
/// documented invariant; the constructor enforces them at the boundary
/// so a malformed event cannot reach <c>Ledger.Emit</c>.
/// </summary>
public sealed class MemoryEventTests
{
    private static MemoryEvent BuildValidGoalEvent()
    {
        SalienceInputs inputs = new(
            stakes: Fixed.Parse("0.9000000000"),
            participantProminenceAvg: Fixed.Parse("0.6000000000"),
            eventClassBaseWeight: Fixed.Parse("0.6000000000"),
            rivalryBoost: Fixed.Zero,
            rarityBoost: Fixed.Zero);
        return new MemoryEvent(
            id: "match:0xdead:tick:1200:seq:0",
            matchId: "0xdead",
            season: 1,
            tick: 1200u,
            careerDate: new CareerDate(1, 5, 3),
            emitter: new EventEmitter(EmitterKind.Match, "0xdead"),
            participants: Array.Empty<Participant>(),
            what: EventClass.GoalScored,
            stakes: Fixed.Parse("0.9000000000"),
            emotion: Emotion.Triumph,
            salience: Fixed.Parse("0.6000000000"),
            salienceInputs: inputs,
            salienceModelVersion: 1,
            schemaVersion: MemoryEvent.CurrentSchemaVersion);
    }

    [Fact]
    public void MemoryEvent_Construction_WithValidInputs_Succeeds()
    {
        MemoryEvent ev = BuildValidGoalEvent();
        Assert.Equal(EventClass.GoalScored, ev.What);
        Assert.Equal(Emotion.Triumph, ev.Emotion);
        Assert.Equal(MemoryEvent.CurrentSchemaVersion, ev.SchemaVersion);
        Assert.True(ev.Salience >= Fixed.Zero && ev.Salience <= Fixed.One);
    }

    [Fact]
    public void MemoryEvent_Construction_WithNoneEventClass_Throws()
    {
        SalienceInputs inputs = new(
            Fixed.Parse("0.5000000000"), Fixed.Parse("0.5000000000"),
            Fixed.Parse("0.5000000000"), Fixed.Zero, Fixed.Zero);
        Assert.Throws<ArgumentException>(() => new MemoryEvent(
            id: "id", matchId: null, season: 1, tick: 0u,
            careerDate: new CareerDate(1, 1, 1),
            emitter: new EventEmitter(EmitterKind.Match, "src"),
            participants: Array.Empty<Participant>(),
            what: EventClass.None,
            stakes: Fixed.Parse("0.5000000000"),
            emotion: Emotion.Triumph,
            salience: Fixed.Parse("0.5000000000"),
            salienceInputs: inputs,
            salienceModelVersion: 1,
            schemaVersion: MemoryEvent.CurrentSchemaVersion));
    }

    [Fact]
    public void MemoryEvent_Construction_WithSalienceAboveOne_Throws()
    {
        SalienceInputs inputs = new(
            Fixed.Parse("0.5000000000"), Fixed.Parse("0.5000000000"),
            Fixed.Parse("0.5000000000"), Fixed.Zero, Fixed.Zero);
        Fixed outOfRange = Fixed.FromRaw(Fixed.OneRaw + 1L);
        Assert.Throws<ArgumentOutOfRangeException>(() => new MemoryEvent(
            id: "id", matchId: null, season: 1, tick: 0u,
            careerDate: new CareerDate(1, 1, 1),
            emitter: new EventEmitter(EmitterKind.Match, "src"),
            participants: Array.Empty<Participant>(),
            what: EventClass.GoalScored,
            stakes: Fixed.Parse("0.5000000000"),
            emotion: Emotion.Triumph,
            salience: outOfRange,
            salienceInputs: inputs,
            salienceModelVersion: 1,
            schemaVersion: MemoryEvent.CurrentSchemaVersion));
    }

    [Fact]
    public void MemoryEvent_Construction_WithFutureSchemaVersion_Throws()
    {
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
    public void MemoryEvent_Construction_WithNullParticipants_Throws()
    {
        SalienceInputs inputs = new(
            Fixed.Parse("0.5000000000"), Fixed.Parse("0.5000000000"),
            Fixed.Parse("0.5000000000"), Fixed.Zero, Fixed.Zero);
        Assert.Throws<ArgumentNullException>(() => new MemoryEvent(
            id: "id", matchId: null, season: 1, tick: 0u,
            careerDate: new CareerDate(1, 1, 1),
            emitter: new EventEmitter(EmitterKind.Match, "src"),
            participants: null!,
            what: EventClass.GoalScored,
            stakes: Fixed.Parse("0.5000000000"),
            emotion: Emotion.Triumph,
            salience: Fixed.Parse("0.5000000000"),
            salienceInputs: inputs,
            salienceModelVersion: 1,
            schemaVersion: MemoryEvent.CurrentSchemaVersion));
    }

    [Fact]
    public void SalienceInputs_Construction_WithInputAboveOne_Throws()
    {
        Fixed outOfRange = Fixed.FromRaw(Fixed.OneRaw + 1L);
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            new SalienceInputs(outOfRange, Fixed.Zero, Fixed.Zero, Fixed.Zero, Fixed.Zero));
    }

    [Fact]
    public void CareerDate_Construction_WithWeekZero_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new CareerDate(1, 0, 1));
    }

    [Fact]
    public void CareerDate_Construction_WithDayEight_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new CareerDate(1, 1, 8));
    }

    [Fact]
    public void EventEmitter_Construction_WithKindNone_Throws()
    {
        Assert.Throws<ArgumentException>(() => new EventEmitter(EmitterKind.None, "src"));
    }

    [Fact]
    public void EventEmitter_Construction_WithEmptySource_Throws()
    {
        Assert.Throws<ArgumentException>(() => new EventEmitter(EmitterKind.Match, ""));
    }

    [Fact]
    public void Participant_Construction_WithEmptyRole_Throws()
    {
        Assert.Throws<ArgumentException>(() => new Participant("", "fwh.core:player_00001"));
    }

    [Fact]
    public void ReaderId_Construction_WithEmptyValue_Throws()
    {
        Assert.Throws<ArgumentException>(() => new ReaderId(""));
    }

    [Fact]
    public void ReaderQuery_Construction_WithFromGreaterThanTo_Throws()
    {
        Assert.Throws<ArgumentException>(() =>
            new ReaderQuery("fwh.core:tag.press-fan",
                fromSeason: 5, toSeason: 3,
                currentSeason: 5, minBand: SalienceBand.Notable));
    }

    [Fact]
    public void ReaderQuery_Construction_WithCurrentSeasonBeforeToSeason_Throws()
    {
        // Per round-1 finding C1: ReaderQuery decouples currentSeason
        // from toSeason for callback-age decay; constructor rejects a
        // window that extends past the in-world present.
        Assert.Throws<ArgumentException>(() =>
            new ReaderQuery("fwh.core:tag.press-fan",
                fromSeason: 1, toSeason: 5,
                currentSeason: 3, minBand: SalienceBand.Notable));
    }

    [Fact]
    public void ExpiryPolicy_Seasons_Construction_WithCountZero_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new ExpiryPolicy.Seasons(0));
    }

    [Fact]
    public void ExpiryPolicy_OnEvent_Construction_WithEventClassNone_Throws()
    {
        Assert.Throws<ArgumentException>(() => new ExpiryPolicy.OnEvent(EventClass.None));
    }

    [Fact]
    public void CallbackTag_Construction_NoConsumingReaders_Throws()
    {
        // Per round-1 finding 2: CallbackTag is constructor-validating
        // (no opt-in Validate()) — a call site cannot bypass the
        // ConsumingReaders.Count >= 1 invariant.
        Assert.Throws<ArgumentException>(() => new CallbackTag(
            id: "fwh.core:tag.test",
            consumingReaders: Array.Empty<ReaderId>(),
            minBand: SalienceBand.Notable,
            expiry: ExpiryPolicy.Never.Instance));
    }

    [Fact]
    public void CallbackTag_Construction_EmptyId_Throws()
    {
        Assert.Throws<ArgumentException>(() => new CallbackTag(
            id: "",
            consumingReaders: new[] { new ReaderId("test-reader") },
            minBand: SalienceBand.Notable,
            expiry: ExpiryPolicy.Never.Instance));
    }

    [Fact]
    public void CallbackTag_Construction_NullExpiry_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => new CallbackTag(
            id: "fwh.core:tag.test",
            consumingReaders: new[] { new ReaderId("test-reader") },
            minBand: SalienceBand.Notable,
            expiry: null!));
    }

    [Fact]
    public void CallbackCandidate_Construction_WithSurfacingSalienceAboveOne_Throws()
    {
        MemoryEvent ev = BuildValidGoalEvent();
        Fixed outOfRange = Fixed.FromRaw(Fixed.OneRaw + 1L);
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            new CallbackCandidate(ev, outOfRange, "fwh.core:callback_template.test"));
    }

    [Fact]
    public void CallbackCandidate_Construction_WithEmptyTemplate_Throws()
    {
        MemoryEvent ev = BuildValidGoalEvent();
        Assert.Throws<ArgumentException>(() =>
            new CallbackCandidate(ev, Fixed.Half, ""));
    }
}

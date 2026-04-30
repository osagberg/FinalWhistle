using System;
using System.Collections.Generic;
using System.Linq;
using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Memory;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Memory;

/// <summary>
/// End-to-end tests for the Phase-3 persistent-development event surface
/// per SPEC line 145. The headline test
/// (<see cref="Breakthrough_EndToEnd_CapReachKeyEventProducesCallback"/>)
/// is the SPEC line 145 acceptance gate: a synthetic
/// <see cref="KeyEventKind.SignatureBreakthrough"/> KeyEvent flows
/// through <see cref="MemoryEmissionRules.EmitForKeyEvents"/> →
/// <see cref="Ledger.Emit"/> → <see cref="BreakthroughReader.QueryForSeason"/>
/// and surfaces as a <see cref="CallbackCandidate"/> carrying the
/// <see cref="BreakthroughReader.BreakthroughTemplateId"/>.
/// </summary>
public sealed class BreakthroughReaderTests
{
    private static KeyEvent BuildBreakthroughKeyEvent(
        long tickValue, TeamSide side, byte jersey)
    {
        return new KeyEvent(
            tick: new Tick(tickValue),
            kind: KeyEventKind.SignatureBreakthrough,
            side: side,
            jerseyNumber: jersey,
            position: Vector3Fixed.Zero);
    }

    [Fact]
    public void Breakthrough_EndToEnd_CapReachKeyEventProducesCallback()
    {
        // Phase-3 SPEC line 145 acceptance: a SignatureBreakthrough
        // KeyEvent flows through the full chain and surfaces as a
        // CallbackCandidate carrying the breakthrough template.
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildBreakthroughKeyEvent(tickValue: 1500, TeamSide.Home, jersey: 6),
        };
        IReadOnlyList<IdentityPacket> homePackets = LoadArchetypePackets("direct-pressing");
        IReadOnlyList<IdentityPacket> awayPackets = LoadArchetypePackets("low-block-counter");

        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents: keyEvents,
            matchId: "0xdeadbeef",
            season: 1,
            careerDate: new CareerDate(1, 5, 3),
            weights: SalienceWeights.Phase3Defaults,
            homePackets: homePackets,
            awayPackets: awayPackets);

        Assert.Single(events);
        MemoryEvent ev = events[0];
        Assert.Equal(EventClass.SignatureBreakthrough, ev.What);
        Assert.Equal(Emotion.Triumph, ev.Emotion);
        Assert.Equal(MemoryEmissionRules.Phase3BreakthroughStakes, ev.Stakes);
        // Per the salience formula with breakthrough-tuned inputs:
        //   0.4·1.0 + 0.2·0.6 + 0.2·0.9 + 0 + 0 = 0.40 + 0.12 + 0.18 = 0.70.
        // Notable band — well above the 0.60 floor.
        Assert.Equal(SalienceBand.Notable, SalienceEngine.ClassifyBand(ev.Salience));

        Ledger ledger = new();
        ledger.Emit(ev);

        BreakthroughReader reader = new(ledger);
        List<CallbackCandidate> candidates = reader.QueryForSeason(1).ToList();

        Assert.Single(candidates);
        CallbackCandidate candidate = candidates[0];
        Assert.Equal(BreakthroughReader.BreakthroughTemplateId, candidate.Template);
        Assert.Equal(ev.Salience, candidate.SurfacingSalience);
        Assert.Equal(ev.Id, candidate.Source.Id);
    }

    [Fact]
    public void Breakthrough_TagBreakthroughHasNeverExpiry()
    {
        // design/breakthrough-moments.md: "Permanent. Awakenings are
        // irreversible." Tag-level guard pins this — Expiry must be Never.
        Assert.IsType<ExpiryPolicy.Never>(CallbackTagRegistry.SignatureBreakthrough.Expiry);
    }

    [Fact]
    public void Breakthrough_OldEvent_StillSurfaces_BecauseExpiryIsNever()
    {
        // Same input from 10 seasons ago — still surfaces (expiry=never).
        // Compare to PressFan (Expiry=Seasons(3)) where a 10-season-old
        // event would be filtered out.
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildBreakthroughKeyEvent(tickValue: 100, TeamSide.Home, jersey: 6),
        };
        IReadOnlyList<IdentityPacket> homePackets = LoadArchetypePackets("direct-pressing");
        IReadOnlyList<IdentityPacket> awayPackets = LoadArchetypePackets("low-block-counter");
        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents, "0xtest", season: 1, new CareerDate(1, 1, 1),
            SalienceWeights.Phase3Defaults,
            homePackets: homePackets, awayPackets: awayPackets);
        Ledger ledger = new();
        ledger.Emit(events[0]);

        BreakthroughReader reader = new(ledger);
        ReaderQuery wideWindow = new(
            tagId: CallbackTagRegistry.SignatureBreakthroughId,
            fromSeason: 0, toSeason: 11, currentSeason: 11,
            minBand: SalienceBand.Notable);
        List<CallbackCandidate> candidates = reader.Query(wideWindow).ToList();
        Assert.Single(candidates);
    }

    [Fact]
    public void Breakthrough_ReaderRejectsPressFanTag()
    {
        // BreakthroughReader is registered as consumer of
        // signature-breakthrough only. Querying with PressFanId via
        // BreakthroughReader must throw at the registry boundary.
        Ledger ledger = new();
        BreakthroughReader reader = new(ledger);
        ReaderQuery wrongTagQuery = new(
            tagId: CallbackTagRegistry.PressFanId,
            fromSeason: 0, toSeason: 1, currentSeason: 1,
            minBand: SalienceBand.Notable);

        var ex = Assert.Throws<InvalidOperationException>(() =>
            reader.Query(wrongTagQuery).GetEnumerator().MoveNext());
        Assert.Contains("not a registered consumer", ex.Message);
        Assert.Contains(CallbackTagRegistry.PressFanId, ex.Message);
    }

    [Fact]
    public void Breakthrough_PressFanReader_RejectsBreakthroughTag()
    {
        // Symmetric check: PressFanReader is NOT a consumer of
        // signature-breakthrough. Cross-reader pollution must fail loud
        // at the registry boundary.
        Ledger ledger = new();
        PressFanReader reader = new(ledger);
        ReaderQuery wrongTagQuery = new(
            tagId: CallbackTagRegistry.SignatureBreakthroughId,
            fromSeason: 0, toSeason: 1, currentSeason: 1,
            minBand: SalienceBand.Notable);

        Assert.Throws<InvalidOperationException>(() =>
            reader.Query(wrongTagQuery).GetEnumerator().MoveNext());
    }

    [Fact]
    public void Breakthrough_RestartKeyEventsAreNotTranslated()
    {
        // MemoryEmissionRules only translates Goal + SignatureBreakthrough
        // in Phase 3. Restart events stay as routine-band match telemetry;
        // signature-execution events translate Phase-4+.
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            new KeyEvent(new Tick(100), KeyEventKind.GoalKickRestart, TeamSide.Home,
                KeyEvent.JerseyUnspecified, Vector3Fixed.Zero),
            new KeyEvent(new Tick(200), KeyEventKind.SignatureExecuted_LowCutback, TeamSide.Home,
                jerseyNumber: 6, position: Vector3Fixed.Zero),
        };
        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents, "0xtest", 1, new CareerDate(1, 1, 1),
            SalienceWeights.Phase3Defaults);
        Assert.Empty(events);
    }

    [Fact]
    public void EventClassRegistry_SignatureBreakthrough_TagsIncludeBreakthrough()
    {
        IReadOnlyList<string> tags = EventClassRegistry.TagsFor(EventClass.SignatureBreakthrough);
        Assert.Contains(CallbackTagRegistry.SignatureBreakthroughId, tags);
    }

    [Fact]
    public void EventClassRegistry_SignatureBreakthrough_BaseWeightHigherThanGoal()
    {
        // Breakthrough is permanent player-development; base weight
        // should sit above goal-base-weight to ensure breakthroughs rank
        // above goals when both surface in the same window.
        Assert.True(
            EventClassRegistry.BaseWeightFor(EventClass.SignatureBreakthrough) >
            EventClassRegistry.BaseWeightFor(EventClass.GoalScored));
    }

    // ============================================================
    // Codex round-1 P1: participant-identity resolution
    // ============================================================

    [Fact]
    public void Breakthrough_EmitForKeyEvents_ResolvesParticipantFromIdentityPackets()
    {
        // Codex round-1 P1: SignatureBreakthrough KeyEvents carry
        // (TeamSide, JerseyNumber); the resolver maps that to the
        // matching IdentityPacket.PlayerId so the resulting MemoryEvent
        // has a real Participant. The Phase-3 fixture roster's home
        // jersey 6 is "fwh.core:player_00006" per
        // MatchSim/Content/identity-packets/direct-pressing/06.json.
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildBreakthroughKeyEvent(tickValue: 1500, TeamSide.Home, jersey: 6),
        };
        IReadOnlyList<IdentityPacket> homePackets = LoadArchetypePackets("direct-pressing");
        IReadOnlyList<IdentityPacket> awayPackets = LoadArchetypePackets("low-block-counter");

        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents: keyEvents,
            matchId: "0xtest",
            season: 1,
            careerDate: new CareerDate(1, 5, 3),
            weights: SalienceWeights.Phase3Defaults,
            homePackets: homePackets,
            awayPackets: awayPackets);

        Assert.Single(events);
        Assert.Single(events[0].Participants);
        Assert.Equal("player", events[0].Participants[0].Role);
        Assert.Equal("fwh.core:player_00006", events[0].Participants[0].EntityId);
    }

    [Fact]
    public void Breakthrough_EmitForKeyEvents_ResolvesAwayParticipant()
    {
        // Mirror check: away-side breakthrough resolves against the
        // away packets, not home.
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildBreakthroughKeyEvent(tickValue: 1500, TeamSide.Away, jersey: 11),
        };
        IReadOnlyList<IdentityPacket> homePackets = LoadArchetypePackets("direct-pressing");
        IReadOnlyList<IdentityPacket> awayPackets = LoadArchetypePackets("low-block-counter");

        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents, "0xtest", 1, new CareerDate(1, 5, 3),
            SalienceWeights.Phase3Defaults,
            homePackets: homePackets, awayPackets: awayPackets);

        Assert.Single(events);
        Assert.Single(events[0].Participants);
        // Low-block-counter jersey 11 ("Mikel Ostrowski" lone striker per
        // SPEC closure note for line 142) — the loaded packet's PlayerId.
        Assert.StartsWith("fwh.core:player_", events[0].Participants[0].EntityId);
    }

    [Fact]
    public void Breakthrough_EmitForKeyEvents_NullHomePackets_Throws()
    {
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildBreakthroughKeyEvent(tickValue: 1500, TeamSide.Home, jersey: 6),
        };
        Assert.Throws<ArgumentException>(() =>
            MemoryEmissionRules.EmitForKeyEvents(
                keyEvents, "0xtest", 1, new CareerDate(1, 5, 3),
                SalienceWeights.Phase3Defaults,
                homePackets: null, awayPackets: null));
    }

    [Fact]
    public void Breakthrough_EmitForKeyEvents_JerseyUnspecified_Throws()
    {
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildBreakthroughKeyEvent(
                tickValue: 1500, TeamSide.Home, jersey: KeyEvent.JerseyUnspecified),
        };
        IReadOnlyList<IdentityPacket> homePackets = LoadArchetypePackets("direct-pressing");
        IReadOnlyList<IdentityPacket> awayPackets = LoadArchetypePackets("low-block-counter");

        Assert.Throws<ArgumentException>(() =>
            MemoryEmissionRules.EmitForKeyEvents(
                keyEvents, "0xtest", 1, new CareerDate(1, 5, 3),
                SalienceWeights.Phase3Defaults,
                homePackets: homePackets, awayPackets: awayPackets));
    }

    [Fact]
    public void Breakthrough_EmitForKeyEvents_GoalKeyEventStillHasEmptyParticipants()
    {
        // Phase-3 goals don't carry scorer attribution (JerseyUnspecified
        // emission), so the resolver returns empty Participants for goals
        // even when packets are supplied. Phase-4+ scorer-tracking flips
        // this — but for Phase 3 the asymmetry is intentional.
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            new KeyEvent(new Tick(1200), KeyEventKind.Goal, TeamSide.Home,
                KeyEvent.JerseyUnspecified, Vector3Fixed.Zero),
        };
        IReadOnlyList<IdentityPacket> homePackets = LoadArchetypePackets("direct-pressing");
        IReadOnlyList<IdentityPacket> awayPackets = LoadArchetypePackets("low-block-counter");

        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents, "0xtest", 1, new CareerDate(1, 5, 3),
            SalienceWeights.Phase3Defaults,
            homePackets: homePackets, awayPackets: awayPackets);

        Assert.Single(events);
        Assert.Equal(EventClass.GoalScored, events[0].What);
        Assert.Empty(events[0].Participants);
    }

    // ============================================================
    // Codex round-1 P2: all-time default-window for permanent events
    // ============================================================

    [Fact]
    public void Breakthrough_QueryForSeason11_StillSurfacesSeason1Event()
    {
        // Codex round-1 P2: a season-1 breakthrough queried at
        // currentSeason=11 must still surface — Expiry=Never +
        // QueryForSeason all-time default. Previously the reader's
        // 3-season default-window silently filtered it out.
        IReadOnlyList<KeyEvent> keyEvents = new[]
        {
            BuildBreakthroughKeyEvent(tickValue: 1500, TeamSide.Home, jersey: 6),
        };
        IReadOnlyList<IdentityPacket> homePackets = LoadArchetypePackets("direct-pressing");
        IReadOnlyList<IdentityPacket> awayPackets = LoadArchetypePackets("low-block-counter");
        IReadOnlyList<MemoryEvent> events = MemoryEmissionRules.EmitForKeyEvents(
            keyEvents, "0xtest", season: 1, new CareerDate(1, 5, 3),
            SalienceWeights.Phase3Defaults,
            homePackets: homePackets, awayPackets: awayPackets);
        Ledger ledger = new();
        ledger.Emit(events[0]);

        BreakthroughReader reader = new(ledger);
        List<CallbackCandidate> candidates = reader.QueryForSeason(currentSeason: 11).ToList();
        Assert.Single(candidates);
        Assert.Equal(BreakthroughReader.BreakthroughTemplateId, candidates[0].Template);
        // Surfacing salience decays per callback-age modifier; base 0.70,
        // age=10 → decay=0.50 → 0.20 (clamped non-negative).
        Assert.True(candidates[0].SurfacingSalience < candidates[0].Source.Salience,
            "Surfacing salience should decay over 10 seasons.");
    }

    private static IReadOnlyList<IdentityPacket> LoadArchetypePackets(string archetype)
    {
        IdentityPacket[] packets = new IdentityPacket[IdentityPackets.PlayersPerArchetype];
        for (byte jersey = 1; jersey <= IdentityPackets.PlayersPerArchetype; jersey++)
        {
            packets[jersey - 1] = IdentityPackets.Load(archetype, jersey);
        }
        return packets;
    }

    [Fact]
    public void Breakthrough_SalienceLandsInNotableBand_NotSeasonDefining()
    {
        // Per pr-review-toolkit:type-design-analyzer 2026-04-30 finding
        // #3: pin the band classification of a Phase-3 breakthrough so a
        // future weight-table tuning that drifts the breakthrough into a
        // different band fails loudly. The salience formula's max with
        // rivalry+rarity=0 is 0.80 (under SeasonDefining's 0.85 cutoff);
        // Phase-3 breakthroughs naturally land in Notable. Phase-4+
        // rivalry/rarity wiring will lift contextually-relevant
        // breakthroughs to SeasonDefining without changing this base
        // case. The tag-level Expiry=Never is the load-bearing
        // permanence guard, not the band scalar.
        SalienceInputs inputs = new(
            stakes: MemoryEmissionRules.Phase3BreakthroughStakes,
            participantProminenceAvg: MemoryEmissionRules.Phase3PlaceholderProminence,
            eventClassBaseWeight: EventClassRegistry.BaseWeightFor(EventClass.SignatureBreakthrough),
            rivalryBoost: Fixed.Zero,
            rarityBoost: Fixed.Zero);
        Fixed salience = SalienceEngine.Compute(inputs, SalienceWeights.Phase3Defaults);
        Assert.Equal(SalienceBand.Notable, SalienceEngine.ClassifyBand(salience));
        Assert.True(salience >= SalienceEngine.NotableThreshold,
            $"Breakthrough salience {salience} dropped below NotableThreshold " +
            $"{SalienceEngine.NotableThreshold} — tag MinBand=Notable would filter it out.");
        Assert.True(salience < SalienceEngine.SeasonDefiningThreshold,
            $"Breakthrough salience {salience} crossed into SeasonDefining " +
            $"{SalienceEngine.SeasonDefiningThreshold} — Phase-4+ rivalry/rarity " +
            "wiring is the planned path to SeasonDefining; an unintended drift " +
            "in Phase-3 placeholders would mask Phase-4 work.");
    }
}

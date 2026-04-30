using System;
using System.Collections.Generic;
using System.Globalization;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Memory;

/// <summary>
/// Bridge from canonical <see cref="KeyEvent"/> stream to
/// <see cref="MemoryEvent"/> stream per ADR-0004 §"Implementation
/// Guidelines" / decoupling rule: <c>MatchSim never touches the ledger</c>.
/// Lives outside <c>MatchSimulationRunner</c> — callers (test fixtures
/// today, a Phase-4+ Match orchestration layer tomorrow) invoke
/// <see cref="EmitForKeyEvents"/> after a match completes (or per
/// ledger-cycle boundary) to translate canonical events into ledger
/// entries.
///
/// <para>
/// <strong>Phase-3 minimum scope</strong>: only
/// <see cref="KeyEventKind.Goal"/> → <see cref="EventClass.GoalScored"/>
/// is wired. Restart events + signature-execution events are NOT
/// translated to <see cref="MemoryEvent"/>s yet — Phase-4+ adds
/// <c>SignatureAwakened</c>/<c>SignatureExecuted</c> classes when the
/// lifecycle ships, restart events stay as match-telemetry only per
/// ADR-0004 §"routine band write-to-ledger cutoff."
/// </para>
///
/// <para>
/// <strong>Determinism</strong>: salience computation is
/// <see cref="Fixed"/>-only (no <c>double</c>); event-id is
/// deterministic-from-inputs (<c>match:&lt;matchId&gt;:tick:&lt;tick&gt;:seq:&lt;n&gt;</c>);
/// no <see cref="Guid"/>, no <see cref="DateTime"/>. Same input → same
/// MemoryEvent stream byte-for-byte.
/// </para>
/// </summary>
public static class MemoryEmissionRules
{
    /// <summary>
    /// Phase-3 placeholder for emission-time stakes. Phase-3 has no
    /// real stakes signal (no career context yet), so a synthetic
    /// value is needed that lifts <see cref="EventClass.GoalScored"/>
    /// events into the Notable band so the press-fan reader actually
    /// surfaces them. 0.95 with the Phase3Defaults weight table gives
    /// <c>(0.4·0.95 + 0.2·0.6 + 0.2·0.6 + 0 + 0) ≈ 0.62</c>, leaving
    /// a ~2 ULP margin above <see cref="SalienceEngine.NotableThreshold"/>
    /// = 0.60 so Q32.32 multiply-rounding cannot flip the band
    /// classification at the boundary. Phase-4+ replaces with a real
    /// stakes signal derived from fixture-context (cup final / derby
    /// / relegation match).
    /// </summary>
    public static readonly Fixed Phase3PlaceholderStakes = Fixed.Parse("0.9500000000");

    /// <summary>
    /// Phase-3 placeholder for participant-prominence average. Phase-4+
    /// computes from the player's actual profile (rep + form + record).
    /// </summary>
    public static readonly Fixed Phase3PlaceholderProminence = Fixed.Parse("0.6000000000");

    /// <summary>
    /// Translate a completed match's canonical KeyEvent stream into
    /// MemoryEvents. Caller appends results to the
    /// <see cref="Ledger"/> via <see cref="Ledger.Emit"/>.
    /// </summary>
    /// <param name="keyEvents">Canonical KeyEvent stream from
    /// <see cref="MatchSimulationState.KeyEvents"/>. Read-only;
    /// never mutated.</param>
    /// <param name="matchId">Stable per-match identifier; used in the
    /// deterministic <see cref="MemoryEvent.Id"/>. Phase-3 callers
    /// commonly pass the match-seed hex string.</param>
    /// <param name="season">In-world season the match belongs to.</param>
    /// <param name="careerDate">Save-world date of the match.</param>
    /// <param name="weights">Salience weights to apply. Production is
    /// <see cref="SalienceWeights.Phase3Defaults"/>; tests substitute
    /// custom weights to exercise specific salience scenarios.</param>
    /// <param name="eventSeqStart">Starting sequence number for the
    /// generated <see cref="MemoryEvent.Id"/>s. Default 0; pass the
    /// running ledger size to chain matches without ID collisions
    /// across the same career.</param>
    public static IReadOnlyList<MemoryEvent> EmitForKeyEvents(
        IReadOnlyList<KeyEvent> keyEvents,
        string matchId,
        ushort season,
        CareerDate careerDate,
        SalienceWeights weights,
        int eventSeqStart = 0)
    {
        if (keyEvents is null) throw new ArgumentNullException(nameof(keyEvents));
        if (string.IsNullOrEmpty(matchId)) throw new ArgumentException("matchId must be non-empty.", nameof(matchId));
        if (eventSeqStart < 0)
        {
            throw new ArgumentOutOfRangeException(
                nameof(eventSeqStart), eventSeqStart, "eventSeqStart must be non-negative.");
        }

        List<MemoryEvent> results = new();
        int seq = eventSeqStart;
        EventEmitter emitter = new(EmitterKind.Match, matchId);

        for (int i = 0; i < keyEvents.Count; i++)
        {
            KeyEvent ke = keyEvents[i];
            if (ke.Kind != KeyEventKind.Goal)
            {
                // Phase-3: only Goal translates. Restart events stay as
                // routine-band match telemetry; signature-execution
                // events translate Phase-4+.
                continue;
            }

            uint tickValue = ToUInt32Tick(ke.Tick.Value);
            string eventId = string.Format(
                CultureInfo.InvariantCulture,
                "match:{0}:tick:{1}:seq:{2}",
                matchId, tickValue, seq);

            SalienceInputs inputs = new(
                stakes: Phase3PlaceholderStakes,
                participantProminenceAvg: Phase3PlaceholderProminence,
                eventClassBaseWeight: EventClassRegistry.BaseWeightFor(EventClass.GoalScored),
                rivalryBoost: Fixed.Zero,
                rarityBoost: Fixed.Zero);

            Fixed salience = SalienceEngine.Compute(inputs, weights);

            results.Add(new MemoryEvent(
                id: eventId,
                matchId: matchId,
                season: season,
                tick: tickValue,
                careerDate: careerDate,
                emitter: emitter,
                participants: Array.Empty<Participant>(),
                what: EventClass.GoalScored,
                stakes: Phase3PlaceholderStakes,
                emotion: Emotion.Triumph,
                salience: salience,
                salienceInputs: inputs,
                salienceModelVersion: SalienceWeights.Phase3ModelVersion,
                schemaVersion: MemoryEvent.CurrentSchemaVersion));
            seq++;
        }

        return results;
    }

    private static uint ToUInt32Tick(long tickValue)
    {
        if (tickValue < 0 || tickValue > uint.MaxValue)
        {
            throw new OverflowException(
                $"KeyEvent.Tick.Value {tickValue} exceeds uint range; " +
                "Phase-3 matches stay well under uint.MaxValue ticks.");
        }
        return (uint)tickValue;
    }
}

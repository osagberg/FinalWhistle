using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Memory;

/// <summary>
/// Phase-6 tuning weights for the emission-time salience formula per
/// ADR-0004 §"Salience formula." Stored as <see cref="Fixed"/>
/// throughout — never as <c>double</c> — to keep the salience compute
/// path on the integer-determinism floor that the rest of MatchSim sits
/// on. Same reasoning as the Codex round-9 P3 fix on the
/// <c>SignaturePresentationRecipe</c> integer-ratio deltas.
///
/// <para>
/// <strong>Phase3Defaults</strong> = the Phase-6 tuning seeds from
/// <c>design/event-sourced-memory.md</c> §Phase-6-tuning-seeds:
/// <c>w_stakes=0.4, w_prominence=0.2, w_event_class=0.2, w_rivalry=0.1,
/// w_rarity=0.1</c>. Sum = 1.0 exactly in Q32.32 — verified at static
/// init by <c>SalienceEngine</c> sanity check (Phase-6 balance-harness
/// reruns this).
/// </para>
/// </summary>
public readonly struct SalienceWeights
{
    /// <summary>
    /// Burned into every emitted <see cref="Contracts.MemoryEvent.SalienceModelVersion"/>
    /// so Phase-6 retroactive analysis can identify the weight table that
    /// produced any given historical Salience scalar.
    /// </summary>
    public const ushort Phase3ModelVersion = 1;

    public Fixed WStakes { get; }
    public Fixed WProminence { get; }
    public Fixed WEventClass { get; }
    public Fixed WRivalry { get; }
    public Fixed WRarity { get; }

    public SalienceWeights(
        Fixed wStakes, Fixed wProminence, Fixed wEventClass,
        Fixed wRivalry, Fixed wRarity)
    {
        WStakes = wStakes;
        WProminence = wProminence;
        WEventClass = wEventClass;
        WRivalry = wRivalry;
        WRarity = wRarity;
    }

    /// <summary>
    /// Phase-3 + Phase-6 tuning seeds from
    /// <c>design/event-sourced-memory.md</c>:
    /// <c>w_stakes=0.4, w_prominence=0.2, w_event_class=0.2, w_rivalry=0.1,
    /// w_rarity=0.1</c>. Constructed via <c>Fixed.Parse</c> (decimal-string
    /// path is the canonical Q32.32 entry point per FW-VAL-A-018) so no
    /// <c>double</c> arithmetic appears in the weight setup.
    /// </summary>
    public static readonly SalienceWeights Phase3Defaults = new(
        wStakes: Fixed.Parse("0.4000000000"),
        wProminence: Fixed.Parse("0.2000000000"),
        wEventClass: Fixed.Parse("0.2000000000"),
        wRivalry: Fixed.Parse("0.1000000000"),
        wRarity: Fixed.Parse("0.1000000000"));
}

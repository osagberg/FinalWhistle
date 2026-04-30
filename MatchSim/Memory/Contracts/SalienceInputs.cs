using System;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// 5-input audit trail for the salience scalar per ADR-0004 §"Why
/// SalienceInputs are persisted." Stored alongside <see cref="MemoryEvent.Salience"/>
/// + <see cref="MemoryEvent.SalienceModelVersion"/> so Phase-6
/// balance-harness analysis can retroactively explain why a historical
/// event scored 0.82 vs 0.79 even after weight tables change.
///
/// <para>
/// All five inputs are Q32.32 in <c>[0, 1]</c>. Constructor rejects
/// out-of-range values — silent clamping would lose audit fidelity.
/// </para>
/// </summary>
public readonly struct SalienceInputs : IEquatable<SalienceInputs>
{
    public Fixed Stakes { get; }
    public Fixed ParticipantProminenceAvg { get; }
    public Fixed EventClassBaseWeight { get; }
    public Fixed RivalryBoost { get; }
    public Fixed RarityBoost { get; }

    public SalienceInputs(
        Fixed stakes,
        Fixed participantProminenceAvg,
        Fixed eventClassBaseWeight,
        Fixed rivalryBoost,
        Fixed rarityBoost)
    {
        ThrowIfOutOfUnitRange(stakes, nameof(stakes));
        ThrowIfOutOfUnitRange(participantProminenceAvg, nameof(participantProminenceAvg));
        ThrowIfOutOfUnitRange(eventClassBaseWeight, nameof(eventClassBaseWeight));
        ThrowIfOutOfUnitRange(rivalryBoost, nameof(rivalryBoost));
        ThrowIfOutOfUnitRange(rarityBoost, nameof(rarityBoost));

        Stakes = stakes;
        ParticipantProminenceAvg = participantProminenceAvg;
        EventClassBaseWeight = eventClassBaseWeight;
        RivalryBoost = rivalryBoost;
        RarityBoost = rarityBoost;
    }

    public bool Equals(SalienceInputs other) =>
        Stakes == other.Stakes
        && ParticipantProminenceAvg == other.ParticipantProminenceAvg
        && EventClassBaseWeight == other.EventClassBaseWeight
        && RivalryBoost == other.RivalryBoost
        && RarityBoost == other.RarityBoost;

    public override bool Equals(object? obj) => obj is SalienceInputs other && Equals(other);

    public override int GetHashCode() => HashCode.Combine(
        Stakes, ParticipantProminenceAvg, EventClassBaseWeight, RivalryBoost, RarityBoost);

    public static bool operator ==(SalienceInputs left, SalienceInputs right) => left.Equals(right);
    public static bool operator !=(SalienceInputs left, SalienceInputs right) => !left.Equals(right);

    private static void ThrowIfOutOfUnitRange(Fixed value, string paramName)
    {
        if (value < Fixed.Zero || value > Fixed.One)
        {
            throw new ArgumentOutOfRangeException(paramName, value,
                "SalienceInputs values must lie in [0, 1].");
        }
    }
}

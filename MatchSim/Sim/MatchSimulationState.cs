using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Mutable production state for the Phase-3 deterministic match loop. The
/// arrays are intentionally mutable because the runner overwrites player
/// snapshots in place each tick; callers must preserve roster order.
/// </summary>
public sealed class MatchSimulationState
{
    /// <summary>Current absolute simulation tick.</summary>
    public Tick CurrentTick { get; set; }

    /// <summary>Current canonical ball state.</summary>
    public BallState Ball { get; set; }

    /// <summary>Home players in stable roster order, length 11.</summary>
    public PlayerState[] HomeTeam { get; }

    /// <summary>Away players in stable roster order, length 11.</summary>
    public PlayerState[] AwayTeam { get; }

    public MatchSimulationState(
        Tick currentTick,
        BallState ball,
        PlayerState[] homeTeam,
        PlayerState[] awayTeam)
    {
        HomeTeam = CopyAndValidateTeam(homeTeam, nameof(homeTeam));
        AwayTeam = CopyAndValidateTeam(awayTeam, nameof(awayTeam));
        CurrentTick = currentTick;
        Ball = ball;
    }

    /// <summary>
    /// Build the canonical Month-3 smoke fixture initial state: 22 players at
    /// archetype formation positions, ball supplied by caller, tick supplied
    /// by caller.
    /// </summary>
    public static MatchSimulationState FromArchetypeFormations(
        Tick currentTick,
        BallState ball,
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype)
    {
        if (homeArchetype is null)
        {
            throw new ArgumentNullException(nameof(homeArchetype));
        }
        if (awayArchetype is null)
        {
            throw new ArgumentNullException(nameof(awayArchetype));
        }

        PlayerState[] homeTeam = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        PlayerState[] awayTeam = new PlayerState[MatchCanonicalState.PlayersPerTeam];

        foreach (FormationSlot homeSlot in homeArchetype.Formation)
        {
            int rosterIndex = homeSlot.RosterSlot - 1;
            homeTeam[rosterIndex] = new PlayerState(
                position: homeSlot.HomeBasePosition,
                velocity: Vector3Fixed.Zero,
                jerseyNumber: homeSlot.RosterSlot,
                side: TeamSide.Home);
        }

        foreach (FormationSlot awaySlot in awayArchetype.Formation)
        {
            int rosterIndex = awaySlot.RosterSlot - 1;
            awayTeam[rosterIndex] = new PlayerState(
                position: awaySlot.AwayBasePosition(),
                velocity: Vector3Fixed.Zero,
                jerseyNumber: awaySlot.RosterSlot,
                side: TeamSide.Away);
        }

        return new MatchSimulationState(currentTick, ball, homeTeam, awayTeam);
    }

    private static PlayerState[] CopyAndValidateTeam(PlayerState[] team, string paramName)
    {
        if (team is null)
        {
            throw new ArgumentNullException(paramName);
        }
        if (team.Length != MatchCanonicalState.PlayersPerTeam)
        {
            throw new ArgumentException(
                $"{paramName} must contain exactly {MatchCanonicalState.PlayersPerTeam} players; got {team.Length}.",
                paramName);
        }

        PlayerState[] copy = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        Array.Copy(team, copy, MatchCanonicalState.PlayersPerTeam);
        return copy;
    }
}

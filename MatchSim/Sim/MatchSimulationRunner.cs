using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Deterministic Phase-3 match-loop composition. This owns the canonical step
/// order exercised by the cross-platform hash fixture:
/// BT.Tick × 2 → PlayerActuator.Step × 22 → BallPhysics.Step → Tick+1.
/// </summary>
public static class MatchSimulationRunner
{
    public static void RunTicks(
        MatchSimulationState state,
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype,
        PlayerKinematics kinematics,
        BallPhysicsCoefficients ballCoefficients,
        int ticks)
    {
        if (state is null)
        {
            throw new ArgumentNullException(nameof(state));
        }
        if (homeArchetype is null)
        {
            throw new ArgumentNullException(nameof(homeArchetype));
        }
        if (awayArchetype is null)
        {
            throw new ArgumentNullException(nameof(awayArchetype));
        }
        if (ticks < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(ticks), ticks, "ticks must be non-negative.");
        }

        PlayerCommand[] homeCommands = new PlayerCommand[MatchCanonicalState.PlayersPerTeam];
        PlayerCommand[] awayCommands = new PlayerCommand[MatchCanonicalState.PlayersPerTeam];

        for (int t = 0; t < ticks; t++)
        {
            BehaviorTreeRunner.Tick(state.Ball, state.HomeTeam, state.AwayTeam,
                TeamSide.Home, homeArchetype, kinematics, homeCommands);
            BehaviorTreeRunner.Tick(state.Ball, state.AwayTeam, state.HomeTeam,
                TeamSide.Away, awayArchetype, kinematics, awayCommands);

            for (int i = 0; i < MatchCanonicalState.PlayersPerTeam; i++)
            {
                state.HomeTeam[i] = PlayerActuator.Step(
                    state.HomeTeam[i], homeCommands[i].DesiredPosition,
                    homeCommands[i].DesiredSpeed, kinematics);
            }

            for (int i = 0; i < MatchCanonicalState.PlayersPerTeam; i++)
            {
                state.AwayTeam[i] = PlayerActuator.Step(
                    state.AwayTeam[i], awayCommands[i].DesiredPosition,
                    awayCommands[i].DesiredSpeed, kinematics);
            }

            state.Ball = BallPhysics.Step(state.Ball, ballCoefficients);
            state.CurrentTick = state.CurrentTick + 1L;
        }
    }
}

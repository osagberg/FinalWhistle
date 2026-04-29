using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Deterministic Phase-3 match-loop composition. This owns the canonical step
/// order exercised by the cross-platform hash fixture:
/// BT.Tick × 2 → PlayerActuator.Step × 22 → BallPhysics.Step → MatchRules.Step → Tick+1.
///
/// <para>
/// <strong>Per-tick step order is canonical contract.</strong> Reordering
/// any of the five stages would change the canonical state hash even for
/// byte-identical input. The order locked here:
/// </para>
/// <list type="number">
///   <item><description><see cref="BehaviorTreeRunner.Tick"/> Home (reads ball + both rosters; writes home commands).</description></item>
///   <item><description><see cref="BehaviorTreeRunner.Tick"/> Away (same surface; writes away commands).</description></item>
///   <item><description><see cref="PlayerActuator.Step"/> × 11 Home players (reads home command, writes home player state).</description></item>
///   <item><description><see cref="PlayerActuator.Step"/> × 11 Away players (same).</description></item>
///   <item><description><see cref="BallPhysics.Step"/> (reads ball + ball coefficients; writes ball state).</description></item>
///   <item><description><see cref="MatchRules.Step"/> (reads pre-step + post-step ball; mutates score / OutOfPlay / KeyEvents / ball-on-restart).</description></item>
///   <item><description>Tick advances by 1.</description></item>
/// </list>
///
/// <para>
/// <strong><see cref="MatchSimulationConfig"/> input</strong> per SPEC
/// 2026-04-29 decisions-log entry (closes Codex P2-03 seed-input refactor):
/// the runner accepts a <see cref="Seed"/> via <see cref="MatchSimulationConfig.MatchSeed"/>
/// even though Phase-3 deterministic code does not consume it yet. Wiring
/// the seed through the runner now means the corpus-replay command threads
/// cleanly when Phase-3 Week-3+ <c>fw replay &lt;seed&gt;</c> ships, and
/// Phase 4+ stochastic events derive their RNG streams from it via
/// <see cref="Seed.Derive"/>.
/// </para>
/// </summary>
public static class MatchSimulationRunner
{
    public static void RunTicks(
        MatchSimulationState state,
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype,
        PlayerKinematics kinematics,
        BallPhysicsCoefficients ballCoefficients,
        MatchSimulationConfig config,
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

        // config.MatchSeed is intentionally unused at Phase 3 — it travels
        // through the runner so corpus fixtures can record it and Phase-4+
        // stochastic events can derive RNG streams from it. Suppress the
        // unused-warning by reading it once into a discard.
        _ = config;

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

            // Cache pre-step ball before BallPhysics mutates state.Ball, so
            // MatchRules.Step has both pre and post available for crossing
            // detection.
            BallState preStepBall = state.Ball;
            state.Ball = BallPhysics.Step(state.Ball, ballCoefficients);
            MatchRules.Step(state, preStepBall);

            state.CurrentTick = state.CurrentTick + 1L;
        }
    }
}

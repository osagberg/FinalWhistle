using System;
using FinalWhistle.MatchSim.Content;

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
///
/// <para>
/// <strong>Phase-3 restarts are EVENT-ONLY placeholders.</strong> Per the
/// 2026-04-30 decisions-log entry (Codex P2 round 4): when
/// <see cref="MatchRules.Step"/> emits a <see cref="KeyEventKind.GoalKickRestart"/>
/// / <see cref="KeyEventKind.ThrowInRestart"/> / <see cref="KeyEventKind.CornerKickRestart"/>,
/// the side recorded on the <see cref="KeyEvent"/> is informational ONLY.
/// The runner does NOT consume any restart-control state on the next tick
/// — there is no possession lock, no taker assignment, no BT-suppression.
/// The ball respawns at the canonical spot with zero velocity, and on the
/// very next tick both BTs run normally and the nearest-player heuristic
/// determines who picks it up. Phase 4 introduces real possession-lock +
/// taker behavior; until then, the canonical sim treats restarts as
/// observable events but not gameplay-authoritative state.
/// </para>
/// </summary>
public static class MatchSimulationRunner
{
    /// <summary>
    /// Legacy overload preserved for callers that don't engage the
    /// Phase-3 signature-dispatch layer. Internally calls the
    /// signature-aware overload with empty <see cref="IdentityPacket"/>
    /// arrays — <see cref="SignatureRules.Step"/> short-circuits on
    /// empty inputs so the canonical-state output is byte-identical to
    /// a no-signature-step run. Pinned 60-tick determinism hash stays
    /// green via this path.
    ///
    /// <para>
    /// <strong>Signature config</strong> is fixed at
    /// <see cref="SignatureConfig.Phase3Defaults"/> when going through
    /// this overload; because the packet arrays are empty,
    /// <see cref="SignatureRules.Step"/> short-circuits before consulting
    /// the config, so the value is never read. A Phase-4+ caller that
    /// needs both signature-suppression AND a non-default config should
    /// call the signature-aware overload directly with explicit empty
    /// packet arrays + the desired config — the legacy overload here
    /// will be retired once Phase-4 signatures fire in real corpus
    /// fixtures and "no signatures" stops being the runtime default.
    /// (Per feature-dev:code-reviewer 2026-04-30 round-2 finding 3.)
    /// </para>
    /// </summary>
    public static void RunTicks(
        MatchSimulationState state,
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype,
        PlayerKinematics kinematics,
        BallPhysicsCoefficients ballCoefficients,
        MatchSimulationConfig config,
        int ticks)
    {
        RunTicks(
            state, homeArchetype, awayArchetype,
            Array.Empty<IdentityPacket>(),
            Array.Empty<IdentityPacket>(),
            kinematics, ballCoefficients, config,
            SignatureConfig.Phase3Defaults,
            ticks);
    }

    /// <summary>
    /// Phase-3 signature-aware <c>RunTicks</c>. Identical to the
    /// legacy overload PLUS:
    /// <list type="bullet">
    ///   <item><description>Allocates a process-lifetime
    ///       <see cref="SignatureCooldownState"/> at match start.</description></item>
    ///   <item><description>Inserts <see cref="SignatureRules.Step"/>
    ///       in the per-tick step order immediately after
    ///       <see cref="MatchRules.Step"/>:
    ///       <c>BT.Tick × 2 → PlayerActuator.Step × 22 → BallPhysics.Step
    ///       → MatchRules.Step → SignatureRules.Step → Tick+1</c>.</description></item>
    /// </list>
    /// Empty <paramref name="homePackets"/>/<paramref name="awayPackets"/>
    /// arrays = signature dispatch is a no-op; canonical hash unchanged
    /// vs. the legacy overload.
    /// </summary>
    public static void RunTicks(
        MatchSimulationState state,
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype,
        IdentityPacket[] homePackets,
        IdentityPacket[] awayPackets,
        PlayerKinematics kinematics,
        BallPhysicsCoefficients ballCoefficients,
        MatchSimulationConfig config,
        SignatureConfig signatureConfig,
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
        if (homePackets is null)
        {
            throw new ArgumentNullException(nameof(homePackets));
        }
        if (awayPackets is null)
        {
            throw new ArgumentNullException(nameof(awayPackets));
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
        // Per-match cooldown lives on MatchSimulationState (closes Codex round-9
        // P1): chunked-tick callers (viewer/replay loops driving ticks=1 in a
        // hot loop) reuse the same SignatureCooldownState instance across every
        // RunTicks invocation, so per-match cooldown windows + fire caps stay
        // honored. Allocating fresh inside this method would let signatures
        // re-fire past their per-match caps once per RunTicks call.
        SignatureCooldownState cooldown = state.SignatureCooldown;

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

            // Phase-3 signature dispatch runs AFTER MatchRules because
            // signature triggers consult the post-rules canonical state
            // (e.g. ball respawn after a goal kick reset). Passes
            // `kinematics` so SignatureRules.IsCarrier reads the same
            // possession radius PlayerActuator uses.
            SignatureRules.Step(state, homePackets, awayPackets, kinematics, cooldown, signatureConfig);

            state.CurrentTick = state.CurrentTick + 1L;
        }
    }
}

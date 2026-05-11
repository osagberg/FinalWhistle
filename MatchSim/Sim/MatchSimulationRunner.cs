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
        // Convenience overload that allocates fresh PlayerCommand[] scratch
        // buffers per call. Used by tests + headless batch runs where GC
        // pressure is not a concern. Live FixedUpdate-driven callers should
        // use the buffer-accepting overload below to avoid allocating two
        // 11-element arrays per tick (Codex round-1 P2 follow-up against
        // 2a79529: at 60Hz × 90min = 324_000 fresh allocations per match,
        // which becomes a real GC concern once Slice 4+ adds shot cameras
        // and Slice 5+ adds URP custom passes that compete for the same
        // managed heap).
        PlayerCommand[] homeCommands = new PlayerCommand[MatchCanonicalState.PlayersPerTeam];
        PlayerCommand[] awayCommands = new PlayerCommand[MatchCanonicalState.PlayersPerTeam];
        RunTicks(state, homeArchetype, awayArchetype,
            homePackets, awayPackets,
            kinematics, ballCoefficients, config, signatureConfig,
            homeCommands, awayCommands, ticks);
    }

    /// <summary>
    /// Buffer-reusing overload per Codex round-1 P2 follow-up against
    /// <c>2a79529</c>: live <c>FixedUpdate</c>-driven callers (Slice 3
    /// dots adapter; future viewer / replay loops) cache <see cref="PlayerCommand"/>
    /// scratch buffers once at session start + reuse them across every
    /// <c>RunTicks</c> call to avoid the per-tick allocation.
    ///
    /// <para>
    /// <strong>Determinism:</strong> the buffers are pure write-then-read
    /// scratch space within a single tick — BT writes commands, actuators
    /// read them in the same iteration. Reusing the buffer across ticks
    /// is safe because every cell is unconditionally rewritten by the BT
    /// before the actuator reads it. Canonical-state output is byte-identical
    /// to the fresh-allocation overload above; the
    /// <c>Match_RunTicks_BufferReusing_ProducesIdenticalCanonicalState</c>
    /// regression in MatchSim.Tests pins this claim.
    /// </para>
    ///
    /// <para>
    /// <strong>Buffer length contract:</strong> both buffers MUST be
    /// exactly <see cref="MatchCanonicalState.PlayersPerTeam"/> entries.
    /// Throws on length mismatch — silently truncating or overflowing
    /// would silently corrupt canonical state.
    /// </para>
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
        PlayerCommand[] homeCommandBuffer,
        PlayerCommand[] awayCommandBuffer,
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
        if (homeCommandBuffer is null)
        {
            throw new ArgumentNullException(nameof(homeCommandBuffer));
        }
        if (awayCommandBuffer is null)
        {
            throw new ArgumentNullException(nameof(awayCommandBuffer));
        }
        if (homeCommandBuffer.Length != MatchCanonicalState.PlayersPerTeam)
        {
            throw new ArgumentException(
                $"homeCommandBuffer must have {MatchCanonicalState.PlayersPerTeam} entries; got {homeCommandBuffer.Length}.",
                nameof(homeCommandBuffer));
        }
        if (awayCommandBuffer.Length != MatchCanonicalState.PlayersPerTeam)
        {
            throw new ArgumentException(
                $"awayCommandBuffer must have {MatchCanonicalState.PlayersPerTeam} entries; got {awayCommandBuffer.Length}.",
                nameof(awayCommandBuffer));
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
                TeamSide.Home, homeArchetype, kinematics, homeCommandBuffer);
            BehaviorTreeRunner.Tick(state.Ball, state.AwayTeam, state.HomeTeam,
                TeamSide.Away, awayArchetype, kinematics, awayCommandBuffer);

            for (int i = 0; i < MatchCanonicalState.PlayersPerTeam; i++)
            {
                state.HomeTeam[i] = PlayerActuator.Step(
                    state.HomeTeam[i], homeCommandBuffer[i].DesiredPosition,
                    homeCommandBuffer[i].DesiredSpeed, kinematics);
            }

            for (int i = 0; i < MatchCanonicalState.PlayersPerTeam; i++)
            {
                state.AwayTeam[i] = PlayerActuator.Step(
                    state.AwayTeam[i], awayCommandBuffer[i].DesiredPosition,
                    awayCommandBuffer[i].DesiredSpeed, kinematics);
            }

            // Phase-3 polish-pass Option 1 (2026-05-11): inter-player soft
            // collision. Runs AFTER PlayerActuator.Step×22 + BEFORE the
            // kick-apply step. Mutates player positions in-place to enforce
            // 2×Kinematics.Radius minimum spacing. Velocity unchanged.
            // Single pass per tick — no convergence loop; next tick's BT
            // redistributes naturally.
            PlayerSeparation.Step(state, kinematics);

            // Phase-3 pass-the-ball: apply any KickIntent from the carrier
            // BEFORE BallPhysics.Step so the kicked velocity gets integrated
            // by gravity / drag / friction this tick. Order: home kicks
            // applied first, then away — last-writer-wins on the rare same-
            // tick double-emit. The BT's carrier-kick gate (ball velocity <
            // threshold + carrier-on-ball) suppresses double-emit in
            // practice because if Home's BT just kicked, ball.Velocity is
            // immediately above the gate when Away's BT runs.
            //
            // Why apply AFTER PlayerActuator + BEFORE BallPhysics: the
            // actuator doesn't read kicks, so order vs actuator is free;
            // applying BEFORE BallPhysics gives the kick instant effect
            // (gravity + drag integrate the kicked velocity this tick) —
            // applying AFTER would idle the ball for one tick before motion.
            state.Ball = ApplyTeamKickIfAny(state.Ball, homeCommandBuffer);
            state.Ball = ApplyTeamKickIfAny(state.Ball, awayCommandBuffer);

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

    /// <summary>
    /// Phase-3 pass-the-ball: scan one team's command buffer for a
    /// <see cref="KickIntent"/> and write it onto <paramref name="ball"/>.
    /// At most one entry per team has a non-null Kick (the carrier; per
    /// <see cref="BehaviorTreeRunner"/>'s emission gate); first-found wins
    /// — same deterministic order as the buffer write.
    /// </summary>
    /// <remarks>
    /// Spin currently always Zero per the Magnus-stub policy; the field is
    /// plumbed through KickIntent so a Phase-4+ spin-aware ball physics
    /// pass picks it up without further schema changes.
    /// </remarks>
    private static BallState ApplyTeamKickIfAny(BallState ball, PlayerCommand[] commandBuffer)
    {
        for (int i = 0; i < commandBuffer.Length; i++)
        {
            KickIntent? kick = commandBuffer[i].Kick;
            if (kick.HasValue)
            {
                // Position is unchanged — the carrier is on the ball;
                // re-stating ball.Position keeps the canonical-state
                // encoding stable. Velocity = kick.Velocity (pitch plane).
                // Spin = kick.Spin (Zero for Phase 3).
                return new BallState(ball.Position, kick.Value.Velocity, kick.Value.Spin);
            }
        }
        return ball;
    }
}

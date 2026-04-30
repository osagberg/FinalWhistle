using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Content;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Phase-3 active-signature trigger detection orchestrator. Mirrors the
/// <see cref="MatchRules"/> pattern: pure-static
/// <see cref="Step(MatchSimulationState, IdentityPacket[], IdentityPacket[], PlayerKinematics, SignatureCooldownState, SignatureConfig)"/>
/// runs immediately after <see cref="MatchRules.Step(MatchSimulationState, BallState)"/>
/// each tick, reading the post-rules canonical state, checking the three
/// Phase-3 active signatures (#13 / #20 / #22 per the locked 2026-04-24
/// month-3-vertical-slice resolution), and emitting:
///
/// <list type="bullet">
///   <item><description>A <see cref="KeyEvent"/> with the corresponding
///       <see cref="KeyEventKind"/> SignatureExecuted_* value into
///       <see cref="MatchSimulationState.KeyEvents"/> (canonical state;
///       byte-for-byte stable; in the determinism hash).</description></item>
///   <item><description>A parallel <see cref="SignatureExecution"/>
///       record into <see cref="MatchSimulationState.SignatureRecipes"/>
///       (presentation metadata; NOT in the canonical hash).</description></item>
/// </list>
///
/// <para>
/// <strong>Phase-3 simplifications</strong>:
/// </para>
/// <list type="bullet">
///   <item><description><strong>Affinity gate is fixture-declared.</strong>
///       Players whose <see cref="IdentityPacket.SignatureCandidates"/>
///       list contains the matching signature ID are eligible. The full
///       Phase-4+ "trigger-history-builds-readiness" awakening lifecycle
///       per <c>design/signatures.md</c> §lifecycle is deferred.</description></item>
///   <item><description><strong>Trigger detection is positional.</strong>
///       Each signature checks coarse spatial conditions (carrier near
///       byline / carrier in box during wide ball delivery / CM with
///       moving ball in middle third) — sufficient for Month-3 narrative
///       legibility. Phase-4+ swaps in true possession-transition events
///       + ball-trajectory analysis.</description></item>
///   <item><description><strong>Sim bias is metadata-only.</strong> The
///       presentation recipe carries the bias field ID + Q32.32 delta;
///       Phase-3 does NOT modify ball physics or player kinematics on
///       fire. Phase-4 wires real <c>SimBiasSnapshot</c> dispatch per
///       ADR-0005.</description></item>
///   <item><description><strong>Cooldown + per-match cap</strong> via
///       <see cref="SignatureCooldownState"/> prevent per-tick spam.
///       Both bounds tuned in <see cref="SignatureConfig.Phase3Defaults"/>
///       so the smoke fixture cannot fire any signature (spatial conditions
///       fail at centre-field with stationary ball + formation-position
///       players).</description></item>
/// </list>
/// </summary>
public static class SignatureRules
{
    // The three Phase-3 signature IDs per ADR-0005 + the locked
    // 2026-04-24 month-3-vertical-slice resolution. Pinned constants
    // so the affinity-membership check stays allocation-free.
    private const string SigIdLowCutback =
        "fwh.core:signature.low-cutback-from-byline";
    private const string SigIdBlindSideRun =
        "fwh.core:signature.blind-side-near-post-run";
    private const string SigIdDiagonalSwitch =
        "fwh.core:signature.first-time-diagonal-switch";

    /// <summary>
    /// Pre-computed presentation recipes per Phase-3 signature kind.
    /// Static-readonly singleton; allocation-free at hot-path access.
    /// Recipe metadata authored per <c>design/signatures.md</c> +
    /// <c>design/semantic-cinema.md</c> 7-shot vocabulary mapping.
    ///
    /// <para>
    /// Q32.32 raw deltas are constructed via INTEGER-RATIO arithmetic on
    /// <see cref="Fixed.OneRaw"/> (closes Codex round-9 P3) so no
    /// <see cref="double"/> arithmetic appears in the MatchSim canonical
    /// path. The recipe deltas are not currently in
    /// <c>MatchCanonicalState.Write</c>, but they ARE deterministic
    /// presentation metadata that will feed ViewerEvent traces, and
    /// admitting a <c>(long)(0.20 * (1L &lt;&lt; 32))</c> precedent here
    /// risks float arithmetic creeping into corpus-feeding paths later.
    /// Each constant below documents its decimal intent + integer ratio.
    /// </para>
    /// </summary>
    // 0.20 = 1/5. Fixed.OneRaw / 5 = 858993459 (truncates 0.2 by ~0.5 ULP).
    private const long Delta020Raw = Fixed.OneRaw / 5L;
    // 0.25 = 1/4. Exact in Q32.32 (Fixed.OneRaw / 4 = 1073741824).
    private const long Delta025Raw = Fixed.OneRaw / 4L;
    // 0.18 = 18/100. Fixed.OneRaw * 18 / 100 = 773094113.
    // Multiplied first to preserve integer-truncation semantics; safe in
    // long arithmetic (Fixed.OneRaw * 18 = 77,309,411,328 ≪ long.MaxValue).
    private const long Delta018Raw = Fixed.OneRaw * 18L / 100L;

    private static readonly SignaturePresentationRecipe RecipeLowCutback = new(
        signatureId: SigIdLowCutback,
        recipeKey: "player-isolation",
        simBiasFieldId: "cutback_xAssist",
        simBiasDeltaRawQ32: Delta020Raw);

    private static readonly SignaturePresentationRecipe RecipeBlindSideRun = new(
        signatureId: SigIdBlindSideRun,
        recipeKey: "pass-shot-impact",
        simBiasFieldId: "near_post_xG",
        simBiasDeltaRawQ32: Delta025Raw);

    private static readonly SignaturePresentationRecipe RecipeDiagonalSwitch = new(
        signatureId: SigIdDiagonalSwitch,
        recipeKey: "tactical-wide",
        simBiasFieldId: "diagonal_switch_trigger",
        simBiasDeltaRawQ32: Delta018Raw);

    /// <summary>
    /// Step the signature dispatch forward one canonical tick. Iterates
    /// both teams; for each player whose <see cref="IdentityPacket"/>
    /// declares affinity for one of the three Phase-3 signatures, checks
    /// the role-and-spatial trigger condition + cooldown + per-match cap;
    /// on fire, emits the <see cref="KeyEvent"/> + recipe pair.
    /// </summary>
    public static void Step(
        MatchSimulationState state,
        IdentityPacket[] homePackets,
        IdentityPacket[] awayPackets,
        PlayerKinematics kinematics,
        SignatureCooldownState cooldown,
        SignatureConfig config)
    {
        if (state is null)
        {
            throw new ArgumentNullException(nameof(state));
        }
        if (homePackets is null)
        {
            throw new ArgumentNullException(nameof(homePackets));
        }
        if (awayPackets is null)
        {
            throw new ArgumentNullException(nameof(awayPackets));
        }
        if (cooldown is null)
        {
            throw new ArgumentNullException(nameof(cooldown));
        }

        // Phase-3 packet-shape contract (closes Codex round-9 P2): callers
        // are in EXACTLY one of two valid states:
        //   (a) Both arrays empty — legacy / signature-suppressed run.
        //       Short-circuits to byte-identical canonical hash.
        //   (b) Both arrays exactly 11 non-null packets per side — Phase-3
        //       full-roster signature dispatch.
        // Any other shape (10 packets, mismatched lengths, a single null
        // entry) is a roster-loader bug; the previous Math.Min + null-skip
        // guard silently dropped signature eligibility for misaligned slots
        // instead of failing at the boundary.
        bool legacyEmpty = homePackets.Length == 0 && awayPackets.Length == 0;
        if (!legacyEmpty)
        {
            ValidateFullRoster(homePackets, nameof(homePackets));
            ValidateFullRoster(awayPackets, nameof(awayPackets));
        }
        if (legacyEmpty)
        {
            return;
        }

        long currentTick = state.CurrentTick.Value;

        CheckAllPlayers(state, state.HomeTeam, homePackets, TeamSide.Home,
            kinematics, cooldown, config, currentTick);
        CheckAllPlayers(state, state.AwayTeam, awayPackets, TeamSide.Away,
            kinematics, cooldown, config, currentTick);
    }

    private static void ValidateFullRoster(IdentityPacket[] packets, string paramName)
    {
        if (packets.Length != MatchCanonicalState.PlayersPerTeam)
        {
            throw new ArgumentException(
                $"{paramName} must contain exactly 0 (legacy / signatures-suppressed) or " +
                $"{MatchCanonicalState.PlayersPerTeam} (Phase-3 full-roster) packets; got {packets.Length}. " +
                $"Mixed lengths or partial rosters silently drop signature eligibility for misaligned slots.",
                paramName);
        }
        for (int i = 0; i < packets.Length; i++)
        {
            if (packets[i] is null)
            {
                throw new ArgumentException(
                    $"{paramName}[{i}] is null. Phase-3 signature dispatch requires every roster slot to carry " +
                    $"an IdentityPacket — a single null entry would silently disable that player's affinity gate.",
                    paramName);
            }
        }
    }

    private static void CheckAllPlayers(
        MatchSimulationState state,
        PlayerState[] team,
        IdentityPacket[] packets,
        TeamSide side,
        PlayerKinematics kinematics,
        SignatureCooldownState cooldown,
        SignatureConfig config,
        long currentTick)
    {
        // Step's ValidateFullRoster guarantees packets.Length == team.Length
        // and every entry non-null on this code path; iterate the full roster.
        for (int i = 0; i < team.Length; i++)
        {
            PlayerState player = team[i];
            IdentityPacket packet = packets[i];

            // Cheap pre-filter: if the player has no signature candidates
            // at all, skip the role-family + position checks entirely.
            if (packet.SignatureCandidates.Count == 0)
            {
                continue;
            }

            int playerIndex = SignatureCooldownState.PlayerIndex(side, player.JerseyNumber);

            // Per-signature dispatch. Each signature checks its specific
            // trigger condition; if more than one signature matches the
            // same player on the same tick (rare but possible — e.g. a
            // CM in the box during a cross), all matching signatures
            // fire independently. Phase-4 may add interaction policy.
            if (HasCandidate(packet, SigIdLowCutback)
                && packet.RoleFamily == RoleFamily.Winger)
            {
                TryFireLowCutback(state, player, side, playerIndex,
                    kinematics, cooldown, config, currentTick);
            }
            if (HasCandidate(packet, SigIdBlindSideRun)
                && packet.RoleFamily == RoleFamily.Striker)
            {
                TryFireBlindSideRun(state, player, side, playerIndex,
                    cooldown, config, currentTick);
            }
            if (HasCandidate(packet, SigIdDiagonalSwitch)
                && packet.RoleFamily == RoleFamily.CentralMidfielder)
            {
                TryFireDiagonalSwitch(state, player, side, playerIndex,
                    kinematics, cooldown, config, currentTick);
            }
        }
    }

    /// <summary>
    /// True iff <paramref name="packet"/>'s candidates list contains the
    /// given signature ID. Linear scan over a small list (Phase-3 caps
    /// at 3 per packet); allocation-free.
    /// </summary>
    private static bool HasCandidate(IdentityPacket packet, string signatureId)
    {
        IReadOnlyList<SignatureCandidate> candidates = packet.SignatureCandidates;
        for (int i = 0; i < candidates.Count; i++)
        {
            if (candidates[i].SignatureId == signatureId)
            {
                return true;
            }
        }
        return false;
    }

    // ----- #20 Low cutback from the byline ---------------------------

    private static void TryFireLowCutback(
        MatchSimulationState state, PlayerState player, TeamSide side,
        int playerIndex, PlayerKinematics kinematics,
        SignatureCooldownState cooldown, SignatureConfig config,
        long currentTick)
    {
        if (!cooldown.CanFire(SignatureKind.LowCutback, playerIndex,
            currentTick, config.LowCutbackCooldownTicks, config.LowCutbackMaxFires))
        {
            return;
        }

        // Possession check (carrier-distance-to-ball within kinematics radius).
        if (!IsCarrier(player, state.Ball, kinematics))
        {
            return;
        }

        // Spatial check: near the attacking byline + in a wide channel +
        // moving laterally (cutback intent).
        Fixed bylineXForSide = side == TeamSide.Home ? MatchRules.GoalLineX : -MatchRules.GoalLineX;
        Fixed distanceToByline = AbsFixed(bylineXForSide - player.Position.X);
        if (distanceToByline > config.BylineProximityMetres)
        {
            return;
        }
        if (AbsFixed(player.Position.Z) <= config.WideChannelZThreshold)
        {
            return;
        }
        if (AbsFixed(player.Velocity.Z) <= config.MinLateralSpeed)
        {
            return;
        }

        EmitSignature(state, KeyEventKind.SignatureExecuted_LowCutback,
            side, player, RecipeLowCutback);
        RecordFireAndMaybeEmitBreakthrough(
            state, side, player, SignatureKind.LowCutback, playerIndex,
            cooldown, currentTick, config.LowCutbackMaxFires);
    }

    // ----- #22 Blind-side near-post run ------------------------------
    //
    // Architectural note (per feature-dev:code-reviewer 2026-04-30, finding
    // 4 / confidence 88): #22 has NO IsCarrier check — by design. The
    // signature fires for the OFF-BALL striker making a near-post run TO
    // RECEIVE a cross, not for the ball-carrier. The ball-wide-in-attacking-
    // half + striker-forward-velocity-in-box conditions proxy the cross-
    // delivery moment. Asymmetry vs. #20 (carrier-driven) and #13 (carrier-
    // driven) is intentional and load-bearing for football realism.

    private static void TryFireBlindSideRun(
        MatchSimulationState state, PlayerState player, TeamSide side,
        int playerIndex, SignatureCooldownState cooldown, SignatureConfig config,
        long currentTick)
    {
        if (!cooldown.CanFire(SignatureKind.BlindSideNearPostRun, playerIndex,
            currentTick, config.BlindSideRunCooldownTicks, config.BlindSideRunMaxFires))
        {
            return;
        }

        // Spatial check: striker in the attacking penalty area.
        Fixed goalLineForSide = side == TeamSide.Home ? MatchRules.GoalLineX : -MatchRules.GoalLineX;
        Fixed distanceToGoalLine = AbsFixed(goalLineForSide - player.Position.X);
        if (distanceToGoalLine > config.PenaltyAreaDepthMetres)
        {
            return;
        }
        if (AbsFixed(player.Position.Z) >= config.PenaltyAreaHalfWidthMetres)
        {
            return;
        }

        // Striker must be in the attacking half of the pitch.
        if (side == TeamSide.Home && player.Position.X <= Fixed.Zero)
        {
            return;
        }
        if (side == TeamSide.Away && player.Position.X >= Fixed.Zero)
        {
            return;
        }

        // Ball must be in a cross-delivery position: in the attacking
        // half + wide.
        if (side == TeamSide.Home && state.Ball.Position.X <= Fixed.Zero)
        {
            return;
        }
        if (side == TeamSide.Away && state.Ball.Position.X >= Fixed.Zero)
        {
            return;
        }
        if (AbsFixed(state.Ball.Position.Z) <= config.CrossDeliveryWideZThreshold)
        {
            return;
        }

        // Player velocity has a non-trivial forward + curve component.
        Fixed forwardVel = side == TeamSide.Home ? player.Velocity.X : -player.Velocity.X;
        if (forwardVel <= config.MinForwardRunSpeed)
        {
            return;
        }
        if (AbsFixed(player.Velocity.Z) <= config.MinNearPostCurveSpeed)
        {
            return;
        }

        EmitSignature(state, KeyEventKind.SignatureExecuted_BlindSideNearPostRun,
            side, player, RecipeBlindSideRun);
        RecordFireAndMaybeEmitBreakthrough(
            state, side, player, SignatureKind.BlindSideNearPostRun, playerIndex,
            cooldown, currentTick, config.BlindSideRunMaxFires);
    }

    // ----- #13 First-time diagonal switch -----------------------------

    private static void TryFireDiagonalSwitch(
        MatchSimulationState state, PlayerState player, TeamSide side,
        int playerIndex, PlayerKinematics kinematics,
        SignatureCooldownState cooldown, SignatureConfig config,
        long currentTick)
    {
        if (!cooldown.CanFire(SignatureKind.FirstTimeDiagonalSwitch, playerIndex,
            currentTick, config.DiagonalSwitchCooldownTicks, config.DiagonalSwitchMaxFires))
        {
            return;
        }

        if (!IsCarrier(player, state.Ball, kinematics))
        {
            return;
        }

        // CM is in the middle third of the pitch.
        if (AbsFixed(player.Position.X) >= config.MiddleThirdHalfDepthMetres)
        {
            return;
        }

        // Phase-3 simplification per the architect blueprint: the "first-
        // time touch" condition needs a possession-transition tracker
        // we don't have at Phase 3. Proxy: require the BALL to have
        // non-trivial velocity in BOTH X and Z (ball is moving and the
        // CM is touching it = a one-touch switch is plausible).
        if (AbsFixed(state.Ball.Velocity.X) <= config.MinBallSpeedForSwitch)
        {
            return;
        }
        if (AbsFixed(state.Ball.Velocity.Z) <= config.MinBallSpeedForSwitch)
        {
            return;
        }

        // Defensive: don't fire off a recent restart (the ball-respawn
        // velocity is zero, so the velocity check above already rules
        // this out, but the doc-comment mention is load-bearing).

        EmitSignature(state, KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch,
            side, player, RecipeDiagonalSwitch);
        RecordFireAndMaybeEmitBreakthrough(
            state, side, player, SignatureKind.FirstTimeDiagonalSwitch, playerIndex,
            cooldown, currentTick, config.DiagonalSwitchMaxFires);
    }

    // ----- Shared helpers --------------------------------------------

    /// <summary>
    /// Carrier check: player position is within the canonical possession
    /// radius of the ball position. Reads <c>kinematics.Radius</c> directly
    /// so any future tuning of the canonical possession radius affects
    /// signature dispatch + possession in lockstep (per pr-review-toolkit
    /// 2026-04-30 round-2 findings — silent-failure-hunter MEDIUM #1 +
    /// feature-dev:code-reviewer #2 / confidence 83). Mirrors the
    /// <see cref="PlayerActuator.HasPossession"/> sqrt-free
    /// distance-squared comparison.
    /// </summary>
    private static bool IsCarrier(PlayerState player, BallState ball, PlayerKinematics kinematics)
    {
        Fixed radiusSquared = kinematics.Radius * kinematics.Radius;
        Vector3Fixed playerPitchPos = new(player.Position.X, Fixed.Zero, player.Position.Z);
        Vector3Fixed ballPitchPos = new(ball.Position.X, Fixed.Zero, ball.Position.Z);
        Fixed distSquared = Vector3Fixed.DistanceSquared(playerPitchPos, ballPitchPos);
        return distSquared <= radiusSquared;
    }

    private static void EmitSignature(
        MatchSimulationState state, KeyEventKind kind, TeamSide side,
        PlayerState player, SignaturePresentationRecipe recipe)
    {
        KeyEvent keyEvent = new(
            tick: state.CurrentTick,
            kind: kind,
            side: side,
            jerseyNumber: player.JerseyNumber,
            position: player.Position);
        state.KeyEvents.Add(keyEvent);
        state.SignatureRecipes.Add(new SignatureExecution(
            keyEventIndex: state.KeyEvents.Count - 1,
            recipe: recipe));
    }

    /// <summary>
    /// Record the signature fire and, if the resulting per-match
    /// fire-count equals the documented cap, emit a parallel
    /// <see cref="KeyEventKind.SignatureBreakthrough"/> KeyEvent per
    /// SPEC line 145 + <c>design/breakthrough-moments.md</c>'s
    /// "third time today" pattern. The breakthrough KeyEvent is the
    /// Phase-3 minimum persistent-development surface — its presence
    /// in <see cref="MatchSimulationState.KeyEvents"/> drives Memory-
    /// layer translation + reader callback discovery downstream.
    ///
    /// <para>
    /// <strong>No <see cref="SignaturePresentationRecipe"/> entry</strong>
    /// is added for the breakthrough event: the recipe stream is
    /// scoped to signature-execution events specifically (per the
    /// stream's documentation on <see cref="MatchSimulationState.SignatureRecipes"/>);
    /// the breakthrough's presentation metadata flows through the
    /// MemoryEvent → BreakthroughReader → CallbackTemplateId path,
    /// not the signature-recipe stream. This keeps the recipe stream's
    /// 1:1 mapping to <c>SignatureExecuted_*</c> KeyEvents intact.
    /// </para>
    /// </summary>
    private static void RecordFireAndMaybeEmitBreakthrough(
        MatchSimulationState state, TeamSide side, PlayerState player,
        SignatureKind signature, int playerIndex,
        SignatureCooldownState cooldown, long currentTick, byte maxFiresPerMatch)
    {
        bool capReached = cooldown.RecordFireAndDidReachCap(
            signature, playerIndex, currentTick, maxFiresPerMatch);
        if (!capReached)
        {
            return;
        }

        KeyEvent breakthrough = new(
            tick: state.CurrentTick,
            kind: KeyEventKind.SignatureBreakthrough,
            side: side,
            jerseyNumber: player.JerseyNumber,
            position: player.Position);
        state.KeyEvents.Add(breakthrough);
    }

    private static Fixed AbsFixed(Fixed value) => value < Fixed.Zero ? -value : value;
}

using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Unit tests for <see cref="SignatureRules.Step"/> per SPEC.md Phase-3
/// line 143 (3 active signatures end-to-end).
///
/// <para>
/// Three coverage axes per the architect blueprint + Codex round-7's
/// "be strict at the boundaries" precedent:
/// </para>
/// <list type="bullet">
///   <item><description><strong>Carrier-affinity gate</strong>: only
///       players whose <see cref="IdentityPacket.SignatureCandidates"/>
///       contains the matching signature ID + whose
///       <see cref="IdentityPacket.RoleFamily"/> matches the signature's
///       designed role can fire. Empty packets / wrong role / missing
///       affinity all suppress.</description></item>
///   <item><description><strong>Per-signature trigger condition</strong>:
///       each of #13/#20/#22 fires when the spatial+kinematic conditions
///       are met (carrier-position / ball-position / velocity).</description></item>
///   <item><description><strong>Cooldown + per-match cap</strong>: same
///       carrier in the same condition can only fire once per cooldown
///       window + bounded total fires per match.</description></item>
/// </list>
/// </summary>
public sealed class SignatureRulesTests
{
    // The three Phase-3 signature IDs (mirror of SignatureRules.cs constants).
    private const string SigIdLowCutback = "fwh.core:signature.low-cutback-from-byline";
    private const string SigIdBlindSideRun = "fwh.core:signature.blind-side-near-post-run";
    private const string SigIdDiagonalSwitch = "fwh.core:signature.first-time-diagonal-switch";

    // Helper Q32.32 raw constants for affinity weights.
    private static readonly long HalfWeight = Fixed.One.RawValue / 2L;

    private static Fixed F(int n) => Fixed.FromInt(n);
    private static Vector3Fixed V3(int x, int y, int z) => new(F(x), F(y), F(z));

    /// <summary>
    /// Build a 22-packet roster where home jersey 6 is a Winger carrier
    /// of #20, home jersey 11 is a Striker carrier of #22, home jersey
    /// 7 is a CM carrier of #13, and the remaining 19 players carry
    /// nothing.
    /// </summary>
    private static (IdentityPacket[] home, IdentityPacket[] away) BuildCarrierRoster()
    {
        IdentityPacket[] home = new IdentityPacket[11];
        IdentityPacket[] away = new IdentityPacket[11];

        for (int i = 0; i < 11; i++)
        {
            byte jersey = (byte)(i + 1);
            home[i] = MakeNonCarrier($"fwh.core:player_{(10 + i):D5}", jersey, RoleFamily.CentralMidfielder);
            away[i] = MakeNonCarrier($"fwh.core:player_{(20 + i):D5}", jersey, RoleFamily.CentralMidfielder);
        }

        // Home jersey 6 (winger, carries #20).
        home[5] = MakeCarrier("fwh.core:player_00006", 6, RoleFamily.Winger, SigIdLowCutback);
        // Home jersey 7 (CM, carries #13).
        home[6] = MakeCarrier("fwh.core:player_00007", 7, RoleFamily.CentralMidfielder, SigIdDiagonalSwitch);
        // Home jersey 11 (striker, carries #22).
        home[10] = MakeCarrier("fwh.core:player_00011", 11, RoleFamily.Striker, SigIdBlindSideRun);

        return (home, away);
    }

    private static IdentityPacket MakeCarrier(string playerId, byte jersey, RoleFamily role, string sigId)
        => new()
        {
            PlayerId = playerId,
            DisplayNameFull = "Test Carrier",
            DisplayNameShort = "T. Carrier",
            RoleFamily = role,
            SignatureCandidates = new[]
            {
                new SignatureCandidate { SignatureId = sigId, AffinityWeightRaw = HalfWeight },
            },
            Genes = MakeNeutralGenes(),
            SchemaVersion = 1,
            SourcePackVersion = "1.0.0",
        };

    private static IdentityPacket MakeNonCarrier(string playerId, byte jersey, RoleFamily role)
        => new()
        {
            PlayerId = playerId,
            DisplayNameFull = "Test NonCarrier",
            DisplayNameShort = "T. NonCarrier",
            RoleFamily = role,
            SignatureCandidates = System.Array.Empty<SignatureCandidate>(),
            Genes = MakeNeutralGenes(),
            SchemaVersion = 1,
            SourcePackVersion = "1.0.0",
        };

    private static IdentityPacketGenes MakeNeutralGenes() => new()
    {
        FastTwitchRawQ32 = HalfWeight,
        PatternRecognitionRawQ32 = HalfWeight,
        DecisionVelocityRawQ32 = HalfWeight,
        FirstTouchRawQ32 = HalfWeight,
        StrikingRawQ32 = HalfWeight,
        LeftFootRawQ32 = HalfWeight,
    };

    private static MatchSimulationState BuildState(
        BallState ball, PlayerState[] home, PlayerState[] away)
        => new(Tick.Zero, ball, home, away);

    /// <summary>
    /// Helper: 22 default players at origin, override specific slots.
    /// </summary>
    private static (PlayerState[] home, PlayerState[] away) BuildTeams()
    {
        PlayerState[] home = new PlayerState[11];
        PlayerState[] away = new PlayerState[11];
        for (byte i = 1; i <= 11; i++)
        {
            home[i - 1] = new PlayerState(Vector3Fixed.Zero, Vector3Fixed.Zero, i, TeamSide.Home);
            away[i - 1] = new PlayerState(Vector3Fixed.Zero, Vector3Fixed.Zero, i, TeamSide.Away);
        }
        return (home, away);
    }

    // ============================================================
    // #20 Low cutback from the byline (Winger)
    // ============================================================

    [Fact]
    public void LowCutback_CarrierAtByline_Fires()
    {
        var (home, away) = BuildTeams();
        // Home winger (jersey 6) at +X byline, wide channel, moving laterally.
        // Position (50, 0, 22) — within 3m of GoalLineX=52.5, |Z|=22 > 20m
        // wide threshold. Velocity (0, 0, 3) — lateral speed > 1m/s.
        home[5] = new PlayerState(V3(50, 0, 22), V3(0, 0, 3), 6, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(50, 0, 22), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.SignatureExecuted_LowCutback, state.KeyEvents[0].Kind);
        Assert.Equal(TeamSide.Home, state.KeyEvents[0].Side);
        Assert.Equal(6, state.KeyEvents[0].JerseyNumber);
        Assert.Single(state.SignatureRecipes);
        Assert.Equal(SigIdLowCutback, state.SignatureRecipes[0].Recipe.SignatureId);
    }

    [Fact]
    public void LowCutback_NonCarrier_DoesNotFire()
    {
        // Same spatial conditions but home jersey 6 is a non-carrier
        // (built without affinity in MakeNonCarrier).
        var (home, away) = BuildTeams();
        home[5] = new PlayerState(V3(50, 0, 22), V3(0, 0, 3), 6, TeamSide.Home);

        IdentityPacket[] homePackets = new IdentityPacket[11];
        IdentityPacket[] awayPackets = new IdentityPacket[11];
        for (int i = 0; i < 11; i++)
        {
            homePackets[i] = MakeNonCarrier($"fwh.core:player_{i:D5}", (byte)(i + 1), RoleFamily.Winger);
            awayPackets[i] = MakeNonCarrier($"fwh.core:player_{(i + 11):D5}", (byte)(i + 1), RoleFamily.Winger);
        }

        var state = BuildState(
            new BallState(V3(50, 0, 22), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        Assert.Empty(state.KeyEvents);
        Assert.Empty(state.SignatureRecipes);
    }

    [Fact]
    public void LowCutback_WrongRole_DoesNotFire()
    {
        // Same spatial conditions but the carrier has RoleFamily.CentralMidfielder
        // — fails the role-family gate even with the affinity present.
        var (home, away) = BuildTeams();
        home[5] = new PlayerState(V3(50, 0, 22), V3(0, 0, 3), 6, TeamSide.Home);

        IdentityPacket[] homePackets = new IdentityPacket[11];
        IdentityPacket[] awayPackets = new IdentityPacket[11];
        for (int i = 0; i < 11; i++)
        {
            homePackets[i] = MakeNonCarrier($"fwh.core:player_{i:D5}", (byte)(i + 1), RoleFamily.CentralMidfielder);
            awayPackets[i] = MakeNonCarrier($"fwh.core:player_{(i + 11):D5}", (byte)(i + 1), RoleFamily.CentralMidfielder);
        }
        homePackets[5] = MakeCarrier("fwh.core:player_00006", 6, RoleFamily.CentralMidfielder, SigIdLowCutback);

        var state = BuildState(
            new BallState(V3(50, 0, 22), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        Assert.Empty(state.KeyEvents);
    }

    [Fact]
    public void LowCutback_NotAtByline_DoesNotFire()
    {
        // Carrier far from byline (at midfield). Spatial check fails.
        var (home, away) = BuildTeams();
        home[5] = new PlayerState(V3(10, 0, 22), V3(0, 0, 3), 6, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(10, 0, 22), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        Assert.Empty(state.KeyEvents);
    }

    // ============================================================
    // #22 Blind-side near-post run (Striker)
    // ============================================================

    [Fact]
    public void BlindSideRun_CarrierInBoxBallWide_Fires()
    {
        var (home, away) = BuildTeams();
        // Home striker (jersey 11) in attacking box: (49, 0, 10) —
        // distance to goal-line = 3.5m < 16m penalty depth; |Z|=10 < 20m;
        // forward velocity (X+) and lateral curve (Z component).
        home[10] = new PlayerState(V3(49, 0, 10), V3(2, 0, 2), 11, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        // Ball wide in attacking half: (20, 0, 25) — Z=25 > 15m wide threshold.
        var state = BuildState(
            new BallState(V3(20, 0, 25), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.SignatureExecuted_BlindSideNearPostRun, state.KeyEvents[0].Kind);
        Assert.Equal(11, state.KeyEvents[0].JerseyNumber);
        Assert.Single(state.SignatureRecipes);
        Assert.Equal("pass-shot-impact", state.SignatureRecipes[0].Recipe.RecipeKey);
    }

    [Fact]
    public void BlindSideRun_BallNotWide_DoesNotFire()
    {
        // Striker in box with right velocity but ball is central, not wide.
        var (home, away) = BuildTeams();
        home[10] = new PlayerState(V3(49, 0, 10), V3(2, 0, 2), 11, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(20, 0, 5), Vector3Fixed.Zero, Vector3Fixed.Zero),  // |Z|=5 < 15
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        Assert.Empty(state.KeyEvents);
    }

    // ============================================================
    // #13 First-time diagonal switch (CentralMidfielder)
    // ============================================================

    [Fact]
    public void DiagonalSwitch_CarrierMidfieldMovingBall_Fires()
    {
        var (home, away) = BuildTeams();
        // Home CM (jersey 7) at (5, 0, 5) — middle third (|X|=5 < 25);
        // ball at same position so carrier owns possession; ball moving
        // both axes.
        home[6] = new PlayerState(V3(5, 0, 5), Vector3Fixed.Zero, 7, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(5, 0, 5), V3(2, 0, 2), Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch, state.KeyEvents[0].Kind);
        Assert.Equal(7, state.KeyEvents[0].JerseyNumber);
        Assert.Single(state.SignatureRecipes);
    }

    [Fact]
    public void DiagonalSwitch_BallStationary_DoesNotFire()
    {
        // Same as above except ball velocity is zero — fails the moving-
        // ball gate. This proves that the smoke fixture (ball at rest)
        // cannot fire #13.
        var (home, away) = BuildTeams();
        home[6] = new PlayerState(V3(5, 0, 5), Vector3Fixed.Zero, 7, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(5, 0, 5), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        Assert.Empty(state.KeyEvents);
    }

    [Fact]
    public void DiagonalSwitch_OutsideMiddleThird_DoesNotFire()
    {
        // CM in defensive third (|X|=30 > 25m middle-third boundary);
        // condition fails.
        var (home, away) = BuildTeams();
        home[6] = new PlayerState(V3(-30, 0, 5), Vector3Fixed.Zero, 7, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(-30, 0, 5), V3(2, 0, 2), Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        Assert.Empty(state.KeyEvents);
    }

    // ============================================================
    // Cooldown + per-match-cap behavior
    // ============================================================

    [Fact]
    public void LowCutback_SecondFireWithinCooldown_DoesNotFire()
    {
        var (home, away) = BuildTeams();
        home[5] = new PlayerState(V3(50, 0, 22), V3(0, 0, 3), 6, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(50, 0, 22), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();
        var config = SignatureConfig.Phase3Defaults;

        // First fire at tick 0.
        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown, config);
        Assert.Single(state.KeyEvents);

        // Advance 60 ticks (well under 180-tick cooldown), conditions
        // still met, second call must NOT fire.
        state.CurrentTick = state.CurrentTick + 60L;
        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown, config);
        Assert.Single(state.KeyEvents);
    }

    [Fact]
    public void LowCutback_FireAfterCooldownExpiry_FiresAgain()
    {
        var (home, away) = BuildTeams();
        home[5] = new PlayerState(V3(50, 0, 22), V3(0, 0, 3), 6, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(50, 0, 22), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();
        var config = SignatureConfig.Phase3Defaults;

        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown, config);
        Assert.Single(state.KeyEvents);

        // Advance past the 180-tick cooldown window.
        state.CurrentTick = state.CurrentTick + 181L;
        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown, config);
        Assert.Equal(2, state.KeyEvents.Count);
    }

    [Fact]
    public void LowCutback_MaxFiresExceeded_StopsFiring()
    {
        var (home, away) = BuildTeams();
        home[5] = new PlayerState(V3(50, 0, 22), V3(0, 0, 3), 6, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(50, 0, 22), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();
        var config = SignatureConfig.Phase3Defaults;

        // Fire 3 times with cooldown gaps between (max = 3).
        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown, config);
        state.CurrentTick = state.CurrentTick + 200L;
        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown, config);
        state.CurrentTick = state.CurrentTick + 200L;
        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown, config);
        Assert.Equal(3, state.KeyEvents.Count);

        // Fourth attempt — past cooldown but max-fires reached.
        state.CurrentTick = state.CurrentTick + 200L;
        SignatureRules.Step(state, homePackets, awayPackets, PlayerKinematics.Phase3Defaults, cooldown, config);
        Assert.Equal(3, state.KeyEvents.Count);
    }

    // ============================================================
    // Empty-packets short-circuit (legacy-overload path)
    // ============================================================

    [Fact]
    public void Step_EmptyPacketArrays_ShortCircuits()
    {
        // The legacy MatchSimulationRunner.RunTicks overload passes
        // Array.Empty<IdentityPacket>() — must produce zero events
        // even if conditions are met.
        var (home, away) = BuildTeams();
        home[5] = new PlayerState(V3(50, 0, 22), V3(0, 0, 3), 6, TeamSide.Home);
        var state = BuildState(
            new BallState(V3(50, 0, 22), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state,
            System.Array.Empty<IdentityPacket>(),
            System.Array.Empty<IdentityPacket>(),
            PlayerKinematics.Phase3Defaults,
            cooldown, SignatureConfig.Phase3Defaults);

        Assert.Empty(state.KeyEvents);
        Assert.Empty(state.SignatureRecipes);
    }

    // ============================================================
    // Multi-signature same-tick + blind-side off-ball architecture
    // ============================================================

    [Fact]
    public void MultiSignatureSameTick_TwoPlayersTwoSignatures_BothFireWithSequentialIndices()
    {
        // Closes Codex round-9 P2 finding "Multi-fire regression test does
        // not test multi-fire": prior construction only fired #13 and
        // asserted Single, which would still pass under a hypothetical
        // bug that capped Step() at one event per tick. This test pins
        // the documented behavior — Step() iterates ALL players on BOTH
        // teams and emits independent KeyEvent + SignatureRecipe pairs
        // per matched signature — by constructing a state where two
        // distinct signatures fire in the same tick.
        //
        // Construction (verified against the trigger conditions in
        // SignatureRules.cs):
        //
        //   #13 First-time diagonal switch (CM jersey 7):
        //     - CM at (15, 0, 18) — middle-third (|X|=15 < 25m).
        //     - Ball at (15, 0, 18) — coincident with CM → IsCarrier
        //       passes (distance 0 < kinematics radius 0.5).
        //     - Ball velocity (3, 0, 3) — both X and Z components
        //       exceed MinBallSpeedForSwitch (1 m/s).
        //
        //   #22 Blind-side near-post run (Striker jersey 11):
        //     - Striker at (49, 0, 8) — distance to GoalLineX=52.5 is
        //       3.5m < 16m PenaltyAreaDepth; |Z|=8 < 20m
        //       PenaltyAreaHalfWidth; X=49 > 0 (attacking half for Home).
        //     - Striker velocity (3, 0, 2) — forward 3 > MinForward 1;
        //       lateral 2 > MinNearPostCurve 1.
        //     - Ball X=15 > 0 (attacking half) and |Z|=18 > 15m
        //       CrossDeliveryWideZ — proxies a wide cross delivery.
        //
        // Iteration order: CheckAllPlayers iterates Home roster slots
        // 0..10 first, then Away. CM is at index 6, Striker at index 10
        // — so CM fires first → KeyEvents[0] is #13, KeyEvents[1] is #22,
        // and SignatureRecipes[0].KeyEventIndex == 0,
        //                  SignatureRecipes[1].KeyEventIndex == 1.

        var (home, away) = BuildTeams();
        home[6] = new PlayerState(V3(15, 0, 18), Vector3Fixed.Zero, 7, TeamSide.Home);
        home[10] = new PlayerState(V3(49, 0, 8), V3(3, 0, 2), 11, TeamSide.Home);

        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(15, 0, 18), V3(3, 0, 3), Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets,
            PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        // Two distinct signatures fire on the same tick.
        Assert.Equal(2, state.KeyEvents.Count);
        Assert.Equal(2, state.SignatureRecipes.Count);

        // Order: CM (jersey 7, roster index 6) precedes Striker
        // (jersey 11, roster index 10).
        Assert.Equal(KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch,
            state.KeyEvents[0].Kind);
        Assert.Equal(7, state.KeyEvents[0].JerseyNumber);
        Assert.Equal(KeyEventKind.SignatureExecuted_BlindSideNearPostRun,
            state.KeyEvents[1].Kind);
        Assert.Equal(11, state.KeyEvents[1].JerseyNumber);

        // Recipe-stream indices point back into KeyEvents in order.
        Assert.Equal(0, state.SignatureRecipes[0].KeyEventIndex);
        Assert.Equal(1, state.SignatureRecipes[1].KeyEventIndex);
        Assert.Equal(SigIdDiagonalSwitch, state.SignatureRecipes[0].Recipe.SignatureId);
        Assert.Equal(SigIdBlindSideRun, state.SignatureRecipes[1].Recipe.SignatureId);
    }

    [Fact]
    public void BlindSideRun_StrikerNotCarrier_StillFires()
    {
        // Per feature-dev:code-reviewer 2026-04-30 round-2 finding #4
        // (confidence 88): #22 has NO IsCarrier check by design — the
        // signature fires for the OFF-BALL striker making a near-post
        // run TO RECEIVE a cross. Asymmetry vs #20 / #13 (both
        // carrier-driven) is intentional and load-bearing for football
        // realism. This test pins that asymmetry so a future refactor
        // that adds IsCarrier to TryFireBlindSideRun fails loudly.
        var (home, away) = BuildTeams();
        // Striker at (49, 0, 10) — in attacking box, forward+curve
        // velocity. Ball at (20, 0, 25) — wide cross-delivery position,
        // 30+ metres from the striker.
        home[10] = new PlayerState(V3(49, 0, 10), V3(2, 0, 2), 11, TeamSide.Home);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        var state = BuildState(
            new BallState(V3(20, 0, 25), Vector3Fixed.Zero, Vector3Fixed.Zero),
            home, away);
        var cooldown = new SignatureCooldownState();

        SignatureRules.Step(state, homePackets, awayPackets,
            PlayerKinematics.Phase3Defaults, cooldown,
            SignatureConfig.Phase3Defaults);

        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.SignatureExecuted_BlindSideNearPostRun,
            state.KeyEvents[0].Kind);
    }

    // ============================================================
    // Argument validation
    // ============================================================

    [Fact]
    public void Step_NullState_Throws()
    {
        var (homePackets, awayPackets) = BuildCarrierRoster();
        Assert.Throws<System.ArgumentNullException>(() =>
            SignatureRules.Step(null!, homePackets, awayPackets,
                PlayerKinematics.Phase3Defaults,
                new SignatureCooldownState(), SignatureConfig.Phase3Defaults));
    }

    [Fact]
    public void Step_NullHomePackets_Throws()
    {
        var (home, away) = BuildTeams();
        var state = BuildState(BallState.AtRest, home, away);
        Assert.Throws<System.ArgumentNullException>(() =>
            SignatureRules.Step(state, null!, System.Array.Empty<IdentityPacket>(),
                PlayerKinematics.Phase3Defaults,
                new SignatureCooldownState(), SignatureConfig.Phase3Defaults));
    }

    [Fact]
    public void Step_NullCooldown_Throws()
    {
        var (home, away) = BuildTeams();
        var state = BuildState(BallState.AtRest, home, away);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        Assert.Throws<System.ArgumentNullException>(() =>
            SignatureRules.Step(state, homePackets, awayPackets,
                PlayerKinematics.Phase3Defaults, null!,
                SignatureConfig.Phase3Defaults));
    }

    // ============================================================
    // Packet-shape validation (Codex round-9 P2 — closes silent
    // signature-suppression on partial / null / mismatched rosters)
    // ============================================================

    [Fact]
    public void Step_ShortRosterTenPackets_ThrowsArgumentException()
    {
        // Per Codex round-9 P2: a 10-packet array (one slot missing) was
        // previously accepted via Math.Min(team.Length, packets.Length)
        // which silently dropped the 11th roster slot's signature
        // eligibility. Now we throw at the boundary so a roster-loader
        // bug fails loudly instead of producing a silently-wrong run.
        var (home, away) = BuildTeams();
        var state = BuildState(BallState.AtRest, home, away);
        IdentityPacket[] homePackets = new IdentityPacket[10];
        IdentityPacket[] awayPackets = new IdentityPacket[11];
        for (int i = 0; i < 10; i++)
        {
            homePackets[i] = MakeNonCarrier($"fwh.core:player_{i:D5}", (byte)(i + 1), RoleFamily.CentralMidfielder);
        }
        for (int i = 0; i < 11; i++)
        {
            awayPackets[i] = MakeNonCarrier($"fwh.core:player_{(i + 11):D5}", (byte)(i + 1), RoleFamily.CentralMidfielder);
        }

        var ex = Assert.Throws<System.ArgumentException>(() =>
            SignatureRules.Step(state, homePackets, awayPackets,
                PlayerKinematics.Phase3Defaults, new SignatureCooldownState(),
                SignatureConfig.Phase3Defaults));
        Assert.Contains("homePackets", ex.Message);
    }

    [Fact]
    public void Step_MismatchedLengths_ThrowsArgumentException()
    {
        // 11 home packets + 0 away packets is NOT a valid combination —
        // both must be empty (legacy / signatures-suppressed) or both
        // exactly 11 (full-roster).
        var (home, away) = BuildTeams();
        var state = BuildState(BallState.AtRest, home, away);
        var (homePackets, _) = BuildCarrierRoster();

        Assert.Throws<System.ArgumentException>(() =>
            SignatureRules.Step(state, homePackets, System.Array.Empty<IdentityPacket>(),
                PlayerKinematics.Phase3Defaults, new SignatureCooldownState(),
                SignatureConfig.Phase3Defaults));
    }

    [Fact]
    public void Step_FullRosterWithSingleNullEntry_ThrowsArgumentException()
    {
        // A single null packet at jersey-7 slot would silently disable
        // that player's affinity gate under the previous null-skip
        // defensive. Now: throws at the boundary so a packet-loader bug
        // fails loudly.
        var (home, away) = BuildTeams();
        var state = BuildState(BallState.AtRest, home, away);
        var (homePackets, awayPackets) = BuildCarrierRoster();
        homePackets[6] = null!;

        var ex = Assert.Throws<System.ArgumentException>(() =>
            SignatureRules.Step(state, homePackets, awayPackets,
                PlayerKinematics.Phase3Defaults, new SignatureCooldownState(),
                SignatureConfig.Phase3Defaults));
        Assert.Contains("homePackets[6]", ex.Message);
    }
}

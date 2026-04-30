using System;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Phase-3 PitchRules unit tests per SPEC 2026-04-28 PitchRules decisions-log
/// entry. Direct unit-test against <c>MatchRules.Step</c> — exercises the
/// orchestrator with crafted pre/post ball states. Determinism + integration
/// coverage already lives in <see cref="MatchDeterminismTests"/>; this file
/// focuses on the rules-layer contract.
/// </summary>
public sealed class MatchRulesTests
{
    private static Fixed F(int n) => Fixed.FromInt(n);
    private static Fixed Half => Fixed.Half;
    private static Vector3Fixed V3(int x, int y, int z) => new(F(x), F(y), F(z));

    /// <summary>Build a minimum-viable state. 22 players at origin (irrelevant for rules tests; rules don't read player positions).</summary>
    private static MatchSimulationState BuildState(BallState ball)
    {
        PlayerState[] home = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        PlayerState[] away = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        for (byte i = 1; i <= 11; i++)
        {
            home[i - 1] = new PlayerState(Vector3Fixed.Zero, Vector3Fixed.Zero, i, TeamSide.Home);
            away[i - 1] = new PlayerState(Vector3Fixed.Zero, Vector3Fixed.Zero, i, TeamSide.Away);
        }
        return new MatchSimulationState(Tick.Zero, ball, home, away);
    }

    #region Goal detection — in-goal-mouth crossings

    [Fact]
    public void Step_BallCrossesPositiveGoalLineInMouth_EmitsGoal_HomeScores()
    {
        // Pre-step ball at (50, 1, 0) — in-field. Post-step at (53, 1, 0) —
        // crossed +X goal line (52.5) at Y=1 (under 2.44 crossbar) and Z=0
        // (within ±3.66 posts). Home attacks +X; goal counts for Home.
        BallState pre = new(V3(50, 1, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(53, 1, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Equal(1, state.HomeScore);
        Assert.Equal(0, state.AwayScore);
        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.Goal, state.KeyEvents[0].Kind);
        Assert.Equal(TeamSide.Home, state.KeyEvents[0].Side);
        // Ball respawned at center for immediate restart.
        Assert.Equal(Vector3Fixed.Zero, state.Ball.Position);
        Assert.Equal(Vector3Fixed.Zero, state.Ball.Velocity);
        // OutOfPlay stays InPlay (immediate respawn — no celebration tick range).
        Assert.Equal(OutOfPlay.InPlay, state.OutOfPlay);
    }

    [Fact]
    public void Step_BallCrossesNegativeGoalLineInMouth_EmitsGoal_AwayScores()
    {
        // Mirror of the home goal: ball crosses -X goal line; Away scores.
        BallState pre = new(V3(-50, 1, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(-53, 1, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Equal(0, state.HomeScore);
        Assert.Equal(1, state.AwayScore);
        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.Goal, state.KeyEvents[0].Kind);
        Assert.Equal(TeamSide.Away, state.KeyEvents[0].Side);
    }

    #endregion

    #region Goal-line crossings outside the mouth — GoalKick

    [Fact]
    public void Step_BallCrossesPositiveGoalLineAboveCrossbar_EmitsGoalKick_NotGoal()
    {
        // Ball crosses +X goal line at Y=3 (above 2.44 crossbar). NOT a goal.
        BallState pre = new(V3(50, 3, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(53, 3, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Equal(0, state.HomeScore);
        Assert.Equal(0, state.AwayScore);
        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.GoalKickRestart, state.KeyEvents[0].Kind);
        Assert.Equal(OutOfPlay.GoalKick, state.OutOfPlay);
        // Ball respawns at goal-line center on the conceding side.
        Assert.Equal(MatchRules.GoalLineX, state.Ball.Position.X);
        Assert.Equal(Fixed.Zero, state.Ball.Position.Y);
        Assert.Equal(Fixed.Zero, state.Ball.Position.Z);
    }

    [Fact]
    public void Step_BallCrossesPositiveGoalLineWideOfPost_EmitsGoalKick()
    {
        // Ball crosses +X goal line at Z=10 (wider than 3.66 post-half-width).
        BallState pre = new(V3(50, 1, 10), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(53, 1, 10), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Equal(0, state.HomeScore);
        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.GoalKickRestart, state.KeyEvents[0].Kind);
        Assert.Equal(OutOfPlay.GoalKick, state.OutOfPlay);
    }

    #endregion

    #region Touchline crossings — ThrowIn

    [Fact]
    public void Step_BallCrossesPositiveTouchline_EmitsThrowIn()
    {
        // Ball crosses +Z touchline (at Z=34). Phase-3 always classifies as
        // ThrowIn with Home side (no last-touched tracking).
        BallState pre = new(V3(0, 0, 33), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(0, 0, 35), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Equal(0, state.HomeScore);
        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.ThrowInRestart, state.KeyEvents[0].Kind);
        Assert.Equal(OutOfPlay.ThrowIn, state.OutOfPlay);
        // Ball respawns at touchline at the X-coordinate of the crossing.
        Assert.Equal(MatchRules.TouchlineZ, state.Ball.Position.Z);
    }

    [Fact]
    public void Step_BallCrossesNegativeTouchline_EmitsThrowIn()
    {
        BallState pre = new(V3(0, 0, -33), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(0, 0, -35), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.ThrowInRestart, state.KeyEvents[0].Kind);
        Assert.Equal(OutOfPlay.ThrowIn, state.OutOfPlay);
        Assert.Equal(-MatchRules.TouchlineZ, state.Ball.Position.Z);
    }

    #endregion

    #region Diagonal corner-adjacent trajectories (Codex audit 2026-04-30)

    // Regression coverage for the boundary-priority bug Codex caught: a
    // fast diagonal ball can be beyond BOTH the goal line AND a touchline
    // at post-step. The earliest crossing in time-order determines the
    // restart per football rules. Prior implementation always preferred
    // goal-line by post-position only, mis-classifying touchline-first
    // trajectories as GoalKickRestart.

    [Fact]
    public void Step_DiagonalTouchlineFirstThenGoalLine_EmitsThrowIn()
    {
        // Pre-step: ball at (51, 0, 33) — in-field but close to both
        // touchline (Z=34) and goal line (X=52.5).
        // Post-step: ball at (53, 0, 36) — past both.
        // ΔX = +2 → crosses X=52.5 at t = 1.5/2 = 0.75
        // ΔZ = +3 → crosses Z=34   at t = 1/3 ≈ 0.333
        // Touchline crosses earlier; restart should be ThrowIn.
        BallState pre = new(V3(51, 0, 33), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(53, 0, 36), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.ThrowInRestart, state.KeyEvents[0].Kind);
        Assert.Equal(OutOfPlay.ThrowIn, state.OutOfPlay);
    }

    [Fact]
    public void Step_DiagonalCornerTie_GoalLineWinsByTieBreak()
    {
        // Tie case: tGoal = tTouch = 0.5 (ball crosses both planes at the
        // exact same parametric instant). Phase-3 policy in MatchRules.Step
        // tie-breaks to goal-line (`tGoal.Value <= tTouch.Value`).
        // Pre (51, 0, 33), post (54, 0, 35):
        //   ΔX = +3 → tGoal  = 1.5/3 = 0.5
        //   ΔZ = +2 → tTouch = 1/2   = 0.5
        // This test pins the tie-break direction.
        BallState pre = new(V3(51, 0, 33), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(54, 0, 35), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.GoalKickRestart, state.KeyEvents[0].Kind);
        Assert.Equal(OutOfPlay.GoalKick, state.OutOfPlay);
    }

    [Fact]
    public void Step_DiagonalTouchlineFirst_NegativeCorner_EmitsThrowIn()
    {
        // Mirror across the -X / -Z corner: Away-side defending corner.
        // Pre (-51, 0, -33), post (-53, 0, -36).
        // ΔX = -2 → crosses X=-52.5 at t = (-1.5)/(-2) = 0.75
        // ΔZ = -3 → crosses Z=-34   at t = (-1)/(-3)   ≈ 0.333
        // Touchline first → ThrowIn.
        BallState pre = new(V3(-51, 0, -33), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(-53, 0, -36), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.ThrowInRestart, state.KeyEvents[0].Kind);
        Assert.Equal(OutOfPlay.ThrowIn, state.OutOfPlay);
    }

    [Fact]
    public void Step_DiagonalCornerTie_NegativeCorner_GoalLineWinsByTieBreak()
    {
        // Mirror tie case at the -X / +Z corner.
        // Pre (-51, 0, 33), post (-54, 0, 35):
        //   ΔX = -3 → tGoal  = (-1.5)/(-3) = 0.5
        //   ΔZ = +2 → tTouch = 1/2         = 0.5
        // Tie-break prefers goal-line. Pinned for the mirror direction.
        BallState pre = new(V3(-51, 0, 33), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(-54, 0, 35), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.GoalKickRestart, state.KeyEvents[0].Kind);
        Assert.Equal(OutOfPlay.GoalKick, state.OutOfPlay);
    }

    [Fact]
    public void Step_DiagonalTouchlineFirst_SubUlpTouchEarlierThanGoal_EmitsThrowIn()
    {
        // Codex P2 round 4 (2026-04-30) regression test.
        //
        // Construct a trajectory where:
        //   • Goal-line crossing parameter t_g and touchline crossing t_t
        //     are BOTH genuinely positive in (0, 1).
        //   • t_t < t_g  (touchline truly crossed first).
        //   • The Q32.32-rounded representations of t_g and t_t collide
        //     to the SAME Fixed value (within 1 ULP). The pre-fix
        //     compare-rounded-Fixed code therefore saw a tie and applied
        //     the goal-line tie-break — emitting GoalKickRestart instead
        //     of ThrowInRestart. The exact-rational compare via cross-
        //     multiplied BigInteger correctly resolves t_t < t_g.
        //
        // Construction:
        //   pre.X = 51.5      → numG = 52.5 - 51.5 = 1.0 (Fixed)
        //   post.X = 53.5 - 1ULP_Fixed
        //                     → denG = 2.0 - 1ULP, slightly < 2 in raw long
        //   t_g exact = 1 / (2 - 2^-32) ≈ 0.5 + 2^-33 (truly > 0.5)
        //
        //   pre.Z = 33        → numT = 34 - 33 = 1.0
        //   post.Z = 35       → denT = 2.0 exactly
        //   t_t exact = 1 / 2 = 0.5 exactly
        //
        //   Q32.32 division of t_g: ((1·2^32) · 2^32) / (2·2^32 - 1)
        //                          ≈ 2^31 + tiny, truncated to 2^31
        //                        = 0.5 in Fixed.
        //   Q32.32 division of t_t: (1·2^32 · 2^32) / (2·2^32) = 2^31
        //                        = 0.5 in Fixed.
        //   Both round to identical Fixed → rounded-CompareTo says EQUAL.
        //
        //   Exact rational: 1/(2-2^-32) > 1/2, so t_g > t_t,
        //   so touchline crossed first → emit ThrowInRestart.
        Fixed preX = Fixed.FromInt(515) / Fixed.FromInt(10);   // 51.5
        // 53.5 in raw is (107 * 2^32) / 2 = 53.5 * 2^32. Subtract 1 raw long
        // unit so denG = post.X - pre.X = (2 * 2^32) - 1 raw — i.e. one Q32.32
        // ULP shy of exactly 2.0. The single 1-ULP shave is the ENTIRE
        // mechanism that pushed the prior rounded-compare implementation into
        // a tie with the touchline crossing.
        Fixed postX = Fixed.FromRaw((Fixed.FromInt(535) / Fixed.FromInt(10)).RawValue - 1L);
        Fixed preZ = Fixed.FromInt(33);
        Fixed postZ = Fixed.FromInt(35);

        BallState pre = new(
            new Vector3Fixed(preX, Fixed.Zero, preZ),
            Vector3Fixed.Zero,
            Vector3Fixed.Zero);
        BallState post = new(
            new Vector3Fixed(postX, Fixed.Zero, postZ),
            Vector3Fixed.Zero,
            Vector3Fixed.Zero);

        // Belt-and-suspenders: prove the construction actually exercises the
        // sub-ULP boundary. The rounded Fixed division of both t values must
        // collide; otherwise this test would also pass under the prior
        // rounded-compare implementation and lose its regression power.
        Fixed numG = (Fixed.FromInt(525) / Fixed.FromInt(10)) - preX;
        Fixed denG = postX - preX;
        Fixed numT = Fixed.FromInt(34) - preZ;
        Fixed denT = postZ - preZ;
        Fixed roundedTGoal = numG / denG;
        Fixed roundedTTouch = numT / denT;
        Assert.Equal(roundedTTouch, roundedTGoal);  // rounded would tie

        MatchSimulationState state = BuildState(post);
        MatchRules.Step(state, pre);

        // Touchline crossed FIRST in true-rational time-order.
        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.ThrowInRestart, state.KeyEvents[0].Kind);
        Assert.Equal(OutOfPlay.ThrowIn, state.OutOfPlay);
        Assert.Equal(0, state.HomeScore);
        Assert.Equal(0, state.AwayScore);
    }

    [Fact]
    public void Step_DiagonalTouchlineFirst_SubUlp_NegativeDeltaCorner_EmitsThrowIn()
    {
        // Mirror of the +X/+Z sub-ULP test across the -X/+Z corner so the
        // exact-rational compare exercises the negative-delta sign branch
        // in BuildCrossingFraction (numRaw < 0 && denRaw < 0). Without
        // this mirror, only the positive-delta branch is regression-locked
        // (per feature-dev:code-reviewer 2026-04-30 finding #5).
        //
        // Construction (mirrored from the +X/+Z fixture):
        //   pre.X = -51.5             → numG_signed = -52.5 - (-51.5) = -1.0
        //   post.X = -53.5 + 1ULP     → denG_signed = (post.X - pre.X)
        //                              = (-2.0 + 1ULP), one Q32.32 ULP shy of -2.0
        //   t_g exact = (-1.0) / (-2.0 + 2^-32) = 1 / (2 - 2^-32) ≈ 0.5 + 2^-33
        //
        //   pre.Z = 33                → numT_signed = 1.0
        //   post.Z = 35               → denT_signed = 2.0 exactly
        //   t_t exact = 0.5 exactly
        //
        // Exact rational: t_g > t_t, touchline crossed first → ThrowIn.
        Fixed preX = -(Fixed.FromInt(515) / Fixed.FromInt(10));   // -51.5
        // -53.5 raw + 1 ULP → one ULP closer to zero than -53.5, i.e., -2.0
        // delta + 1 ULP. Exercises the all-negative-raw branch in
        // BuildCrossingFraction.
        Fixed postX = Fixed.FromRaw((-(Fixed.FromInt(535) / Fixed.FromInt(10))).RawValue + 1L);
        Fixed preZ = Fixed.FromInt(33);
        Fixed postZ = Fixed.FromInt(35);

        BallState pre = new(
            new Vector3Fixed(preX, Fixed.Zero, preZ),
            Vector3Fixed.Zero,
            Vector3Fixed.Zero);
        BallState post = new(
            new Vector3Fixed(postX, Fixed.Zero, postZ),
            Vector3Fixed.Zero,
            Vector3Fixed.Zero);

        // Belt-and-suspenders: prove rounded compare would tie at the
        // mirrored corner too.
        Fixed numG = -(Fixed.FromInt(525) / Fixed.FromInt(10)) - preX;
        Fixed denG = postX - preX;
        Fixed numT = Fixed.FromInt(34) - preZ;
        Fixed denT = postZ - preZ;
        Fixed roundedTGoal = numG / denG;
        Fixed roundedTTouch = numT / denT;
        Assert.Equal(roundedTTouch, roundedTGoal);  // rounded would tie

        MatchSimulationState state = BuildState(post);
        MatchRules.Step(state, pre);

        // Touchline crossed FIRST in true-rational time-order — same outcome
        // as the +X/+Z mirror; symmetry locked.
        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.ThrowInRestart, state.KeyEvents[0].Kind);
        Assert.Equal(OutOfPlay.ThrowIn, state.OutOfPlay);
        Assert.Equal(0, state.HomeScore);
        Assert.Equal(0, state.AwayScore);
    }

    [Fact]
    public void Step_DiagonalTouchlineFirstAtGoalMouthHeight_StillEmitsThrowIn()
    {
        // The boundary-priority fix must apply even when the ball-Y at the
        // goal-line plane WOULD have been a goal — touchline crossed first
        // means the ball was already out-of-play before reaching the goal
        // line, so it can't score.
        // Pre (51, 1, 33), post (53, 1, 36).
        // tGoal = 1.5/2 = 0.75, tTouch = 1/3 ≈ 0.333. Touch wins.
        // Y at goal-line is 1 (under 2.44 crossbar), Z at goal-line is
        // 33 + 0.75 * 3 = 35.25 — but we never reach goal-line in time.
        BallState pre = new(V3(51, 1, 33), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(53, 1, 36), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Equal(0, state.HomeScore);
        Assert.Equal(0, state.AwayScore);
        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.ThrowInRestart, state.KeyEvents[0].Kind);
    }

    #endregion

    #region Non-events

    [Fact]
    public void Step_BallStaysInField_NoEventEmitted()
    {
        BallState pre = new(V3(10, 0, 5), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(11, 0, 5), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Empty(state.KeyEvents);
        Assert.Equal(OutOfPlay.InPlay, state.OutOfPlay);
        Assert.Equal(0, state.HomeScore);
        Assert.Equal(0, state.AwayScore);
    }

    [Fact]
    public void Step_BallAlreadyOutOfField_NoNewEvent()
    {
        // Pre-step OUT, post-step also OUT. The crossing already happened
        // a previous tick; no new event fires this tick.
        BallState pre = new(V3(53, 0, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(54, 0, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);

        MatchRules.Step(state, pre);

        Assert.Empty(state.KeyEvents);
        Assert.Equal(OutOfPlay.InPlay, state.OutOfPlay);
    }

    [Fact]
    public void Step_NullState_Throws()
    {
        Assert.Throws<ArgumentNullException>(() =>
            MatchRules.Step(null!, BallState.AtRest));
    }

    #endregion

    #region OutOfPlay is per-tick transient

    [Fact]
    public void Step_PreviousTickThrowIn_ResetsToInPlay_ThenStaysInPlayIfNoEventThisTick()
    {
        // Manually set OutOfPlay = ThrowIn on the state, then call Step
        // with no crossing event. Step should reset to InPlay.
        BallState pre = new(V3(0, 0, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(1, 0, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);
        state.OutOfPlay = OutOfPlay.ThrowIn;

        MatchRules.Step(state, pre);

        Assert.Equal(OutOfPlay.InPlay, state.OutOfPlay);
        Assert.Empty(state.KeyEvents);
    }

    #endregion

    #region KeyEvent ordering + canonical encoding

    [Fact]
    public void KeyEvent_AppendOrderPreserved_AcrossMultipleTicks()
    {
        // Tick 1: goal. Tick 2: out-of-play (after manually re-positioning ball).
        MatchSimulationState state = BuildState(new BallState(V3(53, 1, 0), Vector3Fixed.Zero, Vector3Fixed.Zero));
        state.CurrentTick = Tick.FromSeconds(1);

        BallState pre1 = new(V3(50, 1, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchRules.Step(state, pre1);

        Assert.Single(state.KeyEvents);
        Assert.Equal(KeyEventKind.Goal, state.KeyEvents[0].Kind);

        // Now move ball to trigger a touchline-out the next tick.
        state.CurrentTick = state.CurrentTick + 1L;
        state.Ball = new BallState(V3(0, 0, 35), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState pre2 = new(V3(0, 0, 33), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchRules.Step(state, pre2);

        Assert.Equal(2, state.KeyEvents.Count);
        Assert.Equal(KeyEventKind.Goal, state.KeyEvents[0].Kind);
        Assert.Equal(KeyEventKind.ThrowInRestart, state.KeyEvents[1].Kind);
        // Tick ordering preserved.
        Assert.True(state.KeyEvents[0].Tick.CompareTo(state.KeyEvents[1].Tick) < 0);
    }

    [Fact]
    public void CanonicalEncoding_KeyEventsChangeHash()
    {
        // Same canonical state except one has an empty KeyEvents list and
        // the other has a single Goal entry. Hashes must differ — proves
        // KeyEvents are part of the canonical encoding.
        MatchSimulationState empty = BuildState(BallState.AtRest);
        MatchSimulationState withGoal = BuildState(BallState.AtRest);
        withGoal.KeyEvents.Add(new KeyEvent(
            tick: Tick.FromSeconds(1),
            kind: KeyEventKind.Goal,
            side: TeamSide.Home,
            jerseyNumber: 0,
            position: V3(52, 1, 0)));
        withGoal.HomeScore = 1;

        string emptyHash = MatchCanonicalState.ComputeHash(empty);
        string withGoalHash = MatchCanonicalState.ComputeHash(withGoal);

        Assert.NotEqual(emptyHash, withGoalHash);
    }

    [Fact]
    public void EncodedByteCountFor_AccountsForKeyEvents()
    {
        MatchSimulationState empty = BuildState(BallState.AtRest);
        MatchSimulationState withTwoEvents = BuildState(BallState.AtRest);
        withTwoEvents.KeyEvents.Add(new KeyEvent(Tick.Zero, KeyEventKind.Goal, TeamSide.Home, 0, Vector3Fixed.Zero));
        withTwoEvents.KeyEvents.Add(new KeyEvent(Tick.One, KeyEventKind.ThrowInRestart, TeamSide.Away, 0, Vector3Fixed.Zero));

        Assert.Equal(MatchCanonicalState.EncodedBaseByteCount, MatchCanonicalState.EncodedByteCountFor(empty));
        Assert.Equal(
            MatchCanonicalState.EncodedBaseByteCount + 2 * KeyEvent.EncodedByteCount,
            MatchCanonicalState.EncodedByteCountFor(withTwoEvents));
    }

    #endregion

    #region Score overflow

    [Fact]
    public void Step_HomeScoreAtMaxValue_GoalWouldOverflow_Throws()
    {
        BallState pre = new(V3(50, 1, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        BallState post = new(V3(53, 1, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);
        MatchSimulationState state = BuildState(post);
        state.HomeScore = byte.MaxValue;

        Assert.Throws<InvalidOperationException>(() => MatchRules.Step(state, pre));
    }

    #endregion

    #region MatchSimulationConfig

    [Fact]
    public void MatchSimulationConfig_Default_HasZeroSeed()
    {
        Assert.Equal(Seed.Zero, MatchSimulationConfig.Default.MatchSeed);
    }

    [Fact]
    public void MatchSimulationConfig_RoundTripsSeed()
    {
        Seed seed = Seed.FromUInt64(0xdeadbeefdeadbeefUL);
        MatchSimulationConfig config = new(seed);

        Assert.Equal(seed, config.MatchSeed);
    }

    [Fact]
    public void MatchSimulationRunner_AcceptsConfig_DoesNotChangeCanonicalHash()
    {
        // Seed is fixture input, not canonical state. Two runs with different
        // seeds (but otherwise identical state) must produce identical
        // canonical hashes. When stochastic events land Phase 4+, this test
        // updates to require different hashes; until then it locks the
        // "seed is not canonical state" invariant.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");

        MatchSimulationState a = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, direct, lowBlock);
        MatchSimulationState b = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, direct, lowBlock);

        MatchSimulationRunner.RunTicks(a, direct, lowBlock,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            new MatchSimulationConfig(Seed.Zero),
            ticks: 60);
        MatchSimulationRunner.RunTicks(b, direct, lowBlock,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            new MatchSimulationConfig(Seed.FromUInt64(0xdeadbeefdeadbeefUL)),
            ticks: 60);

        Assert.Equal(MatchCanonicalState.ComputeHash(a), MatchCanonicalState.ComputeHash(b));
    }

    #endregion

    #region KeyEvent canonical encoding regression

    [Fact]
    public void KeyEvent_WriteCanonical_Produces35Bytes()
    {
        CanonicalEncoder encoder = new();
        KeyEvent ev = new(Tick.Zero, KeyEventKind.Goal, TeamSide.Home, 7, V3(50, 1, 0));

        ev.WriteCanonical(encoder);

        Assert.Equal(KeyEvent.EncodedByteCount, encoder.WrittenCount);
        Assert.Equal(35, KeyEvent.EncodedByteCount);
    }

    [Fact]
    public void KeyEvent_RejectsNoneKind()
    {
        Assert.Throws<ArgumentException>(() =>
            new KeyEvent(Tick.Zero, KeyEventKind.None, TeamSide.Home, 0, Vector3Fixed.Zero));
    }

    [Fact]
    public void KeyEvent_RejectsInvalidSide()
    {
        Assert.Throws<ArgumentException>(() =>
            new KeyEvent(Tick.Zero, KeyEventKind.Goal, default, 0, Vector3Fixed.Zero));
    }

    [Fact]
    public void KeyEvent_RejectsJerseyAbove99()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            new KeyEvent(Tick.Zero, KeyEventKind.Goal, TeamSide.Home, 100, Vector3Fixed.Zero));
    }

    [Fact]
    public void KeyEvent_AllowsJerseyZero_ForUnspecified()
    {
        // jerseyNumber 0 IS valid — Phase-3 emits 0 when scorer/last-toucher
        // unknown (no possession tracking yet).
        var ex = Record.Exception(() =>
            new KeyEvent(Tick.Zero, KeyEventKind.Goal, TeamSide.Home, 0, Vector3Fixed.Zero));
        Assert.Null(ex);
    }

    #endregion
}

using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Phase-3 polish-pass round-3 #4 (2026-05-12) defensive line height tests.
/// Closes the "back-four shifts uniformly with midfield" symptom: Option-3
/// translates everyone by ball.X × 0.5; round-3 #4 differentiates the
/// back-four (roster slots 2-5).
///
/// <list type="bullet">
///   <item><description>In-possession defender: shift X = ball.X × 0.8
///   (= 0.5 + 0.3); compresses up when attacking.</description></item>
///   <item><description>Out-of-possession defender: shift X = ball.X ×
///   0.2 (replaces 0.5); sits deeper than midfield, "stay home"
///   discipline.</description></item>
///   <item><description>Midfielders / strikers unchanged: ball.X × 0.5.</description></item>
/// </list>
///
/// Coefficients 0.3 and 0.2 are NOT exactly representable in Q32.32, so
/// tests compute expected values via the same Fixed arithmetic path the
/// production code uses (rather than asserting against decimal literals
/// that would carry a different rounding error).
/// </summary>
public sealed class DefensiveLineHeightTests
{
    private static readonly PlayerKinematics K = PlayerKinematics.Phase3Defaults;
    private const string HomeArchetypeId = "direct-pressing";
    private const string AwayArchetypeId = "low-block-counter";

    // Mirror the production constants so test-expected values use the
    // identical Fixed arithmetic path. Any change to BehaviorTreeRunner's
    // constants requires the same update here — and the tests will fail
    // loudly until both are aligned.
    private static readonly Fixed FormationBallShiftFactor =
        Fixed.FromInt(1) / Fixed.FromInt(2);
    private static readonly Fixed DefensiveLineExtraShiftFactor =
        Fixed.FromInt(3) / Fixed.FromInt(10);
    private static readonly Fixed DefensiveLineDropFactor =
        Fixed.FromInt(2) / Fixed.FromInt(10);
    private static readonly Fixed LateralBallShiftFactor =
        Fixed.FromInt(3) / Fixed.FromInt(10);

    private static PlayerState[] BuildFormationTeam(BehaviorTreeArchetype archetype, TeamSide side)
    {
        var team = new PlayerState[11];
        foreach (FormationSlot slot in archetype.Formation)
        {
            Vector3Fixed pos = side == TeamSide.Home ? slot.HomeBasePosition : slot.AwayBasePosition();
            team[slot.RosterSlot - 1] = new PlayerState(pos, Vector3Fixed.Zero, slot.RosterSlot, side);
        }
        return team;
    }

    /// <summary>Compute the expected defender X for an in-possession
    /// defender at <paramref name="baseX"/> with ball at <paramref name="ballX"/>.
    /// Mirrors production: base + ball.X × (0.5 + 0.3).</summary>
    private static Fixed ExpectedDefenderXInPossession(Fixed baseX, Fixed ballX)
        => baseX + ballX * (FormationBallShiftFactor + DefensiveLineExtraShiftFactor);

    /// <summary>Compute the expected defender X for an out-of-possession
    /// defender. Mirrors production: base + ball.X × 0.2.</summary>
    private static Fixed ExpectedDefenderXOutOfPossession(Fixed baseX, Fixed ballX)
        => baseX + ballX * DefensiveLineDropFactor;

    /// <summary>Compute the expected non-defender X. Mirrors Option-3:
    /// base + ball.X × 0.5.</summary>
    private static Fixed ExpectedNonDefenderX(Fixed baseX, Fixed ballX)
        => baseX + ballX * FormationBallShiftFactor;

    [Fact]
    public void Defender_InPossession_BallForward_PushesHigherThanMidfielder()
    {
        // Ball at +40 X (home attacking third), home in possession.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        // Home RCM (slot 7 / index 6) on ball → home in possession.
        Fixed ballX = Fixed.FromInt(40);
        homeTeam[6] = new PlayerState(
            new Vector3Fixed(ballX, Fixed.Zero, Fixed.Zero),
            Vector3Fixed.Zero, 7, TeamSide.Home);
        var ball = new BallState(
            position: new Vector3Fixed(ballX, Fixed.Zero, Fixed.Zero),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // RB (slot 2, index 1, base X=-25) — defender in possession.
        Fixed rbExpected = ExpectedDefenderXInPossession(Fixed.FromInt(-25), ballX);
        Assert.Equal(rbExpected, commands[1].DesiredPosition.X);
        // RM (slot 6, index 5, base X=-5) — non-defender uses 0.5.
        Fixed rmExpected = ExpectedNonDefenderX(Fixed.FromInt(-5), ballX);
        Assert.Equal(rmExpected, commands[5].DesiredPosition.X);

        // RB advances more from base than RM does (defender extra shift).
        Fixed rbAdvance = commands[1].DesiredPosition.X - Fixed.FromInt(-25);
        Fixed rmAdvance = commands[5].DesiredPosition.X - Fixed.FromInt(-5);
        Assert.True(rbAdvance.RawValue > rmAdvance.RawValue,
            $"Defender advance {rbAdvance}m must exceed midfielder advance {rmAdvance}m");
    }

    [Fact]
    public void Defender_OutOfPossession_BallBack_HoldsTighterLineThanOption3()
    {
        // Ball at home corner (-50, 0, 30) so back-four is OUTSIDE press
        // radius. Away in possession. RB (slot 2) base (-25, 20).
        // Out-of-possession X shift = base + ball.X × 0.2.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        Fixed ballX = Fixed.FromInt(-50);
        var ballPos = new Vector3Fixed(ballX, Fixed.Zero, Fixed.FromInt(30));
        awayTeam[10] = new PlayerState(ballPos, Vector3Fixed.Zero, 11, TeamSide.Away);
        // Push other away outfield far away so they aren't in press range.
        for (int i = 0; i < 10; i++)
        {
            awayTeam[i] = new PlayerState(
                new Vector3Fixed(Fixed.FromInt(50), Fixed.Zero, Fixed.FromInt(-30 + i * 6)),
                Vector3Fixed.Zero, awayTeam[i].JerseyNumber, awayTeam[i].Side);
        }
        var ball = new BallState(ballPos, Vector3Fixed.Zero, Vector3Fixed.Zero);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        Fixed rbExpected = ExpectedDefenderXOutOfPossession(Fixed.FromInt(-25), ballX);
        Assert.Equal(rbExpected, commands[1].DesiredPosition.X);

        // Compare against what Option-3 standard 0.5 would produce:
        Fixed option3StandardX = ExpectedNonDefenderX(Fixed.FromInt(-25), ballX);
        Assert.True(commands[1].DesiredPosition.X.RawValue > option3StandardX.RawValue,
            $"Defender out-of-possession should stay higher than Option-3 ({option3StandardX}); got {commands[1].DesiredPosition.X}");
    }

    [Fact]
    public void Midfielder_InPossession_StillUsesOption3Translation()
    {
        // Slot 6 RM (index 5) NOT a defender. Standard 0.5 translation.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        Fixed ballX = Fixed.FromInt(40);
        homeTeam[6] = new PlayerState(
            new Vector3Fixed(ballX, Fixed.Zero, Fixed.Zero),
            Vector3Fixed.Zero, 7, TeamSide.Home);
        var ball = new BallState(
            position: new Vector3Fixed(ballX, Fixed.Zero, Fixed.Zero),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        Fixed rmExpected = ExpectedNonDefenderX(Fixed.FromInt(-5), ballX);
        Assert.Equal(rmExpected, commands[5].DesiredPosition.X);
    }

    [Fact]
    public void Defender_RosterSlots2to5_DetectedAsDefenders()
    {
        // All 4 back-four slots should get defender treatment with the
        // in-possession 0.8 factor.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        Fixed ballX = Fixed.FromInt(30);
        homeTeam[6] = new PlayerState(
            new Vector3Fixed(ballX, Fixed.Zero, Fixed.Zero),
            Vector3Fixed.Zero, 7, TeamSide.Home);
        var ball = new BallState(
            position: new Vector3Fixed(ballX, Fixed.Zero, Fixed.Zero),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // direct-pressing formation:
        //   Slot 2 RB base (-25, 20), slot 3 RCB base (-30, 8),
        //   slot 4 LCB base (-30, -8), slot 5 LB base (-25, -20).
        Assert.Equal(
            ExpectedDefenderXInPossession(Fixed.FromInt(-25), ballX),
            commands[1].DesiredPosition.X);
        Assert.Equal(
            ExpectedDefenderXInPossession(Fixed.FromInt(-30), ballX),
            commands[2].DesiredPosition.X);
        Assert.Equal(
            ExpectedDefenderXInPossession(Fixed.FromInt(-30), ballX),
            commands[3].DesiredPosition.X);
        Assert.Equal(
            ExpectedDefenderXInPossession(Fixed.FromInt(-25), ballX),
            commands[4].DesiredPosition.X);
    }

    [Fact]
    public void DefensiveLineHeight_Deterministic_AcrossManyRuns()
    {
        // Same inputs → same commands across 100 invocations.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        homeTeam[6] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(25), Fixed.Zero, Fixed.FromInt(-10)),
            Vector3Fixed.Zero, 7, TeamSide.Home);
        var ball = new BallState(
            position: new Vector3Fixed(Fixed.FromInt(25), Fixed.Zero, Fixed.FromInt(-10)),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);

        var first = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, first);

        for (int run = 0; run < 100; run++)
        {
            var commands = new PlayerCommand[11];
            BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);
            for (int i = 0; i < 11; i++)
            {
                Assert.Equal(first[i].DesiredPosition, commands[i].DesiredPosition);
            }
        }
    }
}

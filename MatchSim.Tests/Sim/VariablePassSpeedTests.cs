using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Phase-3 polish-pass round-3 #3 (2026-05-12) variable pass speed tests.
/// Closes the "all passes at 14 m/s look the same" symptom: ChoosePassKick
/// now tiers pass speed by squared distance — 10 m/s for short (8-15m),
/// 14 m/s for medium (15-25m), 18 m/s for long (25-35m). Long-ball
/// fallback speed (18 m/s when no eligible teammate) UNCHANGED.
/// </summary>
public sealed class VariablePassSpeedTests
{
    private static readonly PlayerKinematics K = PlayerKinematics.Phase3Defaults;
    private const string HomeArchetypeId = "direct-pressing";
    private const string AwayArchetypeId = "low-block-counter";

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

    /// <summary>Build a controlled pass scenario: home slot 7 (RCM, idx 6)
    /// at origin with ball on top. Sole eligible candidate slot 11 (LST,
    /// idx 10) placed at <paramref name="forwardX"/>, Z=0. All other home
    /// outfield + away pushed far away.</summary>
    private static void BuildSinglePassScenario(
        Fixed forwardX,
        out BehaviorTreeArchetype home, out BehaviorTreeArchetype away,
        out PlayerState[] homeTeam, out PlayerState[] awayTeam,
        out BallState ball)
    {
        home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        homeTeam = BuildFormationTeam(home, TeamSide.Home);
        awayTeam = BuildFormationTeam(away, TeamSide.Away);

        // Carrier at origin.
        homeTeam[6] = new PlayerState(
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            Vector3Fixed.Zero, 7, TeamSide.Home);
        // Ball coincident with carrier.
        ball = new BallState(
            position: new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);

        // Push all other home outfield far away to remove pass-target
        // ambiguity and keep them out of carrier-nearest contests.
        homeTeam[0] = new PlayerState(  // GK
            new Vector3Fixed(Fixed.FromInt(-45), Fixed.Zero, Fixed.Zero),
            Vector3Fixed.Zero, 1, TeamSide.Home);
        for (int i = 1; i <= 10; i++)
        {
            if (i == 6) continue;  // carrier
            if (i == 10)
            {
                // Sole candidate: slot 11 at (forwardX, 0).
                homeTeam[10] = new PlayerState(
                    new Vector3Fixed(forwardX, Fixed.Zero, Fixed.Zero),
                    Vector3Fixed.Zero, 11, TeamSide.Home);
            }
            else
            {
                homeTeam[i] = new PlayerState(
                    new Vector3Fixed(Fixed.FromInt(-45), Fixed.Zero, Fixed.FromInt(-20 + i * 4)),
                    Vector3Fixed.Zero, (byte)(i + 1), TeamSide.Home);
            }
        }

        // Push away team to far edge (no markers on candidate).
        for (int i = 0; i <= 10; i++)
        {
            awayTeam[i] = new PlayerState(
                new Vector3Fixed(Fixed.FromInt(48), Fixed.Zero, Fixed.FromInt(-20 + i * 4)),
                Vector3Fixed.Zero, (byte)(i + 1), TeamSide.Away);
        }
    }

    private static Fixed KickSpeed(PlayerCommand carrierCommand)
    {
        // Kick velocity is `direction × speed`. The magnitude of the
        // velocity (sqrt of LengthSquared) IS the speed.
        Vector3Fixed v = carrierCommand.Kick!.Value.Velocity;
        return Fixed.Sqrt(v.LengthSquared());
    }

    [Fact]
    public void ShortPass_10MetreTeammate_UsesShortSpeed()
    {
        // Candidate at X=10 → distance = 10m (squared = 100 ≤ 225).
        // Short-pass speed = 10 m/s.
        BuildSinglePassScenario(Fixed.FromInt(10),
            out var home, out var away, out var homeTeam, out var awayTeam, out var ball);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        Assert.True(commands[6].Kick.HasValue, "Carrier must emit a KickIntent");
        Fixed speed = KickSpeed(commands[6]);
        // 10 m/s ± 0.1 (sqrt rounding tolerance).
        Assert.True(speed.RawValue >= (Fixed.FromInt(10) - Fixed.FromInt(1) / Fixed.FromInt(10)).RawValue,
            $"Expected ≥ 9.9 m/s; got {speed}");
        Assert.True(speed.RawValue <= (Fixed.FromInt(10) + Fixed.FromInt(1) / Fixed.FromInt(10)).RawValue,
            $"Expected ≤ 10.1 m/s; got {speed}");
    }

    [Fact]
    public void MediumPass_20MetreTeammate_UsesMediumSpeed()
    {
        // Candidate at X=20 → distance = 20m (squared = 400, in (225, 625]).
        // Medium-pass speed = 14 m/s.
        BuildSinglePassScenario(Fixed.FromInt(20),
            out var home, out var away, out var homeTeam, out var awayTeam, out var ball);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        Assert.True(commands[6].Kick.HasValue);
        Fixed speed = KickSpeed(commands[6]);
        Assert.True(speed.RawValue >= (Fixed.FromInt(14) - Fixed.FromInt(1) / Fixed.FromInt(10)).RawValue,
            $"Expected ≥ 13.9 m/s; got {speed}");
        Assert.True(speed.RawValue <= (Fixed.FromInt(14) + Fixed.FromInt(1) / Fixed.FromInt(10)).RawValue,
            $"Expected ≤ 14.1 m/s; got {speed}");
    }

    [Fact]
    public void LongPass_30MetreTeammate_UsesLongSpeed()
    {
        // Candidate at X=30 → distance = 30m (squared = 900 > 625).
        // Long-pass speed = 18 m/s.
        BuildSinglePassScenario(Fixed.FromInt(30),
            out var home, out var away, out var homeTeam, out var awayTeam, out var ball);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        Assert.True(commands[6].Kick.HasValue);
        Fixed speed = KickSpeed(commands[6]);
        Assert.True(speed.RawValue >= (Fixed.FromInt(18) - Fixed.FromInt(1) / Fixed.FromInt(10)).RawValue,
            $"Expected ≥ 17.9 m/s; got {speed}");
        Assert.True(speed.RawValue <= (Fixed.FromInt(18) + Fixed.FromInt(1) / Fixed.FromInt(10)).RawValue,
            $"Expected ≤ 18.1 m/s; got {speed}");
    }

    [Fact]
    public void PassSpeed_AtZoneBoundary_DeterministicallyPicks()
    {
        // Exact 15m boundary: distSq = 225. PassSpeedForDistanceSquared
        // treats distSq <= 225 as short → 10 m/s.
        // Exact 25m boundary: distSq = 625. distSq <= 625 → medium → 14 m/s.
        BuildSinglePassScenario(Fixed.FromInt(15),
            out var home15, out var away15, out var homeTeam15, out var awayTeam15, out var ball15);
        var commands15 = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball15, homeTeam15, awayTeam15, TeamSide.Home, home15, K, commands15);
        Fixed speed15 = KickSpeed(commands15[6]);
        Assert.True(speed15.RawValue <= (Fixed.FromInt(10) + Fixed.FromInt(1) / Fixed.FromInt(10)).RawValue,
            $"15m boundary should be short speed (10 m/s); got {speed15}");

        BuildSinglePassScenario(Fixed.FromInt(25),
            out var home25, out var away25, out var homeTeam25, out var awayTeam25, out var ball25);
        var commands25 = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball25, homeTeam25, awayTeam25, TeamSide.Home, home25, K, commands25);
        Fixed speed25 = KickSpeed(commands25[6]);
        Assert.True(speed25.RawValue <= (Fixed.FromInt(14) + Fixed.FromInt(1) / Fixed.FromInt(10)).RawValue,
            $"25m boundary should be medium speed (14 m/s); got {speed25}");
        Assert.True(speed25.RawValue >= (Fixed.FromInt(14) - Fixed.FromInt(1) / Fixed.FromInt(10)).RawValue,
            $"25m boundary should be medium speed (14 m/s); got {speed25}");
    }

    [Fact]
    public void PassSpeed_Deterministic_AcrossManyRuns()
    {
        // Same inputs → same kick speed across 100 invocations.
        BuildSinglePassScenario(Fixed.FromInt(20),
            out var home, out var away, out var homeTeam, out var awayTeam, out var ball);

        var first = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, first);

        for (int run = 0; run < 100; run++)
        {
            var commands = new PlayerCommand[11];
            BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);
            Assert.Equal(first[6].Kick!.Value.Velocity, commands[6].Kick!.Value.Velocity);
        }
    }
}

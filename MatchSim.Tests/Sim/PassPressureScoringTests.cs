using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Phase-3 polish-pass round-3 #2 (2026-05-11) pass-target opponent-pressure
/// scoring tests. Closes the "carrier threads passes into traffic" symptom
/// the Phase-4 follow-up comment in <see cref="BehaviorTreeRunner.ChoosePassKick"/>
/// already named: before round-3 #2, the carrier picked the FORWARD-MOST
/// eligible teammate regardless of marker count. After: score =
/// teammateForwardX − (markerCount × OpponentPressurePenalty); less-forward-
/// but-free teammates can outscore marked-but-forward ones.
/// </summary>
public sealed class PassPressureScoringTests
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

    /// <summary>Build a scenario where home carrier (slot 7 RCM at -10, 5)
    /// has the ball and can pass to forward teammates. Place 2 candidate
    /// home strikers far apart; vary opponent positioning per test.
    /// </summary>
    private static void BuildPassScenario(
        out BehaviorTreeArchetype home, out BehaviorTreeArchetype away,
        out PlayerState[] homeTeam, out PlayerState[] awayTeam,
        out BallState ball)
    {
        home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        homeTeam = BuildFormationTeam(home, TeamSide.Home);
        awayTeam = BuildFormationTeam(away, TeamSide.Away);

        // Carrier = home slot 7 (RCM index 6) at (-10, 5). Move carrier to
        // (0, 0) so the ball can settle on them and they have possession
        // (closest to ball).
        homeTeam[6] = new PlayerState(
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            Vector3Fixed.Zero, 7, TeamSide.Home);
        // Ball coincident with carrier.
        ball = new BallState(
            position: new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);

        // Move all other home outfield far away so the carrier is
        // unambiguously nearest-to-ball + so they don't accidentally
        // become candidate pass targets at uncontrolled distances.
        // Strikers (slots 10 + 11, indices 9 + 10) — these become our
        // controlled pass-target candidates; positioned below per test.
        // Other home (slots 2-6, 8, 9, 12-via-only-1-GK): push outside any
        // sensible pass range. Slot 9 LM is closest threat → push out.
        homeTeam[0] = new PlayerState(  // GK; far from ball
            new Vector3Fixed(Fixed.FromInt(-45), Fixed.Zero, Fixed.Zero),
            Vector3Fixed.Zero, 1, TeamSide.Home);
        // Slots 2-5 are back-four at -25 to -30 X — already far from carrier at 0.
        // Slot 6 RM at -5, 20 → distance from carrier √(25 + 400) ≈ 20.6m
        // (within MaxPassDistance!); push outside.
        homeTeam[5] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(-45), Fixed.Zero, Fixed.FromInt(-30)),
            Vector3Fixed.Zero, 6, TeamSide.Home);
        // Slot 8 LCM at -10, -5 → dist √(100 + 25) ≈ 11m. Inside range. Push out.
        homeTeam[7] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(-45), Fixed.Zero, Fixed.FromInt(20)),
            Vector3Fixed.Zero, 8, TeamSide.Home);
        // Slot 9 LM at -5, -20 → dist √(25 + 400) ≈ 20.6m; push out.
        homeTeam[8] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(-45), Fixed.Zero, Fixed.FromInt(-20)),
            Vector3Fixed.Zero, 9, TeamSide.Home);

        // Default: clear away outfield to far side so they don't mark.
        for (int i = 1; i <= 10; i++)
        {
            awayTeam[i] = new PlayerState(
                new Vector3Fixed(Fixed.FromInt(45), Fixed.Zero,
                    Fixed.FromInt(-20 + i * 4)),
                Vector3Fixed.Zero, (byte)(i + 1), TeamSide.Away);
        }
    }

    [Fact]
    public void PassTarget_PrefersForwardMostWhenAllUnmarked()
    {
        // Two home strikers ahead of carrier. NO opponents nearby. Carrier
        // should pick the more-forward one (no pressure modifier kicks in).
        BuildPassScenario(out var home, out var away, out var homeTeam,
            out var awayTeam, out var ball);
        // Slot 10 (RST, index 9) at (15, 5): forwardness = 15.
        homeTeam[9] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(15), Fixed.Zero, Fixed.FromInt(5)),
            Vector3Fixed.Zero, 10, TeamSide.Home);
        // Slot 11 (LST, index 10) at (25, -5): forwardness = 25.
        homeTeam[10] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(25), Fixed.Zero, Fixed.FromInt(-5)),
            Vector3Fixed.Zero, 11, TeamSide.Home);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // Carrier (index 6) should kick toward slot 11 at (25, -5) — the
        // more-forward unmarked teammate.
        Assert.True(commands[6].Kick.HasValue, "Carrier must emit a KickIntent");
        Vector3Fixed kickVel = commands[6].Kick!.Value.Velocity;
        // Kick direction should be toward (25, 0, -5) from (0, 0, 0).
        // Velocity X should be positive (toward target X=25) and Z should be
        // negative (toward target Z=-5).
        Assert.True(kickVel.X.RawValue > 0L, $"Kick X must be positive (toward RST); got {kickVel.X}");
        Assert.True(kickVel.Z.RawValue < 0L, $"Kick Z must be negative (toward LST z=-5); got {kickVel.Z}");
    }

    [Fact]
    public void PassTarget_PenalisesMarkedForwardTeammate_PicksFreeTeammateBehind()
    {
        // Slot 10 (RST) at (10, 5) — less forward (forwardness=10), FREE.
        // Slot 11 (LST) at (20, -5) — more forward (forwardness=20), MARKED
        //   by 2 opponents within 3m.
        // Scores: slot 10 = 10 - 0 = 10; slot 11 = 20 - (2*4) = 12.
        // Hmm, slot 11 still wins. Increase marker count to make slot 10
        // attractive: 3 markers on slot 11 → score = 20 - 12 = 8 < 10.
        BuildPassScenario(out var home, out var away, out var homeTeam,
            out var awayTeam, out var ball);
        homeTeam[9] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(10), Fixed.Zero, Fixed.FromInt(5)),
            Vector3Fixed.Zero, 10, TeamSide.Home);
        homeTeam[10] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(20), Fixed.Zero, Fixed.FromInt(-5)),
            Vector3Fixed.Zero, 11, TeamSide.Home);
        // Place 3 away players within 3m of slot 11 to tightly mark.
        awayTeam[1] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(20), Fixed.Zero, Fixed.FromInt(-7)),
            Vector3Fixed.Zero, 2, TeamSide.Away);
        awayTeam[2] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(22), Fixed.Zero, Fixed.FromInt(-5)),
            Vector3Fixed.Zero, 3, TeamSide.Away);
        awayTeam[3] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(18), Fixed.Zero, Fixed.FromInt(-5)),
            Vector3Fixed.Zero, 4, TeamSide.Away);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // Carrier should kick toward slot 10 at (10, 5) — free, lower-forward
        // outscores marked higher-forward. Velocity X positive, Z positive.
        Assert.True(commands[6].Kick.HasValue, "Carrier must emit a KickIntent");
        Vector3Fixed kickVel = commands[6].Kick!.Value.Velocity;
        Assert.True(kickVel.X.RawValue > 0L, $"Kick X must be positive; got {kickVel.X}");
        // Z must be POSITIVE (toward slot 10 at Z=+5) not negative (would mean slot 11 at Z=-5).
        Assert.True(kickVel.Z.RawValue > 0L,
            $"Kick Z must be positive (toward FREE slot 10 at Z=+5), NOT negative; got {kickVel.Z}");
    }

    [Fact]
    public void PassTarget_HeavilyMarkedForward_FallsBackToLongBall()
    {
        // Single eligible candidate (slot 11), HEAVILY MARKED so score ≤ 0.
        // Per task-spec long-ball-fallback contract: carrier must fall
        // back to long-ball at opponent goal (kickSpeed = 18 m/s) instead
        // of threading a pass into hopeless traffic (kickSpeed = 14 m/s).
        BuildPassScenario(out var home, out var away, out var homeTeam,
            out var awayTeam, out var ball);
        // Slot 10 BEHIND carrier (not eligible — must be ahead).
        homeTeam[9] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(-20), Fixed.Zero, Fixed.FromInt(5)),
            Vector3Fixed.Zero, 10, TeamSide.Home);
        // Slot 11 modestly forward at X=12 (within MinPass=8m range).
        // forwardness = 12; with 5 markers penalty = 5*4 = 20; score = -8.
        // Negative → long-ball fallback fires.
        homeTeam[10] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(12), Fixed.Zero, Fixed.FromInt(0)),
            Vector3Fixed.Zero, 11, TeamSide.Home);
        // 5 markers tight on slot 11 (all within 3m of (12, 0)).
        awayTeam[1] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(12), Fixed.Zero, Fixed.FromInt(2)),
            Vector3Fixed.Zero, 2, TeamSide.Away);
        awayTeam[2] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(14), Fixed.Zero, Fixed.Zero),
            Vector3Fixed.Zero, 3, TeamSide.Away);
        awayTeam[3] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(10), Fixed.Zero, Fixed.Zero),
            Vector3Fixed.Zero, 4, TeamSide.Away);
        awayTeam[4] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(12), Fixed.Zero, -Fixed.FromInt(2)),
            Vector3Fixed.Zero, 5, TeamSide.Away);
        awayTeam[5] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(13), Fixed.Zero, Fixed.FromInt(1)),
            Vector3Fixed.Zero, 6, TeamSide.Away);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        Assert.True(commands[6].Kick.HasValue, "Carrier must emit a KickIntent");
        Vector3Fixed kickVel = commands[6].Kick!.Value.Velocity;

        // Long-ball speed magnitude is 18 m/s; pass speed is 14 m/s. The
        // squared magnitudes are 324 and 196 respectively — separation
        // bigger than any sqrt-error budget. Assert kick velocity squared
        // magnitude is at least the long-ball threshold (with tolerance).
        Fixed kickMagSq = kickVel.LengthSquared();
        // Long-ball squared = 18² = 324. Allow ≥ 300 (gives slack on the
        // sqrt+normalize path; the actual long-ball will be exactly 324).
        Assert.True(kickMagSq.RawValue >= Fixed.FromInt(300).RawValue,
            $"Heavily-marked forward should trigger long-ball fallback (kickMagSq ≥ 300 from 18²=324); got {kickMagSq}");
        // Long-ball aims at opponent goal which is at (+52.5, 0, 0) — pure
        // +X velocity direction. A pass to the marked candidate at (12, 0)
        // would also have +X direction so this isn't a discriminator BUT
        // a pass would have Z component ≈ 0 (candidate Z=0). Long-ball
        // direction = ((52.5, 0, 0) - (0, 0, 0)).normalized * 18 = (18, 0, 0).
        // Either way the magnitude test above is the load-bearing pin.
        Assert.True(kickVel.X.RawValue > 0L);
    }

    [Fact]
    public void PassTarget_OpponentPressureTiesResolveByLowerIndex()
    {
        // Two candidates with IDENTICAL score → lower index wins.
        // Slot 10 (RST, index 9) at (15, 5), FREE → score = 15.
        // Slot 11 (LST, index 10) at (15, -5), FREE → score = 15.
        // Tied; lower index (9) wins.
        BuildPassScenario(out var home, out var away, out var homeTeam,
            out var awayTeam, out var ball);
        homeTeam[9] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(15), Fixed.Zero, Fixed.FromInt(5)),
            Vector3Fixed.Zero, 10, TeamSide.Home);
        homeTeam[10] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(15), Fixed.Zero, Fixed.FromInt(-5)),
            Vector3Fixed.Zero, 11, TeamSide.Home);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        Assert.True(commands[6].Kick.HasValue);
        Vector3Fixed kickVel = commands[6].Kick!.Value.Velocity;
        // Lower index (slot 10) wins → kick toward (15, +5). Z positive.
        Assert.True(kickVel.X.RawValue > 0L);
        Assert.True(kickVel.Z.RawValue > 0L,
            $"Tie should resolve to lower index (slot 10 at Z=+5); got Z = {kickVel.Z}");
    }

    [Fact]
    public void PassTarget_Deterministic_AcrossManyRuns()
    {
        // Same inputs → same kick decision.
        BuildPassScenario(out var home, out var away, out var homeTeam,
            out var awayTeam, out var ball);
        homeTeam[9] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(15), Fixed.Zero, Fixed.FromInt(5)),
            Vector3Fixed.Zero, 10, TeamSide.Home);
        homeTeam[10] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(25), Fixed.Zero, Fixed.FromInt(-5)),
            Vector3Fixed.Zero, 11, TeamSide.Home);

        var first = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, first);

        for (int run = 0; run < 100; run++)
        {
            var commands = new PlayerCommand[11];
            BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);
            Assert.Equal(first[6].Kick.HasValue, commands[6].Kick.HasValue);
            if (first[6].Kick.HasValue)
            {
                Assert.Equal(first[6].Kick!.Value.Velocity, commands[6].Kick!.Value.Velocity);
            }
        }
    }
}

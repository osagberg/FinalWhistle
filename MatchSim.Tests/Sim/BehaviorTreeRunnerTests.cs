using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

public sealed class BehaviorTreeRunnerTests
{
    private static Fixed F(int n) => Fixed.FromInt(n);
    private static Vector3Fixed V3(int x, int y, int z) => new(F(x), F(y), F(z));

    /// <summary>
    /// Build an 11-player team from the archetype's formation, all at rest
    /// at their base positions. Used as the "neutral" starting state for
    /// tactical-trigger tests.
    /// </summary>
    private static PlayerState[] TeamFromArchetype(BehaviorTreeArchetype archetype, TeamSide side)
    {
        PlayerState[] team = new PlayerState[11];
        for (int i = 0; i < 11; i++)
        {
            FormationSlot slot = archetype.Formation[i];
            Vector3Fixed pos = side == TeamSide.Home ? slot.HomeBasePosition : slot.AwayBasePosition();
            team[i] = new PlayerState(pos, Vector3Fixed.Zero, jerseyNumber: (byte)(i + 1), side);
        }
        return team;
    }

    private static BallState BallAt(Vector3Fixed pos)
        => new(pos, Vector3Fixed.Zero, Vector3Fixed.Zero);

    #region Determinism

    [Fact]
    public void Tick_SameInputProducesSameOutput100Times()
    {
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] home = TeamFromArchetype(archetype, TeamSide.Home);
        PlayerState[] away = TeamFromArchetype(BehaviorTreeArchetypes.Load("low-block-counter"), TeamSide.Away);
        BallState ball = BallAt(V3(0, 0, 0));
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;

        PlayerCommand[] firstResult = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, home, away, TeamSide.Home, archetype, k, firstResult);

        for (int run = 0; run < 100; run++)
        {
            PlayerCommand[] commands = new PlayerCommand[11];
            BehaviorTreeRunner.Tick(ball, home, away, TeamSide.Home, archetype, k, commands);
            for (int i = 0; i < 11; i++)
            {
                Assert.Equal(firstResult[i], commands[i]);
            }
        }
    }

    #endregion

    #region Argument validation

    [Fact]
    public void Tick_NullArchetype_Throws()
    {
        PlayerState[] home = new PlayerState[11];
        PlayerState[] away = new PlayerState[11];
        for (byte i = 1; i <= 11; i++)
        {
            home[i - 1] = new PlayerState(Vector3Fixed.Zero, Vector3Fixed.Zero, i, TeamSide.Home);
            away[i - 1] = new PlayerState(Vector3Fixed.Zero, Vector3Fixed.Zero, i, TeamSide.Away);
        }
        PlayerCommand[] commands = new PlayerCommand[11];

        Assert.Throws<ArgumentNullException>(() =>
            BehaviorTreeRunner.Tick(BallAt(Vector3Fixed.Zero), home, away,
                TeamSide.Home, null!, PlayerKinematics.Phase3Defaults, commands));
    }

    [Fact]
    public void Tick_WrongTeamLength_Throws()
    {
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] tooFew = new PlayerState[10];
        PlayerState[] full = TeamFromArchetype(archetype, TeamSide.Away);
        PlayerCommand[] commands = new PlayerCommand[11];

        Assert.Throws<ArgumentException>(() =>
            BehaviorTreeRunner.Tick(BallAt(Vector3Fixed.Zero), tooFew, full,
                TeamSide.Home, archetype, PlayerKinematics.Phase3Defaults, commands));
        Assert.Throws<ArgumentException>(() =>
            BehaviorTreeRunner.Tick(BallAt(Vector3Fixed.Zero), full, tooFew,
                TeamSide.Home, archetype, PlayerKinematics.Phase3Defaults, commands));
    }

    [Fact]
    public void Tick_CommandsBufferTooSmall_Throws()
    {
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] home = TeamFromArchetype(archetype, TeamSide.Home);
        PlayerState[] away = TeamFromArchetype(archetype, TeamSide.Away);
        PlayerCommand[] tooSmall = new PlayerCommand[5];

        Assert.Throws<ArgumentException>(() =>
            BehaviorTreeRunner.Tick(BallAt(Vector3Fixed.Zero), home, away,
                TeamSide.Home, archetype, PlayerKinematics.Phase3Defaults, tooSmall));
    }

    #endregion

    #region Press logic

    [Fact]
    public void Tick_OpponentsHavePossession_NearbyDefenderPressesBall()
    {
        // Setup: ball is on the home side near the home GK area; the GK is
        // closest to the ball among home players. An away striker has
        // possession (he's even closer). The home GK should press the ball.
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] home = TeamFromArchetype(archetype, TeamSide.Home);
        PlayerState[] away = TeamFromArchetype(archetype, TeamSide.Away);

        // Ball on home side near GK.
        BallState ball = BallAt(V3(-43, 0, 0));
        // Away striker right next to the ball — closer than home GK at -45.
        away[10] = new PlayerState(V3(-43, 0, 1), Vector3Fixed.Zero, 11, TeamSide.Away);

        PlayerCommand[] commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, home, away, TeamSide.Home, archetype,
            PlayerKinematics.Phase3Defaults, commands);

        // Home GK (index 0) is within 25m of the ball and opponents have
        // possession; should be commanded toward the ball at MaxSpeed.
        PlayerCommand gkCommand = commands[0];
        Assert.Equal(ball.Position, gkCommand.DesiredPosition);
        Assert.Equal(PlayerKinematics.Phase3Defaults.MaxSpeed, gkCommand.DesiredSpeed);
    }

    [Fact]
    public void Tick_OpponentsHavePossession_DistantDefenderHoldsShape()
    {
        // Setup: ball on far side of pitch, opponents have possession; a
        // home player far from the ball should NOT press — they hold shape.
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] home = TeamFromArchetype(archetype, TeamSide.Home);
        PlayerState[] away = TeamFromArchetype(archetype, TeamSide.Away);

        // Ball far on opponent's side (X = +30); home GK is at X = -45 (75m
        // away). Press radius is 25. GK does NOT press.
        BallState ball = BallAt(V3(30, 0, 0));
        // Away striker has possession — placed near the ball.
        away[10] = new PlayerState(V3(30, 0, 1), Vector3Fixed.Zero, 11, TeamSide.Away);

        PlayerCommand[] commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, home, away, TeamSide.Home, archetype,
            PlayerKinematics.Phase3Defaults, commands);

        // Home GK should be holding shape — desired position == base position.
        PlayerCommand gkCommand = commands[0];
        Assert.Equal(home[0].Position, gkCommand.DesiredPosition);
        // Hold-shape jog speed = MaxSpeed * 0.5.
        Fixed expectedJog = PlayerKinematics.Phase3Defaults.MaxSpeed * Fixed.Half;
        Assert.Equal(expectedJog, gkCommand.DesiredSpeed);
    }

    #endregion

    #region Build-up logic

    [Fact]
    public void Tick_OwnTeamHasPossession_NearestPlayerHeadsToOpponentGoal()
    {
        // Setup: home player has the ball clearly (ball at home player's
        // position; no away player near). Build-up: nearest home player
        // (the one with the ball) heads to opponent goal.
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] home = TeamFromArchetype(archetype, TeamSide.Home);
        PlayerState[] away = TeamFromArchetype(archetype, TeamSide.Away);

        // Home striker (index 9, slot 10) at base position (~20, 0, 5);
        // ball at striker's position.
        BallState ball = BallAt(home[9].Position);

        PlayerCommand[] commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, home, away, TeamSide.Home, archetype,
            PlayerKinematics.Phase3Defaults, commands);

        // Striker's command: head to opponent goal at MaxSpeed * BuildupSpeedFactor.
        PlayerCommand strikerCommand = commands[9];
        // Opponent goal for Home is at X = +52.5.
        Fixed goalLine = F(105) / F(2);
        Assert.Equal(goalLine, strikerCommand.DesiredPosition.X);
        Assert.Equal(Fixed.Zero, strikerCommand.DesiredPosition.Y);
        Assert.Equal(Fixed.Zero, strikerCommand.DesiredPosition.Z);
        // Speed = MaxSpeed * BuildupSpeedFactor (matches direct-pressing's 0.95).
        Fixed expectedSpeed = PlayerKinematics.Phase3Defaults.MaxSpeed * archetype.BuildupSpeedFactor;
        Assert.Equal(expectedSpeed, strikerCommand.DesiredSpeed);
    }

    [Fact]
    public void Tick_OwnTeamHasPossession_OtherPlayersHoldShape()
    {
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] home = TeamFromArchetype(archetype, TeamSide.Home);
        PlayerState[] away = TeamFromArchetype(archetype, TeamSide.Away);

        // Home striker (index 9) has the ball.
        BallState ball = BallAt(home[9].Position);

        PlayerCommand[] commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, home, away, TeamSide.Home, archetype,
            PlayerKinematics.Phase3Defaults, commands);

        // GK (index 0) is far from the ball; should hold shape (NOT head to opponent goal).
        PlayerCommand gkCommand = commands[0];
        Assert.Equal(home[0].Position, gkCommand.DesiredPosition);
        Fixed expectedJog = PlayerKinematics.Phase3Defaults.MaxSpeed * Fixed.Half;
        Assert.Equal(expectedJog, gkCommand.DesiredSpeed);
    }

    #endregion

    #region Hold-shape mode (neutral state)

    [Fact]
    public void Tick_BallInBetween_NeitherTeamPossessing_AllPlayersHoldShape()
    {
        // Setup: ball is dead-center, but neither team is closer than the
        // other (mirror-symmetric formation). Possession resolves to
        // opponent (defensive default — strict <).
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] home = TeamFromArchetype(archetype, TeamSide.Home);
        PlayerState[] away = TeamFromArchetype(archetype, TeamSide.Away);

        BallState ball = BallAt(V3(0, 0, 0));

        PlayerCommand[] commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, home, away, TeamSide.Home, archetype,
            PlayerKinematics.Phase3Defaults, commands);

        // Strikers (closest to centre on home side) are within press radius —
        // they should be pressing. But GK + back four are far; they hold shape.
        PlayerCommand gkCommand = commands[0];
        Assert.Equal(home[0].Position, gkCommand.DesiredPosition);
        Fixed expectedJog = PlayerKinematics.Phase3Defaults.MaxSpeed * Fixed.Half;
        Assert.Equal(expectedJog, gkCommand.DesiredSpeed);
    }

    #endregion

    #region Side mirroring

    [Fact]
    public void Tick_AwaySide_FormationIsMirrored()
    {
        // Run for Away — formation X coordinates flip sign. Verify the
        // hold-shape command for an Away GK is at +45 (mirror of home -45).
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] home = TeamFromArchetype(archetype, TeamSide.Home);
        PlayerState[] away = TeamFromArchetype(archetype, TeamSide.Away);

        // Ball is on home side (X = -30) so Away holds shape.
        BallState ball = BallAt(V3(-30, 0, 0));
        // Home striker has possession near the ball.
        home[10] = new PlayerState(V3(-30, 0, 1), Vector3Fixed.Zero, 11, TeamSide.Home);

        PlayerCommand[] commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, away, home, TeamSide.Away, archetype,
            PlayerKinematics.Phase3Defaults, commands);

        // Away GK (index 0) base position — mirrored from Home GK at (-45, 0, 0)
        // → Away GK at (+45, 0, 0).
        PlayerCommand gkCommand = commands[0];
        Assert.Equal(F(45), gkCommand.DesiredPosition.X);
    }

    [Fact]
    public void Tick_AwaySide_OwnTeamPossession_HeadsToHomeGoal()
    {
        // Away has the ball; ball-carrier should head to OPPOSITE end (X = -52.5).
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] away = TeamFromArchetype(archetype, TeamSide.Away);
        PlayerState[] home = TeamFromArchetype(archetype, TeamSide.Home);

        // Away striker has the ball.
        BallState ball = BallAt(away[9].Position);

        PlayerCommand[] commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, away, home, TeamSide.Away, archetype,
            PlayerKinematics.Phase3Defaults, commands);

        PlayerCommand strikerCommand = commands[9];
        // Away's opponent goal is at X = -52.5.
        Fixed expectedX = -(F(105) / F(2));
        Assert.Equal(expectedX, strikerCommand.DesiredPosition.X);
    }

    #endregion

    #region Integration with PlayerActuator

    [Fact]
    public void Integration_BehaviorTreeAndActuator_OwnPlayerAdvancesTowardOpponentGoal()
    {
        // End-to-end: BT emits commands; PlayerActuator advances the player
        // one step. Verify the player's velocity points roughly toward the
        // opponent goal.
        BehaviorTreeArchetype archetype = BehaviorTreeArchetypes.Load("direct-pressing");
        PlayerState[] home = TeamFromArchetype(archetype, TeamSide.Home);
        PlayerState[] away = TeamFromArchetype(archetype, TeamSide.Away);
        PlayerKinematics k = PlayerKinematics.Phase3Defaults;

        // Home striker has the ball at his base position.
        BallState ball = BallAt(home[9].Position);

        PlayerCommand[] commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, home, away, TeamSide.Home, archetype, k, commands);

        // Apply the actuator one tick to the striker.
        PlayerState strikerNext = PlayerActuator.Step(home[9], commands[9].DesiredPosition,
            commands[9].DesiredSpeed, k);

        // Striker should have gained +X velocity (toward opponent goal at +X).
        Assert.True(strikerNext.Velocity.X > Fixed.Zero,
            $"Striker should head toward +X (opponent goal); velocity.X = {strikerNext.Velocity.X}");
        // Y velocity is zero (Player stays on pitch plane per Codex review fix).
        Assert.Equal(Fixed.Zero, strikerNext.Velocity.Y);
    }

    #endregion
}

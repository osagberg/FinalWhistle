using System;
using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Phase-3 polish-pass Option-2 (2026-05-11) tests for the goalkeeper
/// branch added to <see cref="BehaviorTreeRunner.Tick"/>. Closes the user-
/// caught polish-pass-review symptom: "Goalkeepers running through all
/// other players with the ball." Pins:
///
/// <list type="bullet">
///   <item><description>GK in possession never commands "head to opponent
///     goal" (move target stays at GK formation slot).</description></item>
///   <item><description>GK in possession still emits a long-ball kick (the
///     ball doesn't sit glued to the GK dot).</description></item>
///   <item><description>GK without possession holds formation if the ball
///     is outside the own penalty area.</description></item>
///   <item><description>GK without possession charges the ball at MaxSpeed
///     if the ball is inside the own penalty area.</description></item>
///   <item><description>Determinism: same input → same output across
///     repeated invocations.</description></item>
/// </list>
/// </summary>
public sealed class GoalkeeperBehaviorTests
{
    private static readonly PlayerKinematics K = PlayerKinematics.Phase3Defaults;
    private const string HomeArchetypeId = "direct-pressing";
    private const string AwayArchetypeId = "low-block-counter";

    /// <summary>Build an 11-player team using <paramref name="archetype"/>'s
    /// formation positions for the given side, with all players at rest.</summary>
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

    [Fact]
    public void GK_InPossession_DoesNotHeadToOpponentGoal()
    {
        // GK is the nearest-to-ball + own team has possession → before
        // Option-2 the GK would command "head to opponent goal at
        // MaxSpeed*BuildupSpeedFactor". After Option-2: GK heads to base
        // position (formation slot, near own goal-line).
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);

        // Place ball right at the home GK's feet (-45, 0, 0).
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        Vector3Fixed gkPos = homeTeam[0].Position;
        var ball = new BallState(
            position: new Vector3Fixed(gkPos.X, Fixed.Zero, gkPos.Z),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);

        // Away team starts on the far side so the home GK is unambiguously
        // the nearest to the ball.
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // Home GK base position is at (-45, 0, 0) per direct-pressing.yaml.
        // Opponent goal is at (+52.5, 0, 0). The pre-Option-2 behaviour
        // commanded DesiredPosition close to opponent goal; Option-2 commands
        // base position. Assert the move target X is on the GK's side of
        // the pitch (negative X), NOT positive (opponent goal direction).
        Assert.True(
            commands[0].DesiredPosition.X.RawValue < 0L,
            $"GK in possession should not head to opponent goal; got move target X = {commands[0].DesiredPosition.X}.");
    }

    [Fact]
    public void GK_InPossession_EmitsLongBallKick()
    {
        // GK in possession at goal-line must still emit a KickIntent so the
        // ball doesn't sit glued to the GK forever. The kick logic is shared
        // with outfield carriers via ChoosePassKick — only the move command
        // differs for the GK branch.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);

        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        Vector3Fixed gkPos = homeTeam[0].Position;
        var ball = new BallState(
            position: new Vector3Fixed(gkPos.X, Fixed.Zero, gkPos.Z),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);

        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // Kick velocity should be non-zero with a positive X (toward
        // opponent goal direction for HOME).
        Assert.True(commands[0].Kick.HasValue,
            "GK on the ball with ball settled must emit a KickIntent.");
        Vector3Fixed kickVelocity = commands[0].Kick!.Value.Velocity;
        Assert.True(kickVelocity.LengthSquared().RawValue > 0L,
            "GK kick velocity must be non-zero.");
        Assert.True(kickVelocity.X.RawValue > 0L,
            $"GK kick should travel toward opponent goal (positive X for HOME); got velocity X = {kickVelocity.X}.");
    }

    [Fact]
    public void GK_WithoutPossession_BallOutsidePenaltyArea_HoldsFormation()
    {
        // Ball at midfield, away team in possession (their nearest is
        // closer). Home GK must NOT chase; commands hold-formation move
        // target (base position, half speed).
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);

        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        // Ball at midfield centre. Away team formation positions on +X side
        // are closer (positive X side); home team's nearest is at +20
        // (striker, slot 10/11), but home GK is at -45 — far from ball.
        var ball = new BallState(
            position: new Vector3Fixed(Fixed.FromInt(10), Fixed.Zero, Fixed.Zero),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);

        // Move away-team striker right onto the ball so away has unambiguous possession.
        Vector3Fixed onBall = new(ball.Position.X, Fixed.Zero, ball.Position.Z);
        awayTeam[9] = new PlayerState(onBall, Vector3Fixed.Zero, awayTeam[9].JerseyNumber, awayTeam[9].Side);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // GK base position X is at -45 (5m off goal-line, well outside the
        // ball's location). Command should target base, not chase ball.
        Vector3Fixed gkBasePos = homeTeam[0].Position;  // since team built at formation positions
        Assert.Equal(gkBasePos, commands[0].DesiredPosition);
        // Speed should be MaxSpeed/2 (jog), not MaxSpeed (sprint).
        Assert.Equal(K.MaxSpeed * Fixed.Half, commands[0].DesiredSpeed);
    }

    [Fact]
    public void GK_WithoutPossession_BallInsidePenaltyArea_ChargesAtBall()
    {
        // Ball inside own penalty area, opponent has possession or contest
        // unclear. GK heads to ball at MaxSpeed (last-line defence).
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);

        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        // Ball at home penalty spot ≈ (-41, 0, 0). 11.5m from goal-line —
        // safely inside the 16.5m penalty area (|x - (-52.5)| = 11.5 <= 16.5).
        var ball = new BallState(
            position: new Vector3Fixed(-Fixed.FromInt(41), Fixed.Zero, Fixed.Zero),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);

        // Move an away striker on top of the ball so away has possession.
        awayTeam[9] = new PlayerState(ball.Position, Vector3Fixed.Zero, awayTeam[9].JerseyNumber, awayTeam[9].Side);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // GK should head to the ball at MaxSpeed.
        Assert.Equal(ball.Position, commands[0].DesiredPosition);
        Assert.Equal(K.MaxSpeed, commands[0].DesiredSpeed);
    }

    [Fact]
    public void GK_Deterministic_AcrossManyRuns()
    {
        // Same inputs → same GK command every run. Pins canonical-state
        // stability for the new GK branch.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);

        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        Vector3Fixed gkPos = homeTeam[0].Position;
        var ball = new BallState(
            position: new Vector3Fixed(gkPos.X, Fixed.Zero, gkPos.Z),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        var first = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, first);

        for (int run = 0; run < 100; run++)
        {
            var commands = new PlayerCommand[11];
            BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);
            Assert.Equal(first[0].DesiredPosition, commands[0].DesiredPosition);
            Assert.Equal(first[0].DesiredSpeed, commands[0].DesiredSpeed);
            Assert.Equal(first[0].Kick.HasValue, commands[0].Kick.HasValue);
            if (first[0].Kick.HasValue)
            {
                Assert.Equal(first[0].Kick!.Value.Velocity, commands[0].Kick!.Value.Velocity);
            }
        }
    }
}

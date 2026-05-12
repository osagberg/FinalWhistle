using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Phase-3 polish-pass Option-3 (2026-05-11) tests for off-ball formation
/// translation in <see cref="BehaviorTreeRunner.Tick"/>'s hold-shape branch.
/// Closes the user-caught polish-pass-review symptom: "Just straight line
/// running for the most part." Before Option-3, hold-shape commanded
/// STATIC formation base positions — outfield players never shifted with
/// the ball, so the team played as 11 disconnected pucks. After: formation
/// shape preserved, centroid shifts toward ball position.
/// </summary>
public sealed class OffBallFormationTranslationTests
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

    [Fact]
    public void BallAtCentre_HoldShapeProducesUnshiftedBasePosition()
    {
        // Ball at (0, 0, 0): translation = (0, 0, 0). Hold-shape players
        // command DesiredPosition == basePosition exactly.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);
        // Move an away player onto the ball so home is unambiguously NOT in
        // possession (we want hold-shape, not build-up). Push the rest of
        // away outside home's press radius so home doesn't enter press mode
        // either. The simplest construction: ball at centre + away striker
        // (slot 11 / index 10) at centre with home pressers ≥ PressRadius
        // away. Home archetype press radius is 25m; with home base
        // positions at -30 to +20, anything within 25m of centre would
        // press. So we have to test a SPECIFIC home player whose base
        // position is > PressRadius from the ball: slot 1 (GK) is at -45 X,
        // 45m from ball — but GK has its own branch.
        // Pick slot 2 (RB) at (-25, 20) — distance √(25² + 20²) ≈ 32m > 25m
        // press radius. Hold-shape applies.
        var ball = new BallState(
            position: new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);
        awayTeam[10] = new PlayerState(ball.Position, Vector3Fixed.Zero, 11, TeamSide.Away);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // Slot 2 = home RB at (-25, 0, 20). With ball at origin, translation
        // is (0,0,0). DesiredPosition should equal base position.
        Vector3Fixed basePosition = new(Fixed.FromInt(-25), Fixed.Zero, Fixed.FromInt(20));
        Assert.Equal(basePosition, commands[1].DesiredPosition);
    }

    [Fact]
    public void BallInAttackingThird_HomeOutfieldShiftsForward()
    {
        // Ball at +40 X (attacking third for home). Tests Option-3 non-
        // defender translation; uses slot 6 (RM, base -5, 20) which is
        // NOT in roster slots 2-5 so it stays on the standard 0.5
        // translation regardless of possession (round-3 #4 only adjusts
        // defenders). Expected: -5 + 40 × 0.5 = +15.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        var ball = new BallState(
            position: new Vector3Fixed(Fixed.FromInt(40), Fixed.Zero, Fixed.Zero),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);
        // Put an away player on the ball so home doesn't have possession.
        awayTeam[10] = new PlayerState(ball.Position, Vector3Fixed.Zero, 11, TeamSide.Away);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // Expected translated base for slot 6 (index 5): (-5 + 40*0.5, 0, 20 + 0*0.3) = (15, 0, 20).
        // Distance from RM (-5, 20) to ball (40, 0) = √(45² + 20²) ≈ 49m
        // > 25m press radius → hold-shape applies; out-of-possession but
        // RM is NOT a defender so uses standard 0.5 factor.
        Vector3Fixed expected = new(Fixed.FromInt(15), Fixed.Zero, Fixed.FromInt(20));
        Assert.Equal(expected, commands[5].DesiredPosition);
    }

    [Fact]
    public void BallInDefendingThird_HomeOutfieldRetreats()
    {
        // Ball at -40 X (defending third for home). Translation X = -20.
        // Slot 11 (LST) at base (20, -5) → translated (0, -5).
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        var ball = new BallState(
            position: new Vector3Fixed(Fixed.FromInt(-40), Fixed.Zero, Fixed.Zero),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);
        // Away player on ball so home doesn't have possession.
        awayTeam[10] = new PlayerState(ball.Position, Vector3Fixed.Zero, 11, TeamSide.Away);
        // Slot 11 home LST base (20, 0, -5). Distance to ball at -40:
        // √(60² + 5²) ≈ 60m > 25m press radius. Hold shape applies.

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // Expected translated base: (20 + -40*0.5, 0, -5 + 0*0.3) = (0, 0, -5).
        Vector3Fixed expected = new(Fixed.Zero, Fixed.Zero, Fixed.FromInt(-5));
        Assert.Equal(expected, commands[10].DesiredPosition);
    }

    [Fact]
    public void BallNearSideline_LateralTranslationClampedToPitchBounds()
    {
        // Ball at Z = +33 (near +34 sideline). Lateral shift factor 0.3
        // applied to slot 2 (RB) at base (-25, 20): translated Z =
        // 20 + 33 * 0.3 = 29.9 — UNDER the 34m clamp. Fine.
        //
        // To force the clamp, pick slot 6 (RM) at base (-5, 20) and put
        // ball at Z = 60 (impossible in practice but legal for the clamp
        // test): translated Z = 20 + 60 * 0.3 = 38 > 34, must clamp to 34.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        // Ball at (0, 0, 60). Z is out of pitch in reality but the BT
        // doesn't enforce that — sim could put the ball there temporarily
        // during a deflection. The clamp must still hold.
        var ball = new BallState(
            position: new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.FromInt(60)),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);
        awayTeam[10] = new PlayerState(ball.Position, Vector3Fixed.Zero, 11, TeamSide.Away);

        var commands = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);

        // Slot 6 home RM at base (-5, 0, 20). Translated Z = 20 + 60*0.3 = 38.
        // Must clamp to 34.
        // Slot 6 = roster slot 6 = index 5.
        Fixed actualZ = commands[5].DesiredPosition.Z;
        Fixed expectedZClamp = Fixed.FromInt(34);
        Assert.Equal(expectedZClamp, actualZ);
        // Also verify Y stays 0 (pitch plane).
        Assert.Equal(Fixed.Zero, commands[5].DesiredPosition.Y);
    }

    [Fact]
    public void Deterministic_AcrossManyRuns()
    {
        // Same inputs → same hold-shape command across repeated invocations.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        PlayerState[] homeTeam = BuildFormationTeam(home, TeamSide.Home);
        PlayerState[] awayTeam = BuildFormationTeam(away, TeamSide.Away);

        var ball = new BallState(
            position: new Vector3Fixed(Fixed.FromInt(25), Fixed.Zero, Fixed.FromInt(-10)),
            velocity: Vector3Fixed.Zero,
            spin: Vector3Fixed.Zero);
        awayTeam[10] = new PlayerState(ball.Position, Vector3Fixed.Zero, 11, TeamSide.Away);

        var first = new PlayerCommand[11];
        BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, first);

        for (int run = 0; run < 100; run++)
        {
            var commands = new PlayerCommand[11];
            BehaviorTreeRunner.Tick(ball, homeTeam, awayTeam, TeamSide.Home, home, K, commands);
            for (int i = 0; i < 11; i++)
            {
                Assert.Equal(first[i].DesiredPosition, commands[i].DesiredPosition);
                Assert.Equal(first[i].DesiredSpeed, commands[i].DesiredSpeed);
            }
        }
    }
}

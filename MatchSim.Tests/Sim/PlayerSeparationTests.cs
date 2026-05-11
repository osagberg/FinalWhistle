using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Polish-pass Option 1 (2026-05-11) tests for
/// <see cref="PlayerSeparation.Step"/>. Closes the "22 dots phase through
/// each other" visual the user caught at the polish-pass review. Pins the
/// minimum-separation invariant + determinism + no-op-on-disjoint cases.
/// </summary>
public sealed class PlayerSeparationTests
{
    private static readonly PlayerKinematics K = PlayerKinematics.Phase3Defaults;
    private static readonly Fixed MinSep = K.Radius + K.Radius;
    private static readonly Fixed MinSepSq = MinSep * MinSep;

    private static MatchSimulationState BuildStateWith(
        PlayerState[] homeTeam, PlayerState[] awayTeam)
    {
        return new MatchSimulationState(Tick.Zero, BallState.AtRest, homeTeam, awayTeam);
    }

    private static PlayerState Player(Vector3Fixed pos, byte jersey, TeamSide side)
        => new(pos, Vector3Fixed.Zero, jersey, side);

    private static PlayerState[] BuildTeamWithTwoCloseAndRestSpread(
        Vector3Fixed posA, Vector3Fixed posB, TeamSide side)
    {
        // 2 close at slots 1+2; 9 more spread far apart so they don't collide
        // with each other or with the close pair.
        var team = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        team[0] = Player(posA, 1, side);
        team[1] = Player(posB, 2, side);
        for (int i = 2; i < 11; i++)
        {
            // Stack outside any plausible collision range (10m+ separation).
            team[i] = Player(new Vector3Fixed(
                Fixed.FromInt(-50 + (i * 5)), Fixed.Zero, Fixed.FromInt(-30)),
                (byte)(i + 1), side);
        }
        return team;
    }

    [Fact]
    public void Step_OverlappingHomePair_ResolvesToMinDistance()
    {
        // Two home players at exactly the same X but 0.5m apart on Z.
        // 0.5m < MinSep (1m), so they overlap; expect separation pushes
        // them to >= 1m apart on Z.
        var team = BuildTeamWithTwoCloseAndRestSpread(
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.FromInt(1) / Fixed.FromInt(2)),
            TeamSide.Home);
        var state = BuildStateWith(team, BuildSparseAwayTeam());
        PlayerSeparation.Step(state, K);

        Fixed distSq = Vector3Fixed.DistanceSquared(
            state.HomeTeam[0].Position, state.HomeTeam[1].Position);
        Assert.True(distSq >= MinSepSq,
            $"Expected dist² >= {MinSepSq} after separation; got {distSq}.");
    }

    [Fact]
    public void Step_DisjointPair_NoOp()
    {
        // 5m apart — well clear of MinSep.
        var team = BuildTeamWithTwoCloseAndRestSpread(
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.FromInt(5)),
            TeamSide.Home);
        PlayerState before0 = team[0];
        PlayerState before1 = team[1];
        var state = BuildStateWith(team, BuildSparseAwayTeam());
        PlayerSeparation.Step(state, K);

        Assert.Equal(before0.Position, state.HomeTeam[0].Position);
        Assert.Equal(before1.Position, state.HomeTeam[1].Position);
    }

    [Fact]
    public void Step_CoincidentPair_PushesAlongPlusXByConvention()
    {
        // Exactly coincident — zero-magnitude direction triggers the
        // "push along +X by convention" fallback. Asserts deterministic
        // non-crashing behavior + that the two players DO end up apart.
        var team = BuildTeamWithTwoCloseAndRestSpread(
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            TeamSide.Home);
        var state = BuildStateWith(team, BuildSparseAwayTeam());
        PlayerSeparation.Step(state, K);

        Fixed distSq = Vector3Fixed.DistanceSquared(
            state.HomeTeam[0].Position, state.HomeTeam[1].Position);
        Assert.True(distSq >= MinSepSq,
            $"Coincident pair must end up at >= MinSep apart; got {distSq}.");
        // +X push: i=0 goes -X, j=1 goes +X (b - a direction = +X fallback,
        // a -= correction → -X, b += correction → +X).
        Assert.True(state.HomeTeam[1].Position.X > state.HomeTeam[0].Position.X);
    }

    [Fact]
    public void Step_Deterministic_AcrossManyRuns()
    {
        // Same input → same output every run. Pins canonical-state stability.
        var team = BuildTeamWithTwoCloseAndRestSpread(
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.FromInt(1) / Fixed.FromInt(4)),
            TeamSide.Home);
        var state1 = BuildStateWith(team, BuildSparseAwayTeam());
        var team2 = BuildTeamWithTwoCloseAndRestSpread(
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.FromInt(1) / Fixed.FromInt(4)),
            TeamSide.Home);
        var state2 = BuildStateWith(team2, BuildSparseAwayTeam());

        for (int run = 0; run < 100; run++)
        {
            PlayerSeparation.Step(state1, K);
            PlayerSeparation.Step(state2, K);
        }

        Assert.Equal(state1.HomeTeam[0].Position, state2.HomeTeam[0].Position);
        Assert.Equal(state1.HomeTeam[1].Position, state2.HomeTeam[1].Position);
    }

    [Fact]
    public void Step_HomeAndAwayClose_BothTeamsResolveOverlap()
    {
        // Home player at origin, Away player at +0.3 on Z. Cross-team pair.
        var homeTeam = BuildTeamWithTwoCloseAndRestSpread(
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            new Vector3Fixed(Fixed.FromInt(50), Fixed.Zero, Fixed.FromInt(-30)),
            TeamSide.Home);
        var awayTeam = BuildTeamWithTwoCloseAndRestSpread(
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.FromInt(3) / Fixed.FromInt(10)),
            new Vector3Fixed(Fixed.FromInt(-50), Fixed.Zero, Fixed.FromInt(30)),
            TeamSide.Away);
        var state = BuildStateWith(homeTeam, awayTeam);
        PlayerSeparation.Step(state, K);

        Fixed crossSq = Vector3Fixed.DistanceSquared(
            state.HomeTeam[0].Position, state.AwayTeam[0].Position);
        Assert.True(crossSq >= MinSepSq,
            $"Cross-team pair must resolve; got {crossSq}.");
    }

    [Fact]
    public void Step_DoesNotMutateBallState()
    {
        // Even when separation has work to do (overlapping pair),
        // PlayerSeparation.Step must leave Ball.Position + Ball.Velocity
        // byte-for-byte unchanged. Pins the contract that separation is a
        // player-position-only pass — ball state belongs to ApplyKick +
        // BallPhysics, never to separation. Without this pin, a careless
        // future refactor that "also nudges the ball if a player is on it"
        // would silently break MatchSimulationRunner ordering invariants.
        var ballBefore = new BallState(
            position: new Vector3Fixed(Fixed.FromInt(5), Fixed.FromInt(1), Fixed.FromInt(-3)),
            velocity: new Vector3Fixed(Fixed.FromInt(7), Fixed.Zero, Fixed.FromInt(2)),
            spin: Vector3Fixed.Zero);
        var homeTeam = BuildTeamWithTwoCloseAndRestSpread(
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.Zero),
            new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.FromInt(1) / Fixed.FromInt(2)),
            TeamSide.Home);
        var state = new MatchSimulationState(Tick.Zero, ballBefore, homeTeam, BuildSparseAwayTeam());

        PlayerSeparation.Step(state, K);

        Assert.Equal(ballBefore.Position, state.Ball.Position);
        Assert.Equal(ballBefore.Velocity, state.Ball.Velocity);
    }

    [Fact]
    public void StepOrderInRunner_DoesNotChangeBallState_WhenNoCarrier()
    {
        // Step-order regression: run ONE MatchSimulationRunner tick with two
        // home players deliberately overlapping at midfield, while the ball
        // sits far from any player (no carrier within possession radius this
        // tick). The runner ordering is BT → Actuator → PlayerSeparation →
        // ApplyKick → BallPhysics → MatchRules. Because no carrier is on
        // the ball, ApplyKick is a no-op; ball must evolve PURELY through
        // BallPhysics.Step. If the separation pass ever quietly nudged
        // Ball state (e.g. via a mis-typed array index in a future refactor
        // that touches state.Ball instead of state.HomeTeam), this test
        // catches it: the ball position would differ from the pure-physics
        // expected output.
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load("low-block-counter");
        MatchSimulationConfig config = new(Seed.FromUInt64(0x1234567890ABCDEFUL));

        // Ball at near-home-goal corner with low velocity. Far from any
        // formation slot in either archetype so no kick fires this tick.
        var initialBall = new BallState(
            position: new Vector3Fixed(Fixed.FromInt(-48), Fixed.Zero, Fixed.FromInt(28)),
            velocity: new Vector3Fixed(Fixed.One, Fixed.Zero, Fixed.Zero),
            spin: Vector3Fixed.Zero);

        MatchSimulationState state = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, initialBall, home, away);

        // Force two outfield home players (slots 6 + 7) into an overlapping
        // pair near midfield — far from both the ball AND the away team —
        // so PlayerSeparation has work to do but kick/physics are isolated.
        state.HomeTeam[5] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(10), Fixed.Zero, Fixed.FromInt(15)),
            Vector3Fixed.Zero, 6, TeamSide.Home);
        state.HomeTeam[6] = new PlayerState(
            new Vector3Fixed(Fixed.FromInt(10), Fixed.Zero,
                Fixed.FromInt(15) + Fixed.FromInt(1) / Fixed.FromInt(4)),
            Vector3Fixed.Zero, 7, TeamSide.Home);

        // Expected ball state = pure BallPhysics.Step output (kick is no-op).
        BallState expectedBall = BallPhysics.Step(initialBall, BallPhysicsCoefficients.Phase3Seeds);

        MatchSimulationRunner.RunTicks(
            state, home, away,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            config,
            ticks: 1);

        Assert.Equal(expectedBall.Position, state.Ball.Position);
        Assert.Equal(expectedBall.Velocity, state.Ball.Velocity);

        // And the separation pass actually fired: the two close home
        // players should now be ≥ MinSep apart on Z.
        Fixed pairDistSq = Vector3Fixed.DistanceSquared(
            state.HomeTeam[5].Position, state.HomeTeam[6].Position);
        Assert.True(pairDistSq >= MinSepSq,
            $"Sanity: separation should have fired; pair dist² = {pairDistSq}.");
    }

    private static PlayerState[] BuildSparseAwayTeam()
    {
        // Away team spread far away so it doesn't interact with home tests.
        var team = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        for (int i = 0; i < 11; i++)
        {
            team[i] = Player(new Vector3Fixed(
                Fixed.FromInt(40 + (i * 1)), Fixed.Zero, Fixed.FromInt(30)),
                (byte)(i + 1), TeamSide.Away);
        }
        return team;
    }
}

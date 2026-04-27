using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Cross-platform determinism gate per <c>design/match-engine.md §Prototype gate</c>:
/// "same seed → same canonical state hash on Win/Mac/Linux." Composes the
/// full BT + Player + Ball stack end-to-end, runs N ticks from a pinned
/// initial state, and asserts the SHA256 of the final canonical state
/// matches a literal pinned hash. Hash values were computed once on macOS
/// (2026-04-27); the Tier-A CI matrix subsequently runs these same tests
/// on Win + Linux runners. Any drift = a real bug, not a tolerance issue.
///
/// <para>
/// Coverage strategy:
/// </para>
/// <list type="bullet">
///   <item><description><strong>Pinned-hash test</strong> — the bedrock.
///     Specific initial state + N=60 ticks + direct-pressing vs low-block-counter
///     produces a SHA256 we lock to a string literal. Win/Mac/Linux disagreement
///     means a determinism leak.</description></item>
///   <item><description><strong>Determinism test</strong> — 100 fresh
///     identical runs produce 1 distinct hash. Catches hidden non-determinism
///     (Random / DateTime / static state / iteration order).</description></item>
///   <item><description><strong>Sensitivity test</strong> — different
///     initial states produce different hashes. Sanity check that the hash
///     isn't constant.</description></item>
///   <item><description><strong>Order-stability test</strong> — encoding
///     home before away vs away before home produces different hashes.
///     Regression guard against accidental sort-on-encode.</description></item>
/// </list>
/// </summary>
public sealed class MatchDeterminismTests
{
    private static Fixed F(int n) => Fixed.FromInt(n);
    private static Vector3Fixed V3(int x, int y, int z) => new(F(x), F(y), F(z));

    /// <summary>Pinned smoke-seed reference value (computed on macOS 2026-04-27 via the test below; CI matrix verifies Win/Linux match).</summary>
    private const string SmokeSeed60TickHash = "sha256:299cdb0cbbc9606e141db278a14585780d0e3b5dbfb8815f634af89be7f6118a";

    #region Composition helper — runs the full BT + Player + Ball stack

    /// <summary>
    /// State held by the simulator between ticks. Plain class with mutable
    /// arrays; the loop overwrites in place to avoid per-tick allocations.
    /// </summary>
    private sealed class MatchSimulationState
    {
        public Tick CurrentTick;
        public BallState Ball;
        public PlayerState[] HomeTeam = new PlayerState[11];
        public PlayerState[] AwayTeam = new PlayerState[11];
    }

    /// <summary>
    /// Run N ticks of the match, advancing each layer in deterministic
    /// order: BT.Tick × 2 → PlayerActuator.Step × 22 → BallPhysics.Step.
    /// Mutates <paramref name="state"/> in place; final state is the state
    /// after the Nth tick.
    /// </summary>
    private static void RunMatch(
        MatchSimulationState state,
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype,
        PlayerKinematics kinematics,
        BallPhysicsCoefficients ballCoeffs,
        int ticks)
    {
        // Pre-allocate command buffers — re-used each tick (allocation-free
        // hot path per the rules-file matchsim discipline).
        PlayerCommand[] homeCommands = new PlayerCommand[11];
        PlayerCommand[] awayCommands = new PlayerCommand[11];

        for (int t = 0; t < ticks; t++)
        {
            // 1. BT for both sides — emits desired (position, speed) per player.
            BehaviorTreeRunner.Tick(state.Ball, state.HomeTeam, state.AwayTeam,
                TeamSide.Home, homeArchetype, kinematics, homeCommands);
            BehaviorTreeRunner.Tick(state.Ball, state.AwayTeam, state.HomeTeam,
                TeamSide.Away, awayArchetype, kinematics, awayCommands);

            // 2. PlayerActuator advances each player. Order: home roster
            //    indices 0..10 then away roster 0..10. Iteration order is
            //    locked at the runner-call level.
            for (int i = 0; i < 11; i++)
            {
                state.HomeTeam[i] = PlayerActuator.Step(
                    state.HomeTeam[i], homeCommands[i].DesiredPosition,
                    homeCommands[i].DesiredSpeed, kinematics);
            }
            for (int i = 0; i < 11; i++)
            {
                state.AwayTeam[i] = PlayerActuator.Step(
                    state.AwayTeam[i], awayCommands[i].DesiredPosition,
                    awayCommands[i].DesiredSpeed, kinematics);
            }

            // 3. Ball advances after players (the canonical match-loop order;
            //    the inverse — ball before players — would still be
            //    deterministic but produce a different hash).
            state.Ball = BallPhysics.Step(state.Ball, ballCoeffs);

            state.CurrentTick = state.CurrentTick + 1L;
        }
    }

    /// <summary>
    /// Build the canonical Phase-3 smoke fixture initial state: 22 players
    /// at their archetype formation positions, ball at centre, tick 0.
    /// </summary>
    private static MatchSimulationState BuildSmokeFixture(
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype)
    {
        MatchSimulationState state = new()
        {
            CurrentTick = Tick.Zero,
            Ball = BallState.AtRest,
        };

        for (int i = 0; i < 11; i++)
        {
            FormationSlot homeSlot = homeArchetype.Formation[i];
            state.HomeTeam[i] = new PlayerState(
                position: homeSlot.HomeBasePosition,
                velocity: Vector3Fixed.Zero,
                jerseyNumber: (byte)(i + 1),
                side: TeamSide.Home);
        }
        for (int i = 0; i < 11; i++)
        {
            FormationSlot awaySlot = awayArchetype.Formation[i];
            state.AwayTeam[i] = new PlayerState(
                position: awaySlot.AwayBasePosition(),
                velocity: Vector3Fixed.Zero,
                jerseyNumber: (byte)(i + 1),
                side: TeamSide.Away);
        }

        return state;
    }

    #endregion

    #region Pinned-hash bedrock — cross-platform parity

    [Fact]
    public void Match_SmokeFixture60Ticks_ProducesPinnedCanonicalStateHash()
    {
        // The cross-platform-determinism gate. This SHA256 was computed on
        // macOS 2026-04-27. The Tier-A CI matrix runs this same test on
        // Win + Linux runners; all three must produce the identical hash.
        // Any disagreement = a determinism leak in BT / Player / Ball / hashing
        // pipeline. Pixel rendering is NOT exercised — this is purely the
        // canonical sim-state.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        MatchSimulationState state = BuildSmokeFixture(direct, lowBlock);

        RunMatch(state, direct, lowBlock,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            ticks: 60);

        string hash = MatchCanonicalState.ComputeHash(
            state.CurrentTick, state.Ball, state.HomeTeam, state.AwayTeam);

        // Use Assert.True with full message so xUnit doesn't truncate the
        // failure output (Assert.Equal truncates string mismatches at ~38 chars).
        Assert.True(hash == SmokeSeed60TickHash,
            $"Canonical-state hash mismatch.\nExpected: {SmokeSeed60TickHash}\nActual:   {hash}");
    }

    #endregion

    #region Determinism — same input → same hash 100 times

    [Fact]
    public void Match_SameInitialState_Run100Times_AllProduceIdenticalHash()
    {
        // 100 fresh identical runs. Single distinct hash means no hidden
        // non-determinism (Random / DateTime / static state / iteration
        // order / dictionary hash randomization).
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");

        HashSet<string> distinctHashes = new();
        for (int run = 0; run < 100; run++)
        {
            MatchSimulationState state = BuildSmokeFixture(direct, lowBlock);
            RunMatch(state, direct, lowBlock,
                PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds,
                ticks: 60);
            distinctHashes.Add(MatchCanonicalState.ComputeHash(
                state.CurrentTick, state.Ball, state.HomeTeam, state.AwayTeam));
        }

        Assert.Single(distinctHashes);
    }

    [Fact]
    public void Match_TickByTickHashing_IsStableAcrossRuns()
    {
        // Sample the canonical hash every 10 ticks across two independent
        // runs and assert each pair matches. Catches non-determinism that
        // emerges only mid-match (e.g., a bug that surfaces once the ball
        // has rolled into a press radius).
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");

        const int Samples = 6;  // every 10 ticks for 60 ticks total
        string[] runA = new string[Samples];
        string[] runB = new string[Samples];

        MatchSimulationState a = BuildSmokeFixture(direct, lowBlock);
        MatchSimulationState b = BuildSmokeFixture(direct, lowBlock);

        for (int s = 0; s < Samples; s++)
        {
            RunMatch(a, direct, lowBlock, PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds, ticks: 10);
            RunMatch(b, direct, lowBlock, PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds, ticks: 10);

            runA[s] = MatchCanonicalState.ComputeHash(
                a.CurrentTick, a.Ball, a.HomeTeam, a.AwayTeam);
            runB[s] = MatchCanonicalState.ComputeHash(
                b.CurrentTick, b.Ball, b.HomeTeam, b.AwayTeam);

            Assert.Equal(runA[s], runB[s]);
        }
    }

    #endregion

    #region Sensitivity — different inputs → different hashes

    [Fact]
    public void Match_DifferentArchetypes_ProduceDifferentHashes()
    {
        // Run the smoke fixture two ways: direct-vs-lowblock vs
        // lowblock-vs-direct. The team-side asymmetry means the canonical
        // states will diverge — distinct hashes are mandatory.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");

        MatchSimulationState a = BuildSmokeFixture(direct, lowBlock);
        MatchSimulationState b = BuildSmokeFixture(lowBlock, direct);

        RunMatch(a, direct, lowBlock, PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds, ticks: 60);
        RunMatch(b, lowBlock, direct, PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds, ticks: 60);

        string hashA = MatchCanonicalState.ComputeHash(
            a.CurrentTick, a.Ball, a.HomeTeam, a.AwayTeam);
        string hashB = MatchCanonicalState.ComputeHash(
            b.CurrentTick, b.Ball, b.HomeTeam, b.AwayTeam);

        Assert.NotEqual(hashA, hashB);
    }

    [Fact]
    public void Match_DifferentBallStartPosition_ProduceDifferentHashes()
    {
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");

        MatchSimulationState a = BuildSmokeFixture(direct, lowBlock);
        MatchSimulationState b = BuildSmokeFixture(direct, lowBlock);
        // Nudge ball off-centre by 1m for state b.
        b.Ball = new BallState(V3(1, 0, 0), Vector3Fixed.Zero, Vector3Fixed.Zero);

        RunMatch(a, direct, lowBlock, PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds, ticks: 60);
        RunMatch(b, direct, lowBlock, PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds, ticks: 60);

        string hashA = MatchCanonicalState.ComputeHash(
            a.CurrentTick, a.Ball, a.HomeTeam, a.AwayTeam);
        string hashB = MatchCanonicalState.ComputeHash(
            b.CurrentTick, b.Ball, b.HomeTeam, b.AwayTeam);

        Assert.NotEqual(hashA, hashB);
    }

    #endregion

    #region Order-stability — encoding order matters

    [Fact]
    public void CanonicalState_EncodingOrder_HomeBeforeAway_DiffersFromAwayBeforeHome()
    {
        // Regression guard: the canonical-state encoding writes home before
        // away (locked at v1 per MatchCanonicalState contract). Any future
        // refactor that accidentally swaps order MUST trip this test.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        MatchSimulationState state = BuildSmokeFixture(direct, lowBlock);

        // Hash 1: canonical order (home, away).
        string canonicalHash = MatchCanonicalState.ComputeHash(
            state.CurrentTick, state.Ball, state.HomeTeam, state.AwayTeam);

        // Hash 2: pass the teams swapped — should produce a different hash.
        string swappedHash = MatchCanonicalState.ComputeHash(
            state.CurrentTick, state.Ball, state.AwayTeam, state.HomeTeam);

        Assert.NotEqual(canonicalHash, swappedHash);
    }

    [Fact]
    public void CanonicalState_TickAdvances_ChangesHash()
    {
        // The Tick is part of the canonical state. Advancing the tick must
        // change the hash even if everything else is identical.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        MatchSimulationState state = BuildSmokeFixture(direct, lowBlock);

        string hashAtTick0 = MatchCanonicalState.ComputeHash(
            Tick.Zero, state.Ball, state.HomeTeam, state.AwayTeam);
        string hashAtTick1 = MatchCanonicalState.ComputeHash(
            Tick.One, state.Ball, state.HomeTeam, state.AwayTeam);

        Assert.NotEqual(hashAtTick0, hashAtTick1);
    }

    #endregion

    #region MatchCanonicalState API surface

    [Fact]
    public void Write_ProducesExpectedByteCount()
    {
        // 8 (Tick) + 72 (Ball) + 4 (home count) + 11×50 (home players) +
        // 4 (away count) + 11×50 (away players) = 1188 bytes.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        MatchSimulationState state = BuildSmokeFixture(direct, lowBlock);

        CanonicalEncoder encoder = new();
        MatchCanonicalState.Write(encoder, state.CurrentTick, state.Ball,
            state.HomeTeam, state.AwayTeam);

        Assert.Equal(1188, encoder.WrittenCount);
    }

    [Fact]
    public void Write_NullEncoder_Throws()
    {
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        MatchSimulationState state = BuildSmokeFixture(direct, lowBlock);

        Assert.Throws<ArgumentNullException>(() =>
            MatchCanonicalState.Write(null!, state.CurrentTick, state.Ball,
                state.HomeTeam, state.AwayTeam));
    }

    [Fact]
    public void Write_WrongTeamLength_Throws()
    {
        CanonicalEncoder encoder = new();
        PlayerState[] tooFew = new PlayerState[10];
        for (byte i = 1; i <= 10; i++)
        {
            tooFew[i - 1] = new PlayerState(Vector3Fixed.Zero, Vector3Fixed.Zero, i, TeamSide.Home);
        }
        PlayerState[] full = new PlayerState[11];
        for (byte i = 1; i <= 11; i++)
        {
            full[i - 1] = new PlayerState(Vector3Fixed.Zero, Vector3Fixed.Zero, i, TeamSide.Away);
        }

        Assert.Throws<ArgumentException>(() =>
            MatchCanonicalState.Write(encoder, Tick.Zero, BallState.AtRest, tooFew, full));
        Assert.Throws<ArgumentException>(() =>
            MatchCanonicalState.Write(encoder, Tick.Zero, BallState.AtRest, full, tooFew));
    }

    #endregion
}

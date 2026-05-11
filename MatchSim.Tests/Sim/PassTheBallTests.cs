using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// Phase-3 pass-the-ball acceptance tests. Pins the regression that
/// blueprint patch 8e2dc1b flagged: prior to this commit, MatchSim had
/// ball physics + player movement + BT runner that emits move commands,
/// but ZERO logic in the match loop that applied velocity to the ball
/// when a player reached possession. 22 players converged on a static
/// ball and the ball never moved. Tests + 60-tick pinned hash all green
/// while the actual match was unplayable.
///
/// <para>
/// These tests assert the ball MOVES across a longer playback window
/// (60 seconds / 3600 ticks) — too long for the prior pinned-hash
/// fixture (60 ticks / 1 second) to catch, which is why the regression
/// shipped.
/// </para>
/// </summary>
public sealed class PassTheBallTests
{
    private const string HomeArchetypeId = "direct-pressing";
    private const string AwayArchetypeId = "low-block-counter";
    private const ulong SmokeSeed = 0xDEADBEEFDEADBEEFUL;

    /// <summary>
    /// 60-second smoke playback — at least one player on the pressing
    /// side reaches the ball, gains possession, kicks it. Ball must
    /// move at least 30 metres accumulated across the run. Prior to
    /// the kick-logic fix this assertion fails with accumulated distance
    /// = 0 (ball never leaves origin).
    /// </summary>
    [Fact]
    public void PassTheBall_60SecondPlayback_BallMovesSubstantially()
    {
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        MatchSimulationState state = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, home, away);
        MatchSimulationConfig config = new(Seed.FromUInt64(SmokeSeed));

        Vector3Fixed lastPos = state.Ball.Position;
        Fixed accumulatedDistanceSq = Fixed.Zero;
        int kickEventsObserved = 0;

        const int totalTicks = 3600;
        for (int i = 0; i < totalTicks; i++)
        {
            MatchSimulationRunner.RunTicks(
                state, home, away,
                PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds,
                config,
                ticks: 1);

            Fixed distSqThisTick = Vector3Fixed.DistanceSquared(lastPos, state.Ball.Position);
            accumulatedDistanceSq = accumulatedDistanceSq + distSqThisTick;

            // Detect "ball velocity jumped" — a proxy for a kick firing.
            // A kick gives velocity ≥ ~14 m/s; ambient ball motion stays
            // well below this. Counting these gives a rough kick-event tally.
            if (state.Ball.Velocity.LengthSquared() > Fixed.FromInt(100))  // ≥10 m/s
            {
                // Only count once per kick-window: skip if we already counted
                // a high-velocity tick recently. For simplicity here we
                // accept the over-count; the floor on accumulated distance
                // is the load-bearing assertion.
                kickEventsObserved++;
            }

            lastPos = state.Ball.Position;
        }

        // The test pins a FLOOR not an EXACT match. Phase-3 pass logic is
        // deterministic so the exact accumulated distance reproduces, but
        // pinning the exact value would re-trigger every time the BT
        // coefficients tune — which is the wrong test contract here. The
        // floor of 30 metres is conservative: at PassSpeed=14 m/s, even a
        // single kick that travels ~5 ticks before another player intercepts
        // produces ~1.2m. Over 3600 ticks, accumulated travel of 30m means
        // at least ~25 kick events fired across the match, which is the
        // minimum "the ball actually moves" check.
        Fixed minDistanceSqFloor = Fixed.FromInt(30 * 30);  // 900 m²
        Assert.True(
            accumulatedDistanceSq.RawValue >= minDistanceSqFloor.RawValue,
            $"Ball accumulated travel was too low: {accumulatedDistanceSq} m² < {minDistanceSqFloor} m². " +
            "Pass-the-ball logic may have regressed; the ball is glued to its start position.");

        // Sanity: at least SOME high-velocity ticks were observed (kicks fired).
        Assert.True(
            kickEventsObserved > 0,
            "No high-velocity ball ticks observed in 3600-tick playback — kick emission may have regressed.");
    }

    /// <summary>
    /// Two runs of the same fixture produce byte-identical canonical state
    /// at intermediate checkpoints + at the end. Pins the determinism
    /// contract for the kick-emission heuristic — any future change that
    /// makes the pass-target selection non-deterministic trips this.
    /// </summary>
    [Fact]
    public void PassTheBall_TwoRuns_ByteIdenticalCanonicalState()
    {
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        MatchSimulationConfig config = new(Seed.FromUInt64(SmokeSeed));

        // Run A
        MatchSimulationState stateA = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, home, away);
        MatchSimulationRunner.RunTicks(
            stateA, home, away,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            config,
            ticks: 600);  // 10 seconds
        string hashA = MatchCanonicalState.ComputeHash(stateA);

        // Run B (fresh state, identical inputs)
        MatchSimulationState stateB = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, home, away);
        MatchSimulationRunner.RunTicks(
            stateB, home, away,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            config,
            ticks: 600);
        string hashB = MatchCanonicalState.ComputeHash(stateB);

        Assert.Equal(hashA, hashB);
    }

    /// <summary>
    /// Pinned canonical-state hash at 600 ticks (10 seconds). When this
    /// trips deliberately (a kick-heuristic change re-baselines), update
    /// the literal + preserve the prior value in the comment so the
    /// history-of-drift is traceable.
    /// </summary>
    [Fact]
    public void PassTheBall_600TickPlayback_HashMatchesPin()
    {
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        MatchSimulationConfig config = new(Seed.FromUInt64(SmokeSeed));

        MatchSimulationState state = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, home, away);
        MatchSimulationRunner.RunTicks(
            state, home, away,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            config,
            ticks: 600);

        string actual = MatchCanonicalState.ComputeHash(state);

        // History (newest first):
        //   sha256:9ef285ab87f9e49c99a09d61544a217dd6fec72f46a2e6a0d7e358b133b10cac —
        //     polish-pass Option 1 (2026-05-11 inter-player soft collision).
        //     Authorized hash drift per the C5-option1 task-spec. 60-tick smoke
        //     hash + 60-tick primed-fixture hash both UNCHANGED (no overlap
        //     within those windows).
        //   sha256:c5ab9e5265724dc79ef5bf038123fbaadb686c3e9d35e79f682ee16882fed1d2 —
        //     pass-the-ball v1 (2026-05-11 first kick logic).
        const string expected = "sha256:9ef285ab87f9e49c99a09d61544a217dd6fec72f46a2e6a0d7e358b133b10cac";
        Assert.Equal(expected, actual);
    }
}

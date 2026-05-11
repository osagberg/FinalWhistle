using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// C4 acceptance tests for <see cref="MatchSimulationState.FromLowCutbackPrimedFixture"/>.
/// Pins that the pre-arranged initial state actually produces a LowCutback
/// signature emission within the first couple of ticks — Slice-7 visual
/// surfaces (SelectionRing / MotionLineEmitter / pressure tint / Tension
/// cadence) all depend on signatures firing, and the existing
/// 0xdeadbeefdeadbeef smoke fixture never satisfies the trigger gates
/// (C1 audit bb136866 confirmed SignatureRecipes.Count=0 over 70s of
/// natural play). This fixture is the natural-play L2 gate that Slice 7
/// retroactively gets validated against.
/// </summary>
public sealed class LowCutbackPrimedFixtureTests
{
    private const string HomeArchetypeId = "direct-pressing";
    private const string AwayArchetypeId = "low-block-counter";
    private const ulong PrimedSeed = 0xFEEDBEEFCAFEFADEUL;

    private static IdentityPacket[] LoadPackets(string archetype)
    {
        IdentityPacket[] packets = new IdentityPacket[IdentityPackets.PlayersPerArchetype];
        for (byte jersey = 1; jersey <= IdentityPackets.PlayersPerArchetype; jersey++)
        {
            packets[jersey - 1] = IdentityPackets.Load(archetype, jersey);
        }
        return packets;
    }

    [Fact]
    public void FromLowCutbackPrimedFixture_FiresSignatureWithinFirstTicks()
    {
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        IdentityPacket[] homePackets = LoadPackets(HomeArchetypeId);
        IdentityPacket[] awayPackets = LoadPackets(AwayArchetypeId);

        MatchSimulationState state = MatchSimulationState.FromLowCutbackPrimedFixture(
            Tick.Zero, home, away);
        MatchSimulationConfig config = new(Seed.FromUInt64(PrimedSeed));

        // 2-tick playback is enough — the signature gates are pre-satisfied
        // at construction so the emission fires on tick 0 or tick 1.
        MatchSimulationRunner.RunTicks(
            state, home, away,
            homePackets, awayPackets,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            config,
            SignatureConfig.Phase3Defaults,
            ticks: 2);

        Assert.True(state.SignatureRecipes.Count >= 1,
            $"Primed fixture must fire >=1 signature within 2 ticks; got {state.SignatureRecipes.Count}. " +
            "Either the trigger gates have drifted, the fixture's player position no longer satisfies them, " +
            "or the IdentityPacket affinities for jersey 6 changed.");

        // The first emitted recipe MUST be LowCutback (jersey 6 / Pielke
        // is Winger with LowCutback affinity in direct-pressing fixtures).
        SignatureExecution first = state.SignatureRecipes[0];
        Assert.True(first.KeyEventIndex < state.KeyEvents.Count,
            "Recipe's KeyEventIndex must be in range of state.KeyEvents.");
        KeyEvent kickKeyEvent = state.KeyEvents[first.KeyEventIndex];
        Assert.Equal(KeyEventKind.SignatureExecuted_LowCutback, kickKeyEvent.Kind);
        Assert.Equal(TeamSide.Home, kickKeyEvent.Side);
        Assert.Equal((byte)6, kickKeyEvent.JerseyNumber);
    }

    [Fact]
    public void FromLowCutbackPrimedFixture_DoesNotDriftPinnedSmokeHash()
    {
        // The primed fixture must be ENTIRELY SEPARATE from the smoke fixture
        // — no shared state, no shared seed, no canonical-hash collision. This
        // test re-runs the existing 60-tick smoke fixture to confirm its
        // pinned hash is byte-identical AFTER the primed-fixture code path
        // is reachable. Defends against accidental cross-contamination
        // (e.g. if FromLowCutbackPrimedFixture were to mutate
        // PlayerKinematics.Phase3Defaults or similar shared state).
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        MatchSimulationConfig config = new(Seed.FromUInt64(0xDEADBEEFDEADBEEFUL));

        MatchSimulationState smokeState = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, home, away);
        MatchSimulationRunner.RunTicks(
            smokeState, home, away,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            config,
            ticks: 60);

        const string SmokeSeed60TickHash = "sha256:7e851976f6a5eea467797e90400ca030c6ab955e21c2f92466cffa00c880f50e";
        string actual = MatchCanonicalState.ComputeHash(smokeState);
        Assert.Equal(SmokeSeed60TickHash, actual);
    }

    [Fact]
    public void FromLowCutbackPrimedFixture_60TickPlayback_HashMatchesPin()
    {
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        IdentityPacket[] homePackets = LoadPackets(HomeArchetypeId);
        IdentityPacket[] awayPackets = LoadPackets(AwayArchetypeId);
        MatchSimulationConfig config = new(Seed.FromUInt64(PrimedSeed));

        MatchSimulationState state = MatchSimulationState.FromLowCutbackPrimedFixture(
            Tick.Zero, home, away);
        MatchSimulationRunner.RunTicks(
            state, home, away, homePackets, awayPackets,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            config, SignatureConfig.Phase3Defaults,
            ticks: 60);

        string actual = MatchCanonicalState.ComputeHash(state);
        // History (newest first):
        //   sha256:34a31b3b9d2426b2639140bec6696a1787e09aa6ce4aed7eaaaa298202e4fb94 —
        //     polish-pass Option 2 (2026-05-11 goalkeeper specialization).
        //     Authorized drift per option2 task-spec — primed fixture starts
        //     with AWAY GK in possession at the cutback zone, so Option-2's
        //     "GK doesn't sprint upfield + emits long-ball immediately"
        //     changes tick-0 trajectory. The signature still fires; the
        //     downstream ball positions differ.
        //   sha256:2f5cc063374b43cfd822043401add3ebddc2e174a1bb0a440e4d10b0e33a4ef6 —
        //     C4 primed-for-LowCutback v1 (2026-05-11). 60-tick playback from
        //     FromLowCutbackPrimedFixture starting position; LowCutback signature
        //     fires at tick 0 → ball kicked toward goal → goal scored by ~tick 50;
        //     ball respawns. Canonical state at tick 60 is the post-respawn pose.
        const string expected = "sha256:34a31b3b9d2426b2639140bec6696a1787e09aa6ce4aed7eaaaa298202e4fb94";
        Assert.Equal(expected, actual);
    }

    [Fact]
    public void FromLowCutbackPrimedFixture_TwoRuns_ByteIdenticalCanonicalState()
    {
        BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(HomeArchetypeId);
        BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(AwayArchetypeId);
        IdentityPacket[] homePackets = LoadPackets(HomeArchetypeId);
        IdentityPacket[] awayPackets = LoadPackets(AwayArchetypeId);
        MatchSimulationConfig config = new(Seed.FromUInt64(PrimedSeed));

        MatchSimulationState a = MatchSimulationState.FromLowCutbackPrimedFixture(Tick.Zero, home, away);
        MatchSimulationRunner.RunTicks(
            a, home, away, homePackets, awayPackets,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            config, SignatureConfig.Phase3Defaults,
            ticks: 60);

        MatchSimulationState b = MatchSimulationState.FromLowCutbackPrimedFixture(Tick.Zero, home, away);
        MatchSimulationRunner.RunTicks(
            b, home, away, homePackets, awayPackets,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            config, SignatureConfig.Phase3Defaults,
            ticks: 60);

        Assert.Equal(MatchCanonicalState.ComputeHash(a), MatchCanonicalState.ComputeHash(b));
    }
}

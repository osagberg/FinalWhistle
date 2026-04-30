using System;
using System.Collections.Generic;
using System.IO;
using System.Text.Json;
using FinalWhistle.MatchSim.Content;
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
/// on Win/Mac/Linux runners. Any drift = a real bug, not a tolerance issue.
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

    /// <summary>
    /// Pinned smoke-seed reference value. Re-baselined 2026-04-30 for the v1
    /// canonical-state schema bump that added score (2 bytes) + OutOfPlay
    /// (1 byte) + KeyEvent count (4 bytes) per SPEC 2026-04-28 PitchRules
    /// decisions-log entry. v0 hash was
    /// <c>sha256:299cdb0cbbc9606e141db278a14585780d0e3b5dbfb8815f634af89be7f6118a</c>
    /// (computed macOS 2026-04-27); v0 is no longer reachable from production
    /// code because <c>MatchCanonicalState.Write</c> now emits the v1 layout.
    /// CI matrix on Win/Mac/Linux verifies the v1 hash holds across platforms.
    /// </summary>
    private const string SmokeSeed60TickHash = "sha256:7e851976f6a5eea467797e90400ca030c6ab955e21c2f92466cffa00c880f50e";

    #region Composition helper — runs the full BT + Player + Ball stack

    /// <summary>
    /// Build the canonical Phase-3 smoke fixture initial state: 22 players
    /// at their archetype formation positions, ball at centre, tick 0.
    /// </summary>
    private static MatchSimulationState BuildSmokeFixture(
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype)
    {
        return MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, homeArchetype, awayArchetype);
    }

    #endregion

    #region Pinned-hash bedrock — cross-platform parity

    [Fact]
    public void Replay_SmokeCorpusFixture_AgreesWithPinnedInCodeHash()
    {
        // Phase-3 enforcement-skeleton-rollout per SPEC 2026-04-28 + Codex
        // round-4 follow-up plan (commit #4 of 6, 2026-04-30). The corpus
        // fixture at MatchSim.Tests/fixtures/replay-corpus/0xdeadbeefdeadbeef.json
        // is the on-disk artifact form of the same smoke gate exercised by
        // Match_SmokeFixture60Ticks_ProducesPinnedCanonicalStateHash above.
        //
        // This test prevents drift between the fixture's expected.final_canonical_state_hash
        // string and the in-code SmokeSeed60TickHash constant. If the hash
        // is intentionally re-baselined (e.g., a future schema bump), both
        // must move together — the fixture file is the corpus contract,
        // the constant is the test bedrock; they MUST agree.
        //
        // The actual `fw replay <seed> --compare-corpus` command shells out
        // to `dotnet test --filter` against this test class; this Fact is
        // the load-bearing assertion that satisfies the Tier-A CI contract
        // documented in design/specs/golden-replay-corpus.md §Tier A.
        string fixturePath = LocateCorpusFixture("0xdeadbeefdeadbeef.json");
        Assert.True(File.Exists(fixturePath),
            $"Corpus fixture missing: {fixturePath}. Phase-3 enforcement-skeleton " +
            $"requires fixtures/replay-corpus/0xdeadbeefdeadbeef.json per SPEC " +
            $"2026-04-28 enforcement-skeleton-rollout entry.");

        using JsonDocument doc = JsonDocument.Parse(File.ReadAllText(fixturePath));
        JsonElement root = doc.RootElement;

        // Schema version must match what this test was authored against;
        // a bump to v2 means the fixture format changed and this test
        // needs review.
        Assert.Equal(1, root.GetProperty("corpus_schema_version").GetInt32());

        // Match seed string must be lowercase hex with 0x prefix per spec.
        Assert.Equal("0xdeadbeefdeadbeef", root.GetProperty("match_seed").GetString());

        // Sim length must match the in-code 60-tick smoke fixture.
        Assert.Equal(60, root.GetProperty("sim_length_ticks").GetInt32());

        // The load-bearing claim: fixture's final canonical-state hash
        // must equal the constant pinned in this test class.
        JsonElement expected = root.GetProperty("expected");
        string fixtureHash = expected.GetProperty("final_canonical_state_hash").GetString()
            ?? throw new InvalidOperationException("final_canonical_state_hash missing or null");
        Assert.True(fixtureHash == SmokeSeed60TickHash,
            $"Corpus fixture hash drift.\n" +
            $"  In-code constant: {SmokeSeed60TickHash}\n" +
            $"  Fixture file:     {fixtureHash}\n" +
            $"  Path: {fixturePath}\n" +
            $"If this is intentional (schema bump), update both in the same commit.");

        // Phase-3 smoke at 60 ticks at centre never scores — final score
        // is [0,0]. Belt-and-suspenders: prevents a fixture author from
        // typo-ing a non-zero score.
        JsonElement finalScore = expected.GetProperty("final_score");
        Assert.Equal(2, finalScore.GetArrayLength());
        Assert.Equal(0, finalScore[0].GetInt32());
        Assert.Equal(0, finalScore[1].GetInt32());

        // Phase-3 smoke produces zero KeyEvents — pre-out / post-out
        // short-circuits prevent any restart emission with the ball at
        // centre over 60 ticks. Pinned at empty array.
        Assert.Equal(0, expected.GetProperty("key_event_hashes").GetArrayLength());
    }

    /// <summary>
    /// Locate a corpus fixture file. xUnit runs from the test bin directory;
    /// the fixtures live alongside source files at the project root. Walk
    /// up until we find <c>fixtures/replay-corpus/</c>.
    /// </summary>
    private static string LocateCorpusFixture(string fileName)
    {
        string current = AppContext.BaseDirectory;
        for (int depth = 0; depth < 10; depth++)
        {
            string candidate = Path.Combine(current, "fixtures", "replay-corpus", fileName);
            if (File.Exists(candidate))
            {
                return candidate;
            }
            DirectoryInfo? parent = Directory.GetParent(current);
            if (parent is null)
            {
                break;
            }
            current = parent.FullName;
        }
        // Last-ditch: the canonical project-relative path. Returned even if
        // missing so the assertion message points to where authors should
        // put the file.
        return Path.Combine(
            AppContext.BaseDirectory, "..", "..", "..", "fixtures",
            "replay-corpus", fileName);
    }

    [Fact]
    public void Match_SmokeFixture60Ticks_ProducesPinnedCanonicalStateHash()
    {
        // The cross-platform-determinism gate. This SHA256 was computed on
        // macOS 2026-04-27. The Tier-A CI matrix runs this same test on
        // Win/Mac/Linux runners; all three must produce the identical hash.
        // Any disagreement = a determinism leak in BT / Player / Ball / hashing
        // pipeline. Pixel rendering is NOT exercised — this is purely the
        // canonical sim-state.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        MatchSimulationState state = BuildSmokeFixture(direct, lowBlock);

        MatchSimulationRunner.RunTicks(state, direct, lowBlock,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            MatchSimulationConfig.Default,
            ticks: 60);

        string hash = MatchCanonicalState.ComputeHash(state);

        // Use Assert.True with full message so xUnit doesn't truncate the
        // failure output (Assert.Equal truncates string mismatches at ~38 chars).
        Assert.True(hash == SmokeSeed60TickHash,
            $"Canonical-state hash mismatch.\nExpected: {SmokeSeed60TickHash}\nActual:   {hash}");
    }

    [Fact]
    public void Match_SmokeFixture60TicksWithSignaturePackets_ProducesIdenticalPinnedHash()
    {
        // Phase-3 semantic-slice signature-dispatch regression check
        // (SPEC line 143 / commit landing the 3 active signatures).
        // The signature-aware RunTicks overload accepts IdentityPacket
        // arrays and inserts SignatureRules.Step in the per-tick order
        // immediately after MatchRules.Step. The architect blueprint's
        // spatial analysis proved the smoke fixture (ball at centre,
        // formation positions, 60 ticks, ball velocity zero) cannot
        // satisfy any of the three Phase-3 trigger conditions:
        //
        //   #20 Low cutback — fails byline-proximity (no player within
        //   3m of GoalLineX over 60 ticks from formation positions
        //   with ball at centre).
        //   #22 Blind-side run — fails penalty-area-depth (no striker
        //   in attacking box).
        //   #13 Diagonal switch — fails ball-velocity gate (ball at
        //   rest stays at rest under drag).
        //
        // Therefore: zero KeyEvents added by SignatureRules → canonical
        // bytes identical → pinned hash unchanged. This test pins that
        // claim. If the architect's spatial analysis ever fails (e.g.,
        // a future BT change moves a striker into the box during the
        // smoke ticks), this test fails LOUDLY rather than the pinned
        // hash silently re-baselining.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        MatchSimulationState state = BuildSmokeFixture(direct, lowBlock);

        IdentityPacket[] homePackets = LoadArchetypePackets("direct-pressing");
        IdentityPacket[] awayPackets = LoadArchetypePackets("low-block-counter");

        MatchSimulationRunner.RunTicks(state, direct, lowBlock,
            homePackets, awayPackets,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            MatchSimulationConfig.Default,
            SignatureConfig.Phase3Defaults,
            ticks: 60);

        string hash = MatchCanonicalState.ComputeHash(state);
        Assert.True(hash == SmokeSeed60TickHash,
            $"Smoke-fixture WITH IdentityPackets produced a different hash.\n" +
            $"Expected: {SmokeSeed60TickHash}\n" +
            $"Actual:   {hash}\n" +
            $"This means SignatureRules.Step fired during the smoke fixture, " +
            $"violating the architect's spatial-analysis claim. Investigate " +
            $"which signature fired + tighten the trigger threshold or " +
            $"intentionally re-baseline the hash with a documented reason.");

        // Belt-and-suspenders: explicitly assert SignatureRules emitted
        // zero events. If a signature DID fire but happened to produce
        // canonical bytes that hashed to the same value (astronomically
        // unlikely but guard against), this surfaces the issue directly.
        int signatureExecutionEvents = 0;
        foreach (var keyEvent in state.KeyEvents)
        {
            if (keyEvent.Kind >= KeyEventKind.SignatureExecuted_LowCutback
                && keyEvent.Kind <= KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch)
            {
                signatureExecutionEvents++;
            }
        }
        Assert.Equal(0, signatureExecutionEvents);
        Assert.Empty(state.SignatureRecipes);
    }

    [Fact]
    public void Match_ChunkedRunTicksWithSignatures_ProducesIdenticalHashAndEventStream()
    {
        // Closes Codex round-9 P1: one RunTicks(ticks=N) MUST produce the
        // same canonical state as N repeated RunTicks(ticks=1) calls when
        // signatures are enabled. Before the fix, the runner allocated a
        // fresh SignatureCooldownState on every call — so chunked callers
        // (viewer/replay loops driving the runner one tick at a time)
        // could re-fire signatures past their per-match cap because the
        // cooldown state was wiped between calls. Now the cooldown lives
        // on MatchSimulationState (allocated once at construction); both
        // call patterns share the same cooldown instance and produce
        // byte-identical output.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        IdentityPacket[] homePackets = LoadArchetypePackets("direct-pressing");
        IdentityPacket[] awayPackets = LoadArchetypePackets("low-block-counter");

        // Run A: one call of 60 ticks.
        MatchSimulationState a = BuildSmokeFixture(direct, lowBlock);
        MatchSimulationRunner.RunTicks(a, direct, lowBlock,
            homePackets, awayPackets,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            MatchSimulationConfig.Default,
            SignatureConfig.Phase3Defaults,
            ticks: 60);

        // Run B: 60 calls of 1 tick each.
        MatchSimulationState b = BuildSmokeFixture(direct, lowBlock);
        for (int t = 0; t < 60; t++)
        {
            MatchSimulationRunner.RunTicks(b, direct, lowBlock,
                homePackets, awayPackets,
                PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds,
                MatchSimulationConfig.Default,
                SignatureConfig.Phase3Defaults,
                ticks: 1);
        }

        Assert.Equal(MatchCanonicalState.ComputeHash(a), MatchCanonicalState.ComputeHash(b));
        Assert.Equal(a.KeyEvents.Count, b.KeyEvents.Count);
        Assert.Equal(a.SignatureRecipes.Count, b.SignatureRecipes.Count);
        // Belt-and-suspenders: the chunked path also matches the pinned
        // smoke-fixture hash.
        Assert.Equal(SmokeSeed60TickHash, MatchCanonicalState.ComputeHash(b));
    }

    [Fact]
    public void Match_ChunkedRunTicks_LowCutbackFiresOnceAcrossChunkedAndSingleCallRuns()
    {
        // Codex round-10 P2: prior chunked-tick regressions (smoke fixture
        // hash equality + pre-saturated cooldown persistence) would NOT
        // catch the original bug coming back if RunTicks reverted to a
        // fresh local SignatureCooldownState while leaving the state
        // property untouched — smoke fires zero signatures, and saturation
        // pre-load doesn't depend on the runner consuming
        // state.SignatureCooldown. This test forces the runner-driven
        // path: build a state where #20 LowCutback fires on tick 0 AND
        // would re-fire on tick 1 if the cooldown were lost between
        // RunTicks calls (the 180-tick cooldown window blocks any second
        // fire when persistence holds — but only if the runner actually
        // reads the persisted state).
        //
        // Construction:
        //   - Smoke-fixture formation positions for 21 of 22 players.
        //   - Home jersey 6 (direct-pressing RM, role=Winger, real fixture
        //     packet carries #20 LowCutback affinity per
        //     `MatchSim/Content/identity-packets/direct-pressing/06.json`)
        //     overridden to (50, 0, 22) with lateral velocity (0, 0, 3).
        //   - Ball overridden to (50, 0, 22) with zero velocity.
        //   - Real loaded direct-pressing + low-block-counter packets.
        //
        // Tick 0 trigger evaluation (post-rules, after BT+actuator+ball):
        //   - Home jersey 6 is sole carrier (ball at player position; all
        //     other players at formation positions, none within 0.5m).
        //   - Direct-pressing BT enters Build-up; sets target velocity
        //     toward opponent goal at MaxSpeed × 0.95. Actuator clamps
        //     velocity-delta to MaxAcceleration·dt = 0.1m/s, so velocity
        //     after tick 0 ≈ (0.008, 0, 2.9) — lateral component still
        //     above MinLateralSpeed=1.
        //   - Position drifts to ~(50.0001, 0, 22.05) — distance to
        //     byline 2.5m < 3m, |Z|=22.05 > 20m, IsCarrier ✓.
        //   - All #20 conditions met → fire. Cooldown records tick 0.
        //
        // Tick 1 trigger evaluation:
        //   - All conditions still hold (position has drifted ~0.05m,
        //     lateral velocity still ~2.8m/s).
        //   - WITH persistent cooldown: 1 - 0 = 1 < 180 → BLOCKED.
        //     Total fires: 1.
        //   - WITHOUT persistent (the bug): fresh cooldown's
        //     lastFiredTick = long.MinValue. 1 - long.MinValue ≫ 180
        //     → CanFire passes → re-fires. Total fires: 2.
        //
        // The test asserts both chunked + single-call paths produce
        // exactly 1 fire (and identical canonical hash). A regression
        // that re-allocates the cooldown per chunk fails LOUDLY because
        // the chunked path fires twice while single-call fires once.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        IdentityPacket[] homePackets = LoadArchetypePackets("direct-pressing");
        IdentityPacket[] awayPackets = LoadArchetypePackets("low-block-counter");

        // Run A: one call of 2 ticks.
        MatchSimulationState single = BuildLowCutbackTriggerFixture(direct, lowBlock);
        MatchSimulationRunner.RunTicks(single, direct, lowBlock,
            homePackets, awayPackets,
            PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds,
            MatchSimulationConfig.Default,
            SignatureConfig.Phase3Defaults,
            ticks: 2);

        int singleFireCount = CountSignatureExecutions(
            single, KeyEventKind.SignatureExecuted_LowCutback);
        // Test-setup invariant: the trigger fixture MUST fire #20 at
        // least once within 2 ticks. If this is 0, either the BT path
        // moved the player out of the trigger zone faster than expected
        // or the affinity wiring is wrong — fail loudly with a
        // diagnostic instead of letting the comparison-equal-zero
        // assertion below pass vacuously.
        Assert.True(singleFireCount > 0,
            $"Trigger-fixture invariant violated: expected >=1 LowCutback fire " +
            $"in 2-tick run; got {singleFireCount}. Check fixture construction " +
            $"or signature-affinity mapping for direct-pressing jersey 6.");
        // Cooldown=180 ticks blocks any second fire within a 2-tick run.
        Assert.Equal(1, singleFireCount);

        // Run B: 2 calls of 1 tick each, on a fresh identical state.
        MatchSimulationState chunked = BuildLowCutbackTriggerFixture(direct, lowBlock);
        for (int t = 0; t < 2; t++)
        {
            MatchSimulationRunner.RunTicks(chunked, direct, lowBlock,
                homePackets, awayPackets,
                PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds,
                MatchSimulationConfig.Default,
                SignatureConfig.Phase3Defaults,
                ticks: 1);
        }

        int chunkedFireCount = CountSignatureExecutions(
            chunked, KeyEventKind.SignatureExecuted_LowCutback);

        // The bug-detector: without persistent cooldown, chunkedFireCount
        // would be 2 (both ticks fire, fresh cooldown each call). With
        // persistent cooldown, chunkedFireCount == singleFireCount == 1.
        Assert.Equal(singleFireCount, chunkedFireCount);
        Assert.Equal(1, chunkedFireCount);

        // Belt + suspenders: full canonical-state hash equality (the
        // chunked path produces byte-identical state to the single call).
        Assert.Equal(MatchCanonicalState.ComputeHash(single),
                     MatchCanonicalState.ComputeHash(chunked));
        // Recipe streams also agree (1:1 mirror of signature KeyEvents
        // for #20).
        Assert.Equal(single.SignatureRecipes.Count, chunked.SignatureRecipes.Count);
    }

    /// <summary>
    /// Builds a state where home jersey 6 (direct-pressing RM, real
    /// fixture packet has Winger role + LowCutback affinity) is parked
    /// at the attacking byline with the ball coincident, lateral velocity
    /// non-trivial — satisfying every spatial/kinematic condition for #20
    /// LowCutback to fire on tick 0 (post-BT+actuator+ball drift).
    /// </summary>
    private static MatchSimulationState BuildLowCutbackTriggerFixture(
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype)
    {
        // Start from the smoke-fixture formation, then override home
        // jersey 6 + ball. The other 21 players stay at formation
        // positions so none are accidentally closer to the override
        // ball position than home jersey 6 (BT possession check is
        // nearest-player-to-ball; want home jersey 6 to be sole carrier).
        MatchSimulationState state = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, homeArchetype, awayArchetype);

        Vector3Fixed bylinePos = new(Fixed.FromInt(50), Fixed.Zero, Fixed.FromInt(22));
        state.HomeTeam[5] = new PlayerState(
            position: bylinePos,
            velocity: new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.FromInt(3)),
            jerseyNumber: 6,
            side: TeamSide.Home);
        state.Ball = new BallState(bylinePos, Vector3Fixed.Zero, Vector3Fixed.Zero);

        return state;
    }

    private static int CountSignatureExecutions(MatchSimulationState state, KeyEventKind kind)
    {
        int count = 0;
        for (int i = 0; i < state.KeyEvents.Count; i++)
        {
            if (state.KeyEvents[i].Kind == kind)
            {
                count++;
            }
        }
        return count;
    }

    [Fact]
    public void Match_ChunkedRunTicks_PersistsSignatureCooldownSaturationAcrossCalls()
    {
        // Closes Codex round-9 P1 (behavior angle): pre-saturate the
        // per-match cooldown so #20 LowCutback has hit its 3-fire cap for
        // home jersey 6. Drive 60 single-tick RunTicks chunks. After the
        // chunks, CanFire must STILL return false — saturation survives.
        // Before the fix, each RunTicks call wiped the cooldown to a fresh
        // instance, so this assertion would flip to true (the saturation
        // was lost) and signatures could re-fire past the per-match cap.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        MatchSimulationState state = BuildSmokeFixture(direct, lowBlock);

        int homeJersey6Index = SignatureCooldownState.PlayerIndex(TeamSide.Home, 6);
        state.SignatureCooldown.RecordFire(SignatureKind.LowCutback, homeJersey6Index, currentTick: 0L);
        state.SignatureCooldown.RecordFire(SignatureKind.LowCutback, homeJersey6Index, currentTick: 1L);
        state.SignatureCooldown.RecordFire(SignatureKind.LowCutback, homeJersey6Index, currentTick: 2L);

        Assert.False(state.SignatureCooldown.CanFire(
            SignatureKind.LowCutback, homeJersey6Index,
            currentTick: 1_000_000L, cooldownTicks: 1, maxFiresPerMatch: 3));

        IdentityPacket[] homePackets = LoadArchetypePackets("direct-pressing");
        IdentityPacket[] awayPackets = LoadArchetypePackets("low-block-counter");
        SignatureCooldownState cooldownBefore = state.SignatureCooldown;

        for (int t = 0; t < 60; t++)
        {
            MatchSimulationRunner.RunTicks(state, direct, lowBlock,
                homePackets, awayPackets,
                PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds,
                MatchSimulationConfig.Default,
                SignatureConfig.Phase3Defaults,
                ticks: 1);
        }

        // Reference equality: runner never swapped the cooldown instance.
        Assert.Same(cooldownBefore, state.SignatureCooldown);

        // Behavioral equality: cap-saturation survives 60 chunked calls.
        Assert.False(state.SignatureCooldown.CanFire(
            SignatureKind.LowCutback, homeJersey6Index,
            currentTick: 1_000_000L, cooldownTicks: 1, maxFiresPerMatch: 3));
    }

    private static IdentityPacket[] LoadArchetypePackets(string archetype)
    {
        IdentityPacket[] packets = new IdentityPacket[IdentityPackets.PlayersPerArchetype];
        for (byte jersey = 1; jersey <= IdentityPackets.PlayersPerArchetype; jersey++)
        {
            packets[jersey - 1] = IdentityPackets.Load(archetype, jersey);
        }
        return packets;
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
            MatchSimulationRunner.RunTicks(state, direct, lowBlock,
                PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds,
                MatchSimulationConfig.Default,
                ticks: 60);
            distinctHashes.Add(MatchCanonicalState.ComputeHash(state));
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
            MatchSimulationRunner.RunTicks(a, direct, lowBlock, PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds, MatchSimulationConfig.Default, ticks: 10);
            MatchSimulationRunner.RunTicks(b, direct, lowBlock, PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds, MatchSimulationConfig.Default, ticks: 10);

            runA[s] = MatchCanonicalState.ComputeHash(a);
            runB[s] = MatchCanonicalState.ComputeHash(b);

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

        MatchSimulationRunner.RunTicks(a, direct, lowBlock, PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds, MatchSimulationConfig.Default, ticks: 60);
        MatchSimulationRunner.RunTicks(b, lowBlock, direct, PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds, MatchSimulationConfig.Default, ticks: 60);

        string hashA = MatchCanonicalState.ComputeHash(a);
        string hashB = MatchCanonicalState.ComputeHash(b);

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

        MatchSimulationRunner.RunTicks(a, direct, lowBlock, PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds, MatchSimulationConfig.Default, ticks: 60);
        MatchSimulationRunner.RunTicks(b, direct, lowBlock, PlayerKinematics.Phase3Defaults,
            BallPhysicsCoefficients.Phase3Seeds, MatchSimulationConfig.Default, ticks: 60);

        string hashA = MatchCanonicalState.ComputeHash(a);
        string hashB = MatchCanonicalState.ComputeHash(b);

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
        // 4 (away count) + 11×50 (away players) + 1 (HomeScore) + 1 (AwayScore)
        // + 1 (OutOfPlay) + 4 (KeyEvent count) = 1195 bytes (the v1 base width
        // post-PitchRules schema bump). Smoke fixture has 0 KeyEvents so total
        // = base.
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");
        MatchSimulationState state = BuildSmokeFixture(direct, lowBlock);

        CanonicalEncoder encoder = new();
        MatchCanonicalState.Write(encoder, state);

        Assert.Equal(MatchCanonicalState.EncodedBaseByteCount, encoder.WrittenCount);
        Assert.Equal(1195, MatchCanonicalState.EncodedBaseByteCount);
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

    [Fact]
    public void MatchSimulationState_WrongTeamLength_Throws()
    {
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
            new MatchSimulationState(Tick.Zero, BallState.AtRest, tooFew, full));
        Assert.Throws<ArgumentException>(() =>
            new MatchSimulationState(Tick.Zero, BallState.AtRest, full, tooFew));
    }

    [Fact]
    public void FromArchetypeFormations_OrdersPlayersByRosterSlot()
    {
        FormationSlot[] shuffled = new FormationSlot[MatchCanonicalState.PlayersPerTeam];
        shuffled[0] = new FormationSlot(2, "RB", V3(20, 0, 0));
        shuffled[1] = new FormationSlot(1, "GK", V3(10, 0, 0));
        for (byte rosterSlot = 3; rosterSlot <= MatchCanonicalState.PlayersPerTeam; rosterSlot++)
        {
            shuffled[rosterSlot - 1] = new FormationSlot(
                rosterSlot,
                "T",
                new Vector3Fixed(F(rosterSlot), Fixed.Zero, Fixed.Zero));
        }

        BehaviorTreeArchetype archetype = new(
            "shuffled",
            "valid but not list-sorted",
            shuffled,
            F(10),
            Fixed.One);

        MatchSimulationState state = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, archetype, archetype);

        Assert.Equal(V3(10, 0, 0), state.HomeTeam[0].Position);
        Assert.Equal(V3(20, 0, 0), state.HomeTeam[1].Position);
        Assert.Equal(V3(-10, 0, 0), state.AwayTeam[0].Position);
        Assert.Equal(V3(-20, 0, 0), state.AwayTeam[1].Position);
    }

    [Fact]
    public void MatchSimulationRunner_NegativeTicks_Throws()
    {
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");

        MatchSimulationState state = MatchSimulationState.FromArchetypeFormations(
            Tick.Zero, BallState.AtRest, direct, lowBlock);

        Assert.Throws<ArgumentOutOfRangeException>(() =>
            MatchSimulationRunner.RunTicks(state, direct, lowBlock,
                PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds,
                MatchSimulationConfig.Default,
                ticks: -1));
    }

    #endregion
}

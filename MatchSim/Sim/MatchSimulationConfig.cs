namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Per-match deterministic-input config carried by
/// <c>MatchSimulationRunner</c>. Closes Codex audit P2-03 (seed-input
/// refactor) per SPEC 2026-04-28 PitchRules decisions-log entry: the runner
/// accepts a <see cref="Seed"/> as input even though Phase-3 deterministic
/// code does not consume it yet (no stochastic events).
///
/// <para>
/// <strong>Why pass through a Seed Phase-3 doesn't read?</strong> Two
/// reasons:
/// </para>
/// <list type="number">
///   <item><description>The golden-replay-corpus spec's smoke-seed
///       (<c>0xdeadbeefdeadbeef</c>) is a real fixture input from Phase-3
///       Week-3+ once <c>fw replay &lt;seed&gt;</c> ships per the 2026-04-28
///       enforcement-skeleton-rollout decision. Wiring the seed through the
///       runner now means the corpus-replay command threads cleanly when it
///       lands.</description></item>
///   <item><description>Phase-4+ stochastic events (foul outcomes, deflection
///       angles, breakthrough trigger probabilities) will derive their RNG
///       streams via <see cref="Seed.Derive"/> from this match seed + tick +
///       per-event ID. Plumbing the seed through the runner from Phase-3
///       avoids a future refactor that touches every test fixture.</description></item>
/// </list>
///
/// <para>
/// <strong>Not part of canonical state.</strong> The match seed is fixture
/// input, not sim state — <c>MatchCanonicalState.Write</c> does NOT
/// encode it. Stochastic events landing Phase 4+ will enter canonical state
/// through emitted events (KeyEvents / future EventClass records), not
/// through the seed itself. This preserves the 2026-04-28 decisions-log
/// invariant: same source state ⇒ same canonical hash regardless of seed
/// (until stochastic code reads it).
/// </para>
/// </summary>
public readonly struct MatchSimulationConfig
{
    /// <summary>
    /// Match-level deterministic seed. Phase-3 sim does not consume it;
    /// Phase-4+ stochastic events derive per-event streams via
    /// <see cref="Seed.Derive"/>.
    /// </summary>
    public readonly Seed MatchSeed;

    public MatchSimulationConfig(Seed matchSeed)
    {
        MatchSeed = matchSeed;
    }

    /// <summary>
    /// Default config with <see cref="Seed.Zero"/>. Convenient for Phase-3
    /// fixtures + tests that don't care about seeds yet (Phase-3 sim
    /// doesn't read the seed).
    /// </summary>
    public static MatchSimulationConfig Default => new(Seed.Zero);
}

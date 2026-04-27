using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Canonical-state encoder for a single MatchSim snapshot. Writes (Tick +
/// Ball + 22 PlayerStates) in locked order to a <see cref="CanonicalEncoder"/>;
/// composes the existing <see cref="Tick"/>, <see cref="BallState.WriteCanonical"/>,
/// and <see cref="PlayerState.WriteCanonical"/> primitives.
///
/// <para>
/// This is the bedrock of cross-platform determinism per
/// <c>design/specs/golden-replay-corpus.md §final_canonical_state_hash</c>.
/// Win/Mac/Linux byte-equality is the contract: same initial state + same
/// sequence of pure-deterministic steps must produce identical bytes (and
/// therefore identical SHA256) across all three platforms. The literal-pinned
/// SHA256 hashes in <c>MatchDeterminismTests</c> verify this at the unit-test
/// layer; the Tier-A CI matrix runs the same tests on each OS.
/// </para>
///
/// <para>
/// <strong>Encoding order (locked at v1):</strong>
/// </para>
/// <list type="number">
///   <item><description><see cref="Tick"/> — 8 bytes LE.</description></item>
///   <item><description><see cref="BallState"/> — 72 bytes (3× <see cref="Vector3Fixed"/>: Position / Velocity / Spin).</description></item>
///   <item><description>Home team count = 11 (4-byte LE int) — defensive header so adapter consumers see explicit per-side count.</description></item>
///   <item><description>Home team's 11 <see cref="PlayerState"/>s in roster order (50 bytes each = 550 bytes).</description></item>
///   <item><description>Away team count = 11 (4-byte LE int).</description></item>
///   <item><description>Away team's 11 <see cref="PlayerState"/>s in roster order (550 bytes).</description></item>
/// </list>
///
/// <para>
/// Total per snapshot: 8 + 72 + 4 + 550 + 4 + 550 = 1188 bytes. Adding any
/// field is a corpus-fixture-invalidating change; handle via SerializationContract
/// version bump.
/// </para>
///
/// <para>
/// <strong>Roster order is caller responsibility.</strong> This helper does
/// NOT sort the player arrays — per ADR-0008 §Determinism contract ordering
/// rules, the caller MUST present players in a stable order (typically the
/// formation roster index, NOT jersey number which can collide across teams).
/// Mid-match swaps are not a concern at Month-3 (no substitutions per
/// <c>match-engine.md §Q4</c>).
/// </para>
/// </summary>
public static class MatchCanonicalState
{
    /// <summary>Canonical Month-3 on-pitch player count per side.</summary>
    public const int PlayersPerTeam = 11;

    /// <summary>
    /// Canonical v1 snapshot width in bytes:
    /// Tick (8) + Ball (72) + two count headers (8) + 22 PlayerStates (22×50).
    /// </summary>
    public const int EncodedByteCount = 1188;

    /// <summary>
    /// Write the canonical state of a match snapshot to the encoder. Bytes
    /// are appended to the encoder's existing buffer; caller controls when
    /// to compute the hash + reset.
    /// </summary>
    public static void Write(
        CanonicalEncoder encoder,
        Tick currentTick,
        BallState ball,
        ReadOnlySpan<PlayerState> homeTeam,
        ReadOnlySpan<PlayerState> awayTeam)
    {
        if (encoder is null)
        {
            throw new ArgumentNullException(nameof(encoder));
        }
        if (homeTeam.Length != PlayersPerTeam)
        {
            throw new ArgumentException($"homeTeam must contain exactly {PlayersPerTeam} players; got {homeTeam.Length}.", nameof(homeTeam));
        }
        if (awayTeam.Length != PlayersPerTeam)
        {
            throw new ArgumentException($"awayTeam must contain exactly {PlayersPerTeam} players; got {awayTeam.Length}.", nameof(awayTeam));
        }

        // 1. Tick.
        encoder.WriteTick(currentTick);

        // 2. Ball.
        ball.WriteCanonical(encoder);

        // 3-4. Home team.
        encoder.WriteCount(PlayersPerTeam);
        for (int i = 0; i < PlayersPerTeam; i++)
        {
            homeTeam[i].WriteCanonical(encoder);
        }

        // 5-6. Away team.
        encoder.WriteCount(PlayersPerTeam);
        for (int i = 0; i < PlayersPerTeam; i++)
        {
            awayTeam[i].WriteCanonical(encoder);
        }
    }

    /// <summary>
    /// Write the canonical state of a production match simulation snapshot.
    /// </summary>
    public static void Write(CanonicalEncoder encoder, MatchSimulationState state)
    {
        if (state is null)
        {
            throw new ArgumentNullException(nameof(state));
        }

        Write(encoder, state.CurrentTick, state.Ball, state.HomeTeam, state.AwayTeam);
    }

    /// <summary>
    /// Convenience: compute the SHA256 hash of a match snapshot. Returns the
    /// canonical form <c>"sha256:&lt;lowercase-hex&gt;"</c> matching
    /// <c>golden-replay-corpus.md</c>.
    /// </summary>
    public static string ComputeHash(
        Tick currentTick,
        BallState ball,
        ReadOnlySpan<PlayerState> homeTeam,
        ReadOnlySpan<PlayerState> awayTeam)
    {
        CanonicalEncoder encoder = new(initialCapacity: EncodedByteCount);
        Write(encoder, currentTick, ball, homeTeam, awayTeam);
        return encoder.ComputeSha256Hex();
    }

    /// <summary>
    /// Convenience: compute the SHA256 hash of a production match simulation
    /// snapshot.
    /// </summary>
    public static string ComputeHash(MatchSimulationState state)
    {
        CanonicalEncoder encoder = new(initialCapacity: EncodedByteCount);
        Write(encoder, state);
        return encoder.ComputeSha256Hex();
    }
}

using System;
using System.Collections.Generic;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Canonical-state encoder for a single MatchSim snapshot. Writes (Tick +
/// Ball + 22 PlayerStates + score + OutOfPlay + KeyEvents) in locked order
/// to a <see cref="CanonicalEncoder"/>; composes the existing
/// <see cref="Tick"/>, <see cref="BallState.WriteCanonical"/>,
/// <see cref="PlayerState.WriteCanonical"/>, and
/// <see cref="KeyEvent.WriteCanonical"/> primitives.
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
/// <strong>Encoding order (locked at v1)</strong>:
/// </para>
/// <list type="number">
///   <item><description><see cref="Tick"/> — 8 bytes LE.</description></item>
///   <item><description><see cref="BallState"/> — 72 bytes (3× <see cref="Vector3Fixed"/>: Position / Velocity / Spin).</description></item>
///   <item><description>Home team count = 11 (4-byte LE int) — defensive header so adapter consumers see explicit per-side count.</description></item>
///   <item><description>Home team's 11 <see cref="PlayerState"/>s in roster order (50 bytes each = 550 bytes).</description></item>
///   <item><description>Away team count = 11 (4-byte LE int).</description></item>
///   <item><description>Away team's 11 <see cref="PlayerState"/>s in roster order (550 bytes).</description></item>
///   <item><description>Home score (1 byte).</description></item>
///   <item><description>Away score (1 byte).</description></item>
///   <item><description><see cref="OutOfPlay"/> flag (1 byte).</description></item>
///   <item><description>KeyEvent count (4-byte LE int).</description></item>
///   <item><description>KeyEvents in append order (35 bytes each).</description></item>
/// </list>
///
/// <para>
/// <strong>Base width</strong>: 8 + 72 + 4 + 550 + 4 + 550 + 1 + 1 + 1 + 4 = 1195
/// bytes. Each <see cref="KeyEvent"/> adds 35 bytes (variable on top of
/// base). Adding any field is a corpus-fixture-invalidating change; handle
/// via SerializationContract version bump.
/// </para>
///
/// <para>
/// <strong>v0 → v1 schema bump</strong>: 2026-04-30 PitchRules layer ship
/// per SPEC 2026-04-28 PitchRules decisions-log entry. v0 was 1188 bytes
/// (Tick + Ball + 22 PlayerStates only); v1 adds 7 base bytes + variable
/// KeyEvent body. Pinned smoke-fixture hash re-baselines as part of this
/// schema bump (intentional, per the decisions-log entry).
/// </para>
///
/// <para>
/// <strong>Roster order is caller responsibility.</strong> This helper does
/// NOT sort the player arrays — per ADR-0008 §Determinism contract ordering
/// rules, the caller MUST present players in a stable order (typically the
/// formation roster index, NOT jersey number which can collide across teams).
/// Mid-match swaps are not a concern at Month-3 (no substitutions per
/// <c>match-engine.md §Q4</c>). KeyEvents are append-only and stable in
/// emission order from <see cref="MatchRules.Step"/>.
/// </para>
/// </summary>
public static class MatchCanonicalState
{
    /// <summary>Canonical Month-3 on-pitch player count per side.</summary>
    public const int PlayersPerTeam = 11;

    /// <summary>
    /// Base canonical-snapshot width in bytes for v1 schema (no KeyEvents):
    /// Tick (8) + Ball (72) + two count headers (8) + 22 PlayerStates (22×50) +
    /// HomeScore (1) + AwayScore (1) + OutOfPlay (1) + KeyEvent count (4)
    /// = 1195 bytes. The total snapshot width is
    /// <c>EncodedBaseByteCount + state.KeyEvents.Count * KeyEvent.EncodedByteCount</c>.
    /// </summary>
    public const int EncodedBaseByteCount = 1195;

    /// <summary>
    /// Compute the exact canonical-snapshot width in bytes for the given
    /// state, including the variable KeyEvent body.
    /// </summary>
    public static int EncodedByteCountFor(MatchSimulationState state)
    {
        if (state is null)
        {
            throw new ArgumentNullException(nameof(state));
        }
        return EncodedBaseByteCount + state.KeyEvents.Count * KeyEvent.EncodedByteCount;
    }

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
        ReadOnlySpan<PlayerState> awayTeam,
        byte homeScore,
        byte awayScore,
        OutOfPlay outOfPlay,
        IReadOnlyList<KeyEvent> keyEvents)
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
        if (keyEvents is null)
        {
            throw new ArgumentNullException(nameof(keyEvents));
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

        // 7-8. Score.
        encoder.WriteByte(homeScore);
        encoder.WriteByte(awayScore);

        // 9. OutOfPlay flag.
        encoder.WriteByte((byte)outOfPlay);

        // 10-11. KeyEvents (count + entries in append order).
        encoder.WriteCount(keyEvents.Count);
        for (int i = 0; i < keyEvents.Count; i++)
        {
            keyEvents[i].WriteCanonical(encoder);
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

        Write(encoder,
            state.CurrentTick, state.Ball,
            state.HomeTeam, state.AwayTeam,
            state.HomeScore, state.AwayScore,
            state.OutOfPlay,
            state.KeyEvents);
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
        ReadOnlySpan<PlayerState> awayTeam,
        byte homeScore,
        byte awayScore,
        OutOfPlay outOfPlay,
        IReadOnlyList<KeyEvent> keyEvents)
    {
        int capacityHint = EncodedBaseByteCount + keyEvents.Count * KeyEvent.EncodedByteCount;
        CanonicalEncoder encoder = new(initialCapacity: capacityHint);
        Write(encoder, currentTick, ball, homeTeam, awayTeam, homeScore, awayScore, outOfPlay, keyEvents);
        return encoder.ComputeSha256Hex();
    }

    /// <summary>
    /// Convenience: compute the SHA256 hash of a production match simulation
    /// snapshot.
    /// </summary>
    public static string ComputeHash(MatchSimulationState state)
    {
        CanonicalEncoder encoder = new(initialCapacity: EncodedByteCountFor(state));
        Write(encoder, state);
        return encoder.ComputeSha256Hex();
    }

    #region Convenience overloads — score=0 / OutOfPlay=InPlay / empty KeyEvents

    /// <summary>
    /// Convenience overload that defaults score to (0, 0), OutOfPlay to
    /// <see cref="OutOfPlay.InPlay"/>, and KeyEvents to an empty list.
    /// Useful for tests + initial-state hashing where the new PitchRules
    /// fields haven't accumulated any non-default values yet. Output bytes
    /// are still the v1 canonical layout (<see cref="EncodedBaseByteCount"/>
    /// total) — defaulting fields does NOT skip them in the encoding.
    /// </summary>
    public static void Write(
        CanonicalEncoder encoder,
        Tick currentTick,
        BallState ball,
        ReadOnlySpan<PlayerState> homeTeam,
        ReadOnlySpan<PlayerState> awayTeam)
    {
        Write(encoder, currentTick, ball, homeTeam, awayTeam,
            homeScore: 0,
            awayScore: 0,
            outOfPlay: OutOfPlay.InPlay,
            keyEvents: EmptyKeyEvents);
    }

    /// <summary>Convenience: SHA256 with score=0, OutOfPlay=InPlay, empty KeyEvents.</summary>
    public static string ComputeHash(
        Tick currentTick,
        BallState ball,
        ReadOnlySpan<PlayerState> homeTeam,
        ReadOnlySpan<PlayerState> awayTeam)
    {
        return ComputeHash(currentTick, ball, homeTeam, awayTeam,
            homeScore: 0,
            awayScore: 0,
            outOfPlay: OutOfPlay.InPlay,
            keyEvents: EmptyKeyEvents);
    }

    private static readonly KeyEvent[] EmptyKeyEvents = Array.Empty<KeyEvent>();

    #endregion
}

using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Mutable per-match per-player per-signature cooldown + fire-count
/// tracker. Allocated once at match start by
/// <see cref="MatchSimulationRunner"/> and threaded through
/// <see cref="SignatureRules.Step"/> each tick so the trigger logic
/// can enforce both per-fire cooldown and per-match fire-count caps
/// without per-tick allocation.
///
/// <para>
/// <strong>Storage shape</strong>: flat 2D arrays
/// <c>[SignatureCount × (PlayersPerTeam × 2)]</c>. Player index
/// encoding: <c>0..10</c> = home roster slots 1..11, <c>11..21</c> =
/// away roster slots 1..11. Signature index encoding: cast
/// <see cref="SignatureKind"/> minus 1 (so <c>LowCutback=1</c> maps to
/// row 0, etc.).
/// </para>
///
/// <para>
/// <strong>Not canonical state.</strong> This object is runner-private
/// scratch — never serialized into <c>MatchCanonicalState.Write</c>.
/// Identical inputs produce identical state because the cooldown-update
/// logic is deterministic, but the cooldown record itself is derivable
/// from the canonical <c>KeyEvents</c> stream + the signature config.
/// Phase-4+ replay tools that need it can reconstruct from those two.
/// </para>
/// </summary>
public sealed class SignatureCooldownState
{
    private const int SignatureCount = 3;  // Phase-3 minimum: #13, #20, #22
    private const int PlayersPerTeam = 11;
    private const int TotalPlayerSlots = PlayersPerTeam * 2;

    private readonly long[,] _lastFiredTick;
    private readonly byte[,] _firedCount;

    public SignatureCooldownState()
    {
        _lastFiredTick = new long[SignatureCount, TotalPlayerSlots];
        _firedCount = new byte[SignatureCount, TotalPlayerSlots];

        // long.MinValue marks "never fired" so any positive cooldown-window
        // check `currentTick - lastFiredTick > windowTicks` is true for the
        // first fire. The default zero-init would mean "fired at tick 0,"
        // which would erroneously suppress a valid tick-0 fire on the very
        // first iteration.
        for (int s = 0; s < SignatureCount; s++)
        {
            for (int p = 0; p < TotalPlayerSlots; p++)
            {
                _lastFiredTick[s, p] = long.MinValue;
            }
        }
    }

    /// <summary>
    /// Player index encoding: home roster slots 1..11 → indices 0..10;
    /// away roster slots 1..11 → indices 11..21.
    /// </summary>
    public static int PlayerIndex(TeamSide side, byte rosterSlot)
    {
        if (rosterSlot < 1 || rosterSlot > PlayersPerTeam)
        {
            throw new ArgumentOutOfRangeException(
                nameof(rosterSlot), rosterSlot,
                $"Roster slot must be 1..{PlayersPerTeam}.");
        }
        int baseOffset = side == TeamSide.Home ? 0 : PlayersPerTeam;
        return baseOffset + (rosterSlot - 1);
    }

    /// <summary>
    /// True iff <paramref name="signature"/> can fire for this player at
    /// <paramref name="currentTick"/>: cooldown window has elapsed AND
    /// per-match fire cap not yet reached.
    /// </summary>
    public bool CanFire(SignatureKind signature, int playerIndex,
        long currentTick, int cooldownTicks, byte maxFiresPerMatch)
    {
        if ((uint)playerIndex >= TotalPlayerSlots)
        {
            throw new ArgumentOutOfRangeException(
                nameof(playerIndex), playerIndex,
                $"playerIndex must be in [0, {TotalPlayerSlots - 1}].");
        }
        int sigRow = (int)signature - 1;
        if (sigRow < 0 || sigRow >= SignatureCount)
        {
            throw new ArgumentOutOfRangeException(
                nameof(signature), signature,
                $"signature must be a defined SignatureKind value (1..{SignatureCount}).");
        }
        if (_firedCount[sigRow, playerIndex] >= maxFiresPerMatch)
        {
            return false;
        }
        long last = _lastFiredTick[sigRow, playerIndex];
        // long.MinValue sentinel: never fired → can fire.
        if (last == long.MinValue)
        {
            return true;
        }
        return currentTick - last >= cooldownTicks;
    }

    /// <summary>
    /// Read the per-match fire count for the given signature + player.
    /// Used by tests + by <see cref="RecordFireAndDidReachCap"/> callers
    /// that need to inspect saturation without recording.
    /// </summary>
    public byte GetFiredCount(SignatureKind signature, int playerIndex)
    {
        if ((uint)playerIndex >= TotalPlayerSlots)
        {
            throw new ArgumentOutOfRangeException(
                nameof(playerIndex), playerIndex,
                $"playerIndex must be in [0, {TotalPlayerSlots - 1}].");
        }
        int sigRow = (int)signature - 1;
        if (sigRow < 0 || sigRow >= SignatureCount)
        {
            throw new ArgumentOutOfRangeException(
                nameof(signature), signature,
                $"signature must be a defined SignatureKind value (1..{SignatureCount}).");
        }
        return _firedCount[sigRow, playerIndex];
    }

    /// <summary>
    /// Record a fire AND report whether the fire-count just reached the
    /// per-match cap. Used by <see cref="SignatureRules"/> to emit the
    /// Phase-3 <see cref="KeyEventKind.SignatureBreakthrough"/> on the
    /// cap-reach moment without exposing internal counter access.
    ///
    /// <para>
    /// <strong>Pre-cap guard</strong> per pr-review-toolkit:feature-dev:code-reviewer
    /// 2026-04-30 finding #1: throws <see cref="InvalidOperationException"/>
    /// if <c>firedCount &gt;= maxFiresPerMatch</c> at entry. Without this
    /// guard, a caller that bypasses <see cref="CanFire"/> (a future
    /// Phase-4+ code path or a misconfigured test) would silently
    /// over-increment the counter past the cap and return <c>false</c>
    /// (since post-record count would equal <c>maxFiresPerMatch + 1</c>,
    /// not <c>maxFiresPerMatch</c>) — silently missing the breakthrough
    /// emission AND corrupting the cap state. <see cref="SignatureRules"/>'s
    /// production path always checks <see cref="CanFire"/> first, so this
    /// guard never fires there; it's defense-in-depth for foreign callers.
    /// </para>
    /// </summary>
    /// <returns>True iff the post-record fire count equals
    /// <paramref name="maxFiresPerMatch"/> (i.e., this fire was the
    /// player's final-allowed fire of this signature this match).</returns>
    public bool RecordFireAndDidReachCap(
        SignatureKind signature, int playerIndex, long currentTick, byte maxFiresPerMatch)
    {
        if ((uint)playerIndex >= TotalPlayerSlots)
        {
            throw new ArgumentOutOfRangeException(
                nameof(playerIndex), playerIndex,
                $"playerIndex must be in [0, {TotalPlayerSlots - 1}].");
        }
        int sigRow = (int)signature - 1;
        if (sigRow < 0 || sigRow >= SignatureCount)
        {
            throw new ArgumentOutOfRangeException(
                nameof(signature), signature,
                $"signature must be a defined SignatureKind value (1..{SignatureCount}).");
        }
        if (_firedCount[sigRow, playerIndex] >= maxFiresPerMatch)
        {
            throw new InvalidOperationException(
                $"RecordFireAndDidReachCap called for signature {signature} " +
                $"player-index {playerIndex} when firedCount " +
                $"({_firedCount[sigRow, playerIndex]}) is already at or past " +
                $"maxFiresPerMatch ({maxFiresPerMatch}). Caller must check " +
                "CanFire before calling.");
        }
        RecordFire(signature, playerIndex, currentTick);
        return _firedCount[sigRow, playerIndex] == maxFiresPerMatch;
    }

    /// <summary>Record that <paramref name="signature"/> fired for the player at <paramref name="currentTick"/>.</summary>
    public void RecordFire(SignatureKind signature, int playerIndex, long currentTick)
    {
        if ((uint)playerIndex >= TotalPlayerSlots)
        {
            throw new ArgumentOutOfRangeException(
                nameof(playerIndex), playerIndex,
                $"playerIndex must be in [0, {TotalPlayerSlots - 1}]. Use SignatureCooldownState.PlayerIndex(side, rosterSlot).");
        }
        int sigRow = (int)signature - 1;
        if (sigRow < 0 || sigRow >= SignatureCount)
        {
            throw new ArgumentOutOfRangeException(
                nameof(signature), signature,
                $"signature must be a defined SignatureKind value (1..{SignatureCount}).");
        }

        _lastFiredTick[sigRow, playerIndex] = currentTick;

        // Per pr-review-toolkit 2026-04-30 round-2: if the fire count
        // ever reaches byte.MaxValue, the silent-cap-at-255 pattern
        // would mask a runaway spam regression deterministically (CanFire
        // would still return true; max-fires gate would never engage).
        // Throw loudly so a future bug surfaces rather than getting
        // averaged out by the byte ceiling.
        if (_firedCount[sigRow, playerIndex] == byte.MaxValue)
        {
            throw new InvalidOperationException(
                $"SignatureCooldownState.RecordFire: fire count saturated at {byte.MaxValue} for " +
                $"signature {signature} player-index {playerIndex}. Phase-3 caps are 2-3 per match; " +
                $"saturation indicates a config bug or runaway dispatch.");
        }
        _firedCount[sigRow, playerIndex]++;
    }

    /// <summary>
    /// Reset all cooldown + fire-count state to "never fired." Lets the
    /// runner reuse one allocation across consecutive matches in a sweep
    /// without re-allocating the 2D arrays. Per pr-review-toolkit:
    /// type-design-analyzer 2026-04-30 round-2.
    /// </summary>
    public void Reset()
    {
        for (int s = 0; s < SignatureCount; s++)
        {
            for (int p = 0; p < TotalPlayerSlots; p++)
            {
                _lastFiredTick[s, p] = long.MinValue;
                _firedCount[s, p] = 0;
            }
        }
    }
}

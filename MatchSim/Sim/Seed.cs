using System;
using System.Globalization;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// A deterministic 64-bit sim seed. Used directly as PRNG state by sim
/// stochastic events, OR as a stable identifier in event ledgers + replay
/// fixtures. Per ADR-0001 forbidden-nondeterminism + ADR-0008
/// <c>ViewerEvent.Seed</c> + TECH_APPROACH.md §3.2: every match carries a
/// <c>match_seed</c>; every in-match stochastic event carries an event seed
/// derived from <c>(match_seed, tick, event_id)</c> via
/// <see cref="Derive(ulong, Tick, ulong)"/>. Replay re-runs the sim with
/// identical outputs because the same triple deterministically produces the
/// same seed.
///
/// <para>
/// Derivation uses SplitMix64 — the well-known finalizer used by Java's
/// <c>SplittableRandom</c>. Reasons: pure integer math (deterministic across
/// platforms), allocation-free, good avalanche (a 1-bit input flip flips
/// roughly half the output bits), and well-tested. Crypto strength is NOT
/// required — we want stable + fast + well-distributed.
/// </para>
///
/// <para>
/// Canonical string form is lowercase hex with <c>0x</c> prefix and 16
/// digits — matches <c>golden-replay-corpus.md</c>'s smoke-seed
/// <c>0xdeadbeefdeadbeef</c> format. <see cref="Parse(string)"/> /
/// <see cref="TryParse(string?, out Seed)"/> round-trip against
/// <see cref="ToString()"/>.
/// </para>
/// </summary>
public readonly struct Seed : IEquatable<Seed>, IComparable<Seed>, IComparable
{
    private readonly ulong _value;

    /// <summary>Construct from a raw <see cref="ulong"/>. Use this when wrapping a known-good seed value.</summary>
    public Seed(ulong value)
    {
        _value = value;
    }

    /// <summary>The raw 64-bit seed value. Stable across runs and platforms.</summary>
    public ulong Value => _value;

    #region Constants

    /// <summary>Seed zero. Default-constructible. Useful as a baseline / sentinel.</summary>
    public static Seed Zero => default;

    #endregion

    #region Factories

    /// <summary>Construct from a raw <see cref="ulong"/>.</summary>
    public static Seed FromUInt64(ulong value) => new(value);

    /// <summary>
    /// Derive a per-event seed from the canonical triple
    /// <c>(matchSeed, tick, eventId)</c>. Same triple → same seed,
    /// guaranteed across platforms. Different triples → avalanching
    /// outputs (~50% bit-flip rate on a 1-bit input change).
    /// </summary>
    public static Seed Derive(ulong matchSeed, Tick tick, ulong eventId)
    {
        // Compose: mix matchSeed with tick, then with eventId. Order matters
        // (composition is non-commutative — `Derive(a, t, b) ≠ Derive(b, t, a)`).
        // Each step runs SplitMix64 over the running hash XOR'd with the
        // next input.
        ulong h = matchSeed;
        h = SplitMix64(h ^ unchecked((ulong)tick.Value));
        h = SplitMix64(h ^ eventId);
        return new Seed(h);
    }

    /// <summary>
    /// SplitMix64 finalizer. Pure integer math; deterministic across platforms;
    /// well-distributed avalanche. Same as Java's <c>SplittableRandom</c>
    /// finalizer + the <c>splitmix64</c> seed-mixer used by xoshiro authors
    /// to seed their PRNGs.
    /// </summary>
    private static ulong SplitMix64(ulong x)
    {
        unchecked
        {
            x = (x ^ (x >> 30)) * 0xBF58476D1CE4E5B9UL;
            x = (x ^ (x >> 27)) * 0x94D049BB133111EBUL;
            x = x ^ (x >> 31);
        }
        return x;
    }

    #endregion

    #region String form (canonical 0x-prefixed lowercase hex)

    /// <inheritdoc />
    public override string ToString()
    {
        // 16 hex digits, lowercase, 0x-prefixed. Matches
        // golden-replay-corpus.md smoke-seed format `0xdeadbeefdeadbeef`.
        return "0x" + _value.ToString("x16", CultureInfo.InvariantCulture);
    }

    /// <summary>
    /// Parse a hex seed string. Accepts an optional <c>0x</c> or <c>0X</c>
    /// prefix (case-insensitive on the <c>x</c>); body is 1-16 hex digits.
    /// Throws <see cref="FormatException"/> on garbage,
    /// <see cref="ArgumentNullException"/> on null.
    /// </summary>
    public static Seed Parse(string s)
    {
        if (s is null)
        {
            throw new ArgumentNullException(nameof(s));
        }
        if (!TryParse(s, out Seed result))
        {
            throw new FormatException($"Cannot parse '{s}' as Seed (expected 1-16 hex digits with optional 0x prefix).");
        }
        return result;
    }

    /// <summary>Try to parse a canonical hex seed string. Returns <c>false</c> on any failure.</summary>
    public static bool TryParse(string? s, out Seed result)
    {
        result = default;
        if (string.IsNullOrEmpty(s))
        {
            return false;
        }

        // Normalize: strip 0x / 0X prefix if present.
        ReadOnlySpan<char> body = s.AsSpan();
        if (body.Length >= 2 && body[0] == '0' && (body[1] == 'x' || body[1] == 'X'))
        {
            body = body.Slice(2);
        }

        if (body.Length == 0 || body.Length > 16)
        {
            return false;
        }
        if (!ulong.TryParse(body, NumberStyles.HexNumber, CultureInfo.InvariantCulture, out ulong value))
        {
            return false;
        }
        result = new Seed(value);
        return true;
    }

    #endregion

    #region Equality + Comparison

    /// <inheritdoc />
    public bool Equals(Seed other) => _value == other._value;

    /// <inheritdoc />
    public override bool Equals(object? obj) => obj is Seed other && Equals(other);

    /// <inheritdoc />
    public override int GetHashCode() => _value.GetHashCode();

    /// <inheritdoc />
    public int CompareTo(Seed other) => _value.CompareTo(other._value);

    /// <inheritdoc />
    int IComparable.CompareTo(object? obj)
    {
        if (obj is null)
        {
            return 1;
        }
        if (obj is Seed other)
        {
            return CompareTo(other);
        }
        throw new ArgumentException("Object must be of type Seed.", nameof(obj));
    }

    public static bool operator ==(Seed left, Seed right) => left._value == right._value;

    public static bool operator !=(Seed left, Seed right) => left._value != right._value;

    public static bool operator <(Seed left, Seed right) => left._value < right._value;

    public static bool operator >(Seed left, Seed right) => left._value > right._value;

    public static bool operator <=(Seed left, Seed right) => left._value <= right._value;

    public static bool operator >=(Seed left, Seed right) => left._value >= right._value;

    #endregion
}

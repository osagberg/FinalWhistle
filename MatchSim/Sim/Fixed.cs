using System;
using System.Globalization;
using System.Numerics;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Q32.32 fixed-point arithmetic — the canonical numeric format for MatchSim.
/// Top 32 bits store the signed integer part (2's complement); bottom 32 bits
/// store the fractional part. Range is roughly ±2.147e9; precision is
/// 2^-32 ≈ 2.328e-10.
///
/// <para>
/// Determinism: pure integer math. No floats. No platform-dependent behavior.
/// Cross-platform parity is guaranteed by integer-bit-equivalence on every
/// runtime that supports <see cref="long"/>. This is the architectural floor
/// for the MatchSim canonical-state determinism contract per
/// TECH_APPROACH.md §3.2 and the 2026-04-23 SPEC decisions-log entry
/// "Q32.32 fixed-point is the canonical MatchSim format".
/// </para>
///
/// <para>
/// Overflow posture: every arithmetic operator is checked. Overflow throws
/// <see cref="OverflowException"/>; silent wraparound is forbidden.
/// </para>
///
/// <para>
/// Serialization: <see cref="ToString()"/> emits the canonical decimal-string
/// form with 10 fractional digits in invariant culture (matches
/// FW-VAL-A-018 in design/specs/content-pack-validation-contract.md).
/// <see cref="Parse(string)"/> round-trips against that form.
/// </para>
/// </summary>
public readonly struct Fixed : IEquatable<Fixed>, IComparable<Fixed>, IComparable, IFormattable
{
    #region Format constants

    /// <summary>
    /// Number of fractional bits in the Q32.32 representation.
    /// </summary>
    public const int FractionalBits = 32;

    /// <summary>
    /// Underlying integer scale: <c>1</c> in fixed-point ≡ 2^<see cref="FractionalBits"/> in raw.
    /// </summary>
    public const long OneRaw = 1L << FractionalBits;

    /// <summary>
    /// Mask covering the fractional bits.
    /// </summary>
    private const long FractionalMask = OneRaw - 1L;

    /// <summary>
    /// Number of fractional digits emitted by canonical <see cref="ToString()"/>.
    /// 10 digits gives full Q32.32 precision (1 ULP ≈ 2.3e-10).
    /// </summary>
    public const int CanonicalFractionalDigits = 10;

    private const NumberStyles DecimalParseStyles =
        NumberStyles.AllowLeadingSign | NumberStyles.AllowDecimalPoint;

    #endregion

    #region Storage

    private readonly long _raw;

    private Fixed(long raw)
    {
        _raw = raw;
    }

    /// <summary>
    /// Underlying integer representation. Stable across runs and platforms.
    /// Exposed for fixture authoring / serialization / debug tools per
    /// design/specs/golden-replay-corpus.md + design/specs/save-migration-fixtures.md.
    /// Normal sim-side code uses arithmetic operators, not this property.
    /// </summary>
    public long RawValue => _raw;

    #endregion

    #region Constants

    /// <summary>Zero. Raw = 0.</summary>
    public static Fixed Zero => default;

    /// <summary>One. Raw = 2^32.</summary>
    public static Fixed One => new(OneRaw);

    /// <summary>Negative one. Raw = -2^32.</summary>
    public static Fixed MinusOne => new(-OneRaw);

    /// <summary>One half (0.5). Raw = 2^31.</summary>
    public static Fixed Half => new(OneRaw >> 1);

    /// <summary>The largest representable value. Raw = <see cref="long.MaxValue"/> ≈ +2^31 - 2^-32.</summary>
    public static Fixed MaxValue => new(long.MaxValue);

    /// <summary>The most negative representable value. Raw = <see cref="long.MinValue"/> ≈ -2^31.</summary>
    public static Fixed MinValue => new(long.MinValue);

    /// <summary>
    /// The smallest positive non-zero value. Raw = 1, magnitude ≈ 2.328e-10.
    /// One ULP of the Q32.32 grid.
    /// </summary>
    public static Fixed Epsilon => new(1L);

    #endregion

    #region Factories

    /// <summary>
    /// Construct from the raw underlying long. Use ONLY when you genuinely
    /// need bit-level control (fixture authoring, deserialization).
    /// Normal code paths use <see cref="FromInt(int)"/> or <see cref="Parse(string)"/>.
    /// </summary>
    public static Fixed FromRaw(long raw) => new(raw);

    /// <summary>
    /// Construct from a 32-bit signed integer. Always safe — every <see cref="int"/>
    /// fits in the Q32.32 integer range (±2^31).
    /// </summary>
    public static Fixed FromInt(int value) => new((long)value << FractionalBits);

    /// <summary>
    /// Construct from a 64-bit signed integer. Throws <see cref="OverflowException"/>
    /// if <paramref name="value"/> is outside [-2^31, 2^31 - 1].
    /// </summary>
    public static Fixed FromLong(long value)
    {
        if (value < int.MinValue || value > int.MaxValue)
        {
            throw new OverflowException(
                $"Value {value} is outside the Q32.32 integer range [-2^31, 2^31 - 1].");
        }
        return new(value << FractionalBits);
    }

    #endregion

    #region Equality + Comparison

    /// <inheritdoc />
    public bool Equals(Fixed other) => _raw == other._raw;

    /// <inheritdoc />
    public override bool Equals(object? obj) => obj is Fixed other && Equals(other);

    /// <inheritdoc />
    public override int GetHashCode() => _raw.GetHashCode();

    /// <inheritdoc />
    public int CompareTo(Fixed other) => _raw.CompareTo(other._raw);

    /// <inheritdoc />
    int IComparable.CompareTo(object? obj)
    {
        if (obj is null)
        {
            return 1;
        }
        if (obj is Fixed other)
        {
            return CompareTo(other);
        }
        throw new ArgumentException("Object must be of type Fixed.", nameof(obj));
    }

    public static bool operator ==(Fixed left, Fixed right) => left._raw == right._raw;

    public static bool operator !=(Fixed left, Fixed right) => left._raw != right._raw;

    public static bool operator <(Fixed left, Fixed right) => left._raw < right._raw;

    public static bool operator >(Fixed left, Fixed right) => left._raw > right._raw;

    public static bool operator <=(Fixed left, Fixed right) => left._raw <= right._raw;

    public static bool operator >=(Fixed left, Fixed right) => left._raw >= right._raw;

    #endregion

    #region Arithmetic — additive

    /// <summary>Negate. Throws on <see cref="MinValue"/>.</summary>
    public static Fixed Negate(Fixed value) => new(checked(-value._raw));

    /// <summary>Absolute value. Throws on <see cref="MinValue"/>.</summary>
    public static Fixed Abs(Fixed value) => value._raw < 0L ? Negate(value) : value;

    /// <summary>Sign: <c>-1</c>, <c>0</c>, or <c>1</c>.</summary>
    public static int Sign(Fixed value) => Math.Sign(value._raw);

    /// <summary>Smaller of two values.</summary>
    public static Fixed Min(Fixed a, Fixed b) => a._raw <= b._raw ? a : b;

    /// <summary>Larger of two values.</summary>
    public static Fixed Max(Fixed a, Fixed b) => a._raw >= b._raw ? a : b;

    /// <summary>
    /// Non-negative square root. Throws on negative input. Uses Newton's
    /// method on <see cref="BigInteger"/> for cross-platform deterministic
    /// integer-only iteration. Result is the floor of the true Q32.32
    /// square root: <c>Sqrt(x) * Sqrt(x) &lt;= x</c> by construction (with
    /// equality only when <c>x</c> is a perfect Q32.32 square).
    ///
    /// <para>
    /// Implementation: for input <c>x</c> with raw <c>X = x · 2^32</c>, the
    /// result raw is <c>floor(sqrt(X · 2^32))</c>. The intermediate
    /// <c>X · 2^32</c> is up to 96 bits — <see cref="BigInteger"/> handles
    /// it exactly. Newton's iteration on integers is monotonically
    /// decreasing once it crosses the root, so we terminate when the next
    /// candidate is no longer smaller than the current.
    /// </para>
    /// </summary>
    public static Fixed Sqrt(Fixed value)
    {
        if (value._raw < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(value), value, "Square root of a negative Fixed is undefined.");
        }
        if (value._raw == 0)
        {
            return Zero;
        }

        // n = X · 2^32; result raw = floor(sqrt(n)).
        BigInteger n = (BigInteger)value._raw << FractionalBits;

        // Newton-Raphson integer square root. Initial guess: bit-length-based
        // over-estimate — for n with B bits, sqrt(n) has ceil(B/2) bits.
        // BigInteger.Log2 / GetBitLength are .NET 5+ only; use byte-count
        // (netstandard2.1-compatible) as a safe over-estimate. Byte count
        // gives bits within ±8; over-estimating is fine since Newton's
        // method on integers monotonically decreases until convergence.
        int bits = n.GetByteCount() * 8;
        BigInteger x = BigInteger.One << ((bits + 1) >> 1);

        // Iterate. Newton's method on integers: x_{n+1} = (x_n + n/x_n) / 2.
        // Once x_{n+1} >= x_n we've crossed the root and can stop.
        while (true)
        {
            BigInteger y = (x + n / x) >> 1;
            if (y >= x)
            {
                break;
            }
            x = y;
        }

        // x is now floor(sqrt(n)). Range-check (must fit in long; for
        // Fixed.MaxValue the result is sqrt(2^31 - epsilon) ≈ 46340.95 in
        // Q32.32 — well within long range, but the assertion guards against
        // future precision-doubling shifts).
        if (x > long.MaxValue)
        {
            throw new OverflowException("Sqrt result overflowed Q32.32 raw range.");
        }
        return new Fixed((long)x);
    }

    public static Fixed operator +(Fixed left, Fixed right) =>
        new(checked(left._raw + right._raw));

    public static Fixed operator -(Fixed left, Fixed right) =>
        new(checked(left._raw - right._raw));

    public static Fixed operator -(Fixed value) => Negate(value);

    public static Fixed operator +(Fixed value) => value;

    #endregion

    #region Arithmetic — multiplicative

    /// <summary>
    /// Q32.32 multiplication. Computes the 128-bit product of the underlying
    /// raw values via 32-bit-split unsigned multiply, shifts right 32 bits
    /// to renormalize the fractional precision, applies sign, and overflow-checks
    /// the result against signed-long range. Allocation-free on the hot path
    /// (everything is stack-local primitives). Special-cases <see cref="long.MinValue"/>
    /// inputs via <see cref="BigInteger"/> because the unsigned magnitude of
    /// <see cref="long.MinValue"/> is not representable as <see cref="long"/>.
    /// </summary>
    public static Fixed operator *(Fixed left, Fixed right)
    {
        long a = left._raw;
        long b = right._raw;

        if (a == 0L || b == 0L)
        {
            return Zero;
        }

        // long.MinValue cannot be negated — its absolute value is 2^63, which
        // doesn't fit in signed long. Route through BigInteger for those inputs.
        if (a == long.MinValue || b == long.MinValue)
        {
            return MultiplyViaBigInteger(a, b);
        }

        bool negative = (a < 0L) ^ (b < 0L);
        ulong au = a < 0L ? (ulong)(-a) : (ulong)a;
        ulong bu = b < 0L ? (ulong)(-b) : (ulong)b;

        // 32-bit-split unsigned 64x64 → 128-bit multiply.
        ulong aLo = au & 0xFFFFFFFFUL;
        ulong aHi = au >> 32;
        ulong bLo = bu & 0xFFFFFFFFUL;
        ulong bHi = bu >> 32;

        ulong loLo = aLo * bLo;
        ulong loHi = aLo * bHi;
        ulong hiLo = aHi * bLo;
        ulong hiHi = aHi * bHi;

        // Compose the 128-bit product into upper / lower 64-bit halves.
        ulong cross = (loLo >> 32) + (loHi & 0xFFFFFFFFUL) + (hiLo & 0xFFFFFFFFUL);
        ulong upper = hiHi + (loHi >> 32) + (hiLo >> 32) + (cross >> 32);
        ulong lower = (loLo & 0xFFFFFFFFUL) | (cross << 32);

        // Q32.32 result = full_128bit_product >> 32. After the shift:
        //   shifted_upper = upper >> 32
        //   shifted_lower = (lower >> 32) | (upper << 32)
        ulong shiftedUpper = upper >> 32;
        ulong shiftedLower = (lower >> 32) | (upper << 32);

        if (shiftedUpper != 0UL)
        {
            throw new OverflowException("Fixed multiplication overflow.");
        }

        // Apply sign + range-check against signed-long.
        if (negative)
        {
            const ulong NegativeBound = (ulong)long.MaxValue + 1UL; // = 2^63 = |long.MinValue|
            if (shiftedLower > NegativeBound)
            {
                throw new OverflowException("Fixed multiplication overflow.");
            }
            if (shiftedLower == NegativeBound)
            {
                return new(long.MinValue);
            }
            return new(-(long)shiftedLower);
        }

        if (shiftedLower > (ulong)long.MaxValue)
        {
            throw new OverflowException("Fixed multiplication overflow.");
        }
        return new((long)shiftedLower);
    }

    private static Fixed MultiplyViaBigInteger(long a, long b)
    {
        BigInteger product = (BigInteger)a * (BigInteger)b;
        BigInteger shifted = product >> FractionalBits;
        if (shifted < long.MinValue || shifted > long.MaxValue)
        {
            throw new OverflowException("Fixed multiplication overflow.");
        }
        return new((long)shifted);
    }

    /// <summary>
    /// Q32.32 division. <c>a / b</c> is computed as <c>(a_raw &lt;&lt; 32) / b_raw</c>;
    /// the left-shift can exceed signed-long range, so the calculation is
    /// performed via <see cref="BigInteger"/> for correctness. BigInteger
    /// allocation cost is acceptable for Phase-3 prototype; profile-and-optimize
    /// if division becomes a hot path. Truncation matches typical
    /// integer-division semantics.
    /// </summary>
    public static Fixed operator /(Fixed left, Fixed right)
    {
        if (right._raw == 0L)
        {
            throw new DivideByZeroException();
        }

        BigInteger numerator = (BigInteger)left._raw << FractionalBits;
        BigInteger result = numerator / (BigInteger)right._raw;

        if (result < long.MinValue || result > long.MaxValue)
        {
            throw new OverflowException("Fixed division overflow.");
        }
        return new((long)result);
    }

    #endregion

    #region Rounding

    /// <summary>Largest <see cref="Fixed"/> with zero fractional part that is ≤ <paramref name="value"/>.</summary>
    public static Fixed Floor(Fixed value) => new((value._raw >> FractionalBits) << FractionalBits);

    /// <summary>Smallest <see cref="Fixed"/> with zero fractional part that is ≥ <paramref name="value"/>.</summary>
    public static Fixed Ceiling(Fixed value)
    {
        if ((value._raw & FractionalMask) == 0L)
        {
            return value;
        }
        return new(checked(((value._raw >> FractionalBits) + 1L) << FractionalBits));
    }

    /// <summary>Round toward zero — drop the fractional part.</summary>
    public static Fixed Truncate(Fixed value)
    {
        if (value._raw >= 0L)
        {
            return new(value._raw & ~FractionalMask);
        }
        if ((value._raw & FractionalMask) == 0L)
        {
            return value;
        }
        return new(checked((value._raw & ~FractionalMask) + OneRaw));
    }

    /// <summary>Round to the nearest integer using banker's rounding (<see cref="MidpointRounding.ToEven"/>).</summary>
    public static Fixed Round(Fixed value)
    {
        long raw = value._raw;
        long intPart = raw >> FractionalBits; // signed-right-shift = floor(raw / 2^32)
        long frac = raw & FractionalMask;
        long half = OneRaw >> 1;

        if (frac < half)
        {
            return new(intPart << FractionalBits);
        }
        if (frac > half)
        {
            return new(checked((intPart + 1L) << FractionalBits));
        }
        // Exactly half: round-to-even.
        return (intPart & 1L) == 0L
            ? new(intPart << FractionalBits)
            : new(checked((intPart + 1L) << FractionalBits));
    }

    #endregion

    #region Serialization (canonical decimal-string per FW-VAL-A-018)

    /// <inheritdoc />
    public override string ToString() => ToString(null, CultureInfo.InvariantCulture);

    /// <inheritdoc />
    public string ToString(string? format, IFormatProvider? formatProvider)
    {
        // We ignore the format string for now — canonical Q32.32 serialization
        // is fixed at CanonicalFractionalDigits regardless of caller intent.
        // Future: support "Rxx" for raw / "Gxx" for general etc. when a real
        // need surfaces in Phase-3+ debug tooling.
        _ = format;
        IFormatProvider provider = formatProvider ?? CultureInfo.InvariantCulture;

        // decimal has enough precision for the 10 fractional digits that make
        // Q32.32 decimal strings round-trip through Parse().
        decimal asDecimal = (decimal)_raw / (decimal)OneRaw;
        return asDecimal.ToString("F" + CanonicalFractionalDigits.ToString(CultureInfo.InvariantCulture), provider);
    }

    /// <summary>
    /// Parse a canonical decimal-string. Round-trips against
    /// <see cref="ToString()"/>. Uses invariant culture; rejects culture-specific
    /// formatting (e.g. comma decimal separator) and scientific notation.
    /// </summary>
    public static Fixed Parse(string s)
    {
        if (s is null)
        {
            throw new ArgumentNullException(nameof(s));
        }
        return Parse(s, CultureInfo.InvariantCulture);
    }

    /// <summary>Parse with explicit culture; scientific notation is still rejected.</summary>
    public static Fixed Parse(string s, IFormatProvider? formatProvider)
    {
        if (s is null)
        {
            throw new ArgumentNullException(nameof(s));
        }
        if (!TryParse(s, formatProvider, out Fixed result))
        {
            throw new FormatException($"Cannot parse '{s}' as Fixed.");
        }
        return result;
    }

    /// <summary>Attempt to parse; returns <c>false</c> on any failure.</summary>
    public static bool TryParse(string? s, out Fixed result) =>
        TryParse(s, CultureInfo.InvariantCulture, out result);

    /// <summary>Attempt to parse with explicit culture; scientific notation is still rejected.</summary>
    public static bool TryParse(string? s, IFormatProvider? formatProvider, out Fixed result)
    {
        result = default;
        if (s is null)
        {
            return false;
        }
        IFormatProvider provider = formatProvider ?? CultureInfo.InvariantCulture;

        if (!decimal.TryParse(
                s,
                DecimalParseStyles,
                provider,
                out decimal asDecimal))
        {
            return false;
        }

        decimal scaled;
        try
        {
            scaled = checked(asDecimal * (decimal)OneRaw);
        }
        catch (OverflowException)
        {
            return false;
        }

        // long.MinValue / long.MaxValue are exactly representable in decimal.
        // Range-check after rounding so Fixed.MaxValue.ToString() round-trips:
        // its 10-digit decimal form is slightly above the true raw maximum.
        decimal rounded = decimal.Round(scaled, 0, MidpointRounding.ToEven);
        if (rounded < long.MinValue || rounded > long.MaxValue)
        {
            return false;
        }
        result = new((long)rounded);
        return true;
    }

    #endregion
}

using System;
using System.Globalization;
using System.Numerics;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// A discrete sim-time tick. MatchSim runs on a fixed 60 Hz logical timestep
/// (<see cref="TicksPerSecond"/>); every sim event fires on a tick boundary;
/// the viewer interpolates at framerate but never drives sim time. This is
/// the architectural floor for deterministic replay per TECH_APPROACH.md §3.2
/// and ADR-0008's <c>ViewerEvent.StartTick</c> / <c>EndTick</c> contract.
///
/// <para>
/// Storage: signed <see cref="long"/> counter. <c>int</c> would cap at ~414
/// in-sim days at 60 Hz, which is enough for one season but not enough for
/// multi-season golden-replay-corpus fixtures or a long balance-harness
/// sweep. The 64-bit width is a guarantee, not a luxury.
/// </para>
///
/// <para>
/// Arithmetic is checked. Tick math overflowing means the replay is past
/// its valid range; throwing is the correct response, not silent wraparound.
/// Subtraction <see cref="Tick"/> − <see cref="Tick"/> returns <see cref="long"/>
/// (a tick-delta) so the type system distinguishes "absolute tick" from
/// "duration in ticks" without overloading.
/// </para>
/// </summary>
public readonly struct Tick : IEquatable<Tick>, IComparable<Tick>, IComparable
{
    /// <summary>
    /// Sim tick rate in Hz. Locked at 60 Hz per the 2026-04-22 SPEC
    /// "MatchSim architectural split" entry + TECH_APPROACH §3.2.
    /// Changing this is a determinism-contract supersession requiring
    /// a new decisions-log entry + golden-replay-corpus + save-migration
    /// fixture refresh.
    /// </summary>
    public const int TicksPerSecond = 60;

    /// <summary>
    /// Number of ticks in one in-sim minute (60 ticks × 60 seconds).
    /// </summary>
    public const long TicksPerMinute = (long)TicksPerSecond * 60L;

    private readonly long _value;

    /// <summary>
    /// Construct from a raw tick count. Most callers should prefer
    /// <see cref="FromSeconds(int)"/> / <see cref="FromMinutes(int)"/>
    /// for clarity at call site.
    /// </summary>
    public Tick(long value)
    {
        _value = value;
    }

    /// <summary>The tick counter value. Stable across runs and platforms.</summary>
    public long Value => _value;

    #region Constants

    /// <summary>Tick zero. Default-constructible; defines the start of any sim run.</summary>
    public static Tick Zero => default;

    /// <summary>Tick one. Useful as a per-step delta in tests.</summary>
    public static Tick One => new(1L);

    #endregion

    #region Factories

    /// <summary>
    /// Construct from whole seconds. Throws if the resulting tick count
    /// overflows <see cref="long"/>.
    /// </summary>
    public static Tick FromSeconds(int seconds) => new(checked((long)seconds * TicksPerSecond));

    /// <summary>
    /// Construct from whole in-sim minutes. Throws on overflow.
    /// </summary>
    public static Tick FromMinutes(int minutes) => new(checked((long)minutes * TicksPerMinute));

    #endregion

    #region Conversions

    /// <summary>
    /// Convert this tick count to in-sim seconds as a <see cref="Fixed"/>.
    /// Converts directly into raw Q32.32 seconds before range-checking, so
    /// long-horizon tick values do not get narrowed to <see cref="Fixed"/>
    /// before division. The conversion is approximate when the tick count is
    /// not an exact multiple of <see cref="TicksPerSecond"/> (1/60 is not
    /// exactly representable in Q32.32). For sim-side arithmetic, prefer
    /// staying in tick units; convert to seconds only at presentation
    /// boundaries.
    /// </summary>
    public Fixed ToSeconds()
    {
        BigInteger rawSeconds = ((BigInteger)_value << Fixed.FractionalBits) / TicksPerSecond;
        if (rawSeconds < long.MinValue || rawSeconds > long.MaxValue)
        {
            throw new OverflowException("Tick-to-seconds conversion overflow.");
        }

        return Fixed.FromRaw((long)rawSeconds);
    }

    #endregion

    #region Arithmetic

    /// <summary>Advance a tick by an integer delta. Throws on overflow.</summary>
    public static Tick operator +(Tick tick, long delta) => new(checked(tick._value + delta));

    /// <summary>Advance a tick by an integer delta (delta-on-the-left form).</summary>
    public static Tick operator +(long delta, Tick tick) => new(checked(delta + tick._value));

    /// <summary>Step a tick back by an integer delta. Throws on overflow.</summary>
    public static Tick operator -(Tick tick, long delta) => new(checked(tick._value - delta));

    /// <summary>
    /// Tick − Tick returns the duration between them as a <see cref="long"/>
    /// tick-delta. Distinct from <see cref="Tick"/> − <see cref="long"/>
    /// (which advances a tick by a delta). Throws on overflow.
    /// </summary>
    public static long operator -(Tick later, Tick earlier) => checked(later._value - earlier._value);

    #endregion

    #region Equality + Comparison

    /// <inheritdoc />
    public bool Equals(Tick other) => _value == other._value;

    /// <inheritdoc />
    public override bool Equals(object? obj) => obj is Tick other && Equals(other);

    /// <inheritdoc />
    public override int GetHashCode() => _value.GetHashCode();

    /// <inheritdoc />
    public int CompareTo(Tick other) => _value.CompareTo(other._value);

    /// <inheritdoc />
    int IComparable.CompareTo(object? obj)
    {
        if (obj is null)
        {
            return 1;
        }
        if (obj is Tick other)
        {
            return CompareTo(other);
        }
        throw new ArgumentException("Object must be of type Tick.", nameof(obj));
    }

    public static bool operator ==(Tick left, Tick right) => left._value == right._value;

    public static bool operator !=(Tick left, Tick right) => left._value != right._value;

    public static bool operator <(Tick left, Tick right) => left._value < right._value;

    public static bool operator >(Tick left, Tick right) => left._value > right._value;

    public static bool operator <=(Tick left, Tick right) => left._value <= right._value;

    public static bool operator >=(Tick left, Tick right) => left._value >= right._value;

    #endregion

    /// <inheritdoc />
    public override string ToString() => _value.ToString(CultureInfo.InvariantCulture);
}

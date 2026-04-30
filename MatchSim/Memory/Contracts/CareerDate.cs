using System;

namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Save-world date stamp for <see cref="MemoryEvent"/> per ADR-0004
/// §"`MemoryEvent` schema" career-date field. Wall-clock time is NOT
/// used anywhere in MatchSim — events are timestamped against the
/// in-game career calendar so replays + memory queries are
/// platform-independent.
///
/// <para>
/// Week is 1..52, Day is 1..7 (constructor enforces both — there is
/// no valid default-initialized form because the all-zero shape would
/// claim "season 0, week 0, day 0" which has no in-world meaning).
/// Season is u16 to support 100+-year career replays without overflow.
/// Per pr-review-toolkit round-1 finding CV3: the previous "default
/// permitted" comment contradicted the constructor; fixed.
/// </para>
/// </summary>
public readonly struct CareerDate : IEquatable<CareerDate>
{
    public ushort Season { get; }
    public byte Week { get; }
    public byte Day { get; }

    public CareerDate(ushort season, byte week, byte day)
    {
        if (week < 1 || week > 52)
        {
            throw new ArgumentOutOfRangeException(
                nameof(week), week, "Week must be in [1, 52].");
        }
        if (day < 1 || day > 7)
        {
            throw new ArgumentOutOfRangeException(
                nameof(day), day, "Day must be in [1, 7].");
        }
        Season = season;
        Week = week;
        Day = day;
    }

    public bool Equals(CareerDate other) =>
        Season == other.Season && Week == other.Week && Day == other.Day;

    public override bool Equals(object? obj) => obj is CareerDate other && Equals(other);

    public override int GetHashCode() => HashCode.Combine(Season, Week, Day);

    public static bool operator ==(CareerDate left, CareerDate right) => left.Equals(right);
    public static bool operator !=(CareerDate left, CareerDate right) => !left.Equals(right);

    public override string ToString() => $"S{Season}W{Week}D{Day}";
}

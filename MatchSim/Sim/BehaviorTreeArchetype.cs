using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// A football "manager-archetype" tactical heuristic. Loaded from YAML; pure
/// data; consumed by <see cref="BehaviorTreeRunner"/>. Defines a base
/// formation (11 player base positions) + a few tactical knobs that drive
/// possession / press / build-up behavior. NOT a full behavior-tree library
/// with sequence/selector nodes — the design-doc framing of "BT archetype"
/// is shorthand for "manager-archetype tactical heuristic" per the Month-3
/// scope.
///
/// <para>
/// <strong>Coordinate convention</strong> (Home orientation): the formation
/// positions are authored from the perspective of a Home team defending the
/// <c>X = -52.5</c> goal. <see cref="BehaviorTreeRunner"/> mirrors X for
/// Away teams. Z is across-pitch.
/// </para>
/// </summary>
public sealed class BehaviorTreeArchetype
{
    /// <summary>Identifier. Used by <see cref="BehaviorTreeArchetypes.Load"/>.</summary>
    public string Name { get; }

    /// <summary>Authored description. Cosmetic only.</summary>
    public string Description { get; }

    /// <summary>11 base formation positions in Home orientation. Order matters — element <c>i</c> is the base position for the <c>i</c>-th player on the team's roster.</summary>
    public IReadOnlyList<FormationSlot> Formation { get; }

    /// <summary>
    /// Distance (metres) from the ball at which an off-the-ball player
    /// engages and presses. Higher = more aggressive pressing.
    /// </summary>
    public Fixed PressRadiusMetres { get; }

    /// <summary>
    /// Multiplier on <see cref="PlayerKinematics.MaxSpeed"/> when the
    /// archetype's team is in possession + advancing. Higher = faster
    /// counter-attacks; lower = patient build-up.
    /// </summary>
    public Fixed BuildupSpeedFactor { get; }

    /// <summary>Construct from validated authored data. Use <see cref="BehaviorTreeArchetypes.Parse"/> / <see cref="BehaviorTreeArchetypes.Load"/> for the canonical YAML path.</summary>
    public BehaviorTreeArchetype(
        string name,
        string description,
        IReadOnlyList<FormationSlot> formation,
        Fixed pressRadiusMetres,
        Fixed buildupSpeedFactor)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("Archetype name must be non-empty.", nameof(name));
        }
        if (formation is null || formation.Count != 11)
        {
            throw new ArgumentException($"Formation must contain exactly 11 slots; got {(formation is null ? "null" : formation.Count.ToString())}.", nameof(formation));
        }
        if (pressRadiusMetres <= Fixed.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(pressRadiusMetres), pressRadiusMetres, "PressRadius must be positive.");
        }
        if (buildupSpeedFactor <= Fixed.Zero)
        {
            throw new ArgumentOutOfRangeException(nameof(buildupSpeedFactor), buildupSpeedFactor, "BuildupSpeedFactor must be positive.");
        }
        ValidateFormationSlots(formation);

        Name = name;
        Description = description ?? string.Empty;
        // Defensive copy + read-only wrap so caller can't mutate after construction.
        FormationSlot[] copy = new FormationSlot[formation.Count];
        for (int i = 0; i < formation.Count; i++)
        {
            copy[i] = formation[i];
        }
        Formation = new ReadOnlyCollection<FormationSlot>(copy);
        PressRadiusMetres = pressRadiusMetres;
        BuildupSpeedFactor = buildupSpeedFactor;
    }

    private static void ValidateFormationSlots(IReadOnlyList<FormationSlot> formation)
    {
        // Roster slots must be unique to avoid accidental duplicates in
        // authored YAML.
        HashSet<byte> seenIds = new();
        foreach (FormationSlot slot in formation)
        {
            if (!seenIds.Add(slot.RosterSlot))
            {
                throw new ArgumentException($"Duplicate roster slot {slot.RosterSlot} in formation; each slot 1-11 appears exactly once.", nameof(formation));
            }
        }
        for (byte i = 1; i <= 11; i++)
        {
            if (!seenIds.Contains(i))
            {
                throw new ArgumentException($"Formation missing roster slot {i}; expected slots 1 through 11.", nameof(formation));
            }
        }
    }
}

/// <summary>
/// One position in a base formation. <see cref="RosterSlot"/> identifies
/// which player on the team's roster occupies this slot (1-11; matches
/// the player's index within the team array, NOT necessarily their jersey
/// number). <see cref="HomeBasePosition"/> is in Home orientation; the
/// runner mirrors X for Away teams.
/// </summary>
public readonly struct FormationSlot : IEquatable<FormationSlot>
{
    /// <summary>1-11 inclusive; uniquely identifies a roster slot.</summary>
    public readonly byte RosterSlot;

    /// <summary>Authored role label (e.g., "GK", "CB", "ST"). Display only — does NOT drive runner logic in Month-3 (that's a Phase-4 concern with role-subtype).</summary>
    public readonly string Role;

    /// <summary>Base position when off-the-ball and holding shape. In Home orientation.</summary>
    public readonly Vector3Fixed HomeBasePosition;

    /// <summary>Construct from explicit components. Validates roster slot range.</summary>
    public FormationSlot(byte rosterSlot, string role, Vector3Fixed homeBasePosition)
    {
        if (rosterSlot is < 1 or > 11)
        {
            throw new ArgumentOutOfRangeException(nameof(rosterSlot), rosterSlot, "RosterSlot must be in [1, 11].");
        }
        RosterSlot = rosterSlot;
        Role = role ?? string.Empty;
        HomeBasePosition = homeBasePosition;
    }

    /// <summary>
    /// Mirror this slot's base position across the X axis for Away usage.
    /// Returns a Vector3Fixed; the slot's <see cref="RosterSlot"/> + Role
    /// are unchanged.
    /// </summary>
    public Vector3Fixed AwayBasePosition() => new(-HomeBasePosition.X, HomeBasePosition.Y, HomeBasePosition.Z);

    /// <inheritdoc />
    public bool Equals(FormationSlot other)
        => RosterSlot == other.RosterSlot && Role == other.Role && HomeBasePosition.Equals(other.HomeBasePosition);

    /// <inheritdoc />
    public override bool Equals(object? obj) => obj is FormationSlot other && Equals(other);

    /// <inheritdoc />
    public override int GetHashCode()
    {
        unchecked
        {
            int h = 17;
            h = h * 31 + RosterSlot.GetHashCode();
            h = h * 31 + (Role?.GetHashCode() ?? 0);
            h = h * 31 + HomeBasePosition.GetHashCode();
            return h;
        }
    }

    public static bool operator ==(FormationSlot left, FormationSlot right) => left.Equals(right);

    public static bool operator !=(FormationSlot left, FormationSlot right) => !left.Equals(right);
}

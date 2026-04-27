using System;
using System.Globalization;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Q32.32 fixed-point 3D vector. Used for ball position / velocity / spin
/// (per <c>design/match-engine.md §Q2</c>) and forthcoming player kinematic
/// state. Determinism-clean: every operation is integer-math over the
/// underlying <see cref="Fixed"/> type, no float / double anywhere.
///
/// <para>
/// <strong>Coordinate convention</strong> (locked at v1, matches
/// <c>design/match-engine.md §Q2</c> ball-physics integrator): <c>X</c> +
/// <c>Z</c> form the pitch plane; <c>Y</c> is altitude (vertical, gravity
/// acts downward). All units are SI (metres for position, m/s for velocity).
/// </para>
///
/// <para>
/// <strong>Equality:</strong> bitwise on each <see cref="Fixed"/> component
/// (no epsilon tolerance). Fixed-point arithmetic is exact within range, so
/// epsilon comparisons are wrong here — they would mask determinism drift.
/// </para>
/// </summary>
public readonly struct Vector3Fixed : IEquatable<Vector3Fixed>
{
    /// <summary>Pitch-plane horizontal (locked alongside <see cref="Z"/>).</summary>
    public readonly Fixed X;

    /// <summary>Altitude (gravity acts on this component; ground is <c>Y &lt;= 0</c>).</summary>
    public readonly Fixed Y;

    /// <summary>Pitch-plane horizontal (locked alongside <see cref="X"/>).</summary>
    public readonly Fixed Z;

    /// <summary>Construct from three <see cref="Fixed"/> components.</summary>
    public Vector3Fixed(Fixed x, Fixed y, Fixed z)
    {
        X = x;
        Y = y;
        Z = z;
    }

    /// <summary>Zero vector. Default-constructible.</summary>
    public static Vector3Fixed Zero => default;

    /// <summary>Unit vector along +X.</summary>
    public static Vector3Fixed UnitX => new(Fixed.One, Fixed.Zero, Fixed.Zero);

    /// <summary>Unit vector along +Y (altitude up).</summary>
    public static Vector3Fixed UnitY => new(Fixed.Zero, Fixed.One, Fixed.Zero);

    /// <summary>Unit vector along +Z.</summary>
    public static Vector3Fixed UnitZ => new(Fixed.Zero, Fixed.Zero, Fixed.One);

    #region Operators

    /// <summary>Component-wise addition.</summary>
    public static Vector3Fixed operator +(Vector3Fixed a, Vector3Fixed b)
        => new(a.X + b.X, a.Y + b.Y, a.Z + b.Z);

    /// <summary>Component-wise subtraction.</summary>
    public static Vector3Fixed operator -(Vector3Fixed a, Vector3Fixed b)
        => new(a.X - b.X, a.Y - b.Y, a.Z - b.Z);

    /// <summary>Unary negation.</summary>
    public static Vector3Fixed operator -(Vector3Fixed v)
        => new(-v.X, -v.Y, -v.Z);

    /// <summary>Scalar multiplication (vector × scalar).</summary>
    public static Vector3Fixed operator *(Vector3Fixed v, Fixed s)
        => new(v.X * s, v.Y * s, v.Z * s);

    /// <summary>Scalar multiplication (scalar × vector).</summary>
    public static Vector3Fixed operator *(Fixed s, Vector3Fixed v)
        => new(v.X * s, v.Y * s, v.Z * s);

    #endregion

    #region Algebra

    /// <summary>
    /// Dot product <c>a · b = a.x*b.x + a.y*b.y + a.z*b.z</c>. Useful for
    /// projections, angle tests, signed-magnitude checks. No sqrt; safe for
    /// hot paths.
    /// </summary>
    public static Fixed Dot(Vector3Fixed a, Vector3Fixed b)
        => a.X * b.X + a.Y * b.Y + a.Z * b.Z;

    /// <summary>
    /// Cross product <c>a × b</c>. Used by Magnus force per
    /// <c>design/match-engine.md §Q2</c>: <c>F_m = C_m · (spin × velocity)</c>.
    /// Returns a vector perpendicular to both inputs.
    /// </summary>
    public static Vector3Fixed Cross(Vector3Fixed a, Vector3Fixed b)
        => new(
            a.Y * b.Z - a.Z * b.Y,
            a.Z * b.X - a.X * b.Z,
            a.X * b.Y - a.Y * b.X
        );

    /// <summary>
    /// Length-squared. Useful for "is this vector zero / near-zero" checks
    /// without taking a sqrt. <c>|v|² = v · v</c>.
    /// </summary>
    public Fixed LengthSquared() => Dot(this, this);

    /// <summary>
    /// Length (Euclidean magnitude). Uses <see cref="Fixed.Sqrt"/> — pay this
    /// only when actual length is needed; prefer <see cref="LengthSquared"/>
    /// for comparisons.
    /// </summary>
    public Fixed Length() => Fixed.Sqrt(LengthSquared());

    /// <summary>
    /// Distance between two vectors. <c>|a - b|</c>.
    /// </summary>
    public static Fixed Distance(Vector3Fixed a, Vector3Fixed b) => (a - b).Length();

    /// <summary>
    /// Distance-squared between two vectors. <c>|a - b|²</c>. Sqrt-free;
    /// preferred for radius / proximity comparisons (compare against
    /// <c>r²</c> instead of computing <see cref="Distance"/> and comparing
    /// against <c>r</c>).
    /// </summary>
    public static Fixed DistanceSquared(Vector3Fixed a, Vector3Fixed b) => (a - b).LengthSquared();

    /// <summary>
    /// Unit vector in the same direction. Throws on zero vector (no defined
    /// direction). Uses <see cref="Fixed.Sqrt"/>; the result is precise to
    /// Q32.32 floor and may be off by a few ULP from a true unit vector
    /// (verifying <c>Length(Normalize(v)) == 1</c> exactly is generally NOT
    /// possible in fixed-point — expect &lt; 2 Q32.32 ULP drift).
    /// </summary>
    public Vector3Fixed Normalize()
    {
        Fixed lenSq = LengthSquared();
        if (lenSq == Fixed.Zero)
        {
            throw new InvalidOperationException("Cannot normalize the zero vector.");
        }
        Fixed len = Fixed.Sqrt(lenSq);
        // Divide each component by length. Fixed division uses BigInteger
        // internally; for hot paths consider caching the inverse-length and
        // multiplying instead.
        return new Vector3Fixed(X / len, Y / len, Z / len);
    }

    #endregion

    #region Equality + ToString

    /// <inheritdoc />
    public bool Equals(Vector3Fixed other) => X.Equals(other.X) && Y.Equals(other.Y) && Z.Equals(other.Z);

    /// <inheritdoc />
    public override bool Equals(object? obj) => obj is Vector3Fixed other && Equals(other);

    /// <inheritdoc />
    public override int GetHashCode()
    {
        // Combine the three Fixed hashes deterministically. Order matters
        // (different permutations produce different hashes).
        unchecked
        {
            int h = 17;
            h = h * 31 + X.GetHashCode();
            h = h * 31 + Y.GetHashCode();
            h = h * 31 + Z.GetHashCode();
            return h;
        }
    }

    public static bool operator ==(Vector3Fixed left, Vector3Fixed right) => left.Equals(right);

    public static bool operator !=(Vector3Fixed left, Vector3Fixed right) => !left.Equals(right);

    /// <inheritdoc />
    public override string ToString()
        => string.Format(CultureInfo.InvariantCulture, "({0}, {1}, {2})", X, Y, Z);

    #endregion
}

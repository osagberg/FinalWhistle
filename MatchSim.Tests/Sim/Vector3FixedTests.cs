using System.Collections.Generic;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

public sealed class Vector3FixedTests
{
    private static Fixed F(int n) => Fixed.FromInt(n);

    #region Construction + components

    [Fact]
    public void Construct_PreservesComponents()
    {
        Vector3Fixed v = new(F(1), F(2), F(3));

        Assert.Equal(F(1), v.X);
        Assert.Equal(F(2), v.Y);
        Assert.Equal(F(3), v.Z);
    }

    [Fact]
    public void Default_IsZero()
    {
        Vector3Fixed v = default;

        Assert.Equal(Fixed.Zero, v.X);
        Assert.Equal(Fixed.Zero, v.Y);
        Assert.Equal(Fixed.Zero, v.Z);
        Assert.Equal(Vector3Fixed.Zero, v);
    }

    [Fact]
    public void UnitVectors_HaveSingleNonZeroComponent()
    {
        Assert.Equal(new Vector3Fixed(Fixed.One, Fixed.Zero, Fixed.Zero), Vector3Fixed.UnitX);
        Assert.Equal(new Vector3Fixed(Fixed.Zero, Fixed.One, Fixed.Zero), Vector3Fixed.UnitY);
        Assert.Equal(new Vector3Fixed(Fixed.Zero, Fixed.Zero, Fixed.One), Vector3Fixed.UnitZ);
    }

    #endregion

    #region Operators

    [Fact]
    public void Add_ComponentWise()
    {
        Vector3Fixed a = new(F(1), F(2), F(3));
        Vector3Fixed b = new(F(10), F(20), F(30));

        Vector3Fixed sum = a + b;

        Assert.Equal(new Vector3Fixed(F(11), F(22), F(33)), sum);
    }

    [Fact]
    public void Subtract_ComponentWise()
    {
        Vector3Fixed a = new(F(10), F(20), F(30));
        Vector3Fixed b = new(F(1), F(2), F(3));

        Vector3Fixed diff = a - b;

        Assert.Equal(new Vector3Fixed(F(9), F(18), F(27)), diff);
    }

    [Fact]
    public void UnaryMinus_NegatesAllComponents()
    {
        Vector3Fixed v = new(F(1), F(-2), F(3));

        Assert.Equal(new Vector3Fixed(F(-1), F(2), F(-3)), -v);
    }

    [Fact]
    public void ScalarMultiply_VectorTimesScalar()
    {
        Vector3Fixed v = new(F(1), F(2), F(3));

        Assert.Equal(new Vector3Fixed(F(2), F(4), F(6)), v * F(2));
    }

    [Fact]
    public void ScalarMultiply_ScalarTimesVector_SameAsVectorTimesScalar()
    {
        Vector3Fixed v = new(F(1), F(2), F(3));

        Assert.Equal(v * F(2), F(2) * v);
    }

    [Fact]
    public void ScalarMultiply_ByZero_GivesZero()
    {
        Vector3Fixed v = new(F(7), F(8), F(9));

        Assert.Equal(Vector3Fixed.Zero, v * Fixed.Zero);
    }

    [Fact]
    public void Add_IsCommutative()
    {
        Vector3Fixed a = new(F(1), F(2), F(3));
        Vector3Fixed b = new(F(4), F(5), F(6));

        Assert.Equal(a + b, b + a);
    }

    #endregion

    #region Dot product

    [Fact]
    public void Dot_OrthogonalVectors_ReturnsZero()
    {
        // x-axis · y-axis = 0
        Assert.Equal(Fixed.Zero, Vector3Fixed.Dot(Vector3Fixed.UnitX, Vector3Fixed.UnitY));
        Assert.Equal(Fixed.Zero, Vector3Fixed.Dot(Vector3Fixed.UnitY, Vector3Fixed.UnitZ));
        Assert.Equal(Fixed.Zero, Vector3Fixed.Dot(Vector3Fixed.UnitX, Vector3Fixed.UnitZ));
    }

    [Fact]
    public void Dot_ParallelVectors_ReturnsProductOfMagnitudes()
    {
        // (3,0,0) · (5,0,0) = 15
        Vector3Fixed a = new(F(3), Fixed.Zero, Fixed.Zero);
        Vector3Fixed b = new(F(5), Fixed.Zero, Fixed.Zero);

        Assert.Equal(F(15), Vector3Fixed.Dot(a, b));
    }

    [Fact]
    public void Dot_AntiParallel_ReturnsNegative()
    {
        // (3,0,0) · (-5,0,0) = -15
        Vector3Fixed a = new(F(3), Fixed.Zero, Fixed.Zero);
        Vector3Fixed b = new(F(-5), Fixed.Zero, Fixed.Zero);

        Assert.Equal(F(-15), Vector3Fixed.Dot(a, b));
    }

    [Fact]
    public void Dot_GeneralCase_SumsComponentProducts()
    {
        // (1,2,3) · (4,5,6) = 4 + 10 + 18 = 32
        Vector3Fixed a = new(F(1), F(2), F(3));
        Vector3Fixed b = new(F(4), F(5), F(6));

        Assert.Equal(F(32), Vector3Fixed.Dot(a, b));
    }

    [Fact]
    public void Dot_IsCommutative()
    {
        Vector3Fixed a = new(F(1), F(2), F(3));
        Vector3Fixed b = new(F(4), F(5), F(6));

        Assert.Equal(Vector3Fixed.Dot(a, b), Vector3Fixed.Dot(b, a));
    }

    #endregion

    #region Cross product

    [Fact]
    public void Cross_UnitX_UnitY_GivesUnitZ()
    {
        // x × y = z (right-handed coordinate system).
        Assert.Equal(Vector3Fixed.UnitZ, Vector3Fixed.Cross(Vector3Fixed.UnitX, Vector3Fixed.UnitY));
    }

    [Fact]
    public void Cross_UnitY_UnitZ_GivesUnitX()
    {
        Assert.Equal(Vector3Fixed.UnitX, Vector3Fixed.Cross(Vector3Fixed.UnitY, Vector3Fixed.UnitZ));
    }

    [Fact]
    public void Cross_UnitZ_UnitX_GivesUnitY()
    {
        Assert.Equal(Vector3Fixed.UnitY, Vector3Fixed.Cross(Vector3Fixed.UnitZ, Vector3Fixed.UnitX));
    }

    [Fact]
    public void Cross_IsAntiCommutative()
    {
        // a × b = -(b × a)
        Vector3Fixed a = new(F(1), F(2), F(3));
        Vector3Fixed b = new(F(4), F(5), F(6));

        Assert.Equal(-Vector3Fixed.Cross(b, a), Vector3Fixed.Cross(a, b));
    }

    [Fact]
    public void Cross_ParallelVectors_GivesZero()
    {
        // a × a = 0 always.
        Vector3Fixed a = new(F(1), F(2), F(3));

        Assert.Equal(Vector3Fixed.Zero, Vector3Fixed.Cross(a, a));
    }

    [Fact]
    public void Cross_GeneralCase_MatchesFormula()
    {
        // (1,2,3) × (4,5,6):
        //   x = 2*6 - 3*5 = 12 - 15 = -3
        //   y = 3*4 - 1*6 = 12 - 6 = 6
        //   z = 1*5 - 2*4 = 5 - 8 = -3
        Vector3Fixed a = new(F(1), F(2), F(3));
        Vector3Fixed b = new(F(4), F(5), F(6));

        Assert.Equal(new Vector3Fixed(F(-3), F(6), F(-3)), Vector3Fixed.Cross(a, b));
    }

    [Fact]
    public void Cross_ResultIsOrthogonalToBothInputs()
    {
        // (a × b) · a = 0 and (a × b) · b = 0.
        Vector3Fixed a = new(F(1), F(2), F(3));
        Vector3Fixed b = new(F(4), F(5), F(6));

        Vector3Fixed c = Vector3Fixed.Cross(a, b);
        Assert.Equal(Fixed.Zero, Vector3Fixed.Dot(c, a));
        Assert.Equal(Fixed.Zero, Vector3Fixed.Dot(c, b));
    }

    #endregion

    #region LengthSquared

    [Fact]
    public void LengthSquared_OfZero_IsZero()
    {
        Assert.Equal(Fixed.Zero, Vector3Fixed.Zero.LengthSquared());
    }

    [Fact]
    public void LengthSquared_UnitVector_IsOne()
    {
        Assert.Equal(Fixed.One, Vector3Fixed.UnitX.LengthSquared());
        Assert.Equal(Fixed.One, Vector3Fixed.UnitY.LengthSquared());
        Assert.Equal(Fixed.One, Vector3Fixed.UnitZ.LengthSquared());
    }

    [Fact]
    public void LengthSquared_GeneralCase_SumsComponentSquares()
    {
        // |(3,4,0)|² = 9 + 16 + 0 = 25
        Vector3Fixed v = new(F(3), F(4), F(0));

        Assert.Equal(F(25), v.LengthSquared());
    }

    #endregion

    #region Equality + ToString

    [Fact]
    public void Equality_SameComponents_AreEqual()
    {
        Vector3Fixed a = new(F(1), F(2), F(3));
        Vector3Fixed b = new(F(1), F(2), F(3));

        Assert.True(a.Equals(b));
        Assert.True(a == b);
        Assert.False(a != b);
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
    }

    [Fact]
    public void Equality_DifferentComponents_AreNotEqual()
    {
        Vector3Fixed a = new(F(1), F(2), F(3));
        Vector3Fixed b = new(F(1), F(2), F(4));

        Assert.False(a.Equals(b));
        Assert.True(a != b);
    }

    [Fact]
    public void GetHashCode_SwappedComponents_DifferentHash()
    {
        // Order matters in the hash combine — (1,2,3) ≠ (3,2,1).
        Vector3Fixed a = new(F(1), F(2), F(3));
        Vector3Fixed b = new(F(3), F(2), F(1));

        Assert.NotEqual(a, b);
        Assert.NotEqual(a.GetHashCode(), b.GetHashCode());
    }

    [Fact]
    public void GetHashCode_DistinctVectors_DistributedHashes()
    {
        // Sanity: 100 distinct vectors produce many (>50) distinct hashes.
        // Not a perfection test; just catches a constant-hash regression.
        HashSet<int> hashes = new();
        for (int i = 0; i < 100; i++)
        {
            hashes.Add(new Vector3Fixed(F(i), F(i * 2), F(i * 3)).GetHashCode());
        }
        Assert.True(hashes.Count > 50, $"Expected reasonable hash distribution; got {hashes.Count} distinct.");
    }

    [Fact]
    public void ToString_FormatsAsParenthesizedTuple()
    {
        Vector3Fixed v = new(F(1), F(2), F(3));

        // Components serialize via Fixed.ToString (canonical decimal).
        Assert.Equal($"({F(1)}, {F(2)}, {F(3)})", v.ToString());
    }

    [Fact]
    public void ToString_UsesInvariantCulture()
    {
        // Negative + non-integer values; verifies no current-culture decimal
        // separator drift.
        Vector3Fixed v = new(F(-1), Fixed.Half, F(0));
        string s = v.ToString();

        // Comma-separated invariant format; no semicolon (which some locales
        // would substitute).
        Assert.Contains(",", s);
        Assert.DoesNotContain(";", s);
    }

    #endregion
}

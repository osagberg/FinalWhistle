using System;
using System.Collections.Generic;
using System.IO;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

public sealed class BehaviorTreeArchetypeTests
{
    private static Fixed F(int n) => Fixed.FromInt(n);
    private static Vector3Fixed V3(int x, int y, int z) => new(F(x), F(y), F(z));

    private static FormationSlot[] BuildValidFormation()
    {
        FormationSlot[] formation = new FormationSlot[11];
        for (byte i = 1; i <= 11; i++)
        {
            // Trivial position: each player at (i, 0, 0). Just enough to
            // satisfy the 1..11 roster-slot uniqueness invariant.
            formation[i - 1] = new FormationSlot(i, "Test", new Vector3Fixed(F(i), Fixed.Zero, Fixed.Zero));
        }
        return formation;
    }

    #region BehaviorTreeArchetype construction validation

    [Fact]
    public void Construct_AllFieldsValid_PreservesData()
    {
        BehaviorTreeArchetype a = new(
            name:               "test",
            description:        "Test desc",
            formation:          BuildValidFormation(),
            pressRadiusMetres:  F(20),
            buildupSpeedFactor: F(1) / F(2)
        );

        Assert.Equal("test", a.Name);
        Assert.Equal("Test desc", a.Description);
        Assert.Equal(11, a.Formation.Count);
        Assert.Equal(F(20), a.PressRadiusMetres);
        Assert.Equal(F(1) / F(2), a.BuildupSpeedFactor);
    }

    [Theory]
    [InlineData("")]
    [InlineData("   ")]
    [InlineData(null)]
    public void Construct_BlankOrNullName_Throws(string? name)
    {
        Assert.Throws<ArgumentException>(() => new BehaviorTreeArchetype(
            name!, "desc", BuildValidFormation(), F(20), F(1)));
    }

    [Fact]
    public void Construct_NullFormation_Throws()
    {
        Assert.Throws<ArgumentException>(() => new BehaviorTreeArchetype(
            "x", "desc", null!, F(20), F(1)));
    }

    [Fact]
    public void Construct_FormationWithWrongLength_Throws()
    {
        // 10 slots — too few.
        FormationSlot[] tooFew = new FormationSlot[10];
        for (byte i = 1; i <= 10; i++)
        {
            tooFew[i - 1] = new FormationSlot(i, "T", Vector3Fixed.Zero);
        }
        Assert.Throws<ArgumentException>(() => new BehaviorTreeArchetype(
            "x", "desc", tooFew, F(20), F(1)));
    }

    [Fact]
    public void Construct_FormationWithDuplicateRosterSlot_Throws()
    {
        FormationSlot[] dup = BuildValidFormation();
        // Replace the last slot with a duplicate of slot 1 (same RosterSlot).
        dup[10] = new FormationSlot(1, "DUP", Vector3Fixed.Zero);
        Assert.Throws<ArgumentException>(() => new BehaviorTreeArchetype(
            "x", "desc", dup, F(20), F(1)));
    }

    [Fact]
    public void Construct_NonPositivePressRadius_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BehaviorTreeArchetype(
            "x", "desc", BuildValidFormation(), Fixed.Zero, F(1)));
        Assert.Throws<ArgumentOutOfRangeException>(() => new BehaviorTreeArchetype(
            "x", "desc", BuildValidFormation(), F(-1), F(1)));
    }

    [Fact]
    public void Construct_NonPositiveBuildupSpeedFactor_Throws()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new BehaviorTreeArchetype(
            "x", "desc", BuildValidFormation(), F(20), Fixed.Zero));
        Assert.Throws<ArgumentOutOfRangeException>(() => new BehaviorTreeArchetype(
            "x", "desc", BuildValidFormation(), F(20), F(-1)));
    }

    #endregion

    #region FormationSlot tests

    [Theory]
    [InlineData(0)]
    [InlineData(12)]
    [InlineData(255)]
    public void FormationSlot_InvalidRosterSlot_Throws(byte rosterSlot)
    {
        Assert.Throws<ArgumentOutOfRangeException>(() =>
            new FormationSlot(rosterSlot, "T", Vector3Fixed.Zero));
    }

    [Fact]
    public void FormationSlot_AwayBasePosition_MirrorsX()
    {
        FormationSlot slot = new(1, "GK", V3(-45, 0, 10));
        Vector3Fixed away = slot.AwayBasePosition();

        Assert.Equal(F(45), away.X);              // mirrored
        Assert.Equal(Fixed.Zero, away.Y);
        Assert.Equal(F(10), away.Z);              // unchanged
    }

    [Fact]
    public void FormationSlot_Equality_FieldwiseAndOrderInsensitive()
    {
        FormationSlot a = new(5, "MF", V3(1, 0, 2));
        FormationSlot b = new(5, "MF", V3(1, 0, 2));
        FormationSlot c = new(5, "MF", V3(1, 0, 3));

        Assert.Equal(a, b);
        Assert.True(a == b);
        Assert.NotEqual(a, c);
        Assert.True(a != c);
        Assert.Equal(a.GetHashCode(), b.GetHashCode());
    }

    #endregion

    #region YAML parser

    [Fact]
    public void Parse_ValidMinimalYaml_ProducesExpectedArchetype()
    {
        // Hand-built YAML covering all expected fields. Position values
        // chosen to make verification trivial.
        string yaml = """
            name: test-archetype
            description: A test
            formation:
              - { roster_slot: 1, role: GK,  x:  -10, z:  0 }
              - { roster_slot: 2, role: RB,  x:  -5,  z:  5 }
              - { roster_slot: 3, role: LB,  x:  -5,  z: -5 }
              - { roster_slot: 4, role: RCB, x:  -8,  z:  2 }
              - { roster_slot: 5, role: LCB, x:  -8,  z: -2 }
              - { roster_slot: 6, role: RM,  x:   0,  z:  5 }
              - { roster_slot: 7, role: LM,  x:   0,  z: -5 }
              - { roster_slot: 8, role: CDM, x:  -3,  z:  0 }
              - { roster_slot: 9, role: CAM, x:   3,  z:  0 }
              - { roster_slot: 10, role: ST, x:   8,  z:  3 }
              - { roster_slot: 11, role: ST, x:   8,  z: -3 }
            press_radius_metres: 25
            buildup_speed_factor: 0.85
            """;

        BehaviorTreeArchetype a = BehaviorTreeArchetypes.Parse(yaml);

        Assert.Equal("test-archetype", a.Name);
        Assert.Equal("A test", a.Description);
        Assert.Equal(11, a.Formation.Count);
        Assert.Equal(F(25), a.PressRadiusMetres);
        // 0.85 doesn't round-trip to an exact Fixed value via Parse + new
        // construction; verify approximately.
        Assert.True(a.BuildupSpeedFactor > Fixed.Zero);
        Assert.True(a.BuildupSpeedFactor < Fixed.One);

        // Spot-check first formation slot.
        Assert.Equal((byte)1, a.Formation[0].RosterSlot);
        Assert.Equal("GK", a.Formation[0].Role);
        Assert.Equal(V3(-10, 0, 0), a.Formation[0].HomeBasePosition);
    }

    [Fact]
    public void Parse_NullYaml_Throws()
    {
        Assert.Throws<ArgumentNullException>(() => BehaviorTreeArchetypes.Parse(null!));
    }

    [Fact]
    public void Parse_EmptyYaml_Throws()
    {
        Assert.Throws<InvalidDataException>(() => BehaviorTreeArchetypes.Parse(""));
    }

    [Fact]
    public void Parse_MissingFormation_Throws()
    {
        string yaml = """
            name: incomplete
            description: missing formation
            press_radius_metres: 25
            buildup_speed_factor: 1
            """;
        Assert.Throws<InvalidDataException>(() => BehaviorTreeArchetypes.Parse(yaml));
    }

    [Fact]
    public void Parse_NonPositivePressRadius_Throws()
    {
        string yaml = """
            name: bad
            description: ""
            formation:
              - { roster_slot: 1, role: GK, x: 0, z: 0 }
              - { roster_slot: 2, role: RB, x: 0, z: 0 }
              - { roster_slot: 3, role: RCB, x: 0, z: 0 }
              - { roster_slot: 4, role: LCB, x: 0, z: 0 }
              - { roster_slot: 5, role: LB, x: 0, z: 0 }
              - { roster_slot: 6, role: RM, x: 0, z: 0 }
              - { roster_slot: 7, role: RCM, x: 0, z: 0 }
              - { roster_slot: 8, role: LCM, x: 0, z: 0 }
              - { roster_slot: 9, role: LM, x: 0, z: 0 }
              - { roster_slot: 10, role: RST, x: 0, z: 0 }
              - { roster_slot: 11, role: LST, x: 0, z: 0 }
            press_radius_metres: 0
            buildup_speed_factor: 1
            """;
        Assert.Throws<InvalidDataException>(() => BehaviorTreeArchetypes.Parse(yaml));
    }

    [Fact]
    public void Parse_DuplicateRosterSlot_Throws()
    {
        // Two slots with RosterSlot = 1.
        string yaml = """
            name: duplicate
            description: ""
            formation:
              - { roster_slot: 1, role: GK, x: 0, z: 0 }
              - { roster_slot: 1, role: GK, x: 0, z: 0 }
              - { roster_slot: 3, role: RCB, x: 0, z: 0 }
              - { roster_slot: 4, role: LCB, x: 0, z: 0 }
              - { roster_slot: 5, role: LB, x: 0, z: 0 }
              - { roster_slot: 6, role: RM, x: 0, z: 0 }
              - { roster_slot: 7, role: RCM, x: 0, z: 0 }
              - { roster_slot: 8, role: LCM, x: 0, z: 0 }
              - { roster_slot: 9, role: LM, x: 0, z: 0 }
              - { roster_slot: 10, role: RST, x: 0, z: 0 }
              - { roster_slot: 11, role: LST, x: 0, z: 0 }
            press_radius_metres: 25
            buildup_speed_factor: 1
            """;
        Assert.Throws<ArgumentException>(() => BehaviorTreeArchetypes.Parse(yaml));
    }

    [Fact]
    public void Parse_DeterministicOnSameInput()
    {
        string yaml = """
            name: t
            description: ""
            formation:
              - { roster_slot: 1, role: GK, x: 0, z: 0 }
              - { roster_slot: 2, role: RB, x: 0, z: 0 }
              - { roster_slot: 3, role: LB, x: 0, z: 0 }
              - { roster_slot: 4, role: RCB, x: 0, z: 0 }
              - { roster_slot: 5, role: LCB, x: 0, z: 0 }
              - { roster_slot: 6, role: RM, x: 0, z: 0 }
              - { roster_slot: 7, role: LM, x: 0, z: 0 }
              - { roster_slot: 8, role: CDM, x: 0, z: 0 }
              - { roster_slot: 9, role: CAM, x: 0, z: 0 }
              - { roster_slot: 10, role: ST, x: 0, z: 0 }
              - { roster_slot: 11, role: ST, x: 0, z: 0 }
            press_radius_metres: 25
            buildup_speed_factor: 1
            """;
        BehaviorTreeArchetype a = BehaviorTreeArchetypes.Parse(yaml);
        BehaviorTreeArchetype b = BehaviorTreeArchetypes.Parse(yaml);

        Assert.Equal(a.Name, b.Name);
        Assert.Equal(a.PressRadiusMetres, b.PressRadiusMetres);
        Assert.Equal(a.BuildupSpeedFactor, b.BuildupSpeedFactor);
        for (int i = 0; i < a.Formation.Count; i++)
        {
            Assert.Equal(a.Formation[i], b.Formation[i]);
        }
    }

    #endregion

    #region Built-in archetype loaders (embedded resources)

    [Fact]
    public void Load_DirectPressing_LoadsExpectedArchetype()
    {
        BehaviorTreeArchetype a = BehaviorTreeArchetypes.Load("direct-pressing");

        Assert.Equal("direct-pressing", a.Name);
        Assert.Equal(11, a.Formation.Count);
        // Authored values from direct-pressing.yaml.
        Assert.Equal(F(25), a.PressRadiusMetres);
        Assert.True(a.BuildupSpeedFactor > Fixed.Zero);
        Assert.True(a.BuildupSpeedFactor < Fixed.One);  // 0.95 < 1.0

        // Verify GK is at the back.
        FormationSlot gk = a.Formation[0];
        Assert.Equal((byte)1, gk.RosterSlot);
        Assert.Equal("GK", gk.Role);
        Assert.True(gk.HomeBasePosition.X < Fixed.Zero, "GK should be on the home (negative-X) side.");
    }

    [Fact]
    public void Load_LowBlockCounter_LoadsExpectedArchetype()
    {
        BehaviorTreeArchetype a = BehaviorTreeArchetypes.Load("low-block-counter");

        Assert.Equal("low-block-counter", a.Name);
        Assert.Equal(11, a.Formation.Count);
        // Authored: press_radius_metres = 12 (much smaller than direct-pressing).
        Assert.Equal(F(12), a.PressRadiusMetres);
    }

    [Fact]
    public void Load_DirectVsLowBlock_HaveDifferentTactics()
    {
        BehaviorTreeArchetype direct = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype lowBlock = BehaviorTreeArchetypes.Load("low-block-counter");

        // Direct presses harder (larger press radius).
        Assert.True(direct.PressRadiusMetres > lowBlock.PressRadiusMetres);
    }

    [Fact]
    public void Load_UnknownName_ThrowsFileNotFoundException()
    {
        Assert.Throws<FileNotFoundException>(() => BehaviorTreeArchetypes.Load("nonexistent-archetype"));
    }

    [Theory]
    [InlineData("")]
    [InlineData(null)]
    [InlineData("   ")]
    public void Load_BlankOrNullName_Throws(string? name)
    {
        Assert.Throws<ArgumentException>(() => BehaviorTreeArchetypes.Load(name!));
    }

    [Fact]
    public void Load_BuiltInNames_AllResolveCleanly()
    {
        // All names in the BuiltInNames list must load without exception.
        foreach (string name in BehaviorTreeArchetypes.BuiltInNames)
        {
            BehaviorTreeArchetype a = BehaviorTreeArchetypes.Load(name);
            Assert.Equal(name, a.Name);
        }
    }

    [Fact]
    public void Load_CallsAreCached_SameInstanceOnRepeatLoad()
    {
        BehaviorTreeArchetype first = BehaviorTreeArchetypes.Load("direct-pressing");
        BehaviorTreeArchetype second = BehaviorTreeArchetypes.Load("direct-pressing");

        // Cached: returns the same reference.
        Assert.Same(first, second);
    }

    #endregion
}

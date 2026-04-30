using System.IO;
using FinalWhistle.MatchSim.Content;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Content;

/// <summary>
/// Strict-parsing tests for the hand-rolled IdentityPacketParser per
/// Codex round-7 (2026-04-30). Closes:
/// <list type="bullet">
///   <item><description>P1#2 — typoed gene field silently parsing as 0
///       (default-int-zero with no presence check); now rejected as
///       unknown field.</description></item>
///   <item><description>P1#2 — missing required field silently parsing
///       as default; now rejected with descriptive error.</description></item>
///   <item><description>P1#2 — unknown top-level field silently ignored
///       by STJ default options; now rejected.</description></item>
///   <item><description>P2 — numeric RoleFamily encoding silently accepted
///       by JsonStringEnumConverter default <c>allowIntegerValues=true</c>;
///       now rejected (string-only).</description></item>
/// </list>
///
/// All tests assert <see cref="InvalidDataException"/> with descriptive
/// messages; the parser is designed to surface fixture-authoring bugs
/// rather than silently corrupt sim state.
/// </summary>
public sealed class IdentityPacketParserStrictTests
{
    // --- P1#2: typoed gene field name ---

    [Fact]
    public void Parse_TypoedGeneField_Rejected()
    {
        // FastTwichRawQ32 (typo) instead of FastTwitchRawQ32 — under STJ
        // default options this would parse as 0 silently (the default-int
        // value), corrupting signature-affinity dispatch. Strict parser
        // rejects.
        string json = ValidPacketJsonWithGenes(@"{
            ""FastTwichRawQ32"": 2576980377,
            ""PatternRecognitionRawQ32"": 3006477107,
            ""DecisionVelocityRawQ32"": 3006477107,
            ""FirstTouchRawQ32"": 2147483648,
            ""StrikingRawQ32"": 2147483648,
            ""LeftFootRawQ32"": 2147483648
        }");

        var ex = Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
        Assert.Contains("FastTwichRawQ32", ex.Message);
        Assert.Contains("unknown", ex.Message, System.StringComparison.OrdinalIgnoreCase);
    }

    // --- P1#2: missing required gene field ---

    [Fact]
    public void Parse_MissingFastTwitchGene_Rejected()
    {
        string json = ValidPacketJsonWithGenes(@"{
            ""PatternRecognitionRawQ32"": 3006477107,
            ""DecisionVelocityRawQ32"": 3006477107,
            ""FirstTouchRawQ32"": 2147483648,
            ""StrikingRawQ32"": 2147483648,
            ""LeftFootRawQ32"": 2147483648
        }");

        var ex = Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
        Assert.Contains("FastTwitchRawQ32", ex.Message);
        Assert.Contains("missing", ex.Message, System.StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Parse_MissingPatternRecognitionGene_Rejected()
    {
        string json = ValidPacketJsonWithGenes(@"{
            ""FastTwitchRawQ32"": 2576980377,
            ""DecisionVelocityRawQ32"": 3006477107,
            ""FirstTouchRawQ32"": 2147483648,
            ""StrikingRawQ32"": 2147483648,
            ""LeftFootRawQ32"": 2147483648
        }");
        var ex = Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
        Assert.Contains("PatternRecognitionRawQ32", ex.Message);
    }

    // --- P1#2: missing required top-level field ---

    [Fact]
    public void Parse_MissingPlayerId_Rejected()
    {
        // Build a JSON that has every field except PlayerId. STJ default
        // would parse PlayerId as null + the validator would catch the
        // empty string later, but the parser-level miss is earlier and
        // points more directly at the fixture-authoring bug.
        string json = @"{
            ""DisplayNameFull"": ""Test Player"",
            ""DisplayNameShort"": ""T. Player"",
            ""RoleFamily"": ""CentralMidfielder"",
            ""SignatureCandidates"": [],
            ""Genes"": " + ValidGenesJson() + @",
            ""SchemaVersion"": 1,
            ""SourcePackVersion"": ""1.0.0""
        }";

        var ex = Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
        Assert.Contains("PlayerId", ex.Message);
    }

    // --- P1#2: unknown top-level field ---

    [Fact]
    public void Parse_UnknownTopLevelField_Rejected()
    {
        string json = @"{
            ""PlayerId"": ""fwh.core:player_99999"",
            ""DisplayNameFull"": ""Test Player"",
            ""DisplayNameShort"": ""T. Player"",
            ""RoleFamily"": ""CentralMidfielder"",
            ""SignatureCandidates"": [],
            ""Genes"": " + ValidGenesJson() + @",
            ""SchemaVersion"": 1,
            ""SourcePackVersion"": ""1.0.0"",
            ""ExtraFutureField"": ""should be rejected at Phase-3""
        }";

        var ex = Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
        Assert.Contains("ExtraFutureField", ex.Message);
        Assert.Contains("unknown", ex.Message, System.StringComparison.OrdinalIgnoreCase);
    }

    // --- P2: numeric RoleFamily encoding ---

    [Fact]
    public void Parse_NumericRoleFamily_Rejected()
    {
        // STJ's JsonStringEnumConverter defaults to allowIntegerValues=true,
        // which silently accepts `"RoleFamily": 7` and parses it as Winger.
        // The strict parser rejects: string only.
        string json = @"{
            ""PlayerId"": ""fwh.core:player_99999"",
            ""DisplayNameFull"": ""Test Player"",
            ""DisplayNameShort"": ""T. Player"",
            ""RoleFamily"": 7,
            ""SignatureCandidates"": [],
            ""Genes"": " + ValidGenesJson() + @",
            ""SchemaVersion"": 1,
            ""SourcePackVersion"": ""1.0.0""
        }";

        var ex = Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
        Assert.Contains("RoleFamily", ex.Message);
        Assert.Contains("string", ex.Message, System.StringComparison.OrdinalIgnoreCase);
    }

    [Fact]
    public void Parse_UnknownRoleFamilyString_Rejected()
    {
        string json = @"{
            ""PlayerId"": ""fwh.core:player_99999"",
            ""DisplayNameFull"": ""Test Player"",
            ""DisplayNameShort"": ""T. Player"",
            ""RoleFamily"": ""SuperStriker"",
            ""SignatureCandidates"": [],
            ""Genes"": " + ValidGenesJson() + @",
            ""SchemaVersion"": 1,
            ""SourcePackVersion"": ""1.0.0""
        }";

        var ex = Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
        Assert.Contains("RoleFamily", ex.Message);
    }

    // --- Defensive: duplicate top-level key ---

    [Fact]
    public void Parse_DuplicateTopLevelKey_Rejected()
    {
        string json = @"{
            ""PlayerId"": ""fwh.core:player_99999"",
            ""PlayerId"": ""fwh.core:player_99998"",
            ""DisplayNameFull"": ""Test Player"",
            ""DisplayNameShort"": ""T. Player"",
            ""RoleFamily"": ""CentralMidfielder"",
            ""SignatureCandidates"": [],
            ""Genes"": " + ValidGenesJson() + @",
            ""SchemaVersion"": 1,
            ""SourcePackVersion"": ""1.0.0""
        }";

        var ex = Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
        Assert.Contains("duplicate", ex.Message, System.StringComparison.OrdinalIgnoreCase);
        Assert.Contains("PlayerId", ex.Message);
    }

    // --- Defensive: numeric leading zero ---

    [Fact]
    public void Parse_NumberWithLeadingZeros_Rejected()
    {
        string json = @"{
            ""PlayerId"": ""fwh.core:player_99999"",
            ""DisplayNameFull"": ""Test Player"",
            ""DisplayNameShort"": ""T. Player"",
            ""RoleFamily"": ""CentralMidfielder"",
            ""SignatureCandidates"": [],
            ""Genes"": " + ValidGenesJson() + @",
            ""SchemaVersion"": 01,
            ""SourcePackVersion"": ""1.0.0""
        }";

        var ex = Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
        Assert.Contains("leading-zero", ex.Message);
    }

    // --- Defensive: float in numeric field ---

    [Fact]
    public void Parse_FloatInNumericField_Rejected()
    {
        string json = ValidPacketJsonWithGenes(@"{
            ""FastTwitchRawQ32"": 2576980377.5,
            ""PatternRecognitionRawQ32"": 3006477107,
            ""DecisionVelocityRawQ32"": 3006477107,
            ""FirstTouchRawQ32"": 2147483648,
            ""StrikingRawQ32"": 2147483648,
            ""LeftFootRawQ32"": 2147483648
        }");

        var ex = Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
        Assert.Contains("decimal", ex.Message, System.StringComparison.OrdinalIgnoreCase);
    }

    // --- Defensive: trailing content after close-brace ---

    [Fact]
    public void Parse_TrailingContentAfterClose_Rejected()
    {
        string json = @"{
            ""PlayerId"": ""fwh.core:player_99999"",
            ""DisplayNameFull"": ""Test Player"",
            ""DisplayNameShort"": ""T. Player"",
            ""RoleFamily"": ""CentralMidfielder"",
            ""SignatureCandidates"": [],
            ""Genes"": " + ValidGenesJson() + @",
            ""SchemaVersion"": 1,
            ""SourcePackVersion"": ""1.0.0""
        } extra-junk-here";

        Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(json));
    }

    // --- Helper: build a complete valid packet with overridable Genes block ---

    private static string ValidPacketJsonWithGenes(string genesJsonObject) => @"{
        ""PlayerId"": ""fwh.core:player_99999"",
        ""DisplayNameFull"": ""Test Player"",
        ""DisplayNameShort"": ""T. Player"",
        ""RoleFamily"": ""CentralMidfielder"",
        ""SignatureCandidates"": [],
        ""Genes"": " + genesJsonObject + @",
        ""SchemaVersion"": 1,
        ""SourcePackVersion"": ""1.0.0""
    }";

    private static string ValidGenesJson() => @"{
        ""FastTwitchRawQ32"": 2576980377,
        ""PatternRecognitionRawQ32"": 3006477107,
        ""DecisionVelocityRawQ32"": 3006477107,
        ""FirstTouchRawQ32"": 2147483648,
        ""StrikingRawQ32"": 2147483648,
        ""LeftFootRawQ32"": 2147483648
    }";
}

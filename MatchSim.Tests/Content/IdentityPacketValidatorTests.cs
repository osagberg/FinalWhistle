using System.IO;
using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Content;

/// <summary>
/// Validator-direct tests per ADR-0006 §validation. Bypass the embedded-
/// resource loader to exercise validator-rejection branches independently.
/// </summary>
public sealed class IdentityPacketValidatorTests
{
    [Fact]
    public void Validate_ValidPacket_ReturnsValid()
    {
        var packet = MakeValidPacket();
        var result = IdentityPacketValidator.Validate(packet);
        Assert.True(result.IsValid, $"Errors: {string.Join("; ", result.Errors)}");
    }

    [Fact]
    public void Validate_AllBuiltInFixtures_ReturnValid()
    {
        // Every embedded fixture must pass the validator. This is the
        // proof that the hand-authored 22 packets are all conforming.
        foreach (var packet in IdentityPackets.LoadAll())
        {
            var result = IdentityPacketValidator.Validate(packet);
            Assert.True(result.IsValid,
                $"Packet {packet.PlayerId} failed validation: " +
                $"{string.Join("; ", result.Errors)}");
        }
    }

    [Fact]
    public void Validate_NullPacket_ReturnsInvalid()
    {
        var result = IdentityPacketValidator.Validate(null!);
        Assert.False(result.IsValid);
        Assert.Single(result.Errors);
    }

    [Fact]
    public void Validate_PlayerIdWithPackMinor_ReturnsInvalid()
    {
        // ADR-0006 §content-pack-ID-rules: pack-minor versions NEVER
        // appear in entity IDs. fwh.core.v1.1:player_00001 must reject.
        var packet = MakeValidPacket() with { PlayerId = "fwh.core.v1.1:player_00001" };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.Contains("PlayerId"));
    }

    [Fact]
    public void Validate_PlayerIdWithCapitalLetters_ReturnsInvalid()
    {
        // Lowercase-only per the regex.
        var packet = MakeValidPacket() with { PlayerId = "FWH.CORE:player_00001" };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
    }

    [Fact]
    public void Validate_PlayerIdMissingPlayerSuffix_ReturnsInvalid()
    {
        var packet = MakeValidPacket() with { PlayerId = "fwh.core:00001" };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
    }

    [Fact]
    public void Validate_AffinityWeightAboveOne_ReturnsInvalid()
    {
        var packet = MakeValidPacket() with
        {
            SignatureCandidates = new[]
            {
                new SignatureCandidate
                {
                    SignatureId = "fwh.core:signature.low-cutback-from-byline",
                    AffinityWeightRaw = Fixed.One.RawValue + 1L,
                },
            },
        };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.Contains("AffinityWeightRaw"));
    }

    [Fact]
    public void Validate_AffinityWeightNegative_ReturnsInvalid()
    {
        var packet = MakeValidPacket() with
        {
            SignatureCandidates = new[]
            {
                new SignatureCandidate
                {
                    SignatureId = "fwh.core:signature.low-cutback-from-byline",
                    AffinityWeightRaw = -1L,
                },
            },
        };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
    }

    [Fact]
    public void Validate_FourSignatureCandidates_ReturnsInvalid()
    {
        // ADR-0006 §affinity-count-distribution caps at 3.
        var packet = MakeValidPacket() with
        {
            SignatureCandidates = new[]
            {
                new SignatureCandidate { SignatureId = "fwh.core:signature.a", AffinityWeightRaw = 0L },
                new SignatureCandidate { SignatureId = "fwh.core:signature.b", AffinityWeightRaw = 0L },
                new SignatureCandidate { SignatureId = "fwh.core:signature.c", AffinityWeightRaw = 0L },
                new SignatureCandidate { SignatureId = "fwh.core:signature.d", AffinityWeightRaw = 0L },
            },
        };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.Contains("count 4"));
    }

    [Fact]
    public void Validate_EmptyDisplayNameFull_ReturnsInvalid()
    {
        var packet = MakeValidPacket() with { DisplayNameFull = "" };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
    }

    [Fact]
    public void Validate_WhitespaceDisplayNameShort_ReturnsInvalid()
    {
        var packet = MakeValidPacket() with { DisplayNameShort = "   " };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
    }

    [Fact]
    public void Validate_SchemaVersionMismatch_ReturnsInvalid()
    {
        var packet = MakeValidPacket() with { SchemaVersion = 2 };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.Contains("SchemaVersion"));
    }

    [Fact]
    public void Validate_RoleFamilyOutOfRange_ReturnsInvalid()
    {
        // RoleFamily is byte-backed; a cast from a byte not in [1,8] is a
        // valid CLR value but invalid per the enum semantic.
        var packet = MakeValidPacket() with { RoleFamily = (RoleFamily)99 };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.Contains("RoleFamily"));
    }

    [Fact]
    public void Validate_BadSignatureIdFormat_ReturnsInvalid()
    {
        var packet = MakeValidPacket() with
        {
            SignatureCandidates = new[]
            {
                new SignatureCandidate
                {
                    SignatureId = "InvalidFormat::not-a-signature-id",
                    AffinityWeightRaw = Fixed.One.RawValue / 2L,
                },
            },
        };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.Contains("SignatureId"));
    }

    [Fact]
    public void Validate_GeneValueAboveOne_ReturnsInvalid()
    {
        // Per pr-review-toolkit:type-design-analyzer 2026-04-30: the
        // Phase-3 validator was missing gene-range bounds. A fixture
        // authoring error (typo, sign flip, decimal-place miscount in
        // Q32.32 raw long) would have silently corrupted signature-
        // affinity dispatch. This test pins the rejection.
        var packet = MakeValidPacket() with
        {
            Genes = new IdentityPacketGenes
            {
                FastTwitchRawQ32 = Fixed.One.RawValue + 1L,  // out of range
                PatternRecognitionRawQ32 = Fixed.One.RawValue / 2L,
                DecisionVelocityRawQ32 = Fixed.One.RawValue / 2L,
                FirstTouchRawQ32 = Fixed.One.RawValue / 2L,
                StrikingRawQ32 = Fixed.One.RawValue / 2L,
                LeftFootRawQ32 = Fixed.One.RawValue / 2L,
            },
        };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.Contains("FastTwitchRawQ32"));
    }

    [Fact]
    public void Validate_GeneValueNegative_ReturnsInvalid()
    {
        var packet = MakeValidPacket() with
        {
            Genes = new IdentityPacketGenes
            {
                FastTwitchRawQ32 = Fixed.One.RawValue / 2L,
                PatternRecognitionRawQ32 = -1L,  // out of range
                DecisionVelocityRawQ32 = Fixed.One.RawValue / 2L,
                FirstTouchRawQ32 = Fixed.One.RawValue / 2L,
                StrikingRawQ32 = Fixed.One.RawValue / 2L,
                LeftFootRawQ32 = Fixed.One.RawValue / 2L,
            },
        };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.Contains("PatternRecognitionRawQ32"));
    }

    [Fact]
    public void Validate_NullGenesRecord_ReturnsInvalid()
    {
        var packet = MakeValidPacket() with { Genes = null! };
        var result = IdentityPacketValidator.Validate(packet);
        Assert.False(result.IsValid);
        Assert.Contains(result.Errors, e => e.Contains("Genes is null"));
    }

    [Fact]
    public void Parse_FromInvalidJson_ThrowsInvalidDataException()
    {
        // Loader-side end-to-end: bad JSON content fails the
        // validator + throws InvalidDataException with the error list
        // in the message.
        string badJson = "{\"PlayerId\":\"not-a-valid-id\",\"SchemaVersion\":1}";
        Assert.Throws<InvalidDataException>(() => IdentityPackets.Parse(badJson));
    }

    private static IdentityPacket MakeValidPacket() => new()
    {
        PlayerId = "fwh.core:player_99999",
        DisplayNameFull = "Test Player",
        DisplayNameShort = "T. Player",
        RoleFamily = RoleFamily.CentralMidfielder,
        SignatureCandidates = System.Array.Empty<SignatureCandidate>(),
        Genes = new IdentityPacketGenes
        {
            FastTwitchRawQ32 = Fixed.One.RawValue / 2L,
            PatternRecognitionRawQ32 = Fixed.One.RawValue / 2L,
            DecisionVelocityRawQ32 = Fixed.One.RawValue / 2L,
            FirstTouchRawQ32 = Fixed.One.RawValue / 2L,
            StrikingRawQ32 = Fixed.One.RawValue / 2L,
            LeftFootRawQ32 = Fixed.One.RawValue / 2L,
        },
        SchemaVersion = IdentityPacket.CurrentSchemaVersion,
        SourcePackVersion = "1.0.0",
    };
}

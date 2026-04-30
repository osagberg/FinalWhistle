using System.Text.Json;
using FinalWhistle.MatchSim.Content;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Content;

/// <summary>
/// Round-trip + cache tests for the embedded IdentityPacket fixtures per
/// SPEC.md Phase-3 line 142 + ADR-0006 §verification-required.
///
/// <para>
/// Round-trip discipline: every fixture deserializes via
/// <see cref="IdentityPackets.Parse"/>, re-serializes via
/// <see cref="JsonSerializer.Serialize{TValue}(TValue, JsonSerializerOptions)"/>,
/// and re-deserializes — the second deserialization must produce a packet
/// structurally equal to the first. This is the explicit ADR-0006
/// "round-trip serialization clean" gate.
/// </para>
/// </summary>
public sealed class IdentityPacketRoundTripTests
{
    [Fact]
    public void Load_AllBuiltInFixtures_LoadsExactly22()
    {
        var packets = IdentityPackets.LoadAll();

        Assert.Equal(22, packets.Count);
    }

    [Fact]
    public void Load_BuiltInFixturesPerArchetype_Yields11Each()
    {
        // Per feature-dev:code-reviewer 2026-04-30: don't infer archetype
        // membership from PlayerId numeric range — that couples the test
        // to fixture-numbering convention rather than the loader's
        // archetype routing. Iterate directly via Load(archetype, jersey)
        // so the test verifies what the loader actually does.
        foreach (var archetype in IdentityPackets.BuiltInArchetypeNames)
        {
            int loaded = 0;
            for (byte jersey = 1; jersey <= IdentityPackets.PlayersPerArchetype; jersey++)
            {
                var packet = IdentityPackets.Load(archetype, jersey);
                Assert.NotNull(packet);
                loaded++;
            }
            Assert.Equal(IdentityPackets.PlayersPerArchetype, loaded);
        }

        // Belt-and-suspenders: total still equals 22.
        Assert.Equal(22, IdentityPackets.LoadAll().Count);
    }

    [Fact]
    public void Load_SameFixtureTwice_ReturnsCachedInstance()
    {
        // The cache lifecycle promise per IdentityPackets.cs doc-comment.
        // ReferenceEquals catches a hidden refactor that reloads on every
        // call (e.g., GetOrAdd → ConcurrentDictionary swap to a non-caching
        // dictionary).
        var first = IdentityPackets.Load("direct-pressing", 1);
        var second = IdentityPackets.Load("direct-pressing", 1);

        Assert.Same(first, second);
    }

    [Fact]
    public void Parse_ValidFixture_RoundTripsThroughJsonSerializer()
    {
        // Deserialize → re-serialize → re-deserialize → structural equality
        // on every field. The whole-cycle test: anything System.Text.Json-
        // related that would silently drop a field (missing converter,
        // mismatched property name, etc.) breaks here.
        //
        // Note: record-level Equals is NOT used here. The default-synthesized
        // `IdentityPacket.Equals(IdentityPacket?)` uses reference equality
        // on the `IReadOnlyList<SignatureCandidate>` field (List<T>.Equals
        // is reference equality). A record-level structural-equality
        // implementation requires either ImmutableArray with overridden
        // EqualityComparer or a hand-rolled Equals override; that's a
        // Phase-4 refactor when more code paths consume IdentityPacket
        // equality semantics. For now, deep-compare here explicitly.
        var original = IdentityPackets.Load("direct-pressing", 6);  // J. Pielke (winger w/ signature)

        string roundTripJson = JsonSerializer.Serialize(original);
        var roundTripped = IdentityPackets.Parse(roundTripJson);

        AssertPacketsEquivalent(original, roundTripped);
    }

    [Fact]
    public void Parse_AllBuiltInFixtures_RoundTripCleanly()
    {
        // Belt-and-suspenders: every one of the 22 fixtures must survive
        // a serialize → deserialize cycle.
        foreach (var packet in IdentityPackets.LoadAll())
        {
            string json = JsonSerializer.Serialize(packet);
            var roundTripped = IdentityPackets.Parse(json);
            AssertPacketsEquivalent(packet, roundTripped);
        }
    }

    private static void AssertPacketsEquivalent(IdentityPacket expected, IdentityPacket actual)
    {
        Assert.Equal(expected.PlayerId, actual.PlayerId);
        Assert.Equal(expected.DisplayNameFull, actual.DisplayNameFull);
        Assert.Equal(expected.DisplayNameShort, actual.DisplayNameShort);
        Assert.Equal(expected.RoleFamily, actual.RoleFamily);
        Assert.Equal(expected.SchemaVersion, actual.SchemaVersion);
        Assert.Equal(expected.SourcePackVersion, actual.SourcePackVersion);
        Assert.Equal(expected.Genes, actual.Genes);

        Assert.Equal(expected.SignatureCandidates.Count, actual.SignatureCandidates.Count);
        for (int i = 0; i < expected.SignatureCandidates.Count; i++)
        {
            Assert.Equal(
                expected.SignatureCandidates[i].SignatureId,
                actual.SignatureCandidates[i].SignatureId);
            Assert.Equal(
                expected.SignatureCandidates[i].AffinityWeightRaw,
                actual.SignatureCandidates[i].AffinityWeightRaw);
        }
    }

    [Fact]
    public void Load_KnownPlayerIds_AreUniqueAcross22Packets()
    {
        // ADR-0006 §content-pack-ID-rules: every PlayerId is stable + unique.
        // This is the validator-vs-batch check — individual validation passes
        // a packet with a duplicate ID (the validator only sees one packet at
        // a time); cross-packet uniqueness needs a sweep here.
        var seen = new System.Collections.Generic.HashSet<string>(System.StringComparer.Ordinal);
        foreach (var packet in IdentityPackets.LoadAll())
        {
            Assert.True(seen.Add(packet.PlayerId),
                $"Duplicate PlayerId found across fixtures: {packet.PlayerId}");
        }
    }

    [Fact]
    public void Load_InvalidArchetype_Throws()
    {
        Assert.Throws<System.IO.FileNotFoundException>(() =>
            IdentityPackets.Load("does-not-exist", 1));
    }

    [Fact]
    public void Load_JerseyOutOfRange_Throws()
    {
        Assert.Throws<System.ArgumentOutOfRangeException>(() =>
            IdentityPackets.Load("direct-pressing", 0));
        Assert.Throws<System.ArgumentOutOfRangeException>(() =>
            IdentityPackets.Load("direct-pressing", 12));
    }

}

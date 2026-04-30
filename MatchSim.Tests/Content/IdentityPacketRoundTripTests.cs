using FinalWhistle.MatchSim.Content;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Content;

/// <summary>
/// Round-trip + cache tests for the embedded IdentityPacket fixtures per
/// SPEC.md Phase-3 line 142 + ADR-0006 §verification-required.
///
/// <para>
/// Round-trip discipline: parse the same fixture twice (via
/// <see cref="IdentityPackets.Load"/> + via direct <see cref="IdentityPackets.Parse"/>
/// of the same source) and assert structural equality. The hand-rolled
/// <c>IdentityPacketParser</c> is strict-by-design — schema drift would
/// surface as a parse failure rather than a silent default-zero (Codex
/// round-7 P1#2 2026-04-30). System.Text.Json was removed from MatchSim
/// because it doesn't ship in Unity 6's Mono runtime (Codex P1#1).
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
    public void Parse_AllBuiltInFixtures_ParseTwice_AgreeStructurally()
    {
        // Parse the same fixture source twice via two different code paths
        // (Load via cache miss / Parse direct from string). Both must
        // produce structurally equal packets. This replaces the prior
        // serialize-then-parse round-trip, which depended on
        // System.Text.Json — removed because STJ doesn't ship in Unity 6's
        // Mono runtime (Codex round-7 P1#1) and silently accepted typoed
        // fields (P1#2). With the hand-rolled strict parser, schema drift
        // surfaces as a parse failure rather than a silent default-zero,
        // so the round-trip safety property is preserved differently:
        // by the parser's strict-mode-by-default.
        foreach (var archetype in IdentityPackets.BuiltInArchetypeNames)
        {
            for (byte jersey = 1; jersey <= IdentityPackets.PlayersPerArchetype; jersey++)
            {
                var loaded = IdentityPackets.Load(archetype, jersey);

                // Re-parse the SAME embedded resource via the direct Parse
                // path (bypasses cache). Locate the resource by name and
                // read its raw JSON text.
                string resourceName =
                    $"FinalWhistle.MatchSim.Content.identity-packets.{archetype}.{jersey:D2}.json";
                using var stream = typeof(IdentityPackets).Assembly
                    .GetManifestResourceStream(resourceName);
                Assert.NotNull(stream);
                using var reader = new System.IO.StreamReader(stream!);
                string sourceJson = reader.ReadToEnd();
                var reparsed = IdentityPackets.Parse(sourceJson);

                AssertPacketsEquivalent(loaded, reparsed);
            }
        }
    }

    /// <summary>
    /// Field-by-field structural-equality assertion.
    ///
    /// <para>
    /// <strong>Maintenance contract</strong> (per feature-dev:code-reviewer
    /// 2026-04-30 round-7 review, confidence-85 finding): when
    /// <see cref="IdentityPacket"/> gains a new field at a Phase-4+ schema
    /// bump, this helper MUST grow a new <c>Assert.Equal</c> line. The
    /// helper has no compile-time exhaustiveness guarantee — a missed
    /// assertion would silently pass tests where the new field's value
    /// differed between <paramref name="expected"/> and
    /// <paramref name="actual"/>. Cross-checked against ADR-0006 schema
    /// at every save-migration-fixture v(N) → v(N+1) bump.
    /// </para>
    /// </summary>
    private static void AssertPacketsEquivalent(IdentityPacket expected, IdentityPacket actual)
    {
        // Phase-3 schema v1: 8 top-level fields (PlayerId / DisplayNameFull
        // / DisplayNameShort / RoleFamily / SignatureCandidates / Genes /
        // SchemaVersion / SourcePackVersion). Adding a Phase-4 field
        // requires adding the matching Assert.Equal here. IdentityPacketGenes
        // is checked via record-level Equals (the record's compiler-
        // synthesized equality compares all 6 long fields structurally).
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

using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Content;

/// <summary>
/// Signature-affinity read tests per SPEC.md Phase-3 line 142 task wording:
/// "sufficient to round-trip through the validator and exercise scout-prose
/// templates + signature-affinity reads." The 3 active Phase-3 signatures
/// (#13 first-time-diagonal-switch / #20 low-cutback-from-byline / #22
/// blind-side-near-post-run) consult the per-player <see cref="SignatureCandidate"/>
/// list to determine which players can awaken each signature; these tests exercise that reader path against the embedded fixtures. // ui-lint:allow term="awakens" reason="signature-mechanic vocabulary referencing ADR-0005 SignatureAwakened MemoryEvent class name; not player-facing UI copy" reviewer="osagberg"
/// </summary>
public sealed class IdentityPacketSignatureAffinityTests
{
    private const string LowCutbackId = "fwh.core:signature.low-cutback-from-byline";
    private const string BlindSideRunId = "fwh.core:signature.blind-side-near-post-run";
    private const string DiagonalSwitchId = "fwh.core:signature.first-time-diagonal-switch";

    [Fact]
    public void Fixtures_LowCutbackSignature_HasAtLeastOneCarrier()
    {
        // The first of the 3 Phase-3 active signatures must have at least
        // one carrier in the 22-packet smoke set, otherwise the dispatch
        // path it exercises is dead in the Month-3 dots viewer.
        var carriers = CountCarriers(LowCutbackId);
        Assert.True(carriers >= 1,
            $"Expected at least 1 carrier for {LowCutbackId}; found 0.");
    }

    [Fact]
    public void Fixtures_BlindSideRunSignature_HasAtLeastOneCarrier()
    {
        var carriers = CountCarriers(BlindSideRunId);
        Assert.True(carriers >= 1,
            $"Expected at least 1 carrier for {BlindSideRunId}; found 0.");
    }

    [Fact]
    public void Fixtures_DiagonalSwitchSignature_HasAtLeastOneCarrier()
    {
        var carriers = CountCarriers(DiagonalSwitchId);
        Assert.True(carriers >= 1,
            $"Expected at least 1 carrier for {DiagonalSwitchId}; found 0.");
    }

    [Fact]
    public void Fixtures_LowCutbackCarriers_AreWingers()
    {
        // Role-affinity sanity: the Phase-3 cutback signature should only
        // be carried by Winger role-family players. If a Goalkeeper picks
        // up cutback affinity in fixture authoring, that's a bug — the
        // Phase-4 affinity-count roll-procedure consults role-family.
        foreach (var packet in IdentityPackets.LoadAll())
        {
            foreach (var candidate in packet.SignatureCandidates)
            {
                if (candidate.SignatureId == LowCutbackId)
                {
                    Assert.Equal(RoleFamily.Winger, packet.RoleFamily);
                }
            }
        }
    }

    [Fact]
    public void Fixtures_BlindSideRunCarriers_AreStrikers()
    {
        foreach (var packet in IdentityPackets.LoadAll())
        {
            foreach (var candidate in packet.SignatureCandidates)
            {
                if (candidate.SignatureId == BlindSideRunId)
                {
                    Assert.Equal(RoleFamily.Striker, packet.RoleFamily);
                }
            }
        }
    }

    [Fact]
    public void Fixtures_DiagonalSwitchCarriers_AreCentralMidfielders()
    {
        foreach (var packet in IdentityPackets.LoadAll())
        {
            foreach (var candidate in packet.SignatureCandidates)
            {
                if (candidate.SignatureId == DiagonalSwitchId)
                {
                    Assert.Equal(RoleFamily.CentralMidfielder, packet.RoleFamily);
                }
            }
        }
    }

    [Fact]
    public void Fixtures_AllAffinityWeights_AreInZeroToOneRange()
    {
        // Belt-and-suspenders against the validator's [0, Fixed.One.RawValue]
        // bound. Direct-decode each candidate's raw weight to a Fixed and
        // assert it lies in [0, 1].
        long oneRaw = Fixed.One.RawValue;
        foreach (var packet in IdentityPackets.LoadAll())
        {
            foreach (var candidate in packet.SignatureCandidates)
            {
                Assert.InRange(candidate.AffinityWeightRaw, 0L, oneRaw);

                Fixed weight = Fixed.FromRaw(candidate.AffinityWeightRaw);
                Assert.True(weight.CompareTo(Fixed.Zero) >= 0,
                    $"Negative weight on {packet.PlayerId}: {weight}");
                Assert.True(weight.CompareTo(Fixed.One) <= 0,
                    $"Above-one weight on {packet.PlayerId}: {weight}");
            }
        }
    }

    [Fact]
    public void Fixtures_TotalSignatureCarriers_ExerciseAffinityCodepath()
    {
        // Phase-3 minimum: at least 4 of the 22 packets carry ≥1 candidate.
        // This isn't a balance assertion — it's a smoke check that the
        // affinity-read pipeline gets exercised in non-zero ways across
        // the smoke set (the dispatch logic short-circuits when zero
        // candidates exist; Phase-3 needs the non-empty-list branch
        // covered).
        int withCandidates = 0;
        foreach (var packet in IdentityPackets.LoadAll())
        {
            if (packet.SignatureCandidates.Count > 0) withCandidates++;
        }
        Assert.True(withCandidates >= 4,
            $"Expected ≥4 packets carrying signature candidates; found {withCandidates}.");
    }

    private static int CountCarriers(string signatureId)
    {
        int count = 0;
        foreach (var packet in IdentityPackets.LoadAll())
        {
            foreach (var candidate in packet.SignatureCandidates)
            {
                if (candidate.SignatureId == signatureId) count++;
            }
        }
        return count;
    }
}

using Xunit;

namespace FinalWhistle.MatchSim.Tests;

/// <summary>
/// Skeleton test asserting MatchSim.csproj is reachable from
/// MatchSim.Tests.csproj. Real determinism + Fixed-point + Tick + Seed
/// tests land as those types are authored per SPEC Phase-3 priority order.
/// </summary>
public sealed class MatchSimAssemblyMarkerTests
{
    [Fact]
    public void AssemblyMarkerVersion_IsNonEmpty()
    {
        Assert.False(string.IsNullOrWhiteSpace(MatchSimAssemblyMarker.AssemblyMarkerVersion));
    }

    [Fact]
    public void AssemblyMarkerVersion_IndicatesPhase3Skeleton()
    {
        Assert.Contains("phase3", MatchSimAssemblyMarker.AssemblyMarkerVersion);
        Assert.Contains("skeleton", MatchSimAssemblyMarker.AssemblyMarkerVersion);
    }
}

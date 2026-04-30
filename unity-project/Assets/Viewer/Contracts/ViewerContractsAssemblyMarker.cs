namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// Cross-asmdef compile-time + load-time linkage marker for the
    /// renderer-agnostic <c>Viewer.Contracts</c> package. The asmdef
    /// declares <c>noEngineReferences: true</c> + a precompiled reference
    /// to <c>FinalWhistle.MatchSim.dll</c>; the static reads below make
    /// both load-bearing — Contracts will not compile if MatchSim's DLL
    /// boundary is broken, and the asmdef-level UnityEngine-free flag
    /// surfaces a stray <c>using UnityEngine</c> at edit time.
    ///
    /// <para>
    /// Phase-3 SPEC line 148 promotes Contracts from skeleton to real
    /// types: <see cref="ViewerEvent"/> / <see cref="ShotTypeDefinition"/>
    /// / <see cref="MemoryHit"/> / <see cref="CallbackSlotValue"/> /
    /// <see cref="AdapterId"/> / <see cref="ShotCategory"/> /
    /// <see cref="ReduceMotionStrategy"/> per ADR-0008's v1 contract.
    /// This marker stays as the cross-asmdef linkage proof.
    /// </para>
    /// </summary>
    public static class ViewerContractsAssemblyMarker
    {
        public const string AssemblyMarkerVersion = "Phase3-EventBridge-v1";

        /// <summary>
        /// Read-time symbol from <c>FinalWhistle.MatchSim.dll</c> proves
        /// the precompiled-reference path is load-bearing. A property
        /// rather than a const so the link is dynamic (a const would be
        /// inlined at compile time + exercise nothing at runtime).
        /// </summary>
        public static string MatchSimVersionStamp
            => FinalWhistle.MatchSim.MatchSimAssemblyMarker.AssemblyMarkerVersion;

        /// <summary>
        /// Read-time symbol from <c>FinalWhistle.MatchSim.Sim.Tick</c>
        /// proves the canonical-state primitives are reachable.
        /// </summary>
        public static int MatchSimTicksPerSecond
            => FinalWhistle.MatchSim.Sim.Tick.TicksPerSecond;
    }
}

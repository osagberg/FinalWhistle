namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// Skeleton-stub marker proving Viewer.Contracts compiles cleanly without
    /// UnityEngine references and that Viewer.Core / Viewer.Adapters.Dots can
    /// reference types from this asmdef. Per the renderer-agnostic posture in
    /// ADR-0008 ShotPresentationContract: this layer hosts the pure-C# DTOs
    /// (ViewerEvent / ShotTypeDefinition / PitchView / ActiveViewerEvent /
    /// MemoryHit / etc.) that adapters consume. Real schema lands in
    /// subsequent Phase-3 SPEC tasks (Viewer.EventBridge minimum impl + 22
    /// IdentityPacket fixtures + 3 active signatures + MemoryEvent reader +
    /// development event); this marker exists only to prove the asmdef
    /// skeleton compiles.
    ///
    /// asmdef-level enforcement of the UnityEngine-free constraint: the
    /// `noEngineReferences: true` flag in Viewer.Contracts.asmdef means a
    /// stray `using UnityEngine` here would FAIL compilation, surfacing the
    /// architectural violation at edit time rather than at runtime. Same
    /// rule MatchSim follows (per .claude/rules/Scripts/MatchSim/RULES.md),
    /// now applied at the renderer-agnostic contract layer.
    /// </summary>
    public static class ViewerContractsAssemblyMarker
    {
        /// <summary>
        /// Public so downstream asmdefs (Viewer.Core / Viewer.Adapters.Dots)
        /// can read it as compile-time proof that this asmdef is referenceable
        /// across the asmdef boundary. The class is also public for the same
        /// reason — an internal class would hide the const from other
        /// assemblies even with `public const` modifier.
        /// </summary>
        public const string AssemblyMarkerVersion = "Phase3-skeleton-v1";
    }
}

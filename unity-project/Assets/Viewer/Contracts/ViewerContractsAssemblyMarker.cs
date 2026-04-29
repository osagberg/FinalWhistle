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

        /// <summary>
        /// Compile-time + load-time proof that the MatchSim DLL drop is
        /// actually consumable across the asmdef precompiled-reference
        /// boundary. The asmdef declaring `precompiledReferences:
        /// ["FinalWhistle.MatchSim.dll"]` is necessary but not sufficient —
        /// without an actual symbol read, Unity can resolve the asmdef
        /// without ever loading or linking against the DLL, leaving the
        /// claimed consumption boundary unverified.
        ///
        /// Reading <see cref="FinalWhistle.MatchSim.MatchSimAssemblyMarker.AssemblyMarkerVersion"/>
        /// here makes the DLL boundary load-bearing: Contracts won't compile
        /// if the MatchSim DLL isn't resolvable, and won't run if the runtime
        /// loader can't find it. This is the same pattern downstream Viewer.
        /// EventBridge (Phase-3 semantic-slice task) will use to derive
        /// `ViewerEvent`s from MatchSim's canonical event stream.
        ///
        /// Property-style accessor (not <c>public const</c>) so the linkage
        /// is dynamic — a const string from another assembly would be
        /// baked in at compile time and exercise nothing at run time.
        /// </summary>
        public static string MatchSimVersionStamp
            => FinalWhistle.MatchSim.MatchSimAssemblyMarker.AssemblyMarkerVersion;

        /// <summary>
        /// Compile-time proof that types in the <c>FinalWhistle.MatchSim.Sim</c>
        /// namespace (the canonical-state primitives — Tick / Fixed / Seed /
        /// BallState / etc.) are reachable from the renderer-agnostic Contracts
        /// layer. Reading <see cref="FinalWhistle.MatchSim.Sim.Tick.TicksPerSecond"/>
        /// (the locked 60Hz canonical-tick rate per ADR-0008 + match-engine.md
        /// §Q1) anchors the most-frequently-consumed Sim symbol in this proof.
        /// </summary>
        public static int MatchSimTicksPerSecond
            => FinalWhistle.MatchSim.Sim.Tick.TicksPerSecond;
    }
}

namespace FinalWhistle.Viewer.Core
{
    /// <summary>
    /// Skeleton-stub marker proving Viewer.Core compiles cleanly with both
    /// UnityEngine + Viewer.Contracts references resolvable. Per
    /// ADR-0008/0009: this layer hosts the Unity-side adapter registry,
    /// Viewer.EventBridge (which derives ViewerEvents from MatchSim canonical
    /// events ordered by `(StartTick, ViewerEventId)`), and the
    /// `ShotTypeSO -> ShotTypeDefinition` projection that hands pure-C# DTOs
    /// to downstream renderer adapters. Real implementations land in
    /// subsequent Phase-3 SPEC tasks (Viewer.EventBridge minimum impl, then
    /// the 7-shot ShotTypeSO authoring, then the dots adapter prototype).
    /// </summary>
    public static class ViewerCoreAssemblyMarker
    {
        public const string AssemblyMarkerVersion = "Phase3-skeleton-v1";

        /// <summary>
        /// Compile-time proof that Viewer.Contracts is referenceable from
        /// Viewer.Core. If the asmdef reference is wired wrong, this read
        /// fails at compile time rather than at runtime. The fully-qualified
        /// name avoids any `Contracts` namespace shortcuts collisioning with
        /// future Unity-side `Contracts` types.
        /// </summary>
        internal static string ContractsMarker
            => FinalWhistle.Viewer.Contracts.ViewerContractsAssemblyMarker
                .AssemblyMarkerVersion;
    }
}

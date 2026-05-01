using System.Runtime.CompilerServices;

// Exposes `internal` test-surface accessors (e.g. ShotCamera.TargetShot /
// CurrentAdapterShot) to the EditMode test assembly. Production callers
// outside this asmdef must not depend on internal members.
[assembly: InternalsVisibleTo("FinalWhistle.Viewer.Tests.EditMode")]

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Skeleton-stub marker proving Viewer.Adapters.Dots compiles cleanly
    /// with Core + Contracts + URP references resolvable. Per ADR-0009: this
    /// layer hosts the dots-phase render adapter — sprite-on-pitch + the
    /// 7-shot-type vocabulary (tactical-wide / diagonal-attack-lane /
    /// pass-shot-impact / player-isolation / aftermath-freeze /
    /// crowd-reaction / tunnel-vision-press) + reduce-motion variant
    /// substitution + UI-Toolkit overlay text + identity overlays.
    ///
    /// Held to ADR-0009 §Polish bar (kit discrimination / identity
    /// legibility / camera rhythm / signature presentation cues / commentary
    /// integration / reduce-motion) because dots may ship at EA per the
    /// 2026-04-26 visual-target supersession outcome (c2). NOT debug-quality.
    ///
    /// Real implementation lands in the "Dots-phase render adapter prototype"
    /// Phase-3 SPEC task, which depends on this asmdef skeleton + the
    /// PitchRules layer + the semantic slice (22 IdentityPacket fixtures + 3
    /// active signatures + 1 MemoryEvent reader + 1 development event +
    /// Viewer.EventBridge minimum impl) per the 2026-04-28 decisions log.
    /// This marker only proves the asmdef skeleton compiles.
    /// </summary>
    public static class ViewerAdaptersDotsAssemblyMarker
    {
        public const string AssemblyMarkerVersion = "Phase3-skeleton-v1";

        /// <summary>
        /// Compile-time proof that both Core and Contracts are referenceable
        /// from this adapter asmdef. Verifies the Dots → Core → Contracts
        /// reference graph at the assembly-resolution layer. If either
        /// reference is missing from Viewer.Adapters.Dots.asmdef, the read
        /// fails at compile time.
        /// </summary>
        internal static string CoreMarker
            => FinalWhistle.Viewer.Core.ViewerCoreAssemblyMarker
                .AssemblyMarkerVersion;

        internal static string ContractsMarker
            => FinalWhistle.Viewer.Contracts.ViewerContractsAssemblyMarker
                .AssemblyMarkerVersion;
    }
}

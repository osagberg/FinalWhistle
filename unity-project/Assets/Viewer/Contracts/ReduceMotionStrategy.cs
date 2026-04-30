namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// How an adapter handles the <c>reduce_motion</c> accessibility flag
    /// per ADR-0008 §"Reduce-motion adapter-awareness." The substitution
    /// boundary is locked at <see cref="EventBridge"/> — adapters do NOT
    /// re-substitute. Adapters MAY further disable adapter-specific
    /// features (motion-line trails, camera-rhythm easing) when the bridge
    /// reports <c>ReduceMotionApplied == true</c>; those feature-toggle
    /// states must be declared in the adapter's pass-activation trace
    /// section.
    /// </summary>
    public enum ReduceMotionStrategy : byte
    {
        /// <summary>Sentinel — never valid in a constructed adapter context.</summary>
        None = 0,

        /// <summary>
        /// Disable affected features at scene-load-time per ADR-0002
        /// structural posture inherited by ADR-0008. Adapter swap requires
        /// scene reload; reduce-motion swap requires scene reload.
        /// </summary>
        SceneLoadTime = 1,

        /// <summary>
        /// Runtime-branch on the flag per frame; less preferred. Allowed
        /// only if the adapter justifies the runtime cost in its ADR.
        /// </summary>
        PerFrame = 2,
    }
}

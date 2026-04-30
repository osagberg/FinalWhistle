namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// Closed code-owned registry of viewer adapter identities per ADR-0008
    /// §"Adapter selection." Community/mod adapters are code plugins shipped
    /// via trusted-build channel (separate Steam beta branch / signed
    /// sideload); they cannot register a new <see cref="AdapterId"/> via a
    /// content pack.
    ///
    /// <para>
    /// <strong>Pinned numeric values; never reuse.</strong> Adding a new
    /// adapter requires a new ADR + decisions-log entry. Removing one is
    /// a supersession event. The <see cref="ushort"/> backing matches the
    /// ADR-0008 §"AdapterId" enum spec.
    /// </para>
    /// </summary>
    public enum AdapterId : ushort
    {
        /// <summary>Sentinel — never valid in a constructed adapter context.</summary>
        None = 0,

        /// <summary>Dots-phase render adapter per ADR-0009.</summary>
        Dots = 1,

        /// <summary>Cel-shaded 3D render adapter per ADR-0010 (conditional on Phase-5/6 production-feasibility spike).</summary>
        CelShaded3d = 2,
    }
}

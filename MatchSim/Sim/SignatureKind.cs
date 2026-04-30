namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Internal Phase-3 signature catalog. Maps 1:1 to the three
/// <see cref="KeyEventKind"/> signature-execution values
/// (<see cref="KeyEventKind.SignatureExecuted_LowCutback"/> etc.) but
/// kept separate so the trigger-detection logic in
/// <see cref="SignatureRules"/> can <c>switch</c> on a focused
/// signature-only enum without polluting the canonical-event enum.
///
/// <para>
/// <strong>Phase-3 minimum scope.</strong> Just the three signatures
/// pinned by the 2026-04-24 month-3-vertical-slice resolution: #13
/// First-time diagonal switch (CM) / #20 Low cutback from the byline
/// (Winger) / #22 Blind-side near-post run (Striker). The full
/// 24-signature catalog (per <c>design/signatures.md</c>) lands at
/// Phase 4+ via the proper <c>SignatureSO</c> ScriptableObject schema
/// per ADR-0005.
/// </para>
///
/// <para>
/// <strong>Numeric values</strong> mirror <c>KeyEventKind</c> Phase-3
/// signature offsets minus 4: <c>LowCutback=1</c> ↔ <c>KeyEventKind.SignatureExecuted_LowCutback=5</c>,
/// etc. This keeps the mapping arithmetic predictable without coupling
/// the enums.
/// </para>
/// </summary>
public enum SignatureKind : byte
{
    /// <summary>#20 Low cutback from the byline (Winger).</summary>
    LowCutback = 1,

    /// <summary>#22 Blind-side near-post run (Striker).</summary>
    BlindSideNearPostRun = 2,

    /// <summary>#13 First-time diagonal switch (CentralMidfielder).</summary>
    FirstTimeDiagonalSwitch = 3,
}

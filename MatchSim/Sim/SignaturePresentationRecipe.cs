namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Pure-C# DTO that travels alongside each signature-execution
/// <see cref="KeyEvent"/> via the parallel
/// <see cref="MatchSimulationState.SignatureRecipes"/> stream. Read by
/// <c>Viewer.EventBridge</c> (Phase-3 next semantic-slice item) which
/// maps <see cref="RecipeKey"/> to the dots-adapter shot-type
/// selection per ADR-0008.
///
/// <para>
/// <strong>Phase-3 minimum scope</strong>: data-only metadata
/// (signature ID + recipe key + sim-bias-field name + delta value).
/// The actual MatchSim canonical-state behavioral modification (e.g.,
/// modifying ball-physics or player-bias when a signature fires) lands
/// in Phase 4 when ADR-0005's full <c>SignatureSO</c> + bake-time
/// <c>SimBiasSnapshot</c> wiring ships. Phase-3 sim bias is metadata
/// only — the dots adapter consumes <see cref="SimBiasFieldId"/> +
/// <see cref="SimBiasDeltaRawQ32"/> for presentation-layer effects
/// (camera cut, overlay text), not for actual sim modification.
/// </para>
///
/// <para>
/// <strong>Not canonical state.</strong> The recipe stream is
/// derived data — not encoded into <c>MatchCanonicalState.Write</c>.
/// Adding presentation metadata to the canonical hash would couple
/// the corpus fixture to viewer-presentation choices, which is the
/// wrong axis (canonical = gameplay outcomes; recipes = display
/// metadata).
/// </para>
/// </summary>
public readonly struct SignaturePresentationRecipe
{
    /// <summary>
    /// Content-pack-qualified signature ID. Format
    /// <c>fwh.core:signature.&lt;slug&gt;</c> per ADR-0005.
    /// </summary>
    public string SignatureId { get; }

    /// <summary>
    /// Shot-type recipe key per ADR-0008. The dots adapter (and the
    /// 3D adapter, if ADR-0010 spike-green) maps this string to the
    /// 7-shot-type vocabulary in <c>design/semantic-cinema.md</c>:
    /// <c>tactical-wide</c> / <c>diagonal-attack-lane</c> /
    /// <c>player-isolation</c> / <c>duel-panel</c> /
    /// <c>pass-shot-impact</c> / <c>crowd-reaction</c> /
    /// <c>aftermath-freeze</c>.
    /// </summary>
    public string RecipeKey { get; }

    /// <summary>
    /// Sim-bias field identifier per ADR-0005 (e.g.
    /// <c>cutback_xAssist</c>, <c>near_post_xG</c>,
    /// <c>diagonal_switch_trigger</c>). Phase-3: viewer-side only.
    /// Phase-4: actual MatchSim canonical-state bias dispatch.
    /// </summary>
    public string SimBiasFieldId { get; }

    /// <summary>
    /// Q32.32 raw delta applied to the named sim-bias field.
    /// Decode via <see cref="Fixed.FromRaw(long)"/>.
    /// </summary>
    public long SimBiasDeltaRawQ32 { get; }

    public SignaturePresentationRecipe(
        string signatureId,
        string recipeKey,
        string simBiasFieldId,
        long simBiasDeltaRawQ32)
    {
        // Per pr-review-toolkit:type-design-analyzer 2026-04-30 round-2:
        // every string field is required + non-empty. Reject early so a
        // null/empty payload doesn't silently flow into the
        // ViewerEvent translation layer where the bridge would have to
        // re-validate every field.
        if (string.IsNullOrEmpty(signatureId))
        {
            throw new System.ArgumentException(
                "SignatureId must be non-empty.", nameof(signatureId));
        }
        if (string.IsNullOrEmpty(recipeKey))
        {
            throw new System.ArgumentException(
                "RecipeKey must be non-empty.", nameof(recipeKey));
        }
        if (string.IsNullOrEmpty(simBiasFieldId))
        {
            throw new System.ArgumentException(
                "SimBiasFieldId must be non-empty.", nameof(simBiasFieldId));
        }
        SignatureId = signatureId;
        RecipeKey = recipeKey;
        SimBiasFieldId = simBiasFieldId;
        SimBiasDeltaRawQ32 = simBiasDeltaRawQ32;
    }
}

/// <summary>
/// Pairs a <see cref="KeyEvent"/> in the canonical event stream with
/// its presentation-recipe metadata. The bridge boundary
/// (<c>Viewer.EventBridge</c>, Phase-3 next semantic-slice item)
/// reads this list each tick and translates to <c>ViewerEvent</c>s
/// per ADR-0008.
/// </summary>
public readonly struct SignatureExecution
{
    /// <summary>
    /// Index into <see cref="MatchSimulationState.KeyEvents"/> for the
    /// matching signature-execution event. Stored as an index (not a
    /// reference) so the recipe stream stays append-only and
    /// reorder-resilient.
    /// </summary>
    public int KeyEventIndex { get; }

    /// <summary>The presentation metadata payload.</summary>
    public SignaturePresentationRecipe Recipe { get; }

    public SignatureExecution(int keyEventIndex, SignaturePresentationRecipe recipe)
    {
        // Per pr-review-toolkit:type-design-analyzer 2026-04-30 round-2:
        // negative KeyEventIndex would silently desync the parallel
        // streams. Reject at construction.
        if (keyEventIndex < 0)
        {
            throw new System.ArgumentOutOfRangeException(
                nameof(keyEventIndex), keyEventIndex,
                "KeyEventIndex must be a non-negative position into MatchSimulationState.KeyEvents.");
        }
        KeyEventIndex = keyEventIndex;
        Recipe = recipe;
    }
}

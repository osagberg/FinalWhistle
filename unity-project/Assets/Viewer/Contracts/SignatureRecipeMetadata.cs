using System;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// Adapter-consumable presentation metadata copied from
    /// <see cref="SignaturePresentationRecipe"/> per Codex round-2 P1
    /// against <c>24767c0</c>. The signature slice (commit
    /// <c>bf2ac1e</c>) authored <see cref="SignatureId"/> +
    /// <see cref="SimBiasFieldId"/> + <see cref="SimBiasDeltaRawQ32"/>
    /// specifically for the dots adapter to consume; the slice-#5
    /// EventBridge dropped them at the contract boundary. This DTO
    /// carries them onto every signature-execution
    /// <see cref="ViewerEvent"/> so adapters can drive cut-in panels,
    /// commentary fragments, and presentation-layer effects from the
    /// authored metadata rather than re-deriving from KeyEventKind.
    ///
    /// <para>
    /// <strong>Phase-3 sim-bias delta is presentation-layer only</strong>
    /// per ADR-0005 SimBiasSnapshot deferral. The dots adapter consumes
    /// <see cref="SimBiasFieldId"/> + <see cref="SimBiasDeltaRawQ32"/>
    /// for camera cut + overlay text; actual MatchSim canonical-state
    /// behavioral modification on signature fire ships in Phase 4
    /// alongside the <c>SignatureSO</c> bake-time pipeline.
    /// </para>
    /// </summary>
    public readonly struct SignatureRecipeMetadata : IEquatable<SignatureRecipeMetadata>
    {
        /// <summary>
        /// Content-pack-qualified signature ID per ADR-0005
        /// (<c>fwh.core:signature.&lt;slug&gt;</c>). Adapters look up
        /// per-signature presentation tables (cut-in title cards,
        /// commentary cadence, etc.) via this ID.
        /// </summary>
        public string SignatureId { get; }

        /// <summary>
        /// Recipe key per ADR-0008 — the short slug
        /// (<c>player-isolation</c> / <c>pass-shot-impact</c> /
        /// <c>tactical-wide</c>) that drove the bridge's shot selection.
        /// Carried so the adapter can verify which recipe rule fired
        /// without re-correlating against the KeyEvents stream.
        /// </summary>
        public string RecipeKey { get; }

        /// <summary>
        /// Sim-bias field identifier per ADR-0005 (e.g.
        /// <c>cutback_xAssist</c>, <c>near_post_xG</c>,
        /// <c>diagonal_switch_trigger</c>). Phase-3: viewer-side only.
        /// </summary>
        public string SimBiasFieldId { get; }

        /// <summary>
        /// Q32.32 raw delta applied to the named sim-bias field per
        /// ADR-0005. Decode via <see cref="Fixed.FromRaw(long)"/>.
        /// </summary>
        public long SimBiasDeltaRawQ32 { get; }

        public SignatureRecipeMetadata(
            string signatureId,
            string recipeKey,
            string simBiasFieldId,
            long simBiasDeltaRawQ32)
        {
            if (string.IsNullOrEmpty(signatureId))
            {
                throw new ArgumentException("SignatureId must be non-empty.", nameof(signatureId));
            }
            if (string.IsNullOrEmpty(recipeKey))
            {
                throw new ArgumentException("RecipeKey must be non-empty.", nameof(recipeKey));
            }
            if (string.IsNullOrEmpty(simBiasFieldId))
            {
                throw new ArgumentException("SimBiasFieldId must be non-empty.", nameof(simBiasFieldId));
            }
            SignatureId = signatureId;
            RecipeKey = recipeKey;
            SimBiasFieldId = simBiasFieldId;
            SimBiasDeltaRawQ32 = simBiasDeltaRawQ32;
        }

        /// <summary>
        /// Project the MatchSim-side <see cref="SignaturePresentationRecipe"/>
        /// into the renderer-agnostic Contracts DTO. Round-trip-safe:
        /// <see cref="SignatureId"/> + <see cref="RecipeKey"/> +
        /// <see cref="SimBiasFieldId"/> + <see cref="SimBiasDeltaRawQ32"/>
        /// match the source bytes.
        /// </summary>
        public static SignatureRecipeMetadata FromRecipe(SignaturePresentationRecipe recipe) =>
            new(
                signatureId: recipe.SignatureId,
                recipeKey: recipe.RecipeKey,
                simBiasFieldId: recipe.SimBiasFieldId,
                simBiasDeltaRawQ32: recipe.SimBiasDeltaRawQ32);

        public bool Equals(SignatureRecipeMetadata other) =>
            SignatureId == other.SignatureId
            && RecipeKey == other.RecipeKey
            && SimBiasFieldId == other.SimBiasFieldId
            && SimBiasDeltaRawQ32 == other.SimBiasDeltaRawQ32;

        public override bool Equals(object? obj) => obj is SignatureRecipeMetadata other && Equals(other);
        public override int GetHashCode() => HashCode.Combine(SignatureId, RecipeKey, SimBiasFieldId, SimBiasDeltaRawQ32);
        public static bool operator ==(SignatureRecipeMetadata left, SignatureRecipeMetadata right) => left.Equals(right);
        public static bool operator !=(SignatureRecipeMetadata left, SignatureRecipeMetadata right) => !left.Equals(right);
        public override string ToString() => $"SignatureRecipeMetadata({SignatureId}, key={RecipeKey}, biasField={SimBiasFieldId}, deltaRaw={SimBiasDeltaRawQ32})";
    }
}

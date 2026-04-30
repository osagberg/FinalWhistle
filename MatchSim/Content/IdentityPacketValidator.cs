using System;
using System.Collections.Generic;
using System.Text.RegularExpressions;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Content;

/// <summary>
/// Phase-3 minimum subset of the IdentityPacket validator per ADR-0006
/// §validation. Phase-3 checks: schema version, ID format (no pack-minor
/// in entity IDs), display-name presence, role-family enum range,
/// signature-candidate count + affinity bounds + signature-ID format.
/// Deferred to Phase-6: phenotype-label lint, real-player-name diff,
/// SignatureId resolution against a loaded SignatureSO catalog (the full
/// content-pack validator).
///
/// <para>
/// Separate from <see cref="IdentityPackets"/> so tests can invoke the
/// validator directly on hand-crafted invalid inputs without going
/// through the embedded-resource loader path.
/// </para>
/// </summary>
public static class IdentityPacketValidator
{
    // Locked regexes per ADR-0006 §content-pack-ID-rules + ADR-0005 §SignatureSO ID format.
    // Compiled once at static-init for hot-path repeat use during full-pack
    // validation sweeps.
    private static readonly Regex PlayerIdPattern = new(
        @"^fwh\.core(?:\.v[0-9]+)?:player_[0-9]{5}$",
        RegexOptions.Compiled);

    private static readonly Regex SignatureIdPattern = new(
        @"^fwh\.core(?:\.v[0-9]+)?:signature\.[a-z0-9-]+$",
        RegexOptions.Compiled);

    /// <summary>
    /// Run the Phase-3 minimum validator over a single packet. Returns
    /// success on first pass; otherwise returns the accumulated error list
    /// so the caller can report all issues at once instead of fix-one-at-a-time.
    /// </summary>
    public static ValidationResult Validate(IdentityPacket packet)
    {
        if (packet is null)
        {
            return new ValidationResult(false, new[] { "IdentityPacket is null." });
        }

        List<string> errors = new();

        if (packet.SchemaVersion != IdentityPacket.CurrentSchemaVersion)
        {
            errors.Add(
                $"SchemaVersion mismatch: expected {IdentityPacket.CurrentSchemaVersion}, " +
                $"got {packet.SchemaVersion}. v2+ requires a save-migration fixture per " +
                $"design/specs/save-migration-fixtures.md.");
        }

        if (string.IsNullOrEmpty(packet.PlayerId) || !PlayerIdPattern.IsMatch(packet.PlayerId))
        {
            errors.Add(
                $"PlayerId '{packet.PlayerId}' does not match format " +
                $"^fwh\\.core(?:\\.v[0-9]+)?:player_[0-9]{{5}}$. Pack-minor versions " +
                $"(v1.1 etc.) NEVER appear in entity IDs per ADR-0006 §content-pack-ID-rules.");
        }

        if (string.IsNullOrWhiteSpace(packet.DisplayNameFull))
        {
            errors.Add("DisplayNameFull is null/empty/whitespace.");
        }
        if (string.IsNullOrWhiteSpace(packet.DisplayNameShort))
        {
            errors.Add("DisplayNameShort is null/empty/whitespace.");
        }

        if (!IsRoleFamilyDefined(packet.RoleFamily))
        {
            errors.Add(
                $"RoleFamily {(byte)packet.RoleFamily} is not a defined enum value. " +
                $"Valid: 1-8 per RoleFamily enum.");
        }

        if (string.IsNullOrEmpty(packet.SourcePackVersion))
        {
            errors.Add("SourcePackVersion is null/empty.");
        }

        ValidateSignatureCandidates(packet.SignatureCandidates, errors);
        ValidateGenes(packet.Genes, errors);

        return errors.Count == 0
            ? ValidationResult.Valid
            : new ValidationResult(false, errors.ToArray());
    }

    private static void ValidateGenes(IdentityPacketGenes genes, List<string> errors)
    {
        // Per pr-review-toolkit:type-design-analyzer 2026-04-30: the Phase-3
        // validator was missing gene-range bounds. A fixture authoring error
        // (typo, sign flip, decimal-place miscount in Q32.32 raw long) would
        // have silently corrupted signature-affinity dispatch.
        //
        // Phase-3 gene values fall in [0, 1] per ADR-0006 §gene-model-locked.
        // Phase-4 schema-v2 may introduce signed [-1, +1] fields (GrowthCurve,
        // Mentality); when they land, this check splits per field rather
        // than the current uniform clamp.
        if (genes is null)
        {
            errors.Add("Genes is null. Phase-3 schema requires the IdentityPacketGenes record.");
            return;
        }

        long oneRaw = Fixed.One.RawValue;
        ValidateGeneRange("FastTwitchRawQ32", genes.FastTwitchRawQ32, oneRaw, errors);
        ValidateGeneRange("PatternRecognitionRawQ32", genes.PatternRecognitionRawQ32, oneRaw, errors);
        ValidateGeneRange("DecisionVelocityRawQ32", genes.DecisionVelocityRawQ32, oneRaw, errors);
        ValidateGeneRange("FirstTouchRawQ32", genes.FirstTouchRawQ32, oneRaw, errors);
        ValidateGeneRange("StrikingRawQ32", genes.StrikingRawQ32, oneRaw, errors);
        ValidateGeneRange("LeftFootRawQ32", genes.LeftFootRawQ32, oneRaw, errors);
    }

    private static void ValidateGeneRange(
        string fieldName, long valueRaw, long oneRaw, List<string> errors)
    {
        if (valueRaw < 0L || valueRaw > oneRaw)
        {
            errors.Add(
                $"Genes.{fieldName} raw value {valueRaw} outside [0, Fixed.One.RawValue={oneRaw}]. " +
                $"Phase-3 gene values are clamped to [0, 1] per ADR-0006 §gene-model-locked. " +
                $"Phase-4 schema-v2 may introduce signed [-1, +1] fields (GrowthCurve, Mentality).");
        }
    }

    private static void ValidateSignatureCandidates(
        IReadOnlyList<SignatureCandidate> candidates,
        List<string> errors)
    {
        if (candidates is null)
        {
            errors.Add("SignatureCandidates is null. Use an empty array for zero-affinity players.");
            return;
        }

        if (candidates.Count > 3)
        {
            errors.Add(
                $"SignatureCandidates count {candidates.Count} exceeds Phase-3 maximum of 3 " +
                $"per ADR-0006 §affinity-count-distribution.");
        }

        long oneRaw = Fixed.One.RawValue;
        for (int i = 0; i < candidates.Count; i++)
        {
            SignatureCandidate candidate = candidates[i];

            if (string.IsNullOrEmpty(candidate.SignatureId)
                || !SignatureIdPattern.IsMatch(candidate.SignatureId))
            {
                errors.Add(
                    $"SignatureCandidates[{i}].SignatureId '{candidate.SignatureId}' does " +
                    $"not match format ^fwh\\.core(?:\\.v[0-9]+)?:signature\\.[a-z0-9-]+$ " +
                    $"per ADR-0005 §SignatureSO-ID-format.");
            }

            if (candidate.AffinityWeightRaw < 0L || candidate.AffinityWeightRaw > oneRaw)
            {
                errors.Add(
                    $"SignatureCandidates[{i}].AffinityWeightRaw {candidate.AffinityWeightRaw} " +
                    $"outside [0, Fixed.One.RawValue={oneRaw}]. Q32.32 affinity weights are " +
                    $"clamped to [0, 1] per ADR-0006.");
            }
        }
    }

    /// <summary>
    /// True iff the byte-backed enum value is a NAMED catalog member.
    /// Uses <see cref="Enum.IsDefined(Type, object)"/> rather than a
    /// contiguous-range byte check (per feature-dev:code-reviewer 2026-04-30):
    /// a Phase-4 schema bump that introduces a non-contiguous enum value
    /// (e.g., gap insertion for serialization-shape reasons) would silently
    /// re-admit any byte in <c>[1, current-max]</c> if we kept the range
    /// check. <c>Enum.IsDefined</c> is correct under any future enum layout.
    /// </summary>
    private static bool IsRoleFamilyDefined(RoleFamily role)
        => Enum.IsDefined(typeof(RoleFamily), role);
}

/// <summary>
/// Validator outcome. Caller checks <see cref="IsValid"/>; if false,
/// <see cref="Errors"/> lists every reason. Bool-bag return shape matches
/// <c>scripts/fw verify-unity-plugins</c>'s drift-counter style — caller
/// gets the full failure surface in one pass instead of fix-one-and-retry.
///
/// <para>
/// <see cref="Errors"/> is exposed as <see cref="IReadOnlyList{T}"/>
/// (not <c>string[]</c>) per pr-review-toolkit:type-design-analyzer
/// 2026-04-30: a mutable array escape would let a caller mutate the
/// shared <see cref="Valid"/> singleton's empty array.
/// </para>
/// </summary>
public readonly record struct ValidationResult(bool IsValid, IReadOnlyList<string> Errors)
{
    public static readonly ValidationResult Valid =
        new(true, System.Array.Empty<string>());
}

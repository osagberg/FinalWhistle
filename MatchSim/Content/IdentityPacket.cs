using System.Collections.Generic;

namespace FinalWhistle.MatchSim.Content;

/// <summary>
/// Phase-3 minimum subset of the IdentityPacket schema per ADR-0006
/// (Accepted 2026-04-24). Stable identity + signature affinity carriers +
/// a 6-field gene subset that the 3 active Phase-3 signatures consult for
/// affinity biasing. Full 22-field gene model + phenotype-label catalog +
/// rivalry/lineage metadata are deferred to Phase 4+ when the AI Content
/// Compiler pipeline lands.
///
/// <para>
/// <strong>Schema version 1.</strong> Bumps require both a code change AND
/// a save-migration fixture per <c>design/specs/save-migration-fixtures.md</c>
/// 4-test discipline. Adding new fields with defaults is forward-compatible;
/// removing/renaming/retyping an existing field is a v2 bump.
/// </para>
///
/// <para>
/// <strong>Serialization shape</strong> (per the Phase-3 hand-rolled
/// <c>Content.Json.IdentityPacketParser</c> + ADR-0006 §canonical-JSON-rules;
/// <c>System.Text.Json</c> was removed in the Codex round-7 refactor
/// 2026-04-30 because STJ + transitive deps don't ship in Unity 6's Mono
/// runtime):
/// </para>
/// <list type="bullet">
///   <item><description>2-space indent, structural-key order (JSON canon).</description></item>
///   <item><description>Q32.32 fields stored as raw <c>long</c> integers (no floats).</description></item>
///   <item><description><see cref="RoleFamily"/> serializes as a string; numeric encoding rejected.</description></item>
/// </list>
///
/// <para>
/// <strong>Architecture note:</strong> ADR-0006 envisions a 4-project split
/// (<c>Content.Contracts</c>, <c>Content.Compiler</c>, <c>Content.Validator</c>,
/// <c>Content.UnityImport</c>) at Phase-6 maturity. Phase-3 collapses these
/// into a single <c>FinalWhistle.MatchSim.Content</c> namespace under the
/// MatchSim asmdef — pure-C# class library; netstandard2.1; zero
/// UnityEngine references. The 4-project split is a Phase-4+ refactor when
/// the compiler + Unity-side import need explicit boundaries.
/// </para>
/// </summary>
public sealed record IdentityPacket
{
    /// <summary>
    /// Locked schema version for the Phase-3 minimum subset. Validator
    /// rejects any value other than this. Bumping requires a migration
    /// fixture per <c>save-migration-fixtures.md</c>.
    /// </summary>
    public const ushort CurrentSchemaVersion = 1;

    /// <summary>Stable content-pack-qualified player ID. Never mutates once shipped.</summary>
    /// <remarks>
    /// Format <c>^fwh\.core(?:\.v[0-9]+)?:player_[0-9]{5}$</c> per ADR-0006
    /// §content-pack-ID-rules. Pack-minor versions (v1.1 etc.) NEVER appear
    /// in entity IDs; only major-pack namespace (.vN) is permitted.
    /// </remarks>
    public string PlayerId { get; init; } = string.Empty;

    /// <summary>Banned-term-lint TARGET (per ADR-0006 + design/ui-vocabulary.md).</summary>
    public string DisplayNameFull { get; init; } = string.Empty;

    /// <summary>Banned-term-lint TARGET. Short form for compact UI surfaces.</summary>
    public string DisplayNameShort { get; init; } = string.Empty;

    /// <summary>Role family per <c>design/signatures.md</c> §role-family-catalog.</summary>
    public RoleFamily RoleFamily { get; init; }

    /// <summary>0–3 entries; affinity-weight load-bearing for signature dispatch.</summary>
    public IReadOnlyList<SignatureCandidate> SignatureCandidates { get; init; }
        = System.Array.Empty<SignatureCandidate>();

    /// <summary>Phase-3 minimum gene subset (6 of 22 fields).</summary>
    public IdentityPacketGenes Genes { get; init; } = new();

    /// <summary>Locked at <see cref="CurrentSchemaVersion"/> for v1 fixtures.</summary>
    public ushort SchemaVersion { get; init; } = CurrentSchemaVersion;

    /// <summary>Manifest cross-ref. Pack-minor versions live HERE, NOT in <see cref="PlayerId"/>.</summary>
    public string SourcePackVersion { get; init; } = string.Empty;
}

/// <summary>
/// Player-side carrier of signature affinity per ADR-0005 + ADR-0006.
/// <c>SignatureSO</c> (Phase-4 type — not yet shipped) is the definition
/// of <em>what</em> the signature is; <see cref="IdentityPacket"/>'s
/// <see cref="SignatureCandidate"/> records are <em>who can awaken it</em> with <em>what likelihood</em>. // ui-lint:allow term="awakens" reason="signature-mechanic vocabulary referencing ADR-0005 SignatureAwakened MemoryEvent class name; not player-facing UI copy" reviewer="osagberg"
/// </summary>
public readonly record struct SignatureCandidate
{
    /// <summary>
    /// Content-pack-qualified signature ID. Format
    /// <c>^fwh\.core(?:\.v[0-9]+)?:signature\.[a-z0-9-]+$</c> per ADR-0005;
    /// validator resolves at load time.
    /// </summary>
    public string SignatureId { get; init; }

    /// <summary>
    /// Q32.32 affinity weight in <c>[0, 1]</c>. Stored as raw <c>long</c>
    /// per ADR-0006 §canonical-JSON-rules (no floats); decode via
    /// <see cref="Sim.Fixed.FromRaw(long)"/>.
    /// </summary>
    public long AffinityWeightRaw { get; init; }
}

/// <summary>
/// Phase-3 minimum gene subset. The 3 active Phase-3 signatures consult
/// these 6 fields for affinity biasing per ADR-0006 §roll-procedure step 3:
///
/// <list type="bullet">
///   <item><description>#13 First-time diagonal switch (CM) — <see cref="PatternRecognitionRawQ32"/> + <see cref="DecisionVelocityRawQ32"/>.</description></item>
///   <item><description>#20 Low cutback from the byline (Winger) — <see cref="FastTwitchRawQ32"/> + <see cref="FirstTouchRawQ32"/> + <see cref="LeftFootRawQ32"/>.</description></item>
///   <item><description>#22 Blind-side near-post run (Striker) — <see cref="FastTwitchRawQ32"/> + <see cref="StrikingRawQ32"/>.</description></item>
/// </list>
///
/// The remaining 16 gene fields (HeightCeiling, FrameDensity, StaminaRecovery,
/// GrowthCurve, AgingCurve, InjuryResilience, ComposureFloor, LearningRate,
/// Ambition, Mentality, Aerial, DeadBall, FlowAccess, PeakCeilingHigh,
/// LateBloomer, AwakeningDormant) ship in schema v2 alongside the full
/// scout-disagreement system + balance-harness — Phase-4+.
///
/// <para>
/// All values are Q32.32 raw <c>long</c> integers. Decode via
/// <see cref="Sim.Fixed.FromRaw(long)"/>. Phase-3 gene values fall in
/// <c>[0, 1]</c> (mapped from the 0.0–1.0 clamped gene-model range);
/// Phase-4 may introduce signed <c>[-1, +1]</c> fields (GrowthCurve,
/// Mentality) at the v2 schema bump.
/// </para>
/// </summary>
public sealed record IdentityPacketGenes
{
    /// <summary>Physical: fast-twitch fibre ratio. Drives explosive sprint + acceleration affinity.</summary>
    public long FastTwitchRawQ32 { get; init; }

    /// <summary>Mental: pattern-recognition speed. Drives diagonal-switch + through-ball reads.</summary>
    public long PatternRecognitionRawQ32 { get; init; }

    /// <summary>Mental: decision velocity (delay between read and execute).</summary>
    public long DecisionVelocityRawQ32 { get; init; }

    /// <summary>Technical: first-touch quality. Drives byline carry + finishing touch.</summary>
    public long FirstTouchRawQ32 { get; init; }

    /// <summary>Technical: striking quality. Drives shot xG + near-post finishing.</summary>
    public long StrikingRawQ32 { get; init; }

    /// <summary>Technical: left-foot affinity. Drives inverted-winger foot preference.</summary>
    public long LeftFootRawQ32 { get; init; }
}

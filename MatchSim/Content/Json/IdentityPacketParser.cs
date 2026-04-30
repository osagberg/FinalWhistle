using System;
using System.Collections.Generic;
using System.IO;

namespace FinalWhistle.MatchSim.Content.Json;

/// <summary>
/// Schema-strict <see cref="IdentityPacket"/> parser per ADR-0006 +
/// Codex round-7 P1#2 (2026-04-30). Replaces <c>System.Text.Json</c>
/// for two reasons:
/// <list type="number">
///   <item><description><strong>Unity loadability</strong> — STJ + transitive
///       deps (<c>System.Memory</c>, <c>System.Buffers</c>,
///       <c>System.Text.Encodings.Web</c>, <c>Microsoft.Bcl.AsyncInterfaces</c>,
///       <c>System.Threading.Tasks.Extensions</c>, etc.) don't ship in
///       Unity 6's Mono runtime. STJ-referenced MatchSim DLL fails to load
///       in Unity (Codex round-7 P1).</description></item>
///   <item><description><strong>Strict by design</strong> — STJ defaults
///       silently accept typoed field names (deserialize-into-default-zero)
///       and silently accept numeric values for string-converter enums
///       (Codex round-7 P1#2 + P2). A schema-aware hand-rolled parser is
///       strict by construction: every accepted field is whitelisted,
///       every required field's presence is verified, RoleFamily MUST be
///       a string, duplicate keys are rejected.</description></item>
/// </list>
///
/// <para>
/// <strong>What this parser rejects</strong> (with descriptive
/// <see cref="InvalidDataException"/>):
/// </para>
/// <list type="bullet">
///   <item><description>Unknown top-level fields (typos like <c>FastTwichRawQ32</c>).</description></item>
///   <item><description>Missing required fields (every Phase-3 field must be present).</description></item>
///   <item><description>Duplicate keys (per-object).</description></item>
///   <item><description>Numeric RoleFamily (<c>"RoleFamily": 7</c>); string-only enum encoding.</description></item>
///   <item><description>Floats / decimals / exponents in numeric fields.</description></item>
///   <item><description>Trailing commas, malformed JSON, unsupported escapes.</description></item>
/// </list>
///
/// <para>
/// Phase-4+ may swap this for a more comprehensive content-pack parser
/// (probably a thin wrapper over a vetted library available in Unity 6's
/// runtime, OR via the AI Content Compiler pipeline's bake-time output).
/// </para>
/// </summary>
internal static class IdentityPacketParser
{
    /// <summary>
    /// Parse + validate-shape an IdentityPacket from JSON text. Throws
    /// <see cref="InvalidDataException"/> on any malformed input (unknown
    /// field, missing field, duplicate key, wrong type, malformed JSON).
    /// Does NOT run the semantic validator (<c>IdentityPacketValidator.Validate</c>);
    /// caller chains those.
    /// </summary>
    public static IdentityPacket Parse(string jsonContent)
    {
        if (jsonContent is null)
        {
            throw new ArgumentNullException(nameof(jsonContent));
        }
        var reader = new JsonReader(jsonContent);
        IdentityPacket packet = ReadIdentityPacket(reader);
        reader.SkipWhitespace();
        if (!reader.IsAtEnd)
        {
            throw reader.Fail("trailing content after IdentityPacket close-brace");
        }
        return packet;
    }

    private static readonly HashSet<string> RequiredTopLevelFields = new(StringComparer.Ordinal)
    {
        "PlayerId",
        "DisplayNameFull",
        "DisplayNameShort",
        "RoleFamily",
        "SignatureCandidates",
        "Genes",
        "SchemaVersion",
        "SourcePackVersion",
    };

    private static readonly HashSet<string> RequiredGeneFields = new(StringComparer.Ordinal)
    {
        "FastTwitchRawQ32",
        "PatternRecognitionRawQ32",
        "DecisionVelocityRawQ32",
        "FirstTouchRawQ32",
        "StrikingRawQ32",
        "LeftFootRawQ32",
    };

    private static readonly HashSet<string> RequiredSignatureCandidateFields = new(StringComparer.Ordinal)
    {
        "SignatureId",
        "AffinityWeightRaw",
    };

    private static IdentityPacket ReadIdentityPacket(JsonReader reader)
    {
        reader.Expect('{');
        var seen = new HashSet<string>(StringComparer.Ordinal);

        string? playerId = null;
        string? displayNameFull = null;
        string? displayNameShort = null;
        RoleFamily roleFamily = default;
        IReadOnlyList<SignatureCandidate>? signatureCandidates = null;
        IdentityPacketGenes? genes = null;
        ushort schemaVersion = 0;
        string? sourcePackVersion = null;

        bool first = true;
        while (true)
        {
            char next = reader.PeekNonWhitespace();
            if (next == '}')
            {
                reader.Expect('}');
                break;
            }
            if (!first)
            {
                reader.Expect(',');
            }
            first = false;

            string key = reader.ReadString();
            if (!seen.Add(key))
            {
                throw reader.Fail($"duplicate field '{key}' in IdentityPacket");
            }
            if (!RequiredTopLevelFields.Contains(key))
            {
                throw reader.Fail(
                    $"unknown field '{key}' in IdentityPacket. Phase-3 schema accepts: " +
                    string.Join(", ", RequiredTopLevelFields));
            }
            reader.Expect(':');

            switch (key)
            {
                case "PlayerId":
                    playerId = reader.ReadString();
                    break;
                case "DisplayNameFull":
                    displayNameFull = reader.ReadString();
                    break;
                case "DisplayNameShort":
                    displayNameShort = reader.ReadString();
                    break;
                case "RoleFamily":
                    // RoleFamily MUST be string; numeric is rejected per
                    // Codex round-7 P2 (2026-04-30).
                    if (reader.PeekNonWhitespace() != '"')
                    {
                        throw reader.Fail(
                            "RoleFamily must be a JSON string (e.g. \"Striker\"); " +
                            "numeric encoding is rejected per the canonical-JSON contract");
                    }
                    string roleStr = reader.ReadString();
                    if (!Enum.TryParse(roleStr, ignoreCase: false, out roleFamily)
                        || !Enum.IsDefined(typeof(RoleFamily), roleFamily))
                    {
                        throw reader.Fail(
                            $"unknown RoleFamily '{roleStr}'. Valid: " +
                            string.Join(", ", Enum.GetNames(typeof(RoleFamily))));
                    }
                    break;
                case "SignatureCandidates":
                    signatureCandidates = ReadSignatureCandidates(reader);
                    break;
                case "Genes":
                    genes = ReadGenes(reader);
                    break;
                case "SchemaVersion":
                    long sv = reader.ReadLong();
                    if (sv < 0 || sv > ushort.MaxValue)
                    {
                        throw reader.Fail($"SchemaVersion {sv} outside ushort range [0, 65535]");
                    }
                    schemaVersion = (ushort)sv;
                    break;
                case "SourcePackVersion":
                    sourcePackVersion = reader.ReadString();
                    break;
                default:
                    throw reader.Fail($"internal: unhandled field '{key}'");
            }
        }

        // Required-field presence check.
        EnsureAllPresent(seen, RequiredTopLevelFields, "IdentityPacket", reader);

        return new IdentityPacket
        {
            PlayerId = playerId ?? string.Empty,
            DisplayNameFull = displayNameFull ?? string.Empty,
            DisplayNameShort = displayNameShort ?? string.Empty,
            RoleFamily = roleFamily,
            SignatureCandidates = signatureCandidates ?? Array.Empty<SignatureCandidate>(),
            Genes = genes ?? new IdentityPacketGenes(),
            SchemaVersion = schemaVersion,
            SourcePackVersion = sourcePackVersion ?? string.Empty,
        };
    }

    private static IReadOnlyList<SignatureCandidate> ReadSignatureCandidates(JsonReader reader)
    {
        reader.Expect('[');
        var result = new List<SignatureCandidate>();

        bool first = true;
        while (true)
        {
            char next = reader.PeekNonWhitespace();
            if (next == ']')
            {
                reader.Expect(']');
                break;
            }
            if (!first)
            {
                reader.Expect(',');
            }
            first = false;

            result.Add(ReadSignatureCandidate(reader));
        }
        return result;
    }

    private static SignatureCandidate ReadSignatureCandidate(JsonReader reader)
    {
        reader.Expect('{');
        var seen = new HashSet<string>(StringComparer.Ordinal);
        string? sigId = null;
        long affinityRaw = 0;

        bool first = true;
        while (true)
        {
            char next = reader.PeekNonWhitespace();
            if (next == '}')
            {
                reader.Expect('}');
                break;
            }
            if (!first)
            {
                reader.Expect(',');
            }
            first = false;

            string key = reader.ReadString();
            if (!seen.Add(key))
            {
                throw reader.Fail($"duplicate field '{key}' in SignatureCandidate");
            }
            if (!RequiredSignatureCandidateFields.Contains(key))
            {
                throw reader.Fail(
                    $"unknown field '{key}' in SignatureCandidate. Accepts: " +
                    string.Join(", ", RequiredSignatureCandidateFields));
            }
            reader.Expect(':');

            switch (key)
            {
                case "SignatureId":
                    sigId = reader.ReadString();
                    break;
                case "AffinityWeightRaw":
                    // Presence enforcement lives in EnsureAllPresent below;
                    // no separate `affinitySeen` flag needed (per
                    // pr-review-toolkit:silent-failure-hunter 2026-04-30
                    // round-7 medium dead-code finding).
                    affinityRaw = reader.ReadLong();
                    break;
            }
        }

        EnsureAllPresent(seen, RequiredSignatureCandidateFields, "SignatureCandidate", reader);

        return new SignatureCandidate
        {
            SignatureId = sigId ?? string.Empty,
            AffinityWeightRaw = affinityRaw,
        };
    }

    private static IdentityPacketGenes ReadGenes(JsonReader reader)
    {
        reader.Expect('{');
        var seen = new HashSet<string>(StringComparer.Ordinal);
        long fastTwitch = 0;
        long patternReco = 0;
        long decisionVel = 0;
        long firstTouch = 0;
        long striking = 0;
        long leftFoot = 0;

        bool first = true;
        while (true)
        {
            char next = reader.PeekNonWhitespace();
            if (next == '}')
            {
                reader.Expect('}');
                break;
            }
            if (!first)
            {
                reader.Expect(',');
            }
            first = false;

            string key = reader.ReadString();
            if (!seen.Add(key))
            {
                throw reader.Fail($"duplicate field '{key}' in IdentityPacketGenes");
            }
            if (!RequiredGeneFields.Contains(key))
            {
                throw reader.Fail(
                    $"unknown field '{key}' in IdentityPacketGenes. Phase-3 accepts: " +
                    string.Join(", ", RequiredGeneFields));
            }
            reader.Expect(':');

            long value = reader.ReadLong();
            switch (key)
            {
                case "FastTwitchRawQ32": fastTwitch = value; break;
                case "PatternRecognitionRawQ32": patternReco = value; break;
                case "DecisionVelocityRawQ32": decisionVel = value; break;
                case "FirstTouchRawQ32": firstTouch = value; break;
                case "StrikingRawQ32": striking = value; break;
                case "LeftFootRawQ32": leftFoot = value; break;
            }
        }

        EnsureAllPresent(seen, RequiredGeneFields, "IdentityPacketGenes", reader);

        return new IdentityPacketGenes
        {
            FastTwitchRawQ32 = fastTwitch,
            PatternRecognitionRawQ32 = patternReco,
            DecisionVelocityRawQ32 = decisionVel,
            FirstTouchRawQ32 = firstTouch,
            StrikingRawQ32 = striking,
            LeftFootRawQ32 = leftFoot,
        };
    }

    /// <summary>
    /// Verify every required-field name appears in <paramref name="seen"/>.
    ///
    /// <para>
    /// <strong>Caller-side precondition</strong> (per feature-dev:code-reviewer
    /// 2026-04-30 round-7 review, confidence-88 finding): every key inserted
    /// into <paramref name="seen"/> MUST first pass through the whitelist
    /// gate <c>required.Contains(key)</c>. If a future caller skips that
    /// gate, the fast-path equality check on <see cref="HashSet{T}.Count"/>
    /// would silently pass on a wrong-keys-but-equal-cardinality set.
    /// All current callers (<c>ReadIdentityPacket</c>, <c>ReadSignatureCandidate</c>,
    /// <c>ReadGenes</c>) honor this; the precondition is non-local, hence
    /// the explicit doc-comment.
    /// </para>
    /// </summary>
    private static void EnsureAllPresent(
        HashSet<string> seen,
        HashSet<string> required,
        string objectName,
        JsonReader reader)
    {
        if (seen.Count == required.Count) return;

        var missing = new List<string>();
        foreach (string field in required)
        {
            if (!seen.Contains(field)) missing.Add(field);
        }
        if (missing.Count > 0)
        {
            throw reader.Fail(
                $"{objectName} is missing required field(s): {string.Join(", ", missing)}. " +
                "Every Phase-3 field is required (no defaults; no optional fields in v1 schema).");
        }
    }
}

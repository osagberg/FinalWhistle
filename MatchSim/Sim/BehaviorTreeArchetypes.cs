using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using YamlDotNet.Serialization;
using YamlDotNet.Serialization.NamingConventions;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Loader + parser for <see cref="BehaviorTreeArchetype"/> instances. The
/// canonical authoring format is YAML embedded as a resource in
/// <c>MatchSim.csproj</c> under <c>Content/archetypes/*.yaml</c>; mod packs
/// will eventually load YAML from external pack manifests through this
/// same parser.
///
/// <para>
/// <strong>Determinism:</strong> parsing is deterministic — same YAML input
/// produces the same archetype byte-for-byte. <see cref="Load"/> caches
/// parsed archetypes by name so repeated calls don't re-parse; the cache
/// is keyed by ordinal name comparison.
/// </para>
///
/// <para>
/// <strong>Built-in archetypes</strong> (Phase-3 Month-3 slice):
/// <c>direct-pressing</c> (4-4-2 high press) +
/// <c>low-block-counter</c> (4-5-1 deep block + fast counter).
/// </para>
/// </summary>
public static class BehaviorTreeArchetypes
{
    /// <summary>The two Phase-3 Month-3 archetype names.</summary>
    public static readonly IReadOnlyList<string> BuiltInNames = new[]
    {
        "direct-pressing",
        "low-block-counter",
    };

    private static readonly ConcurrentDictionary<string, BehaviorTreeArchetype> Cache = new(StringComparer.Ordinal);

    /// <summary>
    /// Load a built-in archetype by name. Names match the YAML file stems
    /// in <c>MatchSim/Content/archetypes/</c>. Throws if the archetype is
    /// not found or the YAML is malformed.
    /// </summary>
    public static BehaviorTreeArchetype Load(string name)
    {
        if (string.IsNullOrWhiteSpace(name))
        {
            throw new ArgumentException("Archetype name must be non-empty.", nameof(name));
        }
        return Cache.GetOrAdd(name, n =>
        {
            string yaml = ReadEmbeddedResource(n);
            return Parse(yaml);
        });
    }

    /// <summary>
    /// Parse YAML content into a <see cref="BehaviorTreeArchetype"/>. Used
    /// directly by tests + future mod-pack loader; <see cref="Load"/> is the
    /// usual entry point for built-in archetypes.
    /// </summary>
    public static BehaviorTreeArchetype Parse(string yamlContent)
    {
        if (yamlContent is null)
        {
            throw new ArgumentNullException(nameof(yamlContent));
        }

        IDeserializer deserializer = new DeserializerBuilder()
            .WithNamingConvention(UnderscoredNamingConvention.Instance)
            .IgnoreUnmatchedProperties()
            .Build();

        ArchetypeYaml dto = deserializer.Deserialize<ArchetypeYaml>(yamlContent)
            ?? throw new InvalidDataException("YAML deserialized to null — file is empty or unparseable.");

        if (dto.Formation is null)
        {
            throw new InvalidDataException($"Archetype '{dto.Name}' has no formation block.");
        }

        FormationSlot[] formation = new FormationSlot[dto.Formation.Count];
        for (int i = 0; i < dto.Formation.Count; i++)
        {
            FormationSlotYaml slotYaml = dto.Formation[i];
            formation[i] = new FormationSlot(
                rosterSlot:       slotYaml.RosterSlot,
                role:             slotYaml.Role ?? string.Empty,
                homeBasePosition: new Vector3Fixed(
                    Fixed.FromInt(slotYaml.X),
                    Fixed.Zero,                          // ground players; Y = 0 invariant
                    Fixed.FromInt(slotYaml.Z))
            );
        }

        return new BehaviorTreeArchetype(
            name:                dto.Name ?? string.Empty,
            description:         dto.Description ?? string.Empty,
            formation:           formation,
            pressRadiusMetres:   ParsePositiveFraction(dto.PressRadiusMetres, nameof(dto.PressRadiusMetres)),
            buildupSpeedFactor:  ParsePositiveFraction(dto.BuildupSpeedFactor, nameof(dto.BuildupSpeedFactor))
        );
    }

    /// <summary>
    /// Convert a YAML-decoded decimal string (e.g. <c>"0.95"</c>) into a
    /// <see cref="Fixed"/>. We accept either a plain integer or a decimal
    /// fraction with up to 4 fractional digits — sufficient for archetype
    /// authoring without dragging Fixed.Parse's full canonical-decimal
    /// surface into the YAML reader.
    /// </summary>
    private static Fixed ParsePositiveFraction(string? value, string fieldName)
    {
        if (string.IsNullOrWhiteSpace(value))
        {
            throw new InvalidDataException($"Archetype field '{fieldName}' is missing or empty.");
        }
        // Round-trip via Fixed.Parse (canonical 10-fractional-digit form
        // accepts shorter inputs). Validates positive.
        Fixed result = Fixed.Parse(value!);
        if (result <= Fixed.Zero)
        {
            throw new InvalidDataException($"Archetype field '{fieldName}' must be positive; got {value}.");
        }
        return result;
    }

    private static string ReadEmbeddedResource(string archetypeName)
    {
        Assembly asm = typeof(BehaviorTreeArchetypes).Assembly;
        string resourceName = $"FinalWhistle.MatchSim.Content.archetypes.{archetypeName}.yaml";

        using Stream? stream = asm.GetManifestResourceStream(resourceName);
        if (stream is null)
        {
            string available = string.Join(", ", asm.GetManifestResourceNames());
            throw new FileNotFoundException(
                $"Embedded YAML resource not found: '{resourceName}'. Available resources: [{available}]");
        }
        using StreamReader reader = new(stream);
        return reader.ReadToEnd();
    }

    #region YAML data-transfer-objects (deserialization shape only)

    /// <summary>Mirrors the top-level YAML shape of an archetype file. Internal — not part of the public API.</summary>
    internal sealed class ArchetypeYaml
    {
        public string? Name { get; set; }
        public string? Description { get; set; }
        public List<FormationSlotYaml>? Formation { get; set; }
        public string? PressRadiusMetres { get; set; }
        public string? BuildupSpeedFactor { get; set; }
    }

    /// <summary>Mirrors a single YAML formation entry. Coordinates are integers in metres.</summary>
    internal sealed class FormationSlotYaml
    {
        public byte RosterSlot { get; set; }
        public string? Role { get; set; }
        public int X { get; set; }
        public int Z { get; set; }
    }

    #endregion
}

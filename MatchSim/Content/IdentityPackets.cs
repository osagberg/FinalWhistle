using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.IO;
using System.Reflection;
using System.Text.Json;

namespace FinalWhistle.MatchSim.Content;

/// <summary>
/// Static loader for the Phase-3 hand-authored <see cref="IdentityPacket"/>
/// JSON fixtures embedded under
/// <c>MatchSim/Content/identity-packets/&lt;archetype&gt;/&lt;jersey&gt;.json</c>.
/// Mirrors the existing <see cref="Sim.BehaviorTreeArchetypes"/> embedded-
/// resource loader pattern.
///
/// <para>
/// <strong>Resource-name format</strong> (locked at the csproj
/// <c>EmbeddedResource</c> block; mirrors the archetype LogicalName fix
/// from Codex round-4):
/// </para>
/// <c>FinalWhistle.MatchSim.Content.identity-packets.&lt;archetype&gt;.&lt;jersey:D2&gt;.json</c>
///
/// <para>
/// <strong>Cache lifecycle:</strong> packets are cached in a process-
/// lifetime <see cref="ConcurrentDictionary{TKey,TValue}"/> after first
/// load. Tests must not mutate loaded packets (the records are immutable
/// by design — <c>{ get; init; }</c>). Any future feature that needs
/// mutation must work on a clone, not the cached instance.
/// </para>
///
/// <para>
/// <strong>Validation policy:</strong> every <see cref="Load"/> /
/// <see cref="Parse"/> call runs the Phase-3 validator
/// (<see cref="IdentityPacketValidator.Validate"/>) AFTER deserialization.
/// Invalid packets throw <see cref="InvalidDataException"/> with the full
/// validator-error list in the message. The cache only holds validated
/// packets — callers never see a packet that failed validation.
/// </para>
/// </summary>
public static class IdentityPackets
{
    private static readonly ConcurrentDictionary<string, IdentityPacket> Cache = new();

    /// <summary>
    /// Built-in Phase-3 archetype roster names. Iterating this list +
    /// jerseys 1-11 yields the full 22-packet smoke set.
    /// </summary>
    public static readonly IReadOnlyList<string> BuiltInArchetypeNames = new[]
    {
        "direct-pressing",
        "low-block-counter",
    };

    /// <summary>Number of players per archetype roster (locked at 11 = a starting XI).</summary>
    public const byte PlayersPerArchetype = 11;

    /// <summary>
    /// Load + cache the IdentityPacket for the given archetype + jersey
    /// number from the embedded JSON resource. Throws on missing resource
    /// or validation failure. Subsequent calls with the same arguments
    /// return the cached instance.
    /// </summary>
    /// <param name="archetype">Roster name (e.g. <c>"direct-pressing"</c>).</param>
    /// <param name="jerseyNumber">1–11; jerseys outside this range have no fixture and throw.</param>
    public static IdentityPacket Load(string archetype, byte jerseyNumber)
    {
        if (string.IsNullOrEmpty(archetype))
        {
            throw new ArgumentException("Archetype must be non-empty.", nameof(archetype));
        }
        if (jerseyNumber < 1 || jerseyNumber > PlayersPerArchetype)
        {
            // Lowercase 'jerseyNumber' avoids the A.5 \bJersey\b place-analogue
            // ban; this is parameter-name reference text, not player-facing copy.
            throw new ArgumentOutOfRangeException(
                nameof(jerseyNumber), jerseyNumber,
                $"jerseyNumber must be 1-{PlayersPerArchetype} (a starting XI roster slot).");
        }

        string cacheKey = $"{archetype}/{jerseyNumber:D2}";
        return Cache.GetOrAdd(cacheKey, _ => LoadAndValidate(archetype, jerseyNumber));
    }

    /// <summary>
    /// Load + validate every Phase-3 fixture (22 packets across the 2 archetypes).
    /// Used by the round-trip test harness + the future balance-harness
    /// sweep entry-point. Invalid fixtures fail-fast on first error.
    /// </summary>
    public static IReadOnlyList<IdentityPacket> LoadAll()
    {
        List<IdentityPacket> result = new(BuiltInArchetypeNames.Count * PlayersPerArchetype);
        foreach (string archetype in BuiltInArchetypeNames)
        {
            for (byte jersey = 1; jersey <= PlayersPerArchetype; jersey++)
            {
                result.Add(Load(archetype, jersey));
            }
        }
        return result;
    }

    /// <summary>
    /// Parse + validate a JSON string directly. Bypasses the embedded-
    /// resource lookup; used by tests that want to assert validator
    /// behavior on hand-crafted payloads without touching the cache.
    ///
    /// <para>
    /// <strong>Cache-lifecycle note</strong> (per feature-dev:code-reviewer
    /// 2026-04-30): the cache is write-once via <see cref="Load"/>. Calling
    /// <see cref="Parse"/> with a valid packet does NOT populate the cache,
    /// even if the same JSON exists as an embedded fixture — the resulting
    /// packet is a fresh instance. Tests asserting cache hits via
    /// <c>Assert.Same</c> must use <see cref="Load"/>, not
    /// <see cref="Parse"/>.
    /// </para>
    /// </summary>
    public static IdentityPacket Parse(string jsonContent)
    {
        if (string.IsNullOrWhiteSpace(jsonContent))
        {
            throw new ArgumentException("JSON content must be non-empty.", nameof(jsonContent));
        }

        IdentityPacket? packet = JsonSerializer.Deserialize<IdentityPacket>(jsonContent);
        if (packet is null)
        {
            throw new InvalidDataException(
                "JsonSerializer.Deserialize<IdentityPacket> returned null. " +
                "Likely a top-level JSON literal `null` rather than an object.");
        }

        ValidationResult result = IdentityPacketValidator.Validate(packet);
        if (!result.IsValid)
        {
            throw new InvalidDataException(
                $"IdentityPacket validation failed with {result.Errors.Count} error(s): " +
                $"\n  - {string.Join("\n  - ", result.Errors)}");
        }

        return packet;
    }

    private static IdentityPacket LoadAndValidate(string archetype, byte jerseyNumber)
    {
        string resourceName =
            $"FinalWhistle.MatchSim.Content.identity-packets.{archetype}.{jerseyNumber:D2}.json";
        Assembly asm = typeof(IdentityPackets).Assembly;

        using Stream? stream = asm.GetManifestResourceStream(resourceName);
        if (stream is null)
        {
            string available = string.Join(", ", asm.GetManifestResourceNames());
            throw new FileNotFoundException(
                $"Embedded IdentityPacket resource not found: '{resourceName}'. " +
                $"Available resources: [{available}]");
        }

        using StreamReader reader = new(stream);
        string json = reader.ReadToEnd();
        return Parse(json);
    }
}

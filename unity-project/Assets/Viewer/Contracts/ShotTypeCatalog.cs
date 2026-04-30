using System;
using System.Collections.Generic;

namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// Phase-3 minimum hard-coded catalog of <see cref="ShotTypeDefinition"/>s
    /// per ADR-0001 §"ShotTypeSO schema" + ADR-0008 §"ShotTypeDefinition" +
    /// <c>design/semantic-cinema.md</c>'s 7-shot vocabulary. Phase-4+
    /// <see cref="FinalWhistle.Viewer.Core"/> projects from real
    /// <c>ShotTypeSO</c> ScriptableObject assets via the
    /// <c>Viewer.ShotAuthoring</c> seam (per ADR-0008 §"Contract package
    /// boundary"); this catalog stands in until that landing.
    ///
    /// <para>
    /// <strong>Lives in <c>Viewer.Contracts</c></strong> (not
    /// <c>Viewer.Core</c>) per SPEC 2026-04-30 EventBridge-home entry +
    /// <c>.claude/rules/Scripts/Viewer/RULES.md</c>: the catalog produces
    /// pure-C# DTOs that the bridge consumes — the asmdef-level
    /// <c>noEngineReferences: true</c> flag makes a stray
    /// <c>using UnityEngine</c> here a compile error, enforcing
    /// ADR-0008's deterministic-conversion contract architecturally.
    /// Phase-4+ ScriptableObject projection in <c>Viewer.Core</c> is
    /// where Unity-side glue lives; the catalog itself is renderer-
    /// agnostic data.
    /// </para>
    /// </summary>
    public static class ShotTypeCatalog
    {
        // Phase-3 shot IDs match the recipe-key strings already in
        // SignaturePresentationRecipe so the bridge translation is a
        // direct lookup. Keeping the same identifiers in both layers
        // means a future authoring change in one surfaces a compile
        // error on the other (the validator can lint cross-doc identity).

        public const string ShotTacticalWide = "fwh.core:shot.tactical-wide";
        public const string ShotPlayerIsolation = "fwh.core:shot.player-isolation";
        public const string ShotPassShotImpact = "fwh.core:shot.pass-shot-impact";
        public const string ShotAftermathFreeze = "fwh.core:shot.aftermath-freeze";

        // Phase-3 reduce-motion variant IDs. Phase-4+ ScriptableObject
        // authoring will add the rest of the 7-shot vocabulary +
        // adapter-specific reduce-motion overrides.
        public const string ShotPlayerIsolationReduceMotion =
            "fwh.core:shot.player-isolation.reduce-motion";

        private static readonly Dictionary<string, ShotTypeDefinition> _byId;

        static ShotTypeCatalog()
        {
            // Phase-3 duration envelope: 3s = 180 ticks at 60Hz canonical
            // per design/breakthrough-moments.md §Q1 default tuning seed.
            // Aftermath-freeze gets 5s (the design doc's upper-bound for
            // genuinely high-stakes beats — breakthroughs qualify).
            const int Duration3s = 180;
            const int Duration4s = 240;
            const int Duration5s = 300;

            ShotTypeDefinition tacticalWide = new(
                id: ShotTacticalWide,
                category: ShotCategory.TacticalWide,
                durationTicks: Duration3s);

            ShotTypeDefinition playerIsolation = new(
                id: ShotPlayerIsolation,
                category: ShotCategory.PlayerIsolation,
                durationTicks: Duration3s,
                reduceMotionVariantId: ShotPlayerIsolationReduceMotion);

            ShotTypeDefinition playerIsolationReduceMotion = new(
                id: ShotPlayerIsolationReduceMotion,
                category: ShotCategory.PlayerIsolation,
                durationTicks: Duration3s);

            ShotTypeDefinition passShotImpact = new(
                id: ShotPassShotImpact,
                category: ShotCategory.PassShotImpact,
                durationTicks: Duration4s);

            ShotTypeDefinition aftermathFreeze = new(
                id: ShotAftermathFreeze,
                category: ShotCategory.AftermathFreeze,
                durationTicks: Duration5s);

            _byId = new Dictionary<string, ShotTypeDefinition>(StringComparer.Ordinal)
            {
                [tacticalWide.Id] = tacticalWide,
                [playerIsolation.Id] = playerIsolation,
                [playerIsolationReduceMotion.Id] = playerIsolationReduceMotion,
                [passShotImpact.Id] = passShotImpact,
                [aftermathFreeze.Id] = aftermathFreeze,
            };

            // Consistency assertion per pr-review-toolkit:type-design-analyzer
            // 2026-04-30 finding #3: every reduce-motion variant referenced
            // by a catalog entry must resolve within the catalog itself.
            // Catches Phase-4+ ScriptableObject-authoring drift the moment
            // a SO author types the wrong variant ID — fail at static init,
            // not at the first reduce-motion ViewerEvent surfaced via
            // EventBridge.
            foreach (KeyValuePair<string, ShotTypeDefinition> kv in _byId)
            {
                string? variantId = kv.Value.ReduceMotionVariantId;
                if (variantId is null) continue;
                if (!_byId.ContainsKey(variantId))
                {
                    throw new InvalidOperationException(
                        $"ShotTypeCatalog inconsistency: '{kv.Key}' references " +
                        $"reduce-motion variant '{variantId}' which is not registered " +
                        "in the catalog. Either register the variant or remove the reference.");
                }
            }
        }

        /// <summary>All registered shot types (read-only view).</summary>
        public static IReadOnlyCollection<ShotTypeDefinition> All => _byId.Values;

        public static ShotTypeDefinition Get(string id)
        {
            if (!_byId.TryGetValue(id, out ShotTypeDefinition? definition))
            {
                throw new KeyNotFoundException(
                    $"Unknown shot type ID: '{id}'. Phase-3 catalog contains: " +
                    $"{string.Join(", ", _byId.Keys)}.");
            }
            return definition;
        }

        public static bool TryGet(string id, out ShotTypeDefinition definition)
        {
            if (_byId.TryGetValue(id, out ShotTypeDefinition? found))
            {
                definition = found;
                return true;
            }
            definition = default!;
            return false;
        }
    }
}

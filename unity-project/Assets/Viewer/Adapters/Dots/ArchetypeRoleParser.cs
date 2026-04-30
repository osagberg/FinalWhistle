using System;
using FinalWhistle.MatchSim.Content;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Parses Phase-3 archetype-YAML role labels (<c>"GK"</c>, <c>"CB"</c>,
    /// <c>"LCB"</c>, <c>"ST"</c>, etc.) into the canonical
    /// <see cref="RoleFamily"/> enum. Pulled out of
    /// <see cref="IdentityTintTable"/> per pr-review-toolkit
    /// <c>type-design-analyzer</c> Slice-2 P1: a string→enum parser of
    /// MatchSim YAML conventions has no business living on a render-side
    /// ScriptableObject (the asmdef chain
    /// <c>Adapters.Dots → Core → Contracts</c> exists specifically so the
    /// Phase-4 IdentityPacket pipeline can't drift toward render-asset
    /// dependencies; a parser parked on the SO would invite that).
    ///
    /// <para>
    /// <strong>Phase-4 retirement:</strong> when the IdentityPacket-driven
    /// roster path lands, callers will receive <see cref="RoleFamily"/>
    /// directly from packet metadata + this helper retires. Phase-3
    /// archetype YAMLs are the only consumer; the helper bridges them
    /// to the role-family enum that <see cref="IdentityTintTable.Lookup"/>
    /// consumes.
    /// </para>
    ///
    /// <para>
    /// <strong>Case sensitivity:</strong> labels are uppercase per the
    /// authored YAML schema in <c>MatchSim/Content/archetypes/*.yaml</c>.
    /// Lowercase / mixed-case labels throw <see cref="ArgumentException"/>
    /// rather than silently <c>ToUpperInvariant</c> — a content-pack lint
    /// failure must surface at scene-load, not get silently coerced.
    /// </para>
    /// </summary>
    public static class ArchetypeRoleParser
    {
        /// <summary>
        /// Map a YAML role label to the canonical role family. Throws on
        /// unknown labels; do not catch-and-fallback (silent fallback to
        /// e.g. <see cref="RoleFamily.Striker"/> would mask a misauthored
        /// archetype YAML and bake the wrong tint into the scene).
        /// </summary>
        public static RoleFamily RoleFamilyForLabel(string label)
        {
            if (string.IsNullOrWhiteSpace(label))
            {
                throw new ArgumentException("Role label must be non-empty.", nameof(label));
            }
            return label switch
            {
                "GK" => RoleFamily.Goalkeeper,
                "CB" or "RCB" or "LCB" => RoleFamily.CentreBack,
                "RB" or "LB" => RoleFamily.FullBack,
                "CDM" or "DM" => RoleFamily.DefensiveMidfielder,
                "CM" or "RCM" or "LCM" => RoleFamily.CentralMidfielder,
                "CAM" or "AM" => RoleFamily.AttackingMidfielder,
                "RM" or "LM" or "RW" or "LW" => RoleFamily.Winger,
                "ST" or "RST" or "LST" or "CF" => RoleFamily.Striker,
                _ => throw new ArgumentException(
                    $"Unknown archetype role label '{label}'. Phase-3 supports GK / CB / RCB / LCB / RB / LB / CDM / DM / CM / RCM / LCM / CAM / AM / RM / LM / RW / LW / ST / RST / LST / CF (uppercase only — case-sensitive per archetype YAML schema).",
                    nameof(label)),
            };
        }
    }
}

namespace FinalWhistle.MatchSim.Content;

/// <summary>
/// The 8 role families per <c>design/signatures.md</c> §role-family-catalog.
/// Each <see cref="IdentityPacket"/> declares one role family; the 24-signature
/// catalog (3 per role × 8 families) keys off this enum.
///
/// <para>
/// <strong>Serialized as a string in JSON.</strong> The hand-rolled
/// <c>Content/Json/IdentityPacketParser</c> rejects numeric encoding
/// (<c>"RoleFamily": 7</c>) per Codex round-7 P2 (2026-04-30); only
/// canonical string values (<c>"Striker"</c>) are accepted. Numeric
/// ordering is NOT canonical — future role-family additions append at
/// the end and never reorder, so the enum-byte form would drift across
/// schema versions. The string form is stable.
/// </para>
/// </summary>
public enum RoleFamily : byte
{
    Goalkeeper = 1,
    CentreBack = 2,
    FullBack = 3,
    DefensiveMidfielder = 4,
    CentralMidfielder = 5,
    AttackingMidfielder = 6,
    Winger = 7,
    Striker = 8,
}

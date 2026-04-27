namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Which side of the match a player belongs to. Identical to ADR-0008
/// <c>Viewer.Contracts.TeamSide</c>; this is the sim-side definition that
/// the viewer-side enum will mirror exactly. Wire format: 1 byte
/// (<c>Home = 1</c>, <c>Away = 2</c>; zero is intentionally NOT a valid
/// side, so a default <c>(byte)0</c> in serialized state is detectable as
/// uninitialized).
/// </summary>
public enum TeamSide : byte
{
    /// <summary>Home team. Default starting side at kick-off.</summary>
    Home = 1,

    /// <summary>Away team.</summary>
    Away = 2,
}

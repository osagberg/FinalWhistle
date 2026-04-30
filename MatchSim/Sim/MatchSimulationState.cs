using System;
using System.Collections.Generic;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Mutable production state for the Phase-3 deterministic match loop. The
/// arrays are intentionally mutable because the runner overwrites player
/// snapshots in place each tick; callers must preserve roster order.
///
/// <para>
/// <strong>Phase-3 PitchRules extensions</strong> per SPEC 2026-04-28
/// PitchRules decisions-log entry: <see cref="HomeScore"/> /
/// <see cref="AwayScore"/> (byte; capacity for any realistic Phase-3 match),
/// <see cref="OutOfPlay"/> (per-tick transient flag), and
/// <see cref="KeyEvents"/> (append-only stream of significant events
/// canonically encoded for replay-corpus hashing). All four fields are
/// canonical state — see <c>MatchCanonicalState.Write</c>.
/// </para>
/// </summary>
public sealed class MatchSimulationState
{
    /// <summary>Current absolute simulation tick.</summary>
    public Tick CurrentTick { get; set; }

    /// <summary>Current canonical ball state.</summary>
    public BallState Ball { get; set; }

    /// <summary>Home players in stable roster order, length 11.</summary>
    public PlayerState[] HomeTeam { get; }

    /// <summary>Away players in stable roster order, length 11.</summary>
    public PlayerState[] AwayTeam { get; }

    /// <summary>
    /// Home team score (number of goals scored). byte capacity = 0-255;
    /// the realistic Phase-3 ceiling is ~10-15 per side. <see cref="MatchRules.Step"/>
    /// throws if this would overflow rather than silently wrap.
    /// </summary>
    public byte HomeScore { get; set; }

    /// <summary>Away team score. Same byte-capacity / overflow contract as <see cref="HomeScore"/>.</summary>
    public byte AwayScore { get; set; }

    /// <summary>
    /// Per-tick transient flag set by <see cref="MatchRules.Step"/> when an
    /// out-of-play event fires THIS tick. Reset to <see cref="OutOfPlay.InPlay"/>
    /// at the start of every <see cref="MatchRules.Step"/> call. The
    /// persistent record of "what restarts have happened" lives in
    /// <see cref="KeyEvents"/>; this flag exists for tick-local consumers
    /// (the dots viewer adapter overlays the restart marker on the tick the
    /// event fires).
    /// </summary>
    public OutOfPlay OutOfPlay { get; set; }

    /// <summary>
    /// Append-only stream of significant match events: goals + restart
    /// emissions + Phase-3 signature executions. Entries are written in
    /// canonical order during <see cref="MatchRules.Step"/> +
    /// <see cref="SignatureRules.Step"/>; never removed or reordered. The
    /// golden-replay-corpus spec's <c>key_event_hashes</c> field hashes the
    /// canonical encoding of this list per replay seed for Tier-A
    /// verification.
    /// </summary>
    public List<KeyEvent> KeyEvents { get; }

    /// <summary>
    /// Parallel append-only stream of presentation-recipe metadata for
    /// signature-execution <see cref="KeyEvent"/>s. Read by
    /// <c>Viewer.EventBridge</c> (Phase-3 next semantic-slice item) which
    /// translates each entry to a <c>ViewerEvent</c> per ADR-0008.
    ///
    /// <para>
    /// <strong>Not canonical state.</strong> Excluded from
    /// <c>MatchCanonicalState.Write</c>. Coupling presentation
    /// metadata to the canonical hash would mean any overlay-text or
    /// shot-recipe change invalidates the corpus fixture — wrong axis
    /// (canonical = gameplay outcomes; recipes = derived display data).
    /// Each entry's <c>KeyEventIndex</c> points back into
    /// <see cref="KeyEvents"/> so the bridge can correlate.
    /// </para>
    /// </summary>
    public List<SignatureExecution> SignatureRecipes { get; }

    public MatchSimulationState(
        Tick currentTick,
        BallState ball,
        PlayerState[] homeTeam,
        PlayerState[] awayTeam)
    {
        HomeTeam = CopyAndValidateTeam(homeTeam, nameof(homeTeam));
        AwayTeam = CopyAndValidateTeam(awayTeam, nameof(awayTeam));
        CurrentTick = currentTick;
        Ball = ball;
        HomeScore = 0;
        AwayScore = 0;
        OutOfPlay = OutOfPlay.InPlay;
        KeyEvents = new List<KeyEvent>();
        SignatureRecipes = new List<SignatureExecution>();
    }

    /// <summary>
    /// Build the canonical Month-3 smoke fixture initial state: 22 players at
    /// archetype formation positions, ball supplied by caller, tick supplied
    /// by caller.
    /// </summary>
    public static MatchSimulationState FromArchetypeFormations(
        Tick currentTick,
        BallState ball,
        BehaviorTreeArchetype homeArchetype,
        BehaviorTreeArchetype awayArchetype)
    {
        if (homeArchetype is null)
        {
            throw new ArgumentNullException(nameof(homeArchetype));
        }
        if (awayArchetype is null)
        {
            throw new ArgumentNullException(nameof(awayArchetype));
        }

        PlayerState[] homeTeam = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        PlayerState[] awayTeam = new PlayerState[MatchCanonicalState.PlayersPerTeam];

        foreach (FormationSlot homeSlot in homeArchetype.Formation)
        {
            int rosterIndex = homeSlot.RosterSlot - 1;
            homeTeam[rosterIndex] = new PlayerState(
                position: homeSlot.HomeBasePosition,
                velocity: Vector3Fixed.Zero,
                jerseyNumber: homeSlot.RosterSlot,
                side: TeamSide.Home);
        }

        foreach (FormationSlot awaySlot in awayArchetype.Formation)
        {
            int rosterIndex = awaySlot.RosterSlot - 1;
            awayTeam[rosterIndex] = new PlayerState(
                position: awaySlot.AwayBasePosition(),
                velocity: Vector3Fixed.Zero,
                jerseyNumber: awaySlot.RosterSlot,
                side: TeamSide.Away);
        }

        return new MatchSimulationState(currentTick, ball, homeTeam, awayTeam);
    }

    private static PlayerState[] CopyAndValidateTeam(PlayerState[] team, string paramName)
    {
        if (team is null)
        {
            throw new ArgumentNullException(paramName);
        }
        if (team.Length != MatchCanonicalState.PlayersPerTeam)
        {
            throw new ArgumentException(
                $"{paramName} must contain exactly {MatchCanonicalState.PlayersPerTeam} players; got {team.Length}.",
                paramName);
        }

        PlayerState[] copy = new PlayerState[MatchCanonicalState.PlayersPerTeam];
        Array.Copy(team, copy, MatchCanonicalState.PlayersPerTeam);
        return copy;
    }
}

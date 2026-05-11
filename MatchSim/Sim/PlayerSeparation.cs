using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Phase-3 polish-pass Option 1 (2026-05-11): inter-player soft collision.
/// Runs as a positional-correction pass between
/// <see cref="PlayerActuator.Step"/> and <see cref="MatchSimulationRunner"/>'s
/// kick-apply step. For each pair of players within
/// <c>2 × Kinematics.Radius</c> of each other, pushes each player half the
/// overlap apart along the inter-centre direction. Single iteration per
/// tick — no convergence loop, no recursive resolution; the BT runner's
/// next tick redistributes naturally.
///
/// <para>
/// <strong>Why position-correction, not repulsion-force-in-actuator</strong>:
/// keeps <see cref="PlayerActuator.Step"/>'s single-player-in / single-
/// player-out signature clean. Separation is a global constraint that
/// needs all 22 positions visible; the actuator's contract is per-player.
/// </para>
///
/// <para>
/// <strong>Determinism</strong>: pair iteration in stable order (home i &lt; j,
/// then home-vs-away, then away i &lt; j). Q32.32 throughout. Ties on inter-
/// centre direction (zero-magnitude — players at identical positions, rare
/// but possible at smoke fixture tick 0) push along +X by convention.
/// </para>
///
/// <para>
/// <strong>Velocity untouched</strong>. The actuator already wrote new
/// velocities this tick; the separation pass corrects positions only.
/// Slight inconsistency between (position, velocity) is acceptable for
/// one tick — the next BT.Tick reads positions and re-issues steering
/// commands.
/// </para>
/// </summary>
public static class PlayerSeparation
{
    /// <summary>
    /// Apply soft separation across all 22 players. Mutates
    /// <paramref name="state"/>.<see cref="MatchSimulationState.HomeTeam"/>
    /// and <see cref="MatchSimulationState.AwayTeam"/> positions in place.
    /// </summary>
    public static void Step(MatchSimulationState state, PlayerKinematics kinematics)
    {
        if (state is null) throw new ArgumentNullException(nameof(state));

        // Minimum centre-to-centre separation = 2 × Radius. Compute squared
        // form for sqrt-free distance checks.
        Fixed minSep = kinematics.Radius + kinematics.Radius;
        Fixed minSepSq = minSep * minSep;

        // Home-vs-home pairs.
        ResolveTeamSelfPairs(state.HomeTeam, minSep, minSepSq);
        // Away-vs-away pairs.
        ResolveTeamSelfPairs(state.AwayTeam, minSep, minSepSq);
        // Home-vs-away pairs.
        ResolveTeamCrossPairs(state.HomeTeam, state.AwayTeam, minSep, minSepSq);
    }

    private static void ResolveTeamSelfPairs(
        PlayerState[] team, Fixed minSep, Fixed minSepSq)
    {
        int n = team.Length;
        for (int i = 0; i < n; i++)
        {
            for (int j = i + 1; j < n; j++)
            {
                team[i] = TryPush(team[i], team[j], minSep, minSepSq, out PlayerState pushedJ);
                team[j] = pushedJ;
            }
        }
    }

    private static void ResolveTeamCrossPairs(
        PlayerState[] home, PlayerState[] away, Fixed minSep, Fixed minSepSq)
    {
        for (int i = 0; i < home.Length; i++)
        {
            for (int j = 0; j < away.Length; j++)
            {
                home[i] = TryPush(home[i], away[j], minSep, minSepSq, out PlayerState pushedAway);
                away[j] = pushedAway;
            }
        }
    }

    /// <summary>
    /// If players <paramref name="a"/> and <paramref name="b"/> are within
    /// <paramref name="minSep"/> of each other, push each half the overlap
    /// apart along the (b - a) direction. Returns the updated pair; if no
    /// overlap, returns the inputs unchanged.
    /// </summary>
    private static PlayerState TryPush(
        PlayerState a, PlayerState b, Fixed minSep, Fixed minSepSq, out PlayerState bOut)
    {
        Vector3Fixed delta = b.Position - a.Position;
        Fixed distSq = delta.LengthSquared();
        if (distSq >= minSepSq)
        {
            bOut = b;
            return a;
        }

        // Compute overlap. distSq < minSepSq means dist < minSep.
        // Direction: prefer (b - a) normalized. Zero-magnitude fallback:
        // push along +X by convention (deterministic + non-crashing).
        Vector3Fixed direction;
        if (distSq == Fixed.Zero)
        {
            direction = new Vector3Fixed(Fixed.One, Fixed.Zero, Fixed.Zero);
        }
        else
        {
            direction = delta.Normalize();
        }

        // Compute actual distance via sqrt — needed for half-overlap.
        Fixed dist = Fixed.Sqrt(distSq);
        Fixed overlap = minSep - dist;
        Fixed halfOverlap = overlap / Fixed.FromInt(2);
        Vector3Fixed correction = direction * halfOverlap;

        bOut = new PlayerState(
            position: ProjectToPitchPlane(b.Position + correction),
            velocity: b.Velocity,
            jerseyNumber: b.JerseyNumber,
            side: b.Side);
        return new PlayerState(
            position: ProjectToPitchPlane(a.Position - correction),
            velocity: a.Velocity,
            jerseyNumber: a.JerseyNumber,
            side: a.Side);
    }

    private static Vector3Fixed ProjectToPitchPlane(Vector3Fixed position)
        => new(position.X, Fixed.Zero, position.Z);
}

using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Phase-3 minimum match-rules orchestrator per SPEC 2026-04-28 PitchRules
/// decisions-log entry. Runs after <see cref="BallPhysics.Step"/> each tick;
/// detects out-of-play crossings, mutates <see cref="MatchSimulationState"/>
/// (score, OutOfPlay flag, KeyEvent stream), respawns the ball at the
/// appropriate restart spot.
///
/// <para>
/// <strong>Pure-deterministic Q32.32 fixed-point math.</strong> Same input
/// state ⇒ same output state across all platforms; the Tier-A determinism
/// matrix in fast-pr-ci.yml verifies this. No floats, no Unity APIs, no
/// platform RNG — same architectural posture as the rest of MatchSim per
/// .claude/rules/Scripts/MatchSim/RULES.md.
/// </para>
///
/// <para>
/// <strong>Phase-3 simplifications</strong> per the 2026-04-28 decisions-log
/// entry's "out of scope for Phase 3" list — implemented here as documented
/// constants:
/// </para>
/// <list type="bullet">
///   <item><description><strong>No last-touched-by tracking.</strong>
///       GoalKick / CornerKick disambiguation requires knowing which side
///       last touched the ball; Phase 3 omits possession tracking, so
///       all non-goal goal-line crossings classify as <see cref="OutOfPlay.GoalKick"/>
///       with <see cref="KeyEventKind.GoalKickRestart"/>. Phase 4+ activates
///       the distinction.</description></item>
///   <item><description><strong>No restart taker behavior.</strong> Ball
///       respawns at the canonical restart spot with zero velocity; players
///       continue their normal BTs. The "restart-taker walks to the spot"
///       choreography lands Phase 4+.</description></item>
///   <item><description><strong>OutOfPlay is a per-tick flag.</strong> Set
///       on the tick the event fires; reset to <see cref="OutOfPlay.InPlay"/>
///       at the start of every <see cref="Step"/>. Canonically a transient
///       marker — the persistent record of "what happened" lives in the
///       <see cref="MatchSimulationState.KeyEvents"/> stream.</description></item>
///   <item><description><strong>No scorer / last-toucher in KeyEvents.</strong>
///       <see cref="KeyEvent.JerseyNumber"/> is 0 for all Phase-3 emissions.
///       Phase 4+ adds proper scorer attribution once possession lands.</description></item>
/// </list>
/// </summary>
public static class MatchRules
{
    /// <summary>
    /// Half the pitch length (m). Goal lines at <c>X = ±GoalLineX</c>.
    /// Pitch is 105×68m per design/match-engine.md §pitch dimensions.
    /// </summary>
    public static readonly Fixed GoalLineX = Fixed.FromInt(105) / Fixed.FromInt(2);

    /// <summary>
    /// Half the pitch width (m). Touchlines at <c>Z = ±TouchlineZ</c>.
    /// </summary>
    public static readonly Fixed TouchlineZ = Fixed.FromInt(68) / Fixed.FromInt(2);

    /// <summary>
    /// Goal-mouth crossbar height (m). Standard FIFA: 2.44m. A ball crossing
    /// the goal line above this height is NOT a goal even within the post-
    /// half-width.
    /// </summary>
    public static readonly Fixed CrossbarHeight = Fixed.FromInt(244) / Fixed.FromInt(100);

    /// <summary>
    /// Half the goal-mouth width (m). Standard FIFA: 7.32m total width →
    /// half = 3.66m. A ball crossing the goal line with <c>|Z| &gt;
    /// PostHalfWidthZ</c> is NOT a goal even below the crossbar.
    /// </summary>
    public static readonly Fixed PostHalfWidthZ = Fixed.FromInt(366) / Fixed.FromInt(100);

    /// <summary>
    /// Step the rules forward one canonical tick. Runs after
    /// <see cref="BallPhysics.Step"/>; detects out-of-play crossings between
    /// <paramref name="preStepBall"/> and <c>state.Ball</c>; mutates
    /// <paramref name="state"/> (score, OutOfPlay, KeyEvents, ball position
    /// for restart spawns).
    /// </summary>
    /// <param name="state">Current sim state (mutated in-place).</param>
    /// <param name="preStepBall">
    /// Ball state from BEFORE the tick's <see cref="BallPhysics.Step"/>.
    /// Required for crossing detection — the runner caches it.
    /// </param>
    public static void Step(MatchSimulationState state, BallState preStepBall)
    {
        if (state is null)
        {
            throw new ArgumentNullException(nameof(state));
        }

        // OutOfPlay is a per-tick transient flag. Reset to InPlay; the
        // crossing checks below may set it to a restart kind if an event
        // fires THIS tick.
        state.OutOfPlay = OutOfPlay.InPlay;

        Vector3Fixed pre = preStepBall.Position;
        Vector3Fixed post = state.Ball.Position;

        bool preInField = IsInField(pre);
        bool postInField = IsInField(post);

        // Only detect on in→out transitions. Out→in (ball returning from a
        // restart spawn) doesn't fire an event — the spawn itself is the
        // event, recorded the tick it happened.
        if (!preInField || postInField)
        {
            return;
        }

        // A crossing happened this tick. There may be MULTIPLE candidate
        // crossings (goal-line + touchline) for fast diagonal trajectories
        // — a ball that exits near a corner can be beyond both planes at
        // post-step. Football-rule correctness: the boundary the ball
        // crossed FIRST in time-order determines the restart. Compute the
        // parametric t for each candidate plane crossing in pre→post; the
        // smallest in-range t wins.
        //
        // (Codex audit 2026-04-30 caught the prior implementation, which
        // checked goal-line vs touchline by post-position only and always
        // preferred goal-line — incorrectly classifying touchline-first-
        // then-also-past-goal-line trajectories as GoalKickRestart.)
        Fixed? tGoal = ComputeGoalLineCrossingT(pre, post);
        Fixed? tTouch = ComputeTouchlineCrossingT(pre, post);

        if (!tGoal.HasValue && !tTouch.HasValue)
        {
            // Pre is in-field + post is out-of-field, yet neither plane
            // crossing resolved a t in [0,1]. Geometrically unreachable —
            // any in→out transition MUST cross at least one of the two
            // planes between pre and post — but per the silent-failure-
            // hunter audit (2026-04-30), if a future refactor of
            // IsInField / ComputeXxxCrossingT changes the invariant, a
            // silent return would corrupt canonical state without trace.
            // Throw loudly so the determinism harness fails the run
            // rather than continuing with "ball is out, no event recorded."
            throw new InvalidOperationException(
                $"MatchRules.Step: ball transitioned in→out but no plane " +
                $"crossing resolved. pre={pre} post={post}. Likely a " +
                $"refactor that desynced IsInField from the helper guards.");
        }

        // Earliest crossing wins. Tie-break: prefer goal-line.
        // Rationale for the tie-break: when the ball exits exactly through
        // a corner (tGoal == tTouch geometrically), it's a corner-kick
        // restart in real football. Phase-3 omits CornerKick activation
        // (no last-touched tracking yet) and emits GoalKickRestart for
        // all non-goal goal-line crossings, so preferring goal-line on
        // tie matches Phase-3 simplification policy. Phase 4+ revisits
        // when last-touched tracking lands.
        bool goalLineFirst = tGoal.HasValue
            && (!tTouch.HasValue || tGoal.Value.CompareTo(tTouch.Value) <= 0);

        if (goalLineFirst)
        {
            HandleGoalLineCrossing(state, pre, post);
        }
        else
        {
            HandleTouchlineCrossing(state, pre, post);
        }
    }

    /// <summary>
    /// Compute the parametric crossing point on the goal-line plane (X = ±GoalLineX)
    /// between pre→post. Returns null if no goal-line crossing happened in [0,1]:
    /// post is still in-field on X, or pre and post are on the same side
    /// of the plane, or the ball didn't move on X.
    /// </summary>
    private static Fixed? ComputeGoalLineCrossingT(Vector3Fixed pre, Vector3Fixed post)
    {
        if (AbsFixed(post.X) < GoalLineX)
        {
            return null;
        }

        Fixed crossingX = post.X > Fixed.Zero ? GoalLineX : -GoalLineX;
        Fixed deltaX = post.X - pre.X;
        if (deltaX == Fixed.Zero)
        {
            // Caller invariant per Step's pre/post field check: if pre is
            // in-field on X (|pre.X| < GoalLineX) and post is past the
            // goal line (|post.X| >= GoalLineX), deltaX cannot be zero.
            // Per the silent-failure-hunter audit (2026-04-30), this is a
            // hidden invariant violation rather than a recoverable
            // condition — throw loudly so a future caller-refactor that
            // breaks the invariant fails the determinism harness instead
            // of silently dropping the crossing.
            throw new InvalidOperationException(
                $"ComputeGoalLineCrossingT: post is past the goal line on X " +
                $"({post.X}) but deltaX is zero. pre={pre} post={post}. " +
                $"This is a caller-invariant violation — Step should have " +
                $"rejected this case earlier.");
        }

        Fixed t = (crossingX - pre.X) / deltaX;
        if (t < Fixed.Zero || t > Fixed.One)
        {
            return null;
        }
        return t;
    }

    /// <summary>
    /// Compute the parametric crossing point on the touchline plane (Z = ±TouchlineZ)
    /// between pre→post. Returns null if no touchline crossing happened in [0,1].
    /// </summary>
    private static Fixed? ComputeTouchlineCrossingT(Vector3Fixed pre, Vector3Fixed post)
    {
        if (AbsFixed(post.Z) < TouchlineZ)
        {
            return null;
        }

        Fixed crossingZ = post.Z > Fixed.Zero ? TouchlineZ : -TouchlineZ;
        Fixed deltaZ = post.Z - pre.Z;
        if (deltaZ == Fixed.Zero)
        {
            // Same caller-invariant rationale as ComputeGoalLineCrossingT:
            // pre in-field on Z + post past touchline implies deltaZ != 0.
            // Throw on violation rather than silently drop the crossing
            // (silent-failure-hunter audit 2026-04-30).
            throw new InvalidOperationException(
                $"ComputeTouchlineCrossingT: post is past the touchline on Z " +
                $"({post.Z}) but deltaZ is zero. pre={pre} post={post}. " +
                $"Caller-invariant violation.");
        }

        Fixed t = (crossingZ - pre.Z) / deltaZ;
        if (t < Fixed.Zero || t > Fixed.One)
        {
            return null;
        }
        return t;
    }

    private static bool IsInField(Vector3Fixed position)
        => AbsFixed(position.X) < GoalLineX
            && AbsFixed(position.Z) < TouchlineZ;

    private static Fixed AbsFixed(Fixed value)
        => value < Fixed.Zero ? -value : value;

    /// <summary>
    /// Linearly interpolate a Fixed-point value at parameter <paramref name="t"/>
    /// between <paramref name="from"/> and <paramref name="to"/>. Q32.32
    /// arithmetic; no float intermediates.
    /// </summary>
    private static Fixed Lerp(Fixed from, Fixed to, Fixed t)
        => from + (to - from) * t;

    private static void HandleGoalLineCrossing(
        MatchSimulationState state,
        Vector3Fixed pre,
        Vector3Fixed post)
    {
        // Find Y, Z at the goal-line plane via linear interpolation between
        // pre-step and post-step ball positions. Solve for t where
        // pre.X + t*(post.X - pre.X) = ±GoalLineX.
        Fixed crossingX = post.X > Fixed.Zero ? GoalLineX : -GoalLineX;
        Fixed deltaX = post.X - pre.X;

        // Defensive: if deltaX is zero, ball didn't actually move on X — a
        // pre-step in-field + post-step out-of-field with deltaX=0 is
        // impossible by definition, but guard anyway. Skip the event.
        if (deltaX == Fixed.Zero)
        {
            return;
        }

        Fixed t = (crossingX - pre.X) / deltaX;
        Fixed yAtCrossing = Lerp(pre.Y, post.Y, t);
        Fixed zAtCrossing = Lerp(pre.Z, post.Z, t);
        Vector3Fixed crossingPosition = new(crossingX, yAtCrossing, zAtCrossing);

        // Goal-mouth check: under crossbar AND between posts AND not below
        // pitch (Y >= 0). Y < 0 is unphysical given BallPhysics ground
        // clamping; the >= 0 check is belt-and-suspenders.
        bool inGoalMouth = yAtCrossing >= Fixed.Zero
            && yAtCrossing < CrossbarHeight
            && AbsFixed(zAtCrossing) < PostHalfWidthZ;

        if (inGoalMouth)
        {
            // GOAL. Home attacks +X, Away attacks -X (per BehaviorTreeRunner
            // §HOME orientation comment). A goal at +X end means HOME scored;
            // at -X end means AWAY scored.
            TeamSide scoringSide = post.X > Fixed.Zero ? TeamSide.Home : TeamSide.Away;
            IncrementScore(state, scoringSide);
            state.KeyEvents.Add(new KeyEvent(
                tick: state.CurrentTick,
                kind: KeyEventKind.Goal,
                side: scoringSide,
                jerseyNumber: KeyEvent.JerseyUnspecified,
                position: crossingPosition));

            // Immediate respawn at center per the 2026-04-28 decisions-log
            // entry's "no KickOff state — goal restarts are immediate" rule.
            state.Ball = new BallState(Vector3Fixed.Zero, Vector3Fixed.Zero, Vector3Fixed.Zero);
            // OutOfPlay stays InPlay (already set at the top of Step).
        }
        else
        {
            // Goal-line crossing outside the goal mouth. Phase-3 simplification
            // per the decisions-log entry: classify as GoalKick (defending
            // team restarts). Phase 4+ adds last-touched tracking which
            // distinguishes CornerKick when the attacking side put it out.
            TeamSide restartSide = post.X > Fixed.Zero ? TeamSide.Away : TeamSide.Home;
            // Restart spot: goal-line center on the conceding side. Phase-4+
            // refines to the proper goal-area corner depending on context.
            Vector3Fixed restartPosition = new(crossingX, Fixed.Zero, Fixed.Zero);

            state.OutOfPlay = OutOfPlay.GoalKick;
            state.KeyEvents.Add(new KeyEvent(
                tick: state.CurrentTick,
                kind: KeyEventKind.GoalKickRestart,
                side: restartSide,
                jerseyNumber: KeyEvent.JerseyUnspecified,
                position: crossingPosition));
            state.Ball = new BallState(restartPosition, Vector3Fixed.Zero, Vector3Fixed.Zero);
        }
    }

    private static void HandleTouchlineCrossing(
        MatchSimulationState state,
        Vector3Fixed pre,
        Vector3Fixed post)
    {
        Fixed crossingZ = post.Z > Fixed.Zero ? TouchlineZ : -TouchlineZ;
        Fixed deltaZ = post.Z - pre.Z;

        if (deltaZ == Fixed.Zero)
        {
            return;
        }

        Fixed t = (crossingZ - pre.Z) / deltaZ;
        Fixed xAtCrossing = Lerp(pre.X, post.X, t);
        Fixed yAtCrossing = Lerp(pre.Y, post.Y, t);
        Vector3Fixed crossingPosition = new(xAtCrossing, yAtCrossing, crossingZ);

        // Throw-in restart at the crossing point on the touchline. Phase-3
        // simplification: side defaults to Home because last-touched tracking
        // is Phase 4+ scope. Phase 4+ flips this to "opposite of last-toucher".
        TeamSide restartSide = TeamSide.Home;
        Vector3Fixed restartPosition = new(xAtCrossing, Fixed.Zero, crossingZ);

        state.OutOfPlay = OutOfPlay.ThrowIn;
        state.KeyEvents.Add(new KeyEvent(
            tick: state.CurrentTick,
            kind: KeyEventKind.ThrowInRestart,
            side: restartSide,
            jerseyNumber: 0,
            position: crossingPosition));
        state.Ball = new BallState(restartPosition, Vector3Fixed.Zero, Vector3Fixed.Zero);
    }

    private static void IncrementScore(MatchSimulationState state, TeamSide scoringSide)
    {
        // byte score capped at 255 — overkill for any realistic match
        // (~10-15 max), but checked anyway so a runaway bug doesn't silently
        // wrap.
        if (scoringSide == TeamSide.Home)
        {
            if (state.HomeScore == byte.MaxValue)
            {
                throw new InvalidOperationException(
                    "HomeScore would overflow byte (255 already). Bug — investigate.");
            }
            state.HomeScore = (byte)(state.HomeScore + 1);
        }
        else
        {
            if (state.AwayScore == byte.MaxValue)
            {
                throw new InvalidOperationException(
                    "AwayScore would overflow byte (255 already). Bug — investigate.");
            }
            state.AwayScore = (byte)(state.AwayScore + 1);
        }
    }
}

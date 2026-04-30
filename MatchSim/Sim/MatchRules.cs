using System;
using System.Numerics;

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
///   <item><description><strong>Restarts are event-only placeholders.</strong>
///       Per the 2026-04-30 decisions-log entry (Codex P2 round 4): the
///       <see cref="KeyEvent.Side"/> on a restart event is informational
///       documentation, NOT a possession lock. <see cref="MatchSimulationRunner"/>
///       does not read it on the next tick — both BTs run as normal and
///       nearest-player heuristics decide who picks up the ball. Phase 4
///       introduces possession-lock + taker behavior so the recorded side
///       becomes authoritative. Until then, "Home throws in" is a label,
///       not a rule the sim enforces.</description></item>
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
        // parametric t for each candidate as an *exact* rational (raw-long
        // numerator + denominator); compare via cross-multiplied BigInteger
        // arithmetic so two near-simultaneous crossings classify correctly
        // even when their Q32.32-divided Fixed values would round to equal.
        //
        // History:
        //   2026-04-30 (commit 5ff42a3) — earliest-t selection introduced,
        //     using Q32.32 division then Fixed.CompareTo. Codex P1
        //     (round 4) caught: division rounds to a Fixed step, so two
        //     crossings within ~1 ULP can collapse to equal t and the
        //     tie-break (prefer goal-line) misclassifies a truly-touchline-
        //     first trajectory as GoalKickRestart.
        //   2026-04-30 (this commit) — switch to exact-rational compare.
        //     Helpers return (|num|, |den|) raw-long pairs; compare via
        //     |numG| * |denT|  vs  |numT| * |denG| in BigInteger space.
        //     Tie-break (prefer goal-line) only applies on TRUE equality.
        CrossingFraction? goalCrossing = ComputeGoalLineCrossingFraction(pre, post);
        CrossingFraction? touchCrossing = ComputeTouchlineCrossingFraction(pre, post);

        if (goalCrossing is null && touchCrossing is null)
        {
            // Pre is in-field + post is out-of-field, yet neither plane
            // crossing resolved a t in [0,1]. Geometrically unreachable —
            // any in→out transition MUST cross at least one of the two
            // planes between pre and post — but per the silent-failure-
            // hunter audit (2026-04-30), if a future refactor of
            // IsInField / ComputeXxxCrossingFraction changes the invariant,
            // a silent return would corrupt canonical state without trace.
            // Throw loudly so the determinism harness fails the run
            // rather than continuing with "ball is out, no event recorded."
            throw new InvalidOperationException(
                $"MatchRules.Step: ball transitioned in→out but no plane " +
                $"crossing resolved. pre={pre} post={post}. Likely a " +
                $"refactor that desynced IsInField from the helper guards.");
        }

        // Earliest crossing wins. Tie-break: prefer goal-line.
        // Rationale for the tie-break: when the ball exits exactly through
        // a corner (numG·denT == numT·denG geometrically), it's a corner-
        // kick restart in real football. Phase-3 omits CornerKick activation
        // (no last-touched tracking yet) and emits GoalKickRestart for
        // all non-goal goal-line crossings, so preferring goal-line on
        // true tie matches Phase-3 simplification policy. Phase 4+ revisits
        // when last-touched tracking lands.
        bool goalLineFirst;
        if (touchCrossing is null)
        {
            goalLineFirst = true;
        }
        else if (goalCrossing is null)
        {
            goalLineFirst = false;
        }
        else
        {
            int cmp = CompareCrossings(goalCrossing.Value, touchCrossing.Value);
            goalLineFirst = cmp <= 0;  // tie → goal-line
        }

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
    /// Exact rational form of a parametric plane-crossing point on
    /// pre→post. Both raw long values are non-negative with
    /// <c>AbsNumeratorRaw &lt;= AbsDenominatorRaw</c> and
    /// <c>AbsDenominatorRaw &gt; 0</c> — i.e., t = num/den is a rational
    /// in [0,1]. The 2^32 scaling factor cancels because both come from
    /// <see cref="Fixed.RawValue"/> of values produced by the same Q32.32
    /// subtraction (no division). Cross-multiplication via
    /// <see cref="BigInteger"/> compares two such fractions exactly,
    /// without Q32.32 rounding.
    ///
    /// <para>
    /// <strong>Construction is constructor-validated.</strong> The only
    /// way to build a <see cref="CrossingFraction"/> is via
    /// <see cref="BuildCrossingFraction"/> (which acceptance-checks the
    /// (numerator, denominator) signed pair); the private constructor
    /// then re-asserts the invariant so any future caller cannot bypass
    /// validation. Per pr-review-toolkit:type-design-analyzer 2026-04-30:
    /// the type's reason-for-existing is "non-negative rational ≤ 1";
    /// keeping the public-on-the-struct ctor unguarded was a
    /// foot-gun for Phase-4+ callers.
    /// </para>
    /// </summary>
    private readonly struct CrossingFraction
    {
        private CrossingFraction(long absNumeratorRaw, long absDenominatorRaw)
        {
            // Re-assert the BuildCrossingFraction acceptance invariant. A
            // future caller that bypasses Build (e.g., a Phase-4 helper for
            // CornerKick disambiguation) cannot smuggle a malformed pair
            // past this gate.
            if (absDenominatorRaw <= 0L)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(absDenominatorRaw), absDenominatorRaw,
                    "CrossingFraction denominator must be strictly positive.");
            }
            if (absNumeratorRaw < 0L)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(absNumeratorRaw), absNumeratorRaw,
                    "CrossingFraction numerator must be non-negative.");
            }
            if (absNumeratorRaw > absDenominatorRaw)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(absNumeratorRaw), absNumeratorRaw,
                    $"CrossingFraction numerator ({absNumeratorRaw}) must be ≤ " +
                    $"denominator ({absDenominatorRaw}); fraction must lie in [0,1].");
            }

            AbsNumeratorRaw = absNumeratorRaw;
            AbsDenominatorRaw = absDenominatorRaw;
        }

        /// <summary>Factory used internally; see <see cref="BuildCrossingFraction"/>.</summary>
        internal static CrossingFraction CreateValidated(long absNumeratorRaw, long absDenominatorRaw)
            => new(absNumeratorRaw, absDenominatorRaw);

        public long AbsNumeratorRaw { get; }
        public long AbsDenominatorRaw { get; }
    }

    /// <summary>
    /// Compute the exact-rational crossing fraction on the goal-line plane
    /// (X = ±GoalLineX) between pre→post. Returns null if no goal-line
    /// crossing happened in [0,1]: post is still in-field on X, the deltas
    /// disagree on sign (ball moves the wrong way to actually cross), or
    /// the absolute numerator exceeds the absolute denominator (t &gt; 1).
    /// </summary>
    private static CrossingFraction? ComputeGoalLineCrossingFraction(Vector3Fixed pre, Vector3Fixed post)
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
                $"ComputeGoalLineCrossingFraction: post is past the goal line " +
                $"on X ({post.X}) but deltaX is zero. pre={pre} post={post}. " +
                $"This is a caller-invariant violation — Step should have " +
                $"rejected this case earlier.");
        }

        return BuildCrossingFraction(crossingX - pre.X, deltaX);
    }

    /// <summary>
    /// Compute the exact-rational crossing fraction on the touchline plane
    /// (Z = ±TouchlineZ) between pre→post. Returns null if no touchline
    /// crossing happened in [0,1].
    /// </summary>
    private static CrossingFraction? ComputeTouchlineCrossingFraction(Vector3Fixed pre, Vector3Fixed post)
    {
        if (AbsFixed(post.Z) < TouchlineZ)
        {
            return null;
        }

        Fixed crossingZ = post.Z > Fixed.Zero ? TouchlineZ : -TouchlineZ;
        Fixed deltaZ = post.Z - pre.Z;
        if (deltaZ == Fixed.Zero)
        {
            // Same caller-invariant rationale as ComputeGoalLineCrossingFraction:
            // pre in-field on Z + post past touchline implies deltaZ != 0.
            // Throw on violation rather than silently drop the crossing
            // (silent-failure-hunter audit 2026-04-30).
            throw new InvalidOperationException(
                $"ComputeTouchlineCrossingFraction: post is past the touchline " +
                $"on Z ({post.Z}) but deltaZ is zero. pre={pre} post={post}. " +
                $"Caller-invariant violation.");
        }

        return BuildCrossingFraction(crossingZ - pre.Z, deltaZ);
    }

    /// <summary>
    /// Build a non-negative-rational crossing fraction from a signed
    /// numerator and signed denominator (both Fixed). The caller invariant
    /// — pre is in-field on the relevant axis, post is past the boundary,
    /// delta is non-zero — guarantees that the resulting (numerator,
    /// denominator) pair shares sign and that |num| &lt;= |den|. Either
    /// guarantee failing is a hidden caller-invariant violation rather
    /// than a recoverable condition; throw with diagnostics so a future
    /// refactor that breaks the invariant fails loudly instead of silently
    /// dropping a crossing.
    /// </summary>
    /// <remarks>
    /// Per pr-review-toolkit:silent-failure-hunter 2026-04-30 (P1 #1):
    /// the prior implementation returned <c>null</c> on either invariant
    /// failure. Combined with the upstream "if either compute returns
    /// null, fall back to the other" branch in <see cref="Step"/>, that
    /// silently classified geometrically-corrupt scenarios as clean
    /// single-axis crossings. Loud throw matches the
    /// <c>delta == Fixed.Zero</c> precedent already established here +
    /// in <see cref="ComputeGoalLineCrossingFraction"/> /
    /// <see cref="ComputeTouchlineCrossingFraction"/>.
    /// </remarks>
    private static CrossingFraction BuildCrossingFraction(Fixed signedNumerator, Fixed signedDenominator)
    {
        long numRaw = signedNumerator.RawValue;
        long denRaw = signedDenominator.RawValue;

        // Same-sign requirement: t in [0,1] forces num and den to share sign
        // (or num == 0). Disagreement → caller-invariant violation; under
        // the documented preconditions in ComputeGoalLineCrossingFraction /
        // ComputeTouchlineCrossingFraction this is unreachable — pre is
        // in-field, post is past the boundary, so (crossing - pre) and
        // (post - pre) must point the same way.
        bool sameSign = (numRaw >= 0 && denRaw > 0) || (numRaw <= 0 && denRaw < 0);
        if (!sameSign)
        {
            throw new InvalidOperationException(
                $"BuildCrossingFraction: signed numerator ({numRaw}) and " +
                $"denominator ({denRaw}) disagree on sign. Caller-invariant " +
                $"violation — pre must be in-field on the relevant axis and " +
                $"post must be past the boundary, which forces same-sign.");
        }

        long absNum = numRaw >= 0 ? numRaw : -numRaw;
        long absDen = denRaw >= 0 ? denRaw : -denRaw;

        // |num| > |den| means t > 1: pre is on the OUT side of the plane,
        // so the crossing happened before this tick. Same caller-invariant
        // class — Step's pre/post field check rules this out before reaching
        // here. Throw rather than silently drop.
        if (absNum > absDen)
        {
            throw new InvalidOperationException(
                $"BuildCrossingFraction: |numerator| ({absNum}) exceeds " +
                $"|denominator| ({absDen}). t > 1 implies pre was already " +
                $"out-of-field on the relevant axis — caller-invariant " +
                $"violation; Step's pre-field guard should have rejected this.");
        }

        return CrossingFraction.CreateValidated(absNum, absDen);
    }

    /// <summary>
    /// Compare two crossing fractions exactly via cross-multiplication.
    /// Returns -1 if <paramref name="goal"/> &lt; <paramref name="touch"/>,
    /// 0 if equal (true tie — caller applies the goal-line preference),
    /// +1 if <paramref name="goal"/> &gt; <paramref name="touch"/>.
    /// Uses <see cref="BigInteger"/> because the raw-long product can
    /// reach ~2^126 (well beyond <c>long</c>'s 2^63 range).
    /// </summary>
    private static int CompareCrossings(CrossingFraction goal, CrossingFraction touch)
    {
        // goal.num/goal.den vs touch.num/touch.den
        //   ⇔ goal.num * touch.den  vs  touch.num * goal.den
        // (both denominators positive ⇒ inequality direction preserved).
        BigInteger lhs = (BigInteger)goal.AbsNumeratorRaw * touch.AbsDenominatorRaw;
        BigInteger rhs = (BigInteger)touch.AbsNumeratorRaw * goal.AbsDenominatorRaw;
        return lhs.CompareTo(rhs);
    }

    /// <summary>
    /// True iff the ball center is strictly inside the pitch boundary on
    /// both X and Z. Boundary equality (|X| == GoalLineX or |Z| == TouchlineZ)
    /// is treated as OUT.
    ///
    /// <para>
    /// <strong>Boundary-line convention is a Phase-3 simplification.</strong>
    /// Real football: the line itself is IN — the ball must wholly cross
    /// the line to be out. We approximate with center-point + strict
    /// inequality for two reasons: (1) Phase 3 has no ball-radius geometry
    /// (the ball is treated as a point in <see cref="BallPhysics"/>), so
    /// the "wholly crossed" test would need to be retrofitted alongside
    /// every other ball-vs-line interaction; (2) Phase-3 GoalKick-everything
    /// classification (no last-touched tracking → no CornerKick distinction)
    /// makes the ball-on-line edge case observationally invisible — the
    /// chosen boundary semantics still emit the same GoalKick event whether
    /// "exactly on the line" counts as IN or OUT. Phase 4+ revisits both:
    /// last-touched tracking activates CornerKick disambiguation, and the
    /// ball-radius geometry can land alongside.
    /// </para>
    /// </summary>
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

        // Caller-invariant: Step routes here only after
        // ComputeGoalLineCrossingFraction returned a valid fraction, which
        // means deltaX != Zero (Compute throws on the zero case). Throw on
        // the unreachable branch rather than silently skip; a refactor that
        // ever calls Handle* without Compute* validation must fail loudly
        // (per pr-review-toolkit:silent-failure-hunter 2026-04-30 P1 #2).
        if (deltaX == Fixed.Zero)
        {
            throw new InvalidOperationException(
                $"HandleGoalLineCrossing: deltaX is zero with pre={pre} " +
                $"post={post}. This is unreachable under the current Step " +
                $"orchestration; a future caller must run through " +
                $"ComputeGoalLineCrossingFraction first.");
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

        // Caller-invariant: Step routes here only after
        // ComputeTouchlineCrossingFraction returned a valid fraction, which
        // means deltaZ != Zero (Compute throws on the zero case). Throw on
        // the unreachable branch (per silent-failure-hunter 2026-04-30 P1 #2).
        if (deltaZ == Fixed.Zero)
        {
            throw new InvalidOperationException(
                $"HandleTouchlineCrossing: deltaZ is zero with pre={pre} " +
                $"post={post}. This is unreachable under the current Step " +
                $"orchestration; a future caller must run through " +
                $"ComputeTouchlineCrossingFraction first.");
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
            jerseyNumber: KeyEvent.JerseyUnspecified,
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

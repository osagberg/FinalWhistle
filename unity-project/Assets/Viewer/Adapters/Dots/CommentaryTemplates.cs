using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Contracts;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Phase-3 commentary + signature title-card text for the dots-adapter
    /// UI Toolkit overlay (Slice 6). All strings are pure-C# constants — no
    /// content-pack lookup at Phase 3; that lands at Phase 4+ when
    /// localisation infrastructure ships.
    ///
    /// <para>
    /// <strong>Slice-6 event-to-overlay matrix</strong> (per Codex
    /// pre-implementation review of 2b3460e..d58b6d8 + the dots-adapter
    /// blueprint §B Slice 6):
    /// </para>
    ///
    /// <list type="table">
    ///   <listheader>
    ///     <description>EventClass</description>
    ///     <description>ShotCategory</description>
    ///     <description>Bridge KeyEventKind</description>
    ///     <description>Commentary</description>
    ///     <description>Title-card</description>
    ///   </listheader>
    ///   <item>
    ///     <description><see cref="EventClass.GoalScored"/></description>
    ///     <description><see cref="ShotCategory.PassShotImpact"/></description>
    ///     <description><c>Goal</c></description>
    ///     <description>5 strings (<c>commentary[Goal_PassShotImpact]</c>)</description>
    ///     <description>NO — goals are loud enough on their own; signature title-card discipline reserves the surface for signature execution per design/anime-presentation-budget.md surface #3</description>
    ///   </item>
    ///   <item>
    ///     <description><see cref="EventClass.SignatureExecuted"/></description>
    ///     <description><see cref="ShotCategory.PlayerIsolation"/></description>
    ///     <description><c>SignatureExecuted_LowCutback</c></description>
    ///     <description>5 strings</description>
    ///     <description>YES — display name "Low cutback from the byline" (signature #20)</description>
    ///   </item>
    ///   <item>
    ///     <description><see cref="EventClass.SignatureExecuted"/></description>
    ///     <description><see cref="ShotCategory.PassShotImpact"/></description>
    ///     <description><c>SignatureExecuted_BlindSideNearPostRun</c></description>
    ///     <description>5 strings (DISTINCT from Goal pool — same ShotCategory but different event class so different commentary feel)</description>
    ///     <description>YES — display name "Blind-side near-post run" (signature #22)</description>
    ///   </item>
    ///   <item>
    ///     <description><see cref="EventClass.SignatureExecuted"/></description>
    ///     <description><see cref="ShotCategory.TacticalWide"/></description>
    ///     <description><c>SignatureExecuted_FirstTimeDiagonalSwitch</c></description>
    ///     <description>5 strings</description>
    ///     <description>YES — display name "First-time diagonal switch" (signature #13)</description>
    ///   </item>
    ///   <item>
    ///     <description><see cref="EventClass.SignatureBreakthrough"/></description>
    ///     <description><see cref="ShotCategory.AftermathFreeze"/></description>
    ///     <description><c>SignatureBreakthrough</c></description>
    ///     <description>5 strings</description>
    ///     <description>NO at Phase 3 — Phase-4 work per design/breakthrough-moments.md §MVP boundary; commentary fires for Slice 6 acceptance, title-card explicit-no</description>
    ///   </item>
    /// </list>
    ///
    /// <para>
    /// <strong>Phase-3 totals</strong>: 5 commentary keys × 5 strings each
    /// = 25 commentary templates + 3 signature display names. Every
    /// bridge-emittable <c>(EventClass, ShotCategory)</c> tuple has either
    /// a commentary path or an explicit documented no-op (per Codex
    /// P2-2 closure: no silent fall-through).
    /// </para>
    ///
    /// <para>
    /// <strong>Vocabulary contract</strong>: every string passes
    /// <c>design/ui-vocabulary.md</c> Category-A lint via the existing
    /// <c>scripts/fw banned-terms</c> umbrella. Football-native British
    /// register only — no capitalised state nouns. Examples per the
    /// blueprint: "He strikes!" / "Gets onto the end of it." / "It's in.
    /// The home side pull ahead." narrative-director sign-off is
    /// required pre-commit per the Slice-6 agent rotation.
    /// </para>
    ///
    /// <para>
    /// <strong>Determinism</strong>: <see cref="PickCommentary"/> selects
    /// from the matching pool via <c>ev.Seed.Value % pool.Count</c> — a
    /// stable per-event index that produces the same string on every
    /// replay of the same canonical state. The replay corpus's
    /// pass-activation trace can pin the rendered string at fixture
    /// authoring time without breaking on engine re-runs.
    /// </para>
    /// </summary>
    internal static class CommentaryTemplates
    {
        // -------------------------------------------------------------
        // Commentary pools — keyed by (EventClass, ShotCategory).
        // -------------------------------------------------------------

        private static readonly IReadOnlyList<string> GoalPool = new[]
        {
            // Side-neutral phrasing per Slice-6 round-1 P3 closure
            // (feature-dev:code-reviewer): GoalPool is consumed by both
            // home + away GoalScored events, so any side-specific copy
            // ("the home side pull ahead.") emits factually-wrong
            // commentary on away goals. Phase-4 adds split pools when
            // ViewerEvent carries an explicit scorer-side discriminator.
            "He strikes! It's in.",
            "Gets onto the end of it — finds the back of the net.",
            "And that's the opener.",
            "What a finish. The crowd's on its feet.",
            "Punished. The keeper had no chance.",
        };

        private static readonly IReadOnlyList<string> LowCutbackPool = new[]
        {
            "Drives outside, cuts it back low across the six-yard box.",
            "Nicks it past the full-back and pulls it back from the byline.",
            "Wide and deep — and now the cutback into traffic.",
            "Onto the line, lifts his head, and rolls it back.",
            "Beats his man, looks up, lays it back from the byline.",
        };

        private static readonly IReadOnlyList<string> BlindSideNearPostPool = new[]
        {
            "Run off the back of the centre-half — and he gets to the near post.",
            "Peels off the shoulder, attacks the near stick.",
            "Loses his marker, into the gap at the near post.",
            "Sneaks across the front man — meeting it at the near post.",
            "Off the back side and into the near post — clever movement.",
        };

        private static readonly IReadOnlyList<string> FirstTimeDiagonalSwitchPool = new[]
        {
            "First time! Switches the play across the field.",
            "Lays it across in one — the angle's opened up.",
            "Spotted the runner on the far side and clipped it first time.",
            "One touch, all the way to the opposite flank.",
            "Doesn't break stride — diagonal ball into space.",
        };

        private static readonly IReadOnlyList<string> SignatureBreakthroughPool = new[]
        {
            "Something's clicked for him this afternoon.",
            "Third time in the match — that's a player finding his level.",
            "Real moment for him, that. He'll remember this one.",
            "He's doing it on the day. The dressing room will have noticed.",
            "Performance has built into something. He's earned this.",
        };

        // -------------------------------------------------------------
        // Pool dispatch — (EventClass, ShotCategory) → pool reference.
        // -------------------------------------------------------------

        private static IReadOnlyList<string>? PoolFor(EventClass eventClass, ShotCategory category)
        {
            return (eventClass, category) switch
            {
                (EventClass.GoalScored, ShotCategory.PassShotImpact) => GoalPool,
                (EventClass.SignatureExecuted, ShotCategory.PlayerIsolation) => LowCutbackPool,
                (EventClass.SignatureExecuted, ShotCategory.PassShotImpact) => BlindSideNearPostPool,
                (EventClass.SignatureExecuted, ShotCategory.TacticalWide) => FirstTimeDiagonalSwitchPool,
                (EventClass.SignatureBreakthrough, ShotCategory.AftermathFreeze) => SignatureBreakthroughPool,
                _ => null,
            };
        }

        /// <summary>
        /// Pick a commentary line for the given event. Returns
        /// <see langword="false"/> when the event class + shot category
        /// combination has no commentary path (silent rather than
        /// throwing — restart events e.g. <c>GoalKickRestart</c> emit no
        /// ViewerEvents at Phase 3 anyway, but a future bridge adding
        /// new categories shouldn't crash the overlay; the new path is
        /// flagged by the matrix-completeness test
        /// <c>CommentaryTemplates_Matrix_AllBridgeEmittedKeysHaveAPool</c>).
        /// </summary>
        public static bool TryPickCommentary(ViewerEvent ev, out string line)
        {
            if (ev is null)
            {
                line = string.Empty;
                return false;
            }
            IReadOnlyList<string>? pool = PoolFor(ev.SourceEventClass, ResolveCategory(ev));
            if (pool is null || pool.Count == 0)
            {
                line = string.Empty;
                return false;
            }
            // Per-event Seed.Value is uniformly distributed (SplitMix64
            // mixer; see Seed.Derive) so simple modulo gives a stable
            // pick that survives replay. ev.Seed is itself derived from
            // (matchSeed, StartTick, ViewerEventId) at bridge time so
            // distinct events get distinct seeds.
            int idx = (int)(ev.Seed.Value % (ulong)pool.Count);
            line = pool[idx];
            return true;
        }

        // -------------------------------------------------------------
        // Signature title-card display names — keyed by SignatureId.
        // -------------------------------------------------------------

        // Verbatim from design/signatures.md §"Phase-3 active set" —
        // football-native register, lowercased "from the byline" /
        // "near-post run" per the design-doc casing convention.
        private static readonly IReadOnlyDictionary<string, string> SignatureDisplayNames =
            new Dictionary<string, string>(StringComparer.Ordinal)
            {
                ["fwh.core:signature.first-time-diagonal-switch"] = "First-time diagonal switch",
                ["fwh.core:signature.low-cutback-from-byline"] = "Low cutback from the byline",
                ["fwh.core:signature.blind-side-near-post-run"] = "Blind-side near-post run",
            };

        /// <summary>
        /// Look up the football-native display name for a signature ID.
        /// Throws on unknown IDs — same loud-fail discipline as
        /// <see cref="DotsAdapterRoot.ResolveShot"/>: a content-pack
        /// drift in <c>SignatureMetadata.SignatureId</c> must surface
        /// loud, not silently render the raw ID string in the title-card.
        /// </summary>
        public static string GetSignatureDisplayName(string signatureId)
        {
            if (string.IsNullOrEmpty(signatureId))
            {
                throw new ArgumentException(
                    "signatureId must be non-empty.", nameof(signatureId));
            }
            if (!SignatureDisplayNames.TryGetValue(signatureId, out string? displayName))
            {
                throw new KeyNotFoundException(
                    $"No display name registered for signatureId '{signatureId}'. " +
                    $"Phase-3 wired: {string.Join(", ", SignatureDisplayNames.Keys)}.");
            }
            return displayName;
        }

        // -------------------------------------------------------------
        // Title-card eligibility — Phase-3 fires on the 3 signature
        // executions only. Goal + SignatureBreakthrough are explicit no-ops
        // per the matrix above.
        // -------------------------------------------------------------

        public static bool ShouldShowTitleCard(EventClass eventClass, ShotCategory category)
        {
            return eventClass == EventClass.SignatureExecuted
                && (category == ShotCategory.PlayerIsolation
                    || category == ShotCategory.PassShotImpact
                    || category == ShotCategory.TacticalWide);
        }

        // -------------------------------------------------------------
        // Test surface — internal accessors for matrix-completeness +
        // pool-coverage + invariant tests.
        // -------------------------------------------------------------

        /// <summary>Test-only enumeration of every (EventClass, ShotCategory) commentary key.</summary>
        internal static readonly IReadOnlyList<(EventClass, ShotCategory)> CommentaryMatrixKeys = new[]
        {
            (EventClass.GoalScored, ShotCategory.PassShotImpact),
            (EventClass.SignatureExecuted, ShotCategory.PlayerIsolation),
            (EventClass.SignatureExecuted, ShotCategory.PassShotImpact),
            (EventClass.SignatureExecuted, ShotCategory.TacticalWide),
            (EventClass.SignatureBreakthrough, ShotCategory.AftermathFreeze),
        };

        /// <summary>Test-only access to the pool for a given matrix key.</summary>
        internal static IReadOnlyList<string>? PoolForTest(EventClass eventClass, ShotCategory category)
            => PoolFor(eventClass, category);

        /// <summary>Test-only enumeration of every signature ID with a registered display name.</summary>
        internal static IEnumerable<string> AllSignatureIds => SignatureDisplayNames.Keys;

        /// <summary>
        /// ShotCategory resolution — duplicates the helper in
        /// DotsMatchDirector but pulled inline here so CommentaryTemplates
        /// stays a self-contained pure-C# unit. Returns
        /// <see cref="ShotCategory.None"/> on a catalog miss; the caller
        /// (TryPickCommentary) treats None as "no commentary path".
        /// </summary>
        private static ShotCategory ResolveCategory(ViewerEvent ev)
        {
            if (ShotTypeCatalog.TryGet(ev.EffectiveShotTypeId, out ShotTypeDefinition def))
            {
                return def.Category;
            }
            return ShotCategory.None;
        }
    }
}

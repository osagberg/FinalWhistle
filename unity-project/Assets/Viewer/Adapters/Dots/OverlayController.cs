using System;
using FinalWhistle.MatchSim.Sim;
using UnityEngine;
using UnityEngine.UIElements;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// MonoBehaviour driver for the Phase-3 dots-adapter UI Toolkit
    /// overlay (Slice 6). Owns a child <see cref="UIDocument"/> sourced
    /// from <c>UI/DotsOverlay.uxml</c>; exposes a small typed API for
    /// <see cref="DotsMatchDirector"/> to drive scoreboard, commentary
    /// line, and signature title-card per the Slice-6 event-to-overlay
    /// matrix in <see cref="CommentaryTemplates"/>.
    ///
    /// <para>
    /// <strong>Topology</strong>: a dedicated <c>OverlayController</c>
    /// GameObject under the scene root (NOT attached to
    /// <see cref="DotsMatchDirector"/>) — keeps the director's lifecycle
    /// clean (it already owns <see cref="Time.fixedDeltaTime"/> override
    /// + shader globals + ViewerEvent dispatch) and lets L2 screenshot
    /// scripting toggle overlay state independently.
    /// <c>PanelSettings.sortingOrder = 1</c> renders above the URP
    /// camera output without fighting the Slice-5 RenderGraph passes.
    /// </para>
    ///
    /// <para>
    /// <strong>Determinism contract</strong>: NO frame-time inputs
    /// (<see cref="Time.time"/>, <see cref="Time.deltaTime"/>) feed any
    /// timing decision in the API surface. Scoreboard minute is
    /// canonical-tick-derived by the caller; title-card lifetime is
    /// driven by <c>ev.StartTick</c> / <c>ev.EndTick</c> / current
    /// canonical tick on the director side
    /// (<see cref="DotsMatchDirector"/> retires the title-card via
    /// <see cref="HideTitleCard"/> when the canonical tick passes the
    /// active event's <c>EndTick</c>). USS animations on the title-card
    /// are presentation-only (transitions on the <c>--visible</c> class
    /// toggle); reduce-motion suppresses the transition class entirely
    /// per Codex P2-4 closure.
    /// </para>
    ///
    /// <para>
    /// <strong>Reduce-motion</strong>: <see cref="ShowTitleCard"/>
    /// takes a <paramref name="reduceMotion"/> bool sourced from
    /// <see cref="ViewerEvent.ReduceMotionApplied"/>. When true, the
    /// title-card appears at full opacity instantly (no fade-in /
    /// type-pulse / slide); when false, the standard USS transition
    /// runs. The renderer-feature side already gates impact-frame +
    /// screen-tone on reduce-motion (Slice 5 round-1); this surface
    /// extends the same discipline to the typography rhythm per
    /// design/anime-presentation-budget.md surface #8.
    /// </para>
    /// </summary>
    [DisallowMultipleComponent]
    public sealed class OverlayController : MonoBehaviour
    {
        [Tooltip("UIDocument component on this GameObject. Must reference DotsOverlay.uxml + the dots-adapter PanelSettings asset.")]
        [SerializeField] private UIDocument uiDocument;

        // Cached element queries — populated in OnEnable; nulled in
        // OnDisable so a hot-reload re-resolves from the rebuilt tree.
        // Per pr-review-toolkit type-design-analyzer Slice-2 P2's
        // "dots == null" pattern: the null check IS the initialization
        // guard, no separate bool flag.
        private Label scoreLabelHome;
        private Label scoreLabelAway;
        private Label minuteLabel;
        private Label commentaryLabel;
        private VisualElement commentaryRoot;
        private VisualElement titleCardRoot;
        private Label titleCardSignatureLabel;
        private Label titleCardPlayerLabel;

        // USS class names — kept as constants here so a stylesheet edit
        // that drifts the class name surfaces as a missing-class warning
        // at runtime rather than silent layout breakage.
        private const string CommentaryActiveClass = "fw-commentary--active";
        private const string TitleCardVisibleClass = "fw-title-card--visible";
        private const string TitleCardReduceMotionClass = "fw-title-card--reduce-motion";
        private const string TitleCardHomeClass = "fw-title-card--home";
        private const string TitleCardAwayClass = "fw-title-card--away";

        private void OnEnable()
        {
            if (uiDocument == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(OverlayController)}.{nameof(uiDocument)} reference missing; " +
                    "assign the UIDocument component in the scene inspector.");
            }
            VisualElement root = uiDocument.rootVisualElement;
            if (root == null)
            {
                // UIDocument can return null root in some edit-mode hot-
                // reload paths; surface loud rather than silently
                // produce a no-op overlay.
                throw new InvalidOperationException(
                    $"{nameof(OverlayController)}: UIDocument.rootVisualElement is null. " +
                    "Verify that DotsOverlay.uxml is assigned to UIDocument.visualTreeAsset.");
            }

            scoreLabelHome = ResolveLabel(root, "fw-score-home");
            scoreLabelAway = ResolveLabel(root, "fw-score-away");
            minuteLabel = ResolveLabel(root, "fw-minute");
            commentaryLabel = ResolveLabel(root, "fw-commentary-text");
            commentaryRoot = ResolveElement(root, "fw-commentary");
            titleCardRoot = ResolveElement(root, "fw-title-card");
            titleCardSignatureLabel = ResolveLabel(root, "fw-title-card-signature");
            titleCardPlayerLabel = ResolveLabel(root, "fw-title-card-player");

            // Hide title-card at startup (no event has fired yet).
            titleCardRoot.RemoveFromClassList(TitleCardVisibleClass);
            titleCardRoot.RemoveFromClassList(TitleCardReduceMotionClass);
            titleCardRoot.RemoveFromClassList(TitleCardHomeClass);
            titleCardRoot.RemoveFromClassList(TitleCardAwayClass);
            commentaryRoot.RemoveFromClassList(CommentaryActiveClass);
        }

        private void OnDisable()
        {
            scoreLabelHome = null;
            scoreLabelAway = null;
            minuteLabel = null;
            commentaryLabel = null;
            commentaryRoot = null;
            titleCardRoot = null;
            titleCardSignatureLabel = null;
            titleCardPlayerLabel = null;
        }

        /// <summary>
        /// Set the scoreboard digits. Caller passes
        /// <see cref="MatchSimulationState.HomeScore"/> /
        /// <see cref="MatchSimulationState.AwayScore"/> directly (both
        /// <see langword="byte"/>; promote to int at the call site).
        /// </summary>
        public void SetScore(int home, int away)
        {
            EnsureBound();
            scoreLabelHome.text = home.ToString(System.Globalization.CultureInfo.InvariantCulture);
            scoreLabelAway.text = away.ToString(System.Globalization.CultureInfo.InvariantCulture);
        }

        /// <summary>
        /// Set the minute display. Caller derives from
        /// <c>state.CurrentTick.Value / Tick.TicksPerSecond / 60</c> —
        /// the controller never reads tick state directly.
        /// </summary>
        public void SetMinute(int minute)
        {
            EnsureBound();
            // Phase-3: clamp to [0, 90] for display sanity. Phase-4+
            // adds stoppage-time text (e.g. "45+2") via a separate
            // SetStoppageMinute(int, int) overload.
            int clamped = minute < 0 ? 0 : minute > 90 ? 90 : minute;
            minuteLabel.text = clamped.ToString(System.Globalization.CultureInfo.InvariantCulture) + "'";
        }

        /// <summary>
        /// Replace the commentary line text + toggle the
        /// <c>fw-commentary--active</c> USS class on so any
        /// USS-defined fade-in transition runs. No internal queue at
        /// Phase 3 — ViewerEvents are sparse (&lt;10 per match) so a
        /// rapid second event simply replaces the prior line.
        /// </summary>
        public void PushCommentary(string line)
        {
            EnsureBound();
            if (string.IsNullOrEmpty(line))
            {
                throw new ArgumentException(
                    "Commentary line must be non-empty.", nameof(line));
            }
            commentaryLabel.text = line;
            commentaryRoot.AddToClassList(CommentaryActiveClass);
        }

        /// <summary>
        /// Show the signature title-card with the given football-native
        /// display name + player name + side tinting. Reduce-motion
        /// flag suppresses the USS transition class so the card appears
        /// instantly per design/anime-presentation-budget.md surface #8
        /// reduce-motion semantics.
        /// </summary>
        public void ShowTitleCard(
            string signatureDisplayName,
            string playerName,
            TeamSide side,
            bool reduceMotion)
        {
            EnsureBound();
            if (string.IsNullOrEmpty(signatureDisplayName))
            {
                throw new ArgumentException(
                    "signatureDisplayName must be non-empty.", nameof(signatureDisplayName));
            }
            // playerName may be empty in fixture cases where no jersey
            // is attributed; the UXML lays out the row with the
            // signature on top + player below, and an empty player
            // string just collapses that row visually (USS handles).
            titleCardSignatureLabel.text = signatureDisplayName;
            titleCardPlayerLabel.text = playerName ?? string.Empty;

            // Side tint — use USS classes rather than inline color so
            // the palette is editable in DotsOverlay.uss without code
            // changes (matches the Slice-2 IdentityTintTable discipline
            // of "palette in data, not in code").
            titleCardRoot.RemoveFromClassList(TitleCardHomeClass);
            titleCardRoot.RemoveFromClassList(TitleCardAwayClass);
            titleCardRoot.AddToClassList(side == TeamSide.Home
                ? TitleCardHomeClass
                : TitleCardAwayClass);

            // Reduce-motion class first (before --visible) so the USS
            // selector `.fw-title-card--reduce-motion.fw-title-card--visible`
            // overrides the default transition.
            if (reduceMotion)
            {
                titleCardRoot.AddToClassList(TitleCardReduceMotionClass);
            }
            else
            {
                titleCardRoot.RemoveFromClassList(TitleCardReduceMotionClass);
            }
            titleCardRoot.AddToClassList(TitleCardVisibleClass);
        }

        /// <summary>
        /// Hide the title-card. Called by the director when the active
        /// event's <c>EndTick</c> has elapsed (canonical-tick driven).
        /// </summary>
        public void HideTitleCard()
        {
            EnsureBound();
            titleCardRoot.RemoveFromClassList(TitleCardVisibleClass);
            titleCardRoot.RemoveFromClassList(TitleCardReduceMotionClass);
        }

        // -------------------------------------------------------------
        // Helpers
        // -------------------------------------------------------------

        private void EnsureBound()
        {
            if (scoreLabelHome == null || scoreLabelAway == null
                || minuteLabel == null || commentaryLabel == null
                || commentaryRoot == null || titleCardRoot == null
                || titleCardSignatureLabel == null || titleCardPlayerLabel == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(OverlayController)} not bound — OnEnable did not run, " +
                    "or the GameObject is disabled. Verify the controller is enabled + " +
                    "DotsOverlay.uxml has the expected element names " +
                    "(fw-score-home, fw-score-away, fw-minute, fw-commentary, " +
                    "fw-commentary-text, fw-title-card, fw-title-card-signature, " +
                    "fw-title-card-player).");
            }
        }

        private static Label ResolveLabel(VisualElement root, string name)
        {
            Label label = root.Q<Label>(name);
            if (label == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(OverlayController)}: UXML missing Label '{name}'. " +
                    "Verify DotsOverlay.uxml has a <Label name=\"" + name + "\" />.");
            }
            return label;
        }

        private static VisualElement ResolveElement(VisualElement root, string name)
        {
            VisualElement element = root.Q<VisualElement>(name);
            if (element == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(OverlayController)}: UXML missing VisualElement '{name}'. " +
                    "Verify DotsOverlay.uxml has an element with name=\"" + name + "\".");
            }
            return element;
        }
    }
}

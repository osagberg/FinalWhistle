using System;
using FinalWhistle.MatchSim.Sim;
using UnityEngine;
using UnityEngine.UI;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// MonoBehaviour driver for the Phase-3 dots-adapter overlay
    /// (Slice 6). Drives a UGUI <see cref="Canvas"/> hierarchy with
    /// legacy <see cref="UnityEngine.UI.Text"/> labels for scoreboard,
    /// commentary line, and signature title-card per the Slice-6
    /// event-to-overlay matrix in <see cref="CommentaryTemplates"/>.
    ///
    /// <para>
    /// <strong>UGUI vs UI Toolkit (Slice-6 round-1 P1 closure)</strong>:
    /// the original Slice-6 implementation used UI Toolkit
    /// (<c>UIDocument</c> + UXML/USS) per the dots-adapter blueprint
    /// and CLAUDE.md tech-stack lock. In this Unity 6.0.4 + URP 17.4
    /// + Mac/Metal configuration the UI Toolkit runtime panel did not
    /// composite painted pixels into the framebuffer despite a
    /// verified-correct tree (8 labels populated, fonts resolved,
    /// PanelSettings cloned from a known-good Cinemachine reference,
    /// console clean, Slice-5 RenderGraph features ruled out).
    /// Codex round-1 review against <c>67c0905</c> flagged this as
    /// a P1 acceptance blocker. UGUI is the documented fallback per
    /// <c>.claude/rules/Scripts/Viewer/RULES.md</c> "UGUI fallback only
    /// when UI Toolkit lacks the surface" — Slice 6 ships UGUI; the
    /// authored UXML/USS stays on disk as a Phase-4+ migration scaffold
    /// once the UI Toolkit composition issue is diagnosed.
    /// </para>
    ///
    /// <para>
    /// <strong>Topology</strong>: a dedicated <c>OverlayController</c>
    /// GameObject hosts a Canvas in Screen Space — Overlay mode
    /// (sortingOrder=1, renders above all camera output) plus the
    /// scoreboard / commentary / title-card child labels. The director
    /// references the <c>OverlayController</c> via SerializeField; all
    /// public methods are zero-allocation (label.text writes only).
    /// </para>
    ///
    /// <para>
    /// <strong>Determinism contract</strong> (preserved from the UI
    /// Toolkit version): NO frame-time inputs feed any timing decision
    /// in the API surface. Scoreboard minute is canonical-tick-derived
    /// by the caller; title-card lifetime is driven by
    /// <c>ev.StartTick</c> / <c>ev.EndTick</c> on the director side.
    /// Title-card visibility is a binary GameObject.SetActive — no
    /// frame-time animations.
    /// </para>
    ///
    /// <para>
    /// <strong>Reduce-motion</strong>: <see cref="ShowTitleCard"/>
    /// takes a <paramref name="reduceMotion"/> bool sourced from
    /// <see cref="ViewerEvent.ReduceMotionApplied"/>. UGUI legacy Text
    /// has no built-in transition, so the title-card always appears
    /// instantly — meaning reduce-motion is the default behaviour at
    /// Phase 3. The flag is preserved on the API for forward
    /// compatibility with the Phase-4+ UI Toolkit migration that
    /// re-introduces fade/translate transitions.
    /// </para>
    /// </summary>
    [DisallowMultipleComponent]
    public sealed class OverlayController : MonoBehaviour
    {
        [Tooltip("UGUI Text component for the home team's score digit.")]
        [SerializeField] private Text scoreLabelHome;

        [Tooltip("UGUI Text component for the away team's score digit.")]
        [SerializeField] private Text scoreLabelAway;

        [Tooltip("UGUI Text component for the match-minute label (top-centre between scores).")]
        [SerializeField] private Text minuteLabel;

        [Tooltip("UGUI Text component for the commentary line (bottom-left strip).")]
        [SerializeField] private Text commentaryLabel;

        [Tooltip("Root GameObject of the commentary panel (parent of commentaryLabel). Toggled active when a commentary line is pushed.")]
        [SerializeField] private GameObject commentaryRoot;

        [Tooltip("Root GameObject of the signature title-card (parent of the two title-card labels). Toggled active by ShowTitleCard / HideTitleCard.")]
        [SerializeField] private GameObject titleCardRoot;

        [Tooltip("UGUI Text component for the signature display name (e.g., 'Low cutback from the byline').")]
        [SerializeField] private Text titleCardSignatureLabel;

        [Tooltip("UGUI Text component for the player attribution under the signature.")]
        [SerializeField] private Text titleCardPlayerLabel;

        [Tooltip("Image component for the title-card's side-tinted accent strip. Tint set via colour swap on home/away events.")]
        [SerializeField] private Image titleCardAccent;

        [Tooltip("Side-tint colour for home-team signatures (cool blue). Matches the Slice-2 IdentityTintTable home palette band.")]
        [SerializeField] private Color titleCardHomeColor = new(0.275f, 0.569f, 0.941f, 1f);

        [Tooltip("Side-tint colour for away-team signatures (warm coral). Matches the Slice-2 IdentityTintTable away palette band.")]
        [SerializeField] private Color titleCardAwayColor = new(0.902f, 0.373f, 0.314f, 1f);

        private void OnEnable()
        {
            // Loud-fail validation: every SerializeField must be wired
            // for the API surface to behave. A missing reference is a
            // scene-wiring bug, not a runtime fallback case.
            EnsureWired(scoreLabelHome, nameof(scoreLabelHome));
            EnsureWired(scoreLabelAway, nameof(scoreLabelAway));
            EnsureWired(minuteLabel, nameof(minuteLabel));
            EnsureWired(commentaryLabel, nameof(commentaryLabel));
            EnsureWired(commentaryRoot, nameof(commentaryRoot));
            EnsureWired(titleCardRoot, nameof(titleCardRoot));
            EnsureWired(titleCardSignatureLabel, nameof(titleCardSignatureLabel));
            EnsureWired(titleCardPlayerLabel, nameof(titleCardPlayerLabel));
            EnsureWired(titleCardAccent, nameof(titleCardAccent));

            // Hide title-card + commentary at startup; they show only on event.
            titleCardRoot.SetActive(false);
            commentaryRoot.SetActive(false);
        }

        /// <summary>
        /// Set the scoreboard digits. Caller passes
        /// <see cref="MatchSim.Sim.MatchSimulationState.HomeScore"/> /
        /// <see cref="MatchSim.Sim.MatchSimulationState.AwayScore"/>
        /// directly (both <see langword="byte"/>; promote to int at
        /// the call site).
        /// </summary>
        public void SetScore(int home, int away)
        {
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
            int clamped = minute < 0 ? 0 : minute > 90 ? 90 : minute;
            minuteLabel.text = clamped.ToString(System.Globalization.CultureInfo.InvariantCulture) + "'";
        }

        /// <summary>
        /// Replace the commentary line text + show the panel. No
        /// internal queue at Phase 3 — ViewerEvents are sparse
        /// (&lt;10 per match) so a rapid second event simply replaces
        /// the prior line.
        /// </summary>
        public void PushCommentary(string line)
        {
            if (string.IsNullOrEmpty(line))
            {
                throw new ArgumentException(
                    "Commentary line must be non-empty.", nameof(line));
            }
            commentaryLabel.text = line;
            commentaryRoot.SetActive(true);
        }

        /// <summary>
        /// Show the signature title-card with the given football-native
        /// display name + player name + side tinting. Reduce-motion
        /// flag preserved for forward compatibility with the Phase-4+
        /// UI Toolkit migration; UGUI fallback has no animations so the
        /// flag is currently a no-op.
        /// </summary>
        public void ShowTitleCard(
            string signatureDisplayName,
            string playerName,
            TeamSide side,
            bool reduceMotion)
        {
            if (string.IsNullOrEmpty(signatureDisplayName))
            {
                throw new ArgumentException(
                    "signatureDisplayName must be non-empty.", nameof(signatureDisplayName));
            }
            titleCardSignatureLabel.text = signatureDisplayName;
            titleCardPlayerLabel.text = playerName ?? string.Empty;
            titleCardAccent.color = side == TeamSide.Home ? titleCardHomeColor : titleCardAwayColor;
            titleCardRoot.SetActive(true);
            // reduceMotion intentionally unused at Phase 3 (UGUI
            // fallback has no transitions to suppress); preserved on
            // the signature for the Phase-4+ UI Toolkit migration.
            _ = reduceMotion;
        }

        /// <summary>
        /// Hide the title-card. Called by the director when the active
        /// event's <c>EndTick</c> has elapsed (canonical-tick driven).
        /// </summary>
        public void HideTitleCard()
        {
            titleCardRoot.SetActive(false);
        }

        private static void EnsureWired(UnityEngine.Object component, string fieldName)
        {
            if (component == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(OverlayController)}.{fieldName} reference missing; " +
                    "wire it in the scene inspector.");
            }
        }
    }
}

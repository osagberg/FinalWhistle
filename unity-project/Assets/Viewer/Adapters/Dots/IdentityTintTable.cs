using System;
using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// ScriptableObject mapping <see cref="RoleFamily"/> + <see cref="TeamSide"/>
    /// → <see cref="UnityEngine.Color"/> for the Phase-3 dots adapter per
    /// blueprint Decision 5 (player dot identity = role-family + team-side
    /// colour tinting at Phase 3; jersey-number rendering deferred to
    /// Phase-4 player-isolation shot work because 23 TextMeshPro instances
    /// at 60fps would burn the allocation budget and the digits are
    /// unreadable at tactical-wide zoom anyway).
    ///
    /// <para>
    /// <strong>Phase-3 starting palette</strong> (subject to async
    /// art-director review): home plays cool blues with an orange
    /// goalkeeper; away plays warm reds/coral with a violet goalkeeper.
    /// Within each side the gradient brightens from defence to attack so
    /// formation depth reads at full-pitch tactical-wide zoom. The shared
    /// "non-outfield" GK colour pair (orange + violet) is intentionally
    /// loud to make the only-allowed-keeper visually instant per the
    /// ADR-0009 polish-bar §"identity legibility" criterion.
    /// </para>
    ///
    /// <para>
    /// <strong>Documented band merges</strong> (Phase-3): defenders
    /// (CB ≡ FB), defensive-mid + central-mid (DM ≡ CM), and
    /// attacking-mid + winger (AM ≡ Winger) each share a colour so the
    /// formation reads as four bands per side rather than eight (the
    /// Phase-3 archetype YAMLs only emit 4 distinct bands per side
    /// anyway). <see cref="OnValidate"/> asserts the merges so a
    /// well-meaning author drifting one of the merged pairs in the
    /// inspector trips loud rather than silently fragmenting the formation
    /// readability. Phase-4 may unmerge any pair by deleting the
    /// corresponding assertion + commenting why.
    /// </para>
    /// </summary>
    [CreateAssetMenu(
        menuName = "Final Whistle/Viewer/Identity Tint Table",
        fileName = "IdentityTintTable")]
    public sealed class IdentityTintTable : ScriptableObject
    {
        [SerializeField] private Color homeGoalkeeper = new(1f, 0.533f, 0f);          // #FF8800
        [SerializeField] private Color homeCentreBack = new(0.118f, 0.353f, 0.659f);  // #1E5AA8
        [SerializeField] private Color homeFullBack = new(0.118f, 0.353f, 0.659f);    // merged with CB per documented band rule
        [SerializeField] private Color homeDefensiveMid = new(0.231f, 0.486f, 0.769f); // #3B7CC4
        [SerializeField] private Color homeCentralMid = new(0.231f, 0.486f, 0.769f);   // merged with DM per documented band rule
        [SerializeField] private Color homeAttackingMid = new(0.373f, 0.627f, 0.910f); // #5FA0E8
        [SerializeField] private Color homeWinger = new(0.373f, 0.627f, 0.910f);       // merged with AM per documented band rule
        [SerializeField] private Color homeStriker = new(0.561f, 0.769f, 1f);          // #8FC4FF — palest blue = furthest forward

        [SerializeField] private Color awayGoalkeeper = new(0.6f, 0.2f, 0.8f);         // #9933CC
        [SerializeField] private Color awayCentreBack = new(0.659f, 0.149f, 0.118f);   // #A8261E
        [SerializeField] private Color awayFullBack = new(0.659f, 0.149f, 0.118f);     // merged with CB
        [SerializeField] private Color awayDefensiveMid = new(0.769f, 0.290f, 0.231f); // #C44A3B
        [SerializeField] private Color awayCentralMid = new(0.769f, 0.290f, 0.231f);   // merged with DM
        [SerializeField] private Color awayAttackingMid = new(0.910f, 0.478f, 0.373f); // #E87A5F
        [SerializeField] private Color awayWinger = new(0.910f, 0.478f, 0.373f);       // merged with AM
        [SerializeField] private Color awayStriker = new(1f, 0.659f, 0.561f);          // #FFA88F

        /// <summary>
        /// Look up the tint colour for a player dot. Throws on unknown enum
        /// values — silent fallback to a default colour would mask
        /// content-pack drift in Phase 4+ where new role families may be
        /// introduced via schema bump (caller is responsible for keeping the
        /// tint table updated).
        /// </summary>
        public Color Lookup(RoleFamily role, TeamSide side)
        {
            return (side, role) switch
            {
                (TeamSide.Home, RoleFamily.Goalkeeper) => homeGoalkeeper,
                (TeamSide.Home, RoleFamily.CentreBack) => homeCentreBack,
                (TeamSide.Home, RoleFamily.FullBack) => homeFullBack,
                (TeamSide.Home, RoleFamily.DefensiveMidfielder) => homeDefensiveMid,
                (TeamSide.Home, RoleFamily.CentralMidfielder) => homeCentralMid,
                (TeamSide.Home, RoleFamily.AttackingMidfielder) => homeAttackingMid,
                (TeamSide.Home, RoleFamily.Winger) => homeWinger,
                (TeamSide.Home, RoleFamily.Striker) => homeStriker,
                (TeamSide.Away, RoleFamily.Goalkeeper) => awayGoalkeeper,
                (TeamSide.Away, RoleFamily.CentreBack) => awayCentreBack,
                (TeamSide.Away, RoleFamily.FullBack) => awayFullBack,
                (TeamSide.Away, RoleFamily.DefensiveMidfielder) => awayDefensiveMid,
                (TeamSide.Away, RoleFamily.CentralMidfielder) => awayCentralMid,
                (TeamSide.Away, RoleFamily.AttackingMidfielder) => awayAttackingMid,
                (TeamSide.Away, RoleFamily.Winger) => awayWinger,
                (TeamSide.Away, RoleFamily.Striker) => awayStriker,
                // Both side and role are jointly the offender when a future
                // enum extension lands; pr-review-toolkit silent-failure-hunter
                // round-1 P3 against b8d400f-equivalent flagged the prior
                // single-paramName attribution.
                _ => throw new ArgumentException(
                    $"Unknown (TeamSide, RoleFamily) pair: ({side}, {role}). " +
                    $"Add a row to IdentityTintTable when introducing a new role family.",
                    paramName: $"{nameof(side)}, {nameof(role)}"),
            };
        }

#if UNITY_EDITOR
        /// <summary>
        /// Asserts the documented band merges (CB ≡ FB; DM ≡ CM; AM ≡ Winger)
        /// per pr-review-toolkit type-design-analyzer Slice-2 P1: the 16
        /// individual SerializeField shape allowed an inspector author to
        /// silently fragment the merge by drifting one half of a pair, making
        /// the formation read as 8 bands instead of the documented 4 +
        /// keeper. Editor-only — won't affect runtime.
        /// </summary>
        private void OnValidate()
        {
            AssertMerged(nameof(homeCentreBack), homeCentreBack, nameof(homeFullBack), homeFullBack);
            AssertMerged(nameof(homeDefensiveMid), homeDefensiveMid, nameof(homeCentralMid), homeCentralMid);
            AssertMerged(nameof(homeAttackingMid), homeAttackingMid, nameof(homeWinger), homeWinger);
            AssertMerged(nameof(awayCentreBack), awayCentreBack, nameof(awayFullBack), awayFullBack);
            AssertMerged(nameof(awayDefensiveMid), awayDefensiveMid, nameof(awayCentralMid), awayCentralMid);
            AssertMerged(nameof(awayAttackingMid), awayAttackingMid, nameof(awayWinger), awayWinger);
        }

        private void AssertMerged(string nameA, Color a, string nameB, Color b)
        {
            if (a != b)
            {
                Debug.LogWarning(
                    $"IdentityTintTable: documented band merge violated — " +
                    $"{nameA} ({a:F3}) != {nameB} ({b:F3}). Phase-3 design intent " +
                    $"is for these to share a colour so the formation reads as 4 bands per side. " +
                    $"To intentionally unmerge, delete this assertion and document why in the SO doc-comment.",
                    this);
            }
        }
#endif
    }
}

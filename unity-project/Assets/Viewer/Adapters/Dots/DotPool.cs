using System;
using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Pool of 23 <see cref="SpriteRenderer"/> children (11 home + 11 away
    /// + 1 ball) per the Phase-3 dots-adapter blueprint §B Slice 2.
    /// Children are pre-instantiated under the pool transform on
    /// <see cref="Initialize"/> + never destroyed at runtime; positions are
    /// updated in-place from <see cref="PitchView.FixedToWorld"/>. Sprite
    /// transforms are rotated <c>Euler(90, 0, 0)</c> on the X axis so the
    /// sprite quad lies flat in the XZ pitch plane (default sprite
    /// orientation faces -Z which is edge-on under the top-down camera).
    ///
    /// <para>
    /// <strong>Sprites are serialized references</strong>, not loaded via
    /// <c>Resources.Load</c>: keeps the slice-2 file layout under
    /// <c>Adapters/Dots/Sprites/</c> without forcing a Unity-special
    /// <c>Resources/</c> folder + supports a clean Addressables migration
    /// later in Phase 4+. Wired through the scene .asset, not authored
    /// inline.
    /// </para>
    ///
    /// <para>
    /// <strong>Initialization gate</strong> (pr-review-toolkit
    /// feature-dev:code-reviewer Slice-2 P1): the prior
    /// <c>private bool initialized</c> field was unreliable — domain
    /// reload re-deserialized it as <c>false</c> while the spawned dot
    /// children persisted in the scene hierarchy, causing
    /// <see cref="SetFormationPositions"/> to throw incorrectly + a
    /// re-<see cref="Initialize"/> to spawn 23 new orphan dots. The
    /// <c>dots</c> array is the truth — it goes <c>null</c> on domain
    /// reload along with the rest of the runtime state, so gating on
    /// <c>dots == null</c> is self-consistent.
    /// </para>
    /// </summary>
    public sealed class DotPool : MonoBehaviour
    {
        public const int PlayersPerSide = 11;
        public const int TotalPlayers = PlayersPerSide * 2;
        public const int TotalDots = TotalPlayers + 1;
        public const int BallIndex = TotalPlayers;

        private const float OutfieldDiameterMetres = 1.4f;
        private const float GoalkeeperDiameterMetres = 1.6f;
        private const float BallDiameterMetres = 0.7f;
        private const float DotYLift = 0.05f;

        [SerializeField] private IdentityTintTable identityTintTable;
        [SerializeField] private Sprite homeDotSprite;
        [SerializeField] private Sprite awayDotSprite;
        [SerializeField] private Sprite ballSprite;

        private PitchView pitchView;
        private SpriteRenderer[] dots;

        /// <summary>
        /// Pre-instantiate the 23-dot pool under this transform. Idempotent
        /// — re-calling clears the prior pool first. Throws if the
        /// inspector references aren't wired (sprite assets + tint table).
        /// </summary>
        public void Initialize(PitchView pitchViewArg)
        {
            if (pitchViewArg is null)
            {
                throw new ArgumentNullException(nameof(pitchViewArg));
            }
            if (identityTintTable == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(IdentityTintTable)} reference is not wired on this DotPool. " +
                    "Assign the IdentityTintTable.asset in the scene inspector.");
            }
            if (homeDotSprite == null || awayDotSprite == null || ballSprite == null)
            {
                throw new InvalidOperationException(
                    "Dot sprite references missing. Assign homeDotSprite + awayDotSprite + " +
                    "ballSprite in the scene inspector (Sprites under Adapters/Dots/Sprites/).");
            }

            pitchView = pitchViewArg;

            // Clear any prior children from a previous Initialize call.
            for (int i = transform.childCount - 1; i >= 0; i--)
            {
                Transform child = transform.GetChild(i);
                if (Application.isPlaying)
                {
                    Destroy(child.gameObject);
                }
                else
                {
                    DestroyImmediate(child.gameObject);
                }
            }

            dots = new SpriteRenderer[TotalDots];
            for (int home = 0; home < PlayersPerSide; home++)
            {
                dots[home] = CreateDot($"Player_Home_{home}", homeDotSprite, OutfieldDiameterMetres);
            }
            for (int away = 0; away < PlayersPerSide; away++)
            {
                dots[PlayersPerSide + away] = CreateDot($"Player_Away_{away}", awayDotSprite, OutfieldDiameterMetres);
            }
            dots[BallIndex] = CreateDot("Ball", ballSprite, BallDiameterMetres);
            dots[BallIndex].color = Color.white;
        }

        private SpriteRenderer CreateDot(string name, Sprite sprite, float diameterMetres)
        {
            GameObject go = new(name);
            go.transform.SetParent(transform, worldPositionStays: false);
            // Lay sprite flat in pitch plane — default SpriteRenderer faces
            // -Z which is edge-on under our top-down -Y camera and renders
            // as a 1px line otherwise.
            go.transform.localRotation = Quaternion.Euler(90f, 0f, 0f);
            float scale = diameterMetres * pitchView.WorldUnitsPerMeter;
            go.transform.localScale = new Vector3(scale, scale, scale);
            SpriteRenderer sr = go.AddComponent<SpriteRenderer>();
            sr.sprite = sprite;
            // Lift slightly above pitch quad to avoid depth-sort ambiguity
            // with the URP/Unlit pitch material at Y=0.
            go.transform.localPosition = new Vector3(0f, DotYLift, 0f);
            return sr;
        }

        /// <summary>
        /// Place the 23 dots at formation positions for Slice 2's static
        /// scene. Slice 3 swaps this for a per-tick UpdatePositions call
        /// driven by <see cref="MatchSimulationRunner"/> output.
        /// </summary>
        public void SetFormationPositions(
            BehaviorTreeArchetype homeArchetype,
            BehaviorTreeArchetype awayArchetype,
            Vector3Fixed ballPosition)
        {
            EnsureInitialized();
            if (homeArchetype is null) throw new ArgumentNullException(nameof(homeArchetype));
            if (awayArchetype is null) throw new ArgumentNullException(nameof(awayArchetype));

            for (int i = 0; i < PlayersPerSide; i++)
            {
                FormationSlot slot = homeArchetype.Formation[i];
                Vector3Fixed pos = slot.HomeBasePosition;
                Vector3 worldPos = pitchView.FixedToWorld(pos);
                SpriteRenderer dot = dots[i];
                ApplyDotPlacement(dot, worldPos, slot.Role, TeamSide.Home);
            }
            for (int i = 0; i < PlayersPerSide; i++)
            {
                FormationSlot slot = awayArchetype.Formation[i];
                Vector3Fixed pos = slot.AwayBasePosition();
                Vector3 worldPos = pitchView.FixedToWorld(pos);
                SpriteRenderer dot = dots[PlayersPerSide + i];
                ApplyDotPlacement(dot, worldPos, slot.Role, TeamSide.Away);
            }

            Vector3 ballWorld = pitchView.FixedToWorld(ballPosition);
            Transform ballT = dots[BallIndex].transform;
            // Preserve local Y lift so ball stays above pitch.
            ballT.position = new Vector3(ballWorld.x, ballT.position.y, ballWorld.z);
        }

        // Centralised lifecycle gate per pr-review-toolkit
        // type-design-analyzer Slice-2 P2: future Slice-3 UpdatePositions +
        // Slice-5 SetSignaturePulse mutators inherit the precondition
        // mechanically rather than each re-implementing the check. Gates
        // on the array, not a bool, because the bool would re-deserialize
        // as false after a domain reload while the array goes null in
        // step with the rest of the runtime state.
        private void EnsureInitialized()
        {
            if (dots == null)
            {
                throw new InvalidOperationException(
                    $"DotPool not initialized; call {nameof(Initialize)} before placing dots.");
            }
        }

        private void ApplyDotPlacement(SpriteRenderer dot, Vector3 worldPos, string roleLabel, TeamSide side)
        {
            RoleFamily role = ArchetypeRoleParser.RoleFamilyForLabel(roleLabel);
            dot.color = identityTintTable.Lookup(role, side);
            float diameter = role == RoleFamily.Goalkeeper
                ? GoalkeeperDiameterMetres
                : OutfieldDiameterMetres;
            float scale = diameter * pitchView.WorldUnitsPerMeter;
            // Preserve local Y lift; only X/Z come from sim.
            Transform t = dot.transform;
            t.position = new Vector3(worldPos.x, t.position.y, worldPos.z);
            t.localScale = new Vector3(scale, scale, scale);
        }
    }
}

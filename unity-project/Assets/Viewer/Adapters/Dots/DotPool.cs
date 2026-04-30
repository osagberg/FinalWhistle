using System;
using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Pool of 23 <see cref="SpriteRenderer"/> children (11 home + 11 away
    /// + 1 ball) per the Phase-3 dots-adapter blueprint §B Slice 2/3.
    /// Children are pre-instantiated under the pool transform on
    /// <see cref="Initialize"/> + never destroyed at runtime; positions are
    /// updated in-place from <see cref="PitchView.FixedToWorld"/>. Sprite
    /// transforms are rotated <c>Euler(90, 0, 0)</c> on the X axis so the
    /// sprite quad lies flat in the XZ pitch plane.
    ///
    /// <para>
    /// <strong>Slice-3 sub-tick interpolation:</strong> Unity's
    /// <c>FixedUpdate</c> drives <see cref="MatchSimulationRunner.RunTicks"/>
    /// at exactly 60Hz to match the canonical sim rate, but the display
    /// renders at variable framerate. To keep dot motion smooth between
    /// ticks, the director calls <see cref="PushTickSnapshot"/> once per
    /// FixedUpdate with the pre-tick + post-tick player/ball positions;
    /// <see cref="Update"/> then linearly interpolates each dot's transform
    /// based on <c>(Time.time - lastFixedTime) / Time.fixedDeltaTime</c>.
    /// Interpolation is presentation-only — never feeds back to canonical
    /// state; reads <c>Time.time</c> only for the alpha factor; output is
    /// <c>transform.position</c> which never enters MatchSim.
    /// </para>
    ///
    /// <para>
    /// <strong>Sprites are serialized references</strong>, not loaded via
    /// <c>Resources.Load</c>: keeps the slice-2 file layout under
    /// <c>Adapters/Dots/Sprites/</c> without forcing a Unity-special
    /// <c>Resources/</c> folder + supports a clean Addressables migration
    /// later in Phase 4+.
    /// </para>
    ///
    /// <para>
    /// <strong>Initialization gate:</strong> <c>dots == null</c> is the
    /// truthful gate (the bool would re-deserialize as <c>false</c> after a
    /// Unity domain reload while spawned children persist; the array goes
    /// null in step with the rest of the runtime state).
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

        // Slice-3 interpolation snapshots. prev*/current* hold the previous
        // and current FixedUpdate's world-space dot positions; lastFixedTime
        // marks when the current snapshot was taken so Update can compute
        // the interpolation alpha.
        private Vector3[] prevPositions;
        private Vector3[] currentPositions;
        private float lastFixedTime;
        private bool snapshotsInitialized;

        // Cached archetypes (per pr-review-toolkit type-design-analyzer
        // Slice-3 P2): the formation layouts are fixed at match-start for
        // Phase-3, so PushTickSnapshot doesn't need them per-call. SetFormationPositions
        // captures them once + Initialize / PushTickSnapshot read from here.
        private BehaviorTreeArchetype cachedHomeArchetype;
        private BehaviorTreeArchetype cachedAwayArchetype;

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
            prevPositions = new Vector3[TotalDots];
            currentPositions = new Vector3[TotalDots];
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
            snapshotsInitialized = false;
            cachedHomeArchetype = null;
            cachedAwayArchetype = null;
        }

        private SpriteRenderer CreateDot(string name, Sprite sprite, float diameterMetres)
        {
            GameObject go = new(name);
            go.transform.SetParent(transform, worldPositionStays: false);
            go.transform.localRotation = Quaternion.Euler(90f, 0f, 0f);
            float scale = diameterMetres * pitchView.WorldUnitsPerMeter;
            go.transform.localScale = new Vector3(scale, scale, scale);
            SpriteRenderer sr = go.AddComponent<SpriteRenderer>();
            sr.sprite = sprite;
            go.transform.localPosition = new Vector3(0f, DotYLift, 0f);
            return sr;
        }

        /// <summary>
        /// Place the 23 dots at formation positions for Slice 2's static
        /// scene OR seed the Slice-3 interpolation snapshots before the
        /// first FixedUpdate. Caches the archetypes for subsequent
        /// <see cref="PushTickSnapshot"/> tint resolution + sets both prev
        /// + current positions to the same formation snapshot so Update
        /// reads zero motion until the first PushTickSnapshot arrives.
        /// </summary>
        public void SetFormationPositions(
            BehaviorTreeArchetype homeArchetype,
            BehaviorTreeArchetype awayArchetype,
            Vector3Fixed ballPosition)
        {
            EnsureInitialized();
            if (homeArchetype is null) throw new ArgumentNullException(nameof(homeArchetype));
            if (awayArchetype is null) throw new ArgumentNullException(nameof(awayArchetype));

            cachedHomeArchetype = homeArchetype;
            cachedAwayArchetype = awayArchetype;

            for (int i = 0; i < PlayersPerSide; i++)
            {
                FormationSlot slot = homeArchetype.Formation[i];
                Vector3Fixed pos = slot.HomeBasePosition;
                Vector3 worldPos = pitchView.FixedToWorld(pos);
                ApplyDotPlacement(dots[i], i, worldPos, slot.Role, TeamSide.Home);
            }
            for (int i = 0; i < PlayersPerSide; i++)
            {
                FormationSlot slot = awayArchetype.Formation[i];
                Vector3Fixed pos = slot.AwayBasePosition();
                Vector3 worldPos = pitchView.FixedToWorld(pos);
                int idx = PlayersPerSide + i;
                ApplyDotPlacement(dots[idx], idx, worldPos, slot.Role, TeamSide.Away);
            }

            Vector3 ballWorld = pitchView.FixedToWorld(ballPosition);
            currentPositions[BallIndex] = ballWorld;
            prevPositions[BallIndex] = ballWorld;
            Transform ballT = dots[BallIndex].transform;
            ballT.position = new Vector3(ballWorld.x, ballT.position.y, ballWorld.z);

            lastFixedTime = Time.fixedTime;
            snapshotsInitialized = true;
        }

        /// <summary>
        /// Push a fresh per-tick canonical-state snapshot. Called once per
        /// <see cref="DotsMatchDirector"/> FixedUpdate after the runner has
        /// advanced one canonical tick. <paramref name="homeTeam"/> +
        /// <paramref name="awayTeam"/> arrays are read by reference but
        /// values are immediately copied to the interpolation snapshot —
        /// the caller is free to mutate the canonical state on the next
        /// tick without affecting interpolation.
        ///
        /// <para>
        /// Archetypes are not parameters: they're cached at
        /// <see cref="SetFormationPositions"/>-time per pr-review-toolkit
        /// type-design-analyzer Slice-3 P2 (the formation layouts are
        /// fixed at match-start for Phase-3 and don't change per tick).
        /// </para>
        /// </summary>
        public void PushTickSnapshot(
            ReadOnlySpan<PlayerState> homeTeam,
            ReadOnlySpan<PlayerState> awayTeam,
            BallState ball)
        {
            EnsureInitialized();
            if (cachedHomeArchetype is null || cachedAwayArchetype is null)
            {
                throw new InvalidOperationException(
                    $"PushTickSnapshot called before {nameof(SetFormationPositions)}; " +
                    "archetypes haven't been cached yet.");
            }
            if (homeTeam.Length != PlayersPerSide)
            {
                throw new ArgumentException(
                    $"homeTeam must have {PlayersPerSide} entries; got {homeTeam.Length}.",
                    nameof(homeTeam));
            }
            if (awayTeam.Length != PlayersPerSide)
            {
                throw new ArgumentException(
                    $"awayTeam must have {PlayersPerSide} entries; got {awayTeam.Length}.",
                    nameof(awayTeam));
            }

            // Roll the previous-frame snapshot forward; the new "current"
            // becomes the next interpolation target.
            Array.Copy(currentPositions, prevPositions, TotalDots);

            for (int i = 0; i < PlayersPerSide; i++)
            {
                currentPositions[i] = pitchView.FixedToWorld(homeTeam[i].Position);
            }
            for (int i = 0; i < PlayersPerSide; i++)
            {
                int idx = PlayersPerSide + i;
                currentPositions[idx] = pitchView.FixedToWorld(awayTeam[i].Position);
            }
            currentPositions[BallIndex] = pitchView.FixedToWorld(ball.Position);

            lastFixedTime = Time.fixedTime;
            snapshotsInitialized = true;
        }

        // Presentation-only interpolation per blueprint §B Slice 3. Reads
        // Time.time (NOT Time.fixedTime — we want elapsed wall-clock since
        // the most recent tick boundary, not "current tick time"; per
        // pr-review-toolkit silent-failure-hunter Slice-3 P3 clarification)
        // + Time.fixedDeltaTime to compute the alpha factor; output is
        // transform.position which NEVER feeds back to MatchSim. Wall-clock
        // float reads are explicitly in scope here for the same reason
        // `_Time` shader reads are NOT (in scope of the fw shader-audit
        // ban): GPU-side time can leak into RenderTexture readback in URP
        // custom passes, but C#-side wall-clock reads consumed only for
        // transform.position can't. The dots adapter's pass-activation hash
        // per `design/specs/golden-replay-corpus.md` records SEMANTIC
        // adapter activations (which shot fired, which signature played) —
        // not pixel-level interpolation state.
        private void Update()
        {
            if (!snapshotsInitialized || dots == null)
            {
                return;
            }

            float fixedDelta = Time.fixedDeltaTime;
            float alpha = fixedDelta > 0f
                ? Mathf.Clamp01((Time.time - lastFixedTime) / fixedDelta)
                : 1f;

            for (int i = 0; i < TotalDots; i++)
            {
                Vector3 prev = prevPositions[i];
                Vector3 current = currentPositions[i];
                Vector3 lerped = Vector3.Lerp(prev, current, alpha);
                Transform t = dots[i].transform;
                // Preserve the per-dot Y lift; only X/Z come from sim.
                t.position = new Vector3(lerped.x, t.position.y, lerped.z);
            }
        }

        /// <summary>
        /// Current ball world-space position — the INTERPOLATED frame-time
        /// position the dot is rendered at, per pr-review-toolkit
        /// feature-dev:code-reviewer Slice-4 P1.1 closure. The prior draft
        /// returned the post-tick snapshot from <c>currentPositions</c>;
        /// the camera consumed this in LateUpdate and the framing visibly
        /// led the rendered ball by up to one tick at zoomed framings
        /// (orthoSize=12 pass-shot-impact saw the ball off-target by
        /// ~16ms-of-travel). Reading <c>transform.position</c> reuses
        /// Update's already-interpolated value — same frame, same dot,
        /// same position the camera should track.
        /// </summary>
        public Vector3 BallWorldPosition
        {
            get
            {
                EnsureInitialized();
                return dots[BallIndex].transform.position;
            }
        }

        // Telemetry: one-shot warning the first time a non-null
        // focal-subject is passed in but TryGetFocalWorldPosition misses.
        // Per pr-review-toolkit silent-failure-hunter Slice-4 P1-B: the
        // doc-comment promised loud-fail when callers pass a non-null
        // focal but the lookup fails; the prior implementation silently
        // returned false, masking Slice-7 observer reports of
        // "camera centred on ball, not focal player."
        private bool warnedFocalSubjectStubbed;

        /// <summary>
        /// Resolve a focal-subject string (<c>"viewer.focal:home.06"</c>
        /// per blueprint Q3) to a world-space position. Phase-3 stub:
        /// always returns false; the camera falls back to ball-tracking
        /// per the SO's <c>BallFocalMidpoint</c> /
        /// <see cref="ShotTypeSO.TargetAnchor.FocalSubject"/> contracts.
        /// Phase-4+ IdentityPacket-driven roster work surfaces the
        /// jersey ↔ dot mapping needed for a real lookup. The first
        /// non-null focal-subject miss logs a one-shot warning so a
        /// Slice-7 observer reporting "camera not following the focal
        /// player" sees the stubbed-ness in the Console without bisecting
        /// through the camera + bridge stack.
        /// </summary>
        public bool TryGetFocalWorldPosition(string focalSubject, out Vector3 worldPos)
        {
            EnsureInitialized();
            if (!string.IsNullOrEmpty(focalSubject) && !warnedFocalSubjectStubbed)
            {
                Debug.LogWarning(
                    $"{nameof(DotPool)}.{nameof(TryGetFocalWorldPosition)}: focal-subject " +
                    $"'{focalSubject}' is non-null but Phase-3 stub returns false — camera " +
                    "falls back to ball-tracking. This message logs once per session; " +
                    "Phase-4+ IdentityPacket-driven roster lands the real jersey↔dot lookup.",
                    this);
                warnedFocalSubjectStubbed = true;
            }
            worldPos = default;
            return false;
        }

        private void EnsureInitialized()
        {
            if (dots == null)
            {
                throw new InvalidOperationException(
                    $"DotPool not initialized; call {nameof(Initialize)} before placing dots.");
            }
        }

        private void ApplyDotPlacement(SpriteRenderer dot, int dotIndex, Vector3 worldPos, string roleLabel, TeamSide side)
        {
            RoleFamily role = ArchetypeRoleParser.RoleFamilyForLabel(roleLabel);
            dot.color = identityTintTable.Lookup(role, side);
            float diameter = role == RoleFamily.Goalkeeper
                ? GoalkeeperDiameterMetres
                : OutfieldDiameterMetres;
            float scale = diameter * pitchView.WorldUnitsPerMeter;
            currentPositions[dotIndex] = worldPos;
            prevPositions[dotIndex] = worldPos;
            Transform t = dot.transform;
            t.position = new Vector3(worldPos.x, t.position.y, worldPos.z);
            t.localScale = new Vector3(scale, scale, scale);
        }
    }
}

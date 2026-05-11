using System;
using System.Collections.Generic;
using FinalWhistle.Viewer.Contracts;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Concrete <see cref="IShotPresentationAdapter"/> implementation for
    /// the Phase-3 dots-phase render adapter per the Slice-3/4 blueprint at
    /// <c>docs/plans/dots-adapter-blueprint.md</c> §B Slice 4. Slice-3
    /// scope was a no-op stub; Slice-4 wires <see cref="ShotCamera"/>
    /// dispatch by <see cref="ShotCategory"/>: when a
    /// <see cref="ViewerEvent"/> fires (goal, signature execution), the
    /// adapter looks up the corresponding <see cref="ShotTypeSO"/> +
    /// hands it to <see cref="ShotCamera.BeginShot"/>.
    ///
    /// <para>
    /// <strong>Adapter id</strong>: <see cref="AdapterId.Dots"/> per
    /// ADR-0008's closed code-owned registry. Used by the future
    /// adapter-keyed pass-activation hash in
    /// <c>design/specs/golden-replay-corpus.md</c> to disambiguate the
    /// dots-phase rendering trace from the conditional 3D adapter's
    /// trace on the same canonical seed.
    /// </para>
    ///
    /// <para>
    /// <strong>Initialization gate</strong>: <c>pitch != null</c> per
    /// pr-review-toolkit type-design-analyzer Slice-3 P1.
    /// </para>
    /// </summary>
    public sealed class DotsAdapterRoot : MonoBehaviour, IShotPresentationAdapter
    {
        public AdapterId AdapterId => AdapterId.Dots;

        [SerializeField] private ShotCamera shotCamera;
        [SerializeField] private SelectionRing selectionRing;
        [SerializeField] private MotionLineEmitter motionLineEmitter;
        [SerializeField] private DotPool dotPool;

        [Tooltip("ShotTypeSO assets, one per ShotCategory the dots adapter wants to render. Phase-3: tactical-wide / diagonal-attack-lane / pass-shot-impact / aftermath-freeze / player-isolation. ResolveShot throws loudly on unregistered ShotCategories — categories must be explicitly authored + wired here. Bridge-emitted categories that aren't registered are wiring bugs.")]
        [SerializeField] private ShotTypeSO[] shotCatalog;

        private PitchView pitch;
        private Dictionary<ShotCategory, ShotTypeSO> shotByCategory;
        private bool warnedUnroutedCategory;

        public void Initialize(PitchView pitchView)
        {
            if (pitchView is null)
            {
                throw new ArgumentNullException(nameof(pitchView));
            }
            pitch = pitchView;

            shotByCategory = new Dictionary<ShotCategory, ShotTypeSO>();
            if (shotCatalog != null)
            {
                foreach (ShotTypeSO so in shotCatalog)
                {
                    if (so == null) continue;
                    if (string.IsNullOrEmpty(so.ShotTypeId))
                    {
                        Debug.LogWarning(
                            $"{nameof(DotsAdapterRoot)}: ShotTypeSO asset {so.name} has empty ShotTypeId; skipping.",
                            so);
                        continue;
                    }
                    if (!ShotTypeCatalog.TryGet(so.ShotTypeId, out ShotTypeDefinition def))
                    {
                        Debug.LogError(
                            $"{nameof(DotsAdapterRoot)}: ShotTypeSO {so.name} references unknown shot id '{so.ShotTypeId}'; skipping.",
                            so);
                        continue;
                    }
                    shotByCategory[def.Category] = so;
                }
            }

            // TacticalWide is the required baseline — it's the dots-adapter
            // default framing (held when no event-driven shot is active per
            // ShotCamera.defaultShot) AND a load-bearing entry of the four
            // bridge-emittable categories. ResolveShot now throws on any
            // unregistered category (per Slice-4 P1.2 closure + Slice-5
            // round-2 P1 closure of aaf710e); a missing TacticalWide would
            // brick the adapter on the first FixedUpdate, so surface the
            // wiring gap loud at scene-load instead. (Prior comment + throw
            // message described TacticalWide as a "fallback for unregistered
            // categories" — stale wording from the silent-fallback era;
            // updated per Codex round-3 P3 against aaf710e.)
            if (!shotByCategory.ContainsKey(ShotCategory.TacticalWide))
            {
                throw new InvalidOperationException(
                    $"{nameof(DotsAdapterRoot)}: shotCatalog must include a {nameof(ShotCategory.TacticalWide)} " +
                    "entry — this is the required baseline framing (the dots-adapter default + " +
                    "the bridge category for FirstTimeDiagonalSwitch signature events). " +
                    "Wire tactical-wide.asset in the scene inspector.");
            }

            // Slice-7: initialize SelectionRing + MotionLineEmitter if
            // their inspector references are wired. Both tolerate missing
            // wiring at PresentShot time (warn-once + skip); the explicit
            // null-checks here let scenes that don't need them (e.g.
            // EditMode tests of the camera path alone) ship without
            // the new sub-components.
            if (selectionRing != null)
            {
                if (dotPool == null)
                {
                    throw new InvalidOperationException(
                        $"{nameof(DotsAdapterRoot)}.{nameof(dotPool)} reference missing; " +
                        $"required by {nameof(SelectionRing)}. Assign the DotPool in the scene inspector.");
                }
                selectionRing.Initialize(dotPool, pitch);
            }
            if (motionLineEmitter != null)
            {
                if (dotPool == null)
                {
                    throw new InvalidOperationException(
                        $"{nameof(DotsAdapterRoot)}.{nameof(dotPool)} reference missing; " +
                        $"required by {nameof(MotionLineEmitter)}. Assign the DotPool in the scene inspector.");
                }
                motionLineEmitter.Initialize(dotPool, pitch);
            }
            warnedUnroutedCategory = false;

            // Slice-7 finding #3 closure: subscribe to ShotCamera.ShotEnded
            // so SelectionRing disengages when the active event-driven shot
            // retires (canonical-tick expiry). Without this, the focal-
            // player ring stayed on-screen until the NEXT non-selection
            // shot arrived. Unsubscribe in Teardown to prevent leaks across
            // re-initializations (scene reload / EditMode test re-init).
            if (shotCamera != null)
            {
                shotCamera.ShotEnded += OnShotCameraShotEnded;
            }
        }

        private void OnShotCameraShotEnded()
        {
            if (selectionRing != null)
            {
                selectionRing.Disengage();
            }
        }

        /// <summary>
        /// Per-tick hook for the Slice-7 <see cref="MotionLineEmitter"/>
        /// fade advance. Called from <see cref="DotsMatchDirector"/>'s
        /// FixedUpdate after the canonical tick advances + before the
        /// per-event PresentShot dispatch.
        /// </summary>
        public void OnSimTick(int currentTick)
        {
            if (motionLineEmitter != null)
            {
                motionLineEmitter.Tick(currentTick);
            }
        }

        public void PresentShot(ActiveViewerEvent active)
        {
            if (pitch == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(DotsAdapterRoot)}.{nameof(Initialize)} must be called " +
                    $"before {nameof(PresentShot)}.");
            }
            if (active is null)
            {
                throw new ArgumentNullException(nameof(active));
            }
            if (shotCamera == null)
            {
                // Slice-3 had this null-check soft (warn-once at the
                // director boundary). Slice-4 elevates it: PresentShot
                // can't render without the camera, so a missing reference
                // is a configuration bug that must surface loud.
                throw new InvalidOperationException(
                    $"{nameof(DotsAdapterRoot)}.{nameof(shotCamera)} reference missing; " +
                    "assign the ShotCamera in the scene inspector.");
            }

            ShotTypeSO shot = ResolveShot(active.ShotCategory);
            shotCamera.BeginShot(shot, active);

            // Slice-7 routing: SelectionRing on focal-subject shots;
            // MotionLineEmitter on signature-execution shots (the bridge
            // marks these with a non-null FocalSubject in the Phase-3
            // schema). All routings tolerate missing inspector wiring at
            // PresentShot time: a null SelectionRing / MotionLineEmitter
            // gets a single warn-once log per Initialize lifecycle so
            // wiring gaps surface in the Console without bricking the
            // camera-side rendering.
            string focal = active.Event.FocalSubject;
            bool hasFocal = !string.IsNullOrEmpty(focal);
            ShotCategory cat = active.ShotCategory;
            bool isSelectionShot = hasFocal &&
                (cat == ShotCategory.PlayerIsolation || cat == ShotCategory.PassShotImpact);
            bool isSignatureBurstShot = hasFocal &&
                (cat == ShotCategory.PlayerIsolation
                 || cat == ShotCategory.PassShotImpact
                 || cat == ShotCategory.AftermathFreeze);

            if (selectionRing != null)
            {
                if (isSelectionShot)
                {
                    selectionRing.Engage(focal);
                }
                else
                {
                    selectionRing.Disengage();
                }
            }

            if (motionLineEmitter != null && isSignatureBurstShot)
            {
                // Pass the canonical start tick of this event. MotionLineEmitter.Tick
                // (driven from DotsMatchDirector per FixedUpdate) will advance the fade.
                motionLineEmitter.EmitBurst(active, (int)active.Event.StartTick.Value);
            }

            if (cat != ShotCategory.TacticalWide
                && cat != ShotCategory.DiagonalAttackLane
                && cat != ShotCategory.PassShotImpact
                && cat != ShotCategory.PlayerIsolation
                && cat != ShotCategory.AftermathFreeze
                && !warnedUnroutedCategory)
            {
                Debug.LogWarning(
                    $"{nameof(DotsAdapterRoot)}: ShotCategory.{cat} reached PresentShot but " +
                    "no Slice-7 routing path engages SelectionRing or MotionLineEmitter for " +
                    "it. ShotCamera still renders the framing. This message logs once per " +
                    nameof(Initialize) + " lifecycle.", this);
                warnedUnroutedCategory = true;
            }
        }

        public void Teardown()
        {
            if (shotCamera != null)
            {
                shotCamera.ShotEnded -= OnShotCameraShotEnded;
            }
            pitch = null;
            shotByCategory = null;
        }

        /// <summary>
        /// Look up the <see cref="ShotTypeSO"/> for a given
        /// <see cref="ShotCategory"/>. Throws on unregistered categories
        /// rather than silently falling back to
        /// <see cref="ShotCategory.TacticalWide"/> per Codex round-1
        /// Slice-4 finding 2: bridge-emitted categories
        /// (e.g. <see cref="ShotCategory.PlayerIsolation"/> for
        /// <c>SignatureExecuted_LowCutback</c>,
        /// <see cref="ShotCategory.AftermathFreeze"/> for
        /// <c>SignatureBreakthrough</c>) that aren't catalog-wired are
        /// authoring/wiring bugs, not fallback cases — silent fallback
        /// would drop the authored framing and ship as tactical-wide on
        /// every match that triggers an unwired category. Loud-fail at
        /// PresentShot time matches the <see cref="Initialize"/>-time
        /// TacticalWide-required gate.
        /// </summary>
        public ShotTypeSO ResolveShot(ShotCategory category)
        {
            if (shotByCategory != null && shotByCategory.TryGetValue(category, out ShotTypeSO so))
            {
                return so;
            }
            throw new InvalidOperationException(
                $"{nameof(DotsAdapterRoot)}: no ShotTypeSO registered for ShotCategory.{category}. " +
                $"The bridge or an adapter heuristic asked for this category but the scene catalog " +
                $"does not include it. Author + register a ShotTypeSO whose ShotTypeId resolves to " +
                $"{nameof(ShotCategory)}.{category} in the scene inspector " +
                $"(Phase-3 wired: TacticalWide / DiagonalAttackLane / PassShotImpact).");
        }
    }
}

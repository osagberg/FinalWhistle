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

        [Tooltip("ShotTypeSO assets, one per ShotCategory the dots adapter wants to render. Phase-3: tactical-wide / diagonal-attack-lane / pass-shot-impact. Categories not in the array fall back to TacticalWide.")]
        [SerializeField] private ShotTypeSO[] shotCatalog;

        private PitchView pitch;
        private Dictionary<ShotCategory, ShotTypeSO> shotByCategory;

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

            // Per pr-review-toolkit feature-dev:code-reviewer Slice-4 P1.2:
            // TacticalWide is the documented fallback when a shot category
            // isn't in the catalog. If it's missing the fallback throws —
            // surface that wiring gap at Initialize time, not at the first
            // FixedUpdate that produces a ResolveShot call. Loud-fail at
            // scene-load matches the discipline elsewhere in the adapter.
            if (!shotByCategory.ContainsKey(ShotCategory.TacticalWide))
            {
                throw new InvalidOperationException(
                    $"{nameof(DotsAdapterRoot)}: shotCatalog must include a {nameof(ShotCategory.TacticalWide)} " +
                    "entry — this is the documented fallback when a non-tactical-wide " +
                    "category isn't registered. Wire tactical-wide.asset in the scene inspector.");
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
        }

        public void Teardown()
        {
            pitch = null;
            shotByCategory = null;
        }

        /// <summary>
        /// Look up the <see cref="ShotTypeSO"/> for a given
        /// <see cref="ShotCategory"/>. Falls back to
        /// <see cref="ShotCategory.TacticalWide"/> when the category isn't
        /// in the catalog (Phase-3 the dots adapter only registers 3 of
        /// the 7 categories; the rest can't fire from the bridge yet so
        /// the fallback stays unreachable in normal play, but the path
        /// is here so a hand-injected debug shot doesn't NRE).
        /// </summary>
        public ShotTypeSO ResolveShot(ShotCategory category)
        {
            if (shotByCategory != null && shotByCategory.TryGetValue(category, out ShotTypeSO so))
            {
                return so;
            }
            if (shotByCategory != null && shotByCategory.TryGetValue(ShotCategory.TacticalWide, out ShotTypeSO fallback))
            {
                return fallback;
            }
            throw new InvalidOperationException(
                $"{nameof(DotsAdapterRoot)}: no ShotTypeSO registered for category {category} " +
                $"and no TacticalWide fallback. Wire shotCatalog in the scene inspector.");
        }
    }
}

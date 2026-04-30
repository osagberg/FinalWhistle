using System;
using FinalWhistle.Viewer.Contracts;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Concrete <see cref="IShotPresentationAdapter"/> implementation for
    /// the Phase-3 dots-phase render adapter per the Slice-3 blueprint at
    /// <c>docs/plans/dots-adapter-blueprint.md</c> §B Slice 3. Phase-3
    /// Slice-3 scope is intentionally minimal: the director routes
    /// newly-emitted <see cref="ViewerEvent"/>s through this adapter;
    /// <see cref="PresentShot"/> is a no-op stub. Slice 4 wires the
    /// `tactical-wide` / `diagonal-attack-lane` / `pass-shot-impact` shot
    /// camera framings; Slice 5 adds URP custom passes; Slice 6 adds the
    /// UI-Toolkit overlay; Slice 7 adds identity cues + observer rubric.
    ///
    /// <para>
    /// <strong>Adapter id</strong>: <see cref="AdapterId.Dots"/> per
    /// ADR-0008's closed code-owned registry. Used by the future
    /// adapter-keyed pass-activation hash in
    /// <c>design/specs/golden-replay-corpus.md</c> to disambiguate the
    /// dots-phase rendering trace from the conditional 3D adapter's trace
    /// on the same canonical seed.
    /// </para>
    ///
    /// <para>
    /// <strong>Initialization gate</strong>: <c>pitch != null</c> is the
    /// truthful gate (per pr-review-toolkit type-design-analyzer Slice-3
    /// P1: a parallel <c>bool initialized</c> field re-deserializes as
    /// <c>false</c> after a Unity domain reload while the cached
    /// <see cref="PitchView"/> reference goes null in step with the rest
    /// of the runtime state. Same lesson the Slice-2 closure on
    /// <see cref="DotPool"/> baked in).
    /// </para>
    /// </summary>
    public sealed class DotsAdapterRoot : MonoBehaviour, IShotPresentationAdapter
    {
        public AdapterId AdapterId => AdapterId.Dots;

        private PitchView pitch;

        public void Initialize(PitchView pitchView)
        {
            if (pitchView is null)
            {
                throw new ArgumentNullException(nameof(pitchView));
            }
            pitch = pitchView;
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
            // Slice-3 scope: no-op stub. Slice-4 dispatches to ShotCamera +
            // overlay controller per blueprint §B Slice 4. Adapter MUST NOT
            // mutate canonical sim state per ADR-0008 §"Determinism contract".
        }

        public void Teardown()
        {
            pitch = null;
        }
    }
}

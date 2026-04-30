using System;
using FinalWhistle.MatchSim.Sim;
using UnityEngine;

namespace FinalWhistle.Viewer.Core
{
    /// <summary>
    /// Pitch-geometry view per the Phase-3 dots-adapter blueprint at
    /// <c>docs/plans/dots-adapter-blueprint.md</c> §A. Carries the constants
    /// every adapter needs to map MatchSim Q32.32 canonical coordinates to
    /// Unity world-space + a single deterministic conversion helper.
    /// Adapters cache the FixedToWorld result per dot per tick; they never
    /// re-derive the geometry. Lives in <c>Viewer.Core</c> (not
    /// <c>Viewer.Contracts</c>) because the conversion target is
    /// <c>UnityEngine.Vector3</c> and Contracts is engine-free per
    /// asmdef-level <c>noEngineReferences: true</c>.
    ///
    /// <para>
    /// <strong>Phase-3 minimum:</strong> 105×68m FIFA-spec pitch
    /// (<c>design/match-engine.md</c> §pitch geometry); world origin at
    /// pitch centre by default; X+Z lie in the pitch plane; Y is altitude;
    /// 1 Unity unit = 1 metre. Phase-4+ adds margin / camera-bounds
    /// geometry when management UI surfaces alongside the match view.
    /// </para>
    ///
    /// <para>
    /// <strong>Determinism note:</strong> the Q32.32 → float conversion is
    /// presentation-only. Canonical state stays in Q32.32; PitchView is
    /// the boundary where Unity-world floats are derived. Replay
    /// reproducibility per <c>design/specs/golden-replay-corpus.md</c>
    /// pins the canonical hash before this conversion ever runs — adapter
    /// rendering divergence is captured separately via the corpus's
    /// adapter-keyed pass-activation hashes.
    /// </para>
    /// </summary>
    public sealed class PitchView
    {
        /// <summary>FIFA-spec full pitch length per design/match-engine.md.</summary>
        public const float DefaultPitchLengthMeters = 105f;

        /// <summary>FIFA-spec full pitch width per design/match-engine.md.</summary>
        public const float DefaultPitchWidthMeters = 68f;

        /// <summary>Phase-3 lock: 1 Unity unit = 1 metre. Avoids per-adapter scale drift.</summary>
        public const float DefaultMetersPerUnit = 1f;

        public float PitchLengthMeters { get; }
        public float PitchWidthMeters { get; }
        public Vector3 Origin { get; }
        public float MetersPerUnit { get; }

        public PitchView(
            float pitchLengthMeters = DefaultPitchLengthMeters,
            float pitchWidthMeters = DefaultPitchWidthMeters,
            float metersPerUnit = DefaultMetersPerUnit,
            Vector3? origin = null)
        {
            // Finite check FIRST per pr-review-toolkit:feature-dev:code-reviewer
            // 2026-04-30 P1 against the slice-1 first draft: `NaN <= 0f`
            // evaluates false in IEEE 754, so NaN/Inf would pass the
            // positivity check and only fail the finite check below — yielding
            // the wrong exception type for NaN inputs (ArgumentException via
            // the finite gate rather than ArgumentOutOfRangeException via the
            // positivity gate, despite the doc summary saying "strictly
            // positive"). Reordering also makes the failure mode loud at the
            // first detected violation rather than the second.
            Vector3 originValue = origin ?? Vector3.zero;
            if (!float.IsFinite(pitchLengthMeters) ||
                !float.IsFinite(pitchWidthMeters) ||
                !float.IsFinite(metersPerUnit))
            {
                throw new ArgumentException("Pitch dimensions must be finite (no NaN/Inf).");
            }
            if (!float.IsFinite(originValue.x) || !float.IsFinite(originValue.y) || !float.IsFinite(originValue.z))
            {
                throw new ArgumentException("Origin must be finite (no NaN/Inf).", nameof(origin));
            }
            if (pitchLengthMeters <= 0f)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(pitchLengthMeters), pitchLengthMeters,
                    "PitchLengthMeters must be strictly positive.");
            }
            if (pitchWidthMeters <= 0f)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(pitchWidthMeters), pitchWidthMeters,
                    "PitchWidthMeters must be strictly positive.");
            }
            if (metersPerUnit <= 0f)
            {
                throw new ArgumentOutOfRangeException(
                    nameof(metersPerUnit), metersPerUnit,
                    "MetersPerUnit must be strictly positive (canonical unit scaling).");
            }

            PitchLengthMeters = pitchLengthMeters;
            PitchWidthMeters = pitchWidthMeters;
            MetersPerUnit = metersPerUnit;
            Origin = originValue;
        }

        /// <summary>
        /// Convert a Q32.32 canonical position to Unity world space.
        /// Coordinate-axis mapping: X→world.x, Y→world.y (altitude),
        /// Z→world.z (pitch lateral) per design/match-engine.md.
        ///
        /// <para>
        /// Implementation note: the intermediate division uses
        /// <see cref="double"/> precision before casting to
        /// <see cref="float"/>. Direct <c>long → float</c> conversion of
        /// the Q32.32 raw value loses precision near the pitch corners
        /// (raw magnitudes in the 2.25e11 range; float mantissa is
        /// 23-bit). The double-precision intermediate keeps sub-mm
        /// accuracy throughout the pitch, well within the dots-adapter
        /// rendering tolerance (1mm = 0.001f).
        /// </para>
        /// </summary>
        public Vector3 FixedToWorld(Vector3Fixed position)
        {
            // MetersPerUnit multiplication stays in double per
            // pr-review-toolkit:feature-dev:code-reviewer 2026-04-30 P1
            // against the slice-1 first draft: a float-side multiply after
            // the float cast discards the double-precision intermediate's
            // benefit when MetersPerUnit != 1f (the binding precision
            // constraint becomes the float-side product, not the double-side
            // divide). Keeping divide + scale together in double precision
            // preserves the sub-mm accuracy claim across the full pipeline,
            // not just the divide.
            const double oneRaw = (double)Fixed.OneRaw;
            double scale = MetersPerUnit;
            float x = (float)(position.X.RawValue / oneRaw * scale);
            float y = (float)(position.Y.RawValue / oneRaw * scale);
            float z = (float)(position.Z.RawValue / oneRaw * scale);
            return new Vector3(Origin.x + x, Origin.y + y, Origin.z + z);
        }
    }
}

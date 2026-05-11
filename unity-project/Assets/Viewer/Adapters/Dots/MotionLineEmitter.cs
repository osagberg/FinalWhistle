using System;
using FinalWhistle.Viewer.Contracts;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Radial motion-line burst per the Phase-3 dots-adapter blueprint
    /// §B Slice 7. Emits <see cref="LinesPerBurst"/> sprite instances
    /// arranged radially around a focal player when a signature-execution
    /// <see cref="ActiveViewerEvent"/> arrives. Each instance fades to
    /// alpha 0 over <see cref="FadeTicks"/> ticks via
    /// <see cref="SpriteRenderer.color"/> alpha Lerp.
    ///
    /// <para>
    /// <strong>Sprite-emitter pattern (NOT URP ScriptableRendererFeature)</strong>
    /// per blueprint §B Slice 7 + Decision 1 (Cinemachine-free / hand-rolled).
    /// A RendererFeature would require shader authoring (forbidden by the
    /// <c>fw shader-audit</c> ban on <c>_Time</c>); sprite-emitter avoids
    /// the issue entirely — fades are C#-side <c>Color.a</c> writes driven
    /// by canonical tick deltas, not GPU time.
    /// </para>
    ///
    /// <para>
    /// <strong>Pool of <see cref="PoolSize"/> sprite instances.</strong>
    /// Lines are reused across bursts; warm-up at
    /// <see cref="Initialize"/> means zero per-burst allocations after
    /// the first frame.
    /// </para>
    ///
    /// <para>
    /// <strong>Reduce-motion:</strong> when
    /// <see cref="ViewerEvent.ReduceMotionApplied"/> is true, emission is
    /// skipped per ADR-0008. The bridge resolves the variant once;
    /// adapters never re-substitute.
    /// </para>
    /// </summary>
    public sealed class MotionLineEmitter : MonoBehaviour
    {
        public const int LinesPerBurst = 10;
        public const int PoolSize = 24;        // 2 bursts worth of headroom
        public const int FadeTicks = 18;       // fades within 20 ticks per acceptance
        public const float TicksPerSecond = 60f;

        // Radial offset from focal centre to each line's start point, in
        // metres. Keeps lines outside the dot itself + reads as energy
        // radiating from the player. 2026-05-11 (C5 polish): bumped from
        // 1.0 → 2.5 so lines clear the dot at tactical-wide zoom.
        private const float RadialOffsetMetres = 2.5f;

        // Visual scale of each line (sprite is 2×16 px @ PPU=16 → 0.125×1.0
        // world-units at WorldUnitsPerMeter=1). 2026-05-11 (C5 polish):
        // Codex L2 feedback flagged the original 1.5m × 0.125m sprites as
        // "may read too small at orthoSize=38". Bumped to 5m × 0.4m so
        // lines are ~7% of view height / clearly legible without being
        // absurd. FadeTicks=18 (~0.3s @60Hz) unchanged — quick-fade
        // discipline preserved.
        private const float LineLengthMultiplier = 5.0f;
        private const float LineWidthMultiplier = 3.2f;

        [SerializeField] private Sprite lineSprite;
        [SerializeField] private Color lineColor = Color.white;

        private DotPool dotPool;
        private PitchView pitchView;
        private SpriteRenderer[] pool;
        private int[] burstStartTicks;          // tick at which each pool entry was emitted (long.MinValue = idle)
        private int poolWriteCursor;            // round-robin write pointer

        public void Initialize(DotPool dotPoolArg, PitchView pitchViewArg)
        {
            if (dotPoolArg == null) throw new ArgumentNullException(nameof(dotPoolArg));
            if (pitchViewArg is null) throw new ArgumentNullException(nameof(pitchViewArg));
            if (lineSprite == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(MotionLineEmitter)}.{nameof(lineSprite)} reference missing; " +
                    "assign motion_line.png in the scene inspector.");
            }

            dotPool = dotPoolArg;
            pitchView = pitchViewArg;

            if (pool == null)
            {
                pool = new SpriteRenderer[PoolSize];
                burstStartTicks = new int[PoolSize];
                float widthUnits = (2f / 16f) * LineWidthMultiplier * pitchView.WorldUnitsPerMeter;
                float lengthUnits = LineLengthMultiplier * pitchView.WorldUnitsPerMeter;
                for (int i = 0; i < PoolSize; i++)
                {
                    GameObject lineObj = new($"MotionLine_{i}");
                    lineObj.transform.SetParent(transform, worldPositionStays: false);
                    lineObj.transform.localRotation = Quaternion.Euler(90f, 0f, 0f);
                    lineObj.transform.localScale = new Vector3(widthUnits, lengthUnits, 1f);
                    var sr = lineObj.AddComponent<SpriteRenderer>();
                    sr.sprite = lineSprite;
                    sr.color = new Color(lineColor.r, lineColor.g, lineColor.b, 0f);
                    sr.enabled = false;
                    sr.sortingOrder = -2; // behind dots + behind ring
                    pool[i] = sr;
                    burstStartTicks[i] = int.MinValue;
                }
                poolWriteCursor = 0;
            }
            else
            {
                // Re-initialize: silence + reset every entry.
                for (int i = 0; i < PoolSize; i++)
                {
                    if (pool[i] != null)
                    {
                        pool[i].enabled = false;
                        burstStartTicks[i] = int.MinValue;
                    }
                }
                poolWriteCursor = 0;
            }
        }

        /// <summary>
        /// Emit a radial burst at the resolved focal subject's world position,
        /// timed from <paramref name="currentTick"/>. No-op when
        /// <see cref="ActiveViewerEvent.Event"/>.<see cref="ViewerEvent.ReduceMotionApplied"/>
        /// is true OR the focal subject does not resolve via
        /// <see cref="DotPool.IndexForFocalSubject"/>.
        /// </summary>
        public void EmitBurst(ActiveViewerEvent active, int currentTick)
        {
            EnsureInitialized();
            if (active is null) throw new ArgumentNullException(nameof(active));
            if (active.Event.ReduceMotionApplied)
            {
                return;
            }
            int focalIdx = dotPool.IndexForFocalSubject(active.Event.FocalSubject);
            if (focalIdx < 0)
            {
                return;
            }

            Transform focalT = dotPool.transform.GetChild(focalIdx);
            Vector3 focalPos = focalT.position;
            float radialMetres = RadialOffsetMetres * pitchView.WorldUnitsPerMeter;

            for (int i = 0; i < LinesPerBurst; i++)
            {
                int slot = poolWriteCursor;
                poolWriteCursor = (poolWriteCursor + 1) % PoolSize;
                SpriteRenderer line = pool[slot];
                if (line == null) continue;

                // Even radial distribution. The 360° / LinesPerBurst step
                // gives a clean radial fan; future rotation jitter
                // (Phase-4+) can read MatchSim Seed.Derive(seed, tick, idx)
                // to stay deterministic.
                float angleDeg = (360f / LinesPerBurst) * i;
                float angleRad = angleDeg * Mathf.Deg2Rad;
                float dx = Mathf.Cos(angleRad);
                float dz = Mathf.Sin(angleRad);
                Vector3 origin = new(focalPos.x + dx * radialMetres,
                                     focalPos.y + 0.07f,
                                     focalPos.z + dz * radialMetres);
                line.transform.position = origin;
                // Rotate line so its length axis points outward from focal.
                // Sprite is rotated Euler(90, 0, 0) on X (flat in pitch
                // plane); add Y-axis rotation to orient the length axis.
                line.transform.localRotation = Quaternion.Euler(90f, -angleDeg, 0f);
                line.color = new Color(lineColor.r, lineColor.g, lineColor.b, 1f);
                line.enabled = true;
                burstStartTicks[slot] = currentTick;
            }
        }

        /// <summary>
        /// Advance the per-line fade given the current canonical tick.
        /// Caller invokes this per FixedUpdate from <see cref="DotsMatchDirector"/>.
        /// Lines whose fade completes are disabled; their slots become
        /// reusable.
        /// </summary>
        public void Tick(int currentTick)
        {
            if (pool == null) return;
            for (int i = 0; i < PoolSize; i++)
            {
                SpriteRenderer line = pool[i];
                if (line == null || !line.enabled) continue;
                int startTick = burstStartTicks[i];
                int elapsed = currentTick - startTick;
                if (elapsed < 0)
                {
                    continue;
                }
                if (elapsed >= FadeTicks)
                {
                    line.enabled = false;
                    burstStartTicks[i] = int.MinValue;
                    continue;
                }
                float alpha = 1f - (elapsed / (float)FadeTicks);
                Color c = line.color;
                line.color = new Color(c.r, c.g, c.b, alpha);
            }
        }

        /// <summary>Test/diagnostic: count of pool entries currently emitting.</summary>
        internal int ActiveLineCount
        {
            get
            {
                if (pool == null) return 0;
                int n = 0;
                for (int i = 0; i < PoolSize; i++)
                {
                    if (pool[i] != null && pool[i].enabled) n++;
                }
                return n;
            }
        }

        private void EnsureInitialized()
        {
            if (pool == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(MotionLineEmitter)}.{nameof(Initialize)} must be called before " +
                    $"{nameof(EmitBurst)}.");
            }
        }
    }
}

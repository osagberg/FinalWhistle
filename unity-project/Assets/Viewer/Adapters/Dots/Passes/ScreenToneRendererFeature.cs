using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.RenderGraphModule;
using UnityEngine.Rendering.RenderGraphModule.Util;
using UnityEngine.Rendering.Universal;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// URP <see cref="ScriptableRendererFeature"/> that injects the
    /// diagonal screen-tone overlay per the Phase-3 dots-adapter
    /// blueprint §B Slice 5. Strength is gated entirely by the
    /// <c>_FW_ScreenToneStrength</c> global shader uniform driven by
    /// <see cref="DotsMatchDirector"/> — when strength is 0 the shader
    /// short-circuits and the pass is a near-zero visual cost.
    ///
    /// <para>
    /// <strong>Pass ordering</strong>: injected at
    /// <see cref="RenderPassEvent.AfterRenderingPostProcessing"/> so
    /// the screen-tone overlay sits on top of any URP post-processing
    /// (bloom / tonemap / color grading). The
    /// <c>ImpactFrameRendererFeature</c> uses the same event with a
    /// +1 nudge so the flash renders on top of the tone.
    /// </para>
    ///
    /// <para>
    /// <strong>Determinism</strong>: the shader is `_Time`-clean per
    /// <c>.claude/rules/Scripts/Viewer/RULES.md</c>. All time-dependent
    /// inputs are explicit uniforms set by the director from canonical
    /// <see cref="MatchSim.Sim.Tick"/> values — the
    /// match-replay-corpus pass-activation hashes stay reproducible.
    /// </para>
    ///
    /// <para>
    /// <strong>RenderGraph API</strong>: written for the URP 17.x
    /// RenderGraph path (Unity 6 default). The
    /// <see cref="RenderGraphUtils.BlitMaterialParameters"/> +
    /// <see cref="RenderGraph.AddBlitPass(RenderGraphUtils.BlitMaterialParameters,string,string,int)"/>
    /// pattern handles the source/destination ping-pong; we don't
    /// allocate temp RTs ourselves.
    /// </para>
    /// </summary>
    [DisallowMultipleRendererFeature("FinalWhistle Screen-Tone")]
    public sealed class ScreenToneRendererFeature : ScriptableRendererFeature
    {
        [Tooltip("Loud-fail if missing — assign FinalWhistle/Viewer/Dots/ScreenTone in the inspector.")]
        [SerializeField] private Shader screenToneShader;

        private Material material;
        private ScreenTonePass pass;
        private bool warnedAddRenderPassesNoMaterial;

        // Cached PropertyToID for the idle-frame skip check in
        // AddRenderPasses. Looked up once at Create + reused per frame.
        private static readonly int strengthId =
            Shader.PropertyToID(AnimePresentationUniforms.ScreenToneStrengthName);

        public override void Create()
        {
            // Defer material creation when the shader reference is
            // missing. A hard throw here would brick the renderer asset
            // until the user reassigns; warn-once + retry on next
            // Create() (which fires on inspector reassignment) keeps
            // the recovery path quick.
            if (screenToneShader == null)
            {
                Debug.LogWarning(
                    $"{nameof(ScreenToneRendererFeature)}: shader reference missing; " +
                    $"assign {nameof(screenToneShader)} in the renderer-feature inspector.");
                return;
            }
            material = CoreUtils.CreateEngineMaterial(screenToneShader);
            pass = new ScreenTonePass(material)
            {
                renderPassEvent = RenderPassEvent.AfterRenderingPostProcessing,
            };
            warnedAddRenderPassesNoMaterial = false;
        }

        public override void AddRenderPasses(ScriptableRenderer renderer, ref RenderingData renderingData)
        {
            if (pass == null || material == null)
            {
                // Loud-once on the recurring no-op so a build with a
                // missing shader reference doesn't ship zero overlay
                // output silently (per pr-review-toolkit silent-failure-
                // hunter Slice-5 P2 closure: prior shape warned once at
                // Create then early-returned forever with no further log).
                if (!warnedAddRenderPassesNoMaterial)
                {
                    Debug.LogError(
                        $"{nameof(ScreenToneRendererFeature)}: AddRenderPasses skipped — " +
                        "material/pass not initialised (missing shader reference). " +
                        "Re-assign the shader in the renderer-feature inspector to recover.");
                    warnedAddRenderPassesNoMaterial = true;
                }
                return;
            }
            // Game cameras only — preview/scene cameras render through
            // their own paths and the overlay is irrelevant there.
            if (renderingData.cameraData.cameraType != CameraType.Game)
            {
                return;
            }
            // Idle-frame fast path per Codex round-1 P2 closure of 2b3460e:
            // when the director's screen-tone state has fully retired,
            // skip enqueueing the pass entirely. The shader's `<= 0` early-
            // out still bailed on no-op rendering but the BLIT itself ran
            // (sample + write a fullscreen target every frame on every
            // game camera). Reading the global is a single hash-lookup +
            // float-load — orders of magnitude cheaper than a fullscreen
            // sample-write.
            if (Shader.GetGlobalFloat(strengthId) <= 0f)
            {
                return;
            }
            renderer.EnqueuePass(pass);
        }

        protected override void Dispose(bool disposing)
        {
            if (material != null)
            {
                CoreUtils.Destroy(material);
                material = null;
            }
            pass = null;
        }

        private sealed class ScreenTonePass : ScriptableRenderPass
        {
            private const string PassName = "FW.ScreenTone";
            private readonly Material material;

            public ScreenTonePass(Material material)
            {
                this.material = material;
            }

            public override void RecordRenderGraph(RenderGraph renderGraph, ContextContainer frameData)
            {
                if (material == null)
                {
                    return;
                }
                UniversalResourceData resourceData = frameData.Get<UniversalResourceData>();
                if (resourceData.isActiveTargetBackBuffer)
                {
                    return;
                }

                TextureHandle source = resourceData.activeColorTexture;
                // Allocate a sibling target with matching descriptor;
                // BlitMaterialParameters handles the source→destination
                // sampling without us having to ping-pong manually.
                var sourceDesc = renderGraph.GetTextureDesc(source);
                sourceDesc.name = "_FW_ScreenToneTarget";
                sourceDesc.clearBuffer = false;
                TextureHandle destination = renderGraph.CreateTexture(sourceDesc);

                RenderGraphUtils.BlitMaterialParameters blitParams =
                    new(source, destination, material, shaderPass: 0);
                renderGraph.AddBlitPass(blitParams, passName: PassName);

                // Subsequent passes (e.g. ImpactFrame) read from this
                // overlay's destination as their new source.
                resourceData.cameraColor = destination;
            }
        }
    }
}

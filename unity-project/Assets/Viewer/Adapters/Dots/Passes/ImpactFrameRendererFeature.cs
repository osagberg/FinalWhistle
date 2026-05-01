using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.RenderGraphModule;
using UnityEngine.Rendering.RenderGraphModule.Util;
using UnityEngine.Rendering.Universal;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// URP <see cref="ScriptableRendererFeature"/> that injects the
    /// impact-frame white-flash overlay per the Phase-3 dots-adapter
    /// blueprint §B Slice 5. Intensity is gated entirely by the
    /// <c>_FW_FlashIntensity</c> global shader uniform driven by
    /// <see cref="DotsMatchDirector"/>. Decay is computed in C#
    /// against the canonical <see cref="MatchSim.Sim.Tick"/> stream so
    /// frame-time variance never influences flash intensity (see the
    /// <see cref="AnimePresentationUniforms"/> helper).
    ///
    /// <para>
    /// <strong>Pass ordering</strong>: injected at
    /// <see cref="RenderPassEvent.AfterRenderingPostProcessing"/> with
    /// a +1 nudge so it renders on top of the screen-tone pass — the
    /// flash always sits over any active screen-tone overlay.
    /// </para>
    ///
    /// <para>
    /// <strong>Cross-pass backbuffer assumption</strong> (per
    /// pr-review-toolkit feature-dev:code-reviewer Slice-5 P1
    /// closure): both this feature and <c>ScreenToneRendererFeature</c>
    /// independently early-return on
    /// <c>resourceData.isActiveTargetBackBuffer</c>. The two passes
    /// must EITHER both bail OR both run for the ordering contract
    /// ("flash sits on tone") to hold. Phase-3 ships PC-only with no
    /// XR / split-screen / secondary-camera scenarios where the two
    /// passes would diverge, so the contract holds by configuration.
    /// Slice-7+ that introduces additional camera setups must verify
    /// the assumption (or move to a single shared feature with two
    /// internal passes).
    /// </para>
    ///
    /// <para>
    /// <strong>Reduce-motion</strong>: the director simply does not
    /// raise <c>_FW_FlashIntensity</c> above 0 when
    /// <c>ViewerEvent.ReduceMotionApplied</c> is true on the
    /// triggering event, so the shader's early-out short-circuits and
    /// no flash blits. The renderer feature is opaque to the
    /// reduce-motion decision per ADR-0008's "adapter-aware reduce
    /// motion" semantics — only the director sees the flag.
    /// </para>
    /// </summary>
    [DisallowMultipleRendererFeature("FinalWhistle Impact-Frame")]
    public sealed class ImpactFrameRendererFeature : ScriptableRendererFeature
    {
        [Tooltip("Loud-fail if missing — assign FinalWhistle/Viewer/Dots/ImpactFrame in the inspector.")]
        [SerializeField] private Shader impactFrameShader;

        private Material material;
        private ImpactFramePass pass;
        private bool warnedAddRenderPassesNoMaterial;

        public override void Create()
        {
            if (impactFrameShader == null)
            {
                Debug.LogWarning(
                    $"{nameof(ImpactFrameRendererFeature)}: shader reference missing; " +
                    $"assign {nameof(impactFrameShader)} in the renderer-feature inspector.");
                return;
            }
            material = CoreUtils.CreateEngineMaterial(impactFrameShader);
            pass = new ImpactFramePass(material)
            {
                // +1 ordering nudge: flash renders AFTER screen-tone in
                // the same render-pass-event window, so the flash sits
                // on top of any active tone.
                renderPassEvent = RenderPassEvent.AfterRenderingPostProcessing + 1,
            };
            warnedAddRenderPassesNoMaterial = false;
        }

        public override void AddRenderPasses(ScriptableRenderer renderer, ref RenderingData renderingData)
        {
            if (pass == null || material == null)
            {
                if (!warnedAddRenderPassesNoMaterial)
                {
                    Debug.LogError(
                        $"{nameof(ImpactFrameRendererFeature)}: AddRenderPasses skipped — " +
                        "material/pass not initialised (missing shader reference). " +
                        "Re-assign the shader in the renderer-feature inspector to recover.");
                    warnedAddRenderPassesNoMaterial = true;
                }
                return;
            }
            if (renderingData.cameraData.cameraType != CameraType.Game)
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

        private sealed class ImpactFramePass : ScriptableRenderPass
        {
            private const string PassName = "FW.ImpactFrame";
            private readonly Material material;

            public ImpactFramePass(Material material)
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
                var sourceDesc = renderGraph.GetTextureDesc(source);
                sourceDesc.name = "_FW_ImpactFrameTarget";
                sourceDesc.clearBuffer = false;
                TextureHandle destination = renderGraph.CreateTexture(sourceDesc);

                RenderGraphUtils.BlitMaterialParameters blitParams =
                    new(source, destination, material, shaderPass: 0);
                renderGraph.AddBlitPass(blitParams, passName: PassName);

                resourceData.cameraColor = destination;
            }
        }
    }
}

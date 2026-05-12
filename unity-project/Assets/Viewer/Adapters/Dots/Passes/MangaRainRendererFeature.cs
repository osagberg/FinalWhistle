using UnityEngine;
using UnityEngine.Rendering;
using UnityEngine.Rendering.RenderGraphModule;
using UnityEngine.Rendering.RenderGraphModule.Util;
using UnityEngine.Rendering.Universal;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    public sealed class MangaRainRendererFeature : ScriptableRendererFeature
    {
        [SerializeField] private Shader rainShader;
        private Material material;
        private MangaRainPass pass;

        public override void Create()
        {
            if (rainShader == null) return;
            material = CoreUtils.CreateEngineMaterial(rainShader);
            pass = new MangaRainPass(material)
            {
                renderPassEvent = RenderPassEvent.AfterRenderingPostProcessing + 2
            };
        }

        public override void AddRenderPasses(ScriptableRenderer renderer, ref RenderingData renderingData)
        {
            if (pass != null && material != null && renderingData.cameraData.cameraType == CameraType.Game)
            {
                renderer.EnqueuePass(pass);
            }
        }

        protected override void Dispose(bool disposing)
        {
            CoreUtils.Destroy(material);
        }

        private sealed class MangaRainPass : ScriptableRenderPass
        {
            private readonly Material material;

            public MangaRainPass(Material material)
            {
                this.material = material;
            }

            public override void RecordRenderGraph(RenderGraph renderGraph, ContextContainer frameData)
            {
                if (material == null) return;
                UniversalResourceData resourceData = frameData.Get<UniversalResourceData>();
                if (resourceData.isActiveTargetBackBuffer) return;

                TextureHandle source = resourceData.activeColorTexture;
                var sourceDesc = renderGraph.GetTextureDesc(source);
                sourceDesc.name = "_FW_MangaRainTarget";
                TextureHandle destination = renderGraph.CreateTexture(sourceDesc);

                // Set elapsed ticks for movement
                int elapsedTicks = Shader.GetGlobalInt(AnimePresentationUniforms.ElapsedTicksName);
                material.SetFloat("_FW_ElapsedTicks", elapsedTicks);

                RenderGraphUtils.BlitMaterialParameters blitParams = new(source, destination, material, 0);
                renderGraph.AddBlitPass(blitParams, "FW.MangaRain");
                resourceData.cameraColor = destination;
            }
        }
    }
}

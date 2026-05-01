// Phase-3 dots-adapter Slice 5 — impact-frame white-flash overlay.
// Anime-presentation-budget surface #1 (impact frames) per
// design/anime-presentation-budget.md. Driven by the
// ImpactFrameRendererFeature URP custom pass; injected after
// post-processing AND after the screen-tone pass so the flash sits
// on top of any active tone. Strength is gated by the
// `_FW_FlashIntensity` global float uniform (0 = invisible,
// 1 = full white-flash), set per FixedUpdate by
// `DotsMatchDirector` (decay computed in C#, not in shader).
//
// Determinism contract per .claude/rules/Scripts/Viewer/RULES.md:
// the frame-time shader-globals are banned in viewer-adapter shaders
// (the rules doc enumerates them; fw shader-audit enforces). All
// time-dependent inputs come through explicit uniforms set by the
// adapter from canonical elapsed-tick state. Decay is computed in
// DotsMatchDirector.cs against the canonical Tick stream so
// frame-time variation never influences flash intensity.

Shader "FinalWhistle/Viewer/Dots/ImpactFrame"
{
    Properties { }
    SubShader
    {
        Tags { "RenderType" = "Opaque" "RenderPipeline" = "UniversalPipeline" }
        ZWrite Off Cull Off ZTest Always

        Pass
        {
            Name "ImpactFrameFlash"
            HLSLPROGRAM

            #pragma vertex Vert
            #pragma fragment Frag

            #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"

            // sampler_LinearClamp is pre-declared by URP's Core.hlsl.
            // _BlitScaleBias.xy=scale, .zw=bias — applied to UVs so the
            // shader is correct under dynamic-resolution / non-1.0
            // render-scale (per engine-programmer Slice-5 P1 closure).
            TEXTURE2D(_BlitTexture);
            float4 _BlitScaleBias;

            float _FW_FlashIntensity;

            // White-flash colour. Per anime-presentation-budget.md the
            // base palette is bright white; tone modulation per stakes/
            // memory tier is Phase-4+ tuning (would lerp this colour
            // toward warm/cool based on stakes-band).
            static const float3 FlashColour = float3(1.0, 1.0, 1.0);

            struct Attributes
            {
                uint vertexID : SV_VertexID;
            };

            struct Varyings
            {
                float4 positionCS : SV_POSITION;
                float2 texcoord   : TEXCOORD0;
            };

            Varyings Vert(Attributes input)
            {
                Varyings output;
                float2 uv = float2((input.vertexID << 1) & 2, input.vertexID & 2);
                output.positionCS = float4(uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
                output.texcoord = uv * _BlitScaleBias.xy + _BlitScaleBias.zw;
                return output;
            }

            half4 Frag(Varyings input) : SV_Target
            {
                half4 scene = SAMPLE_TEXTURE2D_LOD(_BlitTexture, sampler_LinearClamp, input.texcoord, 0);

                if (_FW_FlashIntensity <= 0.0)
                {
                    return scene;
                }

                float a = saturate(_FW_FlashIntensity);
                half3 flashed = lerp(scene.rgb, FlashColour, a);
                return half4(flashed, scene.a);
            }
            ENDHLSL
        }
    }
    Fallback Off
}

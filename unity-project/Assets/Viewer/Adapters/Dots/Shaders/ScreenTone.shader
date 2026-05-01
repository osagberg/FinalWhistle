// Phase-3 dots-adapter Slice 5 — diagonal screen-tone overlay.
// Anime-presentation-budget surface #2 (manga screen-tone variant) per
// design/anime-presentation-budget.md. Driven by the
// ScreenToneRendererFeature URP custom pass; injected after
// post-processing. Strength is gated by the `_FW_ScreenToneStrength`
// global float uniform (0 = invisible, 1 = full overlay), set per
// FixedUpdate by `DotsMatchDirector`.
//
// Determinism contract per .claude/rules/Scripts/Viewer/RULES.md:
// the frame-time shader-globals are banned in viewer-adapter shaders
// (the rules doc enumerates them; fw shader-audit enforces). The
// match-replay corpus pins adapter-keyed pass-activation hashes per
// seed; frame-time intrinsics break replay reproducibility. All
// time-dependent inputs come through explicit uniforms set by the
// adapter from canonical elapsed-tick state (FW_ElapsedTicks).
//
// Pattern: a 45°-rotated dot grid in screen space. Density modulated
// by `_FW_ScreenToneStrength`; tint blended over the existing scene.

Shader "FinalWhistle/Viewer/Dots/ScreenTone"
{
    Properties { }
    SubShader
    {
        Tags { "RenderType" = "Opaque" "RenderPipeline" = "UniversalPipeline" }
        ZWrite Off Cull Off ZTest Always

        Pass
        {
            Name "ScreenToneOverlay"
            HLSLPROGRAM

            #pragma vertex Vert
            #pragma fragment Frag

            #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"

            // Fullscreen-triangle source supplied by URP RenderGraph
            // BlitMaterialParameters via the canonical _BlitTexture +
            // _BlitTexture_TexelSize globals. sampler_LinearClamp is
            // pre-declared by URP's Core.hlsl. _BlitScaleBias.xy=scale,
            // .zw=bias — applied to UVs so the shader is correct under
            // dynamic-resolution / non-1.0 render-scale (per
            // engine-programmer Slice-5 P1 closure).
            TEXTURE2D(_BlitTexture);
            float4 _BlitScaleBias;

            // Globals set by DotsMatchDirector via Shader.SetGlobalFloat /
            // SetGlobalInt. FW_ prefix avoids collision with any third-party
            // global name.
            float _FW_ScreenToneStrength;
            int   _FW_ElapsedTicks;

            // Tone parameters (constants — not author-tunable at Phase-3;
            // SO-based tuning is a Phase-4+ inspector-driven affordance).
            // Dot spacing in screen pixels and dot radius (in [0, 0.5]).
            static const float DotSpacingPx = 14.0;
            static const float DotRadiusFactor = 0.32;
            static const float3 ToneColour = float3(0.06, 0.06, 0.08);

            struct Attributes
            {
                uint vertexID : SV_VertexID;
            };

            struct Varyings
            {
                float4 positionCS : SV_POSITION;
                float2 texcoord   : TEXCOORD0;
            };

            // Standard URP fullscreen triangle from vertex id 0..2.
            // No mesh required — RenderGraph BlitMaterialParameters
            // dispatches the triangle with three vertices.
            Varyings Vert(Attributes input)
            {
                Varyings output;
                float2 uv = float2((input.vertexID << 1) & 2, input.vertexID & 2);
                output.positionCS = float4(uv * float2(2.0, -2.0) + float2(-1.0, 1.0), 0.0, 1.0);
                // Apply _BlitScaleBias so the source-texture UV stays correct
                // when the source is a sub-rect of a larger RT (DRS / render-
                // scale != 1 / XR — non-VR project but the discipline costs
                // nothing).
                output.texcoord = uv * _BlitScaleBias.xy + _BlitScaleBias.zw;
                return output;
            }

            half4 Frag(Varyings input) : SV_Target
            {
                half4 scene = SAMPLE_TEXTURE2D_LOD(_BlitTexture, sampler_LinearClamp, input.texcoord, 0);

                if (_FW_ScreenToneStrength <= 0.0)
                {
                    return scene;
                }

                // Destination-space pixel coordinates from SV_POSITION
                // (in viewport pixels). Using positionCS rather than
                // texcoord keeps the dot-grid metric stable under DRS:
                // the dots are sized in DESTINATION pixels regardless of
                // source-RT scaling. positionCS.xy is half-pixel-centred
                // already so no offset adjustment needed.
                float2 px = input.positionCS.xy;

                // 45° rotation so the grid reads as a manga screen-tone
                // diagonal pattern rather than an axis-aligned dot field.
                const float c = 0.70710678; // cos(45°) = sin(45°)
                float2 rotated = float2(px.x * c + px.y * c, -px.x * c + px.y * c);

                // Tile the rotated screen into DotSpacingPx-sized cells; the
                // dot is a soft circle at the cell centre.
                float2 cell = frac(rotated / DotSpacingPx) - 0.5;
                float r = length(cell);
                // smoothstep gives a soft edge so the dots don't shimmer
                // under upscale/downscale.
                float dotMask = 1.0 - smoothstep(DotRadiusFactor - 0.05,
                                                  DotRadiusFactor + 0.05, r);

                // Strength gates BOTH the tone alpha AND the dot density,
                // so a Lerp from 0 → 1 fades the entire overlay in cleanly.
                float alpha = saturate(dotMask * _FW_ScreenToneStrength);
                half3 toned = lerp(scene.rgb, ToneColour, alpha);
                return half4(toned, scene.a);
            }
            ENDHLSL
        }
    }
    Fallback Off
}

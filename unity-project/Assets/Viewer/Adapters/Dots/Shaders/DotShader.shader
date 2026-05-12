Shader "FinalWhistle/Viewer/Dots/DotShader"
{
    Properties
    {
        [MainTexture] _BaseMap("Base Map", 2D) = "white" {}
        _BaseColor("Base Color", Color) = (1, 1, 1, 1)
        _RimColor("Rim Color", Color) = (1, 1, 1, 1)
        _RimPower("Rim Power", Range(0.1, 8.0)) = 3.0
        _RimIntensity("Rim Intensity", Range(0.0, 2.0)) = 1.0
    }
    SubShader
    {
        Tags { "RenderType" = "Transparent" "Queue" = "Transparent" "RenderPipeline" = "UniversalPipeline" }
        LOD 100
        ZWrite Off
        Blend SrcAlpha OneMinusSrcAlpha

        Pass
        {
            Name "DotPass"
            HLSLPROGRAM
            #pragma vertex Vert
            #pragma fragment Frag

            #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"

            struct Attributes
            {
                float4 positionOS   : POSITION;
                float2 uv           : TEXCOORD0;
                float4 color        : COLOR;
                float3 normalOS     : NORMAL;
            };

            struct Varyings
            {
                float4 positionCS   : SV_POSITION;
                float2 uv           : TEXCOORD0;
                float4 color        : COLOR;
                float3 positionWS   : TEXCOORD1;
            };

            TEXTURE2D(_BaseMap);
            SAMPLER(sampler_BaseMap);

            CBUFFER_START(UnityPerMaterial)
            float4 _BaseColor;
            float4 _RimColor;
            float _RimPower;
            float _RimIntensity;
            CBUFFER_END

            Varyings Vert(Attributes input)
            {
                Varyings output;
                output.positionCS = TransformObjectToHClip(input.positionOS.xyz);
                output.uv = input.uv;
                output.color = input.color;
                output.positionWS = TransformObjectToWorld(input.positionOS.xyz);
                return output;
            }

            half4 Frag(Varyings input) : SV_Target
            {
                half4 tex = SAMPLE_TEXTURE2D(_BaseMap, sampler_BaseMap, input.uv);
                half4 finalColor = tex * input.color * _BaseColor;

                // FM Style: Flat and clean. No rim light.
                
                return finalColor;
            }
            ENDHLSL
        }
    }
    Fallback "Universal Render Pipeline/2D/Sprite-Unlit"
}

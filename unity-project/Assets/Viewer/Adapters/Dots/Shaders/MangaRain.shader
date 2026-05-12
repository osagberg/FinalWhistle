Shader "FinalWhistle/Viewer/Dots/MangaRain"
{
    Properties
    {
        _RainColor ("Rain Color", Color) = (1, 1, 1, 0.4)
        _RainDensity ("Rain Density", Range(1, 100)) = 20
        _RainSpeed ("Rain Speed", Float) = 2.0
    }
    SubShader
    {
        Tags { "RenderType" = "Transparent" "Queue" = "Transparent" }
        LOD 100
        ZWrite Off
        Blend SrcAlpha OneMinusSrcAlpha

        Pass
        {
            HLSLPROGRAM
            #pragma vertex Vert
            #pragma fragment Frag

            #include "Packages/com.unity.render-pipelines.universal/ShaderLibrary/Core.hlsl"

            struct Attributes
            {
                float4 positionOS   : POSITION;
                float2 uv           : TEXCOORD0;
            };

            struct Varyings
            {
                float4 positionCS   : SV_POSITION;
                float2 uv           : TEXCOORD0;
            };

            TEXTURE2D(_BlitTexture);
            SAMPLER(sampler_BlitTexture);

            float4 _RainColor;
            float _RainDensity;
            float _RainSpeed;
            float _FW_ElapsedTicks;

            Varyings Vert(Attributes input)
            {
                Varyings output;
                output.positionCS = TransformObjectToHClip(input.positionOS.xyz);
                output.uv = input.uv;
                return output;
            }

            float Hash(float2 p)
            {
                return frac(sin(dot(p, float2(127.1, 311.7))) * 43758.5453123);
            }

            half4 Frag(Varyings input) : SV_Target
            {
                half4 scene = SAMPLE_TEXTURE2D(_BlitTexture, sampler_BlitTexture, input.uv);
                
                float2 uv = input.uv;
                // Skew UVs for diagonal rain
                uv.x += uv.y * 0.2;
                
                float time = _FW_ElapsedTicks * 0.0166 * _RainSpeed;
                
                float2 grid = float2(uv.x * _RainDensity, (uv.y + time) * _RainDensity * 0.2);
                float2 ipos = floor(grid);
                float2 fpos = frac(grid);
                
                float h = Hash(ipos);
                // Sharp streak pattern
                float rain = step(0.96, h) * smoothstep(0.0, 0.8, fpos.y);
                
                half3 finalColor = lerp(scene.rgb, _RainColor.rgb, rain * _RainColor.a);
                return half4(finalColor, scene.a);
            }
ENDHLSL
        }
    }
}

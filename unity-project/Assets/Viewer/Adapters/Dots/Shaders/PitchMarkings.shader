Shader "FinalWhistle/Viewer/Dots/PitchMarkings"
{
    Properties
    {
        _BaseColor ("Base Color", Color) = (0.18, 0.45, 0.20, 1)
        _LineColor ("Line Color", Color) = (0.92, 0.92, 0.92, 1)
        _LineThickness ("Line Thickness (m)", Float) = 0.12
        _PitchDimensions ("Pitch Dimensions (L, W)", Vector) = (105, 68, 0, 0)
        _GrassStripeWidth ("Grass Stripe Width (m)", Float) = 4.0
        _GrassStripeContrast ("Grass Stripe Contrast", Float) = 0.05
        _WearIntensity ("Wear Intensity", Float) = 0.15
    }
SubShader
    {
        Tags { "RenderType" = "Opaque" "RenderPipeline" = "UniversalPipeline" }
        LOD 100

        Pass
        {
            Name "PitchMarkings"
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
                float3 positionWS   : TEXCOORD1;
            };

            float4 _BaseColor;
            float4 _LineColor;
            float _LineThickness;
            float4 _PitchDimensions; // x: Length, y: Width
            float _FW_GoalFlashHome;
            float _FW_GoalFlashAway;
            float _GrassStripeWidth;
            float _GrassStripeContrast;
            float _WearIntensity;

            Varyings Vert(Attributes input)
{
Varyings output;
                output.positionCS = TransformObjectToHClip(input.positionOS.xyz);
                output.uv = input.uv;
                output.positionWS = TransformObjectToWorld(input.positionOS.xyz);
                return output;
            }

            float Stroke(float dist, float thickness)
            {
                float halfThickness = thickness * 0.5;
                // Use fwidth for anti-aliasing
                float delta = fwidth(dist);
                return smoothstep(delta, -delta, abs(dist) - halfThickness);
            }

            float Circle(float2 p, float r)
            {
                return length(p) - r;
            }

            float Box(float2 p, float2 b)
            {
                float2 d = abs(p) - b;
                return length(max(d, 0.0)) + min(max(d.x, d.y), 0.0);
            }

            float LineSegment(float2 p, float2 a, float2 b)
            {
                float2 pa = p - a, ba = b - a;
                float h = saturate(dot(pa, ba) / dot(ba, ba));
                return length(pa - ba * h);
            }

            half4 Frag(Varyings input) : SV_Target
            {
                float2 p = input.positionWS.xz;
                float halfL = _PitchDimensions.x * 0.5;
                float halfW = _PitchDimensions.y * 0.5;
                float t = _LineThickness * 1.5;

                // 1. Base Pitch with Checkered Pattern (FM style)
                float stripesX = floor(p.x / _GrassStripeWidth);
                float stripesY = floor(p.y / _GrassStripeWidth);
                float checkMask = fmod(stripesX + stripesY, 2.0);
                half3 pitchColor = lerp(_BaseColor.rgb, _BaseColor.rgb * (1.0 - _GrassStripeContrast), checkMask);

                // 2. Localized Wear (Goals and Center)
                // Goal areas wear down to slightly darker/desaturated grass
                float dGoalHome = length(p - float2(halfL, 0));
                float dGoalAway = length(p - float2(-halfL, 0));
                float dCenter = length(p);
                
                float wearMask = smoothstep(12.0, 4.0, dGoalHome) * 0.6;
                wearMask += smoothstep(12.0, 4.0, dGoalAway) * 0.6;
                wearMask += smoothstep(8.0, 2.0, dCenter) * 0.4;
                wearMask = saturate(wearMask) * _WearIntensity;
                
                pitchColor = lerp(pitchColor, pitchColor * 0.7, wearMask);

                float mask = 0;

                // 1. Boundary
                mask = max(mask, Stroke(Box(p, float2(halfL, halfW)), t));

                // 2. Halfway Line
                mask = max(mask, Stroke(p.x, t));

                // 3. Centre Circle
                mask = max(mask, Stroke(Circle(p, 9.15), t));

                // 4. Centre Spot
                mask = max(mask, smoothstep(0.2 + fwidth(length(p)), 0.2, length(p)));

                // 5. Penalty Areas
                // Home side (positive X)
                float2 pAreaHome = float2(halfL - 16.5, 0);
                float pAreaMask = Stroke(LineSegment(p, float2(halfL - 16.5, -20.16), float2(halfL - 16.5, 20.16)), t);
                pAreaMask = max(pAreaMask, Stroke(LineSegment(p, float2(halfL - 16.5, 20.16), float2(halfL, 20.16)), t));
                pAreaMask = max(pAreaMask, Stroke(LineSegment(p, float2(halfL - 16.5, -20.16), float2(halfL, -20.16)), t));
                mask = max(mask, pAreaMask);

                // Away side (negative X)
                float pAreaAwayMask = Stroke(LineSegment(p, float2(-(halfL - 16.5), -20.16), float2(-(halfL - 16.5), 20.16)), t);
                pAreaAwayMask = max(pAreaAwayMask, Stroke(LineSegment(p, float2(-(halfL - 16.5), 20.16), float2(-halfL, 20.16)), t));
                pAreaAwayMask = max(pAreaAwayMask, Stroke(LineSegment(p, float2(-(halfL - 16.5), -20.16), float2(-halfL, -20.16)), t));
                mask = max(mask, pAreaAwayMask);

                // 6. Goal Areas
                // Home
                float gAreaHomeMask = Stroke(LineSegment(p, float2(halfL - 5.5, -9.16), float2(halfL - 5.5, 9.16)), t);
                gAreaHomeMask = max(gAreaHomeMask, Stroke(LineSegment(p, float2(halfL - 5.5, 9.16), float2(halfL, 9.16)), t));
                gAreaHomeMask = max(gAreaHomeMask, Stroke(LineSegment(p, float2(halfL - 5.5, -9.16), float2(halfL, -9.16)), t));
                mask = max(mask, gAreaHomeMask);

                // Away
                float gAreaAwayMask = Stroke(LineSegment(p, float2(-(halfL - 5.5), -9.16), float2(-(halfL - 5.5), 9.16)), t);
                gAreaAwayMask = max(gAreaAwayMask, Stroke(LineSegment(p, float2(-(halfL - 5.5), 9.16), float2(-halfL, 9.16)), t));
                gAreaAwayMask = max(gAreaAwayMask, Stroke(LineSegment(p, float2(-(halfL - 5.5), -9.16), float2(-halfL, -9.16)), t));
                mask = max(mask, gAreaAwayMask);

                // 7. Penalty Spots
                float dSpotHome = length(p - float2(halfL - 11.0, 0));
                mask = max(mask, smoothstep(0.15 + fwidth(dSpotHome), 0.15, dSpotHome));
                float dSpotAway = length(p - float2(-(halfL - 11.0), 0));
                mask = max(mask, smoothstep(0.15 + fwidth(dSpotAway), 0.15, dSpotAway));

                // 8. Corner Arcs
                float2 cp = abs(p);
                float cornerDist = length(cp - float2(halfL, halfW));
                float cornerArc = Stroke(cornerDist - 1.0, t);
                cornerArc *= (cp.x < halfL && cp.y < halfW);
                mask = max(mask, cornerArc);

                // 9. Penalty Arcs
                float dArcHome = length(p - float2(halfL - 11.0, 0));
                float arcHome = Stroke(dArcHome - 9.15, t);
                arcHome *= (p.x < halfL - 16.5);
                mask = max(mask, arcHome);

                float dArcAway = length(p - float2(-(halfL - 11.0), 0));
                float arcAway = Stroke(dArcAway - 9.15, t);
                arcAway *= (p.x > -(halfL - 16.5));
                mask = max(mask, arcAway);

                // 10. Goal Visuals (Obvious U-Frame)
                float goalDepth = 2.0;
                float netT = t * 0.8;
                float gFrameHome = Stroke(LineSegment(p, float2(halfL, -3.66), float2(halfL + goalDepth, -3.66)), netT);
                gFrameHome = max(gFrameHome, Stroke(LineSegment(p, float2(halfL, 3.66), float2(halfL + goalDepth, 3.66)), netT));
                gFrameHome = max(gFrameHome, Stroke(LineSegment(p, float2(halfL + goalDepth, -3.66), float2(halfL + goalDepth, 3.66)), netT));
                mask = max(mask, gFrameHome);
                
                // Goal Posts (Circles at the corners of the goal mouth)
                float postR = 0.2; // 20cm post radius
                float posts = smoothstep(postR + fwidth(length(p - float2(halfL, 3.66))), postR, length(p - float2(halfL, 3.66)));
                posts = max(posts, smoothstep(postR + fwidth(length(p - float2(halfL, -3.66))), postR, length(p - float2(halfL, -3.66))));
                posts = max(posts, smoothstep(postR + fwidth(length(p - float2(-halfL, 3.66))), postR, length(p - float2(-halfL, 3.66))));
                posts = max(posts, smoothstep(postR + fwidth(length(p - float2(-halfL, -3.66))), postR, length(p - float2(-halfL, -3.66))));
                mask = max(mask, posts);

                float gFrameAway = Stroke(LineSegment(p, float2(-halfL, -3.66), float2(-(halfL + goalDepth), -3.66)), netT);
                gFrameAway = max(gFrameAway, Stroke(LineSegment(p, float2(-halfL, 3.66), float2(-(halfL + goalDepth), 3.66)), netT));
                gFrameAway = max(gFrameAway, Stroke(LineSegment(p, float2(-(halfL + goalDepth), -3.66), float2(-(halfL + goalDepth), 3.66)), netT));
                mask = max(mask, gFrameAway);

                // 11. Goal Flash
                float goalHomeLine = LineSegment(p, float2(halfL, -3.66), float2(halfL, 3.66));
                float goalAwayLine = LineSegment(p, float2(-halfL, -3.66), float2(-halfL, 3.66));
                float flashHome = smoothstep(3.0, 0.0, goalHomeLine) * _FW_GoalFlashHome;
                float flashAway = smoothstep(3.0, 0.0, goalAwayLine) * _FW_GoalFlashAway;
                float goalFlash = max(flashHome, flashAway);

                half3 finalColor = lerp(pitchColor, _LineColor.rgb, saturate(mask));
                finalColor += goalFlash * float3(1.2, 1.2, 1.2);
                return half4(finalColor, 1.0);
                }
                ENDHLSL
}
    }
    Fallback "Universal Render Pipeline/Unlit"
}

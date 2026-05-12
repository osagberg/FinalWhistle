using System;
using System.Collections.Generic;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Renders faint, glowing "Tactical Intent" lines between players.
    /// Used to visualize marking, passing lanes, and AI intent in high-stakes moments.
    /// </summary>
    public sealed class TacticalIntentRenderer : MonoBehaviour
    {
        private const int MaxLines = 40;
        private const float DefaultWidth = 0.08f;
        private const float GlowWidth = 0.25f;

        [SerializeField] private Material lineMaterial;
        [SerializeField] private Color homeColor = new(0.3f, 0.6f, 1.0f, 0.4f);
        [SerializeField] private Color awayColor = new(1.0f, 0.4f, 0.3f, 0.4f);

        private LineRenderer[] lines;
        private int activeCount;

        public void Initialize()
        {
            if (lineMaterial == null)
            {
                lineMaterial = new Material(Shader.Find("Universal Render Pipeline/Unlit"));
                lineMaterial.name = "TacticalIntentMaterial";
                lineMaterial.SetFloat("_Surface", 1); // Transparent
                lineMaterial.SetInt("_SrcBlend", (int)UnityEngine.Rendering.BlendMode.SrcAlpha);
                lineMaterial.SetInt("_DstBlend", (int)UnityEngine.Rendering.BlendMode.One); // Additive
                lineMaterial.SetInt("_ZWrite", 0);
            }

            lines = new LineRenderer[MaxLines];
            for (int i = 0; i < MaxLines; i++)
            {
                GameObject go = new($"IntentLine_{i}");
                go.transform.SetParent(transform, worldPositionStays: false);
                var lr = go.AddComponent<LineRenderer>();
                lr.sharedMaterial = lineMaterial;
                lr.startWidth = lr.endWidth = DefaultWidth;
                lr.positionCount = 2;
                lr.useWorldSpace = true;
                lr.enabled = false;
                lines[i] = lr;
            }
        }

        public void Clear()
        {
            for (int i = 0; i < MaxLines; i++)
            {
                lines[i].enabled = false;
            }
            activeCount = 0;
        }

        public void DrawLine(Vector3 start, Vector3 end, Color color, float width = DefaultWidth)
        {
            if (activeCount >= MaxLines) return;

            var lr = lines[activeCount];
            lr.enabled = true;
            lr.SetPosition(0, start + Vector3.up * 0.04f);
            lr.SetPosition(1, end + Vector3.up * 0.04f);
            lr.startColor = lr.endColor = color;
            lr.startWidth = lr.endWidth = width;
            activeCount++;
        }

        public void UpdateIntent(DotPool dotPool, bool highStakes)
        {
            Clear();
            // Force active for verification if any player is near ball
            // if (!highStakes) return; 

            Vector3 ballPos = dotPool.BallWorldPosition;
int carrierIdx = -1;
            float minSqrDist = 2.0f;

            // Simple heuristic to find carrier
            for (int i = 0; i < DotPool.TotalPlayers; i++)
            {
                float sqrDist = (dotPool.transform.GetChild(i).position - ballPos).sqrMagnitude;
                if (sqrDist < minSqrDist)
                {
                    minSqrDist = sqrDist;
                    carrierIdx = i;
                }
            }

            if (carrierIdx == -1) return;

            Transform carrierT = dotPool.transform.GetChild(carrierIdx);
            Vector3 carrierPos = carrierT.position;
            bool isHome = carrierIdx < DotPool.PlayersPerSide;

            // 1. "Pressure" lines: Defenders near carrier
            for (int i = 0; i < DotPool.TotalPlayers; i++)
            {
                if (i == carrierIdx) continue;
                bool otherIsHome = i < DotPool.PlayersPerSide;
                if (otherIsHome == isHome) continue; // Only opposing team

                Transform defenderT = dotPool.transform.GetChild(i);
                Vector3 defenderPos = defenderT.position;
                float sqrDist = (defenderPos - carrierPos).sqrMagnitude;

                if (sqrDist < 100.0f) // Within 10m
                {
                    float alpha = Mathf.InverseLerp(100.0f, 9.0f, sqrDist) * 0.3f;
                    Color c = awayColor;
                    c.a = alpha;
                    DrawLine(defenderPos, carrierPos, c, DefaultWidth * (1.0f + alpha * 2f));
                }
            }

            // 2. "Pass Lane" lines: From carrier to open teammates forward
            for (int i = 0; i < DotPool.TotalPlayers; i++)
            {
                if (i == carrierIdx) continue;
                bool otherIsHome = i < DotPool.PlayersPerSide;
                if (otherIsHome != isHome) continue; // Same team

                Transform teammateT = dotPool.transform.GetChild(i);
                Vector3 teammatePos = teammateT.position;
                
                // Only forward teammates
                float forwardDir = isHome ? 1.0f : -1.0f;
                if ((teammatePos.x - carrierPos.x) * forwardDir < 2.0f) continue;

                float dist = Vector3.Distance(carrierPos, teammatePos);
                if (dist < 25.0f)
                {
                    Color c = homeColor;
                    c.a = 0.15f;
                    DrawLine(carrierPos, teammatePos, c, DefaultWidth);
                }
            }
        }
    }
}

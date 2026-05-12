using System;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Builds stylized manga-style stadium stands as a procedural mesh "bowl"
    /// to avoid SpriteRenderer tiling issues and provide a more immersive environment.
    /// </summary>
    public sealed class StadiumPerimeter : MonoBehaviour
    {
        [SerializeField] private Sprite crowdSprite;
        [SerializeField] private float standHeight = 50f;
        [SerializeField] private float standDistance = 15f;
        [SerializeField] private float standIncline = 20.0f;
        [SerializeField] private Color standColor = new(0.15f, 0.17f, 0.22f, 1f); // Darker, professional grey-blue
        [SerializeField] private Material standMaterial;

        public void Initialize(PitchView pitchView)
        {
            if (pitchView == null) throw new ArgumentNullException(nameof(pitchView));

            // Adjust stand parameters for a more massive feel
            standHeight = 40f;
            standDistance = 12f;
            standIncline = 30f;

            float halfL = (pitchView.PitchLengthMeters * 0.5f + standDistance) * pitchView.WorldUnitsPerMeter;
            float halfW = (pitchView.PitchWidthMeters * 0.5f + standDistance) * pitchView.WorldUnitsPerMeter;

            // Clear existing
            for (int i = transform.childCount - 1; i >= 0; i--)
            {
                if (Application.isPlaying) Destroy(transform.GetChild(i).gameObject);
                else DestroyImmediate(transform.GetChild(i).gameObject);
            }

            // Create tiered stands with more tiers
            int tiersCount = 4;
            for (int t = 0; t < tiersCount; t++)
            {
                float h0 = (t / (float)tiersCount) * standHeight;
                float h1 = ((t + 1) / (float)tiersCount) * standHeight;
                float d0 = (t / (float)tiersCount) * standIncline;
                float d1 = ((t + 1) / (float)tiersCount) * standIncline;

                // Tiling factor increases with tiers
                float tilingX = 15f + t * 5f;
                float tilingY = 4f;

                // North Stand Tier
                CreateSlantedStand($"Stand_North_{t}", 
                    new Vector3(-halfL * 1.5f - d0, h0, halfW + d0), 
                    new Vector3(halfL * 1.5f + d0, h0, halfW + d0), 
                    new Vector3(halfL * 1.5f + d1, h1, halfW + d1), 
                    new Vector3(-halfL * 1.5f - d1, h1, halfW + d1),
                    tilingX, tilingY);
                
                // South
                CreateSlantedStand($"Stand_South_{t}", 
                    new Vector3(halfL * 1.5f + d0, h0, -(halfW + d0)), 
                    new Vector3(-halfL * 1.5f - d0, h0, -(halfW + d0)), 
                    new Vector3(-halfL * 1.5f - d1, h1, -(halfW + d1)), 
                    new Vector3(halfL * 1.5f + d1, h1, -(halfW + d1)),
                    tilingX, tilingY);

                // East
                CreateSlantedStand($"Stand_East_{t}", 
                    new Vector3(halfL + d0, h0, -halfW - d0), 
                    new Vector3(halfL + d0, h0, halfW + d0), 
                    new Vector3(halfL + d1, h1, halfW + d1), 
                    new Vector3(halfL + d1, h1, -halfW - d1),
                    tilingX, tilingY);

                // West
                CreateSlantedStand($"Stand_West_{t}", 
                    new Vector3(-halfL - d0, h0, halfW + d0), 
                    new Vector3(-halfL - d0, h0, -halfW - d0), 
                    new Vector3(-halfL - d1, h1, -halfW - d1), 
                    new Vector3(-halfL - d1, h1, halfW + d1),
                    tilingX, tilingY);
            }
        }

        private void CreateSlantedStand(string name, Vector3 v0, Vector3 v1, Vector3 v2, Vector3 v3, float tilingX, float tilingY)
        {
            GameObject stand = new(name);
            stand.transform.SetParent(transform, false);
            
            MeshFilter mf = stand.AddComponent<MeshFilter>();
            MeshRenderer mr = stand.AddComponent<MeshRenderer>();
            
            Mesh mesh = new Mesh();
            mesh.vertices = new Vector3[] { v0, v1, v2, v3 };
            mesh.triangles = new int[] { 0, 2, 1, 0, 3, 2 };
            mesh.uv = new Vector2[] { new Vector2(0, 0), new Vector2(tilingX, 0), new Vector2(tilingX, tilingY), new Vector2(0, tilingY) };
            mesh.RecalculateNormals();
            mf.sharedMesh = mesh;

            if (standMaterial == null)
            {
                standMaterial = new Material(Shader.Find("Universal Render Pipeline/Lit"));
                standMaterial.name = "StadiumStandMaterial";
                standMaterial.SetFloat("_Smoothness", 0.2f);
            }
            
            mr.sharedMaterial = standMaterial;
            mr.sharedMaterial.SetColor("_BaseColor", standColor);
            if (crowdSprite != null)
            {
                mr.sharedMaterial.SetTexture("_BaseMap", crowdSprite.texture);
                // Set tiling if possible (Lit uses _BaseMap_ST)
            }
        }
    }
}

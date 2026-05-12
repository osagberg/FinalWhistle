using System;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Stylized movement trail for the ball.
    /// Only visible when the ball is moving at high speed.
    /// </summary>
    [RequireComponent(typeof(TrailRenderer))]
    public sealed class BallTrail : MonoBehaviour
    {
        private const float MinSpeedThreshold = 10f; // m/s
        private const float MaxSpeedThreshold = 30f; // m/s

        [SerializeField] private TrailRenderer trail;
        [SerializeField] private float baseWidth = 0.5f;

        private void OnValidate()
        {
            if (trail == null) trail = GetComponent<TrailRenderer>();
        }

        public void Initialize()
        {
            if (trail == null) trail = GetComponent<TrailRenderer>();
            
            trail.widthMultiplier = baseWidth;
            trail.time = 0.2f;
            trail.minVertexDistance = 0.1f;
            trail.emitting = false;
            
            // Setup material if missing
            if (trail.sharedMaterial == null)
            {
                Shader unlitShader = Shader.Find("Universal Render Pipeline/Unlit");
                if (unlitShader != null)
                {
                    Material mat = new Material(unlitShader);
                    mat.name = "BallTrailMat";
                    mat.SetColor("_BaseColor", new Color(1, 1, 1, 0.5f));
                    // Ensure transparency works
                    mat.SetFloat("_Surface", 1); // Transparent
                    mat.SetInt("_SrcBlend", (int)UnityEngine.Rendering.BlendMode.SrcAlpha);
                    mat.SetInt("_DstBlend", (int)UnityEngine.Rendering.BlendMode.OneMinusSrcAlpha);
                    mat.SetInt("_ZWrite", 0);
                    mat.renderQueue = (int)UnityEngine.Rendering.RenderQueue.Transparent;
                    trail.sharedMaterial = mat;
                }
            }
            
            // Gradient: Fade out
            Gradient gradient = new Gradient();
            gradient.SetKeys(
                new GradientColorKey[] { new GradientColorKey(Color.white, 0.0f), new GradientColorKey(Color.white, 1.0f) },
                new GradientAlphaKey[] { new GradientAlphaKey(0.6f, 0.0f), new GradientAlphaKey(0.0f, 1.0f) }
            );
            trail.colorGradient = gradient;
        }

        public void UpdateTrail(Vector3 velocity)
        {
            float speed = velocity.magnitude;
            if (speed > MinSpeedThreshold)
            {
                trail.emitting = true;
                // Scale width and time by speed
                float t = Mathf.InverseLerp(MinSpeedThreshold, MaxSpeedThreshold, speed);
                trail.widthMultiplier = baseWidth * Mathf.Lerp(0.5f, 1.5f, t);
                trail.time = Mathf.Lerp(0.1f, 0.3f, t);
            }
            else
            {
                trail.emitting = false;
            }
        }
    }
}

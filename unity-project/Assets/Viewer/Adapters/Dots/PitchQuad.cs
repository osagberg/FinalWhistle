using System;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Builds the 105×68m green pitch surface as a runtime <see cref="Mesh"/>
    /// per the Phase-3 dots-adapter blueprint §B Slice 2. Single-quad
    /// geometry (4 verts, 2 tris) at world origin in the XZ plane (Y=0 is
    /// the pitch surface). UVs cover 0..1 across both axes so the Slice-5
    /// screen-tone fullscreen pass has a usable texture coordinate without
    /// needing a re-author of this geometry.
    ///
    /// <para>
    /// <strong>Hand-rolled, no Cinemachine:</strong> the camera authoring
    /// stays in <see cref="DotsMatchDirector"/> per blueprint Decision 2
    /// (Cinemachine 3.1.6 is installed but premature for a locked
    /// orthographic top-down view; the 3D adapter ADR-0010 is the natural
    /// Cinemachine consumer if/when the Phase-5 spike greenlights).
    /// </para>
    ///
    /// <para>
    /// <strong>Shader discipline:</strong> uses URP/Unlit at flat green; no
    /// <c>_Time</c> reads. <c>scripts/fw shader-audit</c> trips the moment
    /// any viewer-adapter shader under <c>Assets/Viewer/</c> imports a
    /// frame-time intrinsic, per ADR-0008/0009.
    /// </para>
    /// </summary>
    [RequireComponent(typeof(MeshFilter))]
    [RequireComponent(typeof(MeshRenderer))]
    public sealed class PitchQuad : MonoBehaviour
    {
        // Phase-3 deep football-pitch green; pre-async-art-director-review
        // starting value. Async route: capture L2 screenshot → art-director
        // palette pass → adjust here if the green clashes with the home/away
        // tint families.
        private static readonly Color PitchGreen = new(0.18f, 0.45f, 0.20f);

        // Hairline boundary so the camera framing reads even at full
        // tactical-wide zoom; centre-line + halfway circle land Slice 4
        // when the camera-rhythm work demands more pitch detail.
        private static readonly Color BoundaryWhite = new(0.92f, 0.92f, 0.92f);
        private const float BoundaryWidthMetres = 0.12f;

        // Y lift for boundary line so it z-orders above the pitch quad
        // without z-fighting at the chosen ortho size; default URP 24-bit
        // depth buffer with near=0.1/far=200 gives ~0.00006-unit precision
        // so 1cm is well clear (~166 depth steps; pr-review-toolkit
        // feature-dev:code-reviewer Slice-2 P3 confirmed).
        private const float BoundaryYLift = 0.01f;

        private MeshFilter meshFilter;
        private MeshRenderer meshRenderer;
        private Mesh pitchMesh;
        private Material pitchMaterial;

        private LineRenderer boundary;
        private Material boundaryMaterial;

        /// <summary>
        /// Build the pitch geometry from <paramref name="pitchView"/>.
        /// Called by <see cref="DotsMatchDirector"/> on Awake; idempotent —
        /// re-calling rebuilds the mesh without leaking the previous one.
        /// </summary>
        public void Initialize(PitchView pitchView)
        {
            if (pitchView is null)
            {
                throw new ArgumentNullException(nameof(pitchView));
            }

            EnsureComponents();

            float halfLength = (pitchView.PitchLengthMeters * 0.5f) * pitchView.WorldUnitsPerMeter;
            float halfWidth = (pitchView.PitchWidthMeters * 0.5f) * pitchView.WorldUnitsPerMeter;

            // CCW from above (+Y up) so face normal points toward the
            // top-down camera looking down -Y; otherwise URP backface-culls
            // and the screen reads as the camera clear colour.
            Vector3[] verts = new[]
            {
                new Vector3(-halfLength, 0f, -halfWidth),
                new Vector3(-halfLength, 0f, halfWidth),
                new Vector3(halfLength, 0f, halfWidth),
                new Vector3(halfLength, 0f, -halfWidth),
            };
            int[] tris = new[] { 0, 1, 2, 0, 2, 3 };
            Vector2[] uvs = new[]
            {
                new Vector2(0f, 0f),
                new Vector2(0f, 1f),
                new Vector2(1f, 1f),
                new Vector2(1f, 0f),
            };
            Vector3[] normals = new[] { Vector3.up, Vector3.up, Vector3.up, Vector3.up };

            if (pitchMesh == null)
            {
                pitchMesh = new Mesh { name = "FW.Viewer.Adapters.Dots.PitchQuad" };
            }
            else
            {
                pitchMesh.Clear();
            }
            pitchMesh.vertices = verts;
            pitchMesh.triangles = tris;
            pitchMesh.uv = uvs;
            pitchMesh.normals = normals;
            pitchMesh.RecalculateBounds();

            meshFilter.sharedMesh = pitchMesh;

            if (pitchMaterial == null)
            {
                pitchMaterial = CreateUnlitMaterial("FW.Viewer.Adapters.Dots.PitchMat", PitchGreen);
            }
            meshRenderer.sharedMaterial = pitchMaterial;
            meshRenderer.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
            meshRenderer.receiveShadows = false;

            BuildBoundary(halfLength, halfWidth);
        }

        // Centralised loud-fail per pr-review-toolkit silent-failure-hunter
        // Slice-2 P2: the original pitch-material path threw if URP/Unlit
        // wasn't found, but BuildBoundary made the same Shader.Find call
        // without a guard — a partial URP install would have produced a
        // magenta-fallback boundary line silently. Both paths now route
        // through this helper.
        private static Material CreateUnlitMaterial(string name, Color color)
        {
            Shader unlit = Shader.Find("Universal Render Pipeline/Unlit");
            if (unlit == null)
            {
                throw new InvalidOperationException(
                    "Universal Render Pipeline/Unlit shader not found. " +
                    "PitchQuad requires URP; verify Packages/manifest.json + " +
                    "active GraphicsSettings render pipeline asset.");
            }
            Material mat = new(unlit) { name = name };
            mat.SetColor("_BaseColor", color);
            return mat;
        }

        private void EnsureComponents()
        {
            if (meshFilter == null)
            {
                meshFilter = GetComponent<MeshFilter>();
            }
            if (meshRenderer == null)
            {
                meshRenderer = GetComponent<MeshRenderer>();
            }
        }

        private void BuildBoundary(float halfLength, float halfWidth)
        {
            if (boundary == null)
            {
                GameObject boundaryGO = new("PitchBoundary");
                boundaryGO.transform.SetParent(transform, worldPositionStays: false);
                boundary = boundaryGO.AddComponent<LineRenderer>();
                boundary.useWorldSpace = false;
                boundary.loop = true;
                boundary.startWidth = BoundaryWidthMetres;
                boundary.endWidth = BoundaryWidthMetres;
                boundary.numCornerVertices = 0;
                boundary.numCapVertices = 0;
                boundary.shadowCastingMode = UnityEngine.Rendering.ShadowCastingMode.Off;
                boundary.receiveShadows = false;
            }
            if (boundaryMaterial == null)
            {
                boundaryMaterial = CreateUnlitMaterial("FW.Viewer.Adapters.Dots.BoundaryMat", BoundaryWhite);
                boundary.sharedMaterial = boundaryMaterial;
            }
            boundary.positionCount = 4;
            boundary.SetPositions(new[]
            {
                new Vector3(-halfLength, BoundaryYLift, -halfWidth),
                new Vector3(-halfLength, BoundaryYLift, halfWidth),
                new Vector3(halfLength, BoundaryYLift, halfWidth),
                new Vector3(halfLength, BoundaryYLift, -halfWidth),
            });
        }

        // pr-review-toolkit feature-dev:code-reviewer Slice-2 P1: the
        // boundary material was previously a local variable inside
        // BuildBoundary, so OnDestroy could not destroy it — every
        // re-Initialize leaked one Material. Now stored in the field +
        // destroyed alongside pitchMesh + pitchMaterial.
        private void OnDestroy()
        {
            DestroyOwned(pitchMesh);
            DestroyOwned(pitchMaterial);
            DestroyOwned(boundaryMaterial);
        }

        private static void DestroyOwned(UnityEngine.Object obj)
        {
            if (obj == null)
            {
                return;
            }
            if (Application.isPlaying)
            {
                Destroy(obj);
            }
            else
            {
                DestroyImmediate(obj);
            }
        }
    }
}

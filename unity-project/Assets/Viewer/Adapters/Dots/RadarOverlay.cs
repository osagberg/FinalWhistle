using System;
using FinalWhistle.Viewer.Core;
using UnityEngine;
using UnityEngine.UI;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Renders a 2D radar (mini-map) of the pitch and players.
    /// </summary>
    public sealed class RadarOverlay : MonoBehaviour
    {
        [SerializeField] private RectTransform radarRoot;
        [SerializeField] private Image pitchImage;
        [SerializeField] private RectTransform playerContainer;
        [SerializeField] private GameObject playerMarkerPrefab;

        private Image[] playerMarkers;
        private RectTransform[] playerTransforms;
        private RectTransform ballTransform;
        private PitchView pitchView;

        public void Initialize(PitchView pitchViewArg)
        {
            pitchView = pitchViewArg;

            if (playerMarkerPrefab == null)
            {
                GameObject go = new GameObject("RadarMarker");
                go.SetActive(false);
                var rt = go.AddComponent<RectTransform>();
                rt.sizeDelta = new Vector2(8, 8);
                var img = go.AddComponent<Image>();
                img.color = Color.white;
                playerMarkerPrefab = go;
            }

            // Create markers
            playerMarkers = new Image[DotPool.TotalDots];
            playerTransforms = new RectTransform[DotPool.TotalDots];

            for (int i = 0; i < DotPool.TotalDots; i++)
            {
                var marker = Instantiate(playerMarkerPrefab, playerContainer);
                marker.SetActive(true);
                playerMarkers[i] = marker.GetComponent<Image>();
                playerTransforms[i] = marker.GetComponent<RectTransform>();
            }
            
            // Set ball marker special
            playerMarkers[DotPool.BallIndex].color = Color.white;
            playerTransforms[DotPool.BallIndex].sizeDelta = new Vector2(6, 6);
        }

        public void UpdateRadar(DotPool dotPool, IdentityTintTable tints)
        {
            if (pitchView == null) return;

            float pitchL = pitchView.PitchLengthMeters;
            float pitchW = pitchView.PitchWidthMeters;
            Vector2 radarSize = radarRoot.sizeDelta;

            for (int i = 0; i < DotPool.TotalDots; i++)
            {
                // We use transform children because dots are children of DotPool
                Transform dotT = dotPool.transform.GetChild(i);
                Vector3 worldPos = dotT.position;

                float normX = worldPos.x / (pitchL * 0.5f);
                float normY = worldPos.z / (pitchW * 0.5f);

                playerTransforms[i].anchoredPosition = new Vector2(normX * (radarSize.x * 0.5f), normY * (radarSize.y * 0.5f));

                if (i < DotPool.TotalPlayers)
                {
                    bool isHome = i < DotPool.PlayersPerSide;
                    // Refined colors: Modern Blue and Red
                    playerMarkers[i].color = isHome ? new Color(0.12f, 0.45f, 0.95f) : new Color(0.95f, 0.15f, 0.12f);
                    playerTransforms[i].sizeDelta = new Vector2(12, 12);
                }
                else
                {
                    // Ball
                    playerMarkers[i].color = Color.white;
                    playerTransforms[i].sizeDelta = new Vector2(8, 8);
                }
            }
        }
}
}

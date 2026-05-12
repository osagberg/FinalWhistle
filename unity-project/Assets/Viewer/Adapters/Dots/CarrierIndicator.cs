using System;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Highlights the player currently in possession of the ball (the "carrier").
    /// Per C6.4: always-on while a player is the canonical carrier.
    /// Since the Phase-3 sim does not explicitly expose the carrier field,
    /// we use a proximity heuristic: the dot closest to the ball within
    /// a 1.2m radius is considered the carrier.
    /// </summary>
    public sealed class CarrierIndicator : MonoBehaviour
    {
        private const float RingYLift = 0.055f; // Just above dot, below SelectionRing
        private const float CarrierDiameterMultiplier = 1.3f;
        private const float OutfieldDiameterMetres = 1.4f;
        private const float ProximityThresholdMetres = 1.2f;

        [SerializeField] private Sprite ringSprite;
        [SerializeField] private Color ringColor = new(1f, 1f, 1f, 0.4f);

        private DotPool dotPool;
        private PitchView pitchView;
        private SpriteRenderer ringRenderer;
        private int currentCarrierIndex = -1;

        public void Initialize(DotPool dotPoolArg, PitchView pitchViewArg)
        {
            if (dotPoolArg == null) throw new ArgumentNullException(nameof(dotPoolArg));
            if (pitchViewArg == null) throw new ArgumentNullException(nameof(pitchViewArg));
            if (ringSprite == null)
            {
                throw new InvalidOperationException("CarrierIndicator ringSprite missing.");
            }

            dotPool = dotPoolArg;
            pitchView = pitchViewArg;

            if (ringRenderer == null)
            {
                GameObject ringObj = new("CarrierRing");
                ringObj.transform.SetParent(transform, worldPositionStays: false);
                ringObj.transform.localRotation = Quaternion.Euler(90f, 0f, 0f);
                ringRenderer = ringObj.AddComponent<SpriteRenderer>();
                ringRenderer.sprite = ringSprite;
                ringRenderer.color = ringColor;
                ringRenderer.sortingOrder = -2; // Behind dots and selection ring
                float diameter = OutfieldDiameterMetres * CarrierDiameterMultiplier * pitchView.WorldUnitsPerMeter;
                ringObj.transform.localScale = new Vector3(diameter, diameter, diameter);
            }
            ringRenderer.enabled = false;
        }

        private void LateUpdate()
        {
            if (dotPool == null || ringRenderer == null) return;

            Vector3 ballPos = dotPool.BallWorldPosition;
            int closestIdx = -1;
            float minSqrDist = ProximityThresholdMetres * ProximityThresholdMetres;

            // Check all 22 players (dot indices 0-21)
            for (int i = 0; i < DotPool.TotalPlayers; i++)
            {
                // We can't access dotPool.dots directly as it is private.
                // But we can use the transform children since dots are parents of dotPool.
                Transform dotT = dotPool.transform.GetChild(i);
                Vector3 dotPos = dotT.position;
                float sqrDist = (dotPos - ballPos).sqrMagnitude;
                if (sqrDist < minSqrDist)
                {
                    minSqrDist = sqrDist;
                    closestIdx = i;
                }
            }

            if (closestIdx != -1)
            {
                ringRenderer.enabled = true;
                Transform targetT = dotPool.transform.GetChild(closestIdx);
                Vector3 targetPos = targetT.position;
                ringRenderer.transform.position = new Vector3(targetPos.x, RingYLift, targetPos.z);
            }
            else
            {
                ringRenderer.enabled = false;
            }
        }
    }
}

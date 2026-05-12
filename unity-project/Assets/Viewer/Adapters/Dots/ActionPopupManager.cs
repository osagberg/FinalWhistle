using System;
using System.Collections.Generic;
using UnityEngine;
using UnityEngine.UI;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Manages world-space text popups for big moments.
    /// Classic manga tropes like "GOAL!!", "BLOCK!", "OFFSIDE".
    /// </summary>
    public sealed class ActionPopupManager : MonoBehaviour
    {
        [SerializeField] private GameObject popupPrefab;
        [SerializeField] private Transform poolRoot;

        private Queue<PopupInstance> pool = new Queue<PopupInstance>();
        private List<PopupInstance> active = new List<PopupInstance>();

        private struct PopupInstance
        {
            public GameObject gameObject;
            public Text text;
            public float startTime;
            public Vector3 worldPos;
        }

        public void Initialize()
        {
            if (popupPrefab == null)
            {
                // Create a fallback prefab if none provided
                GameObject go = new GameObject("PopupPrefab");
                go.SetActive(false);
                Canvas canvas = go.AddComponent<Canvas>();
                canvas.renderMode = RenderMode.WorldSpace;
                go.AddComponent<CanvasScaler>();
                
                GameObject textGo = new GameObject("Text");
                textGo.transform.SetParent(go.transform);
                Text t = textGo.AddComponent<Text>();
                // Find the first available font (usually LegacyRuntime or System)
                Font[] allFonts = Resources.FindObjectsOfTypeAll<Font>();
                if (allFonts.Length > 0) t.font = allFonts[0];
t.fontSize = 24;
                t.alignment = TextAnchor.MiddleCenter;
                t.color = Color.white;
                
                var outline = textGo.AddComponent<Outline>();
                outline.effectColor = Color.black;
                outline.effectDistance = new Vector2(2, -2);
                
                RectTransform rt = t.rectTransform;
                rt.sizeDelta = new Vector2(200, 50);
                rt.localPosition = Vector3.zero;
                rt.localRotation = Quaternion.Euler(90, 0, 0); // Flat on pitch
                
                popupPrefab = go;
            }
            
            if (poolRoot == null) poolRoot = transform;
        }

        public void SpawnPopup(string message, Vector3 worldPos)
        {
            PopupInstance instance;
            if (pool.Count > 0)
            {
                instance = pool.Dequeue();
            }
            else
            {
                GameObject go = Instantiate(popupPrefab, poolRoot);
                instance = new PopupInstance
                {
                    gameObject = go,
                    text = go.GetComponentInChildren<Text>()
                };
            }

            instance.gameObject.SetActive(true);
            instance.text.text = message;
            instance.startTime = Time.time;
            instance.worldPos = worldPos;
            instance.gameObject.transform.position = worldPos + Vector3.up * 1.0f;
            
            active.Add(instance);
        }

        private void Update()
        {
            float duration = 1.5f;
            for (int i = active.Count - 1; i >= 0; i--)
            {
                var inst = active[i];
                float elapsed = Time.time - inst.startTime;
                if (elapsed > duration)
                {
                    inst.gameObject.SetActive(false);
                    pool.Enqueue(inst);
                    active.RemoveAt(i);
                    continue;
                }

                // Animation: Float up and scale up/down
                float t = elapsed / duration;
                inst.gameObject.transform.position = inst.worldPos + Vector3.up * (1.0f + t * 2.0f);
                float scale = Mathf.Sin(t * Mathf.PI) * 1.2f;
                inst.gameObject.transform.localScale = Vector3.one * scale;
                
                // Fade out
                Color c = inst.text.color;
                c.a = 1.0f - t;
                inst.text.color = c;
            }
        }
    }
}

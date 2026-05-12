using System;
using System.Collections.Generic;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Manga-style dust puffs emitted during sharp turns or acceleration.
    /// </summary>
    public sealed class SpeedDustEmitter : MonoBehaviour
    {
        public const int PoolSize = 32;
        public const int FadeTicks = 12;

        [SerializeField] private Sprite dustSprite;
        [SerializeField] private Color dustColor = new(1, 1, 1, 0.4f);

        private DotPool dotPool;
        private PitchView pitchView;
        private SpriteRenderer[] pool;
        private int[] startTicks;
        private int poolWriteCursor;

        public void Initialize(DotPool dotPoolArg, PitchView pitchViewArg)
        {
            dotPool = dotPoolArg;
            pitchView = pitchViewArg;

            if (pool == null)
            {
                pool = new SpriteRenderer[PoolSize];
                startTicks = new int[PoolSize];
                for (int i = 0; i < PoolSize; i++)
                {
                    GameObject go = new($"SpeedDust_{i}");
                    go.transform.SetParent(transform, worldPositionStays: false);
                    go.transform.localRotation = Quaternion.Euler(90f, 0f, 0f);
                    go.transform.localScale = Vector3.one * 0.8f;
                    var sr = go.AddComponent<SpriteRenderer>();
                    sr.sprite = dustSprite;
                    sr.color = new Color(dustColor.r, dustColor.g, dustColor.b, 0f);
                    sr.enabled = false;
                    sr.sortingOrder = -3; 
                    pool[i] = sr;
                    startTicks[i] = int.MinValue;
                }
            }
        }

        public void Emit(Vector3 position, int currentTick)
        {
            int slot = poolWriteCursor;
            poolWriteCursor = (poolWriteCursor + 1) % PoolSize;
            
            SpriteRenderer sr = pool[slot];
            if (sr == null) return;

            sr.transform.position = position + Vector3.up * 0.03f;
            sr.color = dustColor;
            sr.enabled = true;
            startTicks[slot] = currentTick;
        }

        public void Tick(int currentTick)
        {
            if (pool == null) return;
            for (int i = 0; i < PoolSize; i++)
            {
                SpriteRenderer sr = pool[i];
                if (sr == null || !sr.enabled) continue;
                int elapsed = currentTick - startTicks[i];
                if (elapsed >= FadeTicks)
                {
                    sr.enabled = false;
                    continue;
                }
                float alpha = 1f - (elapsed / (float)FadeTicks);
                sr.color = new Color(dustColor.r, dustColor.g, dustColor.b, alpha * dustColor.a);
                sr.transform.localScale = Vector3.one * (0.8f + elapsed * 0.05f); // Expand
            }
        }
    }
}

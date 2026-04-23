// AudioGenerator — procedural placeholder .wav generator for prototyping.
//
// Why editor-only?
//   This is a dev-time tool. Lives under Assets/_Project/Editor/Audio/ so it's
//   stripped from shipped builds automatically. UnityEditor API usage
//   (EditorWindow, AssetDatabase.Refresh, MenuItem) requires it.
//
// Why procedural rather than static samples?
//   Zero licensing, zero Asset Store spend, zero external dependencies. You
//   can regenerate every placeholder SFX in under a second. Good enough for
//   "does the interaction feel right?" iteration. Not good enough to ship.
//
// Output layout:
//   Assets/_Project/Audio/SFX/_placeholder/<name>.wav
//   Assets/_Project/Audio/SFX/_placeholder/<name>.meta.json   ← sidecar
//   Assets/_Project/Audio/BGM/_placeholder/<name>.wav
//   Assets/_Project/Audio/BGM/_placeholder/<name>.meta.json
//
// The sidecar records the generator name and every parameter used. Purpose:
//   (a) reproducibility — same params always produce the same file.
//   (b) auditability — you can see why a placeholder sounds the way it does.
//   (c) machine-readable for Claude to batch-regenerate from a spec.

#if UNITY_EDITOR
using System;
using System.Collections.Generic;
using System.Globalization;
using System.IO;
using UnityEditor;
using UnityEngine;

namespace {{PROJECT_NAME}}.Editor.Audio
{
    public static class AudioGenerator
    {
        private const int SampleRate = 44100;
        private const int BitsPerSample = 16;
        private const int Channels = 1;
        private const int GeneratorVersion = 1;

        private const string SfxRoot = "Assets/_Project/Audio/SFX/_placeholder";
        private const string BgmRoot = "Assets/_Project/Audio/BGM/_placeholder";
        private const string ConfigPath = "/tmp/audio-gen.json";
        private const string MenuRoot = "{{PROJECT_NAME}}/Audio/";

        // -----------------------------------------------------------------
        // Menu entries — interactive generators with param windows.
        // -----------------------------------------------------------------

        [MenuItem(MenuRoot + "Generate Beep")]
        public static void MenuBeep() => BeepWindow.Open();

        [MenuItem(MenuRoot + "Generate Noise")]
        public static void MenuNoise() => NoiseWindow.Open();

        [MenuItem(MenuRoot + "Generate Impact")]
        public static void MenuImpact() => ImpactWindow.Open();

        [MenuItem(MenuRoot + "Generate Chime")]
        public static void MenuChime() => ChimeWindow.Open();

        [MenuItem(MenuRoot + "Generate Sweep")]
        public static void MenuSweep() => SweepWindow.Open();

        [MenuItem(MenuRoot + "Generate BGM")]
        public static void MenuBgm() => BgmWindow.Open();

        [MenuItem(MenuRoot + "Generate From Config")]
        public static void MenuFromConfig() => GenerateFromConfig(ConfigPath);

        // -----------------------------------------------------------------
        // Public generator API — callable from other editor scripts / tests.
        // -----------------------------------------------------------------

        public static string GenerateBeep(float frequency, float duration, float volume = 0.8f, float decay = 5f)
        {
            var samples = Beep(frequency, duration, volume, decay);
            var name = $"beep_{FreqTag(frequency)}_{DurTag(duration)}";
            var path = Path.Combine(SfxRoot, name + ".wav").Replace('\\', '/');
            WriteWavAndSidecar(path, samples, new Dictionary<string, object>
            {
                { "generator", "Beep" },
                { "frequency", frequency },
                { "duration", duration },
                { "volume", volume },
                { "decay", decay },
            }, seed: null);
            return path;
        }

        public static string GenerateNoise(float duration, string color = "white", float volume = 0.6f, int? seed = null)
        {
            int usedSeed = seed ?? new System.Random().Next();
            var samples = Noise(duration, color, volume, usedSeed);
            var name = $"noise_{color}_{DurTag(duration)}";
            var path = Path.Combine(SfxRoot, name + ".wav").Replace('\\', '/');
            WriteWavAndSidecar(path, samples, new Dictionary<string, object>
            {
                { "generator", "Noise" },
                { "duration", duration },
                { "color", color },
                { "volume", volume },
            }, seed: usedSeed);
            return path;
        }

        public static string GenerateImpact(float intensity = 0.8f, int? seed = null)
        {
            intensity = Mathf.Clamp(intensity, 0.1f, 1.5f);
            int usedSeed = seed ?? new System.Random().Next();
            var samples = Impact(intensity, usedSeed);
            var name = $"impact_i{(int)(intensity * 100)}";
            var path = Path.Combine(SfxRoot, name + ".wav").Replace('\\', '/');
            WriteWavAndSidecar(path, samples, new Dictionary<string, object>
            {
                { "generator", "Impact" },
                { "intensity", intensity },
            }, seed: usedSeed);
            return path;
        }

        public static string GenerateChime(float baseFrequency, int overtones = 3, float duration = 0.5f, float volume = 0.5f)
        {
            overtones = Mathf.Clamp(overtones, 1, 6);
            var samples = Chime(baseFrequency, overtones, duration, volume);
            var name = $"chime_{FreqTag(baseFrequency)}_o{overtones}";
            var path = Path.Combine(SfxRoot, name + ".wav").Replace('\\', '/');
            WriteWavAndSidecar(path, samples, new Dictionary<string, object>
            {
                { "generator", "Chime" },
                { "baseFrequency", baseFrequency },
                { "overtones", overtones },
                { "duration", duration },
                { "volume", volume },
            }, seed: null);
            return path;
        }

        public static string GenerateSweep(float startFreq, float endFreq, float duration, float volume = 0.6f)
        {
            var samples = Sweep(startFreq, endFreq, duration, volume);
            var name = $"sweep_{FreqTag(startFreq)}to{FreqTag(endFreq)}_{DurTag(duration)}";
            var path = Path.Combine(SfxRoot, name + ".wav").Replace('\\', '/');
            WriteWavAndSidecar(path, samples, new Dictionary<string, object>
            {
                { "generator", "Sweep" },
                { "startFreq", startFreq },
                { "endFreq", endFreq },
                { "duration", duration },
                { "volume", volume },
            }, seed: null);
            return path;
        }

        public static string GenerateBGM(string genre = "ambient", float duration = 8f, int? seed = null)
        {
            int usedSeed = seed ?? new System.Random().Next();
            var samples = Bgm(genre, duration, usedSeed);
            var name = $"bgm_{genre}_{DurTag(duration)}";
            var path = Path.Combine(BgmRoot, name + ".wav").Replace('\\', '/');
            WriteWavAndSidecar(path, samples, new Dictionary<string, object>
            {
                { "generator", "BGM" },
                { "genre", genre },
                { "duration", duration },
            }, seed: usedSeed);
            return path;
        }

        // -----------------------------------------------------------------
        // Config-driven dispatch — lets Claude batch-generate without
        // opening parameter windows. Reads /tmp/audio-gen.json:
        // {
        //   "generator": "Beep",
        //   "frequency": 440, "duration": 0.15
        // }
        // -----------------------------------------------------------------

        public static void GenerateFromConfig(string configPath)
        {
            if (!File.Exists(configPath))
            {
                Debug.LogWarning($"[AudioGenerator] No config at {configPath}");
                return;
            }
            var json = File.ReadAllText(configPath);
            var cfg = JsonUtility.FromJson<GenConfig>(json);
            if (cfg == null || string.IsNullOrEmpty(cfg.generator))
            {
                Debug.LogWarning($"[AudioGenerator] Invalid config at {configPath}");
                return;
            }
            string produced = null;
            switch (cfg.generator.ToLowerInvariant())
            {
                case "beep":   produced = GenerateBeep(cfg.frequency > 0 ? cfg.frequency : 440f, cfg.duration > 0 ? cfg.duration : 0.15f, cfg.volume > 0 ? cfg.volume : 0.8f, cfg.decay > 0 ? cfg.decay : 5f); break;
                case "noise":  produced = GenerateNoise(cfg.duration > 0 ? cfg.duration : 0.3f, string.IsNullOrEmpty(cfg.color) ? "white" : cfg.color, cfg.volume > 0 ? cfg.volume : 0.6f); break;
                case "impact": produced = GenerateImpact(cfg.intensity > 0 ? cfg.intensity : 0.8f); break;
                case "chime":  produced = GenerateChime(cfg.frequency > 0 ? cfg.frequency : 880f, cfg.overtones > 0 ? cfg.overtones : 3, cfg.duration > 0 ? cfg.duration : 0.5f, cfg.volume > 0 ? cfg.volume : 0.5f); break;
                case "sweep":  produced = GenerateSweep(cfg.startFreq > 0 ? cfg.startFreq : 200f, cfg.endFreq > 0 ? cfg.endFreq : 1200f, cfg.duration > 0 ? cfg.duration : 0.5f, cfg.volume > 0 ? cfg.volume : 0.6f); break;
                case "bgm":    produced = GenerateBGM(string.IsNullOrEmpty(cfg.genre) ? "ambient" : cfg.genre, cfg.duration > 0 ? cfg.duration : 8f); break;
                default:       Debug.LogWarning($"[AudioGenerator] Unknown generator: {cfg.generator}"); return;
            }
            Debug.Log($"[AudioGenerator] Generated {produced}");
        }

        [Serializable]
        private class GenConfig
        {
            public string generator;
            public float frequency;
            public float duration;
            public float volume;
            public float decay;
            public string color;
            public float intensity;
            public int overtones;
            public float startFreq;
            public float endFreq;
            public string genre;
        }

        // -----------------------------------------------------------------
        // DSP generators — pure-math; no Unity API calls here.
        // -----------------------------------------------------------------

        private static float[] Beep(float freq, float duration, float volume, float decay)
        {
            int n = (int)(SampleRate * duration);
            var data = new float[n];
            for (int i = 0; i < n; i++)
            {
                float t = (float)i / SampleRate;
                float envelope = Mathf.Exp(-decay * t);
                data[i] = Mathf.Sin(2f * Mathf.PI * freq * t) * volume * envelope;
            }
            return data;
        }

        private static float[] Noise(float duration, string color, float volume, int seed)
        {
            int n = (int)(SampleRate * duration);
            var data = new float[n];
            var rng = new System.Random(seed);

            // Pink / brown via simple IIR running-sum filters. Not
            // psychoacoustically exact, but close enough for placeholders.
            float pinkPrev = 0f;
            float brownPrev = 0f;
            for (int i = 0; i < n; i++)
            {
                float white = (float)(rng.NextDouble() * 2.0 - 1.0);
                float s;
                switch ((color ?? "white").ToLowerInvariant())
                {
                    case "pink":
                        pinkPrev = 0.98f * pinkPrev + 0.1f * white;
                        s = pinkPrev * 3.5f;
                        break;
                    case "brown":
                    case "red":
                        brownPrev = (brownPrev + 0.02f * white) / 1.02f;
                        s = brownPrev * 3.5f;
                        break;
                    default:
                        s = white;
                        break;
                }
                data[i] = Mathf.Clamp(s * volume, -1f, 1f);
            }
            return data;
        }

        private static float[] Impact(float intensity, int seed)
        {
            // Impact = short burst of low-passed noise + sub-frequency sine
            // "thump" decaying fast. Intensity scales both duration and
            // low-pass cutoff, so more intense = bigger/beefier.
            float duration = Mathf.Lerp(0.15f, 0.6f, Mathf.InverseLerp(0.1f, 1.5f, intensity));
            float thumpFreq = Mathf.Lerp(60f, 180f, intensity);
            float filterCutoff = Mathf.Lerp(200f, 1200f, intensity);

            int n = (int)(SampleRate * duration);
            var data = new float[n];
            var rng = new System.Random(seed);
            float prev = 0f;
            float dt = 1f / SampleRate;
            float rc = 1f / (2f * Mathf.PI * filterCutoff);
            float alpha = dt / (rc + dt);

            for (int i = 0; i < n; i++)
            {
                float t = (float)i / SampleRate;
                float envelope = Mathf.Exp(-6f * t);
                float white = (float)(rng.NextDouble() * 2.0 - 1.0);
                prev = prev + alpha * (white - prev);
                float thump = Mathf.Sin(2f * Mathf.PI * thumpFreq * t) * 0.6f;
                data[i] = Mathf.Clamp((prev * 1.8f + thump) * intensity * envelope, -1f, 1f);
            }
            return data;
        }

        private static float[] Chime(float baseFreq, int overtones, float duration, float volume)
        {
            int n = (int)(SampleRate * duration);
            var data = new float[n];

            // Overtone stack: integer-multiple partials with decreasing volume
            // and slightly-faster decay — approximates a bell-ish timbre.
            for (int o = 1; o <= overtones; o++)
            {
                float freq = baseFreq * o;
                float partialVol = volume / o;
                float decay = 3f + o;
                for (int i = 0; i < n; i++)
                {
                    float t = (float)i / SampleRate;
                    float env = Mathf.Exp(-decay * t);
                    data[i] += Mathf.Sin(2f * Mathf.PI * freq * t) * partialVol * env;
                }
            }
            // Normalize so additive overtones don't clip.
            float peak = 0f;
            for (int i = 0; i < n; i++) peak = Mathf.Max(peak, Mathf.Abs(data[i]));
            if (peak > 0.95f) { float scale = 0.95f / peak; for (int i = 0; i < n; i++) data[i] *= scale; }
            return data;
        }

        private static float[] Sweep(float startFreq, float endFreq, float duration, float volume)
        {
            int n = (int)(SampleRate * duration);
            var data = new float[n];
            // Phase-accumulator sweep — linear interpolation of freq produces
            // audible zipper artifacts if you sin(2*PI*freq*t) naively because
            // instantaneous frequency disagrees with phase. Accumulate phase
            // each sample instead.
            float phase = 0f;
            for (int i = 0; i < n; i++)
            {
                float t = (float)i / SampleRate;
                float progress = t / duration;
                float freq = Mathf.Lerp(startFreq, endFreq, progress);
                phase += 2f * Mathf.PI * freq / SampleRate;
                float envelope = Mathf.Sin(Mathf.PI * progress);  // fade in + out
                data[i] = Mathf.Sin(phase) * volume * envelope;
            }
            return data;
        }

        private static float[] Bgm(string genre, float duration, int seed)
        {
            int n = (int)(SampleRate * duration);
            var data = new float[n];
            var rng = new System.Random(seed);

            // Pick scale + timbre per genre.
            float[] scale;
            float bassMul;
            bool useSquare;
            switch ((genre ?? "ambient").ToLowerInvariant())
            {
                case "chiptune":
                    scale = new[] { 261.63f, 329.63f, 392f, 523.25f, 392f, 329.63f, 440f, 392f }; // C E G C G E A G
                    bassMul = 0.5f;
                    useSquare = true;
                    break;
                case "dungeon":
                    scale = new[] { 220f, 261.63f, 293.66f, 349.23f, 329.63f, 261.63f, 293.66f, 220f }; // A minor feel
                    bassMul = 0.5f;
                    useSquare = false;
                    break;
                case "ambient":
                default:
                    scale = new[] { 261.63f, 329.63f, 392f, 329.63f, 349.23f, 392f, 440f, 392f };
                    bassMul = 0.5f;
                    useSquare = false;
                    break;
            }

            float beatDur = duration / scale.Length;

            for (int i = 0; i < n; i++)
            {
                float t = (float)i / SampleRate;
                int beat = Mathf.Min((int)(t / beatDur), scale.Length - 1);
                float beatPos = (t - beat * beatDur) / beatDur;

                float melodyFreq = scale[beat];
                float melodyEnv = 0.3f * Mathf.Clamp01(1f - beatPos * 1.2f);
                float melodyVal = useSquare
                    ? (Mathf.Sin(2f * Mathf.PI * melodyFreq * t) >= 0 ? 1f : -1f) * melodyEnv
                    : Mathf.Sin(2f * Mathf.PI * melodyFreq * t) * melodyEnv;

                float bassFreq = melodyFreq * bassMul;
                float bassVal = Mathf.Sin(2f * Mathf.PI * bassFreq * t) * 0.15f;

                // Simple kick on beat 0 of each beat via short noise burst.
                float perc = 0f;
                float beatLocal = t % beatDur;
                if (beatLocal < 0.05f)
                {
                    float pn = (float)(rng.NextDouble() * 2.0 - 1.0);
                    perc = pn * 0.15f * (1f - beatLocal / 0.05f);
                }

                data[i] = Mathf.Clamp(melodyVal + bassVal + perc, -0.95f, 0.95f);
            }
            return data;
        }

        // -----------------------------------------------------------------
        // WAV writer + sidecar
        // -----------------------------------------------------------------

        private static void WriteWavAndSidecar(string path, float[] samples, Dictionary<string, object> paramsMap, int? seed)
        {
            EnsureDir(Path.GetDirectoryName(path));
            WriteWav(path, samples);

            var sidecarPath = path + ".meta.json";
            var sb = new System.Text.StringBuilder();
            sb.Append("{\n");
            sb.AppendLine($"  \"generator\": \"{paramsMap["generator"]}\",");
            sb.AppendLine($"  \"filename\": \"{Path.GetFileName(path)}\",");
            sb.AppendLine($"  \"sampleRate\": {SampleRate},");
            sb.AppendLine($"  \"channels\": {Channels},");
            sb.AppendLine($"  \"bitsPerSample\": {BitsPerSample},");
            sb.AppendLine($"  \"generatedAtUtc\": \"{DateTime.UtcNow.ToString("o")}\",");
            sb.AppendLine($"  \"generatorVersion\": {GeneratorVersion},");
            sb.AppendLine($"  \"seed\": {(seed.HasValue ? seed.Value.ToString() : "null")},");
            sb.Append("  \"params\": {\n");
            var keys = new List<string>(paramsMap.Keys); keys.Remove("generator");
            for (int i = 0; i < keys.Count; i++)
            {
                var k = keys[i];
                var v = paramsMap[k];
                string vs = v is float f ? f.ToString("0.0####", CultureInfo.InvariantCulture)
                           : v is int iv ? iv.ToString(CultureInfo.InvariantCulture)
                           : v is bool bv ? (bv ? "true" : "false")
                           : $"\"{v}\"";
                sb.Append("    \"").Append(k).Append("\": ").Append(vs);
                if (i < keys.Count - 1) sb.Append(",");
                sb.Append("\n");
            }
            sb.AppendLine("  }");
            sb.AppendLine("}");
            File.WriteAllText(sidecarPath, sb.ToString());

            AssetDatabase.Refresh();
            Debug.Log($"[AudioGenerator] Wrote {path} ({samples.Length / (float)SampleRate:0.00}s)");
        }

        private static void EnsureDir(string dir)
        {
            if (!Directory.Exists(dir)) Directory.CreateDirectory(dir);
        }

        private static void WriteWav(string path, float[] samples)
        {
            using (var stream = new FileStream(path, FileMode.Create))
            using (var writer = new BinaryWriter(stream))
            {
                int byteRate = SampleRate * Channels * BitsPerSample / 8;
                int blockAlign = Channels * BitsPerSample / 8;
                int dataSize = samples.Length * blockAlign;

                // RIFF
                writer.Write(new[] { 'R', 'I', 'F', 'F' });
                writer.Write(36 + dataSize);
                writer.Write(new[] { 'W', 'A', 'V', 'E' });

                // fmt
                writer.Write(new[] { 'f', 'm', 't', ' ' });
                writer.Write(16);
                writer.Write((short)1); // PCM
                writer.Write((short)Channels);
                writer.Write(SampleRate);
                writer.Write(byteRate);
                writer.Write((short)blockAlign);
                writer.Write((short)BitsPerSample);

                // data
                writer.Write(new[] { 'd', 'a', 't', 'a' });
                writer.Write(dataSize);
                for (int i = 0; i < samples.Length; i++)
                {
                    // Clamp hard — a stray float outside [-1,1] becomes wrap-around
                    // noise when cast, not just clipping. 16-bit range is ±32767.
                    float clamped = Mathf.Clamp(samples[i], -1f, 1f);
                    writer.Write((short)(clamped * 32767f));
                }
            }
        }

        // Filename helpers — keep generated names stable and human-readable.
        private static string FreqTag(float hz) => $"{(int)hz}hz";
        private static string DurTag(float s) => $"{s.ToString("0.00", CultureInfo.InvariantCulture).Replace('.', 'p')}s";

        // -----------------------------------------------------------------
        // Parameter windows — interactive path for human use in the Editor.
        // Each is a tiny EditorWindow; they exist so a human in the Editor
        // can tweak params visually. Claude's agentic path should use the
        // public static API or GenerateFromConfig instead.
        // -----------------------------------------------------------------

        private class BeepWindow : EditorWindow
        {
            private float frequency = 440f;
            private float duration = 0.15f;
            private float volume = 0.8f;
            private float decay = 5f;
            public static void Open() => GetWindow<BeepWindow>("Generate Beep").minSize = new Vector2(300, 140);
            void OnGUI()
            {
                frequency = EditorGUILayout.Slider("Frequency (Hz)", frequency, 20f, 8000f);
                duration = EditorGUILayout.Slider("Duration (s)", duration, 0.01f, 3f);
                volume = EditorGUILayout.Slider("Volume", volume, 0f, 1f);
                decay = EditorGUILayout.Slider("Decay", decay, 0f, 30f);
                if (GUILayout.Button("Generate")) { GenerateBeep(frequency, duration, volume, decay); Close(); }
            }
        }

        private class NoiseWindow : EditorWindow
        {
            private float duration = 0.3f;
            private int colorIdx = 0;
            private float volume = 0.6f;
            private readonly string[] colors = { "white", "pink", "brown" };
            public static void Open() => GetWindow<NoiseWindow>("Generate Noise").minSize = new Vector2(300, 140);
            void OnGUI()
            {
                duration = EditorGUILayout.Slider("Duration (s)", duration, 0.01f, 3f);
                colorIdx = EditorGUILayout.Popup("Color", colorIdx, colors);
                volume = EditorGUILayout.Slider("Volume", volume, 0f, 1f);
                if (GUILayout.Button("Generate")) { GenerateNoise(duration, colors[colorIdx], volume); Close(); }
            }
        }

        private class ImpactWindow : EditorWindow
        {
            private float intensity = 0.8f;
            public static void Open() => GetWindow<ImpactWindow>("Generate Impact").minSize = new Vector2(300, 100);
            void OnGUI()
            {
                intensity = EditorGUILayout.Slider("Intensity", intensity, 0.1f, 1.5f);
                if (GUILayout.Button("Generate")) { GenerateImpact(intensity); Close(); }
            }
        }

        private class ChimeWindow : EditorWindow
        {
            private float baseFreq = 880f;
            private int overtones = 3;
            private float duration = 0.5f;
            private float volume = 0.5f;
            public static void Open() => GetWindow<ChimeWindow>("Generate Chime").minSize = new Vector2(300, 140);
            void OnGUI()
            {
                baseFreq = EditorGUILayout.Slider("Base Frequency (Hz)", baseFreq, 100f, 2000f);
                overtones = EditorGUILayout.IntSlider("Overtones", overtones, 1, 6);
                duration = EditorGUILayout.Slider("Duration (s)", duration, 0.05f, 2f);
                volume = EditorGUILayout.Slider("Volume", volume, 0f, 1f);
                if (GUILayout.Button("Generate")) { GenerateChime(baseFreq, overtones, duration, volume); Close(); }
            }
        }

        private class SweepWindow : EditorWindow
        {
            private float startFreq = 200f;
            private float endFreq = 1200f;
            private float duration = 0.5f;
            private float volume = 0.6f;
            public static void Open() => GetWindow<SweepWindow>("Generate Sweep").minSize = new Vector2(300, 140);
            void OnGUI()
            {
                startFreq = EditorGUILayout.Slider("Start Freq (Hz)", startFreq, 20f, 4000f);
                endFreq = EditorGUILayout.Slider("End Freq (Hz)", endFreq, 20f, 4000f);
                duration = EditorGUILayout.Slider("Duration (s)", duration, 0.05f, 3f);
                volume = EditorGUILayout.Slider("Volume", volume, 0f, 1f);
                if (GUILayout.Button("Generate")) { GenerateSweep(startFreq, endFreq, duration, volume); Close(); }
            }
        }

        private class BgmWindow : EditorWindow
        {
            private int genreIdx = 0;
            private float duration = 8f;
            private readonly string[] genres = { "ambient", "chiptune", "dungeon" };
            public static void Open() => GetWindow<BgmWindow>("Generate BGM").minSize = new Vector2(300, 120);
            void OnGUI()
            {
                genreIdx = EditorGUILayout.Popup("Genre", genreIdx, genres);
                duration = EditorGUILayout.Slider("Duration (s)", duration, 2f, 30f);
                if (GUILayout.Button("Generate")) { GenerateBGM(genres[genreIdx], duration); Close(); }
            }
        }
    }
}
#endif

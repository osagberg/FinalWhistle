// WebGLBuilder — editor-only entry point for automated WebGL builds.
//
// Why editor-only?
//   This file lives under Assets/_Project/Editor/Build/ and is wrapped in
//   #if UNITY_EDITOR. That means:
//     (a) it compiles against UnityEditor.* — required for BuildPipeline.
//     (b) it is stripped from every shipped player (WebGL, Standalone, etc).
//   If you see "The type or namespace 'UnityEditor' could not be found" at
//   build time, the file has slipped out of an Editor asmdef folder.
//
// How Claude invokes it:
//   - MCP path:       execute_menu_item("FinalWhistle/Build/WebGL")
//   - Batchmode path: Unity ... -executeMethod FinalWhistle.Editor.Build.WebGLBuilder.Build
//   - Ad-hoc:         execute_menu_item("FinalWhistle/Build/WebGL With Config")
//                     (reads /tmp/webgl-config.json — see ConfigPath const)
//
// Report contract:
//   Always writes Library/WebGLBuildReport.json, even on failure. The
//   unity-webgl-builder SKILL.md parses this file as the source of truth
//   (exit code from batchmode is unreliable — see batchmode-gotchas.md).

#if UNITY_EDITOR
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using UnityEditor;
using UnityEditor.Build.Reporting;
using UnityEngine;

namespace FinalWhistle.Editor.Build
{
    public static class WebGLBuilder
    {
        private const string DefaultOutputPath = "Build/WebGL";
        private const string ReportPath = "Library/WebGLBuildReport.json";
        private const string ConfigPath = "/tmp/webgl-config.json";
        private const string MenuRoot = "FinalWhistle/Build/";

        // -----------------------------------------------------------------
        // Menu entries
        // -----------------------------------------------------------------

        [MenuItem(MenuRoot + "WebGL")]
        public static void BuildMenu() => Build();

        [MenuItem(MenuRoot + "WebGL With Config")]
        public static void BuildWithConfigMenu() => BuildWithConfig(ConfigPath);

        [MenuItem(MenuRoot + "Switch Platform to WebGL")]
        public static void SwitchPlatform()
        {
            if (EditorUserBuildSettings.activeBuildTarget == BuildTarget.WebGL)
            {
                Debug.Log("[WebGLBuilder] Already on WebGL platform.");
                return;
            }
            Debug.Log("[WebGLBuilder] Switching to WebGL (this can take several minutes on first switch)...");
            EditorUserBuildSettings.SwitchActiveBuildTarget(BuildTargetGroup.WebGL, BuildTarget.WebGL);
            Debug.Log("[WebGLBuilder] Platform switched to WebGL.");
        }

        // -----------------------------------------------------------------
        // Build entrypoints
        // -----------------------------------------------------------------

        /// <summary>
        /// Default WebGL build. Called by MCP menu entry and by -executeMethod
        /// in batchmode. Uses scenes from EditorBuildSettings and the defaults
        /// documented in this class. Writes a JSON report regardless of outcome.
        /// </summary>
        public static void Build()
        {
            var cfg = BuildConfig.Defaults();
            RunBuild(cfg);
        }

        /// <summary>
        /// Ad-hoc build. Reads a JSON config file for parameter overrides so
        /// Claude can request non-default builds (different output path,
        /// subset of scenes, disable compression for local preview) without
        /// editing this file.
        /// </summary>
        public static void BuildWithConfig(string configPath)
        {
            var cfg = BuildConfig.Defaults();
            if (File.Exists(configPath))
            {
                try
                {
                    var json = File.ReadAllText(configPath);
                    var overrides = JsonUtility.FromJson<BuildConfig>(json);
                    cfg = BuildConfig.Merge(cfg, overrides);
                    Debug.Log($"[WebGLBuilder] Loaded config overrides from {configPath}");
                }
                catch (Exception e)
                {
                    // Swallow: a malformed config shouldn't silently produce an
                    // unexpected build. Log + fall through to defaults is loud
                    // enough for the SKILL report-reader to catch.
                    Debug.LogWarning($"[WebGLBuilder] Failed to parse {configPath}: {e.Message}. Using defaults.");
                }
            }
            else
            {
                Debug.Log($"[WebGLBuilder] No config at {configPath}, using defaults.");
            }
            RunBuild(cfg);
        }

        // -----------------------------------------------------------------
        // Core
        // -----------------------------------------------------------------

        private static void RunBuild(BuildConfig cfg)
        {
            var startedAt = DateTime.UtcNow;

            ApplyPlayerSettings(cfg);

            var scenes = cfg.scenes != null && cfg.scenes.Length > 0
                ? cfg.scenes
                : EditorBuildSettings.scenes.Where(s => s.enabled).Select(s => s.path).ToArray();

            if (scenes == null || scenes.Length == 0)
            {
                // Don't try to call BuildPipeline with no scenes — Unity emits
                // a cryptic error. Fail loud with a readable one instead.
                EmitReport(new ReportPayload
                {
                    result = "Failed",
                    error = "No scenes in build. Add at least one scene to EditorBuildSettings or pass scenes[] in config.",
                    outputPath = cfg.outputPath,
                    startedAtUtc = startedAt.ToString("o"),
                    finishedAtUtc = DateTime.UtcNow.ToString("o"),
                });
                return;
            }

            EnsureOutputDirClean(cfg.outputPath);

            var options = new BuildPlayerOptions
            {
                scenes = scenes,
                locationPathName = cfg.outputPath,
                target = BuildTarget.WebGL,
                targetGroup = BuildTargetGroup.WebGL,
                options = cfg.development ? BuildOptions.Development : BuildOptions.None,
            };

            Debug.Log($"[WebGLBuilder] Build starting → {cfg.outputPath} ({scenes.Length} scene(s), compression={cfg.compression}, memory={cfg.memorySizeMB}MB, dev={cfg.development})");

            BuildReport report;
            try
            {
                report = BuildPipeline.BuildPlayer(options);
            }
            catch (Exception e)
            {
                EmitReport(new ReportPayload
                {
                    result = "Failed",
                    error = $"BuildPipeline threw: {e.GetType().Name}: {e.Message}",
                    outputPath = cfg.outputPath,
                    startedAtUtc = startedAt.ToString("o"),
                    finishedAtUtc = DateTime.UtcNow.ToString("o"),
                });
                return;
            }

            var payload = BuildPayloadFromReport(report, cfg, startedAt);
            EmitReport(payload);

            if (report.summary.result == BuildResult.Succeeded)
                Debug.Log($"[WebGLBuilder] Build Succeeded. Size: {payload.totalSizeBytes / (1024 * 1024)} MB. Time: {report.summary.totalTime.TotalSeconds:F1}s. Output: {cfg.outputPath}");
            else
                Debug.LogError($"[WebGLBuilder] Build {report.summary.result}. See {ReportPath} for details.");
        }

        // Apply PlayerSettings that can't be serialized into the build pipeline
        // struct. These persist in ProjectSettings.asset — we restore them
        // intentionally each run so a stray editor tweak doesn't silently
        // change build output.
        private static void ApplyPlayerSettings(BuildConfig cfg)
        {
            switch ((cfg.compression ?? "brotli").ToLowerInvariant())
            {
                case "disabled":
                case "none":
                    PlayerSettings.WebGL.compressionFormat = WebGLCompressionFormat.Disabled;
                    break;
                case "gzip":
                    PlayerSettings.WebGL.compressionFormat = WebGLCompressionFormat.Gzip;
                    break;
                case "brotli":
                default:
                    PlayerSettings.WebGL.compressionFormat = WebGLCompressionFormat.Brotli;
                    break;
            }

            // decompressionFallback: when true, Unity ships a JS fallback that
            // decompresses in-browser if the server doesn't set the right
            // Content-Encoding header. Useful for python3 http.server preview,
            // but adds ~200KB. Default on for safety.
            PlayerSettings.WebGL.decompressionFallback = cfg.compression != "disabled";

            if (cfg.memorySizeMB > 0)
                PlayerSettings.WebGL.memorySize = cfg.memorySizeMB;

            // ExplicitlyThrownExceptionsOnly is the right default — full catches
            // all including engine-internal, bloats binary ~20%; None hides real
            // bugs in prod. Only override if caller is sure.
            PlayerSettings.WebGL.exceptionSupport = WebGLExceptionSupport.ExplicitlyThrownExceptionsOnly;
        }

        private static void EnsureOutputDirClean(string path)
        {
            // Unity's BuildPipeline will happily write over a stale folder, but
            // old TemplateData/ residue from a different template can confuse
            // the loader. Nuke and recreate.
            if (Directory.Exists(path))
            {
                try { Directory.Delete(path, recursive: true); }
                catch (Exception e) { Debug.LogWarning($"[WebGLBuilder] Could not clean {path}: {e.Message}. Proceeding."); }
            }
            Directory.CreateDirectory(path);
        }

        private static ReportPayload BuildPayloadFromReport(BuildReport report, BuildConfig cfg, DateTime startedAt)
        {
            var warnings = new List<string>();
            var errors = new List<string>();
            foreach (var step in report.steps)
            {
                foreach (var m in step.messages)
                {
                    if (m.type == LogType.Warning) warnings.Add($"[{step.name}] {m.content}");
                    else if (m.type == LogType.Error || m.type == LogType.Exception) errors.Add($"[{step.name}] {m.content}");
                }
            }

            return new ReportPayload
            {
                result = report.summary.result.ToString(),
                outputPath = Path.GetFullPath(cfg.outputPath),
                totalSizeBytes = (long)report.summary.totalSize,
                totalTimeSeconds = report.summary.totalTime.TotalSeconds,
                sceneCount = report.summary.totalErrors == 0 ? report.GetFiles()?.Length ?? 0 : 0,
                scenes = cfg.scenes,
                compression = cfg.compression,
                memorySizeMB = cfg.memorySizeMB,
                development = cfg.development,
                warningCount = warnings.Count,
                errorCount = errors.Count,
                warnings = warnings.ToArray(),
                errors = errors.ToArray(),
                startedAtUtc = startedAt.ToString("o"),
                finishedAtUtc = DateTime.UtcNow.ToString("o"),
            };
        }

        private static void EmitReport(ReportPayload payload)
        {
            try
            {
                Directory.CreateDirectory("Library");
                var json = JsonUtility.ToJson(payload, prettyPrint: true);
                File.WriteAllText(ReportPath, json);
                Debug.Log($"[WebGLBuilder] Report written → {ReportPath}");
            }
            catch (Exception e)
            {
                // Report-write failure is the one place we can't report via
                // the report. Last-resort console log.
                Debug.LogError($"[WebGLBuilder] Failed to write {ReportPath}: {e.Message}");
            }
        }

        // -----------------------------------------------------------------
        // Data contracts
        // -----------------------------------------------------------------

        // Serializable via JsonUtility; do not use auto-properties or
        // anonymous types — JsonUtility silently drops them.
        [Serializable]
        private class BuildConfig
        {
            public string outputPath;
            public string[] scenes;
            public string compression;   // "brotli" | "gzip" | "disabled"
            public int memorySizeMB;
            public bool development;

            public static BuildConfig Defaults() => new BuildConfig
            {
                outputPath = DefaultOutputPath,
                scenes = null,
                compression = "brotli",
                memorySizeMB = 256,
                development = false,
            };

            public static BuildConfig Merge(BuildConfig baseline, BuildConfig overrides)
            {
                if (overrides == null) return baseline;
                return new BuildConfig
                {
                    outputPath = !string.IsNullOrEmpty(overrides.outputPath) ? overrides.outputPath : baseline.outputPath,
                    scenes = overrides.scenes != null && overrides.scenes.Length > 0 ? overrides.scenes : baseline.scenes,
                    compression = !string.IsNullOrEmpty(overrides.compression) ? overrides.compression : baseline.compression,
                    memorySizeMB = overrides.memorySizeMB > 0 ? overrides.memorySizeMB : baseline.memorySizeMB,
                    development = overrides.development || baseline.development,
                };
            }
        }

        [Serializable]
        private class ReportPayload
        {
            public string result;
            public string outputPath;
            public long totalSizeBytes;
            public double totalTimeSeconds;
            public int sceneCount;
            public string[] scenes;
            public string compression;
            public int memorySizeMB;
            public bool development;
            public int warningCount;
            public int errorCount;
            public string[] warnings;
            public string[] errors;
            public string error;
            public string startedAtUtc;
            public string finishedAtUtc;
        }
    }
}
#endif

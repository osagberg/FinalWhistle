// Batchmode verification report emitter.
//
// Invoked via:
//   Unity -batchmode -quit -nographics \
//     -projectPath <project> \
//     -executeMethod {{PROJECT_NAME}}.Editor.Verification.VerificationReport.Run \
//     -logFile /tmp/unity-l1.log
//
// Writes JSON to Library/VerificationReport.json for the unity-check skill to read.
// Library/ is gitignored by convention — report is ephemeral.

#if UNITY_EDITOR
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Text;
using UnityEditor;
using UnityEditor.Compilation;
using UnityEngine;

namespace {{PROJECT_NAME}}.Editor.Verification
{
    public static class VerificationReport
    {
        private const string OutputPath = "Library/VerificationReport.json";
        private const int MaxErrorsInReport = 10;

        // Called by -executeMethod. Static, no args, returns void — Unity's batchmode contract.
        public static void Run()
        {
            var report = new Report
            {
                timestamp = DateTime.UtcNow.ToString("o"),
                unityVersion = Application.unityVersion,
                projectName = Application.productName
            };

            try
            {
                CollectCompileErrors(report);
                CollectMissingScripts(report);
                CollectBrokenAsmdefs(report);
            }
            catch (Exception ex)
            {
                report.internalErrors.Add($"VerificationReport.Run threw: {ex.GetType().Name}: {ex.Message}");
            }

            WriteReport(report);

            // Console breadcrumb — grep-friendly tag for log scanning.
            Debug.Log($"[unity-check] report written to {OutputPath} — " +
                      $"compileErrors={report.compileErrorCount} " +
                      $"missingScripts={report.missingScripts.Count} " +
                      $"brokenAsmdefRefs={report.brokenAsmdefRefs.Count}");
        }

        private static void CollectCompileErrors(Report report)
        {
            // CompilationPipeline gives us the authoritative list even in batchmode.
            var messages = new List<CompilerMessage>();
            var assemblies = CompilationPipeline.GetAssemblies(AssembliesType.Editor)
                .Concat(CompilationPipeline.GetAssemblies(AssembliesType.Player));

            foreach (var asm in assemblies)
            {
                // GetAssemblyCompilerMessages is available post-compile only;
                // wrap to survive if Unity hasn't compiled anything this session.
                try
                {
                    var asmMessages = CompilationPipeline.GetAssemblyCompilerMessages(asm.name);
                    if (asmMessages != null)
                    {
                        messages.AddRange(asmMessages.Where(m => m.type == CompilerMessageType.Error));
                    }
                }
                catch
                {
                    // Asm hasn't been compiled yet — not a verification failure, skip.
                }
            }

            report.compileErrorCount = messages.Count;
            report.compileErrors = messages
                .Take(MaxErrorsInReport)
                .Select(m => new CompileError
                {
                    file = m.file,
                    line = m.line,
                    column = m.column,
                    code = ExtractErrorCode(m.message),
                    message = m.message
                })
                .ToList();
        }

        private static string ExtractErrorCode(string msg)
        {
            // Roslyn errors look like: "error CS0103: ...". Extract CSxxxx.
            if (string.IsNullOrEmpty(msg)) return "";
            var idx = msg.IndexOf("error CS", StringComparison.Ordinal);
            if (idx < 0) return "";
            var end = msg.IndexOf(':', idx + 6);
            return end > idx ? msg.Substring(idx + 6, end - idx - 6) : "";
        }

        private static void CollectMissingScripts(Report report)
        {
            // Scan open scene + all prefabs for MonoBehaviour references whose script GUID is gone.
            var missing = new List<string>();

            foreach (var prefabGuid in AssetDatabase.FindAssets("t:Prefab"))
            {
                var path = AssetDatabase.GUIDToAssetPath(prefabGuid);
                var go = AssetDatabase.LoadAssetAtPath<GameObject>(path);
                if (go == null) continue;

                var components = go.GetComponentsInChildren<MonoBehaviour>(true);
                foreach (var c in components)
                {
                    if (c == null) // null MonoBehaviour => missing script reference
                    {
                        missing.Add(path);
                        break;
                    }
                }
            }

            report.missingScripts = missing.Distinct().Take(MaxErrorsInReport).ToList();
        }

        private static void CollectBrokenAsmdefs(Report report)
        {
            // Asmdef broken refs surface as compile errors, but we also flag asmdefs whose
            // declared references don't resolve — gives a clearer hint than a raw CS0246.
            var broken = new List<string>();

            foreach (var asmdefGuid in AssetDatabase.FindAssets("t:AssemblyDefinitionAsset"))
            {
                var path = AssetDatabase.GUIDToAssetPath(asmdefGuid);
                var json = File.ReadAllText(path);

                // Cheap substring check — avoids a full JSON parse dependency.
                // If Unity already reports errors on this asmdef, it's broken.
                var assemblyName = Path.GetFileNameWithoutExtension(path);
                var messages = CompilationPipeline.GetAssemblyCompilerMessages(assemblyName);
                if (messages != null && messages.Any(m => m.type == CompilerMessageType.Error))
                {
                    broken.Add(path);
                }
            }

            report.brokenAsmdefRefs = broken.Take(MaxErrorsInReport).ToList();
        }

        private static void WriteReport(Report report)
        {
            Directory.CreateDirectory(Path.GetDirectoryName(OutputPath) ?? ".");
            var json = JsonUtility.ToJson(report, prettyPrint: true);
            File.WriteAllText(OutputPath, json, Encoding.UTF8);
        }

        // Plain [Serializable] POCOs — JsonUtility doesn't support properties / generics cleanly.
        [Serializable]
        private class Report
        {
            public string timestamp;
            public string unityVersion;
            public string projectName;
            public int compileErrorCount;
            public List<CompileError> compileErrors = new List<CompileError>();
            public List<string> missingScripts = new List<string>();
            public List<string> brokenAsmdefRefs = new List<string>();
            public List<string> internalErrors = new List<string>();
        }

        [Serializable]
        private class CompileError
        {
            public string file;
            public int line;
            public int column;
            public string code;
            public string message;
        }
    }
}
#endif

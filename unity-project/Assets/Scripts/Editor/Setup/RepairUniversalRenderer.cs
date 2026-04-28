// Editor-only setup utility — fills nulls left by raw-CreateInstance renderer setup.
//
// When unity-project/Assets/Settings/UniversalRenderer.asset was first created via
// ScriptableObject.CreateInstance<UniversalRendererData>() (UnityMCP execute_code),
// the URP package's normal asset-factory ran neither ResourceReloader nor the
// post-process-data assignment — leaving postProcessData and a few other package
// resources null. This menu-item runs URP's own ResourceReloader pattern over
// the renderer asset(s) under Assets/Settings/ to populate those fields the way
// the in-Editor "Create → Rendering → URP Universal Renderer" menu would have.
//
// Idempotent: if all package fields are already non-null, this is a no-op aside
// from the Save/Reimport pass. Safe to run repeatedly. Intended to be run once
// after the project is opened post-repair-commit, then committed alongside the
// updated UniversalRenderer.asset.

#if UNITY_EDITOR
using System.Linq;
using UnityEditor;
using UnityEngine;
using UnityEngine.Rendering.Universal;

namespace FinalWhistle.Editor.Setup
{
    internal static class RepairUniversalRenderer
    {
        private const string MenuPath = "Final Whistle/Setup/Repair URP Renderer";
        private const string UrpPackageRoot = "Packages/com.unity.render-pipelines.universal";

        [MenuItem(MenuPath)]
        private static void Repair()
        {
            var rendererGuids = AssetDatabase.FindAssets(
                "t:UniversalRendererData",
                new[] { "Assets/Settings" });

            if (rendererGuids.Length == 0)
            {
                Debug.LogWarning("[RepairUniversalRenderer] No UniversalRendererData assets found under Assets/Settings/.");
                return;
            }

            var repaired = 0;
            foreach (var guid in rendererGuids)
            {
                var path = AssetDatabase.GUIDToAssetPath(guid);
                var data = AssetDatabase.LoadAssetAtPath<UniversalRendererData>(path);
                if (data == null) continue;

                // ResourceReloader is internal in URP — invoke via reflection
                // (Unity package internals are stable across the 17.x line; if
                // this breaks at a URP major version bump, the menu surfaces
                // an explicit error instead of a silent no-op).
                var coreUtilsAssembly = typeof(UnityEngine.Rendering.CoreUtils).Assembly;
                var reloaderType = coreUtilsAssembly.GetType("UnityEditor.Rendering.ResourceReloader");
                if (reloaderType == null)
                {
                    Debug.LogError("[RepairUniversalRenderer] UnityEditor.Rendering.ResourceReloader not found — URP package layout changed.");
                    return;
                }

                var reloadMethod = reloaderType.GetMethods()
                    .FirstOrDefault(m => m.Name == "ReloadAllNullIn"
                                         && m.GetParameters().Length == 2);
                if (reloadMethod == null)
                {
                    Debug.LogError("[RepairUniversalRenderer] ResourceReloader.ReloadAllNullIn(object, string) signature not found.");
                    return;
                }

                reloadMethod.Invoke(null, new object[] { data, UrpPackageRoot });
                EditorUtility.SetDirty(data);
                repaired++;
                Debug.Log($"[RepairUniversalRenderer] Repaired {path}");
            }

            AssetDatabase.SaveAssets();
            AssetDatabase.Refresh();
            Debug.Log($"[RepairUniversalRenderer] Done. {repaired} renderer asset(s) processed.");
        }
    }
}
#endif

// Editor-only setup utility — fills nulls left by raw-CreateInstance renderer setup.
//
// When unity-project/Assets/Settings/UniversalRenderer.asset was first created via
// ScriptableObject.CreateInstance<UniversalRendererData>() (UnityMCP execute_code),
// the URP package's normal asset-factory ran neither the explicit
// `data.postProcessData = PostProcessData.GetDefaultPostProcessData()` assignment
// nor `ResourceReloader.ReloadAllNullIn` — leaving postProcessData and several
// other package resources null. This menu-item replays URP's own two-step init
// over UniversalRendererData asset(s) under Assets/Settings/, mirroring what
// `UniversalRenderPipelineAsset.cs:749` and `UniversalRendererDataEditor.cs:235`
// do internally.
//
// Idempotent: if postProcessData is already assigned and the package resources
// are already non-null, this is a no-op aside from the Save/Reimport pass. Safe
// to run repeatedly. Intended to be run once after the project is opened
// post-repair-commit, then committed alongside the updated UniversalRenderer.asset.

#if UNITY_EDITOR
using System.Linq;
using UnityEditor;
using UnityEngine;
using UnityEngine.Rendering.Universal;
using UnityObject = UnityEngine.Object;

namespace FinalWhistle.Editor.Setup
{
    internal static class RepairUniversalRenderer
    {
        private const string MenuPath = "Final Whistle/Setup/Repair URP Renderer";
        private const string UrpPackageRoot = "Packages/com.unity.render-pipelines.universal";
        private const string PostProcessDataAssetPath = UrpPackageRoot + "/Runtime/Data/PostProcessData.asset";

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

            // PostProcessData is internal — resolve its System.Type via reflection
            // on the URP runtime assembly so we can use the non-generic
            // AssetDatabase.LoadAssetAtPath(string, Type) overload.
            var urpRuntimeAssembly = typeof(UniversalRendererData).Assembly;
            var postProcessDataType = urpRuntimeAssembly.GetType("UnityEngine.Rendering.Universal.PostProcessData");
            if (postProcessDataType == null)
            {
                Debug.LogError("[RepairUniversalRenderer] UnityEngine.Rendering.Universal.PostProcessData type not found — URP package layout changed.");
                return;
            }

            var defaultPostProcessData = AssetDatabase.LoadAssetAtPath(PostProcessDataAssetPath, postProcessDataType) as UnityObject;
            if (defaultPostProcessData == null)
            {
                Debug.LogError($"[RepairUniversalRenderer] Default PostProcessData asset not found at {PostProcessDataAssetPath}. Is the URP package installed?");
                return;
            }

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

            var repaired = 0;
            foreach (var guid in rendererGuids)
            {
                var path = AssetDatabase.GUIDToAssetPath(guid);
                var data = AssetDatabase.LoadAssetAtPath<UniversalRendererData>(path);
                if (data == null) continue;

                // Step 1: explicit postProcessData assignment (mirrors
                // UniversalRendererDataEditor.cs:235 — SerializedProperty path
                // ensures Unity's serialization sees the change).
                using (var so = new SerializedObject(data))
                {
                    var postProp = so.FindProperty("postProcessData");
                    if (postProp == null)
                    {
                        Debug.LogError($"[RepairUniversalRenderer] postProcessData SerializedProperty not found on {path} — URP serialization layout changed.");
                        return;
                    }

                    if (postProp.objectReferenceValue == null)
                    {
                        postProp.objectReferenceValue = defaultPostProcessData;
                        so.ApplyModifiedProperties();
                        Debug.Log($"[RepairUniversalRenderer] Assigned default PostProcessData to {path}");
                    }
                }

                // Step 2: reload remaining null package resources (debugShaders /
                // probeVolumeResources / etc.) via URP's own ResourceReloader.
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

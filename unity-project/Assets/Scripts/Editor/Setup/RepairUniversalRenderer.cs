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
using UnityEditor;
using UnityEngine;
using UnityEngine.Rendering;
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
            // AssetDatabase.LoadAssetAtPath(string, Type) overload to load the
            // package-default asset URP itself ships at the well-known path.
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
                // ResourceReloader is a public static class in the
                // UnityEngine.Rendering namespace (NOT UnityEditor.Rendering as
                // an earlier draft of this script assumed) and is gated behind
                // UNITY_EDITOR, so a direct compile-time call works.
                ResourceReloader.ReloadAllNullIn(data, UrpPackageRoot);

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

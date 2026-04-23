// McpRemoteControl — editor + dev-build entry points for the state-dump skill.
//
// Why conditional compilation?
//   Everything in here is debug-only. UNITY_EDITOR makes the menu items
//   appear in the editor; DEVELOPMENT_BUILD keeps the runtime helpers
//   available in "dev build" players for on-device debugging. Release/Steam
//   builds (DEVELOPMENT_BUILD undefined) strip the whole file — no debug
//   surface area in shipped binaries. This is non-negotiable for Steam
//   release hygiene.
//
// How Claude reads the output:
//   DumpState() writes `Library/StateDump.json`. The state-dump SKILL.md
//   tells Claude to Read that path after calling this via
//   `execute_menu_item("{{PROJECT_NAME}}/Debug/Dump State")`. JSON schema
//   is documented in state-dump/SKILL.md "Output JSON shape".
//
// How to add a new god-mode command:
//   1. Add a [MenuItem("{{PROJECT_NAME}}/Debug/<Name>")] static method below.
//   2. Inside, guard with `if (!Application.isPlaying) return;` for anything
//      that mutates runtime state.
//   3. Call into the relevant singleton / service (e.g. GameManager.Instance).
//   4. (Optional) Log the action via Debug.Log so it appears in read_console.
//
// How to extend IDumpable for a new component:
//   1. `public class MyThing : MonoBehaviour, IDumpable { ... }`
//   2. `public object DumpState() => new { a = ..., b = ... };`
//   3. Nothing else — the scanner finds it by reflection.

#if UNITY_EDITOR || DEVELOPMENT_BUILD
using System;
using System.Collections.Generic;
using System.IO;
using System.Linq;
using System.Reflection;
using UnityEngine;
using UnityEngine.SceneManagement;
using Newtonsoft.Json;

#if UNITY_EDITOR
using UnityEditor;
#endif

namespace {{PROJECT_NAME}}.Debug
{
    public static class McpRemoteControl
    {
        private const string MenuRoot = "{{PROJECT_NAME}}/Debug/";
        private const string DumpOutputPath = "Library/StateDump.json";

        // For parameterized commands via manage_components: set this field on
        // a scene-placed DebugBridge component and it'll be consumed next frame.
        public static string PendingCommand;

        // ---------------------------------------------------------------
        // State dump
        // ---------------------------------------------------------------

#if UNITY_EDITOR
        [MenuItem(MenuRoot + "Dump State")]
#endif
        public static void DumpState()
        {
            var snapshot = BuildSnapshot();
            var json = JsonConvert.SerializeObject(snapshot, Formatting.Indented,
                new JsonSerializerSettings
                {
                    // Scene references cycle; ignore to avoid infinite loops.
                    ReferenceLoopHandling = ReferenceLoopHandling.Ignore,
                    NullValueHandling = NullValueHandling.Include
                });

            Directory.CreateDirectory(Path.GetDirectoryName(DumpOutputPath) ?? ".");
            File.WriteAllText(DumpOutputPath, json);

            UnityEngine.Debug.Log($"[state-dump] wrote {DumpOutputPath} ({json.Length} bytes)");
        }

        private static object BuildSnapshot()
        {
            var scene = SceneManager.GetActiveScene();
            var roots = scene.GetRootGameObjects();

            var activeCount = 0;
            var totalCount = 0;
            foreach (var go in roots)
            {
                foreach (var t in go.GetComponentsInChildren<Transform>(true))
                {
                    totalCount++;
                    if (t.gameObject.activeInHierarchy) activeCount++;
                }
            }

            var components = new Dictionary<string, object>();
            foreach (var go in roots)
            {
                foreach (var dumpable in go.GetComponentsInChildren<MonoBehaviour>(true).OfType<IDumpable>())
                {
                    var key = dumpable.GetType().Name;
                    // If multiple instances of the same component exist, suffix with GO name.
                    if (components.ContainsKey(key))
                    {
                        var mb = (MonoBehaviour)dumpable;
                        key = $"{key}@{mb.gameObject.name}";
                    }
                    try
                    {
                        components[key] = dumpable.DumpState();
                    }
                    catch (Exception ex)
                    {
                        components[key] = new { error = $"{ex.GetType().Name}: {ex.Message}" };
                    }
                }
            }

            return new
            {
                timestamp = DateTime.UtcNow.ToString("o"),
                sceneName = scene.name,
                playMode = Application.isPlaying,
                hierarchy = new
                {
                    rootObjects = roots.Select(r => r.name).ToArray(),
                    activeCount,
                    totalCount
                },
                components,
                // eventBus + coroutines blocks are project-specific — populate
                // them here once your EventBus / CoroutineRunner APIs exist.
                eventBus = (object)null,
                coroutines = (object)null
            };
        }

        // ---------------------------------------------------------------
        // God-mode toolkit — generic surface. Expand per game.
        // ---------------------------------------------------------------

        private static bool _godMode;

#if UNITY_EDITOR
        [MenuItem(MenuRoot + "God Mode (toggle)")]
#endif
        public static void ToggleGodMode() => GodMode(!_godMode);

        public static void GodMode(bool on)
        {
            if (!Application.isPlaying)
            {
                UnityEngine.Debug.LogWarning("[god-mode] ignored — not in Play Mode");
                return;
            }
            _godMode = on;
            // TODO at bootstrap: wire into PlayerHealth / DamageReceiver / whatever
            // applies damage. Conventional hook is a static flag the damage code reads.
            // DamageRouter.Invincible = on;
            UnityEngine.Debug.Log($"[god-mode] {(on ? "ON" : "OFF")}");
        }

        public static void SetHealth(int hp)
        {
            if (!Application.isPlaying) return;
            // TODO at bootstrap: call into your PlayerStats / Health singleton.
            // GameManager.Instance.Player.Stats.SetHp(hp);
            UnityEngine.Debug.Log($"[set-health] {hp}");
        }

        public static void AddItem(string itemId)
        {
            if (!Application.isPlaying) return;
            // TODO at bootstrap: call into your Inventory.
            // GameManager.Instance.Inventory.Add(itemId);
            UnityEngine.Debug.Log($"[add-item] {itemId}");
        }

        public static void TeleportTo(string sceneName)
        {
            if (!Application.isPlaying) return;
            // Async scene load is project-specific; default to immediate swap.
            SceneManager.LoadScene(sceneName);
            UnityEngine.Debug.Log($"[teleport] → {sceneName}");
        }

        public static void KillAll()
        {
            if (!Application.isPlaying) return;
            // TODO at bootstrap: enumerate enemies and damage them to 0.
            // foreach (var e in Object.FindObjectsOfType<Enemy>()) e.Kill();
            UnityEngine.Debug.Log("[kill-all] invoked");
        }

#if UNITY_EDITOR
        // Fixed-parameter menu items (Unity MenuItem takes no args).
        // Parameterized calls go through PendingCommand or a direct static call
        // via execute_menu_item.
        [MenuItem(MenuRoot + "Set Health/100")] private static void _SetHp100() => SetHealth(100);
        [MenuItem(MenuRoot + "Set Health/50")]  private static void _SetHp50()  => SetHealth(50);
        [MenuItem(MenuRoot + "Set Health/1")]   private static void _SetHp1()   => SetHealth(1);
        [MenuItem(MenuRoot + "Kill All Enemies")] private static void _KillAll() => KillAll();
#endif

        // ---------------------------------------------------------------
        // PendingCommand consumer — for parameterized calls from MCP without
        // adding a new MenuItem per variant. Attach DebugBridge (below) to a
        // persistent GameObject in the bootstrap scene.
        // ---------------------------------------------------------------

        public static void ConsumePendingCommand()
        {
            if (string.IsNullOrEmpty(PendingCommand)) return;
            var cmd = PendingCommand;
            PendingCommand = null;

            // Format: "verb:arg" e.g. "set-health:42", "add-item:potion", "teleport:MainMenu"
            var parts = cmd.Split(new[] { ':' }, 2);
            var verb = parts[0].Trim().ToLowerInvariant();
            var arg = parts.Length > 1 ? parts[1].Trim() : "";

            switch (verb)
            {
                case "dump": DumpState(); break;
                case "god-on": GodMode(true); break;
                case "god-off": GodMode(false); break;
                case "set-health": if (int.TryParse(arg, out var hp)) SetHealth(hp); break;
                case "add-item": AddItem(arg); break;
                case "teleport": TeleportTo(arg); break;
                case "kill-all": KillAll(); break;
                default: UnityEngine.Debug.LogWarning($"[pending-cmd] unknown verb '{verb}'"); break;
            }
        }
    }

    // Attach to a persistent scene GameObject so the editor/external caller can
    // poke commands via manage_components without defining a MenuItem per call.
    public class DebugBridge : MonoBehaviour
    {
        private void Update() => McpRemoteControl.ConsumePendingCommand();
    }
}
#endif

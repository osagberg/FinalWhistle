// DebugManager — singleton MonoBehaviour managing cheats + debug console at runtime.
//
// Why singleton?
//   DebugManager outlives scenes; cheats toggled in the hub scene must persist
//   into the combat scene the player teleports to. A static + DontDestroyOnLoad
//   singleton is the simplest correct lifetime. Instance lookups from
//   McpRemoteControl (which lives in a separate #if-guarded compilation unit)
//   must hit a predictable address — singleton satisfies that without IoC
//   ceremony. Only one DebugManager should ever exist; enforced at Awake().
//
// Why DontDestroyOnLoad?
//   Cheats spanning scene transitions would otherwise be lost on scene load.
//   Also ensures RegisterCheat() handlers survive scene transitions so plugin
//   systems can wire cheats in their own Awake() without coordinating with
//   DebugManager lifetime. Persistent Tick() lets us keep polling PendingCommand
//   from McpRemoteControl without attaching a DebugBridge to every scene.
//
// Why DEVELOPMENT_BUILD guard?
//   This whole surface must vanish from Release/Steam builds — cheats in a
//   shipped product are a trust + piracy risk. UNITY_EDITOR covers edit-mode
//   and play-mode-in-editor; DEVELOPMENT_BUILD covers dev Player builds that
//   ship to internal testers. Release builds define neither — the whole class
//   compiles out and the compiler discards [SerializeField] references.
//   Non-negotiable. See [Scripts/Debug/RULES.md](../../../../Assets/_Project/Scripts/Debug/RULES.md).
//
// Pair with McpRemoteControl:
//   DebugManager is runtime-side (MonoBehaviour). McpRemoteControl is
//   editor + remote-callable (static + menu items). McpRemoteControl's
//   DebugBridge forwards PendingCommand verbs ("god-on", "set-currency:500")
//   into DebugManager methods when Play Mode is active. See SKILL.md §"Extending".
//
// WIRE HERE during bootstrap:
//   Every TODO-marked method below needs a project-specific hookup. The
//   DebugManager has no opinion on your SO layout — you wire it.
//     AddCurrency       → hook into your Economy / WalletSO
//     SetCurrency       → hook into your Economy / WalletSO
//     TeleportPlayer    → hook into your PlayerController / SceneLoader
//     UnlockAll         → hook into your Progression / AchievementsSO
//     ToggleGodMode     → hook into your DamageRouter / HealthComponent

#if UNITY_EDITOR || DEVELOPMENT_BUILD
using System;
using System.Collections.Generic;
using Cysharp.Threading.Tasks;
using UnityEngine;
using UnityEngine.SceneManagement;

namespace FinalWhistle.Debug
{
    public sealed class DebugManager : MonoBehaviour
    {
        // ---------------------------------------------------------------
        // Singleton — lazy auto-create if missing on first access.
        // ---------------------------------------------------------------

        private static DebugManager _instance;

        public static DebugManager Instance
        {
            get
            {
                if (_instance != null) return _instance;

                // Auto-bootstrap. Lets callers (McpRemoteControl, test rigs)
                // hit DebugManager.Instance without a guarantee that a scene
                // has already spawned one.
                var go = new GameObject("[DebugManager]");
                _instance = go.AddComponent<DebugManager>();
                DontDestroyOnLoad(go);
                return _instance;
            }
        }

        // ---------------------------------------------------------------
        // State
        // ---------------------------------------------------------------

        [Header("Debug state (read-only at runtime; use methods to mutate)")]
        [SerializeField] private bool godMode;
        [SerializeField] private float timeScale = 1f;

        public bool GodMode => godMode;
        public float TimeScale => timeScale;

        // Extensible cheat registry — plugins / systems register during their
        // own Awake(), keeping DebugManager free of project-specific coupling.
        private readonly Dictionary<string, Action> _cheats = new(StringComparer.OrdinalIgnoreCase);

        // ---------------------------------------------------------------
        // Lifecycle
        // ---------------------------------------------------------------

        private void Awake()
        {
            if (_instance != null && _instance != this)
            {
                // Scene reload spawned a duplicate. Keep the original.
                Destroy(gameObject);
                return;
            }
            _instance = this;
            DontDestroyOnLoad(gameObject);
            UnityEngine.Debug.Log("[DebugManager] ready");
        }

        private void OnDestroy()
        {
            if (_instance == this) _instance = null;
        }

        // ---------------------------------------------------------------
        // God mode
        // ---------------------------------------------------------------

        public void ToggleGodMode() => SetGodMode(!godMode);

        public void SetGodMode(bool on)
        {
            godMode = on;
            // WIRE HERE during bootstrap: hook to your damage pipeline.
            //   DamageRouter.Invincible = on;
            //   or broadcast an event: EventBus.Raise(new GodModeChanged(on));
            UnityEngine.Debug.Log($"[DebugManager] god-mode {(on ? "ON" : "OFF")}");
        }

        // ---------------------------------------------------------------
        // Time scale
        // ---------------------------------------------------------------

        public void SetTimeScale(float scale)
        {
            // Clamp to a sane range. 0 freezes the game; 10 is fast-forward
            // past legibility but OK for skip-intro-cutscene cheats.
            timeScale = Mathf.Clamp(scale, 0f, 10f);
            Time.timeScale = timeScale;
            UnityEngine.Debug.Log($"[DebugManager] time-scale = {timeScale}");
        }

        // ---------------------------------------------------------------
        // Teleport
        // ---------------------------------------------------------------

        public void TeleportPlayer(Vector3 worldPos)
        {
            if (!Application.isPlaying)
            {
                UnityEngine.Debug.LogWarning("[DebugManager] TeleportPlayer ignored — not playing");
                return;
            }
            // WIRE HERE during bootstrap: move your PlayerController.
            //   var player = GameManager.Instance.Player;
            //   player.transform.position = worldPos;
            //   or go through a CharacterController.Move for collision-safe teleport.
            UnityEngine.Debug.Log($"[DebugManager] teleport → {worldPos}");
        }

        public void TeleportPlayer(string sceneName)
        {
            if (!Application.isPlaying) return;
            if (string.IsNullOrEmpty(sceneName))
            {
                UnityEngine.Debug.LogWarning("[DebugManager] TeleportPlayer(scene) — empty name");
                return;
            }
            // Async scene swap via UniTask — matches CSharp/RULES.md async doctrine.
            TeleportPlayerAsync(sceneName).Forget();
        }

        private async UniTaskVoid TeleportPlayerAsync(string sceneName)
        {
            // WIRE HERE during bootstrap: replace with your SceneLoader service
            // if you have one (fade-out, loading screen, Addressables-based load).
            var op = SceneManager.LoadSceneAsync(sceneName);
            if (op == null)
            {
                UnityEngine.Debug.LogWarning($"[DebugManager] scene '{sceneName}' not in build settings");
                return;
            }
            while (!op.isDone) await UniTask.Yield();
            UnityEngine.Debug.Log($"[DebugManager] teleported to scene '{sceneName}'");
        }

        // ---------------------------------------------------------------
        // Currency / economy
        // ---------------------------------------------------------------

        public void AddCurrency(int amount)
        {
            if (!Application.isPlaying) return;
            // WIRE HERE during bootstrap: hook AddCurrency to your economy SO
            //   WalletSO.Instance.Add(amount);
            //   or: EconomyService.Deposit(amount);
            UnityEngine.Debug.Log($"[DebugManager] add-currency {amount:+#;-#;0}");
        }

        public void SetCurrency(int amount)
        {
            if (!Application.isPlaying) return;
            // WIRE HERE during bootstrap.
            //   WalletSO.Instance.Set(amount);
            UnityEngine.Debug.Log($"[DebugManager] set-currency = {amount}");
        }

        // ---------------------------------------------------------------
        // Progression
        // ---------------------------------------------------------------

        public void UnlockAll()
        {
            if (!Application.isPlaying) return;
            // WIRE HERE during bootstrap — iterate your ProgressionSO entries.
            //   foreach (var entry in ProgressionDB.Instance.All)
            //       entry.ForceUnlock();
            UnityEngine.Debug.Log("[DebugManager] unlock-all invoked");
        }

        // ---------------------------------------------------------------
        // Cheat registry — extensibility hook for project systems.
        // ---------------------------------------------------------------

        /// <summary>
        /// Register a named cheat callable by string key. Intended for plugin
        /// systems to self-register during their own Awake() without DebugManager
        /// needing to know about them. Overwrites any prior registration with
        /// the same name — last writer wins.
        /// </summary>
        public void RegisterCheat(string name, Action handler)
        {
            if (string.IsNullOrWhiteSpace(name))
            {
                UnityEngine.Debug.LogWarning("[DebugManager] RegisterCheat — blank name ignored");
                return;
            }
            if (handler == null)
            {
                UnityEngine.Debug.LogWarning($"[DebugManager] RegisterCheat('{name}') — null handler ignored");
                return;
            }
            _cheats[name] = handler;
            UnityEngine.Debug.Log($"[DebugManager] registered cheat '{name}'");
        }

        /// <summary>
        /// Remove a previously registered cheat. Idempotent — no-op if absent.
        /// </summary>
        public void UnregisterCheat(string name)
        {
            if (_cheats.Remove(name))
                UnityEngine.Debug.Log($"[DebugManager] unregistered cheat '{name}'");
        }

        /// <summary>
        /// Invoke a registered cheat by name. Unknown names log a warning and
        /// return false — they never throw, keeping MCP-driven calls safe even
        /// when the project's cheat list drifts from the caller's expectations.
        /// </summary>
        public bool InvokeCheat(string name)
        {
            if (!_cheats.TryGetValue(name, out var handler))
            {
                UnityEngine.Debug.LogWarning($"[DebugManager] cheat '{name}' not registered");
                return false;
            }
            try
            {
                handler.Invoke();
                return true;
            }
            catch (Exception ex)
            {
                UnityEngine.Debug.LogError($"[DebugManager] cheat '{name}' threw: {ex}");
                return false;
            }
        }

        public IReadOnlyCollection<string> RegisteredCheats => _cheats.Keys;

        // ---------------------------------------------------------------
        // MCP bridge — integrates with McpRemoteControl.PendingCommand.
        // ---------------------------------------------------------------

        // DebugManager polls McpRemoteControl.PendingCommand each frame so
        // external callers can trigger cheats without attaching DebugBridge
        // to every scene. DebugBridge (in McpRemoteControl.cs) is still an
        // option; this is the singleton convenience path.
        private void Update()
        {
            if (!string.IsNullOrEmpty(McpRemoteControl.PendingCommand))
            {
                HandleMcpCommand(McpRemoteControl.PendingCommand);
                McpRemoteControl.PendingCommand = null;
            }
        }

        private void HandleMcpCommand(string cmd)
        {
            // Format: "verb:arg" — matches McpRemoteControl.ConsumePendingCommand.
            var parts = cmd.Split(new[] { ':' }, 2);
            var verb = parts[0].Trim().ToLowerInvariant();
            var arg = parts.Length > 1 ? parts[1].Trim() : "";

            switch (verb)
            {
                case "god-on": SetGodMode(true); break;
                case "god-off": SetGodMode(false); break;
                case "god-toggle": ToggleGodMode(); break;
                case "time-scale":
                    if (float.TryParse(arg, out var ts)) SetTimeScale(ts);
                    break;
                case "teleport":
                    // Prefer scene teleport; coords would need parsing we don't need here.
                    TeleportPlayer(arg);
                    break;
                case "add-currency":
                    if (int.TryParse(arg, out var add)) AddCurrency(add);
                    break;
                case "set-currency":
                    if (int.TryParse(arg, out var set)) SetCurrency(set);
                    break;
                case "unlock-all":
                    UnlockAll();
                    break;
                case "cheat":
                    InvokeCheat(arg);
                    break;
                default:
                    UnityEngine.Debug.LogWarning($"[DebugManager] unknown MCP verb '{verb}'");
                    break;
            }
        }
    }
}
#endif

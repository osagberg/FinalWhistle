import { defineConfig } from "vite";
import solid from "vite-plugin-solid";
import { resolve } from "node:path";

// Tauri integration notes (2026-05 era):
//   1. Tauri spawns Vite as a child process (per tauri.conf.json `beforeDevCommand`).
//      Port 1420 is the published Tauri default; if you change it here, change it
//      in `src-tauri/tauri.conf.json` `devUrl` too.
//   2. `clearScreen: false` keeps Tauri's pre-bundler logs visible above Vite's.
//   3. `strictPort: true` makes Vite die rather than auto-walk to 1421 — Tauri's
//      devUrl is hardcoded, so silent port-walking causes confusing "Could not
//      connect" errors in the webview.
//   4. The HMR `host` env-var dance is the Tauri 2 recommended setup for
//      cross-OS dev. On macOS + Linux it's unused; on Windows the IPv6 default
//      breaks HMR. Leave as documented.
//   5. Tauri 2 ships `process.env.TAURI_PLATFORM` etc. into the bundler; we
//      don't read them here yet but the pattern is preserved for future
//      per-OS conditional builds.

const host = process.env.TAURI_DEV_HOST;

// Sync config factory — no awaits in the body. Earlier `async ()` form
// fails Vite 6's typecheck because `defineConfig`'s `UserConfigFn` doesn't
// accept Promise return values without an explicit `Promise<UserConfig>`
// generic, and that's overkill when nothing here is asynchronous.
export default defineConfig(() => ({
  plugins: [solid()],
  resolve: {
    alias: {
      "~": resolve(__dirname, "src"),
    },
  },
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : false,
    watch: {
      // Don't waste fs watcher slots on the Rust shell.
      ignored: ["**/src-tauri/**"],
    },
    // DEV-ONLY: proxy /__cmd/* → fw-dev-server at 127.0.0.1:1422/cmd/*.
    // Port 1422, NOT 1421 — 1421 is the HMR websocket port above (under
    // TAURI_DEV_HOST), so the dev-server must not collide with it.
    // Same-origin → no CORS. Activate with ?backend=http or VITE_FW_BROWSER_BACKEND=http.
    // This proxy entry is harmless when fw-dev-server is not running — requests
    // fail with a network error surfaced via IpcShapeError at the safeInvoke seam.
    proxy: {
      "/__cmd": {
        target: "http://127.0.0.1:1422",
        // Rewrite /__cmd/get_standings → /cmd/get_standings
        rewrite: (path: string) => path.replace(/^\/__cmd/, "/cmd"),
        changeOrigin: false,
      },
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    // Tauri uses Chromium on Windows/Linux + WebKit on macOS. Targeting
    // es2022 on the WebKit side is safe in macOS 11+, which is our min spec.
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari14",
    // String-literal cast keeps Vite 6's `build.minify` typing happy
    // (`"esbuild" | "terser" | boolean`); the conditional inferred to
    // `string | boolean` without it.
    minify: (process.env.TAURI_ENV_DEBUG ? false : "esbuild") as "esbuild" | false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
}));

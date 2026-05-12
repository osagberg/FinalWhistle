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

export default defineConfig(async () => ({
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
      : undefined,
    watch: {
      // Don't waste fs watcher slots on the Rust shell.
      ignored: ["**/src-tauri/**"],
    },
  },
  envPrefix: ["VITE_", "TAURI_ENV_*"],
  build: {
    // Tauri uses Chromium on Windows/Linux + WebKit on macOS. Targeting
    // es2022 on the WebKit side is safe in macOS 11+, which is our min spec.
    target: process.env.TAURI_ENV_PLATFORM === "windows" ? "chrome105" : "safari14",
    minify: !process.env.TAURI_ENV_DEBUG ? "esbuild" : false,
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
  },
}));

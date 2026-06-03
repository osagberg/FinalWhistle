/* @refresh reload */
import { createSignal, type JSX } from "solid-js";
import { render } from "solid-js/web";
import { Router } from "@solidjs/router";

// Visual-identity fonts (T4-3). Self-hosted via @fontsource — Vite bundles the
// woff2 locally, so no CDN request and no Tauri-CSP exception is needed. The
// family names ('Anton' / 'Inter' / 'JetBrains Mono') match tailwind.config.ts.
// Weights cover the font-weight classes the UI actually uses: Anton is single-
// weight; Inter spans normal/medium/semibold/bold; JetBrains Mono normal/medium/bold.
import "@fontsource/anton/400.css";
import "@fontsource/inter/400.css";
import "@fontsource/inter/500.css";
import "@fontsource/inter/600.css";
import "@fontsource/inter/700.css";
import "@fontsource/jetbrains-mono/400.css";
import "@fontsource/jetbrains-mono/500.css";
import "@fontsource/jetbrains-mono/700.css";

import "./styles.css";
import App from "./App";
import SplashOverlay from "./components/SplashOverlay";
import { getSettings } from "./lib/api/settings";
import { setTheme, setReduceMotion } from "./lib/state";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Root element #root not found in index.html");
}

// Settings promise — drives both the theme application and the splash ready
// signal. Best-effort: if it fails (browser-preview / test / no Tauri), we
// resolve immediately so the splash doesn't block indefinitely.
const settingsReady: Promise<void> = getSettings()
  .then((s) => {
    setTheme(s.theme);
    setReduceMotion(s.reduceMotion);
  })
  .catch((e: unknown) => {
    // Browser-preview / test envs reject here (no Tauri runtime) — expected, and
    // the signal defaults (light, no reduce-motion) are correct fallbacks. But a
    // REAL Tauri user with a corrupt settings file ALSO lands here
    // (IpcError::SettingsLoadFailed), so log it (matching the repo's IPC-catch
    // convention) rather than silently resetting their theme with no signal.
    console.error("[main] getSettings failed; using default theme/motion:", e);
  });

// Root shell — owns the splash signal so it can be cleared from within the
// SolidJS reactive tree (SplashOverlay uses onMount / onCleanup which need
// to run inside a Solid render context).
function Shell(): JSX.Element {
  const [splashDone, setSplashDone] = createSignal(false);

  return (
    <>
      {/* Main app — rendered beneath the splash from the start so route
          components can begin their own data fetches immediately. */}
      <Router>{App()}</Router>
      {/* Splash overlay — unmounts itself via onDone once app is ready. */}
      {!splashDone() && (
        <SplashOverlay
          ready={settingsReady}
          onDone={() => setSplashDone(true)}
        />
      )}
    </>
  );
}

render(() => <Shell />, root);

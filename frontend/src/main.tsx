/* @refresh reload */
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
import { getSettings } from "./lib/api/settings";
import { setTheme, setReduceMotion } from "./lib/state";

const root = document.getElementById("root");
if (!root) {
  throw new Error("Root element #root not found in index.html");
}

// Apply persisted settings before first render where feasible.
// Best-effort: if this fails (e.g. no Tauri in browser-preview), fall back
// to the signal defaults (light, no reduce-motion) and do NOT crash.
getSettings()
  .then((s) => {
    setTheme(s.theme);
    setReduceMotion(s.reduceMotion);
  })
  .catch(() => {
    // Silently ignore — browser preview and test environments have no Tauri
    // runtime. The signal defaults (light, false) are correct fallbacks.
  });

// <Router>'s children are <Route> definitions. App() returns a <Route>
// tree; invoke it to get JSX (a plain function reference here would type
// as `() => Element`, which @solidjs/router 0.15 won't accept as
// children — needs the materialized route nodes).
render(() => <Router>{App()}</Router>, root);

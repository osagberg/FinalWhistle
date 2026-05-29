/*
 * App-wide reactive state. Solid signals only — no Redux, no Zustand.
 *
 * Scope discipline: this file holds state that MUST be observable across
 * routes. Per-route ephemeral state belongs in the route file. If you find
 * yourself adding a fourth signal here, ask whether it actually crosses
 * route boundaries.
 */

import { createSignal, createMemo, createRoot } from "solid-js";

// Active save/career identifier. `null` until the user starts/loads a career.
const [careerIdSignal, setCareerIdSignal] = createSignal<string | null>(null);

// The club the user is managing within the active career.
const [selectedClubIdSignal, setSelectedClubIdSignal] = createSignal<string | null>(null);

// Theme. Tailwind reads `.dark` on <html>. Persisted via T4-6a settings.
const [themeSignal, setThemeSignal] = createSignal<"light" | "dark">("light");

// Reduce-motion. Frontend applies `.reduce-motion` to <html> when true.
// Persisted via T4-6a settings.
const [reduceMotionSignal, setReduceMotionSignal] = createSignal<boolean>(false);

export const careerId = careerIdSignal;
export const setCareerId = setCareerIdSignal;

export const selectedClubId = selectedClubIdSignal;
export const setSelectedClubId = setSelectedClubIdSignal;

export const theme = themeSignal;
export function setTheme(next: "light" | "dark"): void {
  setThemeSignal(next);
  if (typeof document !== "undefined") {
    document.documentElement.classList.toggle("dark", next === "dark");
  }
}

export const reduceMotion = reduceMotionSignal;

/**
 * Set the reduce-motion preference and toggle the `.reduce-motion` class on
 * `document.documentElement` (mirrors the `.dark` pattern in `setTheme`).
 *
 * The CSS rule in `styles.css` suppresses all transitions + animations when
 * `.reduce-motion` is present on `<html>`.
 */
export function setReduceMotion(next: boolean): void {
  setReduceMotionSignal(next);
  if (typeof document !== "undefined") {
    document.documentElement.classList.toggle("reduce-motion", next);
  }
}

/*
 * True when both a career and a managed club are selected.
 *
 * Wrapped in `createRoot` because this memo lives at module scope — outside
 * any component's reactive owner. Without an owner Solid logs "computations
 * created outside a `createRoot` or `render` will never be disposed" on every
 * page load. The root is intentionally never disposed: this is app-lifetime
 * global state, so it lives as long as the page. Signals above need no root
 * (a plain signal creates no computation); only the memo does.
 */
export const isCareerActive = createRoot(() => {
  const active = createMemo(
    () => careerId() !== null && selectedClubId() !== null,
  );
  return active;
});

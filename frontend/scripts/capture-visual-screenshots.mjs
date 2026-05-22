/*
 * Visual-identity reference screenshots (T4-3).
 *
 * Drives the system-installed Google Chrome via playwright-core
 * (`channel: "chrome"` — no bundled-browser download) against the running
 * Vite dev server and writes PNGs to `docs/visual/`.
 *
 * Reusable for later T4 UI rows — add entries to CAPTURES.
 *
 * Prereq: the dev server must be running — `pnpm dev` (port 1420).
 * Run:    pnpm screenshots   (or: node scripts/capture-visual-screenshots.mjs)
 */
import { chromium } from "playwright-core";
import { mkdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const BASE = "http://localhost:1420";
const OUT_DIR = resolve(
  dirname(fileURLToPath(import.meta.url)),
  "../../docs/visual",
);

/**
 * Each entry: a route, an output filename, a light/dark flag, and an optional
 * extra settle delay (ms) for routes that render asynchronously (the board
 * fetches a fixture then renders on the PixiJS ticker).
 */
const CAPTURES = [
  { route: "/", file: "home-light.png", dark: false },
  { route: "/", file: "home-dark.png", dark: true },
  {
    route: "/dev/board-preview",
    file: "tactical-board.png",
    dark: false,
    settleMs: 2500,
  },
];

async function main() {
  mkdirSync(OUT_DIR, { recursive: true });
  const browser = await chromium.launch({ channel: "chrome", headless: true });
  try {
    for (const cap of CAPTURES) {
      const page = await browser.newPage({
        viewport: { width: 1440, height: 900 },
      });
      await page.goto(`${BASE}${cap.route}`, { waitUntil: "networkidle" });
      // The app uses class-based dark mode (tailwind darkMode: "class"), so
      // toggle the `.dark` class directly rather than emulating a media query.
      if (cap.dark) {
        await page.evaluate(() =>
          document.documentElement.classList.add("dark"),
        );
      }
      // Wait for the @fontsource webfonts so the shot shows the real faces,
      // not the fallback chain.
      await page.evaluate(() => document.fonts.ready.then(() => true));
      await page.waitForTimeout(cap.settleMs ?? 400);
      const outPath = resolve(OUT_DIR, cap.file);
      await page.screenshot({ path: outPath });
      console.log(
        `captured ${cap.route}${cap.dark ? " (dark)" : ""} -> ${outPath}`,
      );
      await page.close();
    }
  } finally {
    await browser.close();
  }
}

main().catch((err) => {
  console.error("screenshot capture failed:", err);
  process.exit(1);
});

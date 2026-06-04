/*
 * board-shots.mjs — per-tick visual verification of the 2D tactical board.
 *
 * The headless successor to the Claude-Preview match loop: drives the dev
 * board against a deterministic `dump_frames` fixture, scrubs to a list of
 * ticks via the `window.fwDev.scrubTo` debug surface, and writes one PNG per
 * tick that the agent reads back. No Tauri shell, no MCP, fully CLI/CI-native.
 *
 * Prereqs:
 *   - Vite dev server running on :1420  (pnpm dev)
 *   - a fixture served by Vite under frontend/public/dev-fixtures/, e.g.
 *       target/release/dump_frames --seed 0x... --ticks 5400 --content content --compact \
 *         > frontend/public/dev-fixtures/verify-current.json
 *
 * Usage:
 *   node scripts/board-shots.mjs <fixturePath> <comma-separated-ticks> [outDir]
 *   node scripts/board-shots.mjs /dev-fixtures/verify-current.json 0,1295,1300,5400
 *
 * Prints a JSON summary (frameCount, per-tick on-screen status, console errors)
 * to stdout so the result is machine-readable as well as visual.
 */
import { chromium } from "playwright-core";
import { mkdirSync } from "node:fs";

const BASE = "http://localhost:1420";
const fixture = process.argv[2] ?? "/dev-fixtures/verify-current.json";
const ticks = (process.argv[3] ?? "0")
  .split(",")
  .map((t) => Number(t.trim()))
  .filter((t) => Number.isFinite(t));
const outDir = process.argv[4] ?? "/tmp/fw-board";

mkdirSync(outDir, { recursive: true });

// FrameSource expects ?source=fixture:<path>. URLSearchParams encodes the colon
// and slashes; the board decodes them when it reads searchParams.
const url = `${BASE}/dev/board?${new URLSearchParams({ source: `fixture:${fixture}` })}`;

const summary = { url, fixture, frameCount: null, ticks: [], consoleErrors: [] };

const browser = await chromium.launch({ channel: "chrome", headless: true });
try {
  const page = await browser.newPage({ viewport: { width: 1280, height: 800 } });
  page.on("console", (m) => {
    if (m.type() === "error") summary.consoleErrors.push(m.text());
  });
  page.on("pageerror", (e) => summary.consoleErrors.push(`pageerror: ${e.message}`));

  await page.goto(url, { waitUntil: "networkidle" });

  // Wait for the FrameSource to finish loading the fixture (board exposes
  // window.fwDev only in DEV builds, and frameCount() > 1 once frames land).
  await page.waitForFunction(
    () => globalThis.fwDev && globalThis.fwDev.frameCount() > 1,
    { timeout: 30_000 },
  );
  summary.frameCount = await page.evaluate(() => globalThis.fwDev.frameCount());

  for (const tick of ticks) {
    await page.evaluate((t) => globalThis.fwDev.scrubTo(t), tick);
    // Let the PixiJS ticker paint the scrubbed frame (it renders off the ticker,
    // not synchronously off the signal), then settle one more frame.
    await page.evaluate(
      () => new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r))),
    );
    await page.waitForTimeout(120);

    const actualTick = await page.evaluate(() => globalThis.fwDev.currentTick());
    // The board header renders "Tick: N / Total | Seed: ... | Score: H–A".
    const status = (await page.locator("body").innerText())
      .split("\n")
      .find((l) => l.startsWith("Tick:")) ?? "(status text not found)";

    const file = `${outDir}/board-tick-${String(tick).padStart(5, "0")}.png`;
    await page.screenshot({ path: file });
    summary.ticks.push({ requested: tick, actualTick, status, file });
  }
} finally {
  await browser.close();
}

console.log(JSON.stringify(summary, null, 2));

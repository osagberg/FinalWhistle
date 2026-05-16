/**
 * `window.fwDev` debug surface — shared type between the producer
 * (Dev/TacticalBoard.tsx exposes it in dev builds) and the consumer
 * (Dev/TacticalBoard.test.tsx + Claude Preview interactive scrubbing).
 *
 * T1-13 type-design audit P2: previously the producer cast as
 * `(window as { fwDev?: unknown }).fwDev = {...}` while the test
 * cast as `(window as { fwDev?: { scrubTo, currentTick, frameCount } }).fwDev` —
 * two inline re-declarations of the same API contract, exactly the
 * drift risk that makes "we test the surface" stop being true after
 * the next change. Centralising the shape here means producer + test
 * + Claude Preview console code all read from one source.
 *
 * Exposure is gated on `import.meta.env.DEV` in the producer, so
 * `window.fwDev` is `undefined` in production builds.
 */

/** The shape of `window.fwDev`. */
export interface FwDevApi {
  /**
   * Set the current frame index, clamped to `[0, frameCount() - 1]`.
   * Negative inputs clamp to 0; above-bounds inputs clamp to the last
   * loaded frame.
   */
  scrubTo: (n: number) => void;
  /** Return the current frame index (the value `scrubTo` last set). */
  currentTick: () => number;
  /** Return the total number of frames loaded by the FrameSource. */
  frameCount: () => number;
}

declare global {
  interface Window {
    /**
     * Dev-only frame-scrubber surface. `undefined` in production builds
     * (the producer gates exposure on `import.meta.env.DEV`).
     */
    fwDev?: FwDevApi;
  }
}

// Empty export to make this file a module — required for `declare global`
// to be valid in a `.d.ts` file under TypeScript's --isolatedModules.
export {};

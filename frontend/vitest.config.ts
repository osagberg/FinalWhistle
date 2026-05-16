/*
 * Vitest configuration for the frontend.
 *
 * - Environment: jsdom (DOM APIs available in tests without a browser).
 * - Solid plugin: required for JSX transform + reactivity in tests.
 * - Path alias "~" mirrors vite.config.ts so imports resolve identically.
 * - setupFiles loads @testing-library/jest-dom matchers globally.
 *
 * Test files convention: `*.test.tsx` or `*.test.ts` co-located with the
 * component, or in `src/__tests__/`. Both patterns are covered by the glob.
 */

import { defineConfig } from "vitest/config";
import solid from "vite-plugin-solid";
import { resolve } from "node:path";

export default defineConfig({
  plugins: [solid()],
  resolve: {
    alias: {
      "~": resolve(__dirname, "src"),
    },
    conditions: ["development", "browser"],
  },
  test: {
    environment: "jsdom",
    globals: true,
    setupFiles: ["./src/test-setup.ts"],
    include: ["src/**/*.test.{ts,tsx}"],
    // Exclude node_modules and dist from test discovery.
    exclude: ["node_modules/**", "dist/**"],
  },
});

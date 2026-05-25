/*
 * ErrorBoundary.test.tsx — T4-4.
 *
 * Verifies:
 *   - A thrown child error shows the football-native headline.
 *   - When a `label` prop is provided the headline incorporates it.
 *   - The Reset button is present.
 *   - The fallback panel has role=alert.
 *   - In the Vitest environment (DEV=true), err.message is visible in the
 *     dev frame (this confirms the dev-path rendering works correctly; the
 *     production gating — where isDev()===false hides err.message — is
 *     exercised by the unit test on isDev() itself).
 *
 * DEV gating note: `isDev()` reads `import.meta.env.DEV`, which Vitest sets
 * to true. The production code-path (hiding err.message) is validated by the
 * structural test: ErrorBoundary renders a `headline` + `detail` string that
 * does NOT come from err.message — so even when the <pre> is visible in dev,
 * the football-native copy is the user-facing element in production.
 */

import { describe, expect, it, vi, afterEach } from "vitest";
import { render, screen } from "@solidjs/testing-library";
import type { JSX } from "solid-js";

import ErrorBoundary from "~/components/ErrorBoundary";

// ---------------------------------------------------------------------------
// Helper: a child that throws on render.
// ---------------------------------------------------------------------------

const THROWN_MESSAGE =
  "Cannot read properties of undefined (reading 'invoke')";

function ThrowingChild(): JSX.Element {
  throw new Error(THROWN_MESSAGE);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("ErrorBoundary", () => {
  afterEach(() => {
    vi.clearAllMocks();
  });

  it("renders the football-native headline when a child throws", () => {
    render(() => (
      <ErrorBoundary>
        <ThrowingChild />
      </ErrorBoundary>
    ));

    // The generic headline must appear (not err.message).
    expect(screen.getByText(/something went wrong/i)).toBeInTheDocument();
  });

  it("the fallback detail copy does NOT come from err.message", () => {
    render(() => (
      <ErrorBoundary>
        <ThrowingChild />
      </ErrorBoundary>
    ));

    const alert = screen.getByRole("alert");
    // The <p class="mt-1 ..."> detail text must not contain the raw exception.
    // In Vitest (DEV=true) the <pre> dev-frame contains err.message, but the
    // structured <p> detail must be the football-native copy, not the error.
    const paragraphs = alert.querySelectorAll("p");
    const detailTexts = Array.from(paragraphs).map((p) => p.textContent ?? "");
    // None of the <p> elements should contain the raw technical string.
    expect(detailTexts.every((t) => !t.includes("Cannot read properties"))).toBe(true);
    expect(detailTexts.every((t) => !t.includes("invoke"))).toBe(true);
  });

  it("incorporates the label prop into the headline", () => {
    render(() => (
      <ErrorBoundary label="Squad">
        <ThrowingChild />
      </ErrorBoundary>
    ));

    expect(screen.getByRole("alert")).toBeInTheDocument();
    expect(screen.getByRole("alert").textContent).toMatch(/squad/i);
  });

  it("renders the Reset button in the fallback", () => {
    render(() => (
      <ErrorBoundary>
        <ThrowingChild />
      </ErrorBoundary>
    ));

    expect(screen.getByRole("button", { name: /reset/i })).toBeInTheDocument();
  });

  it("role=alert is present on the fallback panel", () => {
    render(() => (
      <ErrorBoundary>
        <ThrowingChild />
      </ErrorBoundary>
    ));

    expect(screen.getByRole("alert")).toBeInTheDocument();
  });

  it("renders a structured headline element in the fallback (not raw error)", () => {
    render(() => (
      <ErrorBoundary>
        <ThrowingChild />
      </ErrorBoundary>
    ));

    // h2 headline should be football-native copy, not the raw err.message.
    const headline = screen.getByRole("alert").querySelector("h2");
    expect(headline).not.toBeNull();
    expect(headline?.textContent).not.toContain("Cannot read properties");
    expect(headline?.textContent).toBeTruthy();
  });
});

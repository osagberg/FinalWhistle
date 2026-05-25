import { ErrorBoundary as SolidErrorBoundary, type JSX, type ParentProps } from "solid-js";

/*
 * Thin shell over Solid's built-in ErrorBoundary. Centralises the visual
 * treatment so every route renders failures with the same chrome.
 *
 * T4-4 changes:
 *   - Accepts `label?: string` for the boundary's headline.
 *   - Dev-only frame: raw err.message/stack visible only when import.meta.env.DEV.
 *     Production renders the football-native fallback + Reset button only.
 *   - isDev() helper is extracted so tests can stub it without hacking import.meta.env.
 *
 * Copy rule (DESIGN_DOC §9): no raw error strings leaked to players in production.
 */

interface ErrorBoundaryProps extends ParentProps {
  /** Optional headline for this boundary's fallback panel.
   * Defaults to a generic football-native placeholder. */
  label?: string;
}

/** Extracted so tests can stub it without needing import.meta.env manipulation. */
export function isDev(): boolean {
  return import.meta.env.DEV;
}

export default function ErrorBoundary(props: ErrorBoundaryProps): JSX.Element {
  return (
    <SolidErrorBoundary
      fallback={(err: unknown, reset: () => void) => {
        const headline =
          props.label != null
            ? `Something went wrong in ${props.label}`
            : "Something went wrong on the bench";
        const detail = "An unexpected fault stopped the page from loading. Hit Reset to try again, or restart the app if it keeps happening.";

        return (
          <div
            class="fw-panel p-4 m-2 bg-flag-red/5 border-flag-red/20"
            role="alert"
          >
            <h2 class="font-display text-lg text-flag-red">{headline}</h2>
            <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
              {detail}
            </p>
            {/* Dev-only technical frame — never shown in production builds. */}
            {isDev() && (
              <pre class="mt-2 text-xs font-mono whitespace-pre-wrap text-ink-subtle dark:text-paper-subtle">
                {err instanceof Error ? err.message : String(err)}
              </pre>
            )}
            <button
              type="button"
              class="mt-3 fw-nav-link bg-flag-red/10 hover:bg-flag-red/20"
              onClick={reset}
            >
              Reset
            </button>
          </div>
        );
      }}
    >
      {props.children}
    </SolidErrorBoundary>
  );
}

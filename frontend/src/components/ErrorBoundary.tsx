import { ErrorBoundary as SolidErrorBoundary, type JSX, type ParentProps } from "solid-js";

/*
 * Thin shell over Solid's built-in ErrorBoundary. Centralises the visual
 * treatment so every route renders failures with the same chrome.
 *
 * Copy rule (DESIGN_DOC §9): no raw error strings leaked to players; technical
 * frames are dev-only. T0-2 still surfaces the message — the polish pass at
 * T4-4 swaps to football-native copy ("Something went wrong on the bench.").
 */
export default function ErrorBoundary(props: ParentProps): JSX.Element {
  return (
    <SolidErrorBoundary
      fallback={(err: unknown, reset: () => void) => (
        <div class="fw-panel p-4 m-2 bg-flag-red/5 border-flag-red/20">
          <h2 class="font-display text-lg text-flag-red">Something went wrong</h2>
          <pre class="mt-2 text-xs font-mono whitespace-pre-wrap text-ink-subtle dark:text-paper-subtle">
            {err instanceof Error ? err.message : String(err)}
          </pre>
          <button
            type="button"
            class="mt-3 fw-nav-link bg-flag-red/10 hover:bg-flag-red/20"
            onClick={reset}
          >
            Try again
          </button>
        </div>
      )}
    >
      {props.children}
    </SolidErrorBoundary>
  );
}

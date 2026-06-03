import { createSignal, onMount, onCleanup, Show, type JSX } from "solid-js";
import Loading from "~/components/Loading";

interface SplashOverlayProps {
  /** Called when the splash has finished and should be removed. */
  onDone: () => void;
  /**
   * A Promise that resolves when the app is ready (e.g. settings loaded).
   * The overlay stays visible until BOTH the promise resolves AND the minimum
   * display duration has elapsed, so it never flashes away instantly.
   */
  ready: Promise<void>;
  /**
   * Minimum time (ms) the splash is shown even if the ready promise resolves
   * before it elapses. Default: 800 ms. Prevents a flash on fast machines.
   */
  minDurationMs?: number;
  /**
   * Hard ceiling (ms) after which the splash dismisses NO MATTER WHAT — even if
   * `ready` never settles (e.g. a hung IPC that neither resolves nor rejects).
   * Without this the `fixed inset-0 z-50` overlay would trap the user behind a
   * "Preparing the pitch…" screen forever. Default: 6000 ms.
   */
  maxDurationMs?: number;
}

/**
 * Full-screen splash overlay (T4-7 game-shell polish).
 *
 * Shown at boot, covering the first paint while fonts + settings load.
 * Fades out once the ready promise resolves AND the minimum duration has
 * elapsed. Respects prefers-reduced-motion / the repo's `.reduce-motion`
 * class (set by the Settings route).
 *
 * Accessibility: role="status" + aria-live so screen readers announce the
 * loading state. Unmounts (not just hidden) after clearing so it can never
 * block pointer events.
 *
 * Dark-mode FOUC mitigation: the splash covers the first paint entirely,
 * so the brief window where the JS-driven `dark` class hasn't been applied
 * yet is invisible to the user. The `index.html` `<html>` already carries
 * `bg-paper text-ink` as a safe light-mode fallback — the splash uses the
 * same neutral background, so the visible colour on first paint is always
 * close to the final theme regardless of which mode the user has set.
 */
export default function SplashOverlay(props: SplashOverlayProps): JSX.Element {
  // Capture at construction — this prop is not expected to change reactively.
  // Using a local const avoids the solid/reactivity lint warning about reading
  // props outside a tracked scope.
  // eslint-disable-next-line solid/reactivity
  const minMs = props.minDurationMs ?? 800;
  // eslint-disable-next-line solid/reactivity
  const maxMs = props.maxDurationMs ?? 6000;
  // visible: controls opacity/pointer-events. mounted: controls DOM presence.
  const [visible, setVisible] = createSignal(true);
  const [mounted, setMounted] = createSignal(true);

  // All timers are tracked so onCleanup can cancel any in-flight one — a fired
  // timer on an unmounted component would call setVisible/onDone on a dead owner.
  let holdTimer: ReturnType<typeof setTimeout> | undefined;
  let fadeTimer: ReturnType<typeof setTimeout> | undefined;
  let hardTimer: ReturnType<typeof setTimeout> | undefined;
  let dismissed = false;

  const dismiss = (): void => {
    // Idempotent: the ready-path and the hard-ceiling path can both reach here;
    // only the first wins.
    if (dismissed) return;
    dismissed = true;
    if (hardTimer !== undefined) clearTimeout(hardTimer);
    setVisible(false);
    // Wait for the fade animation, then fully unmount so the layer cannot
    // block interaction. 320 ms > 300 ms transition to ensure the animation
    // always completes before the element is removed.
    fadeTimer = setTimeout(() => {
      setMounted(false);
      props.onDone();
    }, 320);
  };

  onMount(() => {
    const startTime = Date.now();

    // Hard ceiling: the splash NEVER outlives this, even if `ready` hangs
    // (a hung IPC that neither resolves nor rejects would otherwise lock the UI
    // behind the full-screen overlay).
    hardTimer = setTimeout(dismiss, maxMs);

    // A rejected `ready` is no reason to trap the user — treat it like resolved.
    void props.ready
      .catch(() => undefined)
      .then(() => {
        const remaining = minMs - (Date.now() - startTime);
        if (remaining > 0) {
          holdTimer = setTimeout(dismiss, remaining);
        } else {
          dismiss();
        }
      });
  });

  onCleanup(() => {
    if (holdTimer !== undefined) clearTimeout(holdTimer);
    if (fadeTimer !== undefined) clearTimeout(fadeTimer);
    if (hardTimer !== undefined) clearTimeout(hardTimer);
  });

  return (
    <Show when={mounted()}>
      <div
        role="status"
        aria-live="polite"
        aria-label="Preparing the application"
        class="fixed inset-0 z-50 flex flex-col items-center justify-center gap-6 bg-paper dark:bg-midnight transition-opacity duration-300"
        classList={{ "opacity-0 pointer-events-none": !visible() }}
      >
        <h1
          class="font-display text-5xl tracking-wider text-pitch-600 dark:text-pitch-300 select-none"
          aria-hidden="true"
        >
          FINAL WHISTLE
        </h1>
        <Loading message="Preparing the pitch…" />
      </div>
    </Show>
  );
}

import type { JSX } from "solid-js";

interface LoadingProps {
  /** Override the default copy. Football-native commentary tone. */
  message?: string;
}

export default function Loading(props: LoadingProps): JSX.Element {
  return (
    <div class="flex items-center gap-2 p-4 text-ink-mute dark:text-paper-subtle">
      <span class="inline-block w-2 h-2 rounded-sm bg-pitch-500 animate-pulse" />
      <span class="text-sm font-body">{props.message ?? "Working on it…"}</span>
    </div>
  );
}

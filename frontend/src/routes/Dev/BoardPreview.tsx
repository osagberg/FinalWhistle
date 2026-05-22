/*
 * Dev harness — browser-previewable mount of the production `<TacticalBoard>`.
 *
 * The production board (`~/components/TacticalBoard`) is prop-driven: it takes
 * a `frames` array. In the real app the Match route feeds it frames from the
 * `match_frames` Tauri IPC — which only works inside the Tauri runtime, not a
 * plain browser dev server. This route exists so the board can be seen +
 * screenshotted in a browser (and by the Claude Preview MCP): it fetches a
 * committed `dump_frames` fixture JSON and mounts the board against it.
 *
 * Dev-only. Reachable at `/dev/board-preview`; not in the main nav. The fixture
 * is regenerated with:
 *   cargo run --release --bin dump_frames -- \
 *     --seed 0xfeedbeefcafefade --ticks 600 --compact --content content \
 *     > frontend/public/dev-fixtures/board-sample.json
 */

import { createResource, Show, type JSX } from "solid-js";
import TacticalBoard from "~/components/TacticalBoard";
import { isMatchFrameDTOArray } from "~/lib/runtime-validators";
import type { MatchFrameDTO } from "~/lib/types";

/** Committed `dump_frames` fixture, served by Vite from `frontend/public/`. */
const FIXTURE_URL = "/dev-fixtures/board-sample.json";

async function loadFixture(): Promise<MatchFrameDTO[]> {
  const res = await fetch(FIXTURE_URL);
  if (!res.ok) {
    throw new Error(
      `fixture fetch ${FIXTURE_URL} failed — ${res.status} ${res.statusText}`,
    );
  }
  const body: unknown = await res.json();
  if (!isMatchFrameDTOArray(body)) {
    throw new Error(
      `${FIXTURE_URL} did not parse as MatchFrameDTO[] — regenerate it with the dump_frames command in this file's header comment.`,
    );
  }
  return body;
}

export default function BoardPreview(): JSX.Element {
  const [frames] = createResource(loadFixture);

  return (
    <div class="space-y-3">
      <div class="px-1">
        <h1 class="text-lg font-semibold text-ink dark:text-paper">
          Tactical board — dev preview
        </h1>
        <p class="text-sm text-ink-mute dark:text-paper-subtle">
          Browser-previewable harness for the production{" "}
          <code class="font-mono">&lt;TacticalBoard&gt;</code>. Frames loaded
          from the committed fixture{" "}
          <code class="font-mono">{FIXTURE_URL}</code> — no Tauri IPC needed.
        </p>
      </div>

      <Show
        when={frames()}
        fallback={
          <div
            class="fw-panel p-4 text-sm text-ink-mute dark:text-paper-subtle"
            aria-live="polite"
          >
            <Show
              when={frames.error}
              fallback={<span>Loading fixture…</span>}
            >
              <span class="text-rose-600 dark:text-rose-400">
                Fixture load failed:{" "}
                {frames.error instanceof Error
                  ? frames.error.message
                  : String(frames.error)}
              </span>
            </Show>
          </div>
        }
      >
        {(loaded) => <TacticalBoard frames={loaded()} />}
      </Show>
    </div>
  );
}

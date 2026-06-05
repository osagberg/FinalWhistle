/*
 * Generic TanStack Table wrapper. FM-class density: no zebra striping,
 * thin row borders, tabular-nums, monospace for the numeric cells.
 *
 * Pattern: callers pass `columns` + `data` (Solid signals — pass accessors,
 * not raw arrays, when the data is reactive). The wrapper handles sorting,
 * compact rendering, and empty-state copy.
 *
 * Column alignment — the binary text-vs-numeric rule
 * -------------------------------------------------------
 * Each column def MAY carry `meta: { align: "text" | "num" }`:
 *   - "text" (default when omitted): Inter body font, left-aligned, text-sm.
 *     Use for names, roles, labels — anything human-readable.
 *   - "num": JetBrains Mono, right-aligned, tabular-nums.
 *     Use for every value the eye scans vertically (points, GD, goals,
 *     minutes, wages, fees, appearances).
 * The header aligns the same way as its data column.
 *
 * FwColumnMeta shape is declared below and used via `meta as FwColumnMeta`
 * at read sites — this avoids the `@tanstack/table-core` module-augmentation
 * path which is not resolvable from this workspace's symlinked node_modules.
 *
 * Row highlight
 * -------------------------------------------------------
 * Optional `rowHighlight?: (row: TData) => boolean` prop.
 * A row where the predicate returns true receives a subtle pitch-tint
 * background and a pitch-accent left border (2px). Useful for managed-club
 * row in Standings. No highlight is applied when the prop is omitted.
 *
 * Accessibility: sortable column headers are keyboard-operable. Each sortable
 * <th> receives tabindex="0", an onKeyDown handler (Enter + Space toggle sort),
 * role="columnheader", and a dynamic aria-sort attribute so screen readers
 * announce the current sort direction. Non-sortable headers are not focusable.
 * A focus-visible ring is applied via Tailwind so keyboard users see focus.
 * Highlighted rows carry aria-current="true" so screen readers announce the
 * managed-club row without relying only on colour.
 */

import {
  type ColumnDef,
  createSolidTable,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  type SortingState,
} from "@tanstack/solid-table";
import { createSignal, For, type JSX, Show } from "solid-js";

// ---------------------------------------------------------------------------
// FwColumnMeta — typed contract for per-column alignment hints.
// Callers declare via `meta: { align: "num" }` on their ColumnDef.
// Read back via `cell.column.columnDef.meta as FwColumnMeta | undefined`.
// ---------------------------------------------------------------------------

export interface FwColumnMeta {
  /** "text" (default): Inter body font, left-aligned. "num": JetBrains Mono, right-aligned. */
  align?: "text" | "num";
}

/** Extract the align hint from a raw column meta value. */
function metaAlign(meta: unknown): "text" | "num" | undefined {
  if (meta !== null && typeof meta === "object" && "align" in meta) {
    const { align } = meta as FwColumnMeta;
    if (align === "text" || align === "num") return align;
  }
  return undefined;
}

type AriaSortValue = "ascending" | "descending" | "none";

function ariaSortValue(sortDir: false | "asc" | "desc"): AriaSortValue {
  if (sortDir === "asc") return "ascending";
  if (sortDir === "desc") return "descending";
  return "none";
}

/**
 * Return Tailwind alignment + font classes for the given column align hint.
 * Default is "text" — safe when no meta is provided.
 */
function alignClasses(align: "text" | "num" | undefined): string {
  if (align === "num") {
    return "text-right font-mono tabular-nums";
  }
  // "text" or omitted — Inter body font, left.
  return "text-left font-body";
}

export interface DataTableProps<TData extends object> {
  columns: ColumnDef<TData>[];
  data: TData[];
  /** Override the default "No rows yet" empty-state copy. */
  emptyMessage?: string;
  /** Width hint (Tailwind class) for the wrapper. Default `w-full`. */
  class?: string;
  /**
   * Optional row-highlight predicate. When provided, rows where the predicate
   * returns `true` receive a pitch-accent left border + faint pitch tint.
   * Omit (or pass undefined) for no highlight.
   */
  rowHighlight?: (row: TData) => boolean;
}

export default function DataTable<TData extends object>(props: DataTableProps<TData>): JSX.Element {
  const [sorting, setSorting] = createSignal<SortingState>([]);

  const table = createSolidTable({
    get data() {
      return props.data;
    },
    get columns() {
      return props.columns;
    },
    state: {
      get sorting() {
        return sorting();
      },
    },
    onSortingChange: setSorting,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  return (
    <div
      class={`fw-panel overflow-hidden ${props.class ?? "w-full"}`}
      role="region"
      aria-label="Data table"
    >
      <table class="w-full text-sm">
        <thead class="bg-paper-subtle dark:bg-midnight-subtle">
          <For each={table.getHeaderGroups()}>
            {(headerGroup) => (
              <tr>
                <For each={headerGroup.headers}>
                  {(header) => {
                    const canSort = header.column.getCanSort();
                    const sortDir = () => header.column.getIsSorted();
                    const align = metaAlign(header.column.columnDef.meta);

                    function handleKeyDown(e: KeyboardEvent) {
                      if (!canSort) return;
                      if (e.key === "Enter" || e.key === " ") {
                        // Prevent Space from scrolling the page.
                        e.preventDefault();
                        header.column.getToggleSortingHandler()?.(e);
                      }
                    }

                    return (
                      <th
                        role="columnheader"
                        aria-sort={canSort ? ariaSortValue(sortDir()) : undefined}
                        tabindex={canSort ? 0 : undefined}
                        class={[
                          "px-2 py-1.5 font-semibold text-ink-subtle dark:text-paper-subtle select-none",
                          alignClasses(align),
                          canSort
                            ? "cursor-pointer hover:bg-paper-bold dark:hover:bg-midnight-line focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-pitch-500 focus-visible:ring-inset"
                            : "cursor-default",
                        ].join(" ")}
                        onClick={canSort ? header.column.getToggleSortingHandler() : undefined}
                        onKeyDown={handleKeyDown}
                      >
                        {/*
                         * Numeric headers: sort indicator left of label so right-edge
                         * alignment reads cleanly without the indicator spilling right.
                         */}
                        <span class={`inline-flex items-center gap-1 ${align === "num" ? "flex-row-reverse" : ""}`}>
                          {flexRender(header.column.columnDef.header, header.getContext())}
                          <Show when={sortDir()}>
                            <span class="text-pitch-600 dark:text-pitch-300 text-xs" aria-hidden="true">
                              {sortDir() === "asc" ? "▲" : "▼"}
                            </span>
                          </Show>
                        </span>
                      </th>
                    );
                  }}
                </For>
              </tr>
            )}
          </For>
        </thead>
        <tbody>
          <Show
            when={table.getRowModel().rows.length > 0}
            fallback={
              <tr>
                <td
                  colSpan={table.getAllColumns().length}
                  class="px-2 py-6 text-center text-ink-mute dark:text-paper-subtle italic"
                >
                  {props.emptyMessage ?? "No rows yet."}
                </td>
              </tr>
            }
          >
            <For each={table.getRowModel().rows}>
              {(row) => {
                const highlighted = () =>
                  props.rowHighlight ? props.rowHighlight(row.original) : false;

                return (
                  <tr
                    aria-current={highlighted() ? "true" : undefined}
                    class={[
                      "border-t border-ink-mute/10 dark:border-midnight-line",
                      "hover:bg-paper-subtle dark:hover:bg-midnight-subtle",
                      highlighted()
                        ? "border-l-2 border-l-pitch-500 bg-pitch-50/40 dark:bg-pitch-900/20"
                        : "",
                    ].join(" ")}
                  >
                    <For each={row.getVisibleCells()}>
                      {(cell) => {
                        const align = metaAlign(cell.column.columnDef.meta);
                        return (
                          <td class={`px-2 py-1 text-xs ${alignClasses(align)}`}>
                            {flexRender(cell.column.columnDef.cell, cell.getContext())}
                          </td>
                        );
                      }}
                    </For>
                  </tr>
                );
              }}
            </For>
          </Show>
        </tbody>
      </table>
    </div>
  );
}

/*
 * Generic TanStack Table wrapper. FM-class density: no zebra striping,
 * thin row borders, tabular-nums, monospace for the numeric cells.
 *
 * Pattern: callers pass `columns` + `data` (Solid signals — pass accessors,
 * not raw arrays, when the data is reactive). The wrapper handles sorting,
 * compact rendering, and empty-state copy.
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

export interface DataTableProps<TData> {
  columns: ColumnDef<TData>[];
  data: TData[];
  /** Override the default "No rows yet" empty-state copy. */
  emptyMessage?: string;
  /** Width hint (Tailwind class) for the wrapper. Default `w-full`. */
  class?: string;
}

export default function DataTable<TData>(props: DataTableProps<TData>): JSX.Element {
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
                  {(header) => (
                    <th
                      class="px-2 py-1.5 text-left font-semibold text-ink-subtle dark:text-paper-subtle cursor-pointer select-none hover:bg-paper-bold dark:hover:bg-midnight-line"
                      onClick={header.column.getToggleSortingHandler()}
                    >
                      <span class="inline-flex items-center gap-1">
                        {flexRender(header.column.columnDef.header, header.getContext())}
                        <Show when={header.column.getIsSorted()}>
                          <span class="text-pitch-600 dark:text-pitch-300 text-xs">
                            {header.column.getIsSorted() === "asc" ? "▲" : "▼"}
                          </span>
                        </Show>
                      </span>
                    </th>
                  )}
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
              {(row) => (
                <tr class="border-t border-ink-mute/10 dark:border-midnight-line hover:bg-paper-subtle dark:hover:bg-midnight-subtle">
                  <For each={row.getVisibleCells()}>
                    {(cell) => (
                      <td class="px-2 py-1 font-mono text-xs">
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </td>
                    )}
                  </For>
                </tr>
              )}
            </For>
          </Show>
        </tbody>
      </table>
    </div>
  );
}

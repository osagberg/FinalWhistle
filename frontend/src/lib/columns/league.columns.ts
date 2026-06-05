/*
 * League standings column definitions — T2-6.
 *
 * 10 columns total: position (#), Club, P, W, D, L, GF, GA, GD, Pts.
 * Separated from League.tsx per Frontend/RULES.md §3 ("Column defs in
 * dedicated *.columns.ts files alongside the route").
 *
 * The position column is display-only (1-based row index after current sort)
 * and carries enableSorting: false — clicking a sort header does not sort
 * by position number, it re-numbers the position based on the sorted order.
 *
 * Numeric stat columns (P, W, D, L, GF, GA, GD, Pts) are sortable by
 * default (TanStack Table v8 default when getSortedRowModel is active).
 *
 * Column alignment (DataTable meta.align):
 *   - "num"  → JetBrains Mono, right-aligned, tabular-nums. Used for every
 *              value the eye scans vertically: played, wins, goals, points, GD.
 *   - "text" → Inter body, left-aligned. Used for the Club name identifier.
 *   - omitted → defaults to "text" in DataTable.
 * The position (#) column is positional/numeric so it uses "num".
 */

import { type ColumnDef } from "@tanstack/solid-table";
import type { StandingsRow } from "../types";

export const leagueColumns: ColumnDef<StandingsRow>[] = [
  {
    id: "position",
    header: "#",
    // Post-T2-6 silent-failure-hunter P0 fix: TanStack Table v8's `row.index`
    // is the original-data-array index, NOT the post-sort visual position.
    // Using `row.index + 1` here would silently render WRONG positions after
    // any user-driven sort — e.g. a club that arrived 7th in the API but is
    // now visually 2nd would display "7" next to its name.
    //
    // The canonical v8 pattern for "current visual position" is to find the
    // row by its STABLE `id` field in the current sorted row model. Using
    // `indexOf(row)` returns -1 because the row object the cell receives is
    // a different reference than the ones in `getRowModel().rows` (TanStack
    // wraps + memoizes rows separately per call site). `findIndex(r => r.id
    // === row.id)` matches by identity-of-meaning instead of reference.
    cell: (info) =>
      info.table.getRowModel().rows.findIndex((r) => r.id === info.row.id) + 1,
    enableSorting: false,
    meta: { align: "num" },
  },
  {
    accessorKey: "clubName",
    header: "Club",
    meta: { align: "text" },
  },
  {
    accessorKey: "played",
    header: "P",
    meta: { align: "num" },
  },
  {
    accessorKey: "wins",
    header: "W",
    meta: { align: "num" },
  },
  {
    accessorKey: "draws",
    header: "D",
    meta: { align: "num" },
  },
  {
    accessorKey: "losses",
    header: "L",
    meta: { align: "num" },
  },
  {
    accessorKey: "goalsFor",
    header: "GF",
    meta: { align: "num" },
  },
  {
    accessorKey: "goalsAgainst",
    header: "GA",
    meta: { align: "num" },
  },
  {
    accessorKey: "goalDifference",
    header: "GD",
    meta: { align: "num" },
  },
  {
    accessorKey: "points",
    header: "Pts",
    meta: { align: "num" },
  },
];

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
  },
  {
    accessorKey: "clubName",
    header: "Club",
  },
  {
    accessorKey: "played",
    header: "P",
  },
  {
    accessorKey: "wins",
    header: "W",
  },
  {
    accessorKey: "draws",
    header: "D",
  },
  {
    accessorKey: "losses",
    header: "L",
  },
  {
    accessorKey: "goalsFor",
    header: "GF",
  },
  {
    accessorKey: "goalsAgainst",
    header: "GA",
  },
  {
    accessorKey: "goalDifference",
    header: "GD",
  },
  {
    accessorKey: "points",
    header: "Pts",
  },
];

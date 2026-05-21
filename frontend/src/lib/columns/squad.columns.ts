/*
 * Squad column definitions — T2-7.
 *
 * Four columns: Player, Role, Region, Phenotype labels.
 * No Age/Contract — those are T4+ career-roster state absent from PlayerBio.
 *
 * Separated from Squad.tsx per Frontend/RULES.md §3 ("Column defs in
 * dedicated *.columns.ts files alongside the route").
 *
 * The phenotype labels column renders a comma-joined human-readable string —
 * never raw JSON (e.g. '["Explosive first step","Poacher"]') and never
 * raw enum identifiers (e.g. "ExplosiveFirstStep").
 */

import { type ColumnDef, type CellContext } from "@tanstack/solid-table";
import type { SquadPlayer } from "../types";

/**
 * Render the player name as a link to their detail page.
 *
 * Uses a plain `<a>` tag so keyboard navigation works without extra ARIA.
 * `href` is `/player/<playerId>` (the route registered in App.tsx at T3-6).
 * encodeURIComponent guards against special characters in content-pack IDs
 * (e.g. colons in `fwh.core:player_00001`).
 */
function playerNameCell(info: CellContext<SquadPlayer, string>): HTMLElement {
  const a = document.createElement("a");
  a.href = `/player/${encodeURIComponent(info.row.original.playerId)}`;
  a.textContent = info.getValue();
  a.className =
    "text-pitch-600 dark:text-pitch-300 hover:underline focus:underline";
  return a;
}

export const squadColumns: ColumnDef<SquadPlayer>[] = [
  {
    accessorKey: "name",
    header: "Player",
    // T3-6: name cell links to /player/:id. Keyboard-accessible via <a> href.
    cell: playerNameCell,
  },
  {
    accessorKey: "role",
    header: "Role",
  },
  {
    accessorKey: "birthRegion",
    header: "Region",
  },
  {
    accessorKey: "phenotypeLabels",
    header: "Traits",
    // Render the string[] as readable comma-joined text.
    // Do NOT render raw JSON — that would expose implementation details to
    // the player. `toString()` on an array gives comma-joined without brackets.
    cell: (info) => {
      const labels = info.getValue<string[]>();
      return labels.length > 0 ? labels.join(", ") : "—";
    },
    // Disable sorting on the array column — sort semantics are undefined for
    // multi-value cells and would mislead the user.
    enableSorting: false,
  },
];

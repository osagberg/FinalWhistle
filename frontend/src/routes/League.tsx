import { type ColumnDef } from "@tanstack/solid-table";
import type { JSX } from "solid-js";
import DataTable from "~/components/DataTable";
import type { LeagueStanding } from "~/lib/types";

// Three stub columns per the T0-2 spec. Real columns (P / W / D / L / GF / GA
// / GD / Pts) land at T2-6 once `get_league_standings` returns real data.
const columns: ColumnDef<LeagueStanding>[] = [
  {
    accessorKey: "position",
    header: "#",
  },
  {
    accessorKey: "clubName",
    header: "Club",
  },
  {
    accessorKey: "points",
    header: "Pts",
  },
];

export default function League(): JSX.Element {
  const data: LeagueStanding[] = [];

  return (
    <div class="space-y-4">
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">League</h1>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          T0 placeholder — not yet implemented. Real standings land at T2-6.
        </p>
      </header>
      <DataTable
        columns={columns}
        data={data}
        emptyMessage="Standings populate once a season has been generated (T2-2)."
      />
    </div>
  );
}

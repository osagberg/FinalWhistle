import { type ColumnDef } from "@tanstack/solid-table";
import type { JSX } from "solid-js";
import DataTable from "~/components/DataTable";

// T2-6 stub: LeagueStanding type lives inline until the season controller
// ships real standings data. Deleted from lib/types.ts at T1-5 alongside
// the other T0-2 placeholder IPC stubs.
interface LeagueStandingStub {
  position: number;
  clubName: string;
  points: number;
}

// Three stub columns per the T0-2 spec. Real columns (P / W / D / L / GF / GA
// / GD / Pts) land at T2-6 once the season controller is wired.
const columns: ColumnDef<LeagueStandingStub>[] = [
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
  const data: LeagueStandingStub[] = [];

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

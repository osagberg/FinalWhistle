import { type ColumnDef } from "@tanstack/solid-table";
import type { JSX } from "solid-js";
import DataTable from "~/components/DataTable";

// T2-7 stub: PlayerSummary type lives inline until get_squad is wired.
// Deleted from lib/types.ts at T1-5 alongside the other T0-2 placeholder
// IPC stubs.
interface PlayerSummaryStub {
  name: string;
  age: number;
  role: string;
}

// Three stub columns per the T0-2 spec. Real columns + phenotype rendering
// land at T2-7 once the season controller wires up get_squad.
const columns: ColumnDef<PlayerSummaryStub>[] = [
  {
    accessorKey: "name",
    header: "Player",
  },
  {
    accessorKey: "age",
    header: "Age",
  },
  {
    accessorKey: "role",
    header: "Role",
  },
];

export default function Squad(): JSX.Element {
  // Empty data at scaffold time. Switch to a Tauri resource when fw-tauri
  // wires up get_squad at T2-7.
  const data: PlayerSummaryStub[] = [];

  return (
    <div class="space-y-4">
      <header>
        <h1 class="font-display text-3xl text-pitch-600 dark:text-pitch-300">Squad</h1>
        <p class="mt-1 text-sm text-ink-subtle dark:text-paper-subtle">
          T0 placeholder — not yet implemented. Real squad lands at T2-7.
        </p>
      </header>
      <DataTable
        columns={columns}
        data={data}
        emptyMessage="Squad data lands at T2-7."
      />
    </div>
  );
}

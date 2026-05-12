import { type ColumnDef } from "@tanstack/solid-table";
import type { JSX } from "solid-js";
import DataTable from "~/components/DataTable";
import type { PlayerSummary } from "~/lib/types";

// Three stub columns per the T0-2 spec. Real columns + phenotype rendering
// land at T2-7 once `get_squad` returns real data.
const columns: ColumnDef<PlayerSummary>[] = [
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
  // wires up get_squad.
  const data: PlayerSummary[] = [];

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

/*
 * Squad column definitions — T4-2.5h.
 *
 * Roster-based columns: Player (name), Role (derived from slot), Apps,
 * Goals, Minutes. Region and Traits are dropped — they live on PlayerBio,
 * not on the roster PlayerInstance, and are accessible via the Player detail
 * page (get_player_detail).
 *
 * Role is derived from the SQUAD slot (0–21 within one club, per
 * `SLOTS_PER_CLUB = 22`). These are squad-depth slots, NOT match slots —
 * a single club's roster has no "away XI", so there is no slot-11 shift.
 * The starting XI (slots 0–10) maps to the 4-3-3 formation; slots 11–21 are
 * bench/squad depth with no formation position until a real lineup model
 * exists (a later row):
 *   slot 0       → GK
 *   slots 1–4    → DEF
 *   slots 5–7    → MID
 *   slots 8–10   → FWD
 *   slots 11–21  → Sub (bench/depth — NOT a fabricated formation position)
 *
 * Row virtualization is not needed — 22 rows is well below the 50-row
 * threshold (Frontend/RULES.md §3).
 *
 * Separated from Squad.tsx per Frontend/RULES.md §3 ("Column defs in
 * dedicated *.columns.ts files alongside the route").
 */

import { type CellContext, type ColumnDef } from "@tanstack/solid-table";
import type { PlayerRosterDto, SquadPlayer } from "../types";

/** Closed set of position labels a squad slot can render as. */
export type SquadRole = "GK" | "DEF" | "MID" | "FWD" | "Sub";

/**
 * Derive a position label from a SQUAD slot index (0–21 within one club).
 *
 * Starting XI (0–10) → 4-3-3 formation; bench/depth (11–21) → `"Sub"`.
 * There is NO slot-11 away-shift: a single club's roster is not two teams,
 * so slots 11–21 are this club's reserves, not a second starting XI.
 */
function slotToRole(slot: number): SquadRole {
  if (slot >= 11) return "Sub";
  if (slot === 0) return "GK";
  if (slot >= 1 && slot <= 4) return "DEF";
  if (slot >= 5 && slot <= 7) return "MID";
  return "FWD";
}

export const rosterColumns: ColumnDef<PlayerRosterDto>[] = [
  {
    accessorKey: "name",
    header: "Player",
  },
  {
    id: "role",
    header: "Role",
    accessorFn: (row) => slotToRole(row.slot),
  },
  {
    accessorKey: "appearances",
    header: "Apps",
  },
  {
    accessorKey: "goals",
    header: "Goals",
  },
  {
    accessorKey: "minutesPlayed",
    header: "Minutes",
  },
];

// Legacy bio-pool columns retained for the old `getSquad` path (still used by
// some tests). Not used by the T4-2.5h roster view. Imports hoisted to the top.
// TODO(T4-F4): retire `squadColumns` + `getSquad` once no route/test uses them.
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
    cell: (info) => {
      const labels = info.getValue<string[]>();
      return labels.length > 0 ? labels.join(", ") : "—";
    },
    enableSorting: false,
  },
];

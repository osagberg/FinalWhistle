/*
 * DataTable.test.tsx — keyboard accessibility + ARIA coverage.
 *
 * Verifies:
 *   - Sortable headers have tabindex="0", role="columnheader", and
 *     aria-sort="none" before any user interaction.
 *   - Non-sortable headers do NOT have tabindex or aria-sort attributes.
 *   - Enter key on a sortable header toggles sort direction; aria-sort updates.
 *   - Space key on a sortable header toggles sort direction; aria-sort updates.
 *   - Mouse click still toggles sort (regression guard for existing behaviour).
 *   - After first keyboard sort (desc), second keyboard sort (asc) updates
 *     aria-sort to "ascending".
 */

import { describe, expect, it } from "vitest";
import { render, screen, fireEvent, waitFor } from "@solidjs/testing-library";
import { type ColumnDef } from "@tanstack/solid-table";
import DataTable from "~/components/DataTable";

// ---------------------------------------------------------------------------
// Minimal fixture data + column definitions
// ---------------------------------------------------------------------------

interface Row {
  name: string;
  score: number;
}

// Plain object syntax matches how the real column files are authored in this
// project (league.columns.ts, squad.columns.ts) and avoids the covariance
// issue with createColumnHelper + exactOptionalPropertyTypes.
const SORTABLE_COLUMNS: ColumnDef<Row>[] = [
  // Not sortable — enableSorting: false
  {
    accessorKey: "name",
    header: "Name",
    enableSorting: false,
  },
  // Sortable — default (enableSorting implicitly true)
  {
    accessorKey: "score",
    header: "Score",
  },
];

const FIXTURE_ROWS: Row[] = [
  { name: "Ashby", score: 7 },
  { name: "Thornton", score: 3 },
  { name: "Harlow", score: 9 },
];

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("DataTable keyboard accessibility", () => {
  it("sortable header has tabindex=0, role=columnheader, and aria-sort=none before sort", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByRole("columnheader", { name: "Score" })).toBeInTheDocument());

    const scoreHeader = screen.getByRole("columnheader", { name: "Score" });
    expect(scoreHeader).toHaveAttribute("tabindex", "0");
    expect(scoreHeader).toHaveAttribute("aria-sort", "none");
  });

  it("non-sortable header does not have tabindex or aria-sort", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByRole("columnheader", { name: "Name" })).toBeInTheDocument());

    const nameHeader = screen.getByRole("columnheader", { name: "Name" });
    expect(nameHeader).not.toHaveAttribute("tabindex");
    expect(nameHeader).not.toHaveAttribute("aria-sort");
  });

  it("Enter key on sortable header toggles sort descending; aria-sort becomes 'descending'", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByRole("columnheader", { name: "Score" })).toBeInTheDocument());

    const scoreHeader = screen.getByRole("columnheader", { name: "Score" });
    fireEvent.keyDown(scoreHeader, { key: "Enter" });

    await waitFor(() => {
      expect(scoreHeader).toHaveAttribute("aria-sort", "descending");
    });
  });

  it("Space key on sortable header toggles sort descending; aria-sort becomes 'descending'", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByRole("columnheader", { name: "Score" })).toBeInTheDocument());

    const scoreHeader = screen.getByRole("columnheader", { name: "Score" });
    fireEvent.keyDown(scoreHeader, { key: " " });

    await waitFor(() => {
      expect(scoreHeader).toHaveAttribute("aria-sort", "descending");
    });
  });

  it("mouse click still sorts (regression guard); aria-sort updates correctly", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByRole("columnheader", { name: "Score" })).toBeInTheDocument());

    const scoreHeader = screen.getByRole("columnheader", { name: "Score" });
    fireEvent.click(scoreHeader);

    await waitFor(() => {
      expect(scoreHeader).toHaveAttribute("aria-sort", "descending");
    });
  });

  it("second keyboard sort (Enter) flips direction; aria-sort becomes 'ascending'", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByRole("columnheader", { name: "Score" })).toBeInTheDocument());

    const scoreHeader = screen.getByRole("columnheader", { name: "Score" });

    // First Enter → descending
    fireEvent.keyDown(scoreHeader, { key: "Enter" });
    await waitFor(() => {
      expect(scoreHeader).toHaveAttribute("aria-sort", "descending");
    });

    // Second Enter → ascending
    fireEvent.keyDown(scoreHeader, { key: "Enter" });
    await waitFor(() => {
      expect(scoreHeader).toHaveAttribute("aria-sort", "ascending");
    });
  });

  it("keyboard sort reorders rows (data integrity)", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByText("Ashby")).toBeInTheDocument());

    // Sort descending by Score: Harlow(9), Ashby(7), Thornton(3).
    const scoreHeader = screen.getByRole("columnheader", { name: "Score" });
    fireEvent.keyDown(scoreHeader, { key: "Enter" });

    await waitFor(() => {
      const rows = screen
        .getByRole("table")
        .querySelectorAll("tbody tr");
      expect(rows[0]?.textContent).toContain("Harlow");
      expect(rows[2]?.textContent).toContain("Thornton");
    });
  });
});

/*
 * DataTable.test.tsx — keyboard accessibility, ARIA, column alignment,
 * and row-highlight coverage.
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
 *   - "num" column meta → header and cell have text-right + font-mono classes.
 *   - "text" column meta (or omitted) → header and cell have text-left + font-body classes.
 *   - rowHighlight predicate → matching row has border-l-pitch-500 class + aria-current="true".
 *   - rowHighlight=undefined → no row has aria-current set.
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
    meta: { align: "text" },
  },
  // Sortable — default (enableSorting implicitly true)
  {
    accessorKey: "score",
    header: "Score",
    meta: { align: "num" },
  },
];

const FIXTURE_ROWS: Row[] = [
  { name: "Ashby", score: 7 },
  { name: "Thornton", score: 3 },
  { name: "Harlow", score: 9 },
];

// ---------------------------------------------------------------------------
// Keyboard accessibility tests
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

// ---------------------------------------------------------------------------
// Column alignment tests
// ---------------------------------------------------------------------------

describe("DataTable column alignment", () => {
  it("'num' column header has text-right class", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByRole("columnheader", { name: "Score" })).toBeInTheDocument());

    const scoreHeader = screen.getByRole("columnheader", { name: "Score" });
    expect(scoreHeader.className).toContain("text-right");
  });

  it("'text' column header has text-left class", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByRole("columnheader", { name: "Name" })).toBeInTheDocument());

    const nameHeader = screen.getByRole("columnheader", { name: "Name" });
    expect(nameHeader.className).toContain("text-left");
  });

  it("'num' column cells have font-mono class", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByText("Ashby")).toBeInTheDocument());

    const table = screen.getByRole("table");
    // First data row score cell (second td in each row).
    const firstRowCells = table.querySelectorAll("tbody tr:first-child td");
    const scoreCell = firstRowCells[1];
    expect(scoreCell?.className).toContain("font-mono");
  });

  it("'text' column cells have font-body class", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByText("Ashby")).toBeInTheDocument());

    const table = screen.getByRole("table");
    // First data row name cell (first td in each row).
    const firstRowCells = table.querySelectorAll("tbody tr:first-child td");
    const nameCell = firstRowCells[0];
    expect(nameCell?.className).toContain("font-body");
  });
});

// ---------------------------------------------------------------------------
// Row highlight tests
// ---------------------------------------------------------------------------

describe("DataTable row highlight", () => {
  it("row matching rowHighlight predicate has aria-current='true'", async () => {
    render(() => (
      <DataTable
        columns={SORTABLE_COLUMNS}
        data={FIXTURE_ROWS}
        rowHighlight={(row) => row.name === "Ashby"}
      />
    ));

    await waitFor(() => expect(screen.getByText("Ashby")).toBeInTheDocument());

    const table = screen.getByRole("table");
    const rows = table.querySelectorAll("tbody tr");

    // Ashby is the first row in the unsorted fixture.
    expect(rows[0]).toHaveAttribute("aria-current", "true");
    // Thornton and Harlow are not highlighted.
    expect(rows[1]).not.toHaveAttribute("aria-current");
    expect(rows[2]).not.toHaveAttribute("aria-current");
  });

  it("highlighted row has pitch-accent left-border class", async () => {
    render(() => (
      <DataTable
        columns={SORTABLE_COLUMNS}
        data={FIXTURE_ROWS}
        rowHighlight={(row) => row.name === "Thornton"}
      />
    ));

    await waitFor(() => expect(screen.getByText("Thornton")).toBeInTheDocument());

    const table = screen.getByRole("table");
    const rows = table.querySelectorAll("tbody tr");

    // Thornton is the second row.
    expect(rows[1]?.className).toContain("border-l-pitch-500");
    // Non-highlighted rows should not have the accent border.
    expect(rows[0]?.className).not.toContain("border-l-pitch-500");
  });

  it("no rows have aria-current when rowHighlight is omitted", async () => {
    render(() => <DataTable columns={SORTABLE_COLUMNS} data={FIXTURE_ROWS} />);

    await waitFor(() => expect(screen.getByText("Ashby")).toBeInTheDocument());

    const table = screen.getByRole("table");
    const rows = table.querySelectorAll("tbody tr");

    for (const row of Array.from(rows)) {
      expect(row).not.toHaveAttribute("aria-current");
    }
  });
});

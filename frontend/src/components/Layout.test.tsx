/*
 * Layout — Vitest tests for the persistent vertical sidebar shell.
 *
 * Coverage:
 *   SHELL-1  TopStrip renders the FINAL WHISTLE wordmark.
 *   SHELL-2  TopStrip renders a theme-toggle button.
 *   SHELL-3  The sidebar nav renders as a <nav> with aria-label="Primary".
 *   SHELL-4  Every NAV item renders as a link with the correct href.
 *   SHELL-5  No hotkey hint spans ([1], [M] etc.) appear in nav labels.
 *   SHELL-6  Career-context card is NOT rendered when no career is active.
 *   SHELL-7  Career-context card IS rendered when a career is active (club + season).
 *   SHELL-8  No em-dash placeholder ("—") appears for any career context slot.
 *   SHELL-9  "Next fixture" label does NOT appear when no fixture data exists.
 *   SHELL-10 "determinism: pinned" dev string is NOT in the player-facing shell.
 *   SHELL-11 Sidebar is always visible — no hamburger or hidden class.
 *   SHELL-12 TopStrip career context omitted when no career active.
 *   SHELL-13 TopStrip career context rendered (club + season) when career active.
 *
 * Mocking strategy:
 *   - ~/lib/state is mocked to control career signal values per test.
 *   - ~/lib/api/settings is mocked so setSettings never throws.
 *   - @tauri-apps/api/core mocked so invoke() never throws outside Tauri.
 *   - @solidjs/router stubbed so <A> renders an anchor without routing errors.
 */

import { describe, expect, it, vi, beforeEach, afterEach } from "vitest";
import { render, screen } from "@solidjs/testing-library";
import { type JSX } from "solid-js";

// ---------------------------------------------------------------------------
// Module mocks — hoisted before component imports
// ---------------------------------------------------------------------------

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn().mockResolvedValue({}),
}));

vi.mock("~/lib/api/settings", () => ({
  setSettings: vi.fn().mockResolvedValue(undefined),
}));

// We mock state module so tests control career-active signals cleanly.
vi.mock("~/lib/state", () => ({
  theme: vi.fn(() => "light" as "light" | "dark"),
  setTheme: vi.fn(),
  reduceMotion: vi.fn(() => false),
  isCareerActive: vi.fn(() => false),
  managedClubName: vi.fn(() => null as string | null),
  seasonNumber: vi.fn(() => null as number | null),
  selectedClubId: vi.fn(() => null as string | null),
  setCareerId: vi.fn(),
  setSelectedClubId: vi.fn(),
  setManagedClubName: vi.fn(),
  setSeasonNumber: vi.fn(),
  setThemeSignal: vi.fn(),
  setReduceMotion: vi.fn(),
}));

// Stub @solidjs/router so <A> renders as a plain <a> without a router context.
vi.mock("@solidjs/router", () => ({
  A: (props: {
    href: string;
    class?: string;
    activeClass?: string;
    end?: boolean;
    "aria-current"?: string;
    children: JSX.Element;
  }) => (
    <a href={props.href} class={props.class}>
      {props.children}
    </a>
  ),
}));

// Import AFTER mocks are hoisted.
import Layout from "./Layout";
import {
  isCareerActive,
  managedClubName,
  seasonNumber,
} from "~/lib/state";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function renderLayout(): ReturnType<typeof render> {
  return render(() => (
    <Layout>
      <div>page content</div>
    </Layout>
  ));
}

// ---------------------------------------------------------------------------
// State mock helpers
// ---------------------------------------------------------------------------

function setCareerInactive(): void {
  vi.mocked(isCareerActive).mockReturnValue(false);
  vi.mocked(managedClubName).mockReturnValue(null);
  vi.mocked(seasonNumber).mockReturnValue(null);
}

function setCareerActive(club: string, season: number): void {
  vi.mocked(isCareerActive).mockReturnValue(true);
  vi.mocked(managedClubName).mockReturnValue(club);
  vi.mocked(seasonNumber).mockReturnValue(season);
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

beforeEach(() => {
  setCareerInactive();
});

afterEach(() => {
  vi.clearAllMocks();
});

describe("Layout shell — TopStrip", () => {
  it("SHELL-1: renders the FINAL WHISTLE wordmark", () => {
    renderLayout();
    expect(screen.getByText("FINAL WHISTLE")).toBeInTheDocument();
  });

  it("SHELL-2: renders a theme-toggle button with aria-label", () => {
    renderLayout();
    const btn = screen.getByRole("button", { name: /toggle theme/i });
    expect(btn).toBeInTheDocument();
  });

  it("SHELL-12: TopStrip career context is omitted when no career active", () => {
    setCareerInactive();
    renderLayout();
    // Club name and season must not appear in the top strip when inactive.
    // The wordmark and the theme button are the only strip contents.
    expect(screen.queryByText(/season \d+/i)).not.toBeInTheDocument();
  });

  it("SHELL-13: TopStrip career context shows club + season when career active", () => {
    setCareerActive("Northshire Town", 3);
    renderLayout();
    // Club name appears in both the TopStrip and the career-context card —
    // getAllByText is correct here; we assert at least one element is present.
    const clubEls = screen.getAllByText("Northshire Town");
    expect(clubEls.length).toBeGreaterThanOrEqual(1);
    expect(screen.getAllByText(/season 3/i).length).toBeGreaterThanOrEqual(1);
  });
});

describe("Layout shell — PrimarySidebar nav", () => {
  it("SHELL-3: sidebar nav has role=navigation with aria-label='Primary'", () => {
    renderLayout();
    const nav = screen.getByRole("navigation", { name: "Primary" });
    expect(nav).toBeInTheDocument();
  });

  it("SHELL-4a: Home link renders with href='/'", () => {
    renderLayout();
    const link = screen.getByRole("link", { name: "Home" });
    expect(link).toHaveAttribute("href", "/");
  });

  it("SHELL-4b: Squad link renders with href='/squad'", () => {
    renderLayout();
    const link = screen.getByRole("link", { name: "Squad" });
    expect(link).toHaveAttribute("href", "/squad");
  });

  it("SHELL-4c: Tactics link renders with href='/tactics'", () => {
    renderLayout();
    const link = screen.getByRole("link", { name: "Tactics" });
    expect(link).toHaveAttribute("href", "/tactics");
  });

  it("SHELL-4d: League link renders with href='/league'", () => {
    renderLayout();
    const link = screen.getByRole("link", { name: "League" });
    expect(link).toHaveAttribute("href", "/league");
  });

  it("SHELL-4e: Settings link renders with href='/settings'", () => {
    renderLayout();
    const link = screen.getByRole("link", { name: "Settings" });
    expect(link).toHaveAttribute("href", "/settings");
  });

  it("SHELL-5: no hotkey hint spans ([1], [M] etc.) appear in nav labels", () => {
    renderLayout();
    // Check that none of the nav link text contains bracket-wrapped characters.
    const nav = screen.getByRole("navigation", { name: "Primary" });
    expect(nav.textContent).not.toMatch(/\[\w\]/);
  });
});

describe("Layout shell — career-context card", () => {
  it("SHELL-6: career-context card NOT rendered when no career active", () => {
    setCareerInactive();
    renderLayout();
    // The card's internal section labels must not appear.
    expect(screen.queryByText("Club")).not.toBeInTheDocument();
    expect(screen.queryByText("Season")).not.toBeInTheDocument();
  });

  it("SHELL-7: career-context card rendered with club + season when career active", () => {
    setCareerActive("Aardvark United", 2);
    renderLayout();
    // Club name appears in both the TopStrip and the sidebar card — getAllByText
    // is the right query here; we assert at least one is in the document.
    expect(screen.getAllByText("Aardvark United").length).toBeGreaterThanOrEqual(1);
    // Season "2" appears in both the TopStrip "Season 2" span and the sidebar card.
    expect(screen.getAllByText("2").length).toBeGreaterThanOrEqual(1);
  });

  it("SHELL-8: no em-dash placeholder appears in any career context slot", () => {
    // With career inactive, no slots render at all — no em-dash placeholders.
    setCareerInactive();
    renderLayout();
    // The old Sidebar had a hardcoded "—" for next fixture and season fallback.
    expect(screen.queryByText("—")).not.toBeInTheDocument();
  });

  it("SHELL-8b: no em-dash placeholder appears even with career active but partial data", () => {
    // Career is active but seasonNumber is null — omit season slot, no em-dash.
    vi.mocked(isCareerActive).mockReturnValue(true);
    vi.mocked(managedClubName).mockReturnValue("Midton City");
    vi.mocked(seasonNumber).mockReturnValue(null);
    renderLayout();
    expect(screen.queryByText("—")).not.toBeInTheDocument();
  });

  it("SHELL-9: 'Next fixture' label does NOT appear (no fixture data wired)", () => {
    setCareerActive("Northshire Town", 1);
    renderLayout();
    // The next-fixture slot is intentionally absent until the fixture signal exists.
    expect(screen.queryByText(/next fixture/i)).not.toBeInTheDocument();
  });
});

describe("Layout shell — dev diagnostics removed", () => {
  it("SHELL-10: 'determinism: pinned' string is not in player-facing shell", () => {
    renderLayout();
    expect(screen.queryByText(/determinism/i)).not.toBeInTheDocument();
  });

  it("SHELL-10b: 'build: dev' string is not in player-facing shell", () => {
    renderLayout();
    expect(screen.queryByText(/build: dev/i)).not.toBeInTheDocument();
  });
});

describe("Layout shell — sidebar always visible", () => {
  it("SHELL-11: aside element is present in the DOM (no hamburger pattern)", () => {
    renderLayout();
    // The aside element must always be in the document — it is never hidden.
    const aside = document.querySelector("aside");
    expect(aside).toBeInTheDocument();
    // It must not carry a hidden attribute or an aria-hidden on the nav itself.
    expect(aside).not.toHaveAttribute("hidden");
    expect(aside?.getAttribute("aria-hidden")).not.toBe("true");
  });
});

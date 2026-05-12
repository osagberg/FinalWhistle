import type { Config } from "tailwindcss";

// Visual identity per DESIGN_DOC.md §9. Density-first; FM-class info-rich UI;
// muted-pitch-green primary; first-class dark mode (not a recolor).
//
// Type stack (Phase-1 tuning seeds):
//   Display:   Anton              — scorelines, headers
//   Data:      JetBrains Mono     — tables, replay hashes, technical surfaces
//   Body:      Inter              — press, commentary, NPC dialog
//
// Anton + JetBrains Mono + Inter aren't loaded yet; system fallback chains
// render fine for T0-2. Real `@font-face` declarations land at T4-3 in
// `styles.css` (Phase-4 visual-identity lock).

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  darkMode: "class",
  theme: {
    extend: {
      colors: {
        // Pitch greens — muted, slightly desaturated. The "FM-class" rule
        // requires accent green that reads as a club-color cue, not a video-
        // game-grass cue. Hex centred on the DESIGN_DOC §9 seed.
        pitch: {
          50: "#eaf3ed",
          100: "#c6e0ce",
          200: "#9fcaab",
          300: "#76b186",
          400: "#549a65",
          500: "#2d6e3e", // primary accent (DESIGN_DOC seed)
          600: "#235a33",
          700: "#1b4528",
          800: "#13311d",
          900: "#0a1c11",
          950: "#040d07",
        },
        // Accent-flag yellows + reds (state cues, never primary).
        flag: {
          yellow: "#f5c84b",
          red: "#c8412c",
        },
        // Warm-off-white background + neutral charcoal text per DESIGN_DOC §9.
        // The names "paper" + "ink" keep semantic intent legible in markup.
        paper: {
          DEFAULT: "#f7f4ee",
          subtle: "#efeae0",
          bold: "#e6dfd2",
        },
        ink: {
          DEFAULT: "#1d1f1c",
          subtle: "#3d4239",
          mute: "#6b7068",
        },
        // Dark-mode anchors. First-class theme; intentionally not a recolor.
        midnight: {
          DEFAULT: "#0e1411",
          panel: "#161d18",
          subtle: "#1d251f",
          line: "#2a3530",
        },
      },
      fontFamily: {
        display: [
          "Anton",
          "Arial Narrow",
          "Helvetica Neue Condensed",
          "system-ui",
          "sans-serif",
        ],
        body: ["Inter", "system-ui", "-apple-system", "sans-serif"],
        mono: [
          "JetBrains Mono",
          "ui-monospace",
          "SFMono-Regular",
          "Menlo",
          "monospace",
        ],
      },
      // FM-class compact spacing — half-step density between defaults.
      spacing: {
        "0.5": "0.125rem",
        "1.5": "0.375rem",
        "2.5": "0.625rem",
        "3.5": "0.875rem",
      },
      fontSize: {
        // Tighter base than Tailwind's default 16px. Tabular UI shines at 14.
        xs: ["0.75rem", { lineHeight: "1rem" }],
        sm: ["0.8125rem", { lineHeight: "1.125rem" }],
        base: ["0.875rem", { lineHeight: "1.25rem" }],
        lg: ["1rem", { lineHeight: "1.375rem" }],
        xl: ["1.125rem", { lineHeight: "1.5rem" }],
        "2xl": ["1.375rem", { lineHeight: "1.75rem" }],
        "3xl": ["1.75rem", { lineHeight: "2rem" }],
      },
      boxShadow: {
        // Subtle in light mode, almost invisible in dark — UI density wants
        // separation by line, not by drop-shadow halo.
        panel: "0 1px 0 rgba(20, 24, 22, 0.04), 0 1px 2px rgba(20, 24, 22, 0.06)",
      },
      borderRadius: {
        // Two radii only. FM-density doesn't want soft tiles everywhere.
        sm: "2px",
        DEFAULT: "4px",
      },
    },
  },
  plugins: [],
} satisfies Config;

# Final Whistle — Visual Style Guide

The locked visual identity for the Final Whistle frontend (Phase T4, row T4-3).

**Source of truth:** the values below are mirrored from `frontend/tailwind.config.ts`
and `frontend/src/styles.css` — those files are authoritative; this doc is the
human-readable reference. The design intent traces to `docs/DESIGN_DOC.md` §9.
Do not edit hex values or scale steps here without changing the config first.

Design posture: **density-first, FM-class info-rich UI.** Muted pitch-green
primary; warm paper background; first-class dark mode (a real theme, not a
recolor). Separation by thin lines, not drop-shadow halos.

---

## Typography

Three faces, self-hosted via `@fontsource` (no CDN — Tauri-CSP-safe), imported
in `src/main.tsx`. Tailwind classes: `font-display` / `font-body` / `font-mono`.

| Role | Family | Tailwind class | Used for | Weights loaded |
|---|---|---|---|---|
| Display | **Anton** | `font-display` | Wordmark, page + section headers, scorelines | 400 |
| Body | **Inter** | `font-body` | Press, commentary, NPC dialog, prose, controls | 400 / 500 / 600 / 700 |
| Data | **JetBrains Mono** | `font-mono` | Tables, hashes, seeds, status pills, technical surfaces | 400 / 500 / 700 |

Anton is single-weight by design. `h1`/`h2`/`h3` default to `font-display` +
`tracking-tight` (`styles.css`). Body text sets `font-variant-numeric:
tabular-nums` globally so stat tables align without per-cell rules.

### Type scale

Tighter than Tailwind's default — tabular UI reads best at a 14px base.

| Token | Size | Line height |
|---|---|---|
| `text-xs` | 0.75rem (12px) | 1rem |
| `text-sm` | 0.8125rem (13px) | 1.125rem |
| `text-base` | 0.875rem (14px) | 1.25rem |
| `text-lg` | 1rem (16px) | 1.375rem |
| `text-xl` | 1.125rem (18px) | 1.5rem |
| `text-2xl` | 1.375rem (22px) | 1.75rem |
| `text-3xl` | 1.75rem (28px) | 2rem |

---

## Colour

### Pitch — primary accent

Muted, slightly desaturated green — a club-colour cue, not video-game grass.

| Token | Hex | | Token | Hex |
|---|---|---|---|---|
| `pitch-50` | `#eaf3ed` | | `pitch-500` | `#2d6e3e` (primary accent) |
| `pitch-100` | `#c6e0ce` | | `pitch-600` | `#235a33` |
| `pitch-200` | `#9fcaab` | | `pitch-700` | `#1b4528` |
| `pitch-300` | `#76b186` | | `pitch-800` | `#13311d` |
| `pitch-400` | `#549a65` | | `pitch-900` | `#0a1c11` |
| | | | `pitch-950` | `#040d07` |

### Flag — state cues only (never primary)

| Token | Hex |
|---|---|
| `flag-yellow` | `#f5c84b` |
| `flag-red` | `#c8412c` |

### Paper + ink — light mode

| Token | Hex | Role |
|---|---|---|
| `paper` | `#f7f4ee` | Background (warm off-white) |
| `paper-subtle` | `#efeae0` | Recessed surfaces |
| `paper-bold` | `#e6dfd2` | Emphasised surfaces |
| `ink` | `#1d1f1c` | Primary text (neutral charcoal) |
| `ink-subtle` | `#3d4239` | Secondary text |
| `ink-mute` | `#6b7068` | Tertiary text / hairlines |

### Midnight — dark mode

First-class dark theme (`darkMode: "class"` — toggled by the `.dark` class on
`<html>`), intentionally not a recolour of the light palette.

| Token | Hex | Role |
|---|---|---|
| `midnight` | `#0e1411` | Background |
| `midnight-panel` | `#161d18` | Panel surfaces |
| `midnight-subtle` | `#1d251f` | Recessed surfaces |
| `midnight-line` | `#2a3530` | Hairlines / borders |

In dark mode body text uses `paper`; pitch accents shift up the scale
(`pitch-300` on dark vs `pitch-600` on light) to hold contrast.

---

## Spacing, shadow, radius

- **Spacing:** Tailwind defaults plus half-steps `0.5` / `1.5` / `2.5` / `3.5`
  (FM-class compact density).
- **Shadow:** one token, `shadow-panel` — subtle in light, suppressed in dark
  (dark mode separates by line, not halo).
- **Radius:** two only — `rounded-sm` (2px) and `rounded` (4px). FM density does
  not want soft tiles everywhere.

---

## Component classes

Defined in `styles.css` under `@layer components`.

| Class | Purpose |
|---|---|
| `fw-panel` | Card chrome — white (light) / `midnight-panel` (dark), thin border, `shadow-panel`. |
| `fw-nav-link` | Top-nav link — muted resting state, accent when `.active`. |
| `fw-pill` | Status pill — uppercase mono micro-label; pair with `flag-*` / `pitch` for state cues. |

Scrollbars are restyled narrow (8px, `ink-mute`) — dense table panes should not
surrender 16px of width to default Chromium bars.

---

## Reference screenshots

Captured against the dev server with `pnpm screenshots`
(`frontend/scripts/capture-visual-screenshots.mjs` — drives the system Chrome
via `playwright-core`). Regenerate when the identity changes.

| File | Surface |
|---|---|
| `home-light.png` | Home — light mode: wordmark, panels, the paper/ink palette, all three faces. |
| `home-dark.png` | Home — dark mode: the first-class `midnight` theme. |
| `tactical-board.png` | The `/dev/board-preview` tactical board — pitch-green in the game's signature surface. |

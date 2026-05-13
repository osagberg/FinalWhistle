---
description: SolidJS + Tailwind v3 + TanStack Table v8 + PixiJS v8 + ECharts conventions for the Final Whistle frontend.
applies_to:
  - frontend/**
auto_load: when_editing_matching_path
---

# Frontend rules

## §1. SolidJS, not React

- `createSignal` for local state. `createStore` for nested mutable state. `createMemo` for derived values.
- **DO NOT** use React patterns: `useState`, `useEffect`, `useContext`. They are not Solid.
- Components are PascalCase, one per file, default export:
  ```tsx
  export default function Squad() { ... }
  ```
- Props typed via `interface SquadProps { ... }`.

## §2. Tailwind v3 (not v4)

- Utility-first. No custom CSS classes unless component-internal `<style>` is the only option.
- **DO NOT** use `@apply` in shared CSS.
- v4 syntax is forbidden (we're on v3 for stability through T4 — see `MEMORY.md` open question).
- Design tokens live in `tailwind.config.ts`.

## §3. TanStack Table v8

- Column defs in dedicated `*.columns.ts` files alongside the route:
  ```
  frontend/src/routes/Squad.tsx
  frontend/src/lib/columns/squad.columns.ts
  ```
- Row virtualization for any table with ≥50 rows.
- Sorting + filtering enabled by default on management screens.
- v8 only — v9 is not stable.

## §4. PixiJS v8

- Pixi `Application` created **once** in `onMount`, destroyed in `onCleanup`. Never on every signal change.
- Render loop driven by the Pixi ticker, not by Solid effects. Avoid `pixi.render()` inside `createEffect`.
- Scene-graph updates: diff against the previous frame; don't rebuild.
- Texture loading: cache by URL; reuse `Texture` instances across components.

## §5. ECharts

- Lazy import: `const echarts = await import('echarts')` to avoid bundling on routes that don't use it.
- Chart instances disposed on `onCleanup`.
- `dataZoom` for any time-series with >50 data points.

## §6. TypeScript discipline

- No `any`. Use `unknown` + narrowing.
- ESLint v8 + Prettier configured. `pnpm lint` clean before commit.
- Strict mode in `tsconfig.json`.

## §7. IPC types

- Types in `frontend/src/lib/types.ts` mirror Rust DTOs from `fw-tauri`.
- `import { invoke } from '@tauri-apps/api/core'` for calling commands.
- Always `await` IPC calls; never fire-and-forget.

## §8. Accessibility

- Keyboard navigation on every interactive surface.
- ARIA labels on icon-only buttons.
- Contrast: meet WCAG AA on text/background pairs.
- Screen-reader test with VoiceOver (Mac) before merging any dense table change.

## §9. Banned terms apply to UI strings

- Football-native vocabulary only.
- No capitalized mystical state-nouns ("The Hush", "Awakened").
- No "+5 Finishing" tooltips — surface as commentary ("looks sharper").
- `scripts/fw banned-terms` scans `frontend/src/`. Sentinel exemption per `Content/RULES.md`.

## §10. Routing

- File-based routes under `frontend/src/routes/`.
- Lazy-load route components for any screen >100 LoC.

## Cross-references

- `CLAUDE.md` §3 (frontend stack), §7 (UI never drives canonical state)
- `Tauri/RULES.md` — Rust side of the IPC boundary
- `Content/RULES.md` — banned terms catalog

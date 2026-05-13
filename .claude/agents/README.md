# .claude/agents — Final Whistle subagent roster

Seven agents cover the project after the 2026-05-13 Unity→Rust+Tauri pivot. Main thread coordinates; subagents do the focused work.

Self-review (`pr-review-toolkit:silent-failure-hunter` + `pr-review-toolkit:type-design-analyzer` + `feature-dev:code-reviewer`) is **mandatory** on any change ≥100 LoC. See `CLAUDE.md` §5.

| Agent | When to invoke |
|---|---|
| `producer` | Phase-gate, sprint planning, scope negotiation, risk register, Codex review handoff. |
| `lead-programmer` | Code review on ≥100 LoC, API design between crates, refactoring strategy, crate-boundary decisions, Rust-idiom enforcement. |
| `gameplay-programmer` | Match-sim implementation in `fw-match-sim` / `fw-memory` / `fw-replay` / `fw-core` — behavior trees, ball physics, signature triggers, canonical encoders. |
| `systems-designer` | Formulas, curves, salience weights, scouting-uncertainty math, economy balance, signature trigger thresholds. |
| `narrative-director` | Event-sourced memory readers, Tracery grammars, commentary phrase banks, the `docs/design/ui-vocabulary.md` catalog, banned-terms vocabulary. |
| `ui-programmer` | Tauri IPC handlers, SolidJS components, TanStack Table screens, PixiJS 2D tactical board, ECharts analytics, Tailwind v3 styling. |
| `qa-lead` | Acceptance criteria, regression coverage (insta + proptest + canonical-hash gate), FW-VAL content validation, save-migration fixtures, phase-gate quality checks. |

## Task-class → required agent (per `CLAUDE.md` §5)

| Task class | Indicator | Agent |
|---|---|---|
| sim-rust | ≥100 LoC in fw-match-sim/fw-memory/fw-replay/fw-save/fw-core/fw-content/fw-scouting, canonical-state-emitting | `gameplay-programmer` |
| balance-formulas | numeric coefficients, signature thresholds, gene curves, scouting math | `systems-designer` |
| content-narrative | RON authoring, Tracery grammars, memory readers, commentary, banned-terms vocab | `narrative-director` |
| architecture-cross-crate | new crate, API boundary change, Tauri command surface, save schema bump, ADR | `lead-programmer` |
| frontend-ui | SolidJS components, Tauri IPC handlers, TanStack tables, PixiJS board, ECharts | `ui-programmer` |
| qa-tests | insta snapshots, proptest invariants, FW-VAL checks, save-migration fixtures | `qa-lead` |
| phase-coordination | gate check, milestone review, scope negotiation, risk register, Codex handoff | `producer` |

Single-file edits ≤100 LoC may stay on the main thread.

## Voice + format conventions

- Reports under 250 words.
- Always name file(s) touched with absolute paths.
- Cite `CLAUDE.md` section numbers when invoking project rules.
- No emojis. Football-native vocabulary in player-facing copy (per `narrative-director`).
- No premature abstractions; three similar call sites is the threshold.

## Dropped from blueprint (Unity-era cruft)

- `art-director`, `creative-director`, `engine-programmer`, `game-designer`, `technical-director`, `unity-specialist`, `unity-ui-specialist` — collapsed into the seven above or dropped entirely (no 3D, no Unity).

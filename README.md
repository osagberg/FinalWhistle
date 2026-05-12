# Final Whistle

> Deep procedural football management simulation. Rust + Tauri 2 + SolidJS.
> Solo-dev + Claude collaboration. Mac-first; ships Mac / Windows / Linux + Steam Deck.

## What this is

Football Manager-killer scope. The pitch:

- **Procedural fantasy worlds** — every save is a different football universe with its own cultures, leagues, players, histories. No licensing constraints.
- **Careers that remember** — event-sourced memory; players, clubs, managers carry personal history across decades. Old decisions surface years later as rivals, legends, regrets.
- **Deep simulation, text-first presentation** — match engine simulates 22 players + ball at 60Hz with deterministic Q32.32 fixed-point math. Match-day surface is a 2D tactical board + live text recap, not a 3D viewer.
- **Breakthrough-driven player development** — rare narrative growth moments redraw a player's ceilings; not a linear XP grind.
- **Mod-friendly content** — names, cultures, archetypes, grammars are RON files. Steam Workshop on day one.

See `docs/DESIGN_DOC.md` for the full pitch. See `docs/MASTER_PLAN.md` for the build plan.

## Status

Phase **T0** (Scaffold). Determinism gate not yet locked (placeholder hash; fills on first CI green pass). No gameplay yet.

## Quick start

```sh
# install once
brew install rust just node pnpm  # plus rustup if needed
rustup default stable

# develop
just dev        # launch Tauri dev mode
just test       # cargo test --workspace
just lint       # cargo clippy + fmt + frontend lint
just ci-local   # full pre-PR check (lint + test)
```

Or via the bash front-door: `scripts/fw verify` / `scripts/fw dev` / `scripts/fw test`.

## Project structure

```
crates/                   # Rust workspace (8 crates)
  fw-core/               # Q32 fixed-point, Seed, Tick, ID types — zero deps
  fw-match-sim/          # 22-player deterministic match sim
  fw-content/            # Content schema + runtime sampling
  fw-content-baker/      # Bake-time LLM corpus generator (CLI binary)
  fw-scouting/           # Scout uncertainty model
  fw-memory/             # Event-sourced career memory ledger
  fw-replay/             # Canonical hash gate + corpus fixtures
  fw-save/               # Bincode save format + version migrations
  fw-tauri/              # Tauri command handlers (frontend bridge)
src-tauri/               # Tauri shell binary
frontend/                # SolidJS + TypeScript + Tailwind UI
content/
  sources/               # Hand-authored seeds + prompt templates
  baked/                 # LLM-generated corpus (gitignored — regen via `just bake-content`)
  mods/                  # Mod overlays
docs/
  DESIGN_DOC.md          # Stable design contract
  MASTER_PLAN.md         # Tiered execution plan (T0 → T5)
  CONTENT_PIPELINE.md    # Bake-time + runtime content spec
  specs/                 # Detailed per-system specs
  design/                # Design sub-docs (ui-vocabulary, etc)
  archive/               # Historical: MIGRATION_AUDIT from the v1 pivot
.claude/                 # Claude Code: commands, skills, hooks
.github/workflows/       # CI matrix [macos-14, windows-latest, ubuntu-22.04]
Justfile                 # dev task automation
scripts/fw               # bash front-door (verify, dev, test, lint, clean, bake)
```

## Development workflow

Single primary command: **`/next`** (Claude). Reads `docs/MASTER_PLAN.md`, picks first TODO whose deps are DONE, ships it end-to-end. See `.claude/skills/next/SKILL.md` for the full workflow.

Phase-gate Codex review at phase boundaries via PR (`.github/PULL_REQUEST_TEMPLATE.md`).

## Provenance

This is a pivot from the Unity + C# version of Final Whistle. The pre-pivot state is preserved:

- **Git tag**: `v0-pre-pivot-2026-05-13` — last C# commit on this repo
- **Sibling archive**: `/Users/vibelogic/dev/football-archive/` — frozen working copy of the Unity + C# state

See `REFERENCES.md` for what carries forward as design source and what was archived.

## License

Proprietary. All rights reserved.

# Final Whistle

[![CI](https://github.com/osagberg/FinalWhistle/actions/workflows/ci.yml/badge.svg)](https://github.com/osagberg/FinalWhistle/actions/workflows/ci.yml)
[![Determinism Gate](https://github.com/osagberg/FinalWhistle/actions/workflows/determinism-gate.yml/badge.svg)](https://github.com/osagberg/FinalWhistle/actions/workflows/determinism-gate.yml)

**A deterministic procedural-fantasy football management sim.**

Final Whistle is a solo-dev Rust + Tauri + SolidJS game about running a club in
a fictional football world that remembers what happened. No real clubs. No real
players. No runtime LLM calls. No 3D broadcast theatre. The bet is simple: deep
football simulation, strong text, a readable 2D tactical board, and a world that
becomes specific to your save.

The target is Football-Manager-class simulation depth in a world FM structurally
cannot make — procedurally generated, unlicensed, memory-ledger driven, and
built around players with readable on-pitch identities instead of just stat rows.

> Public development snapshot. Source-visible does not mean open-source
> licensed; see [License](#license).

---

## Current State

**Playable career-loop prototype, on a match engine that now reads as football.**

You can start a career into a freshly generated fantasy world, pick your club
from the league, and manage a season: advance week by week while the engine
simulates every other fixture, watch the table, fixtures, and press inbox move,
roll into new seasons, and save or load your career. It builds into a native
desktop app on macOS, Windows, and Linux.

Underneath the loop is the part most management games fake — the match itself —
simulated deterministically tick by tick:

- 22-player fixed-timestep match sim at 60 Hz, Q32.32 fixed-point canonical state.
- Held defensive shape: a per-team zonal block with compactness and a back line.
- Coordinated pressing (primary presser plus cover) and offside enforcement.
- Passes that genuinely contest and fail — completion is a quality × lane-openness
  × pressure roll, not a scripted success — producing a realistic pass mix.
- Goals that come from shots and legitimate deflections, not drift artefacts: the
  keeper comes off his line for loose balls and defenders clear the goal mouth.
- Behaviour-tree decisions, utility scoring, per-team tactic state, and signatures.
- A MatchEvent stream rendered into deterministic Tracery commentary, plus a
  PixiJS 2D tactical board for replay/inspection.

Determinism is enforced, not hoped for: pinned BLAKE3 canonical-state fixtures
are re-checked on every commit across macOS, Windows, and Linux. Same seed plus
same content equals the same world, byte for byte, on every platform.

This is an honest prototype, not a finished game. The match engine is deep but
still being made more believable season by season; the metagame loop is real but
thin (transfers, tactics editing, finance, and training are still ahead). What
works, works end to end and is covered by tests.

---

## Why This Is Interesting

Most football management games are built around licensed databases, statistical
event sampling, or opaque match-engine behaviour. Final Whistle takes a harder
route:

- **Every save is a fantasy football universe.** Names, cultures, clubs,
  archetypes, and commentary are authored as data, then baked. Runtime is
  deterministic; there are no real-world licensed names anywhere, ever.
- **The sim is the source of truth.** The board and commentary project from the
  match state. They do not invent the match after the scoreline is known.
- **Determinism is a product feature.** Same seed, same content, same canonical
  hash across three operating systems.
- **Careers are event-sourced.** An append-only memory ledger lets old decisions
  surface years later; a season-1 event can still be read in season 8.
- **Player growth is narrative, not XP spam.** Breakthroughs are rare,
  salience-gated events that can redraw a player's ceiling.
- **Scouting is uncertain by design.** Scouts are biased observers whose reports
  disagree; truth emerges over seasons.
- **Signatures make players readable.** The aim is to remember players by what
  they do on the pitch, not only by their numbers.

The project is intentionally text-first. The 2D board exists because you cannot
build a football sim blind, and it is the natural shipped match surface. There is
no 3D viewer planned — though the match-frame contract is kept renderer-neutral
so a richer 2.5D surface stays possible later.

---

## What Exists Today

| Area | Status |
|---|---|
| Rust workspace | Ten crates plus the Tauri shell |
| Match engine | Deterministic 22-player sim with team shape, pressing, offside, contested passing, shot-based goals |
| Career loop | Start a career, pick a club, manage a season (advance weeks, AI-sim fixtures, table, fixtures, press), save / load |
| Canonical replay | BLAKE3-pinned fixtures, re-checked on the OS matrix |
| Frontend | SolidJS app: main menu, club selection, squad, standings, fixtures, press, career overview, 2D board |
| Content | RON source schema, structural validators, seed fixtures |
| Commentary | Deterministic Tracery renderer over the MatchEvent stream |
| Saves | Versioned save format (SaveV4) with forward migration |
| CI | macOS / Windows / Linux matrix plus a determinism gate |
| Security gates | `cargo audit` + `cargo deny` wired |
| Tests | 1,400+ Rust unit/integration tests, ~310 frontend tests |

Still ahead: deeper believability (laws of the game, aerial physics, the living
match), the management metagame (transfers, tactics editing, finance, training),
world scale and the content-baker pipeline, richer UI, and Steam packaging.

---

## Tech Stack

**Core**

- Rust 1.95, edition 2024.
- Q32.32 fixed-point via `fixed`; `cordic` for sqrt/trig.
- BLAKE3 canonical hashing.
- `ChaCha8Rng` seeded per `(match_seed, tick, layer, site)` — explicit seed layers.
- `BTreeMap` / deterministic containers only in canonical paths; no floats, no
  clocks, no system RNG in the sim.
- Bincode-backed versioned saves (forward migration only).

**App**

- Tauri 2 shell; the UI never owns canonical state.
- SolidJS + TypeScript.
- Tailwind CSS.
- PixiJS 2D tactical board.
- TanStack Table and ECharts for dense management UI.

**Content**

- RON source files with content-pack-qualified, schema-versioned IDs.
- Bake-time content pipeline; zero runtime LLM calls.
- No real-world licensed player, club, league, or competition data.

---

## Run It Locally

Requirements:

- Rust toolchain from `rust-toolchain.toml` (`1.95`).
- Node 20+, pnpm 9+.
- `just`.
- Tauri system dependencies for your OS.

macOS quick path:

```sh
brew install rustup just node pnpm
rustup toolchain install 1.95
pnpm install --frozen-lockfile
```

Play the prototype (launches the desktop app with hot reload):

```sh
scripts/fw dev
```

Build a native desktop app for your platform:

```sh
scripts/fw bundle
```

Run the full local gate (fmt, clippy, Rust + frontend tests, lint/typecheck,
banned-term lint, determinism + content checks, `cargo audit`, `cargo deny`):

```sh
scripts/fw verify
```

Run the canonical hash regression only:

```sh
scripts/fw test-determinism
```

Generate deterministic board frames without launching Tauri:

```sh
cargo run -p fw-match-sim --bin dump_frames -- \
  --seed 0xfeedbeefcafefade \
  --ticks 600 \
  --content content \
  > /tmp/final-whistle-frames.json
```

---

## Project Map

```text
crates/
  fw-core/            fixed-point math, seeds, ticks, IDs, attributes
  fw-match-sim/       deterministic match engine
  fw-content/         content schema, RON loading, validation
  fw-content-baker/   bake-time content tooling
  fw-scouting/        scouting uncertainty model
  fw-memory/          event-sourced career memory ledger
  fw-replay/          canonical replay fixtures and hash tests
  fw-save/            save format and forward migrations
  fw-tauri/           IPC command layer (sim <-> UI contract)
  fw-dev-server/      dev-only HTTP bridge for browser-preview tooling

frontend/             SolidJS app
src-tauri/            Tauri shell binary
content/sources/      hand-authored source content
docs/                 design docs, ADRs, audits, specs, roadmap
scripts/fw            developer command front-door
```

Key docs:

- [Design contract](docs/DESIGN_DOC.md)
- [Master plan](docs/MASTER_PLAN.md)
- [Decision log](docs/DECISIONS.md)
- [Changelog](CHANGELOG.md)
- [Status pointer](STATUS.md)

---

## Development Model

The roadmap is dynamic and quality-driven rather than a fixed phase march: the
single question is what most improves the game, in dependency order. Believable
football is the foundation the design pillars sit on, and it is held by a
standing believability gate alongside the determinism gate.

Two rules are load-bearing:

- **The UI never owns canonical state.** The Rust sim owns truth; the frontend
  renders one-way projections of it.
- **No masking.** Behaviour changes are independently re-measured, not trusted
  from a self-report; tests are never loosened, deleted, or relabelled to fake a
  pass. Substantive changes get adversarial review before they land.

The main local gate is `scripts/fw verify` (above). Larger changes also go
through external review.

---

## Roadmap

Directional, not a contract — the order shifts to whatever most improves the
game next.

- **Believability** — the match keeps getting more recognisable: laws of the game
  (fouls, cards, set-piece restarts), aerial ball physics, and the living match
  (fatigue, score-state, half-time, substitutions).
- **The metagame loop** — deepen management around the playable spine: transfers,
  tactics and formation editing, finance, training, and the career narrative the
  memory ledger already records.
- **World scale + content bake** — grow from one league to a deeper pyramid and a
  baked content corpus.
- **Ship** — packaging, Steam Deck pass, and release gates.

The prototype is re-cut as these land, so the playable build tracks the latest
work.

---

## Provenance

Final Whistle started as a Unity + C# prototype and pivoted to a clean-slate Rust
rewrite on 2026-05-13.

The old prototype is preserved for reference only:

- Git tag: `v0-pre-pivot-2026-05-13`
- Local archive: `/Users/vibelogic/dev/football-archive/`

No Unity code was copied into the Rust rewrite. The carry-forward material is
design intent, tests, lessons, and prior sim behaviour.

---

## License

Proprietary. All rights reserved. See [`LICENSE`](./LICENSE) for the full notice
and [`NOTICE.md`](./NOTICE.md) for the third-party attribution catalogue.

This repository is public for development visibility, not because the game or
source code is released under an open-source license.

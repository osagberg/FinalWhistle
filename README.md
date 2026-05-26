# Final Whistle

[![CI](https://github.com/osagberg/FinalWhistle/actions/workflows/ci.yml/badge.svg)](https://github.com/osagberg/FinalWhistle/actions/workflows/ci.yml)
[![Determinism Gate](https://github.com/osagberg/FinalWhistle/actions/workflows/determinism-gate.yml/badge.svg)](https://github.com/osagberg/FinalWhistle/actions/workflows/determinism-gate.yml)

**A deterministic procedural-fantasy football management sim.**

Final Whistle is a solo-dev Rust + Tauri + SolidJS game about running a club in
a fictional football world that remembers what happened. No real clubs. No real
players. No runtime LLM calls. No 3D broadcast theatre. The bet is simple:
deep football simulation, strong text, a readable 2D tactical board, and a world
that becomes specific to your save.

The target is FM-class simulation depth in a world Football Manager structurally
cannot make: procedurally generated, unlicensed, memory-ledger driven, and built
around players with readable on-pitch identities instead of just stat rows.

> Public development snapshot. Source-visible does not mean open-source
> licensed; see [License](#license).

---

## Current State

**Vertical slice:** `v0.1.0-first-match`

Phase T1, **First Match**, is closed. The game can now run one deterministic
match end-to-end:

- 22-player fixed-timestep match sim at 60 Hz.
- Q32.32 fixed-point canonical state.
- Ball physics, possession, goal detection, kick-offs, and player movement.
- Behavior-tree driven decisions, utility scoring, tactic state, and signatures.
- MatchEvent stream into deterministic Tracery commentary.
- Tauri `play_match` IPC and frontend Match page.
- PixiJS tactical board for dev/replay inspection.
- Two pinned replay corpus hashes, checked across macOS, Windows, and Linux.

Latest pinned corpus:

```text
60 ticks:  blake3:fcccb840b5868a4ed55c019c353a1d5496259073e2d88bf7abd97d9bdca7a751
600 ticks: blake3:9353bd257d4da92092407355e3c2b32cc6e91abc81664d0015336ebe812947eb
```

The first-match smoke seed currently finishes **2-2** over 600 ticks. This is
not a finished game yet. It is the first working proof that the Rust rewrite can
move from canonical sim state to match events, commentary, and UI without
breaking determinism.

---

## Why This Is Interesting

Most football management games are built around licensed databases, statistical
event sampling, or opaque match-engine behavior. Final Whistle is taking a
harder route:

- **Every save is a fantasy football universe.** Names, cultures, clubs,
  archetypes, commentary, and future content packs are authored as data, then
  baked. Runtime is deterministic.
- **The sim is the source of truth.** The board and commentary project from the
  match state. They do not fake the match after the scoreline is known.
- **Determinism is a product feature.** Same seed, same content, same canonical
  hash across macOS, Windows, and Linux.
- **Careers are event-sourced.** The long-term design is an append-only memory
  ledger where old decisions can surface years later.
- **Player growth is narrative, not XP spam.** Breakthroughs are rare,
  salience-gated events that can redraw a player's ceiling.
- **Scouting is uncertain by design.** Scouts are biased observers. Reports
  disagree. The player triangulates truth.
- **Signatures make players readable.** The goal is to remember players by what
  they do on the pitch, not only by numbers.

The project is intentionally text-first. The 2D board exists because you cannot
build a football sim blind; it is also the natural shipped match surface. There
is no 3D viewer planned.

---

## What Exists Today

| Area | Status |
|---|---|
| Rust workspace | Live, 9 crates plus Tauri shell |
| Match sim | 22-player deterministic first-match vertical |
| Canonical replay | Two BLAKE3-pinned fixtures |
| Frontend | SolidJS Match page + dev tactical board |
| Content | RON source schema, validators, first fixtures |
| Commentary | Deterministic Tracery renderer |
| CI | macOS / Windows / Linux matrix plus determinism gate |
| Security gates | `cargo audit` + `cargo deny` wired |
| Tests | 360+ Rust tests, 50+ frontend tests at T1 close |

Still ahead: league season loop, save migration, full memory ledger, scouting
model, transfer market, richer UI, Steam packaging, and the content-baker
pipeline.

---

## Tech Stack

**Core**

- Rust 1.95, edition 2024.
- Q32.32 fixed-point via `fixed`.
- BLAKE3 canonical hashing.
- ChaCha8Rng with explicit seed layers.
- `BTreeMap` / deterministic containers in canonical paths.
- Bincode 2 save format planned for versioned saves.

**App**

- Tauri 2 shell.
- SolidJS + TypeScript.
- Tailwind CSS.
- PixiJS tactical board.
- TanStack Table and ECharts reserved for dense management UI.

**Content**

- RON source files.
- Bake-time AI content pipeline planned.
- Zero runtime LLM calls.
- No real-world licensed player, club, league, or competition data.

---

## Run It Locally

Requirements:

- Rust toolchain from `rust-toolchain.toml` (`1.95`).
- Node 20+.
- pnpm 9+.
- `just`.
- Tauri system dependencies for your OS.

macOS quick path:

```sh
brew install rustup just node pnpm
rustup toolchain install 1.95
pnpm install --frozen-lockfile
```

Run the full local gate:

```sh
scripts/fw verify
```

Start the app:

```sh
scripts/fw dev
```

Generate deterministic board frames without launching Tauri:

```sh
cargo run -p fw-match-sim --bin dump_frames -- \
  --seed 0xfeedbeefcafefade \
  --ticks 600 \
  --content content \
  > /tmp/final-whistle-frames.json
```

Run the canonical hash regression only:

```sh
scripts/fw test-determinism
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
  fw-save/            save format and migrations
  fw-tauri/           IPC command layer

frontend/             SolidJS app
src-tauri/            Tauri shell binary
content/sources/      hand-authored source content
docs/                 design docs, ADRs, audits, specs, phase plan
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

This repo is built in phase gates. Each phase has a concrete acceptance gate,
local verification, CI verification, and Codex review at the boundary.

The main local gate is:

```sh
scripts/fw verify
```

That currently runs formatting, clippy, Rust tests, frontend tests, frontend
lint/typecheck, banned-term lint, determinism audit, content validation,
canonical hash tests, `cargo audit`, and `cargo deny`.

The design rule is strict: the UI never owns canonical state. The Rust sim owns
truth; the frontend renders projections.

---

## Roadmap

- **T0 - Scaffold:** done. Rust + Tauri + SolidJS workspace, CI, determinism
  bedrock.
- **T1 - First Match:** done. One deterministic match from sim to UI.
- **T2 - League + Season:** next. Full season loop, league table, more
  manager archetypes, first save format.
- **T3 - Career Memory:** event ledger, breakthroughs, scouting disagreement.
- **T4 - Interface + Polish:** production-grade management UI, richer board,
  replay tooling.
- **T5 - Steam:** packaging, Steam Deck pass, store page, release gates.

---

## Provenance

Final Whistle started as a Unity + C# prototype and pivoted to a clean-slate
Rust rewrite on 2026-05-13.

The old prototype is preserved for reference only:

- Git tag: `v0-pre-pivot-2026-05-13`
- Local archive: `/Users/vibelogic/dev/football-archive/`

No Unity code was copied into the Rust rewrite. The carry-forward material is
design intent, tests, lessons, and prior sim behavior.

---

## License

Proprietary. All rights reserved. See [`LICENSE`](./LICENSE) for the full
notice and [`NOTICE.md`](./NOTICE.md) for the third-party attribution
catalogue.

This repository is public for development visibility, not because the game
or source code is released under an open-source license.

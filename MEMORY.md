# Final Whistle — Working Memory

> Updated: 2026-05-13 | Phase: T1 First Match (T1-1 closed; full-project-audit Tranches 1-7 closed; pre-T1-2b re-audit GREEN at 5bb0939d; T1-2a unblocked, awaiting final CI green on HEAD)

## Project

Procedural fantasy football management sim. Rust + Tauri 2 + SolidJS. Solo dev + Claude.
Pivoted from Unity + C# v1 (preserved at git tag `v0-pre-pivot-2026-05-13` and sibling `/Users/vibelogic/dev/football-archive/`).

## Module status (post-T1-1)

| Module | State | Key file | Notes |
|---|---|---|---|
| `fw-core` | T1-1 schema lock landed | `crates/fw-core/src/player_attributes.rs` | Q32 (panic-on-overflow, Codex Q1). Durable u32 IDs (Codex Q2). Seed + Tick + cordic sqrt. **NEW post-T1-1:** `PlayerAttributes` (55-field record), `AbilityCeiling` (encapsulated + breakthrough mutator), `PlayerCondition`, `KNOWN_ATTRIBUTE_NAMES` const. CI matrix green; deterministic macOS-14 + Win + Linux. **Codex audit followups queued:** Q32Inner re-export removal (Tranche 2); AbilityCeiling::try_new validation (Tranche 2); VISIBLE_ATTRIBUTE_NAMES split (Tranche 2). |
| `fw-match-sim` | Stub | `crates/fw-match-sim/src/lib.rs` | 22-player struct + no-op tick reducer. Hand-rolled little-endian canonical encoder (FWMS magic + version). `assert_eq!` slot-order invariant (Codex Imp #11). Float-deny clippy. T1-2b fills behavior (after Tranche 4 specs land). |
| `fw-content` | T1-1 schema lock landed | `crates/fw-content/src/player.rs` + `role_affinity.rs` | `PlayerTemplate` (wraps fw-core types + `schema_version: 1` + `RoleId`), `RoleAffinityTable` (sum-to-10_000 + collect-all `invalid_roles` + `unknown_attribute_keys`), `TacticalArchetype.buildup_speed_factor: u16 bps` (Codex Imp #3 from T0; `BUILDUP_SPEED_BASELINE_BPS = 10_000`). First RON fixtures live. **Codex audit gaps:** `ContentStore::load_baked` returns `Ok(Self::default())` (Tranche 6 — block runtime use until real); CA-weight validation accepts hidden/durability keys (Tranche 2). |
| `fw-content-baker` | CLI stub | `crates/fw-content-baker/src/main.rs` | clap CLI; prompt + schema + validator modules `#![allow(dead_code)]`-staged (T2-3+). Wires to Claude API at T2-3. |
| `fw-scouting` | Empty | `crates/fw-scouting/src/lib.rs` | Compiles, no types. T3-5 begins. |
| `fw-memory` | Empty | `crates/fw-memory/src/lib.rs` | Compiles, MemoryEvent enum stub. `stakes` + `salience` Q32 (Codex Crit #3 doc fix). T3-1 fills the ledger. |
| `fw-replay` | Acceptance test live | `crates/fw-replay/tests/canonical_hash.rs` | Phase-0 acceptance test ACTIVE. Pinned BLAKE3 `d6258107…` verified cross-OS. 4 tests run (1 still ignored — insta baseline pending T1). |
| `fw-save` | Empty | `crates/fw-save/src/lib.rs` | Compiles, SaveV1 enum stub. T2-9 begins. bincode 1 vs 2 alignment (Codex Imp #12) → T2-9. |
| `fw-tauri` | Stub | `crates/fw-tauri/src/lib.rs` + `commands.rs` | Two `#[tauri::command]` handlers in sibling module (Tauri 2 `pub` bug workaround). MatchStateDto projects Q32→f64 (read-only). T1-5 wires real surface. |
| `src-tauri` | Scaffolded | `src-tauri/src/main.rs` + `build.rs` | Tauri shell. build.rs stubs frontend/dist on clean clones. Tracked icon stubs. Local placeholder commands shadow fw-tauri (Codex Imp #10 → T1-5 consolidation). |
| `frontend` | Scaffolded + green | `frontend/src/main.tsx` | SolidJS + Tailwind v3 + 6 placeholder routes. typecheck + lint + build green across CI matrix. `<For />` over `.map()` (lint pass). |

## Recent work

- 2026-05-13: **Codex full-project audit landed + Tranche 1 remediation in progress.** Audit at `docs/audits/codex-full-audit-2026-05-13.md` — ~50 findings (1 P0 + 11 headline P1 + ~30 lower). P0 (bedrock-test `#[ignore]` hole) fixed at `eb0b952e` with 3-layer guard (meta-test + CI grep + hook). Doc-drift cleanup in this commit. T1-2a is BLOCKED on Tranches 2-4 (schema follow-ups + ADR work + companion specs).
- 2026-05-13: **T1-1 closed at `69f900b9`.** ADR-0002 55-field player model in `fw-core`; `AbilityCeiling` encapsulated; `RoleId` newtype; `TacticalArchetype` f32→u16 bps. 65 tests; canonical hash UNCHANGED. Self-review triple twice → Accept. Codex audit subsequently caught additional P0/P1 issues — see audit doc.
- 2026-05-13: **Phase T0 closed.** Pivot (109 files) + blueprint reconciliation (51 files) + Codex pre-T0 audit (14 of 16 findings landed) + canonical hash pinned + CI matrix green + Codex APPROVE. See `docs/postmortems/phase-T0.md`.

## Current task

None active. Re-audit GREEN at `5bb0939d`.

Pre-T1-2b re-audit is closed. Pass #1 (`80c53f76`) closed 6 of 7 prior P1s; pass #2 (`5bb0939d`) closed the remaining 3 P1s + the xG P2 + state-pointer drift + ADR-0012 wording; pass #3 returned **GREEN — no new P0/P1**.

After CI on HEAD goes green (current commit re-triggers CI with the `--release` flag dropped from the content-pack-validation step — Ubuntu was hitting the 20m timeout building fw-content-baker in release mode), `/next` picks **T1-2a**.

### Deferred follow-ups (NOT blockers)

- **T2-1 — xG penalty re-fit.** Phase-1 logistic hits 0.65 for penalty vs. 0.76 target. The miss is structural (single-logistic can't split penalty from 12-yard chance without per-zone intercept). T2-1's calibration loop introduces a penalty-specific β₀ split. Acceptable for T1 playability.
- **P3 — research-doc LoC-budget remnants.** Caveats are present + visible; cleanup is a one-pass `grep -r` editorial when convenient.

<!-- Historical scope-spec for the just-shipped T1-1 retained below for grep-back reference -->

<details>
<summary>T1-1 task spec (closed 2026-05-13)</summary>

- **id:** T1-1
- **title:** `fw-content` schema — `PlayerTemplate` (ADR-0002 55-field model) + `TacticalArchetype` Codex Imp #3 conversion + first RON fixtures
- **started:** 2026-05-13
- **completed:** 2026-05-13
- **task class:** architecture-cross-crate (schema lock across fw-core + fw-content)
- **TDD exemption:** YES (data-only)

### Files out of scope (do NOT touch — escalate if needed)
- `docs/DESIGN_DOC.md`
- `docs/DECISIONS.md` (no new architectural decision logged here — T1-1 IMPLEMENTS ADR-0002, doesn't change it)
- `docs/adr/0002-player-attribute-model.md` (the source of truth — don't drift)
- `CLAUDE.md`
- `docs/MASTER_PLAN.md` (status flip is the only allowed mutation)
- `crates/fw-match-sim/**` (T1-2b's job)
- `crates/fw-memory/**`
- `crates/fw-replay/**`
- `crates/fw-save/**`
- `crates/fw-tauri/**`
- `frontend/**`
- `crates/fw-core/src/q32.rs` (locked at T0)

</details>

## Recently completed

- 2026-05-13 — T1-1 `fw-content` schema lock (ADR-0002 55-field player model + Codex Imp #3 conversion + first RON fixtures). `PlayerAttributes` in `fw-core` (14/10/8/6 visible + 14/3 hidden = 55 Q32 fields); `KNOWN_ATTRIBUTE_NAMES` const + size-of static asserts pin schema shape. `AbilityCeiling` encapsulated with `redraw_ceiling` breakthrough mutator (Pillar 3 contract). `RoleId` newtype + `RoleAffinityTable` with collect-all `invalid_roles` + `unknown_attribute_keys` validators. `TacticalArchetype.buildup_speed_factor` → `u16 bps` with `BUILDUP_SPEED_BASELINE_BPS = 10_000`. `PlayerCondition` deliberately NOT on `PlayerTemplate` (save-migration hygiene). `schema_version: 1` on both new content types + fixtures. 65 tests new in fw-core + fw-content; canonical hash UNCHANGED. Self-review triple ran twice — Accept all three. commit `pending`.
- 2026-05-13 — T0-12 Fix pre-existing scaffold build failures — fw-tauri commands moved to sibling module (known Tauri 2 `pub` + `#[tauri::command]` bug); fw-content-baker `#![allow(dead_code)]` on staging modules; src-tauri build.rs stubs frontend/dist for clean-clone `cargo build`; tauri icons generated (gitignored); ui-vocabulary.md meta-references wrapped in sentinels. `cargo test --workspace --release` 19 test-runs all green.
- 2026-05-13 — T0-7 Pin BLAKE3 canonical hash on dev box — `d6258107…` pinned; `cargo test -p fw-replay` 4/4 green; cross-OS matrix → T0-7b.

## Active decisions

- Q32.32 (`fixed::FixedI64<U32>`) for all canonical-state quantities. `#[deny(clippy::float_arithmetic)]` on sim crates.
- BLAKE3 (not SHA-256) for canonical-state hashing.
- RON files for content sources + replay fixtures (human-diffable).
- Bincode 2 for save format.
- `ChaCha8Rng` for all sim randomness. `thread_rng` / `StdRng` banned in sim code.
- `BTreeMap` / `IndexMap` only in canonical-state-emitting code. `HashMap` banned in sim crates.
- Tauri 2 + SolidJS + Tailwind v3 + TanStack Table v8 + PixiJS v8 + ECharts.
- Single primary workflow command: `/next` (see `.claude/skills/next/SKILL.md`).
- Codex review at phase-gates via PR (not per-task).
- Per-task self-review via `pr-review-toolkit` subagents on ≥100 LoC code changes.

## Carry-forward debts (FW v1 → v2)

Logged 2026-05-13 from the T1-1-vs-IdentityPacket comparison. Full detail + pinning rows in `REFERENCES.md` "Carry-forward debts" table. Headline items:

- **T1-3** owes `PlayerTemplate.signature_candidates: Vec<SignatureCandidate>` — v1 had this in Phase 3, v2 T1-1 deferred. Pillar 5 has no per-player linkage until this lands.
- **T2-3** owes the dedicated `<Kind>Validator` pattern (one class per content kind) instead of v2's current spread-across-methods validation. Easier to audit.
- **T2-4** owes the 46-label phenotype catalog from FW v1's `design/player-generation.md`. The 55-field ADR-0002 model supersedes the 22-field gene model, but the phenotype labels haven't been ported.

These are encoded in MASTER_PLAN T1-3 + T2-3 done-criteria so they fire when the rows do.

## Queued user actions (Codex audit Tranche 7 cleanup)

Cleanup the user runs at their convenience (NOT auto-applied by Claude — these touch user-global state):

- **Remove Unity MCPs from global `claude mcp list`.** Currently `unity-mcp` + `UnityMCP` are still registered. They were used by FW v1; v2 is Rust-only. Run `claude mcp remove unity-mcp` + `claude mcp remove UnityMCP`. Confirm via `claude mcp list`. Codex audit Lane F P2.
- **Install Claude Preview MCP** if not yet done. When the dev-server prompt fires in Claude Code, click through. Until installed, the `mcp__Claude_Preview__preview_*` tools are loaded as deferred but not exercisable end-to-end. Codex audit Lane F P2. ADR-0008's workflow becomes runnable post-install.
- **(Optional) Install repo-local git pre-commit hook for canonical-hash-guard.** Per ADR-0012 §"Three-layer guard" reconciliation, the `.claude/hooks/canonical-hash-guard.sh` is convenience-only — it only fires for commits made via Claude Code. Adding a repo-local `.githooks/pre-commit` that calls the same script would make layer 3 durable. Not required (layers 1 + 2 are already durable + CI-enforced), but useful if you frequently commit via the terminal. To install: `mkdir -p .githooks && cp .claude/hooks/canonical-hash-guard.sh .githooks/pre-commit && chmod +x .githooks/pre-commit && git config core.hooksPath .githooks`.

## Queued research

### Frontend research wave (run before T1-6, NOT now)

**Trigger:** when T1-2a's PR is in flight (background work while waiting for review). Synthesis must land before `/next` picks up T1-6 (frontend Match route + text recap).

**Headline lens — why FM26 sucked:** FM26 (Football Manager 2026 → "FM26" in player parlance) shipped to widely negative reception on UI/UX specifically. The wave must understand WHY before it can recommend what we do instead. Hypotheses to test: (a) menu depth + nesting; (b) lost information density vs prior FMs; (c) inconsistent navigation grammar; (d) Unity-port artefacts (FM moved engines); (e) "designed for a different player" — gamification at the cost of the management-sim core. The wave must read the actual reviews/forums, not assume.

**Other targets:**
- FM 24 (the last well-received FM): match-day page hierarchy, sidebar density, scrubber affordances, post-match recap layout. The reference for "what was lost in FM26."
- OOTP 25: how dense tabular surfaces stay readable — directly relevant to our TanStack Table v8 management screens.
- FOF (Front Office Football): the "ugly but information-perfect" school — what makes its UI work despite the visuals. Inform our text-first stance.
- EHM (Eastside Hockey Manager): text-first match presentation. Closest sibling to our presentation model.
- Tennis Elbow + PCM (Pro Cycling Manager): niche-sport UI patterns we can crib without inheriting their problems.
- Visual style references: color systems, mono vs sans for stats, table zebra-striping, hover affordances, keyboard nav, player-profile page layouts.

**Deliverable:** `docs/research/frontend/00-synthesis.md` with concrete recommendations for T1-6 + the eventual T4 polish pass. Must explicitly answer "what FM26 got wrong, and what we do instead."

## Open questions

(See `docs/DESIGN_DOC.md` §12 for the gameplay design open questions.)

Technical open questions for T0 follow-up:

1. Pinned hash placeholder — fill on first CI green pass (T0-7).
2. Apple-codesign secrets — defer until T5-1 release pipeline.
3. Tailwind v3 vs v4 — picked v3 for May 2026 stability; revisit at T4.

## Next up — Phase T1 — First Match

Per `docs/MASTER_PLAN.md`:
- T1-1 `fw-content` schema + first RON fixtures (also: Codex Imp #3 buildup_speed_factor f32 → u16 bps conversion)
- T1-2 ball physics + 22-player BT runner (XL — biggest row; consider splitting at task-spec time)
- T1-3 signatures stub (types only)
- T1-4 MatchEvent enum + ledger output
- T1-5 fw-tauri play_match command (also: Codex Imp #10 — src-tauri command consolidation)
- T1-6 frontend Match route + text recap
- T1-7 procedural content stub (Markov names + 2 teams)
- T1-8 replay corpus fixture #1 (smoke seed, 600 ticks, two-archetype matchup)

Acceptance gate for T1: two procedural teams play one match end-to-end; text recap surfaces with goals + score + key events; ≥2 replay-corpus fixtures pin across CI matrix.

Critical path: T1-1 → T1-2 → T1-4 → T1-5 → T1-6.

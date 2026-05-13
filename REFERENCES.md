# References

The Rust+Tauri pivot (2026-05-13) carries design forward from the prior Unity + C# version of Final Whistle.

## Archives

| Source | Path | What it contains |
|---|---|---|
| **Pre-pivot git tag** | `git checkout v0-pre-pivot-2026-05-13` | Last C# commit (1d3a58b, Round-3 #4 defensive line height). Reachable in this repo's history. |
| **Sibling working copy** | `/Users/vibelogic/dev/football-archive/` | Frozen on-disk snapshot. `cd` into it to read any FW v1 file directly. 59MB. Read-only by convention. |

## What carried forward as design source

The pivot is a CLEAN-SLATE rewrite. NO code copied. Patterns and design intent carried forward via `docs/DESIGN_DOC.md`. The migration audit at `docs/archive/MIGRATION_AUDIT.md` is the authoritative classification of every notable FW v1 file: REFERENCE / DESIGN-SOURCE / DROP / WORKFLOW-PORT.

### Key carryforward sources (consult when implementing related systems)

| New crate / system | FW v1 source files (for design intent only — DO NOT copy code) |
|---|---|
| `fw-core/q32.rs` | `MatchSim/Sim/Fixed.cs` (Q32.32 reference impl) |
| `fw-core` ID types | `MatchSim/Sim/PlayerId.cs`, `Tick.cs`, `Seed.cs` |
| `fw-match-sim` BT runner | `MatchSim/Sim/BehaviorTreeRunner.cs` (1000 LoC of distilled football intelligence — read for *intent*, port to Rust idioms) |
| `fw-match-sim` ball physics | `MatchSim/Sim/BallPhysics.cs`, `BallPhysicsCoefficients.cs` |
| `fw-match-sim` actuator | `MatchSim/Sim/PlayerActuator.cs` (steering kinematics) |
| `fw-match-sim` separation | `MatchSim/Sim/PlayerSeparation.cs` (Round-3 #1 collision avoidance) |
| `fw-match-sim` signatures | `MatchSim/Sim/SignatureRules.cs`, `design/signatures.md` |
| `fw-match-sim` canonical encoder | `MatchSim/Sim/CanonicalEncoder.cs`, `MatchCanonicalState.cs` |
| `fw-content` schema | `MatchSim/Content/IdentityPacket.cs`, `archetypes/*.yaml` |
| `fw-content/runtime.rs` sampling | `MatchSim/Content/IdentityPacketValidator.cs` |
| `fw-memory` ledger | `MatchSim/Memory/*` (5-reader pattern) |
| `fw-scouting` | `design/scouting.md`, `design/scout-disagreement.md` |
| `fw-replay/canonical_hash.rs` | `MatchSim.Tests/Sim/MatchDeterminismTests.cs` (pinned-hash pattern) |
| Replay corpus format | `MatchSim.Tests/fixtures/replay-corpus/*.json` (FW v1 format) |
| `fw-save` migrations | (no FW v1 source; greenfield) |
| Banned-terms lint | `scripts/lint-banned-terms.py` (ported verbatim with path scope updates) |
| Decisions log + protect hook | `.claude/hooks/protect-decisions-log.sh` (ported as `protect-decisions.sh`) |
| Canonical-hash-guard hook | `.claude/hooks/canonical-hash-guard.sh` (ported with Rust-flavored path checks) |

### Key design docs to consult

In the sibling archive `/Users/vibelogic/dev/football-archive/design/`:

- `breakthrough-moments.md` — the rare-narrative-growth model for player progression
- `scouting.md` + `scout-disagreement.md` — uncertainty-driven scouting pillar
- `memory.md` + related — event-sourced ledger semantics + 5 readers
- `signatures.md` — the 24-signature catalog + 8 role-family taxonomy
- `match-engine.md` — determinism doctrine + sim architecture
- `player-generation.md` — 22-field gene model + 46-label phenotype catalog
- `progression.md` — bounded-gene + breakthrough-redraw model
- `ui-vocabulary.md` — football-native vocabulary rules (banned terms catalog)
- `modding.md` — 12-constraint mod-readiness contract

## Carry-forward debts (open against future MASTER_PLAN rows)

Items FW v1 shipped that the equivalent FW v2 work has NOT yet ported. Pin to the row that should pick them up.

| Owed at row | FW v1 source | What v2 needs to add | Notes |
|---|---|---|---|
| **T1-3** (signatures stub) | `IdentityPacket.SignatureCandidates` + `SignatureCandidate { SignatureId, AffinityWeightRaw }` | Add per-player signature affinity to `PlayerTemplate`: `signature_candidates: Vec<SignatureCandidate>` where each entry pairs `SignatureId` (content-pack-qualified) with a Q32 affinity weight. v1 had this even in Phase 3 — v2 T1-1 deliberately deferred it. Without it, Pillar 5 has no per-player linkage. | `qualified_id` format: `^fwh\.core(?:\.v[0-9]+)?:signature\.[a-z0-9-]+$` |
| **T2-3** (content baker + FW-VAL) | `IdentityPacketValidator.cs` (204 LoC, single dedicated class) | Port the validator-as-one-class pattern. v2 currently spreads checks across method-on-type (`RoleWeights::unknown_attribute_keys`, `RoleAffinityTable::invalid_roles`) + load-tests. Single-class form is easier to audit ("what does well-formed content mean?" → one file). | Don't copy the C# — port the *shape* (one validator type per content kind, chained checks, structured error type) |
| **T2-4** (player generation) | `design/player-generation.md` (22-field gene model + 46-label phenotype catalog) + `IdentityPacketGenes` (the 6 active genes shipped) | Already known; the 55-field model from ADR-0002 supersedes the 22-field schema. Phenotype-label catalog (46 labels) still owed — was Phase-4 in v1, T2-4 in v2. | Cite `docs/research/sports-sims/07-player-attributes-progression.md` for the FOF range-projection model that replaced v1's "single canonical truth + per-scout bias" framing |
| **T2-3 / future** | Hand-rolled `IdentityPacketParser` (Codex round-7 lesson: Unity Mono didn't ship `System.Text.Json` + transitive deps) | NOT directly applicable — Rust's `serde + ron` doesn't have the runtime-dep problem. BUT the meta-lesson stands: **content-format choice has long-tail consequences**. Document somewhere that RON was deliberate and stable. | Already implicit in CLAUDE.md §3 — could be more explicit |

## Comparison snapshots

Snapshot reads of "how does the v2 work compare to v1?" Pinned for the lessons, not for the data.

- **2026-05-13 — T1-1 schema lock vs. FW v1 `IdentityPacket` (Phase-3 minimum subset).** v2 ships FM-class breadth (55 fields vs v1's 6 active / 22 planned) on day one, open `RoleId` newtype (vs v1's closed 8-entry `RoleFamily` enum), and `AbilityCeiling` encapsulation with the breakthrough-only `redraw_ceiling` mutator (vs v1's `init`-only properties with no Pillar-3-equivalent surface — v1 hadn't shipped breakthroughs yet). v1 had per-player signature affinity (`SignatureCandidates`); v2 deferred it to T1-3. v1 had a dedicated 204-line validator class; v2 spreads validation across methods. 3× LoC growth (381 → 1,136) tracks 9× attribute growth honestly — most of v2's lines are field enumeration + doc comments + KNOWN_ATTRIBUTE_NAMES + tests, not bloat. Carry-forward debts above.

## What was dropped permanently

- All `unity-project/**` — Unity 6 project, dots viewer, scenes, shaders
- `design/3d-pipeline.md`, `design/anime-presentation-budget.md`, `design/semantic-cinema.md` — 3D/cinematography ambitions cut
- ADRs 0001 / 0002 / 0008 / 0009 / 0011 — Unity-specific architecture decisions
- The 7-shot semantic-cinema grammar — replaced by 2D PixiJS tactical board + text recap
- All `.claude/agents/` for Unity specialty (art-director, unity-specialist, etc) — 15 agents → 7 agents (after blueprint reconciliation at commit 26f1ba0; see `docs/BLUEPRINT_RECONCILE.md`)
- The `scripts/agent-bus` per-slice Codex review protocol — replaced by phase-gate Codex PR review
- The 4 duo-* skills (`/duo-implement`, `/codex-review-loop`, `/check-reviews`, `/duo-debate`) — replaced by single `/next`

## How to use the archive

```sh
# Look up a specific FW v1 file
cd /Users/vibelogic/dev/football-archive
cat design/signatures.md

# Or via git tag in this repo
git show v0-pre-pivot-2026-05-13:design/signatures.md
git log v0-pre-pivot-2026-05-13 -- design/  # all commits affecting the design folder

# Run the old C# tests (for verification when porting a system)
cd /Users/vibelogic/dev/football-archive
dotnet test MatchSim.Tests/MatchSim.Tests.csproj
```

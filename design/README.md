---
description: Index of authoritative design docs for Final Whistle. Each doc follows the purpose / locked-decisions / MVP-boundary / deferred / open-questions / prototype-gate structure.
last_verified: 2026-04-24
---

# design/ — authoritative for intent

Per-system design docs. The rule is `design > content > code` — if code disagrees with a design doc, fix the code. If a design doc disagrees with another, raise it in Phase 0 open-questions resolution.

Every doc follows this structure:

1. **Purpose** — what question does this doc answer
2. **Locked decisions** — what we committed to (reference the SPEC.md decision entry)
3. **MVP boundary** — what's in vs out for Month-12 EA
4. **Deferred** — what's seeded but surfaces post-MVP, or what's explicitly post-EA
5. **Open questions** — what still needs user resolution before Phase 1/2 gate
6. **Prototype gate** — how we verify the system's feel or architecture works

## Index (this project)

All 12 docs had their open-questions resolved during the Phase-0 consolidated 2026-04-24 resolution pass. "Open questions resolved" below tracks that milestone; Phase-2 ADR authoring lives in `SPEC.md` as separate tasks.

| File | System | Open questions resolved |
|---|---|---|
| [`overview.md`](overview.md) | Game pillars, top-level experience, 4-bucket scope split | Phase 0 / 2026-04-24 |
| [`month-3-vertical-slice.md`](month-3-vertical-slice.md) | Brutal-minimum first-proof spec | Phase 0 / 2026-04-24 |
| [`match-engine.md`](match-engine.md) | MatchSim architecture, determinism, ball physics | Phase 0 / 2026-04-24 |
| [`semantic-cinema.md`](semantic-cinema.md) | Renderer-agnostic 7-shot-type grammar | Phase 0 / 2026-04-24 |
| [`event-sourced-memory.md`](event-sourced-memory.md) | Career memory ledger, readers, compaction | Phase 0 / 2026-04-24 |
| [`signatures.md`](signatures.md) | 24-signature catalog, 3 per role family | Phase 0 / 2026-04-24 |
| [`scout-disagreement.md`](scout-disagreement.md) | Scout-bias system spec; Month-4 feel-gate spec | Phase 0 / 2026-04-24 |
| [`breakthrough-moments.md`](breakthrough-moments.md) | Match-flow cinematic development-change triggers | Phase 0 / 2026-04-24 |
| [`player-generation.md`](player-generation.md) | Internal gene model + Identity Packet compiler | Phase 0 / 2026-04-24 |
| [`worldbuilding.md`](worldbuilding.md) | Fictional nation (Caldren), pyramid structure, cultural priors | Phase 0 / 2026-04-24 |
| [`ui-vocabulary.md`](ui-vocabulary.md) | Banned-terms lint + approved football-native phrasing | Phase 0 / 2026-04-24 |
| [`production-pipeline.md`](production-pipeline.md) | CI/CD tiers, runner policy, build channels, release gates | Phase 0 planning pass / 2026-04-24 |
| [`modding.md`](modding.md) | Cross-ADR data-architecture contract (12 mod-loadability constraints) | Phase 2 synthesis pass / 2026-04-24 |
| [`accessibility.md`](accessibility.md) | EA accessibility feature set (reduce-motion / colorblind / remap / large-text / subtitles) | Phase 2 authoring pass / 2026-04-24 |
| [`content_policy.md`](content_policy.md) | PEGI 12 / ESRB T content boundaries + AI-content disclosure + mod-pack content-safety surface | Phase 2 authoring pass / 2026-04-24 |
| [`3d-pipeline.md`](3d-pipeline.md) | 3D cel-shaded shipping-visual pipeline placeholder + Phase-5/6 production-feasibility spike-gate criteria + animation contract surface + licensing requirements + alternatives | Phase 2 placeholder authored 2026-04-26 per visual-target supersession; full spec lands at Phase-5 spike kickoff |

## Specs (sub-contracts under `design/specs/`)

Implementation contracts that bind one or more system docs above. Each is its own append-only-ish authority for the surface it covers.

- [`specs/golden-replay-corpus.md`](specs/golden-replay-corpus.md) — canonical-seed match fixtures + cross-platform deterministic-replay hash protocol.
- [`specs/save-migration-fixtures.md`](specs/save-migration-fixtures.md) — save-format migration regression fixtures.
- [`specs/content-pack-validation-contract.md`](specs/content-pack-validation-contract.md) — content-pack validator gate (Phase 6 implementation).
- [`specs/artifact-retention-policy.md`](specs/artifact-retention-policy.md) — CI artifact retention windows.
- [`specs/football-rules-matrix.md`](specs/football-rules-matrix.md) — MatchSim football-law simplification matrix + canonical-impact tracking + promotion triggers.

## Future docs (added when trigger hits)
- `balance-harness.md` — Phase 6 tuning methodology
- Per-signature specs under `signatures/*.md` — Phase 3+
- ADRs under `design/adr/NNN-title.md` — one per load-bearing system decision

## Relationship to code

1. `design/` → intent (what we want)
2. content packs + ScriptableObjects → authoritative runtime data (what the game uses)
3. C# → behavior only
4. MatchSim → canonical simulation state

Conflicts resolve: **design > content > code**. If code hardcodes a number, it was SO data that got inlined — fix by moving back.

## Authoring discipline

See [`.claude/rules/design-docs/RULES.md`](../.claude/rules/design-docs/RULES.md) for the full author-contract. Summary:

- YAML frontmatter with at least `description` required
- Cross-references MUST resolve
- Formulas stay formulas
- Template-derived skeleton, never empty
- Single source of truth per fact
- `last_verified` date updated when re-confirmed

## Archive

When iterations get retired, move to `design/archive/<iteration-name>/`. SPEC.md decisions log references retirement. Never delete — retired design docs are historical record.

## Brainstorm history

`design/brainstorm/` contains the 5 research docs produced during Phase 0 kickoff (FM26 gap analysis, anime-sports conventions, genetics system exploration, cutting-edge systems, IP pivot). These are HISTORICAL records of design thinking, not binding specs. Where brainstorm docs disagree with the locked design docs above, the locked docs win.

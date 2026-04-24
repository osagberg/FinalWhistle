---
description: ADR-0007 — Scout archetype + ScoutReport schema + callback/event integration + conditional-MVP gate-fallback behavior. Architecture slot reserved regardless of Month-4 feel-test outcome.
---

# ADR-0007: Scout archetype + ScoutReport schema + gate-fallback

## Status

**Proposed** — pending user review before Accepted.

## Date

2026-04-24

## Last Verified

2026-04-24

## Decision Makers

osagberg (project owner), GPT-5.5 (design partner on 2026-04-24 scout-disagreement resolution), Claude (workhorse).

---

## Summary

Lock the conditional-MVP scout architecture BOTH paths: the **Scout Disagreement** full system (3 archetypes biased-reading `IdentityPacket.InternalGeneSnapshot` at category level) AND the **Scout Uncertainty** fallback if Month-4 feel-test gate fails. `ScoutReport` schema is identical in both paths — only the certainty-level population differs. `ScoutReportConfirmed` + `ScoutReportDisagreement` events reference `FinalWhistle.Memory.Contracts` constants (not duplicate strings); on gate-fail, `ScoutReportDisagreement` is dropped at a schema-version bump per ADR-0004 cross-doc exact-match discipline.

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Engine-agnostic — pure C# schemas + JSON reports; Unity consumes reports via existing Addressables pattern |
| Domain | Gameplay / Content / Narrative Systems | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | LOW — enum-backed archetypes, structured ScoutReport records, already-exercised Contracts/impl split pattern |
| References Consulted | `design/scout-disagreement.md` 2026-04-24 resolution (3 archetypes + Month-4 feel-test + staged-time ledger feedback), ADR-0004 (event-class constants), ADR-0006 (InternalGeneSnapshot + category-level scout bias), `design/signatures.md` §Q5 counterplay-surfaces-via-scout-reports |
| Post-Cutoff APIs Used | None |
| Verification Required | Month-4 feel-test gate (per `design/scout-disagreement.md`): 3 external management-game-literate testers satisfy all three pass criteria on 10 hand-authored Identity Packet stubs with staged-time ledger feedback |

## Dependencies

| Field | Value |
|---|---|
| Depends On | ADR-0004 (MemoryEvent — `ScoutReport*` event-class constants), ADR-0006 (IdentityPacket — category-level `InternalGeneSnapshot` bias filter) |
| Enables | Phase-4 Month-4 feel-test prototype (3 archetypes, 10 hand-authored packets, staged-time feedback); Phase-5 full-system expansion if gate passes; Signature counterplay surfacing per `design/signatures.md` §Q5 (scout-report UI is the constant regardless of gate outcome) |
| Blocks | Phase-4 Scout Disagreement prototype build; counterplay reveals in scout reports (both paths) |

---

## Context

### Problem Statement

`design/scout-disagreement.md` 2026-04-24 resolved the Month-4 feel-test spec but explicitly left the system conditional — it ships in MVP only if the gate passes. This ADR has to commit to **both paths architecturally** so: (a) the full system has a build-ready shape at Phase-4 kickoff, AND (b) if the gate fails, the fallback is not "we have no scouts" but "we have Scout Uncertainty with the same `ScoutReport` surface." Either way, `design/signatures.md` §Q5 counterplay surfacing needs a stable scout-report shape to render against.

### Current State

No runtime implementation. `design/scout-disagreement.md` 2026-04-24 resolution locked:
- 3 prototype archetypes (`physical_profiler`, `technical_purist`, `regional_expert`)
- Structured `ScoutReport` (labels canonical; prose rendered deterministically)
- 3 external testers (user excluded; pass = ≥2 of 3 satisfy all three criteria)
- One-remediation-pass ceiling
- 10 hand-authored Identity Packet stubs
- Staged-time feedback loop for scout track-record
- Event-class contingency: `ScoutReportDisagreement` drops at schema-bump if gate fails; `ScoutReportConfirmed` stays

ADR-0006 locked `InternalGeneSnapshot` category-level bias (physical / mental / technical / narrative_flag) with narrative-flag zero-visibility.

### Constraints

- **Conditional-MVP:** architecture must work for both "ships" and "falls back to Scout Uncertainty" outcomes. No dead code if cut; no unknown shape if kept.
- **Deterministic scout reports** — identical `IdentityPacket` + identical scout archetype = identical `ScoutReport.Labels`. Prose is template-rendered at generation time and stored (matches ADR-0004 reader discipline for press/fan templates).
- **Event emission via `Memory.Contracts` constants** — no inline string literals for `ScoutReportConfirmed` / `ScoutReportDisagreement` per ADR-0005's pattern.
- **Narrative-flag category never directly observable** (ADR-0006 constraint) — flags surface retroactively through trigger events only.
- **Q32.32 for confidence + uncertainty-range bounds** — scout reports feed `key_event_hashes` via emitted `MemoryEvent`s; fixed-point is the determinism posture per ADR-0001/0004/0005/0006.

### Requirements

- **Functional:** `Scout` archetype record with category-level biases; `ScoutReport` record with structured labels + confidence + uncertainty ranges + rendered prose; event-emission integration with ADR-0004; staged-time feedback loop for Phase-4 prototype; fallback path to Scout Uncertainty with identical report surface.
- **Performance:** report generation per (scout × player) = O(1) bias-filter pass; content-pack validator catches unknown archetype IDs; Phase-6 balance harness tests for silent-failure-fraud (e.g., "all scouts agree on every player" = gate-fail signal).
- **Mod-pack-loadability:** content packs can ship additional `Scout` archetypes post-EA Workshop; existing archetypes' IDs never mutate.

---

## Decision

### `Scout` archetype (Accepted)

```csharp
using FinalWhistle.Memory.Contracts;     // EventClass.ScoutReportConfirmed / .ScoutReportDisagreement
using FinalWhistle.Content.Contracts;    // InternalGeneSnapshot (read-only; category-level access via filter)

public sealed record Scout
{
    // --- Stable identity ---
    public ContentPackQualifiedId ArchetypeId;         // e.g. "fwh.core:scout.physical-profiler" — never mutates
    public ScoutArchetypeKind Kind;                    // enum; canonical 3-entry at MVP + conditional-expansion slot

    // --- Player-facing copy — lint TARGET ---
    public string DisplayName;                         // banned-term lint scans this
    public string UiDescription;

    // --- Bias model (category-level per ADR-0006) ---
    public CategoryBiases Biases;                      // { physical, mental, technical, narrative_flag = 0 }
    public ImmutableArray<RegionId> FamiliarRegions;   // regional_expert uses this; others ignore

    // --- Observation noise + confidence ---
    public Fixed BaseObservationNoise;                 // Q32.32 [0,1]
    public Fixed RegionalNoisePenalty;                 // Q32.32 [0,1] — out-of-region observation noise added

    // --- Taste markers for report prose ---
    public ImmutableArray<TasteMarkerId> TasteMarkers;

    // --- Track record (recomputed from ledger; not persisted in archetype) ---
    // Track record lives in save-state, not here. Archetype is the immutable definition.
}

public enum ScoutArchetypeKind
{
    // Month-4 prototype archetypes — locked per design/scout-disagreement.md 2026-04-24 §Q1
    PhysicalProfiler,        // high physical weight, low mental
    TechnicalPurist,         // high technical + mental, low physical
    RegionalExpert,          // cohort-dependent — neutral in-region, noisy out-of-region

    // Conditional-expansion slots — land Phase 5+ ONLY if Month-4 gate passes
    TempoReader,             // Phase 5+: reads midfielders' rhythm
    AcademySpotter,          // Phase 5+: bias toward teenagers
    SetPieceSpecialist,      // Phase 5+: dead-ball depth (requires Phase-4 set-piece system)
    BasicScoutUncertainty,   // Fallback: if Month-4 gate FAILS, this archetype replaces all 3 prototypes
                             //           with fog-over-numbers FM-style scouting; single-report-per-player
}

public readonly struct CategoryBiases
{
    public Fixed Physical;
    public Fixed Mental;
    public Fixed Technical;
    public Fixed NarrativeFlag;   // MUST be 0 at MVP per ADR-0006 — validator enforces
}
```

### `ScoutReport` schema (Accepted — IDENTICAL across both gate outcomes)

```csharp
public sealed record ScoutReport
{
    // --- Canonical structured data ---
    public ScoutId ScoutId;                                   // which scout produced this
    public ContentPackQualifiedId PlayerId;                   // target of the report
    public CareerDate ObservedOn;                             // save-world date
    public Fixed Confidence;                                  // Q32.32 [0,1] — overall scout confidence

    // Per-label structured data — the UI + tests read this, NOT the prose.
    public ImmutableArray<LabelEstimate> Labels;

    // --- Rendered prose — deterministic; stored for replay, never runtime LLM ---
    public string Prose;                                      // rendered at emit time from a template
    public ContentPackQualifiedId SourceTemplateId;           // audit / regeneration

    // --- Event-class emission references (no inline strings per ADR-0005 pattern) ---
    public const EventClass EmitsOnConfirm = EventClass.ScoutReportConfirmed;

    // Only referenced if Scout Disagreement is ACTIVE (post-Month-4 pass).
    // If gate fails, this const is dropped at schema bump per ADR-0004 cross-doc discipline.
    public const EventClass EmitsOnDisagreement = EventClass.ScoutReportDisagreement;
}

public readonly struct LabelEstimate
{
    public PhenotypeLabelId Label;              // from ADR-0006 phenotype enum
    public Fixed Confidence;                    // Q32.32 [0,1]
    public Fixed? LowerBound;                   // Q32.32, optional uncertainty range
    public Fixed? UpperBound;                   // Q32.32, optional uncertainty range
}
```

**Why identical in both paths:** `signatures.md` §Q5 locked that counterplay surfaces via scout reports for **observed** signatures. The UI renders `ScoutReport` records; whether the underlying system is the 3-archetype disagreement model or the single-scout uncertainty fallback, the report shape is the contract the UI binds to. Only the `Confidence` + `LowerBound` / `UpperBound` ranges differ in population character (disagreement: reports from multiple scouts may diverge; uncertainty: single report with wider ranges).

### Conditional-MVP gate-fallback (locked both paths)

**Path A — Month-4 gate PASSES → Scout Disagreement ships at MVP:**
1. 3 prototype archetypes loaded (`PhysicalProfiler`, `TechnicalPurist`, `RegionalExpert`)
2. Each observed player receives 3 `ScoutReport` records (one per active scout sent to observe)
3. Reports may disagree on label confidence + uncertainty ranges
4. Ledger emits `ScoutReportConfirmed` OR `ScoutReportDisagreement` per scout × outcome event
5. Phase 5+ expansion: `TempoReader`, `AcademySpotter`, `SetPieceSpecialist` (post-dependencies)

**Path B — Month-4 gate FAILS → Scout Uncertainty fallback ships instead:**
1. Single `BasicScoutUncertainty` archetype loaded
2. Each observed player receives 1 `ScoutReport` with wider `LowerBound`/`UpperBound` ranges
3. Scout quality (player-authored via scout hiring, if shipped) = uncertainty-range tightening speed
4. Ledger emits only `ScoutReportConfirmed` — `ScoutReportDisagreement` is **dropped at a schema-version bump**, cross-ref ADR-0004 event-class catalog. This triggers the 4-test save-migration fixture discipline.
5. `Scout.Biases.NarrativeFlag = 0` invariant preserved (flags never observable).
6. UI surface unchanged — same `ScoutReport` record renderer.

**Which path is loaded is a SINGLE decision recorded in the 2026-04-24 SPEC decisions log post-Month-4.** No mid-build toggle; no A/B testing in production. One gate decision, one SPEC entry, one path shipped.

### Staged-time feedback loop (Month-4 prototype only)

Per `design/scout-disagreement.md` 2026-04-24 Item B. The prototype doesn't run a full season sim; scripted `later_outcome` entries per test player trigger `ScoutReportConfirmed` / `ScoutReportDisagreement` writes to a minimal ledger slice. Scout reliability scores visibly update between test players. Implementation lives in `FinalWhistle.Scouting.Prototype` — thrown away after the gate verdict; NOT shipped in either path.

### Architecture Sketch

```
                          ┌─────────────────────────────────┐
IdentityPacket (read)  ──►│  ScoutArchetype.ReadFiltered    │
(ADR-0006)                │  - category-level bias applied  │
                          │  - narrative-flag zero-vis      │
                          │  - regional noise if applicable │
                          └─────────────┬───────────────────┘
                                        │ filtered view of InternalGeneSnapshot
                                        v
                          ┌─────────────────────────────────┐
                          │  ScoutReportGenerator           │
                          │  - structured Labels[] + conf   │
                          │  - uncertainty ranges if any    │
                          │  - prose from template          │
                          └─────────────┬───────────────────┘
                                        │ ScoutReport (immutable)
                                        v
                          ┌─────────────────────────────────┐
                          │  ILedgerAppender.Emit           │
                          │  (via Memory.Contracts consts:  │
                          │   ScoutReportConfirmed OR       │
                          │   ScoutReportDisagreement)      │
                          └─────────────────────────────────┘

Path A (gate PASSES): 3 scouts × N players = 3N reports per cycle
Path B (gate FAILS):  1 scout  × N players = N reports (wider ranges)
```

### Implementation Guidelines

- **Asmdef / project boundaries:**
  - `FinalWhistle.Scouting.Contracts` — pure C#, no Unity. Defines `Scout`, `ScoutReport`, `LabelEstimate`, `CategoryBiases`, `ScoutArchetypeKind`. Consumed by MatchSim (for emission + track-record scoring) and by Unity (for report rendering).
  - `FinalWhistle.Scouting.Runtime` — report generation, staged-time feedback loop, Path-A-vs-B dispatcher (reads a single SPEC-decided flag at scene-load, loads the appropriate archetype set).
  - `FinalWhistle.Scouting.Prototype` — Phase-4 Month-4 feel-test scaffolding. Thrown away after the gate verdict.
- **MatchSim.csproj** depends on `FinalWhistle.Scouting.Contracts` ONLY (for emission); same Contracts-split pattern as ADR-0004 / 0005 / 0006.
- **Narrative-flag zero-visibility enforced by validator** — Phase-6 content-pack validator rejects any `Scout.Biases.NarrativeFlag ≠ 0`.
- **Per-archetype `CategoryBiases`** are Phase-3 tuning seeds (live in the archetype SO/JSON); numeric weights not SPEC-locked.
- **Event-class constant discipline** — `ScoutReport.EmitsOnConfirm` = `EventClass.ScoutReportConfirmed`; `EmitsOnDisagreement` = `EventClass.ScoutReportDisagreement`. Rename at `Memory.Contracts` side = compile error here, caught at build time (ADR-0004 cross-doc exact-match discipline).
- **Path-A-vs-B dispatcher is a single decision, not a toggle.** Recorded in SPEC decisions log post-Month-4; content-pack manifest records which archetype set shipped. Save files record the archetype set used at career creation — save-migration fixture discipline (save-migration-fixtures spec) covers Path-A → Path-B incompat if ever needed post-EA.

---

## Alternatives Considered

### Alternative 1: Different ScoutReport schemas per path

- **Description** — Path A uses `ScoutReportDisagreement`-aware schema; Path B uses a simpler `ScoutReportUncertainty` schema.
- **Pros** — Each path's schema is "pure."
- **Cons** — UI renderer must support two shapes; `signatures.md §Q5` counterplay surface must branch on which path shipped; tests multiply. All to save a few unused fields in Path B.
- **Rejected because** — identical-schema-both-paths is the discipline that makes the UI and the counterplay surface stable regardless of gate outcome. Unused fields (e.g., multi-scout aggregation) just sit empty in Path B; cost is a few bytes, not a architecture split.

### Alternative 2: Path-A-vs-B as a runtime toggle

- **Description** — Ship both paths; toggle between them via a settings flag or content-pack config.
- **Pros** — Flexibility; could A/B test post-EA.
- **Cons** — Two codepaths maintained; two test matrices; content-pack validators needing to accept both. The Month-4 gate is explicitly a go/no-go decision per `design/scout-disagreement.md` 2026-04-24 §Q4 ("one remediation pass allowed before hard fallback"). A toggle defeats that discipline.
- **Rejected because** — gate discipline. One gate, one decision, one path ships. Runtime toggle is feature-rescue via back door.

### Alternative 3: Dropping `ScoutReportDisagreement` from the event-class enum pre-Month-4

- **Description** — Don't commit to `ScoutReportDisagreement` in ADR-0004 unless the gate passes.
- **Pros** — No schema churn if gate fails.
- **Cons** — Can't author the prototype without the event class. Can't write the Phase-4 feel-test without the ledger writes. Event-class enums must exist for the feature to be testable.
- **Rejected because** — architecture-slot-reserved-regardless-of-gate is stated by `design/scout-disagreement.md` 2026-04-24 conditional-MVP principle. Schema-bump-on-drop is the right discipline; bake the drop mechanism into the save-migration fixture process, not the ADR-0004 enum authorship.

### Alternative 4: Per-field scout bias (not category-level)

- **Description** — Scouts have per-gene-field biases (22 separate weights).
- **Pros** — Finest-grained scout personality differentiation.
- **Cons** — 22 fields × N archetypes = N×22 weights to tune. Balance-harness sweep tractability collapses. Already rejected in ADR-0006 for the same reason.
- **Rejected because** — inherited from ADR-0006 §Alternative 4. Category-level is tuning-debt-appropriate for MVP.

### Alternative 5: Runtime LLM-generated scout prose

- **Description** — Generate scout-report prose at save-creation time via LLM.
- **Pros** — Infinite variety.
- **Cons** — Runtime LLM is ruled out by `TOOLING.md §Anti-patterns` (inference cost + latency + offline-play impossibility). Same reasoning as ADR-0006 §Alternative 5.
- **Rejected because** — bake-time-only discipline. Prose renders deterministically from templates at report-generation time; stored for replay correctness.

---

## Consequences

### Positive

- Both conditional paths have a committed architecture — no scramble after Month-4 gate verdict either way.
- Identical `ScoutReport` schema makes the UI renderer and `signatures.md §Q5` counterplay surface stable regardless of gate outcome.
- Event-class constant discipline inherits ADR-0005's cross-doc exact-match enforcement — `ScoutReport*` renames caught at compile time.
- `Scout.Biases.NarrativeFlag = 0` validator check is a structural invariant, not prose discipline.
- Category-level scout bias matches ADR-0006's resolution — no ADR-divergence.

### Negative (Accepted Tradeoffs)

- `ScoutArchetypeKind` enum has reserved slots that may never ship (if gate fails, `TempoReader` / `AcademySpotter` / `SetPieceSpecialist` stay unused). Acceptable — enum size is trivial; clarity beats minimalism here.
- Path B (`BasicScoutUncertainty`) ships even if Path A succeeds — fallback archetype always exists in the enum as a documented conditional-MVP artifact. Small clutter cost; preserves the record of the decision.
- `FinalWhistle.Scouting.Prototype` project is explicit throw-away code. Non-shipped; but someone has to maintain it Phase 3-4 until it's deleted.

### Neutral

- Track-record state lives in save-file, not in `Scout` archetype record. Standard separation of immutable-definition vs mutable-save-state.
- Per-archetype `CategoryBiases` are Phase-3 tuning seeds in archetype data, not SPEC-locked — consistent with the Q32.32 tuning-seed discipline from ADR-0004.

---

## Performance Implications

| Metric | Target | Notes |
|---|---|---|
| Report generation per (scout × player) | O(1) bias-filter pass | Category-level, constant-time |
| Path-A full report pass per squad | 3 scouts × 25 players = 75 reports | Runs at scouting-cycle cadence, not per-tick |
| Ledger emissions per cycle | 75 (Path A) or 25 (Path B) events | One-time emission per report; no per-tick cost |
| Staged-time feedback loop (prototype) | <1s per test player | Throwaway Phase-4 code; not shipped |
| Content-pack load of 3 archetypes | <10ms | Trivial JSON parse |

---

## GDD Requirements Addressed

| GDD | System | Requirement | How This ADR Satisfies It |
|---|---|---|---|
| `design/scout-disagreement.md` 2026-04-24 | 3 prototype archetypes | Locked in `ScoutArchetypeKind` enum | Path A section |
| `design/scout-disagreement.md` 2026-04-24 | Conditional-MVP gate | Both paths architecturally committed | Conditional-MVP gate-fallback section |
| `design/scout-disagreement.md` 2026-04-24 | Staged-time ledger feedback | `FinalWhistle.Scouting.Prototype` throwaway project | Staged-time feedback loop section |
| `design/scout-disagreement.md` 2026-04-24 | Event-class contingency | `EmitsOnDisagreement` dropped at schema bump if gate fails | Event-class constant discipline |
| ADR-0004 | Event-class exact-match discipline | `const EventClass` references from `Memory.Contracts`; compile-time rename catches | ScoutReport schema |
| ADR-0006 | Category-level scout bias + narrative-flag zero-visibility | `CategoryBiases.NarrativeFlag = 0` enforced by validator | Scout archetype record |
| `design/signatures.md` §Q5 | Counterplay via scout reports for observed signatures | Identical `ScoutReport` schema across both paths | Why-identical section |

---

## Migration Plan

**Path-B transition (if Month-4 gate fails):**
1. Log a new SPEC decisions-log entry citing the gate outcome + remediation attempt.
2. Schema-bump `MemoryEvent` to drop `ScoutReportDisagreement` from the event-class enum (per ADR-0004 discipline).
3. Add the 4-test save-migration fixture per `design/specs/save-migration-fixtures.md` covering the bump.
4. Update `Scout` catalog to load only `BasicScoutUncertainty`; validator rejects the other three MVP archetypes.
5. `FinalWhistle.Scouting.Prototype` project is deleted from the repo.

**Rollback:** if, post-EA, Path A shipping proves a mistake (e.g., playtest evidence of overwhelming analysis paralysis), supersede with a new ADR switching to Path B. Save-migration fixture policy handles the schema-bump transition.

---

## Validation Criteria

- [ ] Phase 4 Month-4: 3 prototype archetypes authored; 10 hand-authored Identity Packet stubs; staged-time feedback loop produces visibly-updating scout reliability scores between test players.
- [ ] Phase 4 Month-4: feel-test gate verdict logged as SPEC decision per `design/scout-disagreement.md` 2026-04-24 pass criterion.
- [ ] On gate PASS: 3 MVP archetypes ship in `Scout` catalog; `ScoutReportConfirmed` + `ScoutReportDisagreement` both emit.
- [ ] On gate FAIL: `ScoutReportDisagreement` dropped via schema-bump + 4-test save-migration fixture lands; `BasicScoutUncertainty` ships alone; `FinalWhistle.Scouting.Prototype` project deleted.
- [ ] Phase 6: content-pack validator catches a red-team test where a mod pack sets `Scout.Biases.NarrativeFlag > 0`.
- [ ] Phase 6: content-pack validator catches a red-team test where a `Scout.ArchetypeId` embeds a pack-minor version.
- [ ] `ScoutReport` structured `Labels` consumed by the `signatures.md §Q5` counterplay surface regardless of gate outcome — single UI path.
- [ ] Cross-platform: `ScoutReport` emission hashes identical on Win/Mac/Linux Tier-A CI matrix for canonical replay-corpus seeds.

---

## Related

- Depends on: ADR-0004 (event-class constants), ADR-0006 (InternalGeneSnapshot + category-level bias).
- Enables: Phase-4 Month-4 feel-test; `signatures.md §Q5` counterplay surface regardless of gate outcome.
- Cross-refs: `design/scout-disagreement.md` 2026-04-24 resolution (source, including Month-4 pass/fail criterion + staged-time feedback + 10-packet prototype), `design/signatures.md` §Q5 (counterplay surface), `design/event-sourced-memory.md` (event-class enum + save-migration fixture discipline), `design/specs/save-migration-fixtures.md` (4-test bump discipline).
- Code (once implemented): `src/FinalWhistle.Scouting.*` (paths tentative, finalize Phase-3 bootstrap).

---
description: Content-pack validator surface specification. Tier-A fast checks + Tier-D full checks + red-team fixtures + failure-message convention + ownership boundaries. Turns modding.md §12 into an enforceable CI contract.
last_verified: 2026-04-24
status: Phase 2 spec — surface locked; implementation tiers land Phase 3 (Tier-A skeleton) + Phase 6 (full Tier-D). Red-team fixture set grows with each new registry-backed ID class.
---

# Content-Pack Validation Contract — specification

## Purpose

Every content pack — `fwh.core` (base), `fwh.core.v1.patch.N` (delta), and every Workshop mod pack — passes the same validator before it loads. This spec defines the validator's surface: which checks run at which tier, what each check emits on failure, which asmdef owns each check, and which fixtures prove each check actually catches what it claims to catch.

Mod-loadability is an architectural posture (per `design/modding.md §12`). This spec is the enforcement layer. If a check isn't listed here with a red-team fixture, it doesn't count.

## Why this spec exists (not an ADR)

The architectural decisions are already locked:

- **ADR-0004 §Cross-doc exact-match** — event-class enum names must match `design/signatures.md` + `design/scout-disagreement.md` + ADR-0005 / ADR-0007. Rename in any one surface is a validator failure.
- **ADR-0004 §CallbackTag** — every tag declares ≥1 `ConsumingReaders`. Lint-enforced.
- **ADR-0006 §Validator coverage** — ID format + no-pack-minor-in-ID + duplicate-name + legal-sensitive-names-diff + SignatureCandidate resolution + banned phenotype-label leakage.
- **ADR-0007 §NarrativeFlag zero-visibility validator** — every scout archetype's `NarrativeFlag` bias weight = 0. Enforced.
- **ADR-0001 §Registry-backed IDs** — unknown `ChainConditionId` in a content pack fails validation.
- **ADR-0005 §SimBiasFieldId resolution** — unknown or deprecated field ID rejected against the `MatchSim.Contracts`-owned registry.
- **`design/modding.md §12`** — the full Tier-A + Tier-D validator surface enumerated at the doc level.
- **ADR-0003 Tier-A + Tier-D** — CI contract: Tier-A blocks every PR (≤5 min Linux), Tier-D blocks RC (longer budget permitted).
- **`design/ui-vocabulary.md`** — Category-A hard-ban + Category-B audited-exemption catalog that the banned-terms check draws from.

This spec pins the **on-disk file layout + per-check contract + failure-message convention + red-team fixture shape** that implements those commitments.

## Locked decisions

- **Two tiers, no third.** Tier-A = fast fail-fast on every PR, with the import-safe Tier-A subset also running on every pack import. Tier-D = full blocking suite at RC. No optional tier, no warn-only mode for shipping checks. A check either blocks in its declared scope or it isn't here yet.
- **Every check has an ID** of the form `FW-VAL-<tier>-<NNN>`. IDs are append-only; retired checks keep their ID and get marked `superseded by FW-VAL-<...>` in the catalog. Modders + tooling reference checks by ID in failure output + suppression flags.
- **Every check has a red-team fixture** at `MatchSim.Tests/fixtures/validator-red-team/FW-VAL-<id>.pack/` — a minimal synthetic content pack engineered to trip the target check and only the target check. Tier-A tests assert the fixture fails the named check; Tier-D tests assert the full suite catches all red-team fixtures.
- **Failure messages are actionable:** every failure output includes (1) check ID, (2) offending pack ID, (3) offending entity ID / field path / line number where resolvable, (4) one-sentence human explanation, (5) remediation hint or link to this spec. No bare stack traces, no ambiguous "invalid content".
- **Validator runs the same code at every tier.** Tier-A runs a subset of the same binary that Tier-D runs; the difference is filter flags (`fw content-lint --tier=a --scope=ci` vs `--tier=a --scope=import` vs `--tier=d --scope=ci`), not a re-implementation. One implementation of each check; one code path.
- **Checks declare tier + scope.** Each check declares `min_tier: A | D` and `applies_to: ci | import | both`. The runner filters. A Tier-A check promoted to Tier-D would only happen via explicit SPEC entry; the reverse is not supported (Tier-D checks may later be optimized to Tier-A, which is a pure win and logged in this spec's changelog).
- **Zero network I/O, zero runtime evaluation.** The validator reads pack files + registry snapshots + the `design/ui-vocabulary.md`-derived banned-term list and emits JSON + human-readable report. No HTTP, no LLM, no Unity Editor runtime.

## Tier-A check catalog (fail-fast; ≤5-min ceiling)

Every Tier-A check runs on every PR. Tier-A checks with `applies_to: import` or `applies_to: both` also run on user-installed pack import; CI-only checks may reference repository fixtures and never block a Workshop pack at runtime. Each check has an ID, owner asmdef, red-team fixture ID, scope, and failure-message template. In the table below, checks apply to both CI and import unless the name or message marks them `CI-only`.

| Check ID | Name | Owner asmdef | Red-team fixture | Failure message template |
|---|---|---|---|---|
| `FW-VAL-A-001` | **Pack manifest schema** | `Content.Validator` | `FW-VAL-A-001.pack` — manifest with missing `pack_id` field | `[FW-VAL-A-001] Pack at <path> has malformed manifest: <field> missing/invalid. See design/specs/content-pack-validation-contract.md#FW-VAL-A-001.` |
| `FW-VAL-A-002` | **Pack-ID format** | `Content.Validator` | `FW-VAL-A-002.pack` — manifest with `pack_id: "Final Whistle Core"` (spaces, caps) | `[FW-VAL-A-002] Pack <pack_id> fails ID format (expected lowercase dot-separated segments, e.g. 'fwh.core' or 'modname.featurepack').` |
| `FW-VAL-A-003` | **Entity-ID format — player** | `Content.Validator` | `FW-VAL-A-003.pack` — player with ID `fwh.core.v1.1:player_00042` (pack-minor leak) | `[FW-VAL-A-003] Entity <entity_id> in pack <pack_id> violates canonical regex `^fwh\.core(?:\.v[0-9]+)?:player_[0-9]{5}$`. Pack-minor versions never appear in entity IDs; see design/modding.md §1.` |
| `FW-VAL-A-004` | **Entity-ID format — kind-slug** | `Content.Validator` | `FW-VAL-A-004.pack` — signature with ID `fwh.core:Signature.LowCutback` (capitalized, PascalCase) | `[FW-VAL-A-004] Entity <entity_id> in pack <pack_id> violates kind-slug regex (lowercase dot-separated, e.g. 'fwh.core:signature.low-cutback-from-byline').` |
| `FW-VAL-A-005` | **Duplicate entity-ID within pack** | `Content.Validator` | `FW-VAL-A-005.pack` — two SOs with the same `fwh.core:signature.duel-winner` ID | `[FW-VAL-A-005] Pack <pack_id> contains duplicate entity ID <entity_id> (first seen at <path1>, again at <path2>).` |
| `FW-VAL-A-006` | **Unresolved `ContentPackQualifiedId` reference** | `Content.Validator` | `FW-VAL-A-006.pack` — SignatureSO referencing `fwh.core:signature.nonexistent` as a prerequisite | `[FW-VAL-A-006] Entity <source_entity_id>.<field> references unresolved content-pack-qualified ID <target_id>. Target entity not present in pack <pack_id> or any currently-declared base.` |
| `FW-VAL-A-007` | **Unknown `ChainConditionId`** | `Viewer.Cinema.Validator` | `FW-VAL-A-007.pack` — ShotTypeSO with `chain_rules: [{condition: "on-pineapple"}]` | `[FW-VAL-A-007] ShotTypeSO <entity_id> references unknown ChainConditionId <value>. Valid IDs: <list>. Registry owner: FinalWhistle.Viewer.Cinema.` |
| `FW-VAL-A-008` | **Unknown `SimBiasFieldId`** | `MatchSim.Validator` | `FW-VAL-A-008.pack` — SignatureSO referencing `sim_bias_field: "magic_boost"` | `[FW-VAL-A-008] SignatureSO <entity_id> references unknown SimBiasFieldId <value>. Registry owner: MatchSim.Contracts. Known fields: <list>.` |
| `FW-VAL-A-009` | **Unknown `EventClass`** | `Memory.Validator` | `FW-VAL-A-009.pack` — SignatureSO with `emits_on_awaken: "SignatureExploded"` | `[FW-VAL-A-009] Entity <entity_id>.<field> references unknown EventClass <value>. Registry owner: Memory.Contracts.` |
| `FW-VAL-A-010` | **Unknown `CallbackTag`** | `Memory.Validator` | `FW-VAL-A-010.pack` — MemoryEvent template with `callback_tags: ["not-a-real-tag"]` | `[FW-VAL-A-010] Entity <entity_id>.<field> references unknown CallbackTag <value>. Registry owner: Memory.Contracts.` |
| `FW-VAL-A-011` | **`CallbackTag.ConsumingReaders ≥ 1`** | `Memory.Validator` | `FW-VAL-A-011.pack` — CallbackTag registered with empty `consuming_readers` | `[FW-VAL-A-011] CallbackTag <tag_id> declares no consuming readers. Every tag must be consumed by ≥1 reader (ADR-0004 §CallbackTag).` |
| `FW-VAL-A-012` | **Unknown `PhenotypeLabelId`** | `Content.Validator` | `FW-VAL-A-012.pack` — IdentityPacket with `scout_labels: ["made-up-label"]` | `[FW-VAL-A-012] IdentityPacket <entity_id>.scout_labels references unknown PhenotypeLabelId <value>. Registry owner: Content.Contracts.` |
| `FW-VAL-A-013` | **Unknown `ScoutArchetypeKind`** | `Scouting.Validator` | `FW-VAL-A-013.pack` — ScoutArchetype with `kind: "PineapplePicker"` | `[FW-VAL-A-013] ScoutArchetype <entity_id>.kind unknown value <value>. Registry owner: Scouting.Contracts.` |
| `FW-VAL-A-014` | **`NarrativeFlag` scout-bias = 0** | `Scouting.Validator` | `FW-VAL-A-014.pack` — ScoutArchetype with `biases.NarrativeFlag: 0.3` | `[FW-VAL-A-014] ScoutArchetype <entity_id> violates NarrativeFlag zero-visibility invariant (bias_weight=<value>, must be 0). See ADR-0007 §NarrativeFlag.` |
<!-- ui-lint:ignore-start reason="red-team fixture example must cite a Category-A banned token by name to document what the check catches" -->
| `FW-VAL-A-015` | **Banned UI vocabulary — Category A** | `Content.Validator` (uses `scripts/lint-banned-terms.py` engine) | `FW-VAL-A-015.pack` — SignatureSO with `DisplayName: "The Hush"` | `[FW-VAL-A-015] Entity <entity_id>.<field> contains Category-A banned term <term> at <path>:<line>. Category A has no exemption path; see design/ui-vocabulary.md.` |
<!-- ui-lint:ignore-end -->
| `FW-VAL-A-016` | **Banned UI vocabulary — Category B without audited exemption** | `Content.Validator` | `FW-VAL-A-016.pack` — phenotype-label prose using `morale` with no `ui-lint:allow` sentinel | `[FW-VAL-A-016] Entity <entity_id>.<field> contains Category-B term <term> without inline `ui-lint:allow term="..." reason="..." reviewer="..."` exemption. See design/ui-vocabulary.md.` |
| `FW-VAL-A-017` | **First-party schema-version bump requires fixture (CI-only)** | `Content.Validator` | `FW-VAL-A-017.pack` — repository PR raises a first-party schema to version 2 but omits the v1 migration fixture under `MatchSim.Tests/fixtures/saves/` | `[FW-VAL-A-017] Repository schema bump for <schema> to version <N> lacks MatchSim.Tests/fixtures/saves/<schema>-v<N-1>.json. First-party schema bumps require the 4-test fixture per design/specs/save-migration-fixtures.md. Scope: CI only; user pack import validates manifest compatibility, not repository fixture paths.` |
| `FW-VAL-A-018` | **No `Fixed`→`float` drift in SimBias values** | `MatchSim.Validator` | `FW-VAL-A-018.pack` — SignatureSO with a sim-bias value serialized as float literal `0.27` (should be Q32.32 decimal string `"0.2700000000"`) | `[FW-VAL-A-018] Entity <entity_id>.<field> carries non-Fixed sim-bias value <raw>. All sim-affecting values must serialize as fixed-point decimal string; see TECH_APPROACH.md §3.2.` |
| `FW-VAL-A-019` | **Rendered strings fit declared locale sets** | `Content.Validator` | `FW-VAL-A-019.pack` — phenotype label declared in `en_GB` locale set but prose contains only placeholder `"<TODO: translate>"` | `[FW-VAL-A-019] Entity <entity_id>.<field> declares locale <loc> but content is empty / placeholder. Locked locale set for EA: en_GB (JP / ES / PT / DE deferred to Phase 7).` |
| `FW-VAL-A-020` | **`ScoutReport.NarrativeFlag` never populated by bias path** | `Scouting.Validator` | `FW-VAL-A-020.pack` — ScoutArchetype with `biases.NarrativeFlag: 0` but emits `ScoutReport.GeneCategoryEstimate` with `Category: NarrativeFlag` | `[FW-VAL-A-020] ScoutArchetype <entity_id> produces GeneCategoryEstimate with Category=NarrativeFlag via bias path (forbidden even at weight=0). NarrativeFlag surfaces only via trigger events per ADR-0007.` |
| `FW-VAL-A-021` | **Missing `reduce_motion_variant` for motion-heavy shot** | `Viewer.Cinema.Validator` | `FW-VAL-A-021.pack` — ShotTypeSO uses `impact-flash` or `motion-lines` render feature but omits `reduce_motion_variant` | `[FW-VAL-A-021] ShotTypeSO <entity_id> uses motion-heavy render feature <feature> but declares no reduce_motion_variant. Shipping content must provide reduce-motion coverage by Phase-6 content-pack v1 / EA lock; see design/accessibility.md §Reduce-motion toggle.` |

**Scope note:** `FW-VAL-A-017` is CI-only because it checks repository migration-fixture discipline. User-installed pack import never reads `MatchSim.Tests/fixtures/saves/`; it validates the pack manifest's declared schema compatibility against the installed binary's supported schema range and then either loads or fails with the manifest/compatibility check ID.

**Tier-A budget: all 21 checks run in <30s on the `fwh.core` base pack at Phase-6 size (~96 clubs, ~2400 players).** Per-PR budget when only the delta pack is scanned: <5s.

## Tier-D check catalog (full suite; RC gate only)

Tier-D runs everything in Tier-A plus the checks below. Timing budget uncapped (but expected <10 min on `fwh.core` at Phase-6 size).

| Check ID | Name | Owner asmdef | Red-team fixture | Failure message template |
|---|---|---|---|---|
| `FW-VAL-D-001` | **Legal-sensitive name diff** | `Content.Validator` (reads gitignored `dev-config/compiler/legal-sensitive-names.json`) | `FW-VAL-D-001.pack` — club with name matching an entry in the sensitive-names reference list | `[FW-VAL-D-001] Entity <entity_id>.<field> value <value> matches legal-sensitive-names reference (real-world club / real person / trademark). Review required; see design/worldbuilding.md §compiler-internal analogues.` |
| `FW-VAL-D-002` | **Real-world region-analogue leakage** | `Content.Validator` | `FW-VAL-D-002.pack` — prose mentioning real-world place names from `dev-config/compiler/region-analogues.json` | `[FW-VAL-D-002] Entity <entity_id>.<field> leaks compiler-only region analogue <value>. Design-internal analogue strings must not ship in runtime content.` |
| `FW-VAL-D-003` | **Locale coverage — all declared locales complete** | `Content.Validator` | `FW-VAL-D-003.pack` — phenotype label with `en_GB` prose but no `ja_JP` (pack declares `ja_JP` in manifest) | `[FW-VAL-D-003] Entity <entity_id> declares locale set <set> but missing translation for <locale> at <field>.` |
| `FW-VAL-D-004` | **Cross-doc event-class exact-match** | `Memory.Validator` | `FW-VAL-D-004.pack` — SignatureSO emits `SignatureAwoken` (typo; should be `SignatureAwakened`) | `[FW-VAL-D-004] Entity <entity_id>.<field> EventClass value <value> diverges from canonical registry name (closest: <closest>). Rename in any surface = drift; see ADR-0004 §cross-doc exact-match.` |
| `FW-VAL-D-005` | **AI-content disclosure manifest complete** | `Content.Validator` | `FW-VAL-D-005.pack` — pack manifest omits `ai_content_disclosure` block while containing compiler-generated entities | `[FW-VAL-D-005] Pack <pack_id> omits ai_content_disclosure block. Required per Steam 2025 AI-content policy + SETUP.md §7.` |
| `FW-VAL-D-006` | **SignatureCandidate affinity resolves** | `Content.Validator` | `FW-VAL-D-006.pack` — IdentityPacket with `signature_candidates: [{signature_id: "fwh.core:signature.cut-of-the-jib", affinity: 0.4}]` (ID doesn't exist) | `[FW-VAL-D-006] IdentityPacket <entity_id>.signature_candidates[<n>] references SignatureSO <id> which is not present in any loaded pack. ADR-0006 affinity contract violated.` |
| `FW-VAL-D-007` | **Pack manifest pack-minor discipline** | `Content.Validator` | `FW-VAL-D-007.pack` — delta pack v1.1 with an entity that lacks `introduced_in_pack_version` metadata | `[FW-VAL-D-007] Entity <entity_id> in delta pack <pack_id> lacks introduced_in_pack_version manifest field. Required per design/modding.md §1 + player-generation.md §ID stability.` |
| `FW-VAL-D-008` | **Asset-licensing coverage** | `Content.Validator` | `FW-VAL-D-008.pack` — pack containing an asset whose source is not registered in `steam-release/asset-licensing-tracker.csv` | `[FW-VAL-D-008] Asset at <path> in pack <pack_id> not registered in asset-licensing-tracker.csv. Required for every third-party asset; see SETUP.md §7.` |
| `FW-VAL-D-009` | **Determinism-replay parity for base pack** | `MatchSim.Validator` | `FW-VAL-D-009.pack` — base-pack edit that changes a signature's SimBias without bumping `fwh.core` pack version | `[FW-VAL-D-009] Pack <pack_id> version <ver> hash diverges from golden replay corpus baseline (`0xdeadbeefdeadbeef` expected <hash>, got <hash>). Sim-affecting changes require pack-version bump + corpus entry update; see design/specs/golden-replay-corpus.md.` |
| `FW-VAL-D-010` | **Banned-terms audit report clean for EA + RC lock** | `Content.Validator` | `FW-VAL-D-010.pack` — pack with 50+ Category-B exemptions (EA lock threshold) | `[FW-VAL-D-010] Category-B audited-exemption count (<n>) exceeds EA-lock threshold (<threshold>). Review and resolve before RC; see design/ui-vocabulary.md §Category B.` |

**Tier-D budget:** all Tier-A + Tier-D checks run on the full `fwh.core` + any enabled Workshop packs at RC time. Expected <10 min; hard ceiling 30 min before budget review.

## Red-team fixture design

### Path convention

```
MatchSim.Tests/
  fixtures/
    validator-red-team/
      FW-VAL-A-001.pack/
        manifest.json              # intentionally missing pack_id
        README.md                  # describes what this fixture is engineered to trip
      FW-VAL-A-002.pack/
      ...
      FW-VAL-D-009.pack/
```

Each fixture is a **minimal synthetic content pack** engineered to:

1. **Trip one and only one check.** A Tier-A test runs `fw content-lint <fixture>` and asserts (a) exit code non-zero, (b) failure output contains `[FW-VAL-<id>]` token, (c) failure output does NOT contain any other `FW-VAL-` token (no false positives on unrelated checks).
2. **Ship a README** at `FW-VAL-<id>.pack/README.md` describing the engineered failure in one paragraph + the expected failure message excerpt. Makes the fixture self-documenting for the Phase-6 implementer.
3. **Be minimal.** No more files than required to trip the check. If a check needs an IdentityPacket to reference a broken SignatureCandidate ID, the fixture contains exactly one IdentityPacket + the broken reference + a manifest. No padding.

### Growth policy

- **Every new check added to this spec ships with its red-team fixture in the same PR.** Spec entries without fixtures are unmergeable. This mirrors the 4-tests-per-schema-bump discipline in `save-migration-fixtures.md`.
- **Fixtures accumulate forever.** Retired checks keep their fixtures (marked `superseded` in the fixture README) because historical red-team fixtures prove the validator once caught this class of failure — useful for regression rollback decisions.
- **Red-team fixtures never ship to players.** They live under `MatchSim.Tests/fixtures/validator-red-team/` and are excluded from Addressables groups by asmdef boundary.

### Anti-red-team fixture — the negative control

One fixture per tier lives at `MatchSim.Tests/fixtures/validator-clean/minimal.pack/` — a minimal content pack that **passes all checks**. Tier-A tests assert this fixture exits 0. Protects against the "every fixture fails because the runner broke" failure mode.

## Failure-message convention (binding)

Every failure output emitted by the validator, in any tier, MUST:

1. **Start with the check ID in square brackets:** `[FW-VAL-A-003]`.
2. **Name the offending pack and entity when resolvable:** `Pack fwh.core, entity fwh.core:player_00042`.
3. **State the invariant violated in one sentence:** `violates canonical regex ^fwh\.core(?:\.v[0-9]+)?:player_[0-9]{5}$`.
4. **Point at the fix:** `See design/modding.md §1` or `See ADR-0007 §NarrativeFlag`.
5. **Not include stack traces in the human-readable output.** Stack traces go to `--verbose` mode only.

### JSON output shape (`fw content-lint --format=json`)

```jsonc
{
  "pack_id": "fwh.core",
  "pack_version": "1.0.0",
  "tier": "a",
  "result": "fail",
  "checks_run": 20,
  "failures": [
    {
      "check_id": "FW-VAL-A-003",
      "severity": "block",
      "pack_id": "fwh.core",
      "entity_id": "fwh.core.v1.1:player_00042",
      "field_path": "$.id",
      "message": "Entity fwh.core.v1.1:player_00042 violates canonical regex...",
      "remediation_link": "design/modding.md#1-content-pack-qualified-stable-ids-no-pack-minor-leak"
    }
  ],
  "successes": ["FW-VAL-A-001", "FW-VAL-A-002", "..."],
  "duration_ms": 4200
}
```

Machine-readable failure output is what CI annotations, IDE integrations, and Phase-9 mod-editor UX will build against. The schema is additive-only from here.

## Ownership boundaries

Each check lives in the asmdef that owns the relevant registry — NOT in a central `Content.Validator` monolith. This keeps the registry owner authoritative and prevents drift.

| Asmdef | Checks owned | Registry authoritative |
|---|---|---|
| `Content.Validator` | FW-VAL-A-001/002/003/004/005/006/012/015/016/017/019 + D-001/002/003/005/006/007/008/010 | ContentPackQualifiedId format, PhenotypeLabelId registry, pack manifest, locale sets, banned-terms list, legal-sensitive-names + region-analogues references, AI-disclosure manifest, asset licensing |
| `Viewer.Cinema.Validator` | FW-VAL-A-007 | ChainConditionId |
| `MatchSim.Validator` | FW-VAL-A-008, A-018, D-009 | SimBiasFieldId, Fixed-point value format, golden replay corpus parity |
| `Memory.Validator` | FW-VAL-A-009, A-010, A-011, D-004 | EventClass, CallbackTag + ConsumingReaders, cross-doc enum exact-match |
| `Scouting.Validator` | FW-VAL-A-013, A-014, A-020 | ScoutArchetypeKind, NarrativeFlag zero-visibility invariant, ScoutReport category integrity |

The top-level `fw content-lint` command is a thin orchestrator that loads the asmdef-owned validator plugins, runs the filter-by-tier pass, aggregates results, and emits the unified report. No validation logic lives in the orchestrator.

### Why this split matters for mods

When a mod pack introduces a new signature that references an unknown `EventClass`, the validation error surfaces from `Memory.Validator` — whose owner (the `Memory.Contracts` team, which in solo-dev terms means "the ADR-0004 author") is the right person to decide whether a future first-party binary should add the EventClass or the mod should use an existing one. Decentralizing validator ownership decentralizes the review surface.

## CI wiring

### Tier A — every PR

```yaml
# .github/workflows/fast-pr-ci.yml (snippet — actual wiring lands Phase 3)
- name: Content-pack validation (Tier A)
  run: ./scripts/fw content-lint --tier=a --scope=ci --pack=fwh.core
```

Fails the job on any `FW-VAL-A-*` failure. Emits GitHub Actions annotations per-failure using the check ID as the annotation title. Job budget: 5 min (shared with `fw verify` umbrella).

### Pack import — installed content

```sh
./scripts/fw content-lint --tier=a --scope=import --pack=<installed-pack>
```

Runs only import-safe Tier-A checks. It never reads repository-only fixture paths such as `MatchSim.Tests/fixtures/saves/`.

### Tier D — RC only

```yaml
# .github/workflows/release-candidate.yml (Phase 8)
- name: Content-pack validation (Tier D full)
  run: ./scripts/fw content-lint --tier=d --scope=ci --pack=fwh.core --pack=<synthetic-thin-mod-pack>
```

Runs on tagged RC builds. Required to pass before Tier-E (Steam deploy) is invokable.

### Red-team self-check

```yaml
# Runs in Tier-A — proves the validator catches every engineered failure
- name: Validator red-team self-check
  run: ./scripts/fw content-lint --red-team-self-check
```

This iterates every `MatchSim.Tests/fixtures/validator-red-team/FW-VAL-*.pack/` fixture, asserts the validator exits non-zero with the expected check ID, and fails the job if any fixture is missing or any check doesn't fire. Prevents the "added a new check but no fixture" anti-pattern from slipping through even on human review.

## Synthetic thin-mod-pack CI fixture (owed Phase 6)

Per `design/modding.md §Prototype gate`, a synthetic thin mod pack proves the mod-loadability contract end-to-end at Phase 6. New SPEC task owed (added alongside this spec):

**Phase 6 task:** Author `MatchSim.Tests/fixtures/mod-packs/thin-mod.fwh.mod.v1/` — a minimal Workshop-shaped mod pack containing: 1 new signature (references existing `SimBiasFieldId` values), 1 new shot type (references existing `ChainConditionId` values), 5 new IdentityPackets / players using existing `PhenotypeLabelId` values, and 1 new ScoutArchetype using an existing `ScoutArchetypeKind`. The fixture must not modify `Content.Contracts`, add enum/registry values, or require a schema bump. Pack must load cleanly alongside an unchanged `fwh.core@1.0.0` binary at Tier-D. Failure = content pack v1 not actually mod-ready.

This is the Phase-6 integration test for the full modding contract (`design/modding.md §12`) + this validation spec. Added to SPEC.md Phase 6 in the same /done pass as this spec.

## Growth policy

- **New check IDs are append-only** — `FW-VAL-A-022`, `FW-VAL-A-023`, ... No ID reuse even for superseded checks.
- **Tier-D → Tier-A promotion** is allowed (optimization wins are free); annotate in this doc's changelog.
- **Tier-A → Tier-D demotion** requires SPEC decisions-log entry citing the performance evidence.
- **Deleted checks** keep their ID + red-team fixture, marked `superseded by FW-VAL-<new-id>` or `retired (reason)` in the catalog.
- **Spec size ceiling:** ~50 checks total expected by Phase 8 EA. If the list grows past 60, review whether some checks should decompose into sub-specs (e.g. `content-pack-i18n-validation.md`).

## Phase rollout

| Phase | Deliverable |
|---|---|
| Phase 3 | `Content.Validator` + `Memory.Validator` + `MatchSim.Validator` + `Scouting.Validator` + `Viewer.Cinema.Validator` asmdef skeletons with 5-8 Tier-A checks live (the ID-format + registry-backed-ID family). `fw content-lint --tier=a` wired into `fw verify`. Red-team fixtures for every implemented check. |
| Phase 4 | Checks added as scout-disagreement prototype + first signatures land (FW-VAL-A-014 / A-020). |
| Phase 6 | Full Tier-A surface (all 21 `FW-VAL-A-*`) complete. Synthetic thin-mod-pack fixture lands. Tier-D `FW-VAL-D-004` / D-006 / D-007 / D-009 live. |
| Phase 7 | Tier-D `FW-VAL-D-001` / D-002 / D-003 (locale coverage matured as localization pass completes). |
| Phase 8 | Full Tier-D (`FW-VAL-D-005` AI-content disclosure, `FW-VAL-D-008` asset-licensing, `FW-VAL-D-010` Category-B exemption audit). RC workflow wired. |

## Cross-references

- **Contract source:** [design/modding.md §12](../modding.md) — enumerates the Tier-A + Tier-D surface at the architectural level
- **CI tier policy:** [ADR-0003 Production Pipeline](../adr/adr-0003-production-pipeline.md) — Tier-A / Tier-D definitions
- **Check origin ADRs/docs:** [ADR-0001](../adr/adr-0001-shot-type-so-schema.md) + [accessibility.md](../accessibility.md) (`reduce_motion_variant` coverage), [ADR-0004](../adr/adr-0004-memory-event-schema.md) (EventClass + CallbackTag enum matching), [ADR-0006](../adr/adr-0006-identity-packet-compiler.md) (player ID format + validator coverage bullets), [ADR-0007](../adr/adr-0007-scout-archetype-schema.md) (NarrativeFlag zero-visibility)
- **Sibling specs:** [golden-replay-corpus.md](golden-replay-corpus.md) / [save-migration-fixtures.md](save-migration-fixtures.md) — same discipline applied to replay + save fixtures
- **Banned-terms engine:** `scripts/lint-banned-terms.py` + [design/ui-vocabulary.md](../ui-vocabulary.md)
- **Locked-locale source:** `design/overview.md` + Phase-7 localization pass

## Changelog within this doc

- **2026-04-24** — Authored as Phase-2 spec. 21 Tier-A checks + 10 Tier-D checks catalogued with IDs, owners, red-team fixtures, failure-message templates. Ownership decentralized across 5 validator asmdefs. Failure-message convention binding. JSON output shape pinned. Phase-6 synthetic thin-mod-pack fixture owed as new SPEC task. CI wiring sketched (full wiring lands Phase 3 via `fw content-lint` + Phase 8 RC workflow). Red-team self-check discipline locked: every new check ships with its fixture in the same PR.

---
description: Golden replay corpus format specification. Canonical-seed match fixtures with expected hashes that protect MatchSim from regression across Win/Mac/Linux.
last_verified: 2026-04-24
status: Phase 2 spec — format locked; first corpus entries authored in Phase 3 Week 2
---

# Golden Replay Corpus — format specification

## Purpose

Make the determinism posture from ADR-0001 / ADR-0002 / ADR-0003 **testable as an artifact**, not just a philosophy. A small set of canonical match seeds with expected hashes gives CI a cheap cross-platform regression guard that catches any change to the sim, ball physics, behavior trees, signature dispatch, shot selection, or render-feature pass activation — all from a single Linux job running `scripts/fw replay <seed>`.

## Why this spec exists (not an ADR)

This doc specifies a **file format + CI-contract**, not an architectural decision. The architectural decisions are already made:

- `design/match-engine.md` 2026-04-24 — Q32.32 fixed-point, deterministic 60Hz tick
- ADR-0001 — deterministic shot selection contract
- ADR-0002 — pass-activation log as the determinism verifier (not pixel compare)
- ADR-0003 — Tier-A CI runs one canonical seed as smoke; Tier-C local regenerates + diffs the full corpus; Tier-D re-runs full corpus as RC gate

This spec defines the on-disk artifact shape those systems read and emit.

## Locked decisions

- **Corpus is an append-only set of JSON fixtures** at `MatchSim.Tests/fixtures/replay-corpus/<seed-hex>.json`. Never edited; only added. Regeneration is explicit (`fw replay --regenerate-corpus <seed>`) and produces a delta commit reviewed on PR.
- **Every corpus entry is self-describing** — content-pack version, archetype IDs, expected hashes — so the fixture can be validated without consulting external metadata.
- **Hashes are computed from canonical Q32.32 state + ordered event stream**, not from rendered frames. Cross-GPU / cross-driver pixel differences are acceptable; sim-state drift is not.
- **Entries carry a `verification_scope` tag** to distinguish smoke-only fixtures (Tier A) from full-matrix fixtures (Tier D).
- **One fixture pinned at `0xDEADBEEFDEADBEEF` as the Tier-A smoke seed** — stable constant, always valid, references in `fast-pr-ci.yml` by name.

## File format

### Path convention

```
MatchSim.Tests/
  fixtures/
    replay-corpus/
      0xdeadbeefdeadbeef.json        # Tier-A smoke (stable constant)
      0x<16 hex digits>.json          # additional corpus entries; named by match_seed
```

Filename MUST match the `match_seed` field. Rename = new fixture, not edit.

### Schema (v1)

```jsonc
{
  "corpus_schema_version": 1,

  // Inputs — together these fully determine the expected outputs.
  "match_seed": "0xdeadbeefdeadbeef",          // u64, hex with 0x prefix
  "content_pack_version": "fwh.core@1.0.0",    // exact pack ID + semver as loaded

  "home_archetype_id": "fwh.core:archetype.direct-pressing",
  "away_archetype_id": "fwh.core:archetype.low-block-counter",

  "reduce_motion": false,                       // affects pass-activation log only

  "sim_length_ticks": 324000,                   // 60Hz × 90min = 324,000; full match
  "tick_rate_hz": 60,

  // Expected outputs — deterministic across Win/Mac/Linux with the locked
  // fixed-point canonical state. Drift = regression.
  "expected": {
    "final_score": [1, 2],                      // [home, away]

    // Ordered hash of every ledger-emission event in canonical order.
    // Covers: goal, SignatureExecuted, SignatureAwakened, fouls (Phase 4+),  <!-- ui-lint:allow term="awakened" reason="event-class name per MemoryEvent enum (design/event-sourced-memory.md)" reviewer="osagberg" -->
    // substitutions (Phase 4+), callback-surfaced events, etc.
    "key_event_hashes": [
      "sha256:a1b2c3...",
      "sha256:d4e5f6..."
      // ... one entry per ledger-emission event; order matters
    ],

    // Single hash over the full canonical MatchSim state at final tick —
    // Q32.32 fixed-point state dump, serialized in a documented stable order.
    "final_canonical_state_hash": "sha256:...",

    // Pass-activation log from ADR-0002. Empty until Phase 3 viewer lands;
    // once populated, one entry per shot-change event, covering:
    //   - active ShotTypeSO.Id
    //   - reduce-motion variant active (bool)
    //   - impact-flash fired (bool)
    //   - render-feature toggle states
    "pass_activation_log_hash": "sha256:..."
  },

  // Authoring metadata — for humans reading the fixture in a PR review.
  "verification_scope": "tier-a-smoke",          // enum: tier-a-smoke | tier-d-full | archive
  "generated_at": "2026-04-24T00:00:00Z",
  "generated_by": {
    "tool": "scripts/fw replay --regenerate-corpus",
    "build_info": {
      "unity_version": null,                    // null until Phase 3 integration
      "matchsim_commit": "<git sha at regeneration>",
      "content_pack_hash": "sha256:..."         // derived from content_pack_version
    }
  },
  "description": "Opening-day league fixture: direct-pressing vs low-block-counter. Month-3 slice default scenario.",
  "notes": "If this hash changes, investigate before re-baselining — silent regeneration is a hard-ban."
}
```

### Stable serialization rules

These rules make the hashes reproducible across runs / platforms:

1. **JSON canonicalization:** 2-space indent, LF line endings, keys in the order shown above (not alphabetical — structural order is more readable for PR review).
2. **Hex integers** MUST use lowercase (`0xdeadbeef`, not `0xDEADBEEF`) to avoid cross-toolchain case mismatches.
3. **Float values forbidden** — all sim state uses Q32.32; fixtures store raw integer representations when precision matters.
4. **Hash algorithm:** SHA-256 with standard hex encoding. `sha256:` prefix is part of the stored string.
5. **Event-stream serialization** (the input to `key_event_hashes` and `final_canonical_state_hash`): documented in `MatchSim.Tests/SerializationContract.cs` once the sim lands; must be bit-identical across platforms. ADR-0004 (MemoryEvent) locks the MemoryEvent-side of this.

## CI contract

### Tier A (Phase 3+)

```yaml
- name: Determinism smoke
  run: ./scripts/fw replay 0xdeadbeefdeadbeef --compare-corpus
```

`fw replay <seed> --compare-corpus` runs the seed through MatchSim headless, computes actual hashes, compares against `<seed>.json` expected fields, exits 0 on match / non-zero on drift. Must complete in under 30 seconds on GitHub-hosted Linux.

### Tier C (local / self-hosted, Phase 3+)

```bash
# Regenerate all corpus hashes from scratch after an intentional sim change.
scripts/fw replay --regenerate-corpus
```

- Walks every `.json` in `fixtures/replay-corpus/`, re-runs each seed, writes fresh `expected` fields.
- **Never runs on CI.** Regeneration is an intentional developer action; the resulting diff is the PR artifact reviewers check.
- Produces a side-by-side diff report of old-vs-new hashes so drift is explicit in PR review.

### Tier D (RC, Phase 8)

```yaml
- name: Full corpus verification
  run: ./scripts/fw replay --compare-corpus --all
  # Runs every fixture with verification_scope != archive; must all pass.
```

Tier D matrix across Win/Mac/Linux — identical expected-hashes on every platform or the RC gate fails.

## Corpus growth policy

- **Every schema bump** (content pack, MemoryEvent, IdentityPacket, SignatureSO, ShotTypeSO) requires **at least one new corpus entry** exercising the change before the schema bump merges.
- **Every determinism bug found in the wild** is reproduced as a new corpus entry before the fix merges — the fixture is the regression gate.
- **Corpus target at Phase 6:** 20-50 fixtures covering the decisive archetype-pair combinations + edge cases (scorelines, cards, substitutions, signature awakenings, memory callbacks).
- **`archive` scope** — fixtures retained for historical record but not run in regular CI (e.g., fixtures from superseded content-pack versions). Never deleted.

## Authoring a corpus entry (operator runbook)

1. Decide the scenario (opening-day fixture, cup-final, 6-pointer, etc.) and a stable 16-hex-digit seed constant.
2. Run `scripts/fw replay <seed> --generate-fixture` to produce the initial `<seed>.json` with filled hashes.
3. Hand-edit the `description` + `notes` fields to document WHY this fixture exists.
4. Commit the fixture. The first CI run after the commit will verify `fw replay <seed> --compare-corpus` matches.
5. Any future drift means a real regression OR an intentional change — explicit `--regenerate-corpus` PR with reviewer-approved diff is the only path to update the `expected` block.

## MVP boundary

At Phase 3 Week 2: one fixture — `0xdeadbeefdeadbeef.json` — authored from the Month-3 slice opening-day scenario. Tier-A smoke runs it on every push. Other fixtures stub with `verification_scope: archive` until Phase 6 corpus expansion.

At Phase 6: 20-50 fixtures; Tier-C local regen; Tier-D full matrix.

At Phase 8 EA: corpus is a release-gate artifact. Shipped fixture hashes accompany the RC tag.

## Deferred

- Automatic fixture generation from interesting match events (e.g., "harness flagged an unusual scoreline, auto-author a fixture") — Phase 9 if balance-harness feedback demands it.
- Perceptual / visual-diff corpus entries — deferred indefinitely. Pass-activation-log diff per ADR-0002 is sufficient for MVP.
- Player-specific signature-awakening fixtures (one per signature) — Phase 6; bundled with full-corpus expansion.

## Cross-refs

- ADR-0001 (ShotTypeSO) — shot-selection determinism contract feeds pass-activation-log field.
- ADR-0002 (Viewer rendering pipeline) — pass-activation log replaces pixel-compare; this spec provides the hash storage.
- ADR-0003 (Production pipeline) — Tier A / C / D CI contracts for corpus operations.
- `design/match-engine.md` §Prototype gate — Q32.32 canonical state hash test uses this corpus shape.
- `design/event-sourced-memory.md` — MemoryEvent schema defines `key_event_hashes` input (landing in ADR-0004).
- `scripts/fw` — `replay` subcommand (stubbed Phase 1; implemented Phase 3).

## Open questions (resolve before Phase 3 Week 2 corpus authoring)

1. **Exact sim-state serialization order** — needs `MatchSim.Tests/SerializationContract.cs` authored alongside the first corpus fixture. Non-blocking at this spec level; blocker at Phase 3 Week 1.
2. **Pass-activation log field shape** — deferred until ADR-0002 lands in Unity and the ShotSelector is authored. Current spec reserves the hash field; precise field enumeration lands with the viewer implementation.
3. **Should the Tier-A smoke seed be one fixture or a small rotation (e.g., round-robin across 3 fixtures per week)?** — one is simpler; small rotation catches regressions in fixtures less-frequently-exercised. Recommend ONE at Phase 3, expand to rotation at Phase 6 if the budget allows.

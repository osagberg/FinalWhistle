---
description: Banned-terms lint catalog + approved football-native phrasing. The discipline that stops procedurally generated content from drifting into fantasy-RPG vocabulary.
last_verified: 2026-05-13
status: T0 stub. Categories A.1-A.5 + B + C ported from FW v1 docs/design/ui-vocabulary.md per MIGRATION_AUDIT §2.2 carry-forward catalog. The lint script (scripts/lint-banned-terms.py) reads from this doc as its source of truth.
---

# UI Vocabulary — banned-terms lint catalog

## Purpose

The procedural-fantasy USP (DESIGN_DOC §3 Pillar 1) creates a structural
risk: an LLM-baked corpus can drift toward fantasy-RPG vocabulary
<!-- ui-lint:ignore-start -->
("Awakened", "The Hush", "Kismet", "DNA Score")
<!-- ui-lint:ignore-end -->
which would make the game read like a JRPG over football rather than a
football management sim with procedural-fantasy worldbuilding.

This catalog is the structural floor. Every player-facing string —
commentary, headlines, scout reports, manager quotes, fan reactions, UI
labels, menu text, tutorial copy — passes through `scripts/lint-banned-terms.py`.
Hits block the bake (for content) or block the commit (for UI source).

Anchor: **the anime/fantasy layer is visual + structural, never lexical.**
A commentator must be able to say every shipped string without sounding
like they lost a bet.

---

## Lint integration

The lint is a single Python script that reads this doc and enforces the
catalog. Three invocation surfaces:

1. **Pre-bake** — `fw-content-baker` pipes every prompt template through the
   lint before calling the API.
2. **Post-bake, pre-commit** — every generated fragment is linted before
   the baker writes RON. Category-A hit rejects the fragment.
3. **CI on the workspace** — `scripts/fw banned-terms` runs as part of
   `scripts/fw verify` on every commit.

Source of truth: this doc. When you add or remove a term here, update the
regex catalog in `scripts/lint-banned-terms.py` in the same commit. CI
spot-greps doc and lint to verify they stay in sync.

---

## Category A — hard ban, no inline exemption

Sentinel-comment region exemption is the only path
(`<!-- ui-lint:ignore-start reason="..." -->` ... `<!-- ui-lint:ignore-end -->`),
used in this doc itself to wrap the banned-term tables.

<!-- ui-lint:ignore-start reason="banned-term catalog (lint source-of-truth doc)" -->

### A.1 — mystical / RPG / fantasy capitalized state nouns

| Banned | Internal alternative | Player-facing replacement |
|---|---|---|
| The Hush | `signature_readiness` float | "He's locked in." / "The stadium's gone quiet." |
| Weather (as team state) | `team_cohesion` float | "Form: rising" / "They're starting to click" |
| Calling (as player identity) | `role_family` | "A natural winger" / "Plays like a #10" |
| Canon / Shelves / Reading Lists (as pyramid tiers) | `tier` | "Premier Division" / "Championship" / "Tier 3" |
| The Seven (as rival-manager system) | — | (system deferred; no UI noun) |
| Kismet / Soul / Flow (as gene flags) | internal `narrative_triggers` | "Something clicked today." / "A late bloomer, it seems." |
| The Author (as manager identity framing) | — | "manager" / "head coach" / "gaffer" |
| The Ledger (as UI noun) | internal `MemoryEvent` ledger | "Club history" / "Career" / lowercase "the archive" |

### A.2 — system / progression / menu-game vocabulary

| Banned | Internal alternative | Player-facing replacement |
|---|---|---|
| Signature unlocked | internal `SignatureAwakened` event | "He's found something." / "He cuts inside again — and this time he goes through." |
| Awakened (capitalized noun/verb) | internal `SignatureAwakened` event | "Something clicked." / "That's new." (lowercase "awakens" is Category B) |
| XP gained / Level up / Skill point | — | no progression mechanic surfaces; player development is narrative |
| +5 finishing (stat-delta callouts) | internal `sim_bias` deltas | "He's striking the ball cleaner." / scout prose |
| Perk / Trait (as stat-label) | internal `trait` field | phenotype label from `design/player-generation.md` catalog (carry-forward) |

### A.3 — genetics / bloodline vocabulary

| Banned | Internal alternative | Player-facing replacement |
|---|---|---|
| Genes / Genetics / Chromosomes | internal `gene_model` fields | phenotype labels + scout prose |
| Bloodline (as mechanic) | internal `tactical_dna_fragments` | Coaching-lineage surface deferred post-MVP |
| DNA (as player-facing stat) | internal `identity_packet` | phenotype labels + scout prose |

### A.4 — stigmatizing / systemic phenotype framings

| Banned | Canonical replacement |
|---|---|
| Fragile Under Scrutiny | **Struggles Under Scrutiny** |
| Fragile When Tested | **Struggles Under Scrutiny** |
| Plateau Risk | (removed from enum; surface via scout prose + projected-range narrowing) |
| Injury-Prone | (not a label; injury history surfaces as explicit event record) |
| Powerful Striker (as phenotype) | **Powerful Ball Striker** (avoids confusion with striker-as-position) |

### A.5 — real-world place-name analogues

The FW v2 world is **fully procedural-fantasy** per DESIGN_DOC §2 rule 1.
Real-world place names in any shipped content pack or user-facing string
are categorical violations.

Catalog (port from FW v1; expanded at T2-3 once worldbuilding spike lands):

`Manchester`, `Liverpool`, `Leeds`, `London`, `Cardiff`, `Bristol`,
`Brighton`, `Southampton`, `Newcastle`, `Edinburgh`, `Norwich`, `Hull`,
`Birmingham`, `Nottingham`, `Jersey`, `Isle of Man`. Plus regional + global
expansions added at the worldbuilding-spike close.

Internal compiler-analogue strings (used by the baker's prompt templates
to seed regional naming priors WITHOUT leaking into shipped content) live
at `dev-config/region-analogues.json`, gitignored from runtime builds.

<!-- ui-lint:ignore-end -->

---

## Category B — soft ban, inline exemption allowed with audit

Avoid unless the specific surface genuinely needs them. Exemption mechanism:

<!-- ui-lint:ignore-start -->
```rust
// ui-lint:allow term="weapon" reason="cup-final commentary, deliberate" reviewer="osagberg"
commentary.push("He'll need his best weapon in this final.");
```
<!-- ui-lint:ignore-end -->

Rules:
- `term=` must match a Category-B banned term exactly.
- `reason=` must be non-empty and specific.
- `reviewer=` must be a handle.
- All three attributes required; missing any is a lint fail.
- CI emits an exemption report (`scripts/lint-banned-terms.py --report`).
- Exemptions are reviewed before **EA content lock** and before **every
  release candidate** — not on a fixed calendar.

<!-- ui-lint:ignore-start reason="Category-B catalog" -->

Category-B terms (port from FW v1):

- `awakens`, `awakened` (lowercase verb of gene unlock) — prefer "clicked", "found", "broke through"
- `Savant`, `Genius` (as stat-label) — use phenotype labels
- `weapon`, `weaponize` (as ability framing) — use "signature", "technique"
- `Egoist`, `The Ego` — use "manager", "gaffer", "boss"
- `Realm`, `Domain`, `Kingdom` — no royal/fantasy territory framing
- `power-level` — use football-native stakes language
- `Forge`, `Forged` (as generator verb) — use "compiled", "generated", "built"

<!-- ui-lint:ignore-end -->

---

## Category C — over-quoted FM-specific vocabulary (context-use only)

Allowed but only in their proper football context. The lint does not flag
these; they appear here for translator briefs + reviewer awareness.

- `potential`, `ability` — OK as scouting vocabulary, but tempered with
  "projected range" + phenotype labels
- `morale`, `form` — OK, standard football-English
- `condition`, `sharpness` — OK
- `legend` — OK sparingly; never auto-assigned

---

## Approved football-native vocabulary (sample — not exhaustive)

### Match-state language

- "He's locked in." / "Reads it early." / "Something's clicked today."
- "The stadium's gone quiet." / "The home crowd's turned on them."
- "Tempo's shifted." / "The side's on top." / "They're chasing shadows."
- "He arrives late in the box." / "Looks for the early ball."
- "Body's square." / "Leaves his marker for dead."

### Team-state language

- Structured labels: "Form: rising / faltering / holding" / "Confidence: shaky / rising / serene" / "Tempo: controlled / bite-and-kick / open"
- Commentary: "the side's clicking", "they've got a rhythm", "they can't string two passes"

### Player-identity language

**Phenotype labels:** authoritative catalog ports from FW v1
`design/player-generation.md` at T2-4. 46 labels across Physical / Mental /
Technical / Development / Role-specific. Label IDs are content-pack-qualified.

**Signature display names:** authoritative catalog ports from FW v1
`design/signatures.md` at T3-3. 24 signatures, football-copy-only names.

---

## Cross-refs

- `scripts/lint-banned-terms.py` — the lint script (source-of-truth ⇄ lint sync enforced in CI).
- `docs/CONTENT_PIPELINE.md` §5 — how the baker integrates the lint.
- `crates/fw-content-baker/src/validators.rs` — Rust-side wrappers + cliché detector layered on top.
- `CLAUDE.md` §7 — code-style rules including banned-term enforcement.

---

*Authored 2026-05-13 (T0 stub; expands as worldbuilding + player-generation + signatures docs land at T2-T3).*

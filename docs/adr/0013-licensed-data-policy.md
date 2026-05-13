# ADR-0013 — Licensed-data policy

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** Claude (Codex full-project audit Lane B + Lane H "Pillar 1 partial" driver) + Codex (pending pre-T2-3 audit)

---

## Context

**Pillar 1 — Procedural fantasy world** ("no real licensed data, ever"; DESIGN_DOC.md §3 + CLAUDE.md §1) is the most legally-sensitive of the five pillars. v1 had a `licensed-data validator` in the bake-time content pipeline that caught real-player-surname overlaps + real-place-name analogues. v2's `fw-content-baker::validators::*` modules are stubs returning `Ok(())` (per Codex audit P1 + Lane H Pillar-1 finding). The banned-terms lint (`scripts/lint-banned-terms.py`) catches state-noun categories but does NOT catch licensed-data overlaps.

T2-3 lands the real content baker + FW-VAL gauntlet (per Tranche 6 of the audit remediation). The licensed-data policy must be settled BEFORE T2-3 implements validation, otherwise the validator gets authored against a blank spec.

## Decision

### Scope of "licensed data"

For Final Whistle's purposes, "licensed data" means any name, identifier, or distinguishing identifier that could plausibly identify a real person, club, competition, kit, stadium, or trademark. Specifically:

1. **Real players' full names** — `John Smith` (common; allowed) vs `Erling Haaland` (specific; banned). Threshold: the surname-firstname combination must not match any player in the licensed-data corpus (Tranche 4 deliverable; sources: FIFA roster scrapes, FBref database, Wikipedia football-player categories).
2. **Real clubs** — `Manchester United`, `Real Madrid`, `Barcelona` (banned). Also banned: clear analogues that retain the recognizable identity (e.g. `Manchester Red`, `Royal Madrid`).
3. **Real competitions** — `Premier League`, `Champions League`, `La Liga`, `World Cup` (banned). Generic terms (`Cup`, `League`, `Division`) are fine.
4. **Real stadia** — `Old Trafford`, `Camp Nou`, `Bernabéu` (banned). Generic-coded analogues (`The Manchester Stadium`) are also banned; mythological / fictional names are fine.
5. **Trademarked terminology** — `FIFA`, `UEFA`, `MLS`, `EPL` (banned as standalones). Acronyms that resolve to generic words (`FA` = `Football Association`) are case-by-case; default to banned.
6. **Player names that match minor-league / non-pro players** — surnames from a wider licensed-data corpus including non-pro registers. The corpus is procedural fiction, not a thin-disguise of any real human.

### What's allowed

1. **Procedurally-generated names from culture-prior banks** — `Erik Halvarsson`, `James Tabor`, `Aelar Silverleaf`. These are the content packs' Markov-chain + cultural-prior output (per CONTENT_PIPELINE.md). The licensed-data validator checks these against the corpus + rejects any that collide.
2. **Common first names** in any culture — `John`, `Maria`, `Yuki`. These get filtered only when combined with a surname that produces a real-person hit.
3. **Generic football terminology** — `manager`, `coach`, `striker`, `defender`, `forward`, `winger`, `keeper`. These are the language of the sport.
4. **Fantasy / mythological references in fantasy cultures** — `Aelinn`, `Faelar`, `Wraithwood`. The whole point of `culture.fantasy-elvish.ron` is to demonstrate the procedural-fantasy pillar.

### Enforcement layers

Three layers, in order of expense (cheapest first):

1. **Banned-terms lint** (`scripts/lint-banned-terms.py`) — runs in CI. Catches the obvious cases: `"Premier League"`, `"FIFA"`, `"Manchester United"`. Catalogue grows over time. This is the cheap pass; failures here block CI.
2. **FW-VAL licensed-data validator** (`fw-content-baker::validators::licensed_data` — Tranche 6 work) — runs at content-bake time. Against a corpus of ~50k real-player surnames + ~5k real-club identifiers + competition + stadium lists. Cosine-similarity above 0.85 between a baked name + a corpus entry triggers a rejection. Authored before T2-3 content baking begins; sources: openly-licensed datasets only (Wikipedia categories, FBref's CC-licensed historical lists). Corpus refresh cadence: every major game release.
3. **Manual review** at content-bake commit time — the bake-time LLM occasionally produces names that pass cosine-similarity but feel close to real players. Author reviews diff before commit. This is the human-in-the-loop fallback for the long tail.

### Mod-content policy

Mods may ship custom names that violate the validator's strictness — but mods carry their own legal liability. Saves stamp the `mod_load_fingerprint` per ADR-0010; if a save references mod content that the runtime can't load (mod uninstalled), the affected `UnknownEventClass` payloads surface as opaque text. Core (fwh.core:*) content NEVER violates the policy.

### Per-save licensed-data audit

`fw-content-baker` emits an audit report per bake: `content/baked/manifest.ron` includes a `licensed_data_check: { corpus_version: u32, collisions_rejected: u32, names_audited: u32 }` block. The audit report is committed alongside the baked corpus.

If a real-player collision IS found post-ship (e.g. a fan surfaces the issue in a forum), the fix flow:
1. Author a single-character edit to the offending name in the next baked-corpus commit (e.g. `Erik Halvarsson` → `Erik Halverson`).
2. Save migrations preserve historical references via the `mod_load_fingerprint` mechanism — old saves continue to mention the old name (informational), but new careers generate the corrected name.

### Trademarked competition formats

The game's competition structures (league pyramids, cup formats, group stages) are GAMEPLAY MECHANICS — these are not trademark-protected by football's governing bodies. We can ship a 20-team top division with relegation; we cannot call it `Premier League`. The naming layer separates from the mechanic layer.

## Consequences

**Positive:**
- Pillar 1 has a defensible policy, not just a slogan.
- The three-layer enforcement (lint + validator + human review) catches the realistic threat space.
- Per-bake audit report makes the validator's work auditable post-ship.
- Mod liability is explicitly scoped to mod authors, not Final Whistle.

**Negative:**
- Building the 50k-name corpus is real work. Sources are openly-licensed but the data wrangling is non-trivial. Tracked as a Tranche 4 deliverable + a T2-3 dependency.
- Cosine-similarity threshold (0.85) is a tuning parameter — false positives ("Erik Hall" rejected as too close to "Erik Hall" the real player) require either lowering the threshold or per-name overrides. Tuning loop expected in T2-3 + T2-4.
- The validator is a real CI dependency. If the corpus refresh breaks, CI breaks. Mitigation: corpus changes go through a separate PR + green CI before merging into the bake pipeline.

**Neutral:**
- The licensed-data corpus is a build-time dep, not a runtime dep. The shipped game contains no real-player names; the corpus stays in the dev environment.
- Steam / Apple distribution platforms care about trademarked terms (FIFA / UEFA). The policy is sufficient for their content policies as of 2026.

**Rollback path:**
- If the validator catches too many false positives, raise the threshold to 0.92 + add a per-name allowlist. Tuning, not architectural.
- If a real-player collision ships, the audit-report fix flow (above) is the rollback path. No save-format change required.

## Alternatives considered

- **Allow-list rather than block-list.** Rejected — the space of "real things" is far larger than "fantasy things"; an allow-list is the wrong direction.
- **No licensed-data validator; rely on the LLM to "not generate real names".** Rejected — empirical: LLMs trained on football-rich corpora regularly produce real-player surname overlaps. The validator is necessary.
- **Sound-alike matching instead of cosine-similarity.** Sound-alike (Soundex / Metaphone) catches a different threat (`Hawland` → `Haaland`); we want BOTH. The validator runs cosine + Soundex + Levenshtein per Tranche-6 spec; the 0.85 cosine threshold is the primary gate, sound-alikes are secondary.
- **Crowd-sourced bug reports as the only gate.** Rejected — the policy needs to be defensible BEFORE EA ships, not reactive after.

## References

- DESIGN_DOC.md §3 Pillar 1 (the design promise)
- CLAUDE.md §1 (the project-level commitment)
- `Content/RULES.md` §5 (banned-terms lint catalogue location)
- `scripts/lint-banned-terms.py` (the cheap-pass implementation)
- `docs/specs/content-pack-validation-contract.md` (FW-VAL spec — Tranche 6 deliverable)
- `crates/fw-content-baker/src/validators.rs` (the stub; real impl at T2-3)
- v1: bake-time licensed-data validator (the carry-forward pattern; not directly portable code but the spec carries forward)
- Codex full-project audit Lane B "missing ADRs" + Lane H "Pillar 1 partial"

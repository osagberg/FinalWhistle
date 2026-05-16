# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 CLOSED 2026-05-16 at `v0.1.0-first-match`; T1-19 + T1-20 post-T1-close follow-ups landed.** T2 ready to start. Codex Tier-3 ACCEPT + post-T1-close ultimate-review ACCEPT both in hand. T1-20 (`fw-content-baker validate→validate-structural` rename + `content/baked/` lint coverage + `signature_candidate` dangling-ref check + sentinel-block escape close) closed in this commit — second post-T1-close follow-up landed. Cross-language change (Rust + Python + RON fixtures) with mandatory self-review triple firing on fw-content; 1 silent-failure P1 caught in-place (sentinel-scope substring-match security gap re-opened by nested paths; fixed with relative-path prefix match + 3 regression tests). 10 fixture-based tests all green; canonical hashes UNCHANGED.

## Active task

(none — T1-20 closed at this commit; `scripts/fw verify` green; canonical hashes UNCHANGED on both pins. Next `/next` picks from T1-21 / T1-22 (TODO follow-ups) or T2-1 (full BT runner with 20-30 manager archetypes). Per declared order T1-21 is next.)

## Phase pointer

- **Just landed:** **T1-20** — `fw-content-baker validate` → `validate-structural` rename (Codex workflow improvement #4 honesty-naming); `content/baked/` no longer excluded from banned-terms lint; `ContentStore::load_sources` now rejects dangling `PlayerTemplate.signature_candidates[i].signature_id` via existing `DanglingReference` variant; `ui-lint:ignore-start/end` sentinel blocks now scope-restricted to `docs/` + `crates/` + `scripts/` + project-root `.md` only (closes a security escape where attacker could hide Category-A terms inside sentinel-bracketed JSON/RON comments). 1 Rust integration test + 9 Python unittest tests. 9 files changed; canonical hashes UNCHANGED.
- **Next:** **T1-21** per MASTER_PLAN declared order — `fw-core::Tick` arithmetic policy alignment to Q32's panic-on-overflow + `Sim/RULES.md` §11 amendment. Also eligible: **T1-22** (hash-pin registry script + env-driven determinism rerun counts — procedural cleanup) OR **T2-1** (full BT runner with 20-30 manager archetypes + xG/personality coefficient calibration — main T2 row). `/next` will pick T1-21 first per declared order (skip-DEFERRED rule walks past T1-17). **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17 (friction-test rewrite, test-quality only); T4-9 (Stretch 2D viewer).

## Blockers

None.

## Last green verify

2026-05-16 (T1-20 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace --release including new dangling-signature-candidate integration test + pnpm test 56 frontend + banned-terms (new `content/baked/` coverage active + Python test suite added) + canonical-hash regression on both pins UNCHANGED + content-pack `validate-structural` + cargo audit + cargo deny check).

## Last canonical hash

`blake3:fcccb840b5868a4ed55c019c353a1d5496259073e2d88bf7abd97d9bdca7a751` (60-tick smoke seed; UNCHANGED from T1-16 rebaseline).

**Second corpus pin:** `blake3:9353bd257d4da92092407355e3c2b32cc6e91abc81664d0015336ebe812947eb` (600-tick extended seed `0xfeedbeefcafefade`; UNCHANGED from T1-16 rebaseline).

# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 CLOSED 2026-05-16 at `v0.1.0-first-match`; all 4 post-T1-close ultimate-review follow-ups + both post-Codex-followup-review rows landed (T1-19..T1-24).** T2 ready to start. T1-24 closed Codex Finding #2 (fw-hash-pins update_mode atomicity) — the last open Codex finding. Self-review caught a recursive silent-failure-of-a-test in the atomicity regression test itself (hardcoded hash that would have silently rotted at next rebaseline) — fixed in-place via registry-driven hash lookup. Canonical hashes UNCHANGED on both pins. All Codex review findings closed; T2-1 unblocked.

## Active task

(none — T1-24 closed at this commit; `scripts/fw verify` exit 0; canonical hashes UNCHANGED. **All post-T1-close + post-Codex-followup-review rows DONE.** Next `/next` picks **T2-1** (full BT runner with 20-30 manager archetypes + xG/personality coefficient calibration) — the main T2 row.)

## Phase pointer

- **Just landed:** **T1-24** — `scripts/fw-hash-pins.py::update_mode` refactored to preflight-then-write for genuine atomicity. `update_pin` → `preflight_pin` returns `(changed, is_failure, msg, Optional[(Path, str)])`; Phase 1 accumulates `prepared_writes` in memory; Phase 2 writes only if zero failures. New `scripts/test-fw-hash-pins.py` (~290 LoC; 3 tests) verifies the property via SHA-256 byte-identity assertions on sibling files after a deliberately-broken-preflight scenario. Self-review caught 3 P2s all fixed in-place: cross-platform `newline=""` discipline (Windows CI line-ending hazard); registry-driven hash in no-op test (was hardcoded → would have silently rotted at next rebaseline); explicit pre-mutation existence checks + stderr/stdout substring assertions for diagnostic clarity. Wired into Justfile + ci.yml (`hash-pins atomicity test` step on all 3 OSes). `docs/specs/determinism-gate.md §9` updated.
- **Next:** **T2-1** — full BT runner with all 20-30 manager archetypes (port YAML from `MatchSim/Content/archetypes/*.yaml`) + xG/personality coefficient re-fit per `docs/design/xg-coefficients.md` + `docs/design/personality-bias-weights.md` calibration cadence. Codex Lane I flagged "secretly huge" — may need split into T2-1a/b/c by archetype-pair if implementation reveals 20 archetypes is too broad. `gameplay-programmer` subagent rotation per CLAUDE.md §5. T2-1 now inherits a Tick-typed-panic-on-overflow cooldown substrate per T1-23 + a genuinely-atomic hash-pin registry per T1-24. **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17 (friction-test rewrite, test-quality only); T4-9 (Stretch 2D viewer).

## Blockers

None.

## Last green verify

2026-05-16 (T1-24 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace --release + pnpm test 56 frontend + banned-terms + canonical-hash regression on both pins UNCHANGED + new hash-pins atomicity test + content-pack validate-structural + cargo audit + cargo deny).

## Last canonical hash

`blake3:fcccb840b5868a4ed55c019c353a1d5496259073e2d88bf7abd97d9bdca7a751` (60-tick smoke seed; UNCHANGED from T1-16 rebaseline).

**Second corpus pin:** `blake3:9353bd257d4da92092407355e3c2b32cc6e91abc81664d0015336ebe812947eb` (600-tick extended seed `0xfeedbeefcafefade`; UNCHANGED from T1-16 rebaseline).

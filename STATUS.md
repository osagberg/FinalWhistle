# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 CLOSED 2026-05-16 at `v0.1.0-first-match`; ALL 4 post-T1-close ultimate-review follow-ups landed (T1-19 + T1-20 + T1-21 + T1-22).** T2 ready to start. Codex Tier-3 ACCEPT + post-T1-close ultimate-review ACCEPT both in hand. T1-22 (hash-pin registry script `scripts/fw hash-pins` + `FW_DETERMINISM_*_RUNS` env vars) closed in this commit — final post-T1-close cleanup. Cross-tool self-review convergence pattern surfaced 2 distinct silent-failure bug classes across 2 consecutive tasks (T1-21 SAFETY-comment factual error; T1-22 update_pin failure-vs-no-op conflation) — the playbook reliably catches real bugs at single-task scope. Canonical hashes UNCHANGED on both pins.

## Active task

(none — T1-22 closed at this commit; `scripts/fw verify` exit 0; canonical hashes UNCHANGED on both pins. Next `/next` picks T2-1 (full BT runner with 20-30 manager archetypes + xG/personality coefficient calibration) — the main T2 row.)

## Phase pointer

- **Just landed:** **T1-22** — `scripts/fw-hash-pins.py` (~330 LoC) lists + atomically updates the 5-location pin registry across 3 syntactic forms (RON `expected_hash`, Rust `hex!()` macro, Rust raw byte array); `scripts/fw hash-pins` wraps it. `canonical_hash.rs` `runs_for_test(env_var, default)` helper parameterizes the 100×/10× determinism rerun tests via `FW_DETERMINISM_SMOKE_RUNS` + `FW_DETERMINISM_EXTENDED_RUNS`. `docs/specs/determinism-gate.md §9` rewritten to reference the script + the 5-location table + env vars. Self-review triple caught a P1 (update_pin silent-failure: real failures were collapsed into no-op return); fixed in-place by switching to `tuple[bool, bool, str]` tri-state return + exit-1 on any real failure. Regression test verified via simulated regex-drift scenario.
- **Next:** **T2-1** per declared MASTER_PLAN order — full BT runner with all 20-30 manager archetypes (port YAML from `MatchSim/Content/archetypes/*.yaml` per the row) + xG / personality coefficient re-fit per `docs/design/xg-coefficients.md` + `docs/design/personality-bias-weights.md` calibration cadence. Codex audit Lane I flagged the original as "secretly huge" — may need split into T2-1a/b/c by archetype-pair if implementation reveals 20 archetypes is too broad for one row. `gameplay-programmer` subagent rotation per CLAUDE.md §5. **Deferred follow-ups (status `DEFERRED` — `/next` skips)**: T1-17 (friction-test rewrite, test-quality only); T4-9 (Stretch 2D viewer).

## Blockers

None.

## Last green verify

2026-05-16 (T1-22 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace --release including env-var-parameterized rerun-count tests + pnpm test 56 frontend + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + cargo audit + cargo deny check). Env var override verified working (`FW_DETERMINISM_SMOKE_RUNS=5 cargo test ...` runs 5 iterations; `=0` panics fail-loud at config time).

## Last canonical hash

`blake3:fcccb840b5868a4ed55c019c353a1d5496259073e2d88bf7abd97d9bdca7a751` (60-tick smoke seed; UNCHANGED from T1-16 rebaseline).

**Second corpus pin:** `blake3:9353bd257d4da92092407355e3c2b32cc6e91abc81664d0015336ebe812947eb` (600-tick extended seed `0xfeedbeefcafefade`; UNCHANGED from T1-16 rebaseline).

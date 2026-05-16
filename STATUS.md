# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** T1-12 shipped: content validation hardening across all three Codex 2026-05-16 audit P1 prongs. `ContentStore::load_sources` now fails-loud with `ContentLoadError::DuplicateId { kind, id, path_first, path_dupe }` on conflicting IDs (was silently overwriting); bake-time validators (`check_banned_terms` / `check_licensed_data` / `check_cliche`) now return `ValidationError::NotImplemented { validator, defer_to }` with actionable deferral targets instead of `Ok(())` stubs (so when T2-3 wires them through the bake pipeline the gap fails immediately); `RoleId` / `SignatureId` / `SignatureCandidate` serde-derived `Deserialize` was bypassing their existing `try_new` validators — now use manual impls that call `try_new` post-parse + report via `serde::de::Error::custom`. Plus removed two `From<&str>`/`From<String>` panic-backdoors on `RoleId` that gave callers an infallible-looking surface bypassing validation. **3 of 4 audit-triage P1s now closed**: T1-3.5 (ball actions, P0) + T1-11 (signatures unreachable) + T1-5 (IPC split-brain + match_frames unbounded) + T1-12 (content validation). Remaining: T1-10 (LUT bake determinism risk) + T1-13 (frontend test gate + cargo audit). Next: T1-6 (frontend Match page consuming MatchResult).

## Active task

(none — T1-12 closed across 1 implementation pass + 1 main-thread fix-pass for 1 P1 + 2 P2 self-review findings; T1-6 starting next per MASTER_PLAN order)

## Phase pointer

- **Just closed:** **T1-12** — three-prong content-validation hardening in `fw-content` + `fw-content-baker`: (a) `DuplicateId` variant + `insert_unique` helper wired at all 5 `load_sources` insertion sites + 5 duplicate-id integration tests (one per loaded kind); (b) `ValidationError::NotImplemented` variant + 3 bake-time validators fail-loud with actionable defer_to strings ("T2-3" for banned_terms; "T3+" for licensed_data + cliche); (c) manual `Deserialize` impls for `RoleId` (direct String->try_new), `SignatureId` (Visitor with 3 wire forms), `SignatureCandidate` (private `RawSignatureCandidate` bridge + TryFrom) + 12 malformed-fixture serde tests. Fix-pass closed code-reviewer's P1 (added `tempfile` workspace dep + switched duplicate_id_test from manual pid-only tmpdir to `TempDir::new()` for auto-cleanup) + P2 (vacuous `path_first.exists() || !path_first.as_os_str().is_empty()` short-circuit → direct exists assertions) + type-design F P2-1 (removed `From<&str>`/`From<String>` panic-backdoors on RoleId). Second clean silent-failure-hunter verdict in a row.
- **Next:** **T1-6** per MASTER_PLAN order — Frontend Match page with Play button, text recap rendering (goals + minute markers), event-list view; reuses T1-2a board component via debug toggle. Consumes the T1-5 `MatchResult` shape directly. After T1-6: T1-10 (LUT bake — replace runtime f64 LUT generation with committed Q32 tables) per audit-triage order.
- **Recommended /next order** (updated post T1-12): **T1-6** → T1-10 (LUT bake determinism risk) → T1-13 (frontend Vitest + cargo audit) → T1-7 (procgen) → T1-8 (replay corpus) → T1-9 (behavioral assertions). T1-10 + T1-13 are the remaining 2 audit-triage P1s.

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-12 + fix-pass: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `782fcde6…8c0f` + banned-terms + determinism-audit + `fw-content-baker validate` (continues passing — bake-time validators not invoked by the validate subcommand).

## Last canonical hash

`blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f` (60-tick smoke seed; UNCHANGED since T1-3.5; T1-4a/T1-4b/T1-11/T1-5/T1-12 all kept it stable — T1-12 touches loader-side validation only; canonical match-state bytes unaffected).

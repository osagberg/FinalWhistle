# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** Codex Tier-2 audit fix-pass landed on top of T1-6: 2 P0 IPC shape mismatches Codex caught in the "vertical complete" claim are now actually closed. (1) `MatchResult.match_events` was shipping `Vec<fw_content::MatchEvent>` which serde-serialized externally-tagged as `{ "KickOff": {...} }` — incompatible with the frontend's flat `{ tick, minute, kind, description }` interface; replaced with new `MatchEventDto` projection (PascalCase `kind` field matching the TS closed `MatchEventKind` union AS-IS via `format!("{:?}", MatchEvent::discriminant())`). (2) `get_dummy_state` after T1-5 returned `MatchStateDto` but `Home.tsx` still expected the handshake shape; new `get_backend_handshake` command + `BackendHandshakeDto { app_version, message, backend_ready }` module + frontend rename of `getDummyState`→`getBackendHandshake` + `DummyState`→`BackendHandshake`. Plus Codex's recommendation #3: 3 new Rust integration tests pinning the exact JSON wire shape + 1 new Vitest test parsing the literal Rust→TS JSON shape (catches the regression class where mocks drift from reality). **The vertical-complete claim from T1-6 is now actually true** (was false at commit time per Codex). Audit-triage P1 closure status unchanged (4 of 5 closed). Next: T1-10 (LUT bake — will rebaseline canonical hash; Codex recommended closing this fix-pass first so the foundation locks before the math-primitive churn).

## Active task

(none — Codex Tier-2 post-T1-6 audit fix-pass landed; vertical actually works end-to-end now; T1-10 starting next per audit-triage order)

## Phase pointer

- **Just closed:** Codex Tier-2 audit fix-pass (post-T1-6) — `MatchEventDto` projection + `BackendHandshakeDto` + Codex's wire-shape test recommendation #3. 2 P0 closed; 4 new tests (3 Rust + 1 TS) pin the Rust→TS JSON contract.
- **Next:** **T1-10** per audit-triage order — `fw-core` SIGMOID_LUT + EXP_LUT bake-time replacement of runtime f64 generation. Currently built at process startup via `f64::exp()` + quantized to Q32; cross-OS hash passes today but libm/platform-dependent by design (Codex 2026-05-16 audit P1 "Runtime f64 LUT bake is a determinism risk"). T1-10 commits the tables as source / committed data + makes runtime utility math pure Q32. **WILL REBASELINE canonical hash** (math primitives shift bits). After T1-10: T1-13 (frontend test gate + cargo audit) is the 5th + final audit-triage P1.
- **Recommended /next order**: **T1-10** → T1-13 (frontend Vitest broader gate + cargo audit) → T1-7 (procgen) → T1-8 (replay corpus) → T1-9 (behavioral assertions). 4 of 5 audit-triage P1s closed; T1-10 + T1-13 are the remaining 2.

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post Codex Tier-2 fix-pass: fmt + clippy + `cargo test --workspace` (fw-tauri 26 unit + 9 integration including 3 new wire-shape tests) + release-mode canonical-hash regression on `782fcde6…8c0f` + banned-terms + determinism-audit + `fw-content-baker validate`. Frontend `pnpm typecheck` + `pnpm lint` + `pnpm test` (11/11) + `pnpm build` all clean.

## Last canonical hash

`blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f` (60-tick smoke seed; UNCHANGED since T1-3.5; T1-4a/T1-4b/T1-11/T1-12/T1-5/T1-6/Codex-fix-pass all kept it stable — IPC wire shape changes don't touch canonical state bytes).

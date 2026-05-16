# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** T1-10 shipped: `fw-core::math` runtime LazyLock LUT bake replaced with committed const Q32 tables. Previously the 257-entry SIGMOID_LUT + EXP_LUT were built at process startup via `LazyLock<[Q32; 257]>` using `f64::exp()` then quantized — libm/platform-dependent by design. Now committed `pub(crate) const SIGMOID_LUT_RAW: [i64; 257]` + `pub(crate) const EXP_LUT_RAW: [i64; 257]` in new `crates/fw-core/src/math_luts.rs` reconstructed at compile time via a `const fn q32_array_from_raw` helper. Drift becomes a code-review artifact instead of a silent runtime variance. New `#[ignore]`-gated `lut_drift_detection` test re-bakes via f64 + asserts equality with the committed const. New `#[ignore]`-gated `print_luts_oneshot` printer re-bakes from f64 + emits Rust source literals for paste-replace regeneration (Tier-2 fix-pass per silent-failure + code-reviewer: prior printer read the committed const back which would have emitted STALE values after a real drift event). **Canonical hash UNCHANGED at `782fcde6…8c0f`** — the committed const values match the prior LazyLock-baked values byte-for-byte. **4 of 5 audit-triage P1s now closed**. Next: T1-13 (5th + FINAL audit-triage P1 — frontend test gate + cargo audit + cargo deny).

## Active task

(none — T1-10 closed across 1 implementation pass + 1 main-thread fix-pass for 4 P2 self-review findings; T1-13 starting next + finishes the audit-triage P1 sweep)

## Phase pointer

- **Just closed:** **T1-10** — fw-core::math LazyLock-to-const LUT bake removal. 4 chunks: bake-printer (one-shot), commit math_luts.rs (514 raw-bits entries), wire + delete LazyLock from math.rs, drift-detection test. Plus 4-P2 fix-pass: printer re-bakes from f64 instead of echoing committed const (workflow integrity per silent-failure + code-reviewer); stale `pub(crate)` doc-comment fix in signature/dispatcher.rs:649; SIGMOID_LUT/EXP_LUT `pub(crate)` → module-private (tighter encapsulation); `#[doc(hidden)]` on `from_f64_clamped` (discourage rustdoc discovery while keeping callable for drift test + future T2-3 baker). Three clean silent-failure-hunter verdicts in a row (T1-11 + T1-12 + T1-10).
- **Next:** **T1-13** per audit-triage order — Frontend test scaffolding + CI gating. CLAUDE.md §9 says `scripts/fw verify` runs `pnpm test`; it doesn't. Justfile's `verify` recipe doesn't call it; ci.yml doesn't either. T1-6 lit up the Vitest substrate with 1 test file (10 + 1 tests post Codex Tier-2 fix-pass); T1-13 broadens to ≥3 test files (FrameSource + dev-board lifecycle + URL-param parsing) + wires `pnpm test` into `just verify` + CI matrix. Plus the Codex backlog `cargo audit` + `cargo deny` wiring (pulled forward in MASTER_PLAN but never wired) — adds as separate gate steps that don't block dev loop but DO block CI on new vulnerabilities.
- **Audit-triage P1 closure status post-T1-10**: 4 of 5 closed (T1-3.5 ball-actions P0 + T1-11 signatures unreachable + T1-5 IPC split-brain + T1-5 match_frames unbounded + T1-12 content validation + T1-10 LUT bake). **T1-13 is the 5th + final**.
- **Recommended /next order** post T1-13: T1-7 (procgen) → T1-8 (replay corpus) → T1-9 (behavioral assertions). All non-P1 polish remaining in T1.

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-10 + fix-pass: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `782fcde6…8c0f` + banned-terms + determinism-audit (math.rs file-level f64 exemption REMOVED + lut_drift_detection.rs + print_luts_oneshot.rs added to per-rule exemption with documented justification) + `fw-content-baker validate`.

## Last canonical hash

`blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f` (60-tick smoke seed; UNCHANGED since T1-3.5; T1-4a/T1-4b/T1-11/T1-12/T1-5/T1-6/Codex-fix-pass/T1-10 all kept it stable — T1-10's whole point was to swap the BAKE mechanism without touching the bit-for-bit LUT values).

# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** T1-4b (Tracery commentary template bank + renderer) shipped — closes ADR-0007 Layer 1 + Content/RULES.md §4. The match-engine inner loop is now narrated: real ball actions (T1-3.5) → MatchEvent emission (T1-4a) → deterministic Tracery prose (T1-4b). Next: T1-11 (signature wiring into tick_match) per audit-triage order.

## Active task

(none — T1-4b closed across 2-agent dispatch + main-thread fix-pass for 7 self-review findings)

## Phase pointer

- **Just closed:** **T1-4b Tracery commentary template bank + deterministic renderer.** `SeedLayer::Commentary = 0x18` in fw-core::seed (per the ADR-0009 amendment); new `fw-content::commentary` module wrapping `tracery 0.2.1` crate (user-authorized 3rd-party dep with audit caveats); ContentStore loader + 6 .tracery.json files × 5 variants = 30 total. lead-programmer shipped substrate (chunks 1-3); narrative-director shipped templates + read-aloud gate (chunks 4-5). Self-review fix-pass closed 1 P0 (unwrap_or_default silent-failure) + 6 P1 (sentinel doc drift, stale test doc, empty-grammar rejection, repr(u8), Commentary in canonical test, render_with_vars pre-filter).
- **Next:** **T1-11** — wire signatures into the real `tick_match` path. Currently `tick_match` passes `BTreeMap::new()` for sig_definitions; signatures fire only in custom test fixtures. T1-11 adds a real match-setup context that projects content definitions + per-player `signature_candidates` (loaded from PlayerTemplate via T1-3 carry-forward) into MatchState/dispatch_tick. The dev-board scrubber will then actually show signature firings + the commentary renderer (T1-4b) can narrate them.
- **Recommended /next order** (per audit triage): **T1-11** → T1-5 (Tauri play_match + IPC consolidation + match_frames cap) → T1-12 (content validation hardening) → T1-10 (LUT bake) → T1-13 (frontend Vitest + cargo audit) → T1-6 (frontend Match polish) → T1-7 (procgen) → T1-8 (replay corpus) → T1-9 (behavioral assertions).

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-4b + fix-pass: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `782fcde6…8c0f` + banned-terms + determinism-audit + `fw-content-baker validate`.

## Last canonical hash

`blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f` (60-tick smoke seed; pinned at T1-3.5 per ADR-0012 trigger #1 — UNCHANGED through T1-4b because templates are content data, not canonical state).

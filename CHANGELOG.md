# Changelog

Append-only human-readable ship log. One line per shipped MASTER_PLAN task. Phase-summary block on `/done`. Reverse-chronological within each phase section.

---

## Phase T0 — Scaffold

- 2026-05-13 — **T0-7** Pin BLAKE3 canonical hash on macOS-14 dev box — `PINNED_60_TICK = d6258107…` recorded; `#[ignore]` removed; `cargo test -p fw-replay` 4/4 green. Cross-OS matrix verification deferred to T0-7b via the phase-gate PR.
- 2026-05-13 — **Codex audit followup** (`9eb184e`) — Q32 operators panic-on-overflow; IDs are durable `u32` newtypes (slotmap dropped). 7 of 7 Q1–Q7 audit decisions resolved.
- 2026-05-13 — **Codex audit quick wins** (`7dc510d`) — STATUS.md created; hash-non-zero sanity test; DESIGN_DOC float fields → Q32 basis points; fw-content weights → integer bps; determinism-audit script (comment-aware); 9 of 16 Codex findings fixed.
- 2026-05-13 — **Blueprint reconciliation** (`26f1ba0`) — 51 files; 7 agents (down from 14), 6 commands (down from 35), 5 hooks (down from 14), 3 context scopes (down from 5), Rust-flavored path-scoped RULES, 8 design templates, 9 patterns.
- 2026-05-13 — **Rust pivot scaffold** (`81fdeff`) — Clean-slate rewrite from Unity+C# to Rust+Tauri+SolidJS. 109 scaffold files across 8 crates + frontend + Tauri shell. Pre-pivot state preserved at git tag `v0-pre-pivot-2026-05-13` + sibling `/Users/vibelogic/dev/football-archive/`.

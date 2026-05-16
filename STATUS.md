# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** T1-3.5 (ball mutation + possession + goal detection) shipped — closes Codex 2026-05-16 audit P0. Football now actually happens: ball moves on Shot, possession transfers on Pass, goals fire on goal-line crossing, tactic-FSM Goal hookup integrated. Next: T1-4b (Tracery commentary template bank).

## Active task

(none — T1-3.5 closed across implementation + main-thread fix-pass for 7 P0 + 6 P1 self-review findings)

## Phase pointer

- **Just closed:** **T1-3.5 ball mutation + possession + goal detection.** New `possession: Option<PlayerSlot>` + `last_touched_by: Option<PlayerSlot>` canonical fields; pitch geometry constants in fw-core (PITCH_LENGTH_M / PITCH_WIDTH_M with const-derived GOAL_LINE_X / SIDELINE_Y); attribute-modulated ball-speed scaling; ball.vel mutation in apply_intent for Shot/Pass/Dribble/GK; goal detection + score bump + kick-off taker derivation + TacticEvent::Goal hookup; OOB clamp with goal-fired-this-tick guard. Substrate bug fix: ball_physics.rs coordinate convention corrected (pos_z = altitude, not pos_y). Encoder VERSION 7→8; hash `02ab97d0…27e686` → `782fcde6…8c0f`.
- **Next:** **T1-4b** — Tracery commentary template bank + deterministic renderer in `fw-content::commentary`. Owned by `narrative-director` per CLAUDE.md §5 + ADR-0007 line 87. ≥3 variants per MatchEvent slot (≥18 templates total). `SeedLayer::Commentary = 0x18` prereq logged at `docs/DECISIONS.md` 2026-05-16. T1-4b can now narrate real football outcomes (T1-3.5 made Pass/Shot/Goal events describe actual ball trajectories, not player intentions).
- **Recommended /next order** (per audit triage): **T1-4b** → T1-11 (signature wiring into tick_match) → T1-5 (Tauri + IPC consolidation + match_frames cap) → T1-12 (content validation) → T1-10 (LUT bake) → T1-13 (frontend tests + cargo audit) → T1-6 → T1-7 → T1-8 → T1-9.

## Blockers

None. T1-4b's `SeedLayer::Commentary` ADR prereq is satisfied.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-3.5 + fix-pass: fmt + clippy + `cargo test --workspace` + release-mode canonical-hash regression on `782fcde6…8c0f` + banned-terms + determinism-audit + `fw-content-baker validate`.

## Last canonical hash

`blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f` (60-tick smoke seed; rebaselined at T1-3.5 per ADR-0012 trigger #1 — MatchState gained possession + last_touched_by; encoder VERSION 7→8 schema bump; ball.vel mutation + goal detection now active in tick_match; ball_physics coord convention corrected to pos_z = altitude). T1-4b will leave the hash UNCHANGED (Tracery templates are content data, not canonical state).

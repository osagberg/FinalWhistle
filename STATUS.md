# STATUS — Final Whistle

**Last updated**: 2026-05-16

## Phase

**T1 — First Match.** T1-7 shipped: first non-P1 row after the Codex audit-triage sweep completed. Substantive scope per user-confirmed ambiguity-gate decisions: real 2-gram char-level Markov player-name chain (not random-pull stub), `Culture.team_name_bank` schema extension (not hard-coded template), full `ManagerArchetype` with personality-trait Q32 fields (not minimal id+display+ref). 5 new fw-content modules wired in: `manager.rs` + `markov.rs` + `procgen.rs` + Culture+ContentStore extensions + 1 ManagerArchetype RON fixture + 27 new tests. `generate_team(content, ProcGenInputs)` deterministically produces `ProcGenTeam { team_name, manager, players: [PlayerName; 22] }` from a single seed. Main-thread fix-pass closed 3 P1 type-design findings (extending T1-12 hardening pattern to ManagerArchetype: `try_new` Q32 range validators + `RawManagerArchetype` bridge + new `ManagerArchetypeId` newtype + manual `Deserialize`) + 2 P2 + 1 P3 (new `ContentLoadError::DanglingReference` + cross-reference validator at load — closes prior doc/code mismatch). **6th clean silent-failure-hunter verdict in a row.** T1 phase status: 6 of 9 numbered rows now DONE; T1-8 (replay corpus) + T1-9 (behavioral assertions) remain before `/done`.

## Active task

(none — T1-7 closed across 1 implementation pass + 1 main-thread fix-pass for 3 P1 + 2 P2 + 1 P3 self-review findings)

## Phase pointer

- **Just closed:** **T1-7** — Procgen pipeline (Markov + ManagerArchetype + team_name_bank + generate_team). 5 chunks (~820 LoC initial) + fix-pass (~320 LoC for ManagerArchetype hardening + ManagerArchetypeId newtype + ProcGenInputs struct + #[non_exhaustive] + kind-string fix + DanglingReference cross-ref validator). 27 new tests.
- **Next:** **T1-8** per MASTER_PLAN order — Replay corpus fixture #1 (smoke seed, 600 ticks, two-archetype matchup, pinned canonical hash on CI matrix). Creates `crates/fw-replay/fixtures/0xfeedbeefcafefade.ron`. OR **T1-9** (behavioral assertions — proptest invariants for 4 positional + PlayerSeparation + 3 pair-seed knob-isolation + events_chronological per ADR-0007).
- **Recommended /next order**: **T1-8** → T1-9 → `/done`. T1-8 is mostly mechanical fixture bake + CI wire-up; T1-9 is substantive proptest authoring; both can ship in either order since they're independent.

## Blockers

None.

## Last green verify

2026-05-16 — `scripts/fw verify` clean post T1-7 + fix-pass: cargo fmt + clippy + cargo test --workspace (147 fw-content tests) + pnpm test (34 frontend tests) + canonical-hash regression on `782fcde6…8c0f` + banned-terms + determinism-audit + fw-content-baker validate + cargo audit + cargo deny check.

## Last canonical hash

`blake3:782fcde65ba8a0fc12bb90af1b61f77d8cd403103ab3671b0d5d6b03e75c8c0f` (60-tick smoke seed; UNCHANGED across the entire T1 phase since T1-3.5 — T1-4a/T1-4b/T1-11/T1-12/T1-5/T1-6/Codex-fix-pass/T1-10/T1-13/T1-7 all preserved it).

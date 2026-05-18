# STATUS — Final Whistle

**Last updated**: 2026-05-18

## Phase

**PHASE T2 COMPLETE 2026-05-18.** All 10 MVP rows DONE: T2-1a/b/c/d-infra + T2-1-codex-fix + T2-2 + T2-3 + T2-5 + T2-6 + T2-8 + T2-9. Run **`/done`** next to open the T2-10 phase-gate Codex review PR. Effectively-blocked rows: T2-4 (BLOCKED on `design/player-generation.md`) + T2-7 (transitively blocked on T2-4) + T2-1d2 (DEFERRED to end-of-T2 cadence) — all candidates for promotion via `/log-decision` post-review OR roll-forward to T3.

T2-9 just landed: `fw-save` bincode-based save format + V0→V1 migration discipline. **V1 schema LOCKED at this commit** (lives FOREVER). `#[repr(u32)]` + explicit discriminants make variant-tag drift compile-detectable; wire-byte regression tests pin V0 at `0x00` + V1 at `0x01`. `load_envelope` is the production entry point — auto-migrates V0 + propagates unknown discriminants. 11 fw-save tests (9 new) exercise the 4-test migration discipline (forward-migration / callback-preservation / forward-incompat-failure / round-trip-byte-identical). 2 P0 + 2 P1 self-review findings fixed in-place per fail-closed discipline (fw-save is canonical-state-adjacent): variant-tag stability; wire-byte regression; trailing-bytes silent acceptance → typed `SaveError::TrailingBytes`; forward-incompat assertion tightened.

**Canonical hashes UNCHANGED on both pins** — fw-save is not in the canonical-state pin path.

## Active task

(none — T2-9 closed at this commit; `scripts/fw verify` exit 0; **canonical hashes UNCHANGED on both pins**. **PHASE T2 COMPLETE — all 10 MVP rows DONE.** Next user action: run **`/done`** to verify the T2 acceptance gate + sync ledgers + print the `gh pr create` command for Codex Tier-3 phase-boundary review. 16 commits ahead of origin/main waiting to push.)

## Phase pointer

- **Just landed:** **T2-9** — `crates/fw-save/src/lib.rs` extended ~260 LoC with the V0/V1 envelope lock + migration discipline + 9 new tests + wire-byte regression pins.
- **Next user action:** Run **`/done`** to close Phase T2 (validates the acceptance gate + appends to CHANGELOG + rewrites STATUS for T3 + prints the `gh pr create` invocation for Codex Tier-3 phase-boundary review). After Codex acks the phase review, the user may either (a) promote the BLOCKED T2-4 + T2-7 by authoring `design/player-generation.md` via `/log-decision`, (b) promote the DEFERRED T2-1d2 (utility_shoot rewire + coefficient apply per the post-rewire calibrate sweep), OR (c) advance to Phase T3 (Memory + Narrative). **Deferred follow-ups (`/next` skips)**: T1-17, T1-25..T1-28, T2-1d2, T4-9. **Carry-forward known follow-ups from T2-9 self-review (P2/P3)**: introduce `ContentPackVersion(NonZeroU32)` newtype in `fw-core` (the proper crate home) + adopt in `SaveV1.content_pack_version`; `pub type SaveLatest = SaveV1;` alias so `load_envelope` signature stays stable across schema bumps; `#[must_use]` on `migrate_v0_to_v1`; `load_envelope` empty-bytes behavior doc. Plus T2-8 / T2-6 / T2-5 / T2-3 carry-forwards still standing (WindowState.label literal field i18n-hostile; MatchDay newtype; extract `lib/ipc-error.ts` to dedupe Match/League/Transfers trio; SeasonState API surface tightening; BakeManifest.output_path: PathBuf; workspace-hoist Cargo deps).

## Blockers

- **T2-4 (`PlayerBio` generation) BLOCKED** on missing `design/player-generation.md`. Resolve via `/log-decision` ADR authoring OR external design-doc authoring.
- **T2-7 (Squad page) transitively blocked** by T2-4.

## Last green verify

2026-05-18 (T2-9 close): `scripts/fw verify` exit 0 (cargo fmt + clippy + cargo test --workspace + pnpm test 87 frontend + pnpm typecheck + banned-terms + canonical-hash regression on both pins UNCHANGED + content-pack validate-structural + hash-pins atomicity test + cargo audit + cargo deny). 9 new fw-save tests added (total 11 fw-save tests green).

## Last canonical hash

`blake3:eaf842ac3d19651d38dc7ce45d0763cc62b4d571ce2c2a5d56f1ee3c6ddead46` (60-tick smoke seed; UNCHANGED from T2-1b rebaseline — fw-save is not in the canonical-state pin path; sim crates untouched).

**Second corpus pin:** `blake3:aa7efe9b2a567d5e87d12c7da6a4ea928271429729884f38819baed85c3be5ae` (600-tick extended seed; UNCHANGED from T2-1-codex-fix rebaseline).

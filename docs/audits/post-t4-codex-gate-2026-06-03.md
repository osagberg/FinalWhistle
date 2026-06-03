# Post-T4 Codex Gate - Determinism + Saves - 2026-06-03

Focused adversarial pass on the Final Whistle T4 determinism, canonical
rebaseline, save integrity, compaction, and memory-callback render surface.

Verdict: REVISE for the determinism + save surface.

The canonical rebaseline itself looks honest, and the narrow test commands below
are green. The revise call is for save/career-state integrity: a crafted or even
currently committed V4 fixture can load with an impossible breakthrough watermark
and silently suppress future breakthrough evaluation.

## Findings

### P1 - SaveV4 accepts and pins an impossible breakthrough watermark

`SaveV4.breakthrough_eval_watermark` is documented as a ledger-length watermark
(`crates/fw-save/src/lib.rs:335-344`), but the committed V4 fixture encodes
`V4_CAREER_WATERMARK = 7` (`crates/fw-save/tests/migration_fixtures_test.rs:158`)
after building a ledger with only two appended events
(`crates/fw-save/tests/migration_fixtures_test.rs:300-304`). The fixture tests
then assert both facts as correct: `ledger.len() == 2`
(`crates/fw-save/tests/migration_fixtures_test.rs:963`) and watermark `7`
(`crates/fw-save/tests/migration_fixtures_test.rs:965-966`,
`crates/fw-save/tests/migration_fixtures_test.rs:1034-1037`).

The production load path does not validate this invariant. `load_envelope`
returns V4 and only calls `restore_transient_state`
(`crates/fw-save/src/lib.rs:515-525`); `load_career_inner` then copies the
wire watermark directly into runtime state
(`crates/fw-tauri/src/commands.rs:2285`). `advance_season_inner` materializes
new breakthrough events with `career.ledger.iter().skip(watermark)`
(`crates/fw-tauri/src/commands.rs:1294-1296`).

Impact: a corrupt/crafted save, and the current frozen V4 fixture shape, can
load with `watermark > ledger.len()`. Future ledger events are skipped until the
ledger grows past the bogus watermark, so breakthrough meters can silently stop
accumulating for several seasons after load.

Recommended fix: reject or clamp invalid watermarks on load. Prefer rejecting:
`breakthrough_eval_watermark <= ledger.len()` should be a hard save invariant.
Update the V4 fixture to a valid watermark, likely `2`, and add a negative
fixture/test for `watermark > ledger.len()`.

### P1 - Deserialized ledgers are not validated before restore

`MemoryLedger::restore_transient_state` sets `next_id = events.len()` and marks
indexes dirty (`crates/fw-memory/src/ledger.rs:152-153`). It does not validate
that serialized `MemoryEvent.event_id` values are contiguous, strictly
ascending, or even unique. The read path later assumes ascending IDs:
`get_by_id` uses `binary_search_by_key` on `event_id`
(`crates/fw-memory/src/ledger.rs:282-284`), and `append` allocates from
`next_id` (`crates/fw-memory/src/ledger.rs:242`).

Impact: a crafted save can deserialize into a wrong-but-accepted ledger where
lookups miss, IDs collide after append, or reader ordering no longer matches the
identity contract. This is the same class as the fixed `from_events` issue, but
at the save boundary instead of the per-player view boundary.

Recommended fix: add a `MemoryLedger::validate_for_load` or make
`restore_transient_state` fail-returning. Enforce strictly ascending contiguous
IDs, `next_id = max_id + 1`, and `event_id == Vec index` if that is the intended
save invariant. Call it from `fw_save::load_envelope` before returning V4.

### P2 - The 5-seed score envelope admits low-scoring regressions

The T4-2.5j rebaseline note and pin are coherent: `PINNED_600_TICK` is
`12ce5ab7...9c464c1c` (`crates/fw-replay/tests/canonical_hash.rs:747-748`),
and the fixture has the same hash
(`crates/fw-replay/fixtures/0xfeedbeefcafefade.ron:146`). I did not find a
HashMap, unsorted collection, float, clock, or thread_rng leak in the target
signature/memory paths.

The behavior envelope is weaker than its comment says. The broad seed sweep
says "0 catches an engine that's regressed to no shots ever fire"
(`crates/fw-replay/tests/canonical_hash.rs:1022-1024`), but the assertion is
`(0..=7).contains(&total_goals)` (`crates/fw-replay/tests/canonical_hash.rs:1031`).
The pinned seed has a strict `[2,5]` check (`crates/fw-replay/tests/canonical_hash.rs:1008-1018`),
so an all-seed zero-goal regression is caught if it hits the pinned seed, but a
regression that collapses only the four sanity seeds to zero still passes.

Recommended fix: make the sanity envelope `[1,7]` or add an aggregate lower
bound across the five seeds.

### P2 - Compaction is deterministic, but decaying-event ordering can jump

`MemoryLedger::compact` nulls `tick` for every in-window non-compaction event
(`crates/fw-memory/src/ledger.rs:340-361`). `project_salience` treats `tick:
None` as "no decay" and returns emission salience
(`crates/fw-memory/src/readers/mod.rs:226-228`). That is platform-deterministic,
but for `Linear` or `Exponential` events it can move an old event from decayed
low salience back to full emission salience after compaction.

The compaction retention corpus does not exercise that ordering case because it
uses `DecayFunction::Never` throughout
(`crates/fw-memory/tests/compaction_retention_corpus.rs:24`,
`crates/fw-memory/tests/compaction_retention_corpus.rs:143`).

Recommended fix: add a small corpus/test with a decaying high-stakes old event
and a newer event, asserting the intended pre/post-compaction reader order. If
the no-decay fallback is the intended semantic, document that compaction
reclassifies decayed events as timeless anchors.

### P3 - Empty-slot callback rendering can still produce awkward prose

The T4-2.5k renderer fix is deterministic and prevents Tracery from rejecting
empty context slots: empty vars become `" "`, then output is whitespace-collapsed
(`crates/fw-content/src/memory_callback.rs:462-490`). It does not prevent
grammatical holes when a grammar variant explicitly uses a missing slot.

Example: career overview passes `player_name: String::new()` for club-subject
`TitleWon` events (`crates/fw-tauri/src/commands.rs:1553`), while two of the
three `title_won` variants reference `#player_name#`
(`content/sources/grammars/memory-callback.tracery.json:174-175`). Those can
render as "was there..." / "and was part of it" with no subject. This is not
platform nondeterminism and not a silent fallback, but it is still wrong output.

Recommended fix: either supply a resolved squad/player phrase for club-subject
callbacks or split club-subject grammar variants so they do not reference
`#player_name#`.

## Non-findings

- T4-2.5j canonical hash rebaseline looks authorized and mechanically honest:
  current code pin and RON fixture agree, the 60-tick pin remains separate, and
  the signature paths use deterministic `BTreeMap`/`Vec`/Q32/seeded ChaCha.
- `MemoryLedger::from_events` now preserves global event IDs, so the previous
  per-player breakthrough RNG renumbering issue is fixed in current HEAD.
- Compaction and memory callback rendering are deterministic across platforms in
  the code paths reviewed; the issues above are semantic/test-strength issues.

## Verification run

- `cargo test --release -p fw-replay --test canonical_hash -- --nocapture` - pass, 10 tests.
- `cargo test -p fw-save --test migration_fixtures_test -- --nocapture` - pass, 15 tests, 1 ignored.
- `cargo test -p fw-tauri --test save_career_test -- --nocapture` - pass, 2 tests.
- `cargo test -p fw-memory compaction -- --nocapture` - pass, including the 100-ledger corpus.
- `cargo test -p fw-content memory_callback -- --nocapture` - pass for the targeted memory callback tests.

## Remediation — T4-8-CR1 (2026-06-03, Claude-side)

Both P1 gate-blockers fixed (commit `T4-8-CR1`). The P2/P3 findings are deferred to
MASTER_PLAN row **QA-T4H** (land before T4.5's first `/next`).

- **P1-A (impossible watermark) — FIXED.** `load_envelope` now hard-rejects
  `breakthrough_eval_watermark > ledger.len()` via `SaveError::WatermarkBeyondLedger`
  (reject, not clamp). `== len` is valid (all events evaluated). The committed
  `v4_career_sample.fwsave` golden was regenerated to a valid `watermark = 2`
  (byte count unchanged: 2827→2827); `V4_CAREER_WATERMARK` 7→2; the `lib.rs`
  round-trip test that used `watermark: 42` on an empty ledger was corrected to a
  valid non-zero shape; a negative test (`load_rejects_watermark_beyond_ledger`)
  was added.
- **P1-B (unvalidated deserialized ledger) — FIXED.** New
  `MemoryLedger::validate_for_load() -> Result<(), LedgerIntegrityError>` enforces
  the forever-save invariant `events[i].event_id.0 == i` (contiguous-from-zero),
  called by `load_envelope` BEFORE `restore_transient_state`. Non-contiguous /
  duplicate / gappy ledgers are rejected via `SaveError::MalformedLedger`. Unit
  tests (empty/contiguous/gappy) + a save-boundary negative test
  (`load_rejects_non_contiguous_ledger`, built via `from_events([id0,id5])`) added.
- **Verification:** `scripts/fw verify` exit 0; both canonical pins UNCHANGED
  (`85f45bf8…` / `12ce5ab7…` — no sim/match-state touched); fw-save 17 +
  fw-memory 91 + all suites 0 failed. Self-review triple → all three ACCEPT.
- **Deferred to QA-T4H:** P2 (5-seed envelope `[0,7]→[1,7]` or aggregate lower
  bound — needs an empirical per-seed sweep first to avoid a CI flake), P2
  (compaction decaying-event reader-order test + doc note on the timeless-anchor
  semantic), P3 (club-subject `TitleWon` callback prose — `narrative-director`).

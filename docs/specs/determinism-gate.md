---
description: Phase-0 determinism gate. The load-bearing acceptance test for T0 (Scaffold phase). Without this passing on Mac + Windows + Linux, no other Final Whistle work proceeds.
last_verified: 2026-05-13
status: Accepted
phase: 0
---

# Final Whistle — Determinism Gate (Phase 0, T0 Scaffold)

> **Stop sign for the entire project.** If `cargo test -p fw-replay
> canonical_hash` is red on any platform in CI, NO other Phase-0+ work
> proceeds until it is green. This is the non-negotiable floor that every
> later system (replay corpus, save migration, careers ledger, content
> compiler) builds on.

---

## 1. Why determinism matters

The Final Whistle non-negotiables (DESIGN_DOC §2 rule 2 + §5
"Determinism contract") pin three commercial-grade promises that all
collapse if the sim is non-deterministic.

### 1.1 Replay corpus
Every commit re-verifies a pinned-hash regression corpus. Seeds map to
canonical-state hashes; a hash drift on any platform is either a real bug
or an intentional re-baseline requiring reviewer-approved diff. Without
determinism, the corpus catches nothing — every run produces a different
hash and the discipline collapses to "trust me bro."

### 1.2 Save migrations
Saves reference seed + content-pack version. When a player loads a save
from schema v1 into the v2 build, the migration logic must reproduce
byte-identical state to what v1 wrote — otherwise mid-career events
mutate silently across upgrades. Schema bumps owe four-tests-per-bump
(forward-migration + callback-preservation + forward-incompat-failure +
round-trip-byte-identical) per `design/specs/save-migration-fixtures.md`,
all of which assume the underlying sim is reproducible.

### 1.3 Cross-platform parity
Steam ships Mac + Windows + Linux from a single repo. A Mac-developed
save must open identically on Windows. Without bit-exact reproducibility,
"my striker scored on Mac but missed on Windows" is a defensible bug
report — and one we cannot triage without a deterministic baseline.

### 1.4 Anti-cheat via replay
Premium single-player, no server-side state. The only credible defense
against career-edit cheating (modified saves competing on community
leaderboards / Workshop achievements) is "this save's claimed history
must replay to its canonical hash." A non-deterministic sim makes this
trivially forgeable.

---

## 2. Three layers

Determinism is composed, not assumed. Each layer is independently
verifiable; all three must hold for the gate to pass.

### Layer 1 — Numerical (Q32.32 fixed-point)

Every canonical-state quantity (positions, velocities, scores, ball
physics state, timers, derived trajectory values) is a `Q32` —
Q32.32 fixed-point integer arithmetic. `f32` / `f64` are **forbidden**
in `fw-match-sim`, `fw-memory`, `fw-replay`, `fw-save`, `fw-content`.
The forbidden status is clippy-enforced via
`#[deny(clippy::float_arithmetic)]` at the crate root of every
canonical-state crate.

Viewer-side interpolation (in `fw-tauri` / the SolidJS frontend) MAY
use floats — that's the renderer's prerogative. The canonical
authoritative state is fixed-point.

Q32.32 is a signed 64-bit integer where the top 32 bits are the
two's-complement integer part and the bottom 32 bits are the
fractional part. Range is approximately ±2.147e9; precision is
2^-32 ≈ 2.328e-10.

### Layer 2 — Encoding (canonical byte order)

A `MatchState` (or any canonical-state struct) serializes to bytes via
an explicit `serialize_in_canonical_order(&self, buf: &mut Vec<u8>)`
method. **NOT** derive(Serialize) — derive macros can reorder fields
across edition / nightly / minor compiler releases, and any reorder
silently changes the hash. The explicit method writes one field at a
time in a locked order documented in source.

Encoding rules (locked at v1):

- **Endianness:** little-endian for every multi-byte primitive. Use
  `to_le_bytes()` everywhere; never `to_ne_bytes()`.
- **Primitives:** `Q32` writes its raw `i64` as 8 bytes little-endian.
  `Tick` writes its raw `u64` as 8 bytes little-endian.
- **Strings:** 4-byte little-endian length prefix (UTF-8 byte count, not
  char count) + UTF-8 bytes. Empty string writes 4 zero bytes.
- **Collections:** 4-byte little-endian non-negative count + elements
  written one-by-one. Element order MUST be canonical — sorted by a
  stable key (typically a content-pack-qualified ID, ordinal-compared)
  if the in-memory order is not already authoritative.
- **Maps (BTreeMap / IndexMap):** iterate in key order; for `BTreeMap`
  this is the default. **NEVER** `HashMap` in canonical-emitting code.
- **Booleans:** 1 byte. `false` = `0x00`; `true` = `0x01`.

### Layer 3 — Hashing (BLAKE3)

Bytes hash to a 32-byte digest via BLAKE3. The pinned-hash table stores
the digest as `[u8; 32]` literals. The hash is the regression contract;
drift on any platform fails the gate.

---

## 3. Q32.32 in Rust — API surface

The newtype lives in `crates/fw-core/src/q32.rs`. It is the determinism
primitive that the rest of `fw-match-sim` is built on.

```rust
pub struct Q32(pub FixedI64<U32>);
```

Backed by `fixed = { version = "1", features = ["serde"] }` — the
`fixed` crate is a mature, well-tested Rust fixed-point library that
gives us `FixedI64<U32>` (signed 64-bit, 32 fractional bits) with
correct integer-bit-exact arithmetic. Serde support is enabled so we
can round-trip via RON / bincode for fixtures + saves.

### 3.1 Construction
- `Q32::from_int(n: i32) -> Q32` — always safe; every `i32` fits.
- `Q32::zero() -> Q32` — additive identity.
- `Q32::one() -> Q32` — multiplicative identity.
- `Q32::from_raw(bits: i64) -> Q32` — bit-level construction for
  fixture authoring + deserialization. Normal sim code does NOT call
  this.

### 3.2 Arithmetic
- `+`, `-`, `*`, `/`, unary `-` — standard operator overloads with
  checked semantics. Overflow panics in debug, wraps in release UNLESS
  we explicitly opt into checked. **Decision:** use `checked_*`
  methods in canonical-state-touching code; bare operators in viewer
  code. Per the FW MatchSim reference impl, silent wraparound is
  forbidden — overflow signals a real bug.
- `sqrt` via the `cordic` crate. CORDIC is an integer-only iterative
  algorithm with bit-exact cross-platform behavior. The FW reference
  uses Newton-Raphson on `BigInteger`; CORDIC is the Rust idiom and
  faster on the hot path.

### 3.3 Constants
- `Q32::EPSILON` — one ULP (raw = 1, magnitude ≈ 2.328e-10).
- `Q32::MAX` — `FixedI64::<U32>::MAX`.
- `Q32::MIN` — `FixedI64::<U32>::MIN`.

### 3.4 Serde
- `#[serde(transparent)]` — `Q32` serializes as its underlying
  `FixedI64<U32>`, which serializes as a string in RON (human-diffable
  in PR review) and as bytes in bincode (compact for saves).

### 3.5 Clippy enforcement

In `crates/fw-match-sim/src/lib.rs` (and the four other canonical
crates):

```rust
#![deny(clippy::float_arithmetic)]
#![deny(clippy::cast_possible_truncation)]
#![deny(clippy::cast_precision_loss)]
```

Any `f32` / `f64` arithmetic in these crates fails CI. Viewer-side
crates (`fw-tauri`, frontend) are exempt.

---

## 4. Deterministic randomness

The sim uses `rand_chacha::ChaCha8Rng` seeded from a `Seed(u64)` newtype.

**Banned** (clippy + audit-time grep):
- `rand::thread_rng()` — pulls from OS entropy.
- `rand::rngs::StdRng` — wraps an unspecified algorithm; changes
  between minor versions.
- `rand::rngs::OsRng` — non-reproducible by definition.
- Any `std::time::Instant::now()` / `SystemTime::now()` in sim code.

The seed lifecycle (per DESIGN_DOC §5 + ADR-0009):
- Each match carries `match_seed: Seed`.
- Per-draw stochastic events derive from `ChaCha8Rng::seed_from_u64(seed_fn(match_seed, tick, layer, site))` — never from a single long-lived RNG, so out-of-order event emission cannot drift the stream.
- `seed_fn` is BLAKE3 over a 17-byte fixed-order buffer
  (`match_seed.to_le_bytes()` ++ `tick.to_le_bytes()` ++ `layer as u8` ++
  `site.to_le_bytes()`), truncated to `u64` little-endian. Identical
  `(match_seed, tick, layer, site)` → identical `u64` across
  macOS / Windows / Linux / aarch64 / x86_64.
- `SeedLayer` is an 8-variant `#[repr(u8)]` enum with stable discriminants:
  `Decision` (0x10), `UtilityTieBreak` (0x11), `ReactiveInterrupt`
  (0x12), `BallPhysics` (0x13), `SignatureTrigger` (0x14),
  `MemoryEvent` (0x20), `ScoutObservation` (0x30), `ContentBake` (0x40).
  Layers are non-overlapping; `site` disambiguates within a layer (e.g.
  `(player_id << 16) | slot` for `Decision`).
- Reconciled 2026-05-13 per ADR-0009; the prior `match_seed ^ tick ^ event_id_salt` XOR shape was retracted because it (a) had no layer discriminator (two layers drawing at the same tick collided) and (b) had weak avalanche on short inputs.

---

## 5. Deterministic containers

In canonical-state-emitting code, use `BTreeMap` / `BTreeSet` / `Vec`
/ `IndexMap` (with explicit insertion order discipline). **`HashMap`
and `HashSet` are banned** in `fw-match-sim`, `fw-memory`,
`fw-replay`, `fw-save`, `fw-content`.

Rust's `HashMap` uses `RandomState` by default, which keys the hasher
from per-process entropy — iteration order changes between runs, even
on the same machine. Even with a deterministic hasher (FxHash etc.),
the iteration order is implementation-defined and may change between
compiler releases. `BTreeMap` is sorted-by-key, which is the
contract canonical encoding needs.

`IndexMap` is allowed where insertion order is the canonical order
(e.g., per-pack content-load order); the discipline is that the
inserter owns the order, not a hash function.

---

## 6. Canonical encoder

A `CanonicalEncoder` writes the locked-order byte stream. The Rust
shape mirrors the FW C# reference at
`MatchSim/Sim/CanonicalEncoder.cs`:

```rust
pub struct CanonicalEncoder {
    buf: Vec<u8>,
}

impl CanonicalEncoder {
    pub fn write_q32(&mut self, v: Q32) { ... }
    pub fn write_tick(&mut self, v: Tick) { ... }
    pub fn write_u32(&mut self, v: u32) { ... }
    pub fn write_i32(&mut self, v: i32) { ... }
    pub fn write_u64(&mut self, v: u64) { ... }
    pub fn write_i64(&mut self, v: i64) { ... }
    pub fn write_bool(&mut self, v: bool) { ... }
    pub fn write_string(&mut self, v: &str) { ... }
    pub fn write_count(&mut self, n: usize) { ... }  // panics on > i32::MAX
    pub fn finish(self) -> Vec<u8> { self.buf }
    pub fn finish_hash(self) -> [u8; 32] {
        blake3::hash(&self.buf).into()
    }
}
```

Types in canonical state implement:

```rust
pub trait CanonicalEncode {
    fn serialize_in_canonical_order(&self, enc: &mut CanonicalEncoder);
}
```

**Not** a derive macro. **Not** `serde::Serialize`. Explicit hand-rolled
implementation so that the field order is read out of source code, not
inferred from struct declaration order. (Derive-based serialization
breaks if a future contributor reorders the struct for readability —
silent canonical-state drift.)

---

## 7. Hash function — why BLAKE3 over SHA-256

The FW C# reference uses SHA-256. The Rust pivot upgrades to BLAKE3 for
three reasons. The choice is recorded here so future-Claude doesn't
re-litigate it.

1. **2–5× faster.** BLAKE3 hashes ~3 GB/s single-threaded on modern
   AMD64; SHA-256 hashes ~500 MB/s without SHA-NI hardware extensions.
   Match canonical state is small (~1 KB) so absolute speed barely
   matters per-tick — but the replay-corpus regeneration in
   `scripts/fw verify` rehashes thousands of seeds × thousands of
   ticks during heavy-local sweeps, and BLAKE3 cuts that wall-clock
   noticeably.
2. **Same security level.** Both are 256-bit cryptographic hashes
   with no known practical attacks. The anti-cheat-via-replay use case
   (forging a save that hashes to a legitimate value) requires a
   second-preimage attack on either, and both are out of reach for
   the foreseeable future. BLAKE3 is the modern default for new
   projects.
3. **Smaller dependency surface.** The `blake3` crate is ~3k LoC of
   well-audited Rust with zero unsafe in the default build. The
   `sha2` crate is fine too, but BLAKE3 is the contemporary choice
   when starting from scratch.

The 32-byte digest format is the same as SHA-256, so the pinned-hash
table layout (`[u8; 32]`) is unchanged.

**The trade-off:** the FW C# reference's pinned hashes are SHA-256
strings of the form `sha256:abc123...`. The Rust corpus uses a fresh
literal — there is no migration path for old hashes because the
underlying sim is being rewritten anyway. This is a clean break, not
a hash-algorithm swap on a stable codebase.

---

## 8. Replay corpus format

Fixtures live at `crates/fw-replay/fixtures/<seed>.ron`. **RON, not
JSON**, for two reasons:

1. **Human-diffable in PR.** RON allows comments + trailing commas +
   `Q32(0x00000000_3FB851EB)` style hex literals. PR reviewers can
   read the fixture and see what changed.
2. **Forward-compatible.** RON tolerates field additions without
   breaking older parsers (with `#[serde(default)]` on the consuming
   struct). JSON does too, but RON's serde integration is more
   idiomatic in Rust.

Schema:

```ron
ReplayCorpusEntry(
    schema_version: 1,
    seed: "0xdeadbeefdeadbeef",
    content_pack_version: "fwh.core@0.1.0",
    tick_count: 60,
    tick_rate_hz: 60,
    expected_hash: "blake3:0000000000000000000000000000000000000000000000000000000000000000",
    metadata: (
        description: "Tier-A smoke seed — 60-tick cross-platform replay gate.",
        generated_at: "2026-05-13T00:00:00Z",
        generated_by: "scripts/fw replay --regenerate-corpus",
    ),
)
```

The `expected_hash` is the BLAKE3 digest of the canonical encoding of
the final `MatchState` after `tick_count` ticks. The fixture's hash is
authoritative; the in-code `PINNED_HASHES` table in
`crates/fw-replay/tests/canonical_hash.rs` MUST agree (a separate test
asserts the agreement, so the two cannot drift independently).

---

## 9. The pinned-hash registry (T1-22)

The canonical-state hash is pinned in MULTIPLE locations. Pre-T1-22 this
was a "remember the N places" manual list that already drifted from 4 to 5
during T1-15 + T1-16 rebaselines. T1-22 codified the registry as a script:

```sh
# List the current state of all pin locations:
scripts/fw hash-pins

# Atomically update all locations for a given seed:
scripts/fw hash-pins --update <NEW_HASH> --seed <SEED>
scripts/fw hash-pins --update <NEW_HASH> --seed <SEED> --dry-run  # preview
```

Implementation: `scripts/fw-hash-pins.py`. The `PIN_LOCATIONS` table inside
the script is the single source of truth for "which files pin the hash";
adding a new pin location = adding a new entry there.

**Genuine atomicity guarantee (T1-24)**: `--update` runs in two phases.
Phase 1 (preflight) reads every matching file and computes every replacement
text in memory WITHOUT writing. If any preflight fails (pin file missing /
regex no longer matches / unknown form), the script aborts with exit code 1
and ZERO files modified — sibling locations that would have written are
NOT written. Phase 2 (write) only runs when every preflight succeeded.
This closes the partial-update silent-failure mode Codex Finding #2
(post-followup-review 2026-05-16) caught in the T1-22 ship: the prior
tri-state fix made failures loud but the loop still wrote-as-you-go.

Atomicity is verified by `scripts/test-fw-hash-pins.py::test_partial_preflight_failure_writes_nothing`
which deliberately breaks one pin location and asserts no sibling files
were modified after the update attempt. Wired into `just banned-terms` +
CI's `hash-pins atomicity test` step on all 3 OSes.

### Current pin locations (5 total across 2 corpus seeds)

| Seed | Location | Form |
|---|---|---|
| `0xdeadbeefdeadbeef` (60-tick smoke) | `crates/fw-replay/tests/canonical_hash.rs::PINNED_60_TICK` | `hex!()` macro |
| `0xdeadbeefdeadbeef` (60-tick smoke) | `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron::expected_hash` | RON `"blake3:..."` |
| `0xdeadbeefdeadbeef` (60-tick smoke) | `crates/fw-content/tests/fixtures_load.rs::EXPECTED` | raw `[u8; 32]` byte array |
| `0xfeedbeefcafefade` (600-tick extended) | `crates/fw-replay/tests/canonical_hash.rs::PINNED_600_TICK` | `hex!()` macro |
| `0xfeedbeefcafefade` (600-tick extended) | `crates/fw-replay/fixtures/0xfeedbeefcafefade.ron::expected_hash` | RON `"blake3:..."` |

The cross-check test `smoke_seed_corpus_fixture_matches_pinned_constant`
(canonical_hash.rs) asserts the in-code constant + the RON fixture agree
for the smoke seed; the analogous test exists for the extended seed.

### Rebaseline procedure

When an ADR-0012-authorized rebaseline is needed:

1. Run `scripts/fw hash-pins` to see the current state.
2. Determine the new hash by editing ONE pin location to a placeholder
   (e.g. `[0u8; 32]`), running `cargo test -p fw-replay --test canonical_hash
   <pinned_test>`, and reading the actual hash from the test failure message.
3. Run `scripts/fw hash-pins --update <NEW_HASH> --seed <SEED>` to atomically
   propagate the new hash to all sibling locations. (Use `--dry-run` first to
   confirm scope.)
4. Author a re-baseline history comment in each affected file per the existing
   `Prior hash (T1-XX / reason): <old_hash>` convention.
5. Run `scripts/fw verify` to confirm all tests pass against the new pin.
6. Commit with the `canonical hash: REBASELINED (trigger: <1-4>; old: ...;
   new: ...; reason: ...)` marker per `.claude/hooks/validate-commit.sh`.

### Intra-process determinism test parameters

The 100×/10× rerun-count tests (`smoke_seed_runs_100_times_produce_one_hash` +
`extended_seed_runs_10_times_produce_one_hash`) are parameterized via env vars:

| Env var | Default | Purpose |
|---|---|---|
| `FW_DETERMINISM_SMOKE_RUNS` | `100` | Override the smoke-seed rerun count |
| `FW_DETERMINISM_EXTENDED_RUNS` | `10` | Override the extended-seed rerun count |

Audit-time stress testing — e.g. `FW_DETERMINISM_SMOKE_RUNS=10000` to push
100× harder — no source edits needed. CI defaults preserve the current wall-
clock cost.

---

## 10. Test structure

Three tests in `crates/fw-replay/tests/canonical_hash.rs`:

### 10.1 `smoke_seed_60_tick_canonical_hash_pinned`
The bedrock. Builds `MatchState::initial(SMOKE_SEED)`, ticks 60 times,
canonical-encodes, BLAKE3-hashes, asserts equality with the pinned
`PINNED_60_TICK` literal. **This is the Phase-0 gate.** Red on any of
{macOS-14, windows-latest, ubuntu-22.04} blocks the merge.

### 10.2 `smoke_seed_runs_100_times_produce_one_hash`
Sanity check that the hash is stable on the same machine. 100 fresh
runs of the same seed → exactly one distinct hash. Catches hidden
non-determinism (a leaked `HashMap` iteration, a stray `thread_rng()`,
a `SystemTime::now()` snuck into the seed derivation).

### 10.3 `smoke_seed_corpus_fixture_matches_pinned_constant`
Reads the RON fixture from disk, parses out `expected_hash`, asserts
it equals the in-code `PINNED_60_TICK` constant. Prevents drift
between the fixture file (which `scripts/fw replay --compare-corpus`
reads) and the test constant (which CI's `cargo test` reads).

### 10.4 `insta` snapshot tests (additional)

`insta` snapshot of the final `MatchState` post-60-ticks (rendered
via `Debug` or a stable `Display` impl) for human-readable change
detection. The snapshot files (`*.snap`) live next to the test file
and are committed. Drift surfaces in PR as a textual diff — much
easier to triage than a hex-string mismatch.

---

## 11. CI matrix

`.github/workflows/fast-pr-ci.yml` runs the determinism job on:
- `macos-14` (Apple Silicon, the dev box)
- `windows-latest`
- `ubuntu-22.04`

All three must be green. A single platform red = merge blocked. The
job is `cargo test -p fw-replay canonical_hash --release` (release
mode because debug-mode integer overflow checks can mask real bugs by
panicking instead of producing a different hash — we want the hash
itself to be the contract).

Tier-A (every push) runs the smoke seed (60 ticks). Tier-D (RC gate,
manual dispatch) runs the full corpus.

---

## 12. Phase-0 acceptance gate

The Phase-0 (T0 Scaffold) gate is closed when:

- [ ] `crates/fw-core/src/q32.rs` compiles + passes unit tests on all
  three OSes.
- [ ] `crates/fw-replay/tests/canonical_hash.rs` compiles. Tests may
  initially `#[ignore]` the pinned assertion (because the placeholder
  `[0u8; 32]` will not equal the real hash); once a real
  `MatchState::initial` + `tick_match` land, the test is un-ignored
  and the placeholder is replaced with the first CI green hash.
- [ ] `scripts/fw verify` calls `cargo test -p fw-replay canonical_hash`
  as part of its `verify` umbrella.
- [ ] `.claude/hooks/canonical-hash-guard.sh` runs the targeted test
  before allowing any `git commit` that touches `fw-match-sim/**` or
  `fw-core/**`.

When all four are checked, the rest of Final Whistle can build on
this floor with confidence.

---

## 13. References

- `docs/DESIGN_DOC.md` §2 rule 2 (non-negotiable determinism) + §5
  (Match Simulation Layer / Determinism contract).
- `MIGRATION_AUDIT.md` §3.1 (pinned-hash regression discipline) + §3.11
  (canonical encoder + seeded RNG).
- FW C# reference: `MatchSim/Sim/Fixed.cs`,
  `MatchSim/Sim/CanonicalEncoder.cs`,
  `MatchSim.Tests/Sim/MatchDeterminismTests.cs`,
  `MatchSim.Tests/fixtures/replay-corpus/0xdeadbeefdeadbeef.json`.
- `fixed` crate: <https://docs.rs/fixed>.
- `rand_chacha` crate: <https://docs.rs/rand_chacha>.
- `blake3` crate: <https://docs.rs/blake3>.

---

*Authored 2026-05-13. Phase 0 / T0 Scaffold acceptance test. Revise
only via append-only `docs/DECISIONS.md` entry citing this section.*

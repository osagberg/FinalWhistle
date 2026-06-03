# ADR-0010 — Save format

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** Claude (Codex full-project audit Lane B "missing ADR" driver) + Codex (pending pre-T2-9 audit)

---

## Context

`fw-save` is the canonical persistence boundary. Save → quit → load MUST reproduce canonical state byte-identically (`docs/MASTER_PLAN.md` T2 Exit Gate). The Codex full-project audit Lane B flagged that the save format's structural choices — bincode version, compression, migration rules — live as folklore in CLAUDE.md / `MEMORY.md` / a Codex-Imp-12 deferral marker, without an authoritative ADR. T2-9 is the first row that lands actual save code; settling the spec now prevents drift during implementation.

The choice surface:

1. **Serialization framework.** Two real candidates: `bincode 1.x` (legacy, but `fw-core`'s existing dev-deps use it) vs `bincode 2.x` (current, breaking API change, derive-encode/decode replaces serde). Codex pre-T0 audit Imp #12 flagged the version mismatch as a real follow-up.
2. **Compression.** `zstd` is the only seriously-considered candidate. The 50k-player attribute store compresses to ~6 MB at `zstd::DEFAULT_COMPRESSION_LEVEL` (3) per the ADR-0002 §Consequences storage analysis.
3. **Migration discipline.** Forward-only (`v_N` → `v_{N+1}`), per `Content/RULES.md` §3 pattern. Each schema bump owes four tests per `docs/specs/save-migration-fixtures.md` (forward-migration + callback-preservation + forward-incompat + round-trip-byte-identical).
4. **Mod-load fingerprint.** Saves stamp the `BLAKE3(sorted_mod_id_list)` hash; loading a save with different mods active surfaces a warning (per `Content/RULES.md` §6).

## Decision

### Serialization: bincode 2 + serde compat shim

We adopt **`bincode = "2"`** for `fw-save`'s on-disk format. `fw-core`'s existing dev-deps stay on `bincode = "1"` until those tests get migrated; the two versions coexist in the workspace (different crates).

**Why bincode 2:** stable since 2024-Q4, smaller emitted bytes for varint-encoded fields, `#[derive(Encode, Decode)]` is faster than serde-based encode/decode. Source for `bincode = "1"` is in maintenance-only mode upstream.

**Workspace plan to retire bincode 1.x:** during T2-9, migrate `fw-core`'s existing bincode-1 round-trip tests to bincode 2 with the `serde` feature (`bincode = { version = "2", features = ["serde"] }`). Net workspace ends on bincode 2 only. Tracked as a Tranche 6 follow-up.

### Compression: zstd

`zstd = "0.13"` with `compression_level = 3` (the default). A formal `save-format-benchmarks.md` was never authored (the planned Tranche-4 deliverable was descoped); level 3 has been adequate through SaveV4. Revisit with a real benchmark only if save size or load latency becomes a problem.

Saves are emitted as a single binary blob: `[magic 4 bytes "FWS1"][version u32][zstd-compressed bincode-2 payload]`. The leading magic + version are uncompressed for cheap version-check during load (no need to decompress before deciding which migration chain to apply).

### Migration: forward-only

Per-version migration adapters live in `crates/fw-save/src/migrations/<n>_to_<n+1>.rs`. Loading flow:

```
read magic + version
  → if version > MAX_SUPPORTED: error "save from a newer game version"
  → if version < CURRENT: chain through migrations 0_to_1, 1_to_2, ..., (current-1)_to_current
  → decode current version
```

Migrations are pure functions `Save_v{N} → Save_v{N+1}`. Each migration owes the four-test contract per `docs/specs/save-migration-fixtures.md`. **Re-writing on-disk fixtures across versions is forbidden** — the `v_N` fixture stays as it was the day `v_{N+1}` shipped, so the migration tests bite if the migration breaks.

### Byte-identical round-trip

`SaveBundle::encode(state) → bytes → SaveBundle::decode(bytes) → state'` MUST satisfy `state.encode_canonical() == state'.encode_canonical()` for every well-formed `state`. The save format is the persistence boundary, not just storage; canonical-hash regression covers this directly.

### Mod-load fingerprint

Saves stamp:
```rust
struct ModLoadFingerprint {
    mod_id_blake3: [u8; 32],   // BLAKE3 over sorted (mod_id, mod_version) pairs
    mod_count: u32,            // for the UI "you had N mods" hint
}
```

Loading a save whose fingerprint doesn't match the currently-loaded mod set surfaces a warning DTO to the frontend (per ADR-0004 IPC contract). The save still loads — mod-overlay drift is informational, not fatal — but `MemoryEvent`s emitted from mod-content `UnknownEventClass` payloads may surface as opaque text in commentary.

## Consequences

**Positive:**
- One save format spec, ADR-backed. Future drift is auditable.
- bincode 2 + zstd matches the industry-default Rust persistence stack (Bevy + many serde consumers landed there).
- Forward-only migrations have a clean test contract.
- Mod-load fingerprint surfaces "this save was made with a different mod set" without forcing version-lock.

**Negative:**
- bincode-1 + bincode-2 coexist in the workspace during the T2-9 → Tranche 6 transition. Workspace `cargo tree` shows both. Tracked.
- `zstd` is a `cc`-built dependency, adding ~30s to cold builds. Acceptable; we already build Tauri (which is heavier).
- The 4-byte magic + u32 version prefix is uncompressed, so a save tampered with by editing the magic/version doesn't fail-fast loudly — it would error at "unsupported version". Acceptable; users editing the magic byte are not in our threat model.

**Neutral:**
- The save format is binary, not human-readable. Replay corpus stays in RON for diffability; saves are binary for size + speed. Different formats for different purposes.

**Rollback path:**
- If `zstd`'s `cc` build hurts CI badly, swap to `lz4_flex` (pure-Rust, no `cc`). Compression ratio is worse (~2× the byte count) but acceptable for a few MB of save data. The format magic byte would bump to "FWS2" to mark the change.
- If `bincode 2` proves unstable under our workload, fall back to `bincode = "1"` via a v1 migration adapter. The on-disk format absorbs the change.

## Alternatives considered

- **CBOR / MessagePack.** Larger emitted bytes than bincode for typed Rust data, and the `serde_cbor` crate is unmaintained. Rejected.
- **JSON.** Human-readable but huge byte counts for attribute-heavy data. Rejected on size.
- **Protobuf.** Cross-language interchange, which we don't need (single Rust codebase). Adds `prost` + `protoc` toolchain. Rejected on dependency-weight grounds.
- **No compression.** ~22 MB per save for a 50k-player career is fine on disk but slow to load + sync to Steam Cloud. Rejected on UX grounds.
- **In-place mutation of older saves on load.** Convenient but violates `Content/RULES.md` §3 (forward-migration only). Rejected on discipline grounds.

## References

- `docs/MASTER_PLAN.md` T2-9 (the implementation row)
- `docs/specs/save-migration-fixtures.md` (the 4-test contract per bump)
- `save-format-benchmarks.md` — descoped/never authored; zstd level 3 has held through SaveV4 (see §"Decision" above)
- `Content/RULES.md` §3 (forward-migration discipline)
- `Content/RULES.md` §6 (mod-overlay load order)
- ADR-0002 §Consequences (storage budget — ~22 MB pre-compression, ~6 MB post)
- ADR-0004 IPC command surface (mod-fingerprint mismatch DTO surfaces here)
- Codex pre-T0 audit Imp #12 (bincode 1 vs 2 alignment deferral)
- Codex full-project audit Lane B "missing ADRs" (the immediate driver)

#!/usr/bin/env python3
"""
fw-hash-pins.py — canonical-state hash pin registry + atomic update tool.

The Final Whistle canonical-state hash is pinned in MULTIPLE locations across
the codebase (in-code constants + RON fixture files + cross-check tests). At
T1-15 + T1-16 rebaselines the pin count cascaded from 4 → 5; manual "remember
the 4-or-5 places" is already drifting. This script codifies the registry
+ provides an atomic `--update` mode so rebaselines can't silently leave a
location stale.

Three forms supported:
  - **RON `expected_hash` field**: `expected_hash: "blake3:HEX..."` (in
    `crates/fw-replay/fixtures/*.ron`).
  - **Rust `hex!()` macro**: `hex!("HEX...")` (in
    `crates/fw-replay/tests/canonical_hash.rs::PINNED_*_TICK`).
  - **Rust raw `[u8; 32]` byte array**: `[0xfc, 0xcc, ...]` (in
    `crates/fw-content/tests/fixtures_load.rs::EXPECTED`).

Each form is detected by the per-location entry in PIN_LOCATIONS below;
adding a new pin location = adding a new entry there.

Usage:
    scripts/fw hash-pins                                  # list mode (default)
    scripts/fw hash-pins --update <NEW_HASH> --seed <SEED>  # atomic rebaseline
    scripts/fw hash-pins --update <NEW_HASH> --seed <SEED> --dry-run  # preview

Exit codes:
    0 - list mode succeeded with no inconsistency / update succeeded with
        every requested location either updated or already at target hash
    1 - list mode: inconsistency detected (sibling locations disagree on
        the pinned hash for the same seed); OR update mode: at least one
        location failed PREFLIGHT (file missing / regex did not match /
        unknown form). The atomicity guarantee (T1-24): on exit 1 from
        update mode, ZERO files have been modified — sibling locations
        that would have written are NOT written. The script genuinely
        delivers all-or-nothing semantics, not just fail-loud.
    2 - invalid CLI arguments (bad hash format, unknown seed label, missing
        required arg)

Cross-references:
    - docs/specs/determinism-gate.md §9 (procedure)
    - docs/adr/0012-hash-rebaseline-policy.md (when rebaseline is allowed)
    - Sim/RULES.md §7 (canonical-state hashing contract)

This script is dev-only. Not packaged with the shipped game.
"""
from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
HASH_REGEX = re.compile(r"^[a-f0-9]{64}$")


# -----------------------------------------------------------------------------
# Pin location registry.
#
# Each entry pins a specific hash for a specific corpus seed in a specific
# file at a specific syntactic form. Adding a new pin location = adding a new
# PinLocation row here.
#
# WHEN TO UPDATE THIS TABLE:
#   - A new corpus seed lands (add 1-3 rows: in-code const + RON fixture +
#     optional cross-check).
#   - An existing pin location moves to a new file or line (update the
#     `file` field; line is regex-anchored, not line-numbered).
#   - The hash form changes (e.g. `hex!()` macro → raw byte array). Update
#     `form` + `pattern`.
# -----------------------------------------------------------------------------

@dataclass(frozen=True)
class PinLocation:
    """A single canonical-hash pin location."""
    seed_label: str                # e.g. "0xdeadbeefdeadbeef" (matches RON fixture filename + seed)
    file: str                      # repo-relative path
    pattern: re.Pattern[str]       # regex with a single capture group on the 64-char hex hash
    form: str                      # "ron" | "hex_macro" | "byte_array" — for human-readable output
    description: str               # short label for the registry's `list` output

    def absolute_path(self) -> Path:
        return REPO_ROOT / self.file


# The 5 pin locations cascaded through at T1-15 + T1-16 rebaselines.
PIN_LOCATIONS: tuple[PinLocation, ...] = (
    PinLocation(
        seed_label="0xdeadbeefdeadbeef",
        file="crates/fw-replay/tests/canonical_hash.rs",
        pattern=re.compile(
            r'const\s+PINNED_60_TICK\s*:\s*\[u8;\s*32\]\s*=\s*\n?\s*hex!\("([a-f0-9]{64})"\)'
        ),
        form="hex_macro",
        description="PINNED_60_TICK (60-tick smoke seed in-code constant)",
    ),
    PinLocation(
        seed_label="0xfeedbeefcafefade",
        file="crates/fw-replay/tests/canonical_hash.rs",
        pattern=re.compile(
            r'const\s+PINNED_600_TICK\s*:\s*\[u8;\s*32\]\s*=\s*\n?\s*hex!\("([a-f0-9]{64})"\)'
        ),
        form="hex_macro",
        description="PINNED_600_TICK (600-tick extended seed in-code constant)",
    ),
    PinLocation(
        seed_label="0xdeadbeefdeadbeef",
        file="crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron",
        pattern=re.compile(r'expected_hash:\s*"blake3:([a-f0-9]{64})"'),
        form="ron",
        description="0xdeadbeefdeadbeef.ron::expected_hash (60-tick RON fixture)",
    ),
    PinLocation(
        seed_label="0xfeedbeefcafefade",
        file="crates/fw-replay/fixtures/0xfeedbeefcafefade.ron",
        pattern=re.compile(r'expected_hash:\s*"blake3:([a-f0-9]{64})"'),
        form="ron",
        description="0xfeedbeefcafefade.ron::expected_hash (600-tick RON fixture)",
    ),
    PinLocation(
        seed_label="0xdeadbeefdeadbeef",
        file="crates/fw-content/tests/fixtures_load.rs",
        # Raw byte-array form: 32 hex bytes spread across multiple lines as `0xFC, 0xCC, ...`.
        # The pattern captures the bytes + a post-processor reassembles them into the hex string.
        pattern=re.compile(
            r'const\s+EXPECTED\s*:\s*\[u8;\s*32\]\s*=\s*\[\s*\n?'
            r'((?:\s*0x[0-9a-fA-F]{2},?\s*\n?)+)'
            r'\s*\]'
        ),
        form="byte_array",
        description="EXPECTED (60-tick smoke fixture cross-check raw byte array)",
    ),
)


# -----------------------------------------------------------------------------
# Read helpers
# -----------------------------------------------------------------------------

def read_pin(loc: PinLocation) -> str | None:
    """Return the 64-char hex hash currently pinned at `loc`, or None if absent.

    Uses `newline=""` per T1-24 silent-failure-hunter P2: on Windows, Python's
    default universal-newlines translation can turn LF reads + writes into
    CRLF, breaking byte-level round-trip equivalence. `newline=""` preserves
    the file's on-disk byte sequence verbatim.
    """
    abs_path = loc.absolute_path()
    if not abs_path.exists():
        return None
    # Post-T2-close CI fix: `Path.read_text(newline=...)` is Python 3.13+;
    # CI runs older Python. `read_bytes().decode()` is byte-exact (no
    # universal-newline translation) + works on Python 3.6+ — same semantic
    # guarantee we wanted from `newline=""`.
    text = abs_path.read_bytes().decode("utf-8")
    m = loc.pattern.search(text)
    if not m:
        return None
    captured = m.group(1)
    if loc.form == "byte_array":
        return _byte_array_to_hex(captured)
    return captured.lower()


def _byte_array_to_hex(raw_bytes_block: str) -> str:
    """Convert `0xFC, 0xCC, ...` capture block to a 64-char hex string."""
    bytes_found = re.findall(r"0x([0-9a-fA-F]{2})", raw_bytes_block)
    if len(bytes_found) != 32:
        raise ValueError(
            f"byte_array pin contained {len(bytes_found)} bytes, expected 32; "
            f"raw block:\n{raw_bytes_block}"
        )
    return "".join(b.lower() for b in bytes_found)


# -----------------------------------------------------------------------------
# Write helpers
# -----------------------------------------------------------------------------

def preflight_pin(
    loc: PinLocation, new_hash: str
) -> tuple[bool, bool, str, tuple[Path, str] | None]:
    """Preflight a single pin location for an update to `new_hash`.

    PURE FUNCTION — does NOT write to disk. Returns the prepared `(path,
    new_text)` tuple ready for a separate write phase, or `None` if no
    write is needed (no-op or failure).

    Returns `(changed, is_failure, status_message, prepared_write)`:

    - `changed=True, is_failure=False, prepared_write=Some` — write needed.
      Caller will write `new_text` to `path` only if EVERY location in the
      batch's preflight succeeded (genuine atomicity).
    - `changed=False, is_failure=False, prepared_write=None` — no-op
      (already at target hash). Benign.
    - `changed=False, is_failure=True, prepared_write=None` — real failure
      (file missing / regex drift / unknown form). Caller must exit non-zero
      WITHOUT writing any sibling locations.

    T1-24 (post-Codex Finding #2) split the prior `update_pin` into this
    preflight phase + a separate write phase in `update_mode` to deliver
    genuine atomicity: if location 1 preflight would succeed but location 2
    preflight fails, location 1's file is NEVER written. T1-22's tri-state
    fix closed the silent-success path but the loop still wrote-as-you-go,
    contradicting the "atomic updater" claim.
    """
    if not HASH_REGEX.match(new_hash):
        return False, True, f"INVALID HASH FORMAT (must be 64 lowercase hex chars): {new_hash}", None

    abs_path = loc.absolute_path()
    if not abs_path.exists():
        return False, True, f"FILE NOT FOUND: {loc.file}", None

    # `newline=""` preserves on-disk byte sequence (no LF↔CRLF translation
    # on Windows). T1-24 silent-failure-hunter P2 — Windows CI must not
    # introduce line-ending drift via the read/write round-trip.
    # Post-T2-close CI fix: `Path.read_text(newline=...)` is Python 3.13+;
    # CI runs older Python. `read_bytes().decode()` is byte-exact (no
    # universal-newline translation) + works on Python 3.6+ — same semantic
    # guarantee we wanted from `newline=""`.
    text = abs_path.read_bytes().decode("utf-8")
    m = loc.pattern.search(text)
    if not m:
        return False, True, f"PATTERN NOT MATCHED in {loc.file} — pin location regex out of date?", None

    current = read_pin(loc)
    if current == new_hash:
        return False, False, f"already at {new_hash[:8]}… (no-op)", None

    if loc.form == "byte_array":
        new_text = _replace_byte_array(text, loc.pattern, new_hash)
    elif loc.form == "hex_macro":
        new_text = _replace_hex_macro(text, loc.pattern, new_hash)
    elif loc.form == "ron":
        new_text = _replace_ron(text, loc.pattern, new_hash)
    else:
        return False, True, f"unknown form {loc.form!r} — registry bug", None

    msg = f"would update {current[:8] if current else '?'}… → {new_hash[:8]}…"
    return True, False, msg, (abs_path, new_text)


def _replace_hex_macro(text: str, pattern: re.Pattern[str], new_hash: str) -> str:
    """Replace the captured hash inside the hex!() macro call."""
    def sub(m: re.Match[str]) -> str:
        full = m.group(0)
        old_hash = m.group(1)
        return full.replace(old_hash, new_hash)
    return pattern.sub(sub, text, count=1)


def _replace_ron(text: str, pattern: re.Pattern[str], new_hash: str) -> str:
    """Replace the captured hash inside a RON `expected_hash: "blake3:..."` field."""
    def sub(m: re.Match[str]) -> str:
        full = m.group(0)
        old_hash = m.group(1)
        return full.replace(old_hash, new_hash)
    return pattern.sub(sub, text, count=1)


def _replace_byte_array(text: str, pattern: re.Pattern[str], new_hash: str) -> str:
    """Replace the captured byte-array block with a new 32-byte array for `new_hash`.

    Output layout mirrors what `cargo fmt` produces for a `[u8; 32]` const at the
    100-char default line width: 15 bytes / 15 bytes / 2 bytes. Matching the
    existing format means update + revert leaves the file byte-identical to the
    pre-update state.
    """
    new_bytes = [new_hash[i:i + 2] for i in range(0, 64, 2)]
    # 15 / 15 / 2 layout (matches `cargo fmt`'s default-line-width output for
    # this specific `[u8; 32]` const at indent 8).
    chunks = [new_bytes[0:15], new_bytes[15:30], new_bytes[30:32]]
    lines = []
    for i, chunk in enumerate(chunks):
        formatted = ", ".join(f"0x{b}" for b in chunk)
        # rustfmt emits trailing comma after EVERY chunk (including the last)
        # in a multi-line array literal.
        formatted += ","
        lines.append(f"        {formatted}")
    block = "\n".join(lines) + "\n    "
    # Reconstruct the const ... = [ ... ] with our new block.
    def sub(m: re.Match[str]) -> str:
        prefix_match = re.match(r"(const\s+EXPECTED\s*:\s*\[u8;\s*32\]\s*=\s*\[)", m.group(0))
        if prefix_match is None:
            return m.group(0)
        prefix = prefix_match.group(1)
        return f"{prefix}\n{block}]"
    return pattern.sub(sub, text, count=1)


# -----------------------------------------------------------------------------
# CLI
# -----------------------------------------------------------------------------

def list_mode() -> int:
    """Print a table of all pin locations + their current hash. Detect cross-seed mismatch."""
    seeds_seen: dict[str, set[str]] = {}
    rows: list[tuple[str, str, str, str]] = []  # (seed, file, form, hash)
    any_missing = False

    for loc in PIN_LOCATIONS:
        current = read_pin(loc)
        if current is None:
            rows.append((loc.seed_label, loc.file, loc.form, "<NOT FOUND>"))
            any_missing = True
        else:
            rows.append((loc.seed_label, loc.file, loc.form, current))
            seeds_seen.setdefault(loc.seed_label, set()).add(current)

    print("fw-hash-pins — canonical-state pin registry")
    print(f"({len(PIN_LOCATIONS)} pin locations across {len(seeds_seen)} corpus seed(s))")
    print()
    print(f"{'seed':<22} {'form':<11} {'hash':<70} location")
    print("-" * 130)
    for seed, file, form, hash_val in rows:
        short_hash = f"{hash_val[:16]}…{hash_val[-8:]}" if len(hash_val) == 64 else hash_val
        print(f"{seed:<22} {form:<11} {short_hash:<70} {file}")
    print()

    inconsistencies = [(s, h) for s, h in seeds_seen.items() if len(h) > 1]
    if inconsistencies:
        print("INCONSISTENCY DETECTED — sibling locations disagree on the pinned hash:")
        for seed, hashes in inconsistencies:
            print(f"  seed {seed} pins {len(hashes)} distinct hashes: {sorted(hashes)}")
        print()
        print("Likely cause: a previous rebaseline updated some pin locations but not all.")
        print("Fix: run `scripts/fw hash-pins --update <correct-hash> --seed <seed>` to converge.")
        return 1

    if any_missing:
        print("WARNING: one or more pin locations reported <NOT FOUND>.")
        print("The pin-location regex may be out of date relative to the source file.")
        return 1

    return 0


def update_mode(new_hash: str, seed: str, dry_run: bool) -> int:
    """Atomically update all pin locations for `seed` to `new_hash`.

    GENUINE ATOMICITY (T1-24 per Codex post-followup-review Finding #2):
    the function runs in TWO phases. Phase 1 is preflight — for each
    matching location, read the file + compute the new text in memory
    WITHOUT writing. If any preflight fails (FILE NOT FOUND / PATTERN
    NOT MATCHED / unknown form), the function aborts with ZERO file
    modifications, prints the per-location status (including which
    siblings WOULD have written had the bad location not failed), and
    returns exit code 1. Phase 2 (write) ONLY runs when all preflights
    succeed.

    Pre-T1-24 the loop wrote-as-you-go: location 1 wrote, then location 2
    failed preflight → exit 1 left disk state half-rebaselined. That
    contradicted the "atomic updater" claim Codex's Finding #2 spotted.
    """
    if not HASH_REGEX.match(new_hash):
        print(f"ERROR: bad hash format {new_hash!r}; expected 64 lowercase hex chars", file=sys.stderr)
        return 2

    matching_locs = [loc for loc in PIN_LOCATIONS if loc.seed_label == seed]
    if not matching_locs:
        # Unknown seed label = bad CLI argument per docstring exit-code contract.
        known_seeds = sorted({loc.seed_label for loc in PIN_LOCATIONS})
        print(f"ERROR: no pin locations for seed {seed!r}", file=sys.stderr)
        print(f"Known seeds: {known_seeds}", file=sys.stderr)
        return 2

    print(f"fw-hash-pins {'--dry-run ' if dry_run else ''}--update {new_hash[:8]}… --seed {seed}")
    print(f"({len(matching_locs)} pin location(s) match this seed)")
    print()

    # -----------------------------------------------------------------
    # PHASE 1: preflight. Read every file + compute every replacement in
    # memory. Accumulate (location, did_change, is_failure, msg, prepared_write).
    # NO disk writes in this phase.
    # -----------------------------------------------------------------
    preflight_results: list[tuple[PinLocation, bool, bool, str, tuple[Path, str] | None]] = []
    for loc in matching_locs:
        did_change, is_failure, msg, prepared = preflight_pin(loc, new_hash)
        preflight_results.append((loc, did_change, is_failure, msg, prepared))

    # Print the preflight status per location.
    changed = 0
    failures = 0
    prepared_writes: list[tuple[Path, str]] = []
    for loc, did_change, is_failure, msg, prepared in preflight_results:
        if is_failure:
            symbol = "✗"
            failures += 1
        elif did_change:
            symbol = "•"
            changed += 1
            assert prepared is not None, "did_change=True must yield a prepared write"
            prepared_writes.append(prepared)
        else:
            symbol = "·"  # benign no-op
        print(f"  {symbol} {loc.file:<55} {msg}")
    print()

    # -----------------------------------------------------------------
    # PHASE 1 GATE: if ANY preflight failed, abort with zero writes.
    # This is the atomicity guarantee — sibling locations that WOULD have
    # written are NOT written when even one location fails.
    # -----------------------------------------------------------------
    if failures:
        verb = "would have failed" if dry_run else "FAILED preflight"
        print(
            f"{failures} location(s) {verb} — registry regex likely drifted or "
            f"pin file moved.",
            file=sys.stderr,
        )
        if not dry_run and changed:
            print(
                f"ATOMICITY GUARANTEE: zero files modified. {changed} sibling "
                f"location(s) that would have written were NOT written because "
                f"sibling preflight(s) failed. Partial-update is exactly the "
                f"silent-failure class fw-hash-pins exists to prevent.",
                file=sys.stderr,
            )
        else:
            print(
                "Investigate before re-running. Partial-update is exactly the "
                "silent-failure class fw-hash-pins exists to prevent.",
                file=sys.stderr,
            )
        return 1

    # -----------------------------------------------------------------
    # PHASE 2: write. Only reached if all preflights succeeded.
    #
    # `newline=""` preserves on-disk byte sequence (no Python universal-
    # newlines LF↔CRLF translation on Windows). T1-24 silent-failure-hunter
    # P2 — the round-trip `read_text(newline="")` → modify → `write_text(newline="")`
    # is byte-equivalent on every platform.
    # -----------------------------------------------------------------
    if not dry_run:
        for path, new_text in prepared_writes:
            path.write_text(new_text, encoding="utf-8", newline="")

    if dry_run:
        if changed:
            print(f"Dry-run complete. {changed} location(s) would be updated.")
            print("Re-run without --dry-run to apply.")
        else:
            print(f"Dry-run no-op: all {len(matching_locs)} location(s) already at {new_hash[:8]}…")
    else:
        if changed:
            print(f"Update complete. {changed} location(s) modified atomically.")
            print()
            print("Next steps:")
            print("  1. Run `scripts/fw verify` to confirm tests pass against the new pin.")
            print("  2. Author a re-baseline history entry in the affected files per the")
            print("     ADR-0012-trigger doc convention (see existing entries for the template).")
            print("  3. Commit with `canonical hash: REBASELINED (trigger: ...)` marker per")
            print("     `validate-commit.sh`'s hook expectations.")
        else:
            print(f"No-op: all {len(matching_locs)} location(s) already pin {new_hash[:8]}…")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Final Whistle canonical-hash pin registry + atomic updater.",
    )
    parser.add_argument(
        "--update", metavar="NEW_HASH",
        help="Atomically update all pin locations for --seed to NEW_HASH (64-char lowercase hex).",
    )
    parser.add_argument(
        "--seed", metavar="SEED",
        help="Corpus seed label (e.g. 0xdeadbeefdeadbeef). Required with --update.",
    )
    parser.add_argument(
        "--dry-run", action="store_true",
        help="Print what --update would do without modifying files.",
    )
    args = parser.parse_args(argv)

    if args.update:
        if not args.seed:
            print("ERROR: --update requires --seed", file=sys.stderr)
            return 2
        return update_mode(args.update, args.seed, dry_run=args.dry_run)
    return list_mode()


if __name__ == "__main__":
    sys.exit(main())

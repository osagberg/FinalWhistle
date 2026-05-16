#!/usr/bin/env python3
"""
test-fw-hash-pins.py — T1-24 acceptance test for genuine atomicity.

T1-22 shipped `scripts/fw-hash-pins.py` with a tri-state return that closed
the silent-success silent-failure-class P1. Codex post-followup-review
Finding #2 caught that the loop still wrote-as-you-go: if location 1 wrote
to disk and location 2 then failed preflight, you'd get a partial rebaseline
with exit 1 — contradicting the "atomic updater" claim. T1-24 refactored
`update_mode` to preflight-then-write: phase 1 reads + computes all
replacements in memory; phase 2 writes ONLY if every preflight succeeded.

This test verifies the atomicity guarantee with a real partial-failure
scenario: it deliberately breaks ONE of the 3 pin locations for the
`0xdeadbeefdeadbeef` seed by renaming its sentinel construct, runs the
script with a fake new hash, and asserts:

  1. Script exit code == 1 (preflight failure detected).
  2. Every pin-location file byte-identical to its pre-run snapshot —
     including the 2 sibling files that WOULD have been written had
     the broken-location preflight succeeded.

If atomicity regresses (e.g. someone reverts to write-as-you-go), the
sibling-file assertions fail with concrete byte-diff output naming the
regression.

Usage:
    python3 scripts/test-fw-hash-pins.py        # runs all tests
    python3 scripts/test-fw-hash-pins.py -v     # verbose

Exit codes:
    0 - all tests pass
    1 - any test failed

The test mutates real repo files temporarily — a try/finally guarantees
restoration even on test crash. If the test crashes BEFORE the restore,
`git status` will show the temporary mutation; run `git checkout --` on
the affected files to restore manually.
"""
from __future__ import annotations

import hashlib
import importlib.util
import subprocess
import sys
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
SCRIPT = REPO_ROOT / "scripts" / "fw-hash-pins.py"


def _load_fw_hash_pins_module():
    """Import `scripts/fw-hash-pins.py` as a module so the test can consult the
    live `PIN_LOCATIONS` registry instead of hard-coding hash values. T1-24
    silent-failure-hunter P2: hard-coded constants silently rot at the next
    rebaseline (the test would still pass but stop testing what it claims to).

    Registers the module in `sys.modules` BEFORE `exec_module` so the script's
    `@dataclass(frozen=True)` decorator on `PinLocation` can resolve the
    module via `sys.modules.get(cls.__module__)` — Python 3.14 made this
    resolution strict; 3.12+ requires the registration step.
    """
    name = "fw_hash_pins"
    spec = importlib.util.spec_from_file_location(name, SCRIPT)
    assert spec is not None and spec.loader is not None, "spec_from_file_location returned None"
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module  # MUST register before exec_module per Py3.14+ dataclass strict-mode
    spec.loader.exec_module(module)
    return module


_FWHP = _load_fw_hash_pins_module()

# The 3 pin-location files for seed 0xdeadbeefdeadbeef per the script's
# PIN_LOCATIONS registry. If a future PIN_LOCATIONS addition introduces a
# 4th file for this seed, add it here too (or the test would miss the
# atomicity check on the new file).
DEADBEEF_PIN_FILES: tuple[str, ...] = (
    "crates/fw-replay/tests/canonical_hash.rs",
    "crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron",
    "crates/fw-content/tests/fixtures_load.rs",
)

# A 64-char hex hash that we'll use as the "new hash" in the test update
# attempt. We choose a value that DOES NOT match the current pin so the
# script would actually try to write if the broken-location preflight didn't
# abort the batch. (If it matched, every location would be a no-op + the
# write phase would skip naturally — making the test vacuous.)
FAKE_NEW_HASH = "aabbccddeeff00112233445566778899aabbccddeeff00112233445566778899"


def file_sha256(path: Path) -> str:
    """Return SHA-256 of `path`'s bytes. Used for byte-identical snapshot diff."""
    h = hashlib.sha256()
    h.update(path.read_bytes())
    return h.hexdigest()


def snapshot_files(paths: tuple[str, ...]) -> dict[str, str]:
    """Return {repo-relative-path: sha256} for each path. Used for byte-identical assertions."""
    return {p: file_sha256(REPO_ROOT / p) for p in paths}


class T1_24_GenuineAtomicity(unittest.TestCase):
    """Codex post-followup-review Finding #2: --update must be all-or-nothing."""

    def test_partial_preflight_failure_writes_nothing(self) -> None:
        """When ONE of N preflight passes fails, ZERO files must be modified.

        Setup: snapshot all 3 deadbeef-pin-files via SHA-256. Mutate ONE file
        (fixtures_load.rs's `const EXPECTED` → `const EXPECTED_DELIBERATELY_RENAMED`)
        so the script's byte_array regex no longer matches. Restore guarantee
        is enforced in a try/finally that runs regardless of test outcome.

        Run: `scripts/fw-hash-pins.py --update <FAKE_HASH> --seed 0xdeadbeefdeadbeef`
        — this would have written 3 files pre-T1-24 (one per location). With
        the new atomicity, the broken-location preflight fails + the batch
        aborts before any writes.

        Assert: exit code 1; every file's post-run SHA-256 matches its
        pre-run SHA-256 (modulo the deliberately-mutated file, which the
        test restores at the end).
        """
        pre_snapshot = snapshot_files(DEADBEEF_PIN_FILES)

        # Pick the byte_array fixtures_load.rs as the file we'll break.
        # The mutation: rename `const EXPECTED` so the regex no longer matches.
        # We back up the original content and restore in finally below.
        target_path = REPO_ROOT / "crates/fw-content/tests/fixtures_load.rs"
        self.assertTrue(
            target_path.exists(),
            f"pre-setup error: target file does not exist at {target_path}. "
            f"DEADBEEF_PIN_FILES may have drifted from the script's PIN_LOCATIONS "
            f"registry — verify with `scripts/fw hash-pins`.",
        )
        # newline="" preserves on-disk byte sequence — see T1-24 silent-failure
        # P2: Python's default universal-newlines translation can break
        # SHA-256 byte-equality on Windows by silently rewriting LF↔CRLF.
        original_text = target_path.read_text(encoding="utf-8", newline="")
        broken_text = original_text.replace(
            "const EXPECTED:", "const EXPECTED_DELIBERATELY_RENAMED:", 1,
        )
        self.assertNotEqual(
            original_text, broken_text,
            "test setup error: target file does not contain `const EXPECTED:`",
        )

        try:
            target_path.write_text(broken_text, encoding="utf-8", newline="")

            # Run the script. We expect exit code 1 (preflight failure).
            result = subprocess.run(
                [sys.executable, str(SCRIPT),
                 "--update", FAKE_NEW_HASH,
                 "--seed", "0xdeadbeefdeadbeef"],
                capture_output=True,
                text=True,
                check=False,
                cwd=REPO_ROOT,
            )

            self.assertEqual(
                result.returncode, 1,
                f"expected exit code 1 (preflight failure detected); "
                f"got returncode={result.returncode}\n"
                f"stdout={result.stdout!r}\nstderr={result.stderr!r}",
            )

            # The error message must explicitly name the atomicity guarantee
            # so a CI failure-mode pin points at the right thing.
            self.assertIn(
                "ATOMICITY GUARANTEE",
                result.stderr,
                f"stderr must name the atomicity guarantee so partial-update "
                f"regression is loud at CI; got stderr={result.stderr!r}",
            )
            self.assertIn(
                "zero files modified",
                result.stderr,
                f"stderr must claim zero-files-modified; got stderr={result.stderr!r}",
            )

            # The critical atomicity assertion: every sibling file must be
            # byte-identical to its pre-run snapshot. The broken file
            # (fixtures_load.rs) is currently in the broken state, NOT the
            # pre-run state — we check it AFTER the finally restore.
            for path_rel in DEADBEEF_PIN_FILES:
                if path_rel == "crates/fw-content/tests/fixtures_load.rs":
                    continue  # mutated file; check after restore
                post_sha = file_sha256(REPO_ROOT / path_rel)
                self.assertEqual(
                    pre_snapshot[path_rel], post_sha,
                    f"ATOMICITY REGRESSION: sibling file {path_rel} was "
                    f"modified despite broken-location preflight failure. "
                    f"Pre-run sha={pre_snapshot[path_rel]}; post-run sha={post_sha}.",
                )
        finally:
            # Restore the deliberately-broken file. ALWAYS runs, even on
            # test failure or crash mid-run.
            # If THIS write itself fails, the file is left in the broken
            # state; recovery is `git checkout -- crates/fw-content/tests/fixtures_load.rs`.
            target_path.write_text(original_text, encoding="utf-8", newline="")

        # Post-restore: verify the broken file is now back to its pre-run state.
        post_restore_sha = file_sha256(target_path)
        self.assertEqual(
            pre_snapshot["crates/fw-content/tests/fixtures_load.rs"],
            post_restore_sha,
            "test cleanup failed: deliberately-mutated file not restored to "
            "pre-run state. Run `git checkout -- "
            "crates/fw-content/tests/fixtures_load.rs` to restore manually.",
        )

    def test_dry_run_partial_failure_still_writes_nothing(self) -> None:
        """`--dry-run` with broken preflight should also write nothing.

        Sanity check: --dry-run already shouldn't write, but verify the
        broken-preflight path interacts correctly with --dry-run (e.g. no
        accidental disk touch even when reporting the failure).
        """
        pre_snapshot = snapshot_files(DEADBEEF_PIN_FILES)

        target_path = REPO_ROOT / "crates/fw-content/tests/fixtures_load.rs"
        self.assertTrue(target_path.exists(), f"setup error: {target_path} not found")
        original_text = target_path.read_text(encoding="utf-8", newline="")
        broken_text = original_text.replace(
            "const EXPECTED:", "const EXPECTED_DELIBERATELY_RENAMED:", 1,
        )

        try:
            target_path.write_text(broken_text, encoding="utf-8", newline="")

            result = subprocess.run(
                [sys.executable, str(SCRIPT),
                 "--update", FAKE_NEW_HASH,
                 "--seed", "0xdeadbeefdeadbeef",
                 "--dry-run"],
                capture_output=True,
                text=True,
                check=False,
                cwd=REPO_ROOT,
            )

            self.assertEqual(result.returncode, 1, f"dry-run with broken preflight must exit 1")

            # Dry-run takes the "would have failed" stderr branch (not the
            # "ATOMICITY GUARANTEE" branch, which is dry-run-excluded because
            # dry-run never writes regardless of failures). Verify the
            # human-readable failure message IS emitted so a CI regression on
            # the dry-run-error-path can't sneak through silently.
            self.assertIn(
                "would have failed",
                result.stderr,
                f"dry-run failure path must emit `would have failed` stderr; "
                f"got stderr={result.stderr!r}",
            )

            for path_rel in DEADBEEF_PIN_FILES:
                if path_rel == "crates/fw-content/tests/fixtures_load.rs":
                    continue
                post_sha = file_sha256(REPO_ROOT / path_rel)
                self.assertEqual(
                    pre_snapshot[path_rel], post_sha,
                    f"dry-run modified sibling file {path_rel} — should never write",
                )
        finally:
            target_path.write_text(original_text, encoding="utf-8", newline="")

    def test_successful_dry_run_against_real_state_does_not_write(self) -> None:
        """Sanity: --dry-run against the current pinned hash (no-op path) writes nothing.

        T1-24 silent-failure-hunter P2: the hash MUST come from the live
        PIN_LOCATIONS registry, not a hard-coded constant. A hard-coded
        constant would silently rot at the next rebaseline — the test would
        still pass (script reports `would update` instead of `no-op` but
        dry-run still exits 0 + writes nothing), but it would stop testing
        the no-op path it claims to test.
        """
        pre_snapshot = snapshot_files(DEADBEEF_PIN_FILES)

        # Read the current pin from the live registry. If the script's
        # PIN_LOCATIONS table is out of sync with disk state (sibling
        # locations disagree), this test setup itself errors out — which is
        # what we want (a registry-vs-disk inconsistency is its own bug
        # class detected by `scripts/fw hash-pins` list mode).
        deadbeef_locs = [
            loc for loc in _FWHP.PIN_LOCATIONS if loc.seed_label == "0xdeadbeefdeadbeef"
        ]
        self.assertGreater(
            len(deadbeef_locs), 0,
            "PIN_LOCATIONS registry has no entries for 0xdeadbeefdeadbeef",
        )
        current_hash = _FWHP.read_pin(deadbeef_locs[0])
        self.assertIsNotNone(
            current_hash,
            f"read_pin returned None for {deadbeef_locs[0].file} — registry regex drift?",
        )

        result = subprocess.run(
            [sys.executable, str(SCRIPT),
             "--update", current_hash,
             "--seed", "0xdeadbeefdeadbeef",
             "--dry-run"],
            capture_output=True,
            text=True,
            check=False,
            cwd=REPO_ROOT,
        )
        self.assertEqual(result.returncode, 0, f"no-op dry-run must exit 0; got {result.returncode}")
        # Verify the script REALLY hit the no-op path (not the "would update" path).
        # If a future caller hard-codes a stale hash here, this assertion catches it.
        self.assertIn(
            "no-op",
            result.stdout,
            f"script must report `no-op` for an already-pinned hash; "
            f"got stdout={result.stdout!r} — if you see `would update`, the test "
            f"is no longer testing the no-op path",
        )

        for path_rel in DEADBEEF_PIN_FILES:
            post_sha = file_sha256(REPO_ROOT / path_rel)
            self.assertEqual(
                pre_snapshot[path_rel], post_sha,
                f"no-op dry-run modified file {path_rel}",
            )


if __name__ == "__main__":
    unittest.main(verbosity=2)

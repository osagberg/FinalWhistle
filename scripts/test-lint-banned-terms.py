#!/usr/bin/env python3
"""
test-lint-banned-terms.py — T1-20 acceptance tests for `lint-banned-terms.py`.

Tests three T1-20 changes to the lint script:

  - AC2: `/content/baked/` is no longer excluded from EXCLUDE_DIR_FRAGMENTS;
    a banned term placed under a synthetic `content/baked/x.ron` MUST be
    detected by the lint.
  - AC4: `ui-lint:ignore-start`/`-end` sentinel blocks are honored ONLY under
    `docs/**`, `crates/**`, `scripts/**`, and project-root `.md` files. A
    sentinel-wrapped banned term inside `content/sources/x.ron` (or any path
    outside the allowlist) MUST be detected; the same sentinel block inside
    `docs/x.md` MUST suppress the term.

Also includes a sanity test for AC1 (fw-content-baker `validate-structural`
rename) via the cargo binary — the test is Python-side here so the whole T1-20
acceptance surface runs under one invocation: `python3 scripts/test-lint-banned-terms.py`.

Usage:
    python3 scripts/test-lint-banned-terms.py        # runs all tests
    python3 scripts/test-lint-banned-terms.py -v     # verbose

Exit codes:
    0 - all tests pass
    1 - any test failed

This script uses stdlib unittest only — no third-party deps. Runs everywhere
Python 3.11+ runs (matching scripts/lint-banned-terms.py's Python target).
"""
from __future__ import annotations

import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent
LINT_SCRIPT = REPO_ROOT / "scripts" / "lint-banned-terms.py"

# A Category-A banned term that's guaranteed to be detected by lint-banned-terms.py.
# `Manchester` is in the A.5 catalog (real-world place analogue). Using it here
# means a positive-control regression: if the lint script's catalog ever drops
# `Manchester`, the test fails loudly + asks the maintainer to update the test.
BANNED_TERM = "Manchester"
BANNED_TERM_LABEL = "A.5 real-world place analogue"


def run_lint(scope: Path) -> subprocess.CompletedProcess[str]:
    """Run lint-banned-terms.py against `scope`; return CompletedProcess."""
    return subprocess.run(
        [sys.executable, str(LINT_SCRIPT), str(scope), "--root", str(scope)],
        capture_output=True,
        text=True,
        check=False,
    )


class T1_20_LintCoverage(unittest.TestCase):
    """AC2 — `/content/baked/` is scanned by the lint after the exclusion drops."""

    def test_baked_corpus_banned_term_is_detected(self) -> None:
        """A `Manchester` written into `content/baked/x.ron` MUST be detected."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baked_dir = tmp_path / "content" / "baked"
            baked_dir.mkdir(parents=True)
            offending = baked_dir / "x.ron"
            offending.write_text(
                f'// Synthetic baked artifact for T1-20 lint test.\n'
                f'(name: "Welcome to {BANNED_TERM} United")\n',
                encoding="utf-8",
            )

            result = run_lint(tmp_path)

            self.assertEqual(
                result.returncode,
                1,
                f"lint must exit 1 when a banned term sits under content/baked/; "
                f"got returncode={result.returncode}\n"
                f"stdout={result.stdout!r}\nstderr={result.stderr!r}",
            )
            self.assertIn(
                BANNED_TERM,
                result.stdout,
                f"lint output must name the {BANNED_TERM!r} hit; got stdout={result.stdout!r}",
            )

    def test_baked_corpus_clean_passes(self) -> None:
        """Sanity: a clean `content/baked/x.ron` MUST NOT trip the lint."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            baked_dir = tmp_path / "content" / "baked"
            baked_dir.mkdir(parents=True)
            clean = baked_dir / "x.ron"
            clean.write_text(
                '// Synthetic clean baked artifact for T1-20 lint test.\n'
                '(name: "Brookhaven United")\n',
                encoding="utf-8",
            )

            result = run_lint(tmp_path)

            self.assertEqual(
                result.returncode,
                0,
                f"lint must exit 0 on clean baked content; got returncode={result.returncode}\n"
                f"stdout={result.stdout!r}\nstderr={result.stderr!r}",
            )


class T1_20_SentinelScopeRestriction(unittest.TestCase):
    """AC4 — sentinels honored only under docs/Rust paths."""

    SENTINEL_WRAPPED = (
        "// ui-lint:ignore-start reason=\"test sentinel\"\n"
        f"This text contains {BANNED_TERM} — should be detected outside docs.\n"
        "// ui-lint:ignore-end\n"
    )

    def test_sentinel_inside_content_sources_does_not_suppress_banned_term(self) -> None:
        """A sentinel block inside `content/sources/x.ron` MUST be IGNORED.

        Security rationale: the linter strips sentinel blocks BEFORE matching,
        so without scope restriction an attacker could hide Category-A terms
        inside JSON/RON comment-bracketed sentinel pairs.
        """
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            sources_dir = tmp_path / "content" / "sources"
            sources_dir.mkdir(parents=True)
            attack = sources_dir / "attack.ron"
            attack.write_text(self.SENTINEL_WRAPPED, encoding="utf-8")

            result = run_lint(tmp_path)

            self.assertEqual(
                result.returncode,
                1,
                f"lint must detect banned term inside sentinel block under content/sources/; "
                f"got returncode={result.returncode}\n"
                f"stdout={result.stdout!r}\nstderr={result.stderr!r}",
            )
            self.assertIn(
                BANNED_TERM,
                result.stdout,
                f"lint output must name the {BANNED_TERM!r} hit; got stdout={result.stdout!r}",
            )

    def test_sentinel_inside_docs_suppresses_banned_term(self) -> None:
        """A sentinel block inside `docs/x.md` MUST suppress the banned term.

        This is the legitimate use-case the sentinel feature was designed for:
        meta-references inside banned-term catalogs at `docs/design/ui-vocabulary.md`
        and similar docs.
        """
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            docs_dir = tmp_path / "docs"
            docs_dir.mkdir(parents=True)
            catalog = docs_dir / "catalog.md"
            # Markdown sentinel uses HTML-comment form.
            catalog.write_text(
                "<!-- ui-lint:ignore-start reason=\"legitimate catalog entry\" -->\n"
                f"This documents that {BANNED_TERM} is a banned A.5 term.\n"
                "<!-- ui-lint:ignore-end -->\n",
                encoding="utf-8",
            )

            result = run_lint(tmp_path)

            self.assertEqual(
                result.returncode,
                0,
                f"lint must respect sentinel under docs/**; got returncode={result.returncode}\n"
                f"stdout={result.stdout!r}\nstderr={result.stderr!r}",
            )

    def test_sentinel_inside_frontend_does_not_suppress_banned_term(self) -> None:
        """A sentinel block inside `frontend/src/x.ts` MUST be IGNORED."""
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            frontend_dir = tmp_path / "frontend" / "src"
            frontend_dir.mkdir(parents=True)
            offender = frontend_dir / "x.ts"
            offender.write_text(self.SENTINEL_WRAPPED, encoding="utf-8")

            result = run_lint(tmp_path)

            self.assertEqual(
                result.returncode,
                1,
                f"lint must detect banned term inside sentinel block under frontend/src/; "
                f"got returncode={result.returncode}\n"
                f"stdout={result.stdout!r}\nstderr={result.stderr!r}",
            )

    # ------------------------------------------------------------------
    # T1-20 silent-failure-hunter P1 fix regression tests: sentinel scope
    # must anchor at the relative-path PREFIX, not substring-match against
    # the absolute path. The prior substring design accidentally honored
    # sentinels inside any path containing `/docs/` / `/crates/` / `/scripts/`
    # anywhere mid-path (e.g. `frontend/src/docs/legacy/x.ts`).
    # ------------------------------------------------------------------

    def test_sentinel_inside_nested_docs_under_frontend_does_not_suppress(self) -> None:
        """`frontend/src/docs/legacy/x.ts` MUST NOT honor sentinels.

        Even though the path contains the substring `docs/`, the relative-path
        PREFIX is `frontend/`, not `docs/`. Sentinels are inert here.
        """
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            nested = tmp_path / "frontend" / "src" / "docs" / "legacy"
            nested.mkdir(parents=True)
            offender = nested / "x.ts"
            offender.write_text(self.SENTINEL_WRAPPED, encoding="utf-8")

            result = run_lint(tmp_path)

            self.assertEqual(
                result.returncode,
                1,
                f"lint must detect banned term inside sentinel block at "
                f"frontend/src/docs/legacy/x.ts; got returncode={result.returncode}\n"
                f"stdout={result.stdout!r}\nstderr={result.stderr!r}",
            )

    def test_sentinel_inside_nested_scripts_under_content_does_not_suppress(self) -> None:
        """`content/sources/scripts/x.ron` MUST NOT honor sentinels.

        Same shape as the nested-`docs/` test: the substring `scripts/` appears
        mid-path but the relative-path PREFIX is `content/`. Sentinels inert.
        """
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            nested = tmp_path / "content" / "sources" / "scripts"
            nested.mkdir(parents=True)
            offender = nested / "x.ron"
            offender.write_text(self.SENTINEL_WRAPPED, encoding="utf-8")

            result = run_lint(tmp_path)

            self.assertEqual(
                result.returncode,
                1,
                f"lint must detect banned term inside sentinel block at "
                f"content/sources/scripts/x.ron; got returncode={result.returncode}\n"
                f"stdout={result.stdout!r}\nstderr={result.stderr!r}",
            )

    def test_sentinel_inside_top_level_docs_subdir_still_suppresses(self) -> None:
        """Control: `docs/design/x.md` MUST continue to honor sentinels.

        This is the legitimate use-case the rule allows. Pins the relative-path
        prefix check doesn't over-tighten and break docs/ subdirectories.
        """
        with tempfile.TemporaryDirectory() as tmp:
            tmp_path = Path(tmp)
            nested = tmp_path / "docs" / "design"
            nested.mkdir(parents=True)
            catalog = nested / "x.md"
            catalog.write_text(
                "<!-- ui-lint:ignore-start reason=\"design doc meta-reference\" -->\n"
                f"This documents that {BANNED_TERM} is A.5 banned.\n"
                "<!-- ui-lint:ignore-end -->\n",
                encoding="utf-8",
            )

            result = run_lint(tmp_path)

            self.assertEqual(
                result.returncode,
                0,
                f"lint must respect sentinels under docs/design/x.md (legitimate use-case); "
                f"got returncode={result.returncode}\n"
                f"stdout={result.stdout!r}\nstderr={result.stderr!r}",
            )


class T1_20_ContentBakerValidateRename(unittest.TestCase):
    """AC1 — `fw-content-baker validate` renamed `validate-structural`."""

    def test_validate_structural_subcommand_exists_and_succeeds(self) -> None:
        """`cargo run -p fw-content-baker -- validate-structural` exits 0 with STRUCTURAL output."""
        result = subprocess.run(
            ["cargo", "run", "--quiet", "-p", "fw-content-baker", "--",
             "--workspace", str(REPO_ROOT), "validate-structural"],
            capture_output=True,
            text=True,
            check=False,
            cwd=REPO_ROOT,
        )
        self.assertEqual(
            result.returncode,
            0,
            f"validate-structural must exit 0; got returncode={result.returncode}\n"
            f"stdout={result.stdout!r}\nstderr={result.stderr!r}",
        )
        self.assertIn(
            "STRUCTURAL",
            result.stdout,
            f"output must mention STRUCTURAL to be honest about scope; got stdout={result.stdout!r}",
        )

    def test_old_validate_subcommand_is_rejected(self) -> None:
        """`cargo run -p fw-content-baker -- validate` MUST be rejected (clap unknown subcommand)."""
        result = subprocess.run(
            ["cargo", "run", "--quiet", "-p", "fw-content-baker", "--", "validate"],
            capture_output=True,
            text=True,
            check=False,
            cwd=REPO_ROOT,
        )
        self.assertNotEqual(
            result.returncode,
            0,
            f"old `validate` subcommand must be rejected after T1-20 rename; "
            f"got returncode=0\nstdout={result.stdout!r}\nstderr={result.stderr!r}",
        )
        self.assertIn(
            "validate-structural",
            result.stderr,
            f"clap should tip the user toward `validate-structural`; got stderr={result.stderr!r}",
        )


if __name__ == "__main__":
    unittest.main(verbosity=2)

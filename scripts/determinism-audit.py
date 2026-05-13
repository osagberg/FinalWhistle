#!/usr/bin/env python3
"""Determinism audit for Final Whistle sim crates.

Static-grep companion to clippy lints. Catches violations of
.claude/rules/Sim/RULES.md (HashMap, tokio, clocks, system RNG) in code
positions — strips line/block comments first so the rule docs that
discuss the bans don't false-positive.

Exit 0 on clean. Exit 1 on any violation (prints file:line:rule).

Codex pre-T0 audit follow-up — wired into `just determinism-audit` +
`scripts/fw determinism-audit` + `just ci-local`.
"""

from __future__ import annotations
import re
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent

SIM_CRATES = [
    "crates/fw-core",
    "crates/fw-match-sim",
    "crates/fw-memory",
    "crates/fw-replay",
    "crates/fw-save",
    "crates/fw-content",
    "crates/fw-scouting",
]

# (rule-name, regex, applies-to-these-crates-or-None-for-all)
# Patterns match CODE positions — `use`, `::`, `:`, `<` — not prose.
RULES: list[tuple[str, re.Pattern[str], list[str] | None]] = [
    (
        "Sim/RULES.md §2 — HashMap in code",
        re.compile(r"\b(use\s+\S*::HashMap\b|HashMap::|:\s*HashMap\s*<|<HashMap\s*<|->\s*HashMap\b|FxHashMap|AHashMap|DashMap)"),
        None,
    ),
    (
        "Sim/RULES.md §2 — HashSet in code",
        re.compile(r"\b(use\s+\S*::HashSet\b|HashSet::|:\s*HashSet\s*<|<HashSet\s*<|->\s*HashSet\b|FxHashSet|AHashSet)"),
        None,
    ),
    (
        "Sim/RULES.md §5 — async/tokio in sim or memory",
        re.compile(r"(\btokio::|use\s+tokio\b|\basync\s+fn\b|\.await\b|use\s+futures\b)"),
        ["crates/fw-match-sim", "crates/fw-memory"],
    ),
    (
        "Sim/RULES.md §3 — Instant::now / SystemTime::now",
        re.compile(r"(Instant::now\s*\(|SystemTime::now\s*\(|UNIX_EPOCH)"),
        None,
    ),
    (
        "Sim/RULES.md §4 — thread_rng / OsRng / getrandom",
        re.compile(r"(\bthread_rng\s*\(|\bOsRng\b|\bgetrandom\s*\()"),
        None,
    ),
    (
        "Sim/RULES.md §1 — f32/f64 type (denied by clippy::float_arithmetic but type can still appear)",
        # Allow it in fw-content (loaded content has documented escape — TacticalArchetype.buildup_speed_factor)
        re.compile(r"(:\s*f(32|64)\b|->\s*f(32|64)\b|<f(32|64)>|as\s+f(32|64)\b)"),
        # Allowed in fw-content (documented content-load escape hatch).
        ["crates/fw-core", "crates/fw-match-sim", "crates/fw-memory", "crates/fw-replay", "crates/fw-save", "crates/fw-scouting"],
    ),
]


def strip_comments(source: str) -> str:
    """Remove // line comments and /* block comments */ — leaves whitespace
    so line numbers stay aligned for reporting."""
    # Strip /* ... */ first (greedy across lines, but minimal-match).
    source = re.sub(r"/\*.*?\*/", lambda m: "\n" * m.group(0).count("\n"), source, flags=re.DOTALL)
    # Strip // ... to end-of-line. Doc comments (`///` and `//!`) also get
    # stripped — they're prose, not code, and any code-shaped match inside
    # them would be a false positive for this audit.
    source = re.sub(r"//[^\n]*", "", source)
    return source


def audit_file(path: Path, crate_dir: str) -> list[tuple[int, str, str]]:
    """Return list of (line_number, rule_name, line_snippet) violations."""
    try:
        text = path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError):
        return []
    stripped = strip_comments(text)
    violations: list[tuple[int, str, str]] = []
    for rule_name, pattern, applies_to in RULES:
        if applies_to is not None and crate_dir not in applies_to:
            continue
        for line_no, line in enumerate(stripped.split("\n"), 1):
            if pattern.search(line):
                # Original line for the report (with any comment context).
                original = text.split("\n")[line_no - 1] if line_no - 1 < len(text.split("\n")) else line
                violations.append((line_no, rule_name, original.strip()[:120]))
    return violations


def main() -> int:
    violations: list[tuple[Path, int, str, str]] = []
    for crate in SIM_CRATES:
        crate_path = REPO_ROOT / crate
        if not crate_path.exists():
            continue
        for rs_file in crate_path.rglob("*.rs"):
            # Skip target/ build artifacts.
            if "target" in rs_file.parts:
                continue
            # File-level exemptions for documented Q32 → f64 viewer-side
            # projection modules. These are NOT canonical-state code —
            # they project canonical state out to JSON for the renderer.
            # Each exemption MUST have an `#![allow(clippy::float_arithmetic)]`
            # at the module head + a `Float boundary` comment block
            # explaining the one-way contract.
            rel = rs_file.relative_to(REPO_ROOT)
            EXEMPT_FILES = {
                # T1-2a (per ADR-0007 + ADR-0008): MatchFrameDto is the
                # renderer-side per-tick projection consumed by the
                # dev-tier 2D tactical board AND by the dump_frames
                # binary. The Q32 → f64 cast is the only float
                # arithmetic; nothing reads it back into the sim.
                Path("crates/fw-match-sim/src/dto.rs"),
            }
            if rel in EXEMPT_FILES:
                continue
            for line_no, rule, snippet in audit_file(rs_file, crate):
                violations.append((rs_file.relative_to(REPO_ROOT), line_no, rule, snippet))

    if not violations:
        print("determinism-audit: clean")
        return 0

    print(f"determinism-audit: {len(violations)} violation(s)")
    print("-" * 60)
    for path, line_no, rule, snippet in violations:
        print(f"{path}:{line_no}: [{rule}]")
        print(f"  {snippet}")
    print("-" * 60)
    print("Fix by replacing with the determinism-safe equivalent:")
    print("  HashMap   -> BTreeMap (sorted iteration)")
    print("  HashSet   -> BTreeSet")
    print("  async fn  -> sync; route async through fw-tauri")
    print("  *::now()  -> Tick supplied by the caller")
    print("  thread_rng -> ChaCha8Rng::seed_from_u64(seed_fn(...))")
    print("  f32/f64   -> Q32 newtype from fw-core")
    return 1


if __name__ == "__main__":
    sys.exit(main())

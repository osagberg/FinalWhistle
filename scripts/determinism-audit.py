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


def strip_cfg_test_blocks(source: str) -> str:
    """Replace #[cfg(test)] mod blocks with blank lines (preserving line count).

    Test helpers are allowed to use f64 for convenient literal construction;
    they never participate in canonical state. This keeps the audit focused on
    production code paths.
    """
    lines = source.split("\n")
    out: list[str] = []
    depth = 0       # brace depth inside the cfg(test) region
    in_test = False # True while scanning inside #[cfg(test)] mod
    skip_next = False  # True after we see the #[cfg(test)] line

    i = 0
    while i < len(lines):
        line = lines[i]
        stripped = line.strip()

        if skip_next:
            # This line should be `mod tests {` or similar
            if "{" in stripped:
                depth = stripped.count("{") - stripped.count("}")
                in_test = True
                out.append("")  # blank out
                skip_next = False
                i += 1
                continue
            # Edge case: attribute on next non-blank line
            out.append("")
            skip_next = False
            i += 1
            continue

        if in_test:
            depth += stripped.count("{") - stripped.count("}")
            out.append("")  # blank out the line
            if depth <= 0:
                in_test = False
                depth = 0
            i += 1
            continue

        # Detect #[cfg(test)]
        if re.match(r"#\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]", stripped):
            out.append("")  # blank out the attribute line
            skip_next = True
            i += 1
            continue

        out.append(line)
        i += 1

    return "\n".join(out)


FLOAT_RULE_NAME = "Sim/RULES.md §1 — f32/f64 type (denied by clippy::float_arithmetic but type can still appear)"

# Per-rule file exemptions (P2-12 fix: tighten from file-level to rule-level).
# Maps relative Path → set of rule names to skip for that file.
# A file here is NOT fully skipped — only the listed rules are suppressed.
# Use the full rule name strings from RULES above.
PER_RULE_EXEMPT: dict[Path, set[str]] = {
    # T1-10 (Codex 2026-05-16 audit P1 closure): math.rs is now PURE Q32 — no
    # runtime f64 bake. The prior LazyLock bake using f64::exp() was replaced
    # with committed const tables in math_luts.rs (also pure i64, no f64).
    # The math.rs file-level exemption is REMOVED entirely; if f64 reappears
    # in math.rs it should fail the audit.
    #
    # q32.rs contains one f64-bearing function: from_f64_clamped (T1-10
    # promoted to `pub` for integration-test drift detection; called ONLY at
    # bake time, never on canonical paths). Exempt from the f64 rule ONLY.
    Path("crates/fw-core/src/q32.rs"): {FLOAT_RULE_NAME},
    # T2-R7(c) — post-T2 Codex Track E-4 fix: calibrate binary moved from
    # FULLY_EXEMPT_FILES to per-rule float-only exemption. The corpus
    # runner (in this same file) MUST stay covered by the HashMap / time /
    # RNG bans so a future edit can't silently add SystemTime::now() or
    # thread_rng() to the corpus-collection path without tripping the
    # audit. Only the offline Newton-Raphson f64 fit is rule-exempted.
    Path("crates/fw-match-sim/src/bin/calibrate.rs"): {FLOAT_RULE_NAME},
    # DX-2: inspect_frames and render_contact_sheet are viewer-side analysis
    # tools (pure functions of dump_frames JSON — never canonical state). Both
    # carry #![allow(clippy::float_arithmetic)] at the module head and are
    # exempt from the float rule for the same reason as dto.rs. HashMap / time /
    # RNG bans stay active on these files.
    Path("crates/fw-match-sim/src/bin/inspect_frames.rs"): {FLOAT_RULE_NAME},
    Path("crates/fw-match-sim/src/bin/render_contact_sheet.rs"): {FLOAT_RULE_NAME},
    # T1-10: integration test that re-bakes the math LUTs via f64 + asserts
    # equality with the committed const tables in src/math_luts.rs. f64 is
    # intentional + bake-time-only; the test is `#[ignore]`-gated so it
    # doesn't run by default.
    Path("crates/fw-core/tests/lut_drift_detection.rs"): {FLOAT_RULE_NAME},
    # T1-10 self-review fix-pass per silent-failure-hunter P3 + code-reviewer
    # P2: the printer test was originally self-referential (read committed
    # const + printed it back), so a libm drift would yield STALE values not
    # NEW f64-baked truth. Rewrote to re-bake from f64 — same path as the
    # drift-detection test — making the printer the authoritative
    # regeneration source on the current toolchain. f64 is intentional +
    # bake-time-only; `#[ignore]`-gated.
    Path("crates/fw-core/tests/print_luts_oneshot.rs"): {FLOAT_RULE_NAME},
    # T3-1: perf_test.rs is the single documented exception to Sim/RULES.md §3
    # (no clocks). `Instant::now()` is the explicit purpose of the test —
    # measuring wall-clock elapsed for a 1000-event encode+decode round-trip.
    # fw-save tests are non-sim, non-canonical-state. `#[ignore]`-gated so it
    # never runs on the default `cargo test` sweep.
    Path("crates/fw-save/tests/perf_test.rs"): {"Sim/RULES.md §3 — Instant::now / SystemTime::now"},
}

# Fully-exempt files: ALL rules are suppressed. Reserved for renderer-side
# projection modules that are not canonical-state code and carry their own
# documented float-boundary contract.
FULLY_EXEMPT_FILES: set[Path] = {
    # T1-2a (per ADR-0007 + ADR-0008): MatchFrameDto is the renderer-side
    # per-tick projection consumed by the dev-tier 2D tactical board AND by
    # the dump_frames binary. The Q32 → f64 cast is the only float arithmetic;
    # nothing reads it back into the sim. Has #![allow(clippy::float_arithmetic)]
    # at the module head.
    Path("crates/fw-match-sim/src/dto.rs"),
    # T2-R7(c) note: calibrate.rs was here originally (T2-1d) as a
    # full-file exemption. Codex Track E-4 of the post-T2 ultimate-review
    # caught that as an audit blind-spot: a future edit could add
    # SystemTime::now() or HashMap to the corpus-collection path + the
    # script would still report clean. Moved to PER_RULE_EXEMPT above with
    # FLOAT_RULE_NAME only; HashMap / time / RNG bans stay active.
}


def audit_file(path: Path, crate_dir: str, per_rule_exempt: set[str]) -> list[tuple[int, str, str]]:
    """Return list of (line_number, rule_name, line_snippet) violations.

    `per_rule_exempt` is the set of rule names to skip for this specific file.
    """
    try:
        text = path.read_text(encoding="utf-8")
    except (UnicodeDecodeError, OSError) as e:
        # T2-R-C8 (post-T2 ultimate-review Track C-8): the prior shape
        # `except ...: return []` silently treated unreadable / non-UTF8
        # files as having zero violations. determinism-audit is the
        # LAST line of defense for Sim/RULES.md's HashMap / clock /
        # RNG / float bans — silent skip-on-unreadable lets a banned
        # pattern slip into a file that briefly errored during walk.
        # Fail-loud + exit 1 so the operator + CI both surface the
        # cause rather than passing a partial-walk audit as clean.
        print(
            f"ERROR: determinism-audit cannot read {path}: {e}",
            file=sys.stderr,
        )
        sys.exit(1)
    # Strip cfg(test) blocks first (test helpers may use f64 literals — that
    # is allowed and does not touch canonical state).  Then strip comments so
    # rule-doc prose mentioning banned terms doesn't false-positive.
    stripped = strip_cfg_test_blocks(strip_comments(text))
    violations: list[tuple[int, str, str]] = []
    for rule_name, pattern, applies_to in RULES:
        if applies_to is not None and crate_dir not in applies_to:
            continue
        if rule_name in per_rule_exempt:
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
            rel = rs_file.relative_to(REPO_ROOT)
            # Full-file exemptions: renderer-side projection modules with their
            # own float-boundary contract. Skip ALL rules.
            if rel in FULLY_EXEMPT_FILES:
                continue
            # Per-rule exemptions: suppress only the listed rules for this file.
            per_rule_exempt = PER_RULE_EXEMPT.get(rel, set())
            for line_no, rule, snippet in audit_file(rs_file, crate, per_rule_exempt):
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

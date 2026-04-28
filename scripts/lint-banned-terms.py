#!/usr/bin/env python3
"""
lint-banned-terms.py — Final Whistle banned-vocabulary lint.

Phase-1 SPEC task. Banned-term source is `design/ui-vocabulary.md` Categories
A.1-A.5 (hard bans, no exemption except sentinel blocks) and Category B (soft
bans, inline `ui-lint:allow` exemption).

Usage:
    scripts/lint-banned-terms.py [paths...]
    scripts/lint-banned-terms.py --report   # emit JSON of active Category-B
                                            # exemptions for EA/RC audit

Exit codes:
    0 - clean
    1 - Category-A hit (hard-ban violation) OR malformed exemption
    2 - invalid CLI

Design notes (cross-ref: design/ui-vocabulary.md 2026-04-24 resolution,
memory file feedback_placeholder_lint_strategy.md):
  - Category-A matches are CASE-SENSITIVE with word boundaries. "Canon" is
    banned; "canonical state" (MatchSim technical term) is unaffected.
  - Sentinel blocks `<!-- ui-lint:ignore-start reason="..." --> ...
    <!-- ui-lint:ignore-end -->` skip their content entirely. This is the
    ONE legitimate exemption path for Category-A — used for banned-term
    catalogs, historical meta-references, and checklist reminders.
  - Category-B requires an inline `ui-lint:allow term="X" reason="..."
    reviewer="..."` exemption on the SAME LINE as the match. All three
    attributes required; missing any is a lint fail (malformed exemption).
  - Excludes .claude/bootstrap (source-of-truth templates) and
    design-templates (literal templates).
"""
from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

# ---------------------------------------------------------------------------
# Banned-term catalog. Source of truth: design/ui-vocabulary.md §Category A.
# When that catalog changes, update here too; cross-check in CI via spot-grep.
# ---------------------------------------------------------------------------

# ui-lint:ignore-start reason="banned-term pattern definitions (lint source-of-truth)"
# Category A — hard ban, no inline exemption. Sentinel blocks skip.
# Each entry: (pattern, human label).
# Patterns use \b word boundaries to avoid false positives on compound words.
CATEGORY_A_PATTERNS: list[tuple[re.Pattern[str], str]] = [
    # A.1 — mystical / RPG / fantasy capitalized state nouns
    (re.compile(r"\bThe Hush\b"), "A.1 mystical state noun"),
    (re.compile(r"\bWeather\b(?=\s+(?:rising|falling|shift))"), "A.1 Weather as team state"),
    (re.compile(r"\bThe Seven\b"), "A.1 The Seven as rival-manager system"),
    (re.compile(r"\bKismet\b"), "A.1 mystical state noun"),
    (re.compile(r"\bThe Author\b"), "A.1 manager identity framing"),
    (re.compile(r"\bThe Ledger\b"), "A.1 UI noun (internal only)"),
    # "Canon", "Calling", "Soul", "Flow" as bare capitalized state nouns are
    # tricky — they legitimately appear in other contexts. We match only when
    # used as a surfaced UI noun (followed by colon or at start of sentence
    # context). Conservative patterns here; expand if false negatives appear.
    (re.compile(r"(?<![a-zA-Z])Canon(?=\s*[:.]|\s+tier|\s+Shelves|\s+Reading)"), "A.1 Canon as pyramid-tier noun"),
    (re.compile(r"(?<![a-zA-Z])Calling(?=\s*[:.]|\s+unlocked|\s+awakens)"), "A.1 Calling as player-identity noun"),
    (re.compile(r"\bReading Lists?\b"), "A.1 Reading Lists as pyramid tier"),
    # A.2 — system / progression / menu-game vocabulary
    (re.compile(r"\bSignature unlocked\b", re.IGNORECASE), "A.2 progression vocabulary"),
    (re.compile(r"\bAwakened\b(?![_a-z])"), "A.2 Awakened as capitalized noun/verb"),
    (re.compile(r"\bXP gained\b", re.IGNORECASE), "A.2 progression vocabulary"),
    (re.compile(r"\bLevel up\b", re.IGNORECASE), "A.2 progression vocabulary"),
    (re.compile(r"\bSkill point\b", re.IGNORECASE), "A.2 progression vocabulary"),
    (re.compile(r"\+\d+\s+(?:finishing|pace|passing|tackling|shooting|dribbling|strength)\b", re.IGNORECASE), "A.2 stat-delta callout"),
    # A.3 — genetics / bloodline
    (re.compile(r"\bGenetics\b"), "A.3 genetics vocabulary"),
    (re.compile(r"\bChromosomes?\b"), "A.3 genetics vocabulary"),
    (re.compile(r"\bBloodline\b(?!\s+mechanics|\s+deferred)"), "A.3 bloodline as mechanic"),
    # "Genes" and "DNA" are also banned but appear legitimately in internal
    # design-doc technical prose (e.g., "internal gene model"); we catch
    # the PLAYER-FACING risky forms only. Wider patterns cause too many
    # false positives at Phase 1.
    (re.compile(r"\bGene\s+Score\b"), "A.3 genetics UI surface"),
    (re.compile(r"\bDNA\s+(?:Score|Rating|Level)\b"), "A.3 genetics UI surface"),
    # A.4 — stigmatizing / systemic phenotype framings
    (re.compile(r"\bFragile Under Scrutiny\b"), "A.4 stigmatizing phenotype (use Struggles Under Scrutiny)"),
    (re.compile(r"\bFragile When Tested\b"), "A.4 stigmatizing phenotype (use Struggles Under Scrutiny)"),
    (re.compile(r"\bPlateau Risk\b"), "A.4 systemic phenotype (removed from enum)"),
    (re.compile(r"\bInjury-Prone\b"), "A.4 stigmatizing phenotype"),
    # "Powerful Striker" is banned AS A PHENOTYPE LABEL — but "powerful ball
    # striker" is the approved replacement. Word-boundary + capital-S match
    # catches the banned form.
    (re.compile(r"\bPowerful Striker\b(?!\s)"), "A.4 use Powerful Ball Striker"),
    # A.5 — real-world place-name analogues (runtime content only; design-doc
    # meta-references must be sentinel-wrapped)
    (re.compile(r"\bManchester\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bLiverpool\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bLeeds\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bLondon\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bCardiff\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bBristol\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bBrighton\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bSouthampton\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bNewcastle\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bEdinburgh\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bNorwich\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bHull\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bBirmingham\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bNottingham\b"), "A.5 real-world place analogue"),
    (re.compile(r"\bJersey\b(?!\s+(?:kit|number|shirt))"), "A.5 real-world place analogue"),
    (re.compile(r"\bIsle of Man\b"), "A.5 real-world place analogue"),
]

# Category B — soft ban. Inline `ui-lint:allow term="X" reason="Y" reviewer="Z"`
# on the same line exempts the match.
CATEGORY_B_TERMS: dict[str, re.Pattern[str]] = {
    "awakens":    re.compile(r"\bawakens?\b"),
    "awakened":   re.compile(r"\bawakened\b"),
    "savant":     re.compile(r"\bSavants?\b"),
    "genius":     re.compile(r"\bGenius\b"),
    "weapon":     re.compile(r"\bweapons?\b", re.IGNORECASE),
    "weaponize":  re.compile(r"\bweaponize[sd]?\b", re.IGNORECASE),
    "egoist":     re.compile(r"\bEgoists?\b"),
    "the-ego":    re.compile(r"\bThe Ego\b"),
    "realm":      re.compile(r"\bRealms?\b"),
    "domain":     re.compile(r"\bDomains?\b(?!\.)"),  # not ".com/.net domains"
    "kingdom":    re.compile(r"\bKingdoms?\b"),
    "power-level": re.compile(r"\bpower[- ]?levels?\b", re.IGNORECASE),
    "forge":      re.compile(r"\bForged?\b"),
}
# ui-lint:ignore-end

# Sentinel block markers — HTML-comment syntax (markdown) OR // (C#/Python/JS).
SENTINEL_START = re.compile(r"(?:<!--|//|#)\s*ui-lint:ignore-start(?:\s+reason=\"[^\"]*\")?\s*(?:-->)?")
SENTINEL_END = re.compile(r"(?:<!--|//|#)\s*ui-lint:ignore-end\s*(?:-->)?")

# Inline exemption — must have all three attributes or it's malformed.
EXEMPTION_RE = re.compile(
    r"""ui-lint:allow\s+
        term=\"(?P<term>[^\"]+)\"\s+
        reason=\"(?P<reason>[^\"]+)\"\s+
        reviewer=\"(?P<reviewer>[^\"]+)\"""",
    re.VERBOSE,
)

# Path filters — never lint these.
# Scope is ACTIVE shipped-or-near-shipped content: root project docs, active
# design docs (NOT brainstorm/, which is historical archive per
# design/README.md:69-70), game code (Phase 3+), content packs (Phase 6+),
# .github/ PR-infra. Not: .claude/ dev scaffolding, design-templates/,
# Unity regenerable caches, node_modules.
EXCLUDE_DIR_FRAGMENTS = (
    "/.git/",
    "/.claude/",                  # all Claude infra (commands/skills/agents/hooks/rules/bootstrap)
    "/design/brainstorm/",        # historical ideation archive; not binding spec
    "/design-templates/",         # literal templates
    "/node_modules/",
    "/Library/",                  # Unity package cache
    "/Temp/",                     # Unity transient
    "/Obj/",                      # Unity build intermediate
    "/Logs/",                     # Unity logs
    "/UserSettings/",             # Unity per-user editor state
    "/MemoryCaptures/",           # Unity memory profiler dumps
    "/Build/",                    # local Unity builds
    "/Builds/",                   # local Unity builds (plural variant)
)

# File extensions in scope.
LINT_EXTENSIONS = {".md", ".cs", ".json", ".uxml", ".uss", ".sh", ".yml", ".yaml", ".py"}


# ---------------------------------------------------------------------------
# Data classes
# ---------------------------------------------------------------------------

@dataclass
class Finding:
    path: Path
    line_num: int
    category: str
    term: str
    snippet: str

    def format(self, root: Path) -> str:
        rel = self.path.relative_to(root) if self.path.is_relative_to(root) else self.path
        return f"{rel}:{self.line_num}: [{self.category}] {self.term} — {self.snippet.strip()[:100]}"


@dataclass
class Exemption:
    path: Path
    line_num: int
    term: str
    reason: str
    reviewer: str


@dataclass
class Report:
    findings: list[Finding] = field(default_factory=list)
    exemptions: list[Exemption] = field(default_factory=list)
    malformed_exemptions: list[Finding] = field(default_factory=list)

    @property
    def has_violations(self) -> bool:
        return bool(self.findings or self.malformed_exemptions)


# ---------------------------------------------------------------------------
# Core lint
# ---------------------------------------------------------------------------

def should_lint(path: Path) -> bool:
    if not path.is_file():
        return False
    if path.suffix not in LINT_EXTENSIONS:
        return False
    path_str = str(path) + "/"
    return not any(frag in path_str for frag in EXCLUDE_DIR_FRAGMENTS)


def strip_sentinel_blocks(lines: list[str]) -> list[tuple[int, str]]:
    """Return (original_line_num, line_text) for lines OUTSIDE sentinel blocks."""
    result = []
    in_block = False
    for idx, line in enumerate(lines, start=1):
        if SENTINEL_START.search(line):
            in_block = True
            continue
        if SENTINEL_END.search(line):
            in_block = False
            continue
        if not in_block:
            result.append((idx, line))
    return result


def lint_file(path: Path, report: Report) -> None:
    try:
        text = path.read_text(encoding="utf-8", errors="replace")
    except OSError:
        return
    lines = text.splitlines()
    active_lines = strip_sentinel_blocks(lines)

    for line_num, line in active_lines:
        # Check Category-A hits first.
        for pattern, label in CATEGORY_A_PATTERNS:
            m = pattern.search(line)
            if m:
                report.findings.append(Finding(
                    path=path,
                    line_num=line_num,
                    category=label,
                    term=m.group(0),
                    snippet=line,
                ))

        # Collect inline exemptions declared on this line.
        line_exemptions: set[str] = set()
        for em in EXEMPTION_RE.finditer(line):
            term = em.group("term")
            reason = em.group("reason")
            reviewer = em.group("reviewer")
            if not (term and reason and reviewer):
                report.malformed_exemptions.append(Finding(
                    path=path, line_num=line_num,
                    category="exemption", term="(malformed)",
                    snippet=line,
                ))
            else:
                line_exemptions.add(term.lower())
                report.exemptions.append(Exemption(
                    path=path, line_num=line_num,
                    term=term, reason=reason, reviewer=reviewer,
                ))

        # Check Category-B — any match without a same-line exemption is a finding.
        for term_key, pattern in CATEGORY_B_TERMS.items():
            m = pattern.search(line)
            if m:
                if term_key not in line_exemptions:
                    report.findings.append(Finding(
                        path=path,
                        line_num=line_num,
                        category=f"B ({term_key})",
                        term=m.group(0),
                        snippet=line,
                    ))


def walk(root: Path, targets: list[Path]) -> list[Path]:
    """Expand target paths into a flat list of files to lint."""
    files: list[Path] = []
    for t in targets:
        if t.is_file():
            if should_lint(t):
                files.append(t)
        elif t.is_dir():
            for p in t.rglob("*"):
                if should_lint(p):
                    files.append(p)
    return sorted(set(files))


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(
        description="Final Whistle banned-terms lint (Phase-1 SPEC task).",
    )
    parser.add_argument(
        "paths", nargs="*", default=["."],
        help="Files or directories to lint (default: current directory)",
    )
    parser.add_argument(
        "--report", action="store_true",
        help="Emit JSON of active Category-B exemptions for EA/RC audit",
    )
    parser.add_argument(
        "--root", default=".",
        help="Repo root for relative-path reporting (default: cwd)",
    )
    args = parser.parse_args()

    root = Path(args.root).resolve()
    targets = [Path(p).resolve() for p in args.paths]

    report = Report()
    for f in walk(root, targets):
        lint_file(f, report)

    if args.report:
        data = [
            {
                "file": str(e.path.relative_to(root)) if e.path.is_relative_to(root) else str(e.path),
                "line": e.line_num,
                "term": e.term,
                "reason": e.reason,
                "reviewer": e.reviewer,
            }
            for e in report.exemptions
        ]
        print(json.dumps({"exemptions": data}, indent=2))
        return 0

    # Normal lint mode
    if report.findings:
        print(f"BANNED-TERMS LINT: {len(report.findings)} violation(s)")
        print("-" * 60)
        for f in report.findings:
            print(f.format(root))

    if report.malformed_exemptions:
        print()
        print(f"BANNED-TERMS LINT: {len(report.malformed_exemptions)} malformed exemption(s)")
        print("(exemption comments must have term=, reason=, reviewer= — all three)")
        print("-" * 60)
        for f in report.malformed_exemptions:
            print(f.format(root))

    if report.exemptions:
        print()
        print(f"Category-B exemptions granted: {len(report.exemptions)}")
        print("(run with --report for the full audit JSON)")

    if report.has_violations:
        return 1

    print("banned-terms lint: clean")
    return 0


if __name__ == "__main__":
    sys.exit(main())

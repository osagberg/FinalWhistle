#!/usr/bin/env bash
# canonical-hash-guard.sh — PreToolUse hook on Bash(git commit*).
#
# Defensive auto-rollback for the pinned canonical-state hash regression
# test. If the staged diff includes any change to canonical-state-bearing
# Rust source (fw-core, fw-match-sim, fw-memory, fw-replay, fw-save,
# fw-content) AND the targeted determinism test would fail post-commit,
# this hook BLOCKS the commit with exit 2.
#
# Ported from FW v1's .claude/hooks/canonical-hash-guard.sh, retargeted
# from `dotnet test` → `cargo test` and from MatchSim/{Sim,Content} →
# the six Rust canonical-state crates.
#
# Per docs/specs/determinism-gate.md §12: this hook is one of the four
# Phase-0 acceptance-gate boxes. Even if subagent review missed a hash-
# affecting change, the structural floor here prevents silent drift from
# landing.
#
# Hook contract:
#   - PreToolUse on Bash(git commit*).
#   - Reads tool input as JSON on stdin.
#   - Exits 0 if commit is allowed.
#   - Exits 2 with stderr message if commit is blocked.
#
# Performance budget: ~5-12s on a warm cargo cache. Cold cache is slower;
# the hook is designed to be skippable (with stderr warning) when the
# user has explicitly authorized a re-baseline via --no-verify + a
# matching DECISIONS.md entry. See block-message escape-hatch text.

set -euo pipefail

INPUT="$(cat)"

if ! command -v jq >/dev/null 2>&1; then
    printf 'canonical-hash-guard.sh: jq not found; allowing commit\n' >&2
    exit 0
fi

# Parse the tool input. We only inspect Bash tool calls whose command
# starts with "git commit".
TOOL_NAME="$(printf '%s' "$INPUT" | jq -r '.tool_name // ""')"
COMMAND="$(printf '%s' "$INPUT" | jq -r '.tool_input.command // ""')"

if [ "$TOOL_NAME" != "Bash" ]; then exit 0; fi
case "$COMMAND" in
    git\ commit*|*\ git\ commit*)
        ;;
    *)
        exit 0
        ;;
esac

# Locate the repo root so this hook works regardless of CWD.
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || true)"
if [ -z "$REPO_ROOT" ]; then exit 0; fi
cd "$REPO_ROOT"

# Check if the staged diff touches any canonical-state-bearing crate.
#
# In scope (must trigger the guard):
#   - `src/` under fw-core / fw-match-sim / fw-memory / fw-replay /
#     fw-save / fw-content (canonical-state-emitting code)
#   - crates/fw-replay/tests/canonical_hash.rs (the test file itself —
#     Codex audit P0, 2026-05-13: editing the pinned constant, removing
#     the bedrock test, or adding `#[ignore]` to it MUST trigger the
#     guard; previously these slipped through because only `src/`
#     was watched)
#   - crates/fw-replay/fixtures/** (the corpus RON files — editing
#     `expected_hash` alongside re-baselining MUST go through the gate)
#
# Out of scope (still exempt):
#   - tests under other crates (they assert on canonical state but don't
#     emit it; adding a new proptest invariant doesn't drift the hash)
#   - non-canonical-state code in any crate's src/ (e.g. fw-tauri,
#     fw-scouting, fw-content-baker)
TOUCHES_CANONICAL=0
if git diff --cached --name-only 2>/dev/null \
    | grep -E '^crates/(fw-core|fw-match-sim|fw-memory|fw-replay|fw-save|fw-content)/src/|^crates/fw-replay/tests/canonical_hash\.rs$|^crates/fw-replay/fixtures/' \
    >/dev/null 2>&1; then
    TOUCHES_CANONICAL=1
fi

if [ "$TOUCHES_CANONICAL" -eq 0 ]; then
    # No canonical-state-bearing change. Allow.
    exit 0
fi

# Cargo is required. If missing, the hook can't run — fail open with a
# stderr warning so the user knows to install cargo or repair PATH,
# rather than silently disabling the gate.
if ! command -v cargo >/dev/null 2>&1; then
    printf 'canonical-hash-guard.sh: cargo not found on PATH; allowing commit. Install rustup/cargo or repair PATH.\n' >&2
    exit 0
fi

# Run the narrow regression test. Release mode per the determinism-gate
# spec §11: debug-mode overflow checks can mask hash-affecting bugs by
# panicking instead of producing a different hash; release mode lets the
# hash itself be the contract.
#
# We capture stdout+stderr together; cargo test returns 0 on pass and
# non-zero on failure. `set -e` would fire on the non-zero exit, so we
# defang with `|| true` and detect explicitly below.
TEST_OUTPUT="$(cargo test --release -p fw-replay --test canonical_hash --no-fail-fast 2>&1 || true)"
TEST_EXIT="$?"

# `set -e` defanging: re-check the captured exit. Cargo test exits 0 on
# pass, 101 on test failure, other codes on infrastructure failure.
# 0 → allow; 101 → block; anything else → fail open with stderr (the
# hook is broken, not the source).
if printf '%s' "$TEST_OUTPUT" | grep -E 'test result: ok' >/dev/null 2>&1; then
    # Test passed. Hash unchanged. Allow commit.
    exit 0
fi

if printf '%s' "$TEST_OUTPUT" | grep -E 'test result: FAILED' >/dev/null 2>&1; then
    # Test failed. Hash drifted. Block commit.
    cat <<EOF >&2
========================================================================
BLOCKED: pinned canonical-state hash regression detected.

The staged diff modifies a canonical-state-bearing crate AND the
targeted determinism test (cargo test --release -p fw-replay --test
canonical_hash) fails. The pinned 60-tick canonical-state hash has
drifted on this machine.

Per docs/specs/determinism-gate.md §12 (Phase-0 acceptance gate) and
CLAUDE.md §7 (Q32.32 determinism contract): unauthorized hash drift is
a hard block. Determinism is non-negotiable for the replay-corpus +
save-migration + anti-cheat contract.

Resolution path:
  (a) Revert the offending change. Re-stage. Re-commit.
  (b) If the hash drift is INTENTIONAL (rare; requires DECISIONS.md
      entry + cross-platform CI matrix re-pin per the schema-version
      protocol):
        - Append a new docs/DECISIONS.md entry naming the schema bump.
        - Update the pinned [u8; 32] literal in
          crates/fw-replay/tests/canonical_hash.rs alongside.
        - Update the RON fixture under crates/fw-replay/fixtures/
          with the new expected_hash.
        - Bypass this hook ONLY ON THIS COMMIT via:
              git commit --no-verify ...
        (The pr-review-reminder hook still fires; document the
        intentional bump in the commit body.)

Failed test output (last 30 lines):
$(printf '%s' "$TEST_OUTPUT" | tail -30)
========================================================================
EOF
    exit 2
fi

# Test runner produced unexpected output — fail open, log to stderr so
# the user notices the hook is broken rather than silently passing every
# commit.
printf 'canonical-hash-guard.sh: could not parse cargo test output (exit %s); allowing commit. Repair the hook.\n' "$TEST_EXIT" >&2
exit 0

#!/usr/bin/env bash
# canonical-hash-guard.sh — PreToolUse hook on Bash(git commit*).
#
# Purpose: defensive auto-rollback for the pinned MatchCanonicalState hash
# regression test. If the staged diff includes any MatchSim/** change AND
# the targeted regression test would fail post-commit, this hook BLOCKS
# the commit with exit 2.
#
# Per ADR-0012 §Component 3 (auto-rollback on canonical-hash drift). Per
# CLAUDE.md §6.3 mandatory rotation: MatchSim code requires gameplay-/
# engine-programmer subagent + pr-review-toolkit triple — but autonomous
# Tier-2 implementations might bypass review on a small change. This hook
# is the structural floor: even if review missed it, hash drift cannot
# silently land.
#
# Hook contract:
#   - PreToolUse on Bash(git commit*).
#   - Reads tool input as JSON on stdin.
#   - Exits 0 if commit is allowed.
#   - Exits 2 with stderr message if commit is blocked.
#
# Performance budget: ~3-7s. The targeted test runs only one canonical-state
# regression theory, not the full 644-test suite. We do NOT pass --no-build:
# Codex 2026-05-11 P1 flagged that stale binaries would let the hook silently
# pass after MatchSim source changes. The trade-off is +2-3s for incremental
# build, which is acceptable on a pre-commit gate.

set -euo pipefail

INPUT="$(cat)"

if ! command -v jq >/dev/null 2>&1; then
    printf 'canonical-hash-guard.sh: jq not found; allowing commit\n' >&2
    exit 0
fi

# Parse the tool input. We only inspect Bash tool calls whose command starts
# with "git commit" (anything matching the pre-existing pre-tool-use matcher
# pattern in .claude/settings.json on Bash(git commit*)).
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

# Check if the staged diff touches MatchSim/** (Sim or Content; tests
# exempt because tests don't change canonical state, only assert on it).
TOUCHES_MATCHSIM=0
if git diff --cached --name-only 2>/dev/null | grep -E '^MatchSim/(Sim|Content)/' >/dev/null 2>&1; then
    TOUCHES_MATCHSIM=1
fi

if [ "$TOUCHES_MATCHSIM" -eq 0 ]; then
    # No MatchSim canonical-state-bearing change. Allow.
    exit 0
fi

# Check for an explicit allow marker in any staged file's commit message
# preview (commit-msg is not yet finalized at PreToolUse time, so this is
# best-effort: look for the marker in the staged content of files known
# to carry such markers, e.g. a top-of-commit-message file convention or
# the ADR / SPEC entry for the change). For now: skip the deep parsing
# and rely on the user to use --no-verify if they have an authorized
# canonical-state change (escape hatch, surfaced in the block message).

# Run the narrow regression test. Only one theory; ~2s on a warm cache.
# Test name is brittle to renames; if it breaks the hook fails open and
# logs to stderr so the user knows to repair the hook, not to silently
# allow drift.
TEST_FILTER='Match_SmokeFixture60TicksWithSignaturePackets_ProducesIdenticalPinnedHash'
TEST_OUTPUT="$(dotnet test MatchSim.Tests/MatchSim.Tests.csproj \
    --nologo \
    --logger 'console;verbosity=minimal' \
    --filter "$TEST_FILTER" 2>&1 || true)"

# Detect explicit pass/fail. dotnet test returns 1 on failure; we already
# captured stderr so set -e doesn't fire. Look for the result line.
if printf '%s' "$TEST_OUTPUT" | grep -E 'Failed:[[:space:]]*0' >/dev/null 2>&1; then
    # Test passed. Hash unchanged. Allow commit.
    exit 0
fi

if printf '%s' "$TEST_OUTPUT" | grep -E 'Failed:[[:space:]]*[1-9]' >/dev/null 2>&1; then
    # Test failed. Hash drifted. Block commit.
    cat <<EOF >&2
========================================================================
BLOCKED: pinned MatchCanonicalState hash regression detected.

The staged diff modifies MatchSim/Sim/** or MatchSim/Content/** AND the
targeted regression test '$TEST_FILTER' fails. The pinned 60-tick
canonical-state hash has drifted.

Per ADR-0012 §Component 3 (auto-rollback on canonical-hash drift) +
CLAUDE.md §3 (Q32.32 fixed-point determinism contract): unauthorized
hash drift is a hard-block. Determinism is non-negotiable for the
golden-replay-corpus + cross-platform-determinism gate.

Resolution path:
  (a) Revert the offending change. Re-stage. Re-commit.
  (b) If the hash drift is INTENTIONAL (rare; requires SPEC entry +
      cross-platform CI matrix re-pin per the schema-version protocol):
      - Append a new SPEC.md decisions-log entry naming the schema bump.
      - Update the pinned literal in MatchDeterminismTests.cs alongside.
      - Bypass this hook ONLY ON THIS COMMIT via:
              git commit --no-verify ...
        (But the §6.3 pr-review-reminder still fires; document the
        intentional bump in the commit body.)

Failed test output (last 30 lines):
$(printf '%s' "$TEST_OUTPUT" | tail -30)
========================================================================
EOF
    exit 2
fi

# Test runner produced unexpected output — fail open, log to stderr so the
# user notices the hook is broken rather than silently passing every commit.
printf 'canonical-hash-guard.sh: could not parse dotnet test output; allowing commit. Repair the hook.\n' >&2
exit 0

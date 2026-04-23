#!/bin/bash
# Bootstrap verification — runs after /bootstrap's Phase C.
#
# Reports any remaining placeholders, missing files, broken permissions, or
# inconsistent state. Exit 0 if clean; exit 1 if issues found.
#
# Invoked by Claude during /bootstrap Phase C step 6. Also safe to run
# manually at any time via: bash .claude/bootstrap/scripts/verify.sh

set -uo pipefail

PROJECT_DIR="${CLAUDE_PROJECT_DIR:-$(pwd)}"
cd "$PROJECT_DIR"

FAILURES=0

log_fail() {
    echo "  FAIL: $*" >&2
    FAILURES=$((FAILURES + 1))
}

log_ok() {
    echo "  OK:   $*"
}

log_warn() {
    echo "  WARN: $*"
}

echo "=== Bootstrap verification ==="
echo "Project: $PROJECT_DIR"
echo ""

# --- Placeholder leaks ---
echo "[1/7] Checking for unreplaced placeholders..."
PLACEHOLDER_HITS=$(grep -rn "{{PROJECT_NAME}}\|{{STUDIO}}\|{{GENRE}}" \
    --include='*.md' --include='*.yml' --include='*.yaml' --include='*.json' \
    --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=Library \
    . 2>/dev/null | grep -v ".claude/bootstrap/" || true)
if [ -n "$PLACEHOLDER_HITS" ]; then
    log_fail "Unreplaced {{PLACEHOLDER}} markers found:"
    echo "$PLACEHOLDER_HITS" | head -10 | sed 's/^/         /'
    [ "$(echo "$PLACEHOLDER_HITS" | wc -l)" -gt 10 ] && echo "         ... (more)"
else
    log_ok "No unreplaced {{PLACEHOLDER}} markers"
fi

FILLIN_HITS=$(grep -rn "<fill-in>\|<TBD>" \
    --include='*.md' --include='*.yml' --include='*.yaml' \
    --exclude-dir=.git --exclude-dir=node_modules --exclude-dir=Library \
    . 2>/dev/null | grep -v ".claude/bootstrap/" | grep -v "blueprint" || true)
if [ -n "$FILLIN_HITS" ]; then
    FILLIN_COUNT=$(echo "$FILLIN_HITS" | wc -l | tr -d ' ')
    log_warn "$FILLIN_COUNT <fill-in>/<TBD> markers remain (some are OK as Phase-0 deferred):"
    echo "$FILLIN_HITS" | head -5 | sed 's/^/         /'
    [ "$FILLIN_COUNT" -gt 5 ] && echo "         ... (more)"
else
    log_ok "No <fill-in>/<TBD> markers"
fi

# --- Required files present ---
echo ""
echo "[2/7] Checking required files..."
for file in CLAUDE.md PROJECT_CONTEXT.md TECH_APPROACH.md SPEC.md STATUS.md \
            CHANGELOG.md SETUP.md TOOLING.md .gitignore \
            .claude/settings.json \
            .claude/hooks/protect-decisions-log.sh \
            .claude/hooks/update-status-timestamp.sh; do
    if [ ! -f "$file" ]; then
        log_fail "Missing: $file"
    fi
done
[ $FAILURES -eq 0 ] && log_ok "All required files present"

# --- Hook executability ---
echo ""
echo "[3/7] Checking hook executability..."
for hook in .claude/hooks/*.sh; do
    [ -f "$hook" ] || continue
    if [ ! -x "$hook" ]; then
        log_fail "Not executable: $hook (run: chmod +x $hook)"
    fi
done
[ $FAILURES -eq 0 ] && log_ok "All hooks executable"

# --- Slash commands present ---
echo ""
echo "[4/7] Checking slash commands..."
for cmd in next done audit refresh-docs log-decision status; do
    if [ ! -f ".claude/commands/${cmd}.md" ]; then
        log_fail "Missing slash command: /${cmd}"
    fi
done
[ $FAILURES -eq 0 ] && log_ok "All slash commands present"

# --- Decisions log hook protects SPEC.md ---
echo ""
echo "[5/7] Checking decisions-log hook wiring..."
if grep -q "protect-decisions-log.sh" .claude/settings.json 2>/dev/null; then
    log_ok "Hook wired in settings.json"
else
    log_fail ".claude/settings.json does not reference protect-decisions-log.sh"
fi

# --- Git initialized ---
echo ""
echo "[6/7] Checking git state..."
if [ -d .git ]; then
    COMMIT_COUNT=$(git rev-list --count HEAD 2>/dev/null || echo 0)
    if [ "$COMMIT_COUNT" -ge 1 ]; then
        log_ok "Git initialized with $COMMIT_COUNT commit(s)"
    else
        log_warn "Git initialized but no commits yet"
    fi
else
    log_warn "Git not initialized (run: git init && git add . && git commit -m 'Initial commit')"
fi

# --- CLI tool availability ---
echo ""
echo "[7/7] Checking host CLI tools..."
for cli in gh git node python3; do
    if ! command -v "$cli" >/dev/null 2>&1; then
        log_warn "Missing (not blocking bootstrap): $cli"
    fi
done
for cli in git-lfs jq; do
    if ! command -v "$cli" >/dev/null 2>&1; then
        log_warn "Missing (will matter later): $cli (brew install $cli)"
    fi
done

# --- Summary ---
echo ""
echo "=== Summary ==="
if [ $FAILURES -eq 0 ]; then
    echo "✓ Bootstrap verification: PASSED"
    exit 0
else
    echo "✗ Bootstrap verification: $FAILURES failure(s)"
    exit 1
fi

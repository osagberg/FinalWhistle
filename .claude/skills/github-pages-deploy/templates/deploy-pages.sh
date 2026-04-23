#!/usr/bin/env bash
# deploy-pages.sh — idempotent WebGL deploy to GitHub Pages.
#
# Usage:
#   deploy-pages.sh [BUILD_DIR] [--branch NAME] [--message MSG] [--dry-run]
#
# BUILD_DIR defaults to Build/WebGL. The directory must contain index.html.
#
# Auth: this script delegates to `gh` CLI. Run `gh auth login` once beforehand.
# No personal access tokens should be hand-wired here.
#
# Idempotency: safe to re-run. The gh-pages branch is built fresh each time
# from a temporary worktree, then force-pushed (with --force-with-lease) so
# history doesn't accumulate stale assets.

set -eu
# Not set -o pipefail — macOS default bash is 3.2 and pipefail predates it,
# but some CI sandboxes don't enable it. Explicit fail-checks used instead.

# -------------------------------------------------------------------------
# Args
# -------------------------------------------------------------------------

BUILD_DIR="Build/WebGL"
BRANCH="gh-pages"
COMMIT_MSG="deploy: WebGL build $(date -u '+%Y-%m-%dT%H:%M:%SZ')"
DRY_RUN=0

while [ $# -gt 0 ]; do
  case "$1" in
    --branch)
      BRANCH="$2"
      shift 2
      ;;
    --message)
      COMMIT_MSG="$2"
      shift 2
      ;;
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --help|-h)
      sed -n '2,12p' "$0"
      exit 0
      ;;
    -*)
      echo "unknown flag: $1" >&2
      exit 2
      ;;
    *)
      BUILD_DIR="$1"
      shift
      ;;
  esac
done

LOG=/tmp/gh-pages-deploy.log
: > "$LOG"

log()  { printf '%s\n' "$*" | tee -a "$LOG"; }
fail() { printf 'FAIL: %s\n' "$*" | tee -a "$LOG" >&2; exit 1; }

# -------------------------------------------------------------------------
# Preconditions
# -------------------------------------------------------------------------

log "deploy-pages.sh → branch=$BRANCH build=$BUILD_DIR dry-run=$DRY_RUN"

command -v git >/dev/null 2>&1 || fail "git not installed."
command -v gh  >/dev/null 2>&1 || fail "gh CLI not installed — https://cli.github.com/"

if ! gh auth status >/dev/null 2>&1; then
  fail "gh CLI not authenticated. Run: gh auth login"
fi

if ! git rev-parse --git-dir >/dev/null 2>&1; then
  fail "not inside a git repo."
fi

REMOTE_URL="$(git remote get-url origin 2>/dev/null || true)"
case "$REMOTE_URL" in
  *github.com*) : ;;
  "")           fail "no 'origin' remote. This skill is GitHub-only." ;;
  *)            fail "origin is not a github.com remote ($REMOTE_URL). This skill is GitHub-only." ;;
esac

if [ ! -d "$BUILD_DIR" ]; then
  fail "build directory not found: $BUILD_DIR. Run the unity-webgl-builder skill first."
fi

if [ ! -f "$BUILD_DIR/index.html" ]; then
  fail "$BUILD_DIR has no index.html. Did the WebGL build succeed?"
fi

# -------------------------------------------------------------------------
# Resolve owner / repo / pages URL
# -------------------------------------------------------------------------

# parse owner/repo out of git remote — handles both https://github.com/o/r(.git)
# and git@github.com:o/r(.git) forms.
OWNER_REPO="$(printf '%s' "$REMOTE_URL" \
  | sed -E 's#^https?://github\.com/##; s#^git@github\.com:##; s#\.git$##')"
OWNER="${OWNER_REPO%%/*}"
REPO="${OWNER_REPO##*/}"
[ -n "$OWNER" ] && [ -n "$REPO" ] || fail "could not parse owner/repo from $REMOTE_URL"

PAGES_URL="https://${OWNER}.github.io/${REPO}/"
log "repo=$OWNER/$REPO target=$PAGES_URL"

# Check visibility + pro-requirement — fail fast rather than after a 30s push.
VISIBILITY="$(gh repo view "$OWNER/$REPO" --json visibility -q .visibility 2>/dev/null || true)"
if [ "$VISIBILITY" = "private" ]; then
  # Pages on private repos requires Pro. Don't try to detect Pro (no reliable
  # API); if push fails later with the right signature, we'll remediate then.
  log "note: repo is private. GitHub Pages on private repos requires GitHub Pro."
fi

# -------------------------------------------------------------------------
# Build a throwaway worktree with the build contents
# -------------------------------------------------------------------------

REPO_ROOT="$(git rev-parse --show-toplevel)"
TMP_DIR="$(mktemp -d -t gh-pages-XXXXXX)"
trap 'rm -rf "$TMP_DIR"' EXIT

log "staging to $TMP_DIR"

# Copy the build output into the temp dir. Use tar-pipe to preserve symlinks
# cleanly and avoid cp's platform differences between GNU and BSD.
( cd "$REPO_ROOT" && tar cf - -C "$BUILD_DIR" . ) | ( cd "$TMP_DIR" && tar xf - )

# .nojekyll — tells Pages not to process the site through Jekyll, which
# strips files/folders starting with underscore. Unity emits Build/
# (uppercase, fine) but templates occasionally use _framework.
touch "$TMP_DIR/.nojekyll"

cd "$TMP_DIR"
git init -q -b "$BRANCH"
git remote add origin "$REMOTE_URL"
git add -A

# Use a local committer identity scoped to this temp repo to avoid
# touching the user's global git config. This is safe: we're not
# signing or performing identity-bearing operations beyond pushing to
# a branch the user owns.
git -c user.email="deploy@localhost" \
    -c user.name="deploy-pages.sh" \
    commit -q -m "$COMMIT_MSG"

# -------------------------------------------------------------------------
# Push
# -------------------------------------------------------------------------

if [ "$DRY_RUN" -eq 1 ]; then
  log "dry-run: would push $(git rev-parse HEAD) → origin/$BRANCH"
  log "$PAGES_URL"
  exit 0
fi

log "pushing to origin/$BRANCH (force-with-lease, safe because branch is generated)"

# --force-with-lease is safer than --force: it refuses to overwrite remote
# refs that moved since we last fetched. For a generated-only branch this
# is effectively the same as --force but lossless if two machines race.
if ! git push --force-with-lease origin "HEAD:refs/heads/$BRANCH" 2>>"$LOG"; then
  # Private repo without Pro shows up here with a 403. Surface the hint.
  if [ "$VISIBILITY" = "private" ]; then
    fail "push failed. For private repos, GitHub Pages requires Pro. Either upgrade, make repo public, or use itch.io / Cloudflare Pages."
  fi
  fail "push to $BRANCH failed. See $LOG for git output."
fi

# -------------------------------------------------------------------------
# Configure Pages (first-run only; idempotent)
# -------------------------------------------------------------------------

# Try to set Pages source to gh-pages / root if not already configured.
# Rejections (422 already-configured, 403 insufficient permissions) are
# normal on repeat runs — log and move on.
if ! gh api "repos/$OWNER/$REPO/pages" >/dev/null 2>&1; then
  log "configuring Pages source=$BRANCH path=/"
  gh api "repos/$OWNER/$REPO/pages" -X POST \
    -f "source[branch]=$BRANCH" \
    -f "source[path]=/" >>"$LOG" 2>&1 || log "note: Pages config API returned non-zero (may be already configured or require repo-admin)."
fi

# -------------------------------------------------------------------------
# Report
# -------------------------------------------------------------------------

log "deploy complete"
log "$PAGES_URL"

# Final smoke: HEAD the URL, don't fail if it's still propagating.
if command -v curl >/dev/null 2>&1; then
  CODE="$(curl -s -o /dev/null -w "%{http_code}" -m 10 "$PAGES_URL" || echo "???")"
  log "HEAD $PAGES_URL → $CODE (200=live, 404=propagating, first deploy ~1-2min)"
fi

# stdout contract: last line is the URL so the calling skill can `tail -1`.
echo "$PAGES_URL"

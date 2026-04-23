---
name: github-pages-deploy
description: Publish a Unity WebGL build to GitHub Pages so it has a public (or private-with-Pro) URL. Chains from unity-webgl-builder; ships an idempotent deploy script. Triggers on "deploy to pages", "publish webgl", "public demo url", "github pages deploy", "gh-pages".
triggers:
  - deploy pages
  - github pages
  - gh-pages
  - publish webgl
  - public demo url
  - share build
---

# GitHub Pages Deploy — public-URL hosting for WebGL builds

Push a local WebGL build folder to a `gh-pages` branch and let GitHub Pages serve it. Useful for solo-dev share-with-friends demos, pre-release press kits, dev-log embeds. Not a CI pipeline — a one-shot CLI that's re-runnable.

**Identity check (non-negotiable):** this skill uses GitHub. If the project has an identity-firewall rule that forbids GitHub (check project `CLAUDE.md` §2), do NOT invoke. Ask the user to confirm GitHub is the correct destination for this project before running.

## When to invoke

| Situation | Trigger |
|---|---|
| After a successful [unity-webgl-builder](../unity-webgl-builder/SKILL.md) run | Publish the freshly built artifact. |
| `/release-checklist` "demo build live" step | Automate the public URL part. |
| User asks "share this build with someone" | Quickest option when GitHub is available. |
| Periodic dev log / press kit update | Re-run; script is idempotent. |

Skip for: projects with identity-firewall rules against GitHub; builds containing NSFW / private content unless repo is private AND user has GitHub Pro (Pages on private repos requires Pro).

## Integration

- Upstream: [unity-webgl-builder](../unity-webgl-builder/SKILL.md) — typical chain is `webgl build → pages deploy`.
- Companion: itch.io HTML5 upload is a better target for indie / NSFW / non-GitHub flows. Not covered by this skill; may ship as a separate skill later.
- Auth: relies on `gh` CLI being authenticated. User completes `gh auth login` once; this skill assumes the token is usable.

## Preconditions

1. `gh` CLI installed and authenticated: `gh auth status` must show a user.
2. Current git repo has a GitHub remote. `git remote get-url origin` must return a `github.com` URL.
3. A WebGL build exists at the expected location (default `Build/WebGL/`, override with `--build-dir`).
4. If repo is private: user has GitHub Pro (Pages on private repos is Pro-only). Public repo → no Pro needed.
5. First-run: the repo's Pages config must be set to "Deploy from branch: gh-pages / (root)". Script sets this automatically if the repo owner has permissions.

## Procedure

### Step 1 — Verify preconditions

```bash
gh auth status 2>&1 | head -5
git remote get-url origin
ls Build/WebGL/index.html  # or wherever the build lives
```

Failure here → halt. Report to user which precondition is unmet; do not attempt to push.

### Step 2 — Run the deploy script

```bash
bash .claude/skills/github-pages-deploy/templates/deploy-pages.sh [build-dir]
```

Default build dir is `Build/WebGL`. Script is idempotent — re-running replaces the previous deploy atomically.

### Step 3 — Verify the deploy is live

The script prints the published URL to stdout. After a fresh first-deploy, Pages needs ~1–2 minutes to propagate. Subsequent deploys are faster (~10–30s).

Smoke check via Bash:
```bash
URL=$(tail -1 /tmp/gh-pages-deploy.log)
curl -s -o /dev/null -w "%{http_code}" "$URL"  # expect 200 (or 404 during propagation window)
```

If the URL returns 200 but browser shows blank page → check browser console for Content-Encoding errors. This is the compression mismatch from unity-webgl-builder; rebuild with `compression=brotli` (GitHub Pages supports it) or `compression=disabled` (works anywhere but bigger).

### Step 4 — Report

```
pages-deploy PASS
  url         : https://<user>.github.io/<repo>/
  branch      : gh-pages
  build-dir   : Build/WebGL (47.3 MB)
  first-load  : 200 OK in 1.4s
  propagation : ~30s (subsequent deploys faster)
```

## Auth flow

```
user                    gh CLI                    github
 │                         │                          │
 │ gh auth login           │                          │
 │────────────────────────>│                          │
 │                         │ opens browser for OAuth  │
 │                         │─────────────────────────>│
 │                         │<─ token, stored in       │
 │                         │   ~/.config/gh/hosts.yml │
 │                         │                          │
 │ run deploy-pages.sh     │                          │
 │────────────────────────>│                          │
 │                         │ git push gh-pages        │
 │                         │─────────────────────────>│
 │                         │<─ deploy triggered       │
```

If `gh auth status` fails, the script prints:
```
gh CLI not authenticated. Run: gh auth login
```
…and exits non-zero. Do not try to paper over with PATs — user's own `gh auth login` is the one-time correct path.

## Options & flags

`deploy-pages.sh [build-dir] [--branch name] [--message 'commit msg'] [--dry-run]`

- `build-dir` — positional, default `Build/WebGL`.
- `--branch` — default `gh-pages`. Change only if repo already uses that branch name for something else.
- `--message` — commit message on the gh-pages branch. Default includes UTC timestamp.
- `--dry-run` — log actions but don't push. Use for first-run verification.

## Private repo + Pro note

GitHub Pages on **private** repos requires **GitHub Pro** ($4/mo) or Team+ ($4/user/mo). If the user's repo is private and they don't have Pro:
- Public repo path: user accepts public exposure → `gh repo edit --visibility public` (destructive: confirm first).
- Alternative: itch.io HTML5 upload, or a private static host (Netlify, Cloudflare Pages — free tiers exist).

Script detects private+no-Pro and halts with a remediation message; it does not attempt to bypass.

## Failure modes

### gh-not-authenticated

Symptom: `deploy-pages.sh` exits with `gh auth status` failure before any push.
Fix: `gh auth login`. One-time; persists in `~/.config/gh/`.

### no-github-remote

Symptom: `git remote get-url origin` returns a non-github.com URL (or nothing).
Fix: This skill only handles GitHub. For GitLab / Gitea / self-hosted: use the platform's equivalent (GitLab Pages, etc.) — not in scope.

### private-repo-no-pro

Symptom: `gh api` reports `Pages must be enabled. Upgrade to Pro`.
Fix: Upgrade to Pro, make repo public, or use itch.io.

### branch-already-exists-with-unrelated-history

Symptom: Script tries to push gh-pages and gets `rejected (non-fast-forward)`.
Fix: Script uses `--force-with-lease` by default on `gh-pages`-only; this is safe because the branch is generated, not authored. If the error persists, `git push origin --delete gh-pages` and re-run.

### pages-not-configured

Symptom: Deploy succeeds but URL returns 404 for >5 minutes.
Fix: `gh api repos/{owner}/{repo}/pages -X POST -f "source[branch]=gh-pages" -f "source[path]=/"`. Script attempts this on first run; may need manual retry with the repo-admin token.

### build-dir-empty

Symptom: `build-dir` exists but has no `index.html`.
Fix: Run [unity-webgl-builder](../unity-webgl-builder/SKILL.md) first. Script fails fast with a clear message.

## Related

- [templates/deploy-pages.sh](templates/deploy-pages.sh) — the script.
- [../unity-webgl-builder/SKILL.md](../unity-webgl-builder/SKILL.md) — the thing that produces the artifact this skill deploys.
- Official `gh` docs: https://cli.github.com/manual/gh_auth_login
- GitHub Pages on private repos: https://docs.github.com/en/pages/getting-started-with-github-pages/about-github-pages

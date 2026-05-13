---
description: Stage explicit files and commit with a structured message. Rarely needed — /next commits automatically.
---

# /commit — manual structured commit

Use only for ad-hoc commits outside the `/next` flow (e.g., doc-only edits, STATUS sync that didn't pair with code). For feature work, `/next` commits automatically.

## Procedure

### 1. Inspect state

```bash
git status
git diff --stat
git diff --cached --stat
git log --oneline -3
```

Identify staged vs. unstaged.

### 2. Decide files-in-scope

ONE commit = ONE intent. If the working tree spans two intents, do two commits.

NEVER `git add .` or `git add -A`. Stage explicit paths:

```bash
git add /Users/vibelogic/dev/football/<path1> /Users/vibelogic/dev/football/<path2>
```

### 3. Compose the message

```
<type>(<scope>): <one-line summary, imperative mood, <72 chars>

<2-3 line body: WHAT changed at a high level, WHY now. No how — diff speaks for itself.>

Co-Authored-By: Claude <noreply@anthropic.com>
```

Types: `feat`, `fix`, `refactor`, `test`, `docs`, `chore`, `perf`, `build`, `ci`.

Scopes (project-specific): `sim`, `memory`, `replay`, `save`, `content`, `core`, `scouting`, `tauri`, `ui`, `cli`, `decisions`, `status`, `plan`, `changelog`, or a crate name.

Examples:
- `feat(sim): wire ball-physics integrator into BT runner tick`
- `fix(save): preserve callback IDs across v3 → v4 migration`
- `docs(plan): mark Phase T0 row 3 DONE after canonical-hash pin`

### 4. Commit via HEREDOC

```bash
git commit -m "$(cat <<'EOF'
<type>(<scope>): <summary>

<body>

Co-Authored-By: Claude <noreply@anthropic.com>
EOF
)"
```

### 5. Post-commit verify

```bash
scripts/fw verify
```

If red, the commit is already in. Decide: revert (`git revert HEAD`) or queue a fix-up commit. Do NOT amend — CLAUDE.md §4.5 forbids it.

### 6. Report

One line: `<hash> <type>(<scope>): <summary>` + verify status (green / red + first failure).

## Hard rules

- No `git add .` / `git add -A`. Explicit paths only.
- No `--amend`. Create new commits.
- No `--no-verify`. Hooks run.
- One intent per commit.
- Always include `Co-Authored-By: Claude` footer.

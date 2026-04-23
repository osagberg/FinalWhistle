---
description: Load the full blueprint reference library for heavy-research phases (Phase 0/2/8)
argument-hint: "[session-only|permanent] [topic: design|unity|steam|patterns|all]"
---

# /deep-research — load the full reference library

Elevates scope to `research`. Proactively loads the complete blueprint reference library: `patterns/**/*.md`, `unity/**/*.md`, `steam-release/*.md`, `asset-pipelines/*.md`, `references/*-CATALOG.md`. Sets context budget to ~800KB proactive-load — **intended for 1M-context tier sessions only**.

## When to invoke

| Phase | Why deep research matters |
|---|---|
| **Phase 0 — Kickoff** | Cross-reference comparable titles / genre conventions / market data |
| **Phase 2 — Design Bible** | Authoring decisions need MDA/SDT/Flow/Bartle frameworks + per-mechanic precedents from industry |
| **Phase 8 — Launch Prep** | Age rating questionnaire decisions, platform gotchas, marketing timeline, accessibility compliance — all breadth-heavy |
| **Rare architectural decisions** | When a single decision spans engine + platform + release + design (e.g., "add networking" requires touching ~everything) |

## When NOT to invoke

| Signal | Why deep-research is overkill |
|---|---|
| Active coding session (Phase 4-7) | Rich is enough; reference library doesn't help write C# code |
| 200K-context tier | Budget too tight; research mode eats working memory |
| Already know which reference you need | Just `@` read that specific file; `/deep-research` is proactive-load, not targeted |
| Solo dev with simple game | Research mode is for projects where you're orchestrating many cross-cutting concerns |

## Arguments

- `session-only` (default) — loads references for this session; next session defaults back to current scope
- `permanent` — writes `research` to `.claude/.current-scope`; persists until contracted
- `topic` — optional focus: `design` (patterns/ + design-templates/), `unity` (unity/), `steam` (steam-release/), `patterns` (patterns/), `all` (everything — default)

## Procedure

1. Read `.claude/context-scopes.json` to confirm current scope
2. Read `~/.claude/tier-capabilities.json` if present; verify `context_window: "1M"` or warn
3. If context_window != "1M": WARN user that research mode eats ~800KB proactive-load. Offer to proceed with topic-filtering instead (`topic: <target>`).
4. Determine scope target: if `session-only`, track in conversation; if `permanent`, write `.claude/.current-scope` = `research`
5. Load selected reference topics (log to user which file groups):
   - `design` → `patterns/*.md`, `patterns/**/*.md`, `templates/design-templates/*.md`
   - `unity` → `unity/*.md`
   - `steam` → `steam-release/*.md`
   - `patterns` → `patterns/**/*.md`
   - `all` → union of above + `asset-pipelines/*.md` + `references/*-CATALOG.md`
6. Announce to user: "Research scope loaded. <N> files now proactively available. <M> KB budget consumed. Ready to orchestrate breadth-heavy work."

## If on 1M context tier and phase is one of `phase_0|phase_2|phase_8`

Recommend `/deep-research permanent all` if user hasn't already — the phase intrinsically benefits from full library access.

## Contracting back

`/contract-scope <target>` where `<target> in {minimal, standard, rich, studio}`. Or just start a new session — session-only research doesn't persist.

## Memory budget math

| Scope | Proactive-load (est KB) | Remaining context at 1M (est K tokens) |
|---|---|---|
| minimal | 15 | ~999K |
| standard | 80 | ~996K |
| rich | 150 | ~993K |
| studio | 300 | ~988K |
| research | 800 | ~968K |

Research mode still leaves plenty of room for actual work content on the 1M tier. On 200K, it's prohibitive.

## Related

- `/expand-studio` — scale to studio (less heavy than research)
- `/contract-scope <target>` — scale back down
- `.claude/context-scopes.json` — scope declaration
- `~/.claude/tier-capabilities.json` — user's context-window capability
- `references/*-CATALOG.md` — summarized catalogs of cloned reference repos
- Reads files: `.claude/context-scopes.json`, `.claude/.current-scope`, `~/.claude/tier-capabilities.json`, everything in the scope's proactive_load list
- Writes files: `.claude/.current-scope` (if `permanent`)

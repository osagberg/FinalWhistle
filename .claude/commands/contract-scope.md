---
description: Scale the current project's context scope DOWN (research → studio → rich → standard → minimal)
argument-hint: "<target_scope: minimal|standard|rich|studio>"
---

# /contract-scope — scale down to a smaller scope

Opposite of `/expand-studio` + `/deep-research`. Reduces proactive-load by switching to a smaller scope.

## When to invoke

- Session startup is too slow / context is bloating / you want more working-memory headroom
- Phase shifted to a lighter-weight mode (e.g., Phase 7 grind → Phase 9 post-launch hotfix cycle)
- You elevated scope for a specific decision (`/deep-research`) and want to go back

## Arguments

- `target_scope` (required): one of `minimal | standard | rich | studio`. Can't contract to `research`; use `/deep-research` for that direction.

## Procedure

1. Read `.claude/context-scopes.json` + `.claude/.current-scope` to confirm current
2. Validate target is LOWER than current (minimal < standard < rich < studio < research)
3. Write target to `.claude/.current-scope`
4. Announce to user: what was proactively loaded before is no longer; lists the newly-hidden content (still available on-demand, just not orienting you at session start)
5. Recommend: if contracting because of specific issue (e.g., context bloat), consider logging via `/log-decision`

## If target = current scope

No-op. Report current state.

## If target > current scope

Redirect: "That's elevation, not contraction. Did you mean `/expand-studio` or `/deep-research`?"

## Related

- `/expand-studio` — scale up to studio
- `/deep-research` — scale up to research (the top)
- `.claude/context-scopes.json` — scope declarations

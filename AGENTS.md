# Codex Project Guide

Read `CLAUDE.md` first and treat it as the project contract. Then read `STATUS.md`, `SPEC.md`, `PROJECT_CONTEXT.md`, and `TECH_APPROACH.md` before changing behavior, architecture, content structure, or Unity setup.

Codex-specific additions:

- Use the global `game-studio-codex` skill for browser-game, asset-pipeline, and playtest workflow context.
- Use the global `superpowers-codex` skill for larger planning, debugging, TDD, verification, and review loops.
- Preserve the existing `.claude/` agents, commands, hooks, skills, and rules as source material until they are explicitly ported to Codex-native equivalents.
- For visual/gameplay work, verify with runtime evidence: Unity MCP screenshot/play-mode check, browser screenshot, state dump, or targeted test depending on the task.

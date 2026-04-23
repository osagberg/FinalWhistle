---
description: One-shot project bootstrap — intake interview, customize every template file, install MCPs, queue plugins, init git, verify
---

# /bootstrap — intelligent project setup

**Run this exactly once per project, as the first message in the first Claude Code session.** After `/bootstrap` completes, the `.claude/bootstrap/` folder can be deleted (or kept as a historical record of setup).

The user wants: "plan the game with Claude; Claude handles every MCP, plugin, skill, tool, customization, install."

This command orchestrates that.

## Phase structure

### Phase A — Intake (conversational)

1. Read `.claude/bootstrap/INTAKE.md` for the questionnaire
2. Walk the user through it **one question at a time** (batch is overwhelming) — 13 questions total, including **Q12 (context tier + default scope)**
3. Before asking Q12, check for `~/.claude/tier-capabilities.json` — if present, announce the declared tier and ask to confirm or override
4. After each answer, note it in a running mental model; don't over-explain yet
5. At the end, produce a **summary** of everything learned: project name, genre, platforms, scope, character fidelity, narrative weight, technical complexity, commercial intent, **context tier + default scope**
6. Confirm summary with user before proceeding

**If user says "just figure it out" or "minimal-question fast path":** use the five-question fast path at the bottom of INTAKE.md instead. It infers context tier from `~/.claude/tier-capabilities.json` and scope from tier.

### Phase B — Profile match + plan

1. Read `.claude/bootstrap/PROFILES.md` — 5 reference profiles, each specifying a recommended default scope
2. Match the user's answers to the closest profile (or declare "custom" and compose per-answer)
3. **Pick default scope.** Each profile declares a recommended scope. If intake answer for `default_scope` differs from the profile's recommendation, use the intake answer (user intent wins) but flag the divergence in the plan announcement
4. Read `.claude/bootstrap/CUSTOMIZATION.md` — rules for translating answers into file edits, including **"Context scope initialization"**, **"Agent roster initialization"**, and **"Rules initialization"** sections
5. Read `.claude/bootstrap/INSTALLABLES.md` — full catalog of MCPs / plugins / skills / Unity packages / Asset Store assets, plus ships-pre-installed inventory (skills, agents, design-templates, asset-pipelines, patterns)
6. **Announce the plan before executing.** One message containing:
   - Matched profile (if any)
   - **Default scope** to be written to `.claude/.current-scope` (e.g., `rich`), with rationale and note that user can re-scope anytime via `/expand-studio` / `/deep-research` / `/contract-scope`
   - List of MCPs that will be installed (with scopes)
   - List of plugins that will be queued for install (slash-command list)
   - List of template files that will be customized (and a note on "deleted sections" where applicable)
   - List of Unity package additions queued for Phase 3
   - Tier estimate for Phase 1 budget
7. Ask for user confirmation: "Proceed?"
8. Respect course-corrections — the user may refine the plan before executing

### Phase C — Execute (automated; minimal user interaction)

Do these in parallel where safe; sequential where ordering matters:

1. **Customize every template file** per CUSTOMIZATION.md rules:
   - Replace double-brace project/studio/genre markers with real values — grep across root docs AND template/pattern/release subtrees
   - Fill template fill-in fields with intake-derived content
   - Delete sections that don't apply (e.g., narrative sections for non-narrative games, character-pipeline sections for non-character games)
   - Add project-specific sections where profile suggests
   - Ensure CLAUDE.md + PROJECT_CONTEXT.md + TECH_APPROACH.md reflect real choices, not placeholder stubs

2. **Initialize context scope** (per CUSTOMIZATION.md → "Context scope initialization"):
   - Write chosen scope to `.claude/.current-scope` (overwrites the `rich` default written by `new-project.sh`, if intake differs)
   - If intake says `research` but `context_window = 200K`, downgrade to `rich` and log the conflict in SPEC.md decisions log
   - Note: the 14 core agents under `.claude/agents/`, 17 rule files under `.claude/rules/`, and 5 core skills under `.claude/skills/` are already on disk — no copy step; scope only affects proactive-load behavior

3. **Install MCPs** (where Claude Code CLI supports `claude mcp add` at the shell level):
   - Run the user-scoped MCPs (once per machine — detect if already installed via `claude mcp list`, skip if present)
   - Run the project-scoped MCPs (always add fresh per project)
   - Handle auth flags: for GitHub MCP, require user to have `gh auth login` done (check via `gh auth status`)

4. **Write the plugin install script.** Plugin installation is slash-command-only — Claude cannot invoke `/plugin install` as a tool call. Instead, write `.claude/bootstrap/scripts/install-plugins.txt` with one slash command per line. At the end of bootstrap, display this list and tell the user to paste them into Claude one at a time.

5. **Initialize git** (unless it already exists):
   - `git init`
   - Initial commit
   - Optional: offer to create GitHub remote (`gh repo create`) after asking user for studio/org + public/private preference

6. **Record provenance**: update `.blueprint-version` with intake summary + customization decisions, including `default_scope` + `context_window`

7. **Run verification**: `.claude/bootstrap/scripts/verify.sh` reports any remaining `{{placeholder}}` / `<fill-in>` markers, missing files, broken hook permissions, etc. Also confirm `.claude/.current-scope` exists and contains a valid scope name declared in `.claude/context-scopes.json`. If any issues, fix them before reporting success.

### Phase D — Report + handoff

Single message to user covering:
- **What was done** (summary of customizations + installs)
- **Starting scope** — announce `.claude/.current-scope` value and brief rationale. Note user can re-scope any time:
  - `/expand-studio` — scale up to studio (more agents + sprint patterns)
  - `/deep-research` — scale up to research (full reference library loaded)
  - `/contract-scope <target>` — scale back down
- **What needs manual action** (plugin install commands — paste these into Claude; account sign-ins; purchases deferred per budget trigger table)
- **What's next** — phase currently ACTIVE is Phase 0 (Kickoff); first task is typically "confirm pitch" or similar; invite user to run `/next`

Then stop. Do not auto-run `/next` — let the user take the next action.

## Safety rules

- **Never run `gh repo create` without user confirmation on repo name + visibility.** Private-by-default, public only on explicit yes.
- **Never run `git push` during bootstrap** — remote setup is fine, first push is user-gated.
- **Never install global tools via Homebrew / npm -g** — stay at `claude mcp add` + `/plugin install` scope. If a dep is missing, ASK the user to install it.
- **Never modify files outside this project directory** (except `~/.claude/` for user-scoped MCP adds, explicitly intentional).
- **Never purchase anything.** If a profile recommends a paid asset, add it to SETUP.md §10 trigger table with the specific pain point that would trigger purchase; DO NOT prompt for a purchase during bootstrap.
- **Age gate / content policy:** if user's game concept touches mature/adult content, ensure PROJECT_CONTEXT.md has honest age rating target + the content_policy.md section is populated with "what we will and won't depict." No shipping without explicit user decisions here.

## If bootstrap is interrupted

If `/bootstrap` is run partway (e.g., user already answered intake but Claude crashed before executing):
- Re-read `.blueprint-version` — if it contains an `intake_complete: true` marker but no `customize_complete: true`, skip re-asking questions, resume at Phase B plan-announcement
- If `.blueprint-version` has partial state, confirm with user before resuming

## After bootstrap

The `.claude/bootstrap/` folder can be deleted:
```bash
rm -rf .claude/bootstrap/
```

This removes ~100KB of now-one-shot data. OR keep it as historical record of how the project was set up. Either is valid.

The `/bootstrap` command itself (`.claude/commands/bootstrap.md`) can also be deleted after use — it's one-shot. Remove with:
```bash
rm .claude/commands/bootstrap.md
```

## Resume-friendly

If the user wants to re-customize mid-project (e.g., scope shifted, genre clarified):
- Do NOT re-run full bootstrap
- Instead, propose targeted edits to the specific files that need updating
- Log the shift in SPEC.md decisions log via `/log-decision`

Bootstrap is for initial setup, not for ongoing config drift. Drift is handled by normal `/next` / `/done` / `/log-decision` flows.

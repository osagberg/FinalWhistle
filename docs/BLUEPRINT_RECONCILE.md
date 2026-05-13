# Blueprint reconciliation — 2026-05-13

One-shot audit doc explaining how `/Users/vibelogic/dev/blueprint/` (the Unity-era multi-game framework) was slimmed + adapted INTO `/Users/vibelogic/dev/football/` (the Rust+Tauri+SolidJS pivot of Final Whistle).

---

## Why this exists

The 2026-05-13 pivot moved Final Whistle off Unity+C# onto Rust+Tauri+SolidJS for solo-dev shippability + text-first scope. The blueprint at `/Users/vibelogic/dev/blueprint/` (306MB, 8833 files) was originally designed for Unity games — heavy on art-direction agents, Addressables / ScriptableObjects rules, 3D-pipeline templates, semantic-cinema patterns.

Adapting it in-place (rather than forking it into a separate `blueprint-rust`) keeps the project self-contained and avoids dual-source drift. The blueprint at `~/dev/blueprint/` remains a reference for future games; this project's `.claude/` and `templates/` and `patterns/` are now derivative-and-slimmed for this specific game + stack.

## What we adopted

From the blueprint, kept as patterns + behavior:

- **Single primary command (`/next`)** end-to-end loop. The blueprint command was Unity-flavored; we kept the 9-step shape, replaced cargo/pnpm for the verify line, rewired to the 7-agent rotation.
- **Decisions log discipline** — `docs/DECISIONS.md` as append-only, hook-enforced via `.claude/hooks/protect-decisions.sh`. Supersede via new entries citing prior verbatim.
- **STATUS / CHANGELOG / MASTER_PLAN cadence** — STATUS is a state pointer (not a diary); CHANGELOG is append-only; MASTER_PLAN is delivery SoT.
- **Phase-gate workflow** — Codex review at phase boundaries (not per-task agent-bus relay).
- **Per-task self-review** on ≥100 LoC via the `pr-review-toolkit` subagents.
- **Path-scoped rules** — `.claude/rules/<Path>/RULES.md` lazy-loaded per matching edit.
- **Context-scope architecture** — declared in `.claude/context-scopes.json`, active scope at `.claude/.current-scope`. Slimmed from 5 levels to 3 (`minimal` / `standard` / `rich`).
- **Director/lead/specialist agent tiers** — slimmed from 14 to 7.
- **Design-doc templates** under `templates/design-templates/` — 8 templates retained, all Unity refs stripped.
- **Patterns library** under `patterns/` — 9 patterns, Rust-flavored.

## What we dropped

- **Unity-specific agents** (7): `art-director`, `unity-specialist`, `unity-ui-specialist`, `engine-programmer`, `creative-director`, `game-designer`, `technical-director` — collapsed into the 7 remaining or removed entirely (text-first; no 3D).
- **29 of 35 blueprint slash commands** — kept only `/next`, `/done`, `/status`, `/log-decision`, `/commit`, `/audit`.
- **9 of 14 blueprint hooks** — kept `protect-decisions`, `update-status-timestamp`, `canonical-hash-guard`, plus new `validate-commit` + `pr-review-reminder`. Dropped `refresh-unity-on-script`, `validate-assets`, `pre-build-verify`, `log-agent`, `log-agent-stop`, `session-start`, `session-stop`, `detect-gaps`, `pre-compact`, `post-compact`.
- **Asset-pipelines / unity / steam-release / ci-cd folders** — Unity-specific.
- **3D / shader / cinematography / manga-broadcast / semantic-cinema** — all viewer-side concerns are now 2D PixiJS tactical board + text recap.
- **The duo-* skills** (`/duo-implement`, `/duo-debate`, `/codex-review-loop`, `/check-reviews`) — replaced by single `/next` + phase-gate PR.
- **The agent-bus per-slice protocol** — replaced by phase-gate Codex PR review.
- **`studio` + `research` context scopes** — overkill for solo+text. Kept `minimal` / `standard` / `rich`.
- **The `bootstrap/` machinery** — one-time pass already complete; no re-bootstrap needed in-project.
- **Design templates for art-bible / HUD / ux-spec** — Unity / 3D / specifically-visual.
- **Unity-specific rule directories** (`CSharp/`, `Assemblies/`, `Addressables/`, `ScriptableObjects/`, `Scripts/`) — replaced with `Rust/`, `Sim/`, `Tauri/`, `Frontend/`, `Content/`, `design-docs/`.

## What we adapted (file-by-file)

| Source (blueprint) | Destination (football) | Adaptation |
|---|---|---|
| `templates/CLAUDE.md` | `CLAUDE.md` | Trimmed Unity references; added Tauri/SolidJS stack; 7-agent rotation; 3-scope architecture; ~210 lines. |
| `templates/.claude/commands/next.md` | `.claude/commands/next.md` | Thin wrapper, references the 9-step skill; Rust task classes; replaced `/phase-gate` references with `/done`. |
| `templates/.claude/commands/done.md` | `.claude/commands/done.md` | Phase-gate close with explicit `gh pr create` for Codex review; Final-Whistle phase acceptance examples. |
| `templates/.claude/commands/status.md` | `.claude/commands/status.md` | <150-word state pointer; STATUS staleness flag. |
| `templates/.claude/commands/log-decision.md` | `.claude/commands/log-decision.md` | Append-only enforcement; supersede protocol. |
| `templates/.claude/commands/audit.md` | `.claude/commands/audit.md` | 10-check sweep; Rust-flavored (HashMap/f32/tokio bans + clippy + cargo tree). |
| `templates/.claude/agents/*.md` | `.claude/agents/*.md` | 7 of 14 retained, all rewritten for Rust+Tauri+SolidJS+text-first. `ui-programmer` rebranded from UI Toolkit to SolidJS+Tauri+PixiJS. |
| `templates/.claude/rules/CSharp/RULES.md` | `.claude/rules/Rust/RULES.md` | Full content rewrite — Rust edition 2024, errors via thiserror, no speculative abstractions, etc. |
| (none) | `.claude/rules/Sim/RULES.md` | NEW — determinism non-negotiables for sim crates (Q32, BTreeMap, no async, no clocks). |
| (none) | `.claude/rules/Tauri/RULES.md` | NEW — Tauri IPC patterns, async-OK, UI-never-drives-canonical-state. |
| (none) | `.claude/rules/Frontend/RULES.md` | NEW — SolidJS + Tailwind v3 + TanStack v8 + PixiJS v8 + ECharts. |
| (none) | `.claude/rules/Content/RULES.md` | NEW — RON, content-pack IDs, banned terms, mod overlays. |
| `templates/.claude/rules/design-docs/RULES.md` | `.claude/rules/design-docs/RULES.md` | Ported; tuning-coefficients-out-of-SPEC rule from user MEMORY. |
| `templates/.claude/hooks/protect-decisions-log.sh` | `.claude/hooks/protect-decisions.sh` | Already ported earlier in the pivot; verified Rust-aware. |
| `templates/.claude/hooks/update-status-timestamp.sh` | `.claude/hooks/update-status-timestamp.sh` | Ported verbatim. |
| `templates/.claude/hooks/validate-commit.sh` | `.claude/hooks/validate-commit.sh` | NEW — blocks --amend, --no-verify, interactive, secrets, DECISIONS mutation. |
| (none) | `.claude/hooks/pr-review-reminder.sh` | NEW — soft Stop hook; reminds when ≥100 LoC code committed without Self-review footer. |
| `templates/design-templates/*.md` | `templates/design-templates/*.md` | 8 retained: ADR / game-concept / game-design-document / game-pillars / systems-index / test-plan / playtest-report / postmortem. Unity refs stripped. |
| `patterns/*.md` | `patterns/*.md` | 9 patterns: behavior-trees / event-driven / fsm / save-load / unit-testing / phase-gate-workflow / dependency-injection / bake-time-llm-content-pipeline / narrative-subagent-pattern. Unity ECS / Addressables patterns dropped. |

## Final structure

After this reconciliation, the project layout under `/Users/vibelogic/dev/football/`:

```
.claude/
  agents/         (7 .md files + README — 7-agent slim roster)
  commands/       (6 .md files — /next /done /status /log-decision /commit /audit)
  hooks/          (5 .sh files — all executable)
  rules/          (6 RULES.md across Rust/ Sim/ Tauri/ Frontend/ Content/ design-docs/ + README)
  skills/next/    (SKILL.md + RECIPE.md)
  context-scopes.json
  .current-scope  (single line: "standard")
  settings.json
docs/
  DESIGN_DOC.md, MASTER_PLAN.md, DECISIONS.md (existing)
  BLUEPRINT_RECONCILE.md (this file)
  CONTENT_PIPELINE.md (existing)
  archive/ (FW v1 historical, read-only)
  design/ (per-system design docs)
  specs/ (determinism-gate, etc.)
templates/
  design-templates/ (8 templates + README)
patterns/
  9 patterns + README
crates/ frontend/ src-tauri/ content/  (unchanged from pivot scaffold)
scripts/fw  Justfile  Cargo.toml  package.json  (existing; integration step verifies coherence)
CLAUDE.md  README.md  REFERENCES.md  MEMORY.md  STATUS.md  CHANGELOG.md
```

## Future re-syncs

If `/Users/vibelogic/dev/blueprint/` evolves upstream and we want to selectively pull updates:

1. `cd ~/dev/blueprint && git log` — see what changed.
2. Decide per-file: is the change Unity-specific (skip) or generic (port)?
3. For ports: copy + adapt to Rust idioms following the patterns established here.
4. Update this `BLUEPRINT_RECONCILE.md` with a "## Re-sync YYYY-MM-DD" subsection naming what landed.
5. Run `/audit` to verify nothing regressed.

## Cross-references

- `CLAUDE.md` — the project contract
- `REFERENCES.md` — pivot provenance (FW v1 carry-forward catalog)
- `.claude/agents/README.md` — 7-agent roster
- `.claude/rules/README.md` — path-scoped rule layout
- `templates/design-templates/README.md` — doc templates
- `patterns/README.md` — pattern library

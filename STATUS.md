# Status

**Last updated**: 2026-04-24 (Phase 1 🟡 ACTIVE — Setup. Remote + Tier-A CI + ops runbooks shipped; GPT-5.5 review pass applied 5 findings.)

## Currently working on

**Phase 1 — Setup** 🟡 ACTIVE.

Shipped so far: Unity install (user), `scripts/fw` front-door, PR template, issue templates, asset-licensing tracker, backup-restore policy, `osagberg/FinalWhistle` private remote, Tier-A workflow v0, branch-protection policy doc, Actions-budget runbook. `fw verify-docs` + `fw status` green locally. Remaining Phase-1 work is user-action in GitHub UI (budget cap, branch protection) or low-urgency (Blender/VS Code, slash-command smoke, plugin install).

## Blockers

- User must open fresh Claude Code session inside `/Users/vibelogic/dev/football/` for project-scoped `.claude/` (hooks + slash commands) to activate.
- User must paste `.claude/bootstrap/scripts/install-plugins.txt` commands one at a time.

## Pending async

- ~~GitHub remote creation~~ — **done 2026-04-24**: `osagberg/FinalWhistle` private; `vibelogic` org deferred to Phase 8 transfer if publisher branding wants it.
- User creates Steam Direct account at Phase 8 ($100 one-time).
- Phase-8 prep: formal trademark + Steam-name clearance for "Final Whistle" (existing non-AAA uses: `finalwhistle.es`, `finalwhistle.club`).

## Open questions for user

- Review design docs under `design/` — each remaining doc has "Open questions" section requiring user resolution before Phase 1 / Phase 2 start. Order: `month-3-vertical-slice.md` → `match-engine.md` → `semantic-cinema.md` → `event-sourced-memory.md` → `signatures.md` → `scout-disagreement.md` → `breakthrough-moments.md` → `player-generation.md` → `worldbuilding.md` → `ui-vocabulary.md`.

## Next action

**User-action in GitHub UI** (runbooks written):
- **Set Actions spending cap to $0** per `docs/ops/actions-budget.md §2b` (`github.com/settings/billing/spending_limit`). Prevents silent overage. ~1 min.
- ~~Configure branch protection~~ — **BLOCKED on plan**: GitHub Free disallows branch protection on private repos (verified via `gh api`). Current posture: local-discipline-only per `docs/ops/branch-protection.md §0`. Revisit when upgrading to Pro/Team or going public (Phase-4 itch / Phase-8 EA triggers).

**Low-urgency Phase-1 user-actions:**
- Install Blender (Phase-3 trigger, safe to defer)
- Install VS Code / Rider (editor choice)
- Smoke-test slash commands next session
- Plugin install via slash commands

**Claude-actionable on next `/next`:**
- `scripts/lint-banned-terms.py` (Phase-1 lint rule per `design/ui-vocabulary.md`) — closes another Phase-1 checkbox and uncomments the Tier-A banned-terms job
- Pre-seed SPEC-stale-task notes — Task 52 (Unity CI stub) effectively superseded by Task 58 (Tier-A workflow); could mark `[x] (superseded)`

## Recent milestones

- 2026-04-24: Phase 1 — GPT-5.5 Codex review pass applied 5 findings: branch-protection reframed as local-discipline-only (Free plan doesn't allow protection on private repos; verified via `gh api`), approvals corrected to 0 (GitHub blocks author self-approval), JetBrains Mono license corrected to SIL OFL 1.1 (Apache-2.0 covers source code only), `fw verify` umbrella added + false broken-link claim removed, spaced-placeholder hack documented as tech-debt to resolve when `lint-banned-terms.py` lands
- 2026-04-24: Phase 1 — `osagberg/FinalWhistle` private remote created + initial push (2 commits on origin/main); `.github/workflows/fast-pr-ci.yml` Tier-A v0 authored (fw verify-docs only, phase-gated TODOs commented); `docs/ops/branch-protection.md` + `docs/ops/actions-budget.md` runbooks shipped. `vibelogic` org membership gap diagnosed + namespaced-around
- 2026-04-24: Phase 1 — first parallel-to-Unity-install batch shipped (6 tasks): Unity 6 LTS installed (user), asset-licensing tracker seeded (Anton/JetBrains Mono/Rajdhani/Magica Cloth 2), `scripts/fw` front-door with `help`/`status`/`verify-docs` implemented + 5 phase-gated stubs, PR template, 2× issue templates, `docs/ops/backup-restore.md`. `fw verify-docs` + `fw status` both green
- **2026-04-24: Phase 0 ✅ COMPLETE.** All 12 design docs resolved, 12 consolidated SPEC entries, 7 Phase-2 ADRs pre-seeded, ~33 pipeline-related SPEC tasks threaded across downstream phases, `/refresh-docs` green (6 findings fixed). Phase 1 🟡 Setup now active
<!-- ui-lint:ignore-start reason="historical /refresh-docs finding description" -->
- 2026-04-24: Phase 0 — `/refresh-docs` pass fixed 6 findings including user-visible `{{ PROJECT_NAME }}` placeholder leak in `.claude/hooks/session-start.sh` (was showing literal template token in SessionStart banner every new session), plus state-dump script + statusline comment + design/README.md stale date + missing production-pipeline row + production-pipeline.md verifier-trip. Intentionally left: bootstrap placeholder source-of-truth + TECH_APPROACH §8.5 non-standard numbering
<!-- ui-lint:ignore-end -->
- 2026-04-24: Phase 0 — production-pipeline planning pass landed (GPT-5.5 report). Authored `design/production-pipeline.md` with 5-tier workflow policy, 5-channel build policy, core-deliverable specs (golden replay corpus / save fixtures / content validator / `scripts/fw` / playtest ops / crash export / backup). Phase-2 production-pipeline ADR pre-seeded. ~33 pipeline-related SPEC tasks threaded across Phases 1/2/3/4/5/6/8. `TECH_APPROACH.md` §8.5 added
- 2026-04-24: Phase 0 — `design/ui-vocabulary.md` open questions resolved; **all 11 design docs now locked**. Lint scope (code + content + rendered output), sentinel-ignore mechanism (no whole-file self-whitelist), Category-A expanded to 5 subsections absorbing bans from all prior 2026-04-24 resolutions, Category-B audited inline-exemption mechanism (audit before EA lock + RCs, not quarterly), flatter ~140-template commentary pool structure, British-football tone register default. Phase-1 lint task upgraded to full `scripts/lint-banned-terms.py`. Template governance folds into AI Content Compiler ADR (no new ADR)
- 2026-04-24: Phase 0 — `design/worldbuilding.md` open questions resolved; nation locked as **Caldren** (Cresland fallback after lightweight clearance pass), 8 regions, pyramid 20/24/16/14/12/10=96 reframed as simulated-slice-not-entire-ecosystem, three cups (National all-tier + League Cup top-2 + Trophy tiers-3-6), compiler-only analogues with Phase-1 lint rule pre-seeded
- 2026-04-24: Phase 0 — `design/player-generation.md` open questions resolved; 22-field internal model locked, 46-label phenotype catalog with 3 targeted edits, default-off advanced tooltip, canonical-JSON reproducibility, **ID-stability correction** (no pack-minor in entity IDs), authoritative affinity P(k) tables materialized here, category-level scout biases, regional-priors integration. Sixth Phase-2 ADR pre-seeded
- 2026-04-24: Phase 0 — `design/breakthrough-moments.md` open questions resolved; cinema duration locked to 3-5s (default seed 3s), strict no-system-vocabulary rule, silent-first-near-miss anti-farming policy, regressive parity, pillar-tiebreaker live-fire-on-resolving-action-only rule. No new ADR (composes existing schemas)
- 2026-04-24: Phase 0 — `design/scout-disagreement.md` open questions resolved; Month-4 feel-prototype spec locked (3 archetypes, hand-authored packets, staged-time ledger feedback, user-excluded pass criterion, one-remediation-pass ceiling to prevent rescue-loop). Fifth Phase-2 ADR pre-seeded — architecture slot reserved regardless of gate outcome
- 2026-04-24: Phase 0 — `design/signatures.md` open questions resolved; Pillar-2 24-sig catalog locked with dependency metadata, #19 corrected to "stronger foot", #6 scoped as defensive_line. Affinity distribution tier-weighted. Multi-signature stacking uses field-level caps, not softmax. Counterplay via scout reports for observed sigs only. Fourth Phase-2 ADR pre-seeded
- 2026-04-24: Phase 0 — `design/event-sourced-memory.md` open questions resolved; Pillar-1 ledger architecture locked (salience structure, CallbackTag schema with consuming-reader metadata, ~38-entry PascalCase event enum, three-tier compaction with per-season quota cap, load-time migration). Third Phase-2 ADR pre-seeded. Callback-age + player-attention clarified as reader-side modifiers (not emission-time) to reconcile with 2026-04-22 SPEC seed
- 2026-04-24: Phase 0 — `design/semantic-cinema.md` open questions resolved; 7-shot vocabulary locked through Month-3 gate, ShotTypeSO schema drafted with chain_rules + reduce_motion_variant, rendering stack + typography locked (scoreline override: not Anton), two Phase-2 ADRs pre-seeded
- 2026-04-24: Phase 0 — `design/match-engine.md` open questions resolved; ball-physics structure + steering-target movement + Month-3 in-match event scope locked. Numeric coefficients explicitly kept out of SPEC as tuning seeds
- 2026-04-24: Phase 0 — `design/month-3-vertical-slice.md` open questions resolved; Month-3 gate parameters locked (match type / first-3 signatures / 3-min recording artifact / football-literate observer criterion / observer-pool fallback)
- 2026-04-24: Phase 0 — `design/overview.md` open questions resolved; 4 pillar-level decisions locked via consolidated SPEC entry (nation framing / title / quickstart archetypes / pillar tiebreaker)
- 2026-04-22: Project bootstrapped from blueprint v2; composed profile (sim-management + action-character + narrative trimmings); research scope active
- 2026-04-22: 19 initial decisions logged to `SPEC.md` decisions log (append-only)
- 2026-04-23: Codex review appended Q32.32 fixed-point decision and tightened Month-3 slice scope
- 2026-04-22: All 11 design docs scaffolded with real content under `design/`

---

*Timestamp auto-maintained by `.claude/hooks/update-status-timestamp.sh`. Everything else Claude-edited per `/done`.*

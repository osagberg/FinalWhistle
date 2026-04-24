# Status

**Last updated**: 2026-04-24 (**Phase 1 ✅ COMPLETE.** Phase 2 🟡 ACTIVE — Design Bible / ADR authoring.)

## Currently working on

**Phase 2 — Design Bible** 🟡 ACTIVE.

Phase 1 shipped everything Claude-actionable (Tier-A CI green, banned-terms lint live, `fw verify` umbrella, full repo on `origin/main`). Phase 2 now focuses on ADR authoring — ordered to unblock Phase-3's real risk (first deterministic MatchSim + watchable 2D viewer). All 11 design docs are substantively locked per Phase-0 resolutions; the ADRs below finalize the architecture commitments.

**Phase-2 ADR priority order (Phase-3-unblocking first):**
1. ShotTypeSO schema + Addressables grouping
2. Viewer rendering pipeline + URP custom-pass ordering
3. Production pipeline ADR
4. Golden replay corpus format spec
5. Save migration fixture policy spec
6. MemoryEvent schema
7. SignatureSO schema
8. IdentityPacket / AI Content Compiler
9. Scout archetype (Phase-4 dependency)

Plus three new design docs: `modding.md`, `accessibility.md`, `content_policy.md`.

## Blockers

- None currently blocking Phase 2. Fresh-session project-scoped `.claude/` activation + plugin install via `.claude/bootstrap/scripts/install-plugins.txt` remain as Phase-1 carryover user-actions (see "Phase-1 carryover" below) — not Phase-2 blockers.

## Pending async

- ~~GitHub remote creation~~ — **done 2026-04-24**: `osagberg/FinalWhistle` private; `vibelogic` org deferred to Phase 8 transfer if publisher branding wants it.
- User creates Steam Direct account at Phase 8 ($100 one-time).
- Phase-8 prep: formal trademark + Steam-name clearance for "Final Whistle" (existing non-AAA uses: `finalwhistle.es`, `finalwhistle.club`).

## Open questions for user

- None currently blocking. All 11 design-doc open-question resolutions landed via 2026-04-24 Phase-0 consolidated SPEC entries. Phase-2 ADR authoring may surface narrow follow-ups per ADR; user gets each ADR for sign-off before it's marked Accepted.

## Next action

**ADR-0001 + 0002 + 0003 + 0004 Accepted. ADR-0005 Proposed** (just drafted with user's 6 pre-constraints baked in — no review round expected to surface major issues). `/next` picks up **ADR (6): IdentityPacket / AI Content Compiler** — houses the latent signature-affinity that ADR-0005 explicitly does NOT store, plus phenotype enum governance + content-pack ID rules.

## Phase-1 carryover (non-gating user-actions)

Phase 1 closed 2026-04-24 with these items intentionally open. None gate Phase 2 — they're timed by user convenience / phase triggers:

- Actions $0 spending cap (1-min GH UI step; runbook at `docs/ops/actions-budget.md §2b`). Recommended soon; not urgent
- Blender install (Phase-3 explicit trigger per `SETUP.md §4`)
- VS Code / Rider (user editor preference)
- Slash-command smoke + plugin install — `.claude/bootstrap/scripts/install-plugins.txt` (next fresh Claude session)
- Branch protection (blocked on plan upgrade — GitHub Free constraint; revisit Phase 4 closed-itch OR Phase 8 EA)

## Recent milestones

- 2026-04-24: Phase 2 — **ADR-0005 SignatureSO Proposed** with user's 6 pre-constraints baked in from first draft: event names via Memory.Contracts const references (not duplicate strings), explicit Scope enum, non-behavioral dependency metadata, field-level capped stacking with Id-tiebreak ordering, Display/ID lint-target separation, latent affinity explicitly not-here (lives in IdentityPacket per ADR-0006). Five rejected alternatives. Separate Signatures.Authoring / Runtime asmdefs; SimBiasSnapshot DTO boundary to MatchSim
- 2026-04-24: Phase 2 — **ADR-0004 Accepted** after five user-review tightenings (SalienceInputs + SalienceModelVersion persistence for audit; `FinalWhistle.Memory.Contracts` / impl split decouples MatchSim from persistence; quota rounding formula locked; event-class count corrected to ~40; float-salience added as 5th rejected alternative). Phase-6 content-pack validator SPEC task expanded to cover cross-doc event-class enum exact-match + `CallbackTag.ConsumingReaders ≥ 1`
- 2026-04-24: Phase 2 — **ADR-0002 Accepted** via self-review tightening pass (Knowledge Risk MEDIUM gate remains baked in; `fw shader-audit` promoted to explicit Phase-3 SPEC task). **ADR-0004 (MemoryEvent schema + CallbackTag registry + compaction tiers + migration framework) Proposed** — first real user of both save-migration-fixtures and golden-replay-corpus specs. Four rejected alternatives documented. Cross-doc exact-match discipline formalized for `SignatureAwakened`/`SignatureExecuted`/`ScoutReport*` enum names
- 2026-04-24: Phase 2 — Corpus spec tightened (Q1 converted to explicit Phase-3 SPEC task for `SerializationContract.cs`; seed policy locked at 1-seed Phase 3 / 3-seed Phase 6; generator owns key order). **Save-migration fixture policy spec** authored at `design/specs/save-migration-fixtures.md` — 4-test-per-schema-bump discipline (forward/callback-preservation/forward-incompat/round-trip); fixtures append-only; Phase 3 ~5 fixtures → Phase 8 ~15. `design/specs/` now holds 2 sibling specs
- 2026-04-24: Phase 2 — **ADR-0003 Accepted** after user tightenings (self-hosted-runner 4-condition acceptance gate, stale commit-hash removed). **Golden replay corpus spec** authored at `design/specs/golden-replay-corpus.md` — JSON fixture schema, Tier-A smoke seed `0xdeadbeefdeadbeef`, stable serialization rules, append-only growth policy, 3 Phase-3 open questions. New `design/specs/` subdirectory established
- 2026-04-24: Phase 2 — **ADR-0003 Proposed** (Production pipeline). 5-tier model + 5-channel build metadata + runner policy + cost discipline + release-gate rules formalized. Four rejected alternatives documented. Phase-1 validation criteria already satisfied (4 of 11)
- 2026-04-24: Phase 2 — **ADR-0001 Accepted** after user tightenings (ChainConditionId registry-backed / explicit deterministic-selection contract / per-content-pack Addressables grouping). **ADR-0002 Proposed** (Viewer rendering pipeline + URP custom-pass ordering). Knowledge Risk MEDIUM flagged on ADR-0002 — Unity 6 LTS URP 17+ Render Graph verification required at Phase 3 Week 1 spike. Two inline Category-B exemptions in place
- 2026-04-24: Phase 2 — ADR-0001 ShotTypeSO schema + Addressables grouping drafted as **Proposed** at `design/adr/adr-0001-shot-type-so-schema.md`. First Category-B inline exemption recorded (`term="domain"` ADR-template field). Awaiting user/GPT-5.5 sign-off before Accepted
- **2026-04-24: Phase 1 ✅ COMPLETE.** Phase 2 🟡 ACTIVE — Design Bible / ADR authoring. ADR order reprioritized per GPT-5.5 2026-04-24 guidance to feed Phase-3's playable slice (MatchSim + 2D viewer) rather than tidy-doc order. Stale SPEC tasks cleaned up (Task 52 Unity CI stub superseded, Task 50 accounts done, 11 design-doc locks marked)
- 2026-04-24: Phase 1 — `scripts/lint-banned-terms.py` shipped. Category-A hard-ban across 5 subsections from ui-vocabulary.md + Category-B soft-ban with inline `ui-lint:allow term=/reason=/reviewer=` exemption audit. Sentinel-aware (ui-lint:ignore-start/end), both-forms matching (per GPT-5.5 feedback). Wired as `fw banned-terms`, integrated into `fw verify` umbrella → Tier-A CI auto-picks up. 11 files sentinel-wrapped for legitimate meta-references; repo lint-clean. Resolved the GPT-5.5 tech-debt note about spaced-placeholder hack via proper sentinel discipline
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

---
description: ADR-0003 — Production pipeline. CI/CD tiers, runner policy, artifact retention, build channels, release gates, cost controls. Formalizes the 2026-04-24 production-pipeline planning pass.
---

# ADR-0003: Production pipeline — CI/CD tiers, runners, channels, release gates

## Status

**Accepted** — 2026-04-24. Tightened on self-hosted-runner acceptance gate + stale commit anchor removed per user review.

## Date

2026-04-24

## Last Verified

2026-04-24

## Decision Makers

osagberg (project owner), GPT-5.5 (planning author; 2026-04-24 production-pipeline report), Claude (workhorse).

---

## Summary

Solo-dev CI/CD as a 5-tier model: GitHub-hosted fast-PR checks (Linux-only baseline ≤5 min, with an explicit cross-platform carve-out for the deterministic-core dotnet test suite per SPEC 2026-04-28 decisions log), manual-dispatch Unity smoke, local/self-hosted heavy-sim work, paid-GitHub-minutes release-candidate matrix, and manual-approval-gated Steam deploy. Five named build channels (`dev`, `tester-closed`, `demo`, `ea`, `hotfix`) carry validation-tier metadata. No paid pipeline services through MVP; macOS-hosted runner minutes reserved for release-candidate runs only; Steam deploys never auto-fire on tag push.

## Engine Compatibility

| Field | Value |
|---|---|
| Engine | Engine-agnostic at this layer — pipeline wraps Unity 6 LTS (Phase 3+) + dotnet (MatchSim.csproj) + content-compiler scripts |
| Domain | Infrastructure / DevOps | <!-- ui-lint:allow term="domain" reason="ADR template canonical field name for engine-compat area" reviewer="osagberg" -->
| Knowledge Risk | LOW — GitHub Actions + `actions/checkout` + `actions/setup-dotnet` are well-documented stable APIs. Self-hosted-runner semantics may drift; verify before Phase-3 escalation |
| References Consulted | `design/production-pipeline.md` 2026-04-24 planning pass; GitHub billing + Actions + self-hosted-runner docs; GameCI Unity-builder docs |
| Post-Cutoff APIs Used | None — all referenced GitHub Actions APIs predate the knowledge cutoff |
| Verification Required | `actions/checkout@v4` Node-20 deprecation (noted on first Tier-A run 2026-04-24): swap to Node-24-compatible pin before 2026-09-16 GitHub forced-upgrade |

## Dependencies

| Field | Value |
|---|---|
| Depends On | None |
| Enables | Every downstream ADR that ships a testable artifact (golden replay corpus, save migration fixtures, content-pack validator, Tier-D RC matrix). Specifically: ADR-0004 (MemoryEvent — load-time migration path lands in Phase-6 CI); ADR-0005 (SignatureSO — bake-time validation); ADR-0006 (IdentityPacket — content-pack validator) |
| Blocks | Phase-3 Unity + MatchSim bootstrap (needs Tier-A already running against `dotnet test`); Phase-6 balance-harness nightly (Tier C); Phase-8 Steam deploy (Tier E) |

---

## Context

### Problem Statement

Solo-dev shipping a Unity game to Steam EA within 12 months faces a classic CI/CD trap: naive "put everything in GitHub Actions" bills macOS minutes at ~10× Linux cost and fails to scale to heavy balance-harness sweeps; the opposite trap ("I'll just run it locally") loses PR-gate discipline and pushes regressions into the shippable branch. A third trap (paid pipeline service — Buildkite / CircleCI / etc.) adds a ~$30-200/month line item before the project has audience revenue.

The production-pipeline planning pass on 2026-04-24 (`design/production-pipeline.md`) laid out the 5-tier model and 5-channel build-metadata model. This ADR promotes that plan into an Accepted architecture commitment and enumerates the rejected alternatives so future drift is a conscious choice, not a default.

### Current State

As of Phase 1 closure (2026-04-24):
- Tier A live and green (`.github/workflows/fast-pr-ci.yml` running `scripts/fw verify` — currently `verify-docs` + `banned-terms`)
- `scripts/fw` umbrella present; phase-gated stubs for `test` / `replay` / `content-lint` / `build-local` / `package-playtest`
- `docs/ops/branch-protection.md` + `docs/ops/actions-budget.md` + `docs/ops/backup-restore.md` runbooks written
- Branch protection blocked on GitHub Free plan (non-gating)
- No Tier-B / C / D / E workflows yet — those land at their phase triggers (3 / 3 / 8 / 8)

### Constraints

- **Zero paid pipeline services through MVP.** Tier-1 buy-on-pain per `SETUP.md §3`. Any flip is an explicit SPEC decisions-log entry.
- **Hard $0 spending cap on GitHub Actions.** User-action runbook at `docs/ops/actions-budget.md §2b`.
- **macOS-hosted runner minutes are ~10× Linux** (per GitHub billing docs 2026-04-24). Reserve for release-candidate runs; never let Tier-A / Tier-B default to macOS.
- **Deterministic replay is a first-class CI concern** — ADR-0001/0002's determinism contracts require a cross-platform verifiable path. Pass-activation log (not pixel compare) + canonical replay-corpus hashes are the mechanism.
- **Steam deploy is irreversible-ish** — bad build shipped to `default` branch hits real users. Manual-approval gate is non-negotiable.
- **Solo-dev attention budget** — tier proliferation has to pay back through bugs-caught-per-author-minute, not just coverage.

### Requirements

- **Functional:** 5 tiers with documented triggers + runners + scope; 5 build channels with validation-tier metadata; manual-approval gate on Tier E; local `scripts/fw` dispatch that mirrors Tier-A locally.
- **Cost:** Tier A must fit inside GitHub Free 2k (or Pro 3k) included minutes under typical solo-dev PR volume; Tier D may spend paid minutes; Tier C stays local.
- **Safety:** no auto-deploy to Steam `default` branch on any trigger; rollback build pre-tested before each EA release.

---

## Decision

### Five-tier workflow model (locked)

| Tier | Trigger | Runner | Budget | Scope |
|---|---|---|---|---|
| **A — Fast PR CI** | every PR + push to `main`/`develop` | GitHub-hosted Linux for general checks; **cross-platform matrix (Ubuntu + Windows + macOS) for `MatchSim.Tests`** as explicit carve-out (per SPEC 2026-04-28 decisions log — cross-platform determinism is the floor invariant the entire game depends on; matrix expansion beyond `MatchSim.Tests` requires a new SPEC decision) | ≤5 min | `scripts/fw verify` umbrella: docs + banned-terms now, `dotnet test` + determinism smoke + save-migration smoke Phase 3+, content-pack validator subset Phase 6+ |
| **B — Unity smoke** | manual dispatch / nightly (Phase 3+) | GitHub-hosted Win OR self-hosted | ≤30 min | Unity EditMode tests; one PlayMode subset; one build target; one viewer-capture smoke |
| **C — Heavy local** | local dev command / self-hosted schedule (Phase 3+) | **Local or self-hosted — never GitHub-hosted** | uncapped (local electricity) | 10K-match sweeps; full-season sim; replay-corpus regen + diff; full Unity matrix builds; visual-regression capture batches |
| **D — Release candidate** | tagged RC + manual dispatch (Phase 8) | GitHub-hosted (macOS acceptable here only) | willing to spend; quarterly-scale | Full `MatchSim.Tests` matrix; full Unity builds; content-pack validator full suite; save migration matrix; license audit; banned-term exemption audit |
| **E — Steam deploy** | final tag + **manual approval** (Phase 8) | GitHub-hosted (small) | minimal | Download RC artifacts; upload to Steam branch; **never** auto-deploys to public |

**Tier-A "umbrella inside `scripts/fw verify`" pattern:** new Tier-A checks wire into `fw verify` (not into the workflow file directly) as their phase triggers hit. Keeps the workflow diff-stable while the check surface grows. Parallel-job shape (commented reference in `fast-pr-ci.yml`) reserved for checks that need their own failure surface or independent timeout.

### Five build channels (locked)

| Channel | Audience | Validation gate | Distribution |
|---|---|---|---|
| `dev` | solo dev | Tier A | local only |
| `tester-closed` | Phase-4 closed itch | Tier A + Tier B | itch.io private |
| `demo` | Phase-6 public demo / Next Fest | Tier A + B + D | Steam demo app |
| `ea` | Phase-8 Early Access | All tiers | Steam EA |
| `hotfix` | emergency post-EA patches | Tier D abbreviated + manual QA | Steam EA |

Channel is embedded in build metadata (`channel: "ea"` in the build-info struct) for diagnostic reporting. `dev` never uploads to Steam; `hotfix` skips full content regression but requires explicit manual QA sign-off.

### Runner policy (locked)

- **GitHub-hosted Linux** — default for Tier A + Tier B (when Win target not needed) + Tier E (thin upload jobs).
- **GitHub-hosted Windows** — Tier B default when Windows-specific testing needed; Tier D full matrix.
- **GitHub-hosted macOS** — **Tier D only.** Never default for Tier A/B. First allowed use: pre-EA RC run (Phase 8).
- **Self-hosted runner (optional, Phase 3+)** — solo Mac runs Tier B/C when GitHub-hosted minutes are insufficient; restricted to labeled workflows only (never exposed to untrusted PRs — security-critical).

### Cost discipline (locked)

- Private repo. Hard Actions spending cap = **$0** (`github.com/settings/billing/spending_limit`). Overage blocks runs rather than charging payment method. User user-action per `docs/ops/actions-budget.md §2b`.
- No paid pipeline services through MVP (Buildkite / CircleCI / Codefresh / GitLab Enterprise / etc.). Flip requires explicit SPEC decisions-log entry.
- Every new `.github/workflows/*.yml` addition answers (in PR body): trigger frequency × duration × runner cost × expected monthly minutes × % of included budget. Reject if the answer isn't stated.

### Release-gate discipline (locked)

- **Tier E requires manual approval.** Not optional. Not "ok for patch-versions." Never.
- **Rollback build pre-tested** before each EA / hotfix release on a clean machine.
- **AI-content disclosure metadata** checked at Tier D (per Valve 2025 policy referenced in `SETUP.md §7`).
- **Release checklist** copied to version-specific doc per release (`release/v0.x.y-checklist.md`) — blueprint template at `steam-release/launch-checklist.md`.

### Architecture Sketch

```
                          ┌────────────────────────┐
PR / push to develop ──►  │ Tier A — Fast PR CI    │  Linux · ≤5min
                          │ scripts/fw verify      │  verify-docs + banned-terms +
                          │ (umbrella)             │  (Phase 3+) dotnet test + determinism
                          └───────────┬────────────┘
                                      │ green
                                      ▼
      manual dispatch     ┌────────────────────────┐
      or nightly ──────►  │ Tier B — Unity smoke   │  Win/self-hosted · ≤30min
                          │ EditMode + 1 build     │
                          └───────────┬────────────┘
                                      │
      local / self-hosted ┌────────────────────────┐
      ──────────────────► │ Tier C — Heavy local   │  uncapped · 10K sweeps,
                          │ never GitHub-hosted    │  full Unity matrix, replay corpus
                          └───────────┬────────────┘
                                      │ summary artifact uploaded
                                      ▼
      tagged RC + ───────►┌────────────────────────┐
      manual dispatch     │ Tier D — Release cand. │  macOS OK here · full matrix
                          │                        │  content validator + save migration
                          └───────────┬────────────┘
                                      │ RC green
                                      ▼
      final tag +   ─────►┌────────────────────────┐
      MANUAL APPROVAL     │ Tier E — Steam deploy  │  thin · upload to Steam branch
                          │ never auto-fires       │  NEVER public without 2nd approval
                          └────────────────────────┘

Build-channel metadata embedded per build: dev / tester-closed / demo / ea / hotfix
Validation-tier requirement scales per channel.
```

### Implementation Guidelines

- **Workflow files land at their phase triggers** — not preemptively. `fast-pr-ci.yml` live (Phase 1). `unity-smoke.yml` authored at Phase 3 kickoff. `release-candidate.yml` + `steam-deploy.yml` at Phase 8.
- **Tier-A umbrella pattern is the default.** New fast-PR checks land inside `scripts/fw verify`; parallel jobs only for checks that need their own failure surface / timeout / heavy setup.
- **Phase-3 self-hosted-runner verification:** before enabling a self-hosted Mac for Tier B/C, verify current GitHub self-hosted-runner free-for-Actions status (docs have drifted historically; verify before escalation) and confirm the runner is restricted to labeled workflows only.
- **Self-hosted runner security hard rule:** never exposes the runner to untrusted PRs (unverified external contributors don't exist here yet, but if Workshop modding ever changes that, the runner stays restricted).
- **Self-hosted runner acceptance gate (hard prereq before any runner registers):** the enabling PR must demonstrate all four conditions are satisfied. Day-one validation is manual checklist; a CI-side automated check can follow later if needed but is NOT required to register the runner.
  1. The workflow using the runner is triggered **only** by `workflow_dispatch` and/or `schedule` events — **never** by `pull_request`, `pull_request_target`, `issue_comment`, or any trigger that can fire from an external-contributor action.
  2. The workflow uses **explicit `runs-on` labels** that match ONLY the self-hosted runner tag (e.g. `runs-on: [self-hosted, fw-mac-local]`), never the bare `self-hosted` keyword which could match any registered runner.
  3. The self-hosted runner is registered with a **restricted label set** so only workflows using those exact labels can target it.
  4. The enabling PR body answers: "what's the blast radius if this runner executes arbitrary code?" with a concrete answer (e.g. "access to local repo clone + Unity license; no access to 1Password, no access to Steam credentials"). If the answer is hand-wavy, the runner doesn't register.

---

## Alternatives Considered

### Alternative 1: Paid pipeline service (Buildkite / CircleCI / etc.)

- **Description** — Move CI off GitHub Actions to a third-party service with better concurrency / macOS pricing / richer DSL.
- **Pros** — More powerful DSL, often cheaper macOS, sometimes better caching.
- **Cons** — Monthly cost ($30-200/mo) before audience revenue. Integration surface (secrets, Steam credentials, Unity license) is bigger. Extra service to maintain. Not a solo-dev-appropriate cost profile at Tier 1.
- **Rejected because** — Tier-1 buy-on-pain discipline. Revisit post-EA only if GitHub Actions proves inadequate at content-scaling phase.

### Alternative 2: Everything on GitHub Actions (no Tier C local)

- **Description** — Run 10K-match sweeps + full Unity matrix builds + visual-regression capture on GitHub-hosted runners.
- **Pros** — Single place for all automation; no self-hosted-runner setup.
- **Cons** — 10K-match sweeps on macOS-hosted minutes would blow the month's budget in a single run (rough math: 10K matches × 2s/match × $0.062/min = ~$20+ per sweep on macOS; Linux cheaper but still expensive for routine use). Balance-harness is better suited to local iteration where "I can re-run it any time" matters more than "it ran overnight on CI."
- **Rejected because** — cost + iteration-speed mismatch. Heavy sims belong local.

### Alternative 3: Auto-deploy to Steam on tag push

- **Description** — Tagging `v0.x.y` immediately deploys to Steam `default` branch.
- **Pros** — Fully automated release flow.
- **Cons** — A bad build in `main` becomes a live-audience bug at tag time with no human pause. Recovery is a hotfix-and-wait cycle while users have the bad build. Solo dev has no second pair of eyes; the manual-approval gate IS the second pair of eyes.
- **Rejected because** — release-safety non-negotiable for a 30-hour-career-save game. Save-corruption from a bad build is the worst-case player experience.

### Alternative 4: Single-tier mega-workflow

- **Description** — One workflow runs everything on every PR — docs lint + dotnet test + Unity build + balance harness + release readiness.
- **Pros** — No tier taxonomy to maintain.
- **Cons** — 5-minute PR-feedback budget can't coexist with 2-hour full-matrix validation. Either PR feedback becomes glacial, or full validation never runs. Tier separation exists because the feedback-loop requirement varies by 100× across check types.
- **Rejected because** — single-tier collapses either feedback speed or validation depth; losing either defeats the point of CI.

---

## Consequences

### Positive

- Cost-bounded by design. GitHub Free's 2k minutes handle Tier-A volume under realistic solo-dev PR cadence; overage blocks rather than bills.
- Heavy sim iteration (balance harness, replay corpus regen) runs at local iteration speed — no CI-queue wait.
- Steam deploys are human-gated; release-safety posture matches the product's save-data-stakes reality.
- Umbrella pattern (`fw verify`) keeps the workflow file stable as Tier-A coverage grows — minimizes workflow-yaml churn.
- Channel metadata in builds gives diagnostic bundles a clean provenance ("this crash came from a `tester-closed` build on commit X").

### Negative (Accepted Tradeoffs)

- Self-hosted-runner discipline is a non-trivial operational surface when it's turned on (Phase 3+ optional escalation). Hard rule on labeled-workflow-only restriction must be enforced manually until there's a validator.
- Manual-approval gate on Tier E adds a human step to every release. Low frequency (quarterly-ish post-EA), high value. Accepted.
- Channel taxonomy is one more thing to get right in build metadata; validator catches drift but doesn't prevent initial mistakes.

### Neutral

- New Tier-A checks land inside `fw verify` rather than as separate workflow jobs — slight loss in per-check failure surface (one job reports one umbrella pass/fail), traded for workflow-diff stability.
- Quarterly exemption audit ritual (per `docs/ops/branch-protection.md`) folds into the EA / RC gate — not a separate calendar item, just an RC-gate step.

---

## Performance Implications

| Metric | Target | Notes |
|---|---|---|
| Tier A duration | ≤5 min | Current: ~10s on just `fw verify`. Expected to grow with `dotnet test` (~2 min) + determinism smoke (~30s) + save-migration smoke (~30s) in Phase 3 |
| GitHub Actions monthly spend | $0 (hard cap) | Overage blocks; reassess cap at EA if paid overrides become necessary |
| Tier D duration | 20-60 min | Quarterly-scale; one run per RC |
| Tier C (local) duration | uncapped | Typical 10K-sweep: 5-10 min on mid-range Mac; overnight full-season batches |
| Local `fw verify` | <30s | Matches Tier-A locally for fast iteration |

---

## GDD Requirements Addressed

| GDD | System | Requirement | How This ADR Satisfies It |
|---|---|---|---|
| `design/production-pipeline.md` | CI/CD strategy | Tiered pipeline with local + GitHub-hosted runner separation | Tier A-E model + runner policy |
| `design/production-pipeline.md` | Cost discipline | No paid services; $0 Actions cap; macOS reserved | Runner policy + cost-discipline section |
| `design/production-pipeline.md` | Release-gate discipline | Manual-approval on Steam deploys | Tier E hard rule + rollback pre-test |
| `design/match-engine.md` | Cross-platform determinism | Win/Mac/Linux replay-hash parity | Tier A determinism smoke (Phase 3+); Tier D full matrix |
| `design/event-sourced-memory.md` | Save migration | Per-schema-bump fixture tests | Tier A save-migration smoke; Tier D full migration matrix |
| ADR-0001, ADR-0002 | Determinism contracts | Pass-log verification (no pixel compare) | Tier A pass-log hash check (Phase 3+); verifiable without GPU |

---

## Migration Plan

Not applicable — greenfield. Current Phase-1 state is the first tier (A) live; subsequent tiers land at their phase triggers per "Implementation Guidelines."

**Rollback:** if GitHub Actions proves inadequate at any phase and a paid service becomes necessary, supersede this ADR with a new one citing this one + rationale. Current expectation: no paid escalation through EA.

---

## Validation Criteria

- [x] Phase 1: Tier A `fast-pr-ci.yml` live and green on realistic PR flow (verified 2026-04-24 across 6 commits).
- [x] Phase 1: `scripts/fw verify` locally mirrors Tier-A output (verified 2026-04-24).
- [x] Phase 1: Spending cap runbook written (`docs/ops/actions-budget.md`); user-action carry-over.
- [x] Phase 1: Branch-protection runbook written (`docs/ops/branch-protection.md`); blocked on plan upgrade (non-gating).
- [ ] Phase 3: self-hosted-runner acceptance gate satisfied BEFORE any runner registers — manual checklist of the 4 conditions in the "Self-hosted runner acceptance gate" implementation guideline above: trigger-restricted to workflow_dispatch/schedule, explicit label match (no bare `self-hosted`), restricted label set on the runner, blast-radius written. Gate failure = no runner. No exceptions.
- [ ] Phase 3: `unity-smoke.yml` exists as manual-dispatch-only workflow; one target green.
- [ ] Phase 3: `fw verify` absorbs `dotnet test` + determinism smoke without busting 5-minute budget.
- [ ] Phase 3: Canonical replay-corpus hash verified identical across Win/Mac/Linux Tier-A runs.
- [ ] Phase 6: `fw verify` absorbs content-pack validator subset; full validator runs Tier D.
- [ ] Phase 8: `release-candidate.yml` green end-to-end on tagged RC.
- [ ] Phase 8: `steam-deploy.yml` exists with manual-approval gate; tested against a Steam beta branch before EA tag.
- [ ] Ongoing: monthly check that Tier A has NOT silently exceeded the 5-minute budget as checks accrete.

---

## Related

- Depends on: none (foundational).
- Enables: ADR-0004 (MemoryEvent migration framework — Tier-A save-migration smoke), ADR-0005 (SignatureSO — bake-time validation), ADR-0006 (IdentityPacket — content-pack validator), every phase-gated ADR that ships a testable artifact.
- Cross-refs: `design/production-pipeline.md` (source planning pass), `SETUP.md §3` (Tier-1 buy-on-pain), `docs/ops/branch-protection.md`, `docs/ops/actions-budget.md`, `docs/ops/backup-restore.md`.
- Code: `.github/workflows/fast-pr-ci.yml` (live), `scripts/fw` (live), future `.github/workflows/unity-smoke.yml` + `release-candidate.yml` + `steam-deploy.yml` (phase-triggered).

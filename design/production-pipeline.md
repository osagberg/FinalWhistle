---
description: CI/CD tiers, local-vs-GitHub runner policy, artifact retention, build channels, release gates, cost controls. Authoritative production pipeline plan for Final Whistle.
last_verified: 2026-04-24
status: Phase 0 planning pass complete; authored per GPT-5.5 production-pipeline report. Implementation seeds live in Phase 1/2/3/4-6/8 SPEC tasks. Phase-2 ADR pre-seeded.
---

# Production Pipeline — CI/CD, builds, release ops

## Purpose

Answer "what runs where, when, and at what cost, so that Final Whistle ships as a disciplined solo-AI-native production without paying for pipeline infrastructure we don't yet need?"

The core tension: **GitHub is cheap for source-of-truth + small PR checks; Unity CI is slow, license-sensitive, and expensive (macOS especially); heavy sim sweeps are cheap locally and expensive in the cloud.** This doc distributes work to the cheapest correct place.

## Locked decisions (2026-04-24)

- **GitHub is source-of-truth.** Private repo. Source control, PR review, issues, design-doc history, lightweight CI, small artifacts, release tags. Never made public just to save Actions minutes.
- **Fast PR CI first; Unity CI later; heavy work local.** Tiering is explicit (see "Workflow tiers" below).
- **Release CI is manual-approval only.** Steam deploys never trigger automatically on a tag push.
- **Hard budget cap on GitHub Actions usage.** Usage stops at included-minutes cap; overage is off by default. Tier A (fast PR) must fit inside the included-minutes budget for a typical month of solo-dev PRs.
- **No paid pipeline services at MVP.** No Buildkite / CircleCI / Codefresh / etc. GitHub Actions + local + optional self-hosted runner only.
- **Architecture from day one, not Product MVP.** This is infrastructure seeded into SPEC / ADRs, not a Steam-facing feature.

## Cost posture

Source: GitHub docs as of 2026-04-24 (see references at bottom of doc).

- Private repos: Free tier = 2,000 Actions minutes/month; Pro = 3,000/month.
- Runner minute rates (baseline, subject to change): Linux 2-core ~$0.006/min, Windows 2-core ~$0.010/min, **macOS ~$0.062/min** (10x Linux).
- Self-hosted runners are currently free for Actions usage — **verify before relying on it and set a hard budget cap regardless.**
- GameCI Unity builds require Unity activation / license secrets.

**Implication:** every Unity minute on GitHub-hosted runners is ~10x a dotnet-only Linux minute. Every macOS minute is ~10x a Linux minute. Budget accordingly.

## Workflow tiers

Five tiers. Each tier has a trigger, a budget, and a scope. Higher tiers do not run automatically — they require manual dispatch or explicit release gating.

### Tier A — Fast PR CI

**Trigger:** every PR to `develop`, every push to `develop` / `main`.
**Runner:** GitHub-hosted Linux **for general checks**; cross-platform matrix (Ubuntu + Windows + macOS) **for the deterministic-core dotnet test suite** (carve-out per SPEC 2026-04-28 decisions log: cross-platform determinism is the floor invariant the entire game depends on, so `MatchSim.Tests` runs on all three platforms even at the Tier-A budget). Matrix expansion BEYOND `MatchSim.Tests` requires a new SPEC decision.
**Budget:** ≤5 minutes per run. Must fit inside included-minutes envelope for typical solo-dev PR volume.
**Scope:**
<!-- ui-lint:ignore-start reason="meta-reference to placeholder tokens being lint-checked" -->
- Markdown placeholder-leak check (template tokens like `{{PROJECT_NAME}}` / `{{STUDIO}}` — catches both spaced and unspaced forms — plus `TODO:` and unresolved template tokens).
<!-- ui-lint:ignore-end -->
- Markdown frontmatter validation (every design doc has `description`).
- Banned UI-vocabulary lint (`scripts/lint-banned-terms.py` — Phase-1 deliverable).
- JSON / content-pack schema validation (once content exists).
- Content-pack ID stability checks (no mutated IDs between pack versions).
- `dotnet test` on `MatchSim.Tests` (unit tests; fast path).
- Deterministic replay hash smoke test (one canonical seed).
- Save migration fixture smoke test (once fixtures exist).

**Explicitly NOT in Tier A:** Unity Editor boot, scene captures, balance-harness sweeps, full-matrix determinism, build artifacts.

### Tier B — Unity smoke

**Trigger:** manual dispatch / nightly (after Phase 3 Unity project exists).
**Runner:** GitHub-hosted Windows OR Linux (avoid macOS-hosted except for RC gate — see Tier D). Self-hosted runner preferred once local Mac is available.
**Budget:** ≤30 minutes per run.
**Scope:**
- Unity EditMode tests.
- A selected small PlayMode test subset.
- One build target (probably Windows standalone or WebGL).
- One viewer-capture smoke (renders a canonical 30s MatchSim segment; diff-checks one keyframe hash).

**Does not run on every PR.** Gate is manual dispatch or nightly schedule to avoid burning the included-minutes budget on Unity compilation overhead.

### Tier C — Heavy local

**Trigger:** manual local command, OR self-hosted runner on schedule.
**Runner:** **Local dev machine OR self-hosted runner — never GitHub-hosted.**
**Budget:** uncapped locally (it's your electricity).
**Scope:**
- 10K-match balance-harness sweeps.
- Full-season simulation batches.
- Golden-replay-corpus regeneration and diff.
- Performance benchmarks (frame-budget, MatchSim tick budget).
- Visual-regression capture batches.
- Unity full-matrix (Win/Mac/Linux) builds for pre-RC validation.

GitHub stores the summary artifact (JSON digest, screenshots, log tail) uploaded from the local run. The expensive work itself stays local.

### Tier D — Release candidate

**Trigger:** tagged RC (`v0.x.0-rc.N`) + manual workflow dispatch.
**Runner:** GitHub-hosted (acceptable to burn macOS minutes here — infrequent).
**Budget:** willing to spend. Run quarterly or less.
**Scope:**
- Full `MatchSim.Tests` matrix (Win/Mac/Linux).
- Full Unity build targets (Win/Mac/Linux — macOS included).
- Content-pack validation (see core systems below).
- Save migration matrix (v1 → current on every version pair).
- Asset-license audit.
- Release-checklist automation hooks.
- Banned-term exemption audit report (per `design/ui-vocabulary.md` Category-B discipline).

### Tier E — Steam deploy

**Trigger:** final tag (`v0.x.0`) + **manual approval.**
**Runner:** GitHub-hosted (small, just uploads artifacts).
**Budget:** minimal.
**Scope:**
- Download/prepare artifacts from Tier-D RC run.
- Upload to Steam branch (`default` or `beta` based on approval).
- Never deploys direct to `default` / public without a second approval.
- Steam deploy is the only tier that gates on human-in-the-loop for every run.

## Repo structure (planned)

```
.github/
  workflows/
    fast-pr-ci.yml            # Tier A (Phase 1)
    unity-smoke.yml           # Tier B (Phase 3+, manual dispatch)
    release-candidate.yml     # Tier D (Phase 8)
    steam-deploy.yml          # Tier E (Phase 8, manual approval)
  PULL_REQUEST_TEMPLATE.md    # Phase 1
  ISSUE_TEMPLATE/
    bug_report.md             # Phase 1
    feature_request.md        # Phase 1

scripts/
  fw                          # local command front-door (Phase 3+)
  lint-banned-terms.py        # Phase 1
  verify.sh                   # project-wide pre-commit smoke (Phase 1)

docs/
  ops/
    local-runner.md           # optional (Phase 3+)
    playtest-distribution.md  # Phase 4
    release-channels.md       # Phase 8
    crash-logs-telemetry.md   # Phase 5+
    backup-restore.md         # Phase 1
```

## Core systems owed by pipeline discipline

These are architectural deliverables that the pipeline depends on. Each has a pre-seeded SPEC task in the phase where it's needed.

### Golden replay corpus (Phase 2 spec, Phase 3 implement)

A small canonical set of match seeds that protect the sim from regression. Each corpus entry stores:

- `match_seed: u64`
- `content_pack_version: string`
- `home_archetype` + `away_archetype` (BT archetype IDs)
- `expected_final_score: (u8, u8)`
- `expected_key_event_hashes: [hash]` — hash of each ledger-emission event in order
- `expected_final_canonical_state_hash: hash`

Tier A runs ONE corpus seed as smoke (fast). Tier C regenerates / diffs the full corpus. Tier D runs the full corpus as regression check. **Corpus drift is caught at Tier D, not at content-pack-release time.**

### Save migration fixtures (Phase 2 spec, Phase 6 implement)

Every schema bump ships with:

- A **previous-version save fixture** (checked into repo as test data).
- A **migration test** that loads the previous-version fixture with the current build and asserts successful migration.
- A **callback-eligibility preservation test** — compacted events' callback tags survive the migration.
- A **failure test** confirming a newer-schema save loaded in an older build fails cleanly (no silent corruption).

Fixtures accumulate in `MatchSim.Tests/fixtures/saves/` forever. Deleting a fixture requires a SPEC decision citing why.

### Content pack validator (Phase 2 spec, Phase 6 implement)

Before any content pack lands in `main`:

- Duplicate names (player / club / stadium) check.
- Legal-sensitive names check (real-player-name database diff).
- Missing localized strings check.
- Invalid phenotype IDs (against `design/player-generation.md` authoritative catalog).
- Invalid signature IDs (against `design/signatures.md` authoritative catalog).
- Broken event-class enum references (against `design/event-sourced-memory.md`).
- Unresolved content-pack-qualified IDs.
- Real-world analogue place-name leakage (per `design/worldbuilding.md` 2026-04-24 lock).
- Banned UI vocabulary (per `design/ui-vocabulary.md` Categories A.1-A.5).

Validator runs in Tier A for schema/ID checks (fast) and Tier D for the full suite including the legal-sensitive-names diff (slower, dataset-dependent).

### Local command front-door (Phase 3+)

Bash / Makefile-first; no paid task runner. Commands:

```
scripts/fw verify           # project-wide smoke (all Tier A checks, locally)
scripts/fw test             # dotnet test on MatchSim.Tests
scripts/fw replay <seed>    # run one canonical replay, print final hash
scripts/fw content-lint     # full content pack validator locally
scripts/fw build-local      # Unity build, current platform only
scripts/fw package-playtest # build + itch-packaging for tester distribution
```

Keep it cheap — avoid introducing a dependency-heavy task runner (Task, Just, Nx, etc.) unless the Bash version becomes painful.

### Playtest ops (Phase 4)

No cloud telemetry at MVP. Local-first distribution:

- Local build zip with embedded seed + content-pack version.
- itch.io private / restricted page for Phase-4 closed itch distribution.
- Feedback via Google Form OR local markdown bug-report template in the build.
- Build includes "Export bug bundle" button that produces a zip containing:
  - Current save file
  - Last N replay seeds
  - Rolling logs
  - Player settings
  - Content-pack version manifest
- Testers email or itch-message the zip; no cloud ingest endpoint.

### Crash / log / telemetry (Phase 5+)

Local-first, opt-in, minimal:

- Rotating local logs (bounded size).
- Exportable diagnostics zip (same shape as playtest bug bundle).
- **Optional** anonymous playtest JSON — opt-in only, imported locally into SQLite / CSV for analysis.
- No PII by default. No cloud endpoint at MVP.

### Backup policy (Phase 1)

Explicit, written, not assumed:

- GitHub for code + docs (source-of-truth).
- Git LFS only if genuinely needed (Unity binary assets at Phase 3+) — not by default.
- Local Time Machine for Unity / Blender working files.
- Periodic zipped content-pack snapshots before destructive imports.
- `Library/` NOT backed up (regenerable).
- Asset source files (Blender, Aseprite, raw Suno/Udio stems) backed up before destructive imports.

## Build channels

Named channels, separated from the commit graph. The same git SHA can produce different channel builds with different content-pack pinning + validation severity.

| Channel | Audience | Validation gate | Distribution |
|---|---|---|---|
| `dev` | solo dev | Tier A | local only |
| `tester-closed` | Phase-4 closed itch | Tier A + Tier B | itch.io private |
| `demo` | Phase-6 public demo / Next Fest | Tier A + B + D | Steam demo app |
| `ea` | Phase-8 Early Access | All tiers | Steam EA |
| `hotfix` | emergency post-EA patches | Tier D abbreviated + manual QA | Steam EA |

Channel-specific metadata is embedded in the build (`channel: "ea"`) for diagnostic reporting. A `dev` build never uploads to Steam; a `hotfix` build skips full content regression but requires explicit manual QA sign-off.

## Branch protection + PR discipline (target process — current posture is direct-to-main)

> **Current posture (2026-04-26 onward):** direct-to-main solo-dev mode per CLAUDE.md §5.6. GitHub Free does not allow private-repo branch protection; until that constraint lifts, direct scoped commits to `main` are acceptable provided `scripts/fw verify` is green before each commit. The PR-only discipline below becomes active when (a) GitHub plan upgrade enables private-repo branch protection, OR (b) the user explicitly opts in to PR-only mode (e.g. when the first external collaborator joins). Migration runbook: `docs/ops/branch-protection.md` — flips the protections + the PR template when the trigger fires.

**Target process (when branch protection becomes available):**

- `main` protected: no direct pushes; PR + approval required.
- `develop` protected: no force pushes; PR + Tier-A-green required.
- Feature branches (`feat/<name>`, `fix/<name>`) — free-fire, PR into `develop`.
- **No squash-and-rebase-without-verification.** History on `main` must preserve CHANGELOG traceability.
- PR template (Phase 1 deliverable):
  - Summary
  - Why
  - Test plan (checkbox list)
  - Breaking changes (schema bump? content-pack version bump?)
  - Linked SPEC task / ADR
- Issue templates (Phase 1):
  - Bug report (with diagnostics-bundle ask)
  - Feature request (with SPEC-task-candidate tag)

## MVP boundary

At **Phase 1 (bootstrap)**:
- Tier A runs on GitHub Actions.
- PR + issue templates in place.
- Branch protection configured.
- `scripts/fw` front-door skeleton (empty or no-op commands acceptable).
- Backup policy written.
- GitHub Actions budget cap set.

At **Phase 3 (first Unity build)**:
- Tier B exists as manual-dispatch workflow.
- Local MatchSim CI scripts runnable from `scripts/fw`.
- Determinism-matrix smoke (Win/Mac/Linux via GitHub-hosted) green on `MatchSim.Tests`.
- Unity full-build remains local until Phase-8 RC.

At **Phase 4-6 (content + season scaling)**:
- Playtest build-distribution flow runnable end-to-end.
- Crash / log bundle exporter implemented.
- Balance-harness Tier-C local runs produce uploadable summary artifacts.
- Save-compatibility fixtures checked in; migration test green in Tier A.
- Content-pack validator green in Tier A + Tier D.

At **Phase 8 (EA launch)**:
- Tier D release-candidate workflow green end-to-end.
- Tier E Steam deploy workflow exists but manual-approval-gated.
- Release checklist version-specific doc copied from template.
- Rollback build tested.
- AI-content disclosure metadata checked.
- Asset-license tracker complete.

## Deferred

- Paid pipeline services (Buildkite / CircleCI / etc.) — ruled out through MVP; revisit only if solo-dev pipeline proves inadequate at post-EA scale.
- Cloud telemetry ingest — post-EA only, opt-in, minimal PII.
- Cross-save sharing / Legend Exchange backend — post-1.0.
- Automated store-asset generation (screenshots, trailers) — evaluated at Phase 7 polish.
- Self-hosted runner cluster — single self-hosted Mac sufficient through EA.

## Prototype gate

**Phase 1 gate:** Tier A runs green on the first real PR. `scripts/fw verify` produces the same result locally as Tier A produces on GitHub.

**Phase 3 gate:** Determinism-matrix smoke green on Win/Mac/Linux (per `design/match-engine.md` prototype gate). Month-3 slice's 3 signatures + 3 shot types build locally via `scripts/fw build-local`.

**Phase 6 gate:** Tier-C local balance-harness produces a 10K-sweep summary artifact uploadable to GitHub. Content-pack validator catches an intentionally-broken pack (red-team test).

**Phase 8 gate:** Tier E deploy workflow runs end-to-end against a Steam beta branch with manual approval. Rollback build verified on a clean machine.

## Cross-refs

- `TECH_APPROACH.md` — engineering architecture; Production Pipeline section cross-refs here.
- `design/match-engine.md` — determinism test + CI matrix owner.
- `design/event-sourced-memory.md` — save migration framework owner.
- `design/player-generation.md` + `design/signatures.md` + `design/worldbuilding.md` + `design/ui-vocabulary.md` — content-pack validator consumes all four as authoritative catalogs.
- `SETUP.md` — trigger-table for deferred purchases / licenses (paid pipeline services never triggered by MVP).
- `SPEC.md` — Phase-1/2/3/4-6/8 tasks pre-seeded 2026-04-24.

## References

- GitHub included usage (Free 2k/Pro 3k Actions minutes): https://docs.github.com/en/billing/reference/product-usage-included
- GitHub Actions billing + per-minute rates: https://docs.github.com/en/billing/concepts/product-billing/github-actions
- GitHub self-hosted runners (free for Actions usage as of doc review): https://docs.github.com/en/actions/reference/runners/self-hosted-runners
- GameCI Unity activation: https://game.ci/docs/2/github/activation/
- GameCI Unity builder: https://game.ci/docs/github/builder/

Cost facts above reflect GitHub documentation as of 2026-04-24. Verify before relying on them for budget planning; rates and included-minutes tiers change.

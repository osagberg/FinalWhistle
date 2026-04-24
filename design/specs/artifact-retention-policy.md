---
description: Artifact retention policy specification. Per-artifact-class TTLs across 5 retention tiers (ephemeral / short / release-tied / permanent-in-repo / local-only). Keeps storage costs inside GitHub Free/Pro caps while preserving release-reproducibility + determinism-replay for every shipped build.
last_verified: 2026-04-24
status: Phase 2 spec — retention tiers + per-class TTLs locked; cleanup automation manual at Phase 3 (`fw artifact-cleanup`), uploaded-artifact retention via workflow `retention-days:` + repo/org Actions retention-setting audit from Phase 6 onward, release bundles verified as GitHub release assets.
---

# Artifact Retention Policy — specification

## Purpose

Pin the TTL and storage location of every artifact the production pipeline produces — so (a) Actions storage costs stay inside Free 2k / Pro 3k minute caps + the 500MB-2GB storage ceiling; (b) every shipped release is reproducible from archived artifacts years later; (c) nothing load-bearing expires silently; (d) solo-dev has a single reference to consult instead of re-deriving retention per pipeline change.

Framing: the pipeline already defines *what* gets produced (per `design/production-pipeline.md` 5-tier model + ADR-0003). This spec defines *how long each thing lives*, *where it lives*, and *what happens when it ages out*.

## Why this spec exists (not an ADR)

The architectural decisions are already locked:

- **ADR-0003** — 5-tier CI/CD model (A / B / C / D / E); GitHub-hosted vs local/self-hosted runner discipline; manual-approval-only Steam deploy through EA. Retention is declared as "owed" in both the ADR description and the sibling `production-pipeline.md`.
- **`design/production-pipeline.md`** — cost discipline posture: "Private repo. Hard Actions budget cap (Free 2k / Pro 3k included minutes). Overage off by default."
- **`design/specs/golden-replay-corpus.md`** — corpus fixtures are append-only, checked into the repo. Retention is "forever".
- **`design/specs/save-migration-fixtures.md`** — fixtures are append-only, checked into the repo. Retention is "forever".
- **`design/specs/content-pack-validation-contract.md`** — red-team fixtures are append-only, checked into the repo. Retention is "forever".
- **`SETUP.md §7 + §8`** — AI-content-disclosure metadata shipped with every build; Time Machine covers local backup.

This spec defines the **on-disk / GitHub-storage layout + TTL-per-class + release-archival rule + cleanup automation posture** that implements those commitments.

## Locked decisions

- **Five retention tiers, no sixth.** Every artifact lives in exactly one tier:
  - `ephemeral` — ≤7 days GitHub Actions retention for PR-scoped uploaded artifacts via per-step `retention-days: 7`.
  - `short` — 14-30 days GitHub Actions retention. Tier-C summary uploads, nightly-Tier-B outputs.
  - `release-tied` — retained as GitHub release-assets attached to the tag; persists as long as the release exists (effectively permanent for shipped builds).
  - `permanent-in-repo` — checked into the repo alongside source; version-controlled forever. Fixtures.
  - `local-only` — never uploaded to GitHub. Stays on dev machine (Time Machine covers per `SETUP.md §8`). Raw balance-harness outputs, visual-regression reference captures, tester-submitted bug bundles.
- **Uploaded Actions artifacts declare `retention-days` at workflow level** via GitHub Actions `retention-days` attribute on every `upload-artifact` step. Per-workflow, not per-repo default. GitHub workflow logs are platform-managed logs governed by the repo/org Actions artifact-and-log retention setting; they are reported by audit tooling but are not part of the five artifact TTL tiers because that shared setting must stay high enough to allow 30-day short-tier artifacts. Release-tied bundles are GitHub release assets and are never modeled as `upload-artifact` outputs. Phase-3 manual audit; Phase-6 `fw workflow-audit` CLI tool enforces declared retention matches this spec (SPEC task owed).
- **Every RC tag permanently retains its full artifact bundle** as a GitHub release-asset (manual upload gated by Tier-E workflow approval). Losing an RC artifact bundle means losing the ability to reproduce that shipped build; storage cost is negligible compared to the reproducibility guarantee.
- **Determinism-replay posture:** any shipped build must be reproducible from its release-tied artifact bundle + the repo at its tagged commit. Archived bundles include: full validator Tier-D report, Category-B exemption audit JSON, AI-content-disclosure manifest hash, golden-replay-corpus hashes for that pack version, pack-manifest `canonical_artifact_sha256`. Together with the git tag, these uniquely reproduce the build.
- **No user data ever enters any retention tier without explicit opt-in.** Tester-exported bug bundles (Phase 4+) are `local-only` by design — testers email the zip; it stays on dev machine; no cloud ingest at MVP per `design/production-pipeline.md §Playtest ops`.
- **Cleanup is manual at Phase 3, workflow-declared at Phase 6 for uploaded Actions artifacts, and release-asset-verified for shipped builds.** The `fw artifact-cleanup` CLI covers the Phase-3 gap (list old artifacts; prompt for deletion; verify release assets); workflow `retention-days:` attributes take over automatically for uploaded Actions artifacts once all workflows carry them. No third-party retention-management tooling.

## Artifact catalog (by retention tier)

### Ephemeral (≤7 days Actions retention)

Every CI tier's per-run transient output. Lost-is-fine; re-runnable by re-triggering the workflow.

| Artifact class | Source | Storage | TTL | Rationale |
|---|---|---|---|---|
| Tier-A `fw verify` output (verify-docs + banned-terms + future content-lint) | `fast-pr-ci.yml` | Actions artifacts | 7 days | Diagnostic only; re-runnable |
| Tier-A `fw content-lint --format=json` output | `fast-pr-ci.yml` (Phase 3+) | Actions artifacts | 7 days | Per-PR; superseded by merge |
| Tier-B Unity-smoke build outputs | `unity-smoke.yml` (manual-dispatch) | Actions artifacts | 7 days | Not shipping artifacts; sanity-check only |
| Tier-B one viewer-capture PNG | `unity-smoke.yml` | Actions artifacts | 7 days | Re-runnable; Phase-3 only when viewer exists |
| Red-team validator self-check output | `fast-pr-ci.yml` (Phase 3+) | Actions artifacts | 7 days | Confirms checks fire; no historical value |

### Short (14-30 days Actions retention)

Longer window because nightly / weekly cadence means a 7-day TTL could lose the last run.

| Artifact class | Source | Storage | TTL | Rationale |
|---|---|---|---|---|
| Tier-C balance-harness summary artifact (10K-sweep JSON digest) | self-hosted / local → `upload-artifact` | Actions artifacts | 30 days | Nightly cadence at Phase 6+; 30d covers a month-of-trend observation |
| Tier-C viewer-capture reference PNG set | self-hosted / local | Actions artifacts | 30 days | Visual regression diffs run against this; superseded when a new reference uploads |
| Tier-C golden-replay-corpus diff output | self-hosted / local → `fw replay --regenerate-corpus` | Actions artifacts | 30 days | Regeneration cadence slow; diff drives next corpus commit |
| Tier-B nightly Unity smoke bundle | `unity-smoke.yml` scheduled | Actions artifacts | 14 days | Two weeks of failure archaeology enough; older runs usually superseded |
| Tier-D validator dry-run (non-tagged manual dispatch) | `release-candidate.yml` manual | Actions artifacts | 30 days | Pre-RC sanity pass; final RC bundle tier-promotes to release-tied |

### Release-tied (permanent while the release exists)

Every tagged RC + final Steam-deployed release carries its full artifact bundle as GitHub release-assets attached to the tag. The release-tag's GitHub retention is "until deleted"; we never delete.

| Artifact class | Source | Storage | TTL | Rationale |
|---|---|---|---|---|
| Tier-D full validator report (JSON) | `release-candidate.yml` | GitHub release-asset on tag | permanent | Reproducibility of RC + audit trail |
| Category-B exemption audit JSON | `fw banned-terms --report` | GitHub release-asset on tag | permanent | Required for every RC + EA lock per `design/ui-vocabulary.md` |
| AI-content-disclosure manifest snapshot | pack-manifest block extract | GitHub release-asset on tag | permanent | Steam 2025 policy compliance trail; per-release immutable |
| Golden-replay-corpus hashes at release-commit | `fw replay --export-hashes` | GitHub release-asset on tag | permanent | Cross-platform determinism replay verifiable years later |
| Steam deploy bundle manifest | `steam-deploy.yml` | GitHub release-asset on tag | permanent | Pre-upload inventory; reproducibility backstop |
| Full asset-licensing-tracker snapshot | `steam-release/asset-licensing-tracker.csv` at tagged commit | git (tag) | permanent | Legal-compliance evidence at release time |
| Save-migration test results at release | `fw test --save-migration --json` | GitHub release-asset on tag | permanent | Every shipped schema's migration path provably tested at that release |
| Phase-8+: PEGI / ESRB questionnaire PDFs | `design/content_policy.md §Phase-8 gate` output | GitHub release-asset on tag | permanent | Rating-submission evidence |

**Release-asset size budget:** total artifact bundle per tag targeted ≤100MB. If a bundle grows past 250MB, audit it — likely a raw log or binary snuck in where a summary should be.

### Permanent (in-repo, forever)

Checked-in source that never expires.

| Artifact class | Source | Storage | TTL | Rationale |
|---|---|---|---|---|
| Golden-replay-corpus fixtures | `MatchSim.Tests/fixtures/replay-corpus/*.json` | git | permanent (append-only) | `golden-replay-corpus.md §Locked decisions` |
| Save-migration fixtures | `MatchSim.Tests/fixtures/saves/*.json` | git | permanent (append-only) | `save-migration-fixtures.md §Locked decisions` |
| Red-team validator fixtures | `MatchSim.Tests/fixtures/validator-red-team/FW-VAL-*.pack/` | git | permanent (append-only) | `content-pack-validation-contract.md §Growth policy` |
| Anti-red-team clean fixture | `MatchSim.Tests/fixtures/validator-clean/minimal.pack/` | git | permanent | Negative-control, same spec |
| Synthetic thin-mod-pack fixture | `MatchSim.Tests/fixtures/mod-packs/thin-mod.fwh.mod.v1/` | git | permanent (Phase-6 onward) | `modding.md §Prototype gate` + `content-pack-validation-contract.md §Synthetic thin-mod-pack` |

### Local-only (never uploaded)

Personal dev-machine or tester-machine only; Time Machine covers per `SETUP.md §8`.

| Artifact class | Source | Storage | TTL | Rationale |
|---|---|---|---|---|
| Balance-harness raw sweep data (pre-digest) | `fw balance-harness` local output | local `balance-output/` (gitignored) | user-managed | Raw 10K-sweep outputs can be 100s of MB; only the digest uploads |
| Visual-regression reference captures (full set) | local Unity capture tool | local `viewer-captures/` (gitignored) | user-managed | Full set is 100s of PNGs per viewer version; only diff uploads to Actions |
| Tester-submitted bug-bundle zips | itch.io-distributed tester → email/itch DM | local `playtest-bundles/` (gitignored) | user-managed | No cloud ingest at MVP per `production-pipeline.md §Playtest ops` + `design/content_policy.md §Mod-pack content-safety`. Each bundle contains save + replay-seeds + rolling logs + settings + pack version; PII-free per bundle spec but treated as user-data-respectful |
| Dev-side crash-log bundles (Phase 5+) | `scripts/fw crash-export` local | local `crash-bundles/` (gitignored) | user-managed | Rotating logs; `docs/ops/crash-logs-telemetry.md` (Phase 5 owed) covers |
| Local `.blend` / Blender source files (Post-EA 3D push) | local Blender | local + cloud storage | user-managed | Per `SETUP.md §8 Backup strategy` — cloud storage (not git) |

## Cost discipline

### GitHub Actions storage cap math

GitHub Free: 500MB artifact storage + 2000 minutes/month. GitHub Pro (if upgraded): 1GB storage + 3000 minutes/month.

**Daily artifact footprint estimate at Phase 3:**
- Tier-A CI per-PR (assume ~5 PRs/week): ~5 × 2MB × 7 days = ~70MB active at any time
- Tier-B nightly (weekly cadence): ~1 × 40MB × 14 days ≈ 80MB active (if scheduled)
- Tier-C local upload (if run): ~1 × 10MB × 30 days = ~300MB active if run daily

**Running total at Phase 3:** comfortably <500MB even on Free plan.

**Phase 6 projection** (10K-sweep digest + red-team fixtures + content-pack validator Tier-D dry-runs):
- Tier-C sweep digest nightly: ~30 × 10MB = ~300MB active
- Tier-D validator dry-runs (weekly): ~4 × 20MB = ~80MB active
- Plus Tier-A + Tier-B ongoing

**Running total at Phase 6:** ~500-700MB active. Free plan hits cap; Pro plan (1GB) comfortable. Decision point: if Phase-6 pipeline pressures the cap, upgrade to Pro ($4/mo) before rewriting retention math. `design/production-pipeline.md §Cost discipline` already names this.

**Release-asset bundles are NOT counted in the Actions storage cap** — they're release-asset storage, which is effectively free under GitHub TOS for private repos (no published cap at the time of writing).

### Minutes budget

Retention affects storage, not minutes directly. Minutes math is governed by CI workflow duration per run × frequency; unchanged by retention policy.

### Cleanup triggers

- Solo-dev monthly check (manual, ~5 min): `fw artifact-cleanup --list` shows per-workflow artifact sizes; delete anything anomalously large.
- Phase-6 onward: CI enforces `retention-days:` per workflow upload step for uploaded Actions artifacts; repo/org Actions retention setting is audited for awareness but does not replace per-artifact TTLs.
- EA-lock artifact audit: `fw artifact-cleanup --release-lock` asserts every release-asset for tagged versions resolves + checksums match its manifest; any drift = investigation.

## RC / EA-launch archival rule

Every RC tag + every EA build tag carries its full artifact bundle as GitHub release-assets. The bundle is created by the `release-candidate.yml` workflow's final step (or the `steam-deploy.yml` for EA builds); manual upload occurs as part of the workflow's manual-approval gate. Losing this bundle means the build is no longer reproducible from the git tag alone — a non-option for rating-compliance, legal, or post-hoc debugging.

**Minimum bundle for each tagged RC (Phase-8 spec):**
1. Tier-D full validator report (JSON)
2. Category-B exemption audit JSON
3. AI-content-disclosure manifest snapshot (pack-manifest block copy)
4. Golden-replay-corpus hashes (`fw replay --export-hashes`)
5. Steam deploy bundle manifest (if EA-candidate)
6. Save-migration test results (JSON)
7. Asset-licensing-tracker CSV snapshot
8. Full `fw verify` output (archived as text)
9. Content-pack pack-manifest for every loaded pack at RC time
10. Determinism-replay-readme — a per-release text file with "to reproduce this build, check out tag X, install Unity 6 LTS version Y, run `./scripts/fw replay` against the corpus hashes in bundle item 4, verify all match"

**EA-build bundle adds:**
- PEGI / ESRB submission PDFs (final form)
- Steam store-page snapshot (descriptor text at launch)
- Launch-day replay capture (one full match, archived as reference trailer footage)

## Determinism-replay posture

Per `design/specs/golden-replay-corpus.md`, MatchSim determinism is verified cross-platform via state hashes. Retention policy's obligation: keep those hashes retrievable per-release forever, so any regression suspected in a post-launch hotfix can be bisected against a known-good reference.

Concretely, archived per RC:
- `content_pack_version` + `canonical_artifact_sha256` for every pack loaded
- Full `key_event_hashes` + `final_canonical_state_hash` for every corpus entry at that RC
- `pass_activation_log_hash` per corpus entry (includes reduce-motion variant path per `design/accessibility.md §Replay / viewer test expectations`)
- Unity Editor version + URP package version pinned at RC
- `MatchSim.csproj` commit SHA (should equal git tag)

Reproducing a historical build's match-replay output, years later: check out the tag, install the pinned Unity + URP versions, run `fw replay <seed>`, compare hashes to the archived bundle. Divergence = either platform drift (investigate CI-matrix-parity) or a bug in the hash comparator itself (unlikely; the hash code is `MatchSim.Contracts`-owned and covered by Tier-A determinism tests).

## Cleanup automation posture

**Phase 3 (manual via `fw artifact-cleanup`):**

CLI commands authored as Phase-3 SPEC task (added alongside this spec):

```
fw artifact-cleanup --list
  List GitHub Actions uploaded artifacts older than their declared
  retention, or any uploaded without a retention-days attribute.

fw artifact-cleanup --delete-expired
  Delete Actions artifacts past their retention window.
  Safe to re-run.

fw artifact-cleanup --release-lock <tag>
  Verify every release-asset for <tag> resolves + checksums match.
  Fails if any asset is missing or drifted.

fw artifact-cleanup --audit-local
  Scan local gitignored artifact folders (`balance-output/`,
  `viewer-captures/`, `playtest-bundles/`, `crash-bundles/`),
  report size + oldest entry per folder. User decides what to
  prune.
```

**Phase 6 onward:** `fw workflow-audit` enforces every `upload-artifact` step in `.github/workflows/*.yml` carries a `retention-days:` attribute matching this spec's Actions-artifact tiers (ephemeral=7, short=14/30). Tier-A CI fails if a workflow uploads an Actions artifact without `retention-days:`. Adds a `FW-WF-A-001` check ID (workflow-upload retention checks, separate ID space from `FW-VAL-*`). It also reports the repository/organization Actions artifact-and-log retention setting as `FW-WF-A-002`, because CI logs are not `upload-artifact` outputs and that shared platform setting must remain at least 30 days for short-tier artifacts. Release-tied bundles are audited only by `fw artifact-cleanup --release-lock <tag>` / the Phase-8 bundle-completeness gate; `retention-days: 0` is not a release-retention mechanism.

**Phase 8 EA:** per-release retention audit as part of RC → EA promotion gate. Bundle-completeness check (all 10 required items present, checksums match). Missing bundle = EA promotion blocked.

## Playtest bug-bundle policy

Per `design/production-pipeline.md §Playtest ops` + `design/content_policy.md §Mod-pack content-safety review posture`, tester-submitted bug bundles are `local-only` at MVP. Expanded here:

- **Bundle contents** (per `SPEC.md Phase 4` task *"In-build 'Export bug bundle' button ships"*): save + last-N replay seeds + rolling logs + settings + content-pack version. No telemetry. No PII by default.
- **Transport:** tester emails the zip OR sends via itch.io message. No in-game cloud upload.
- **Retention:** solo-dev's local `playtest-bundles/` folder, gitignored. Bundles retained indefinitely; periodic manual prune via `fw artifact-cleanup --audit-local` when folder grows past ~1GB.
- **Sharing:** a specific bundle shared only if tester consents AND the bundle is required for a specific bug-fix PR. Never included in release bundles. Never published.
- **Deletion on request:** tester can email to request deletion of their prior submissions; honored within 7 days.

## Migration from Phase-3 to Phase-8 retention posture

| Phase | Retention posture |
|---|---|
| Phase 3 | Manual `fw artifact-cleanup`; workflows carry `retention-days: 7` or `: 30` for uploaded Actions artifacts per this spec; no release-tied artifacts yet (no releases) |
| Phase 4 | Playtest bundles land as `local-only`; tester-bundle folder + prune discipline live |
| Phase 5 | Crash-log bundles land as `local-only`; Phase-5 `docs/ops/crash-logs-telemetry.md` authored per `SPEC.md Phase 5` |
| Phase 6 | `fw workflow-audit` enforces declared uploaded-artifact retention + reports repo/org Actions artifact-and-log retention setting; Tier-C balance-harness summary retention (30d) tested; red-team + synthetic-mod-pack fixtures land as `permanent-in-repo` |
| Phase 7 | Pre-EA retention-policy audit; `fw artifact-cleanup --release-lock` tested against a pre-release dry-run tag |
| Phase 8 | First real RC + EA bundles land as `release-tied`; per-release archival rule enforced; PEGI / ESRB submission PDFs attached; determinism-replay-readme authored |
| Phase 9 | Post-EA hotfix bundles follow EA bundle rule; historical RCs remain retrievable; no rewriting of prior retention posture |

## Growth policy

- **New artifact classes are append-only** to the catalog tables above. When a new pipeline component produces something artifact-shaped, add a row (class / source / storage / TTL / rationale).
- **Retention-tier promotions** (e.g. moving a class from `short` → `release-tied`) require a SPEC decisions-log entry citing the rationale. Demotions (release-tied → short) are blocked for any class that belongs to a released build.
- **Spec size ceiling:** ~60 artifact classes total expected by Phase 9. If the catalog grows past 80, audit for dead pipeline components (classes still listed but pipelines retired).

## Cross-references

- **`design/production-pipeline.md`** — 5-tier model + cost discipline posture + playtest ops
- **ADR-0003 Production pipeline** — artifact retention listed as "owed" in description; this spec closes
- **`design/specs/golden-replay-corpus.md`** — permanent-in-repo retention for corpus fixtures; key_event_hashes archived per RC
- **`design/specs/save-migration-fixtures.md`** — permanent-in-repo retention for save fixtures
- **`design/specs/content-pack-validation-contract.md`** — permanent-in-repo retention for red-team + synthetic-thin-mod-pack fixtures; Category-B audit JSON as release-tied artifact
- **`design/ui-vocabulary.md`** — Category-B exemption audit review cadence (every RC + EA lock)
- **`design/content_policy.md`** — AI-content-disclosure manifest snapshot retention + PEGI/ESRB submission PDF retention
- **`design/accessibility.md`** — reduce-motion paired-fixture retention (permanent-in-repo via corpus)
- **`SETUP.md §7 + §8`** — backup strategy + AI-content disclosure + no-secrets-in-repo
- **`docs/ops/backup-restore.md`** — local-only artifact backup (Time Machine coverage)

## Changelog within this doc

- **2026-04-24** — Authored as Phase-2 spec. Five retention tiers locked (ephemeral / short / release-tied / permanent-in-repo / local-only). Per-class TTLs catalogued across ~30 artifact classes spanning Tier-A through Tier-E + Phase-4/5 playtest-bundle + crash-log surfaces. Cost-discipline math projected through Phase 6 (Free 500MB cap tight; Pro 1GB comfortable). RC / EA-launch bundle minimum-10-items rule locked. Determinism-replay posture pinned: any historical build reproducible from bundle + tag. Phase-3 `fw artifact-cleanup` CLI commands authored as SPEC task owed. Phase-6 `fw workflow-audit` enforces uploaded Actions artifact retention (`FW-WF-A-001`) plus reports repo/org Actions artifact-and-log retention setting (`FW-WF-A-002`); workflow logs are platform-managed, not cataloged as 7-day uploaded artifacts. Release-tied bundles are verified through release assets, not `retention-days: 0`. Playtest bug-bundle policy explicit: local-only, gitignored, tester-consent-on-share, deletion-on-request honored within 7 days.

# Branch protection policy — Final Whistle

Operational policy for `osagberg/FinalWhistle`. Authored 2026-04-24 (Phase 1). Updated on every branch-workflow change.

## ⚠ 0. Reality check — enforcement is currently local-discipline-only

**GitHub Free plan does NOT allow branch protection rules on private repos.** Verified 2026-04-24: `gh api repos/osagberg/FinalWhistle/branches/main/protection` returns 403 with the message *"Upgrade to GitHub Pro or make this repository public."*

Until one of the following is true, the rules in §2 + §3 are **aspirational — not machine-enforced**:

- **GitHub Pro** ($4/mo per user) — unlocks branch protection on private repos
- **Repository made public** — unlocks branch protection on Free, but forces early launch posture we aren't ready for
- **Transfer to a paid org / Team** (including a future Vibelogic publisher org, Phase 8)

**Current posture (Tier-1 buy-on-pain, pre-audience-signal):** local-discipline-only. The rules in this doc are the contract; you enforce them by not pushing directly to `main` or `develop`, opening PRs for your own work, and running `scripts/fw verify-docs` before merging. Tier-A CI is already live and will run on every push / PR — CI-green *can* be a gate even without branch protection, because you choose not to merge red.

**Upgrade trigger:** when a second contributor lands on the project, OR when the first closed-itch playtest (Phase 4) happens and a rollback discipline becomes genuinely expensive-if-broken — whichever comes first. Log the plan flip as a SPEC decisions-log entry.

---

## 1. Branch strategy

Per `CLAUDE.md §5.6`:

| Branch | Role | Protected? | How changes land |
|---|---|---|---|
| `main` | Shippable only. EA / RC / hotfix builds tag from here. | Yes | PR from `develop` only (except emergency hotfixes — see §4) |
| `develop` | Integration. All feature work merges here first. | Yes | PR from `feat/*` / `fix/*` branches |
| `feat/<name>` | Per-feature work. | No | Free-fire local; push when ready |
| `fix/<name>` | Per-fix work. | No | Free-fire local; push when ready |

**Solo-dev caveat:** self-PRs are fine and expected. The protection is against:
- Accidental direct-push to `main` or `develop`
- Merging without CI green
- Merging without at least one deliberate review pass (self or otherwise)

---

## 2. `main` protection rules

Configure via `github.com/osagberg/FinalWhistle/settings/branches` **once on a paid plan** — see §0.

- [ ] **Require pull request before merging** — yes
- [ ] **Require approvals** — **0** (not 1). GitHub does NOT allow PR authors to approve their own PRs, so `required_approving_review_count: 1` would block solo-dev merges forever without a second collaborator. Use `0` and rely on status checks + conversation resolution + self-review discipline (see §6) as the real gates.
- [ ] **Dismiss stale pull request approvals when new commits are pushed** — yes (takes effect only if a second contributor ever reviews)
- [ ] **Require status checks to pass before merging** — yes
- [ ] **Require branches to be up to date before merging** — yes
- [ ] **Status checks required** (once they exist):
  - `Fast PR CI (Tier A) / Verify docs`
  - Future Phase-3: `MatchSim.Tests`, `Deterministic replay hash smoke`
  - Future Phase-6: `Content-pack schema`, `Save migration fixture smoke`
- [ ] **Require conversation resolution before merging** — yes
- [ ] **Require signed commits** — optional; enable if/when a code-signing cert is purchased (Phase 8 optional)
- [ ] **Require linear history** — yes (forces rebase or squash; avoids merge-commit noise)
- [ ] **Do not allow bypassing the above settings** — yes (applies to admins too)
- [ ] **Restrict who can push to matching branches** — yes; only you + service accounts
- [ ] **Allow force pushes** — **no** (never)
- [ ] **Allow deletions** — **no** (never)

---

## 3. `develop` protection rules

Slightly looser than `main` to keep iteration cheap while still gating on CI:

- [ ] **Require pull request before merging** — yes
- [ ] **Require approvals** — 0 (same constraint as `main`; approvals=0 is correct, not a looseness)
- [ ] **Require status checks to pass before merging** — yes
  - `Fast PR CI (Tier A) / Verify docs`
  - (same status checks as `main` once they exist)
- [ ] **Require branches to be up to date before merging** — yes
- [ ] **Require linear history** — yes
- [ ] **Do not allow bypassing** — yes
- [ ] **Allow force pushes** — **no**
- [ ] **Allow deletions** — **no**

---

## 4. Emergency hotfix path

Post-EA only. Before EA, there's no live audience and hotfixes are not an emergency.

For a true post-EA hotfix:

1. Branch from `main`: `git checkout -b fix/hotfix-<description>`
2. Apply minimum viable fix.
3. Run local `scripts/fw verify-docs` + whatever tests exist for the touched system.
4. Push branch; open PR to `main`.
5. PR requires:
   - Fast PR CI green.
   - Release-candidate Tier D abbreviated run (per `design/production-pipeline.md` §channel hotfix).
   - Manual QA sign-off (you; explicit note in PR body).
6. Tag release on merge; Tier-E Steam deploy workflow fires with manual approval gate.
7. Backport fix to `develop` (cherry-pick or merge `main` → `develop`).

---

## 5. Pre-merge checklist (for every PR)

Matches `.github/PULL_REQUEST_TEMPLATE.md`. Key items:

- [ ] Summary + Why filled in
- [ ] Linked SPEC task / ADR / decisions-log entry cited
- [ ] Test plan executed locally; results in PR body
- [ ] Banned-term lint green (once Phase-1 lint lands)
- [ ] Content-pack validator green (once content exists)
- [ ] No `{{ PROJECT_NAME }}` / `TODO:` / `FIXME` leaks in shipped content
- [ ] Decisions log left append-only (no mutations)
- [ ] CHANGELOG line drafted

---

## 6. Solo-dev review discipline

You ARE the reviewer. Treat self-review as a discipline, not a formality:

- Wait at least 15 minutes between "I finished coding" and "I open the PR". Fresh eyes catch more.
- Read the PR diff **in the GitHub UI** on a different screen than where you wrote the code. Browser rendering beats IDE rendering for catching copy-paste errors.
- Verbalize the "why" out loud once before clicking Approve. If it doesn't sound right, it probably isn't.
- If you find yourself hunting for the Approve button to get unblocked fast, that's a signal to step away and come back.
- On pillar-level / architectural work (new SPEC decision, new ADR, new schema surface): always bounce to GPT-5.5 before merge, not after.

---

## 7. Configuration drift check

The branch-protection rules above are the **source of truth**. Quarterly, compare them against the live GitHub config via:

```bash
gh api repos/osagberg/FinalWhistle/branches/main/protection
gh api repos/osagberg/FinalWhistle/branches/develop/protection
```

Drift between this doc and the live config is a finding — update whichever is correct.

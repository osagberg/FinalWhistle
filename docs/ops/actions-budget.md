# GitHub Actions budget — Final Whistle

Runbook for keeping CI cost at or near zero. Authored 2026-04-24 (Phase 1).

---

## 1. The policy (source: `design/production-pipeline.md`)

- **Hard budget cap on GitHub Actions usage.** Usage stops at included-minutes; overage is off by default.
- **Tier A (fast PR CI) must fit inside the included-minutes budget** for typical solo-dev PR volume.
- **macOS-hosted runner minutes reserved for Tier D (Release Candidate)** — infrequent.
- **Heavy work (Tier C) runs local / self-hosted, not GitHub-hosted.**

Cost facts (verify before acting — pricing changes):
- Free plan private repos: 2,000 Actions minutes / month included.
- Pro plan private repos: 3,000 / month.
- Linux 2-core: ~$0.006/min. Windows: ~$0.010/min. macOS: ~$0.062/min (~10× Linux).

---

## 2. One-time setup

### 2a. Confirm plan + included minutes

1. Visit `github.com/settings/billing`.
2. Under "Plans and usage" note current plan (Free or Pro).
3. Note included Actions minutes for the month.

### 2b. Set hard spending cap to $0

1. Visit `github.com/settings/billing/spending_limit`.
2. Under "Actions & Packages spending limit":
   - Click **"Manage spending limit"** or "Edit".
   - Set to **$0** ("Unlimited" must NOT be selected).
3. Save.

Effect: when the month's included minutes are exhausted, Actions runs **fail with "billing limit reached"** instead of charging your payment method. This is the correct behavior at MVP — forces an explicit decision to raise the cap rather than silent overage.

### 2c. Scope email / dashboard alerts

Under `github.com/settings/notifications` → Actions:
- Enable "Send notifications for failed workflows".
- Disable noisy categories (pushes / successes) — we only care about failures + budget warnings.

---

## 3. Ongoing hygiene

### 3a. Weekly (during active PR volume)

- Check `github.com/osagberg/FinalWhistle/actions` for:
  - Runs that took >80% of the 5-minute Tier-A budget (they should all be well under)
  - Repeated failures on the same PR (indicates lint-rule or test-infra issue, not real breakage)
- If usage trending toward 80% of included minutes before month-end, temporarily disable non-essential workflow triggers (e.g., push-to-branch triggers while only merging via PR).

### 3b. Monthly

- Visit `github.com/settings/billing/summary` → Actions usage.
- Confirm actual usage matches expectation.
- If overage was blocked by the $0 cap: investigate why the budget was hit (runaway workflow, stuck job, new workflow added without budget review) before raising the cap.

### 3c. Before enabling a new workflow

Every new `.github/workflows/*.yml` addition answers in the PR body:

1. **Trigger frequency:** per PR / per push / manual / scheduled? If scheduled, which runner tier?
2. **Typical run duration:** estimated minutes per run on the chosen runner.
3. **Runner cost:** Linux / Windows / macOS / self-hosted?
4. **Expected monthly minutes:** `frequency × duration` per the run.
5. **Budget impact:** new total as % of included minutes.

Reject any new workflow whose budget impact can't be stated.

---

## 4. When to raise the cap

Only raise the spending limit above $0 for a **specific, time-bounded reason**. Examples:

- Release-candidate Tier D run needs macOS-hosted build that month → raise to $25 for the RC week, lower to $0 after.
- Unity CI matrix during a pre-EA polish sprint → raise with explicit budget ceiling.

**Never** leave the cap raised "just in case." Silent overage is how solo-dev CI bills become "wait, I spent $200 on what?"

Document cap changes in a SPEC decisions-log entry if the change lasts >1 week.

---

## 5. Self-hosted runner option (Phase 3+)

If GitHub-hosted minutes prove insufficient (unlikely through Phase 2), the first escalation is a **self-hosted runner on your local Mac**:

- Currently free for Actions usage per GitHub docs (verify when enabling).
- Runner agent installs via `github.com/osagberg/FinalWhistle/settings/actions/runners`.
- Restrict to labeled workflows only — never let arbitrary PR-triggered workflows run on your local machine (security risk — untrusted PRs could run arbitrary code).
- Self-hosted runner is **NOT** the default answer to "CI is slow" — the default answer is "move the work to Tier C (local-only)."

Escalating to self-hosted is a Phase-3+ decision. Through Phase 2, GitHub-hosted Linux runners at ~5 minutes per PR should stay well within the 2k/3k included minutes.

---

## 6. Kill switches

If Actions usage spikes unexpectedly:

1. **Immediate:** disable the offending workflow via `github.com/osagberg/FinalWhistle/settings/actions` → "Disable Actions". Re-enable after triage.
2. **Per-workflow:** add `if: false` to the job, push to main, investigate.
3. **Nuclear:** the $0 spending cap means worst-case you can't run Actions for the remainder of the month. Your code still works locally; `scripts/fw verify-docs` is the Tier-A equivalent.

---

## 7. References

- `design/production-pipeline.md` §cost-posture, §workflow-tiers.
- `docs/ops/branch-protection.md` — status-check requirements.
- GitHub docs:
  - Included usage: https://docs.github.com/en/billing/reference/product-usage-included
  - Actions billing: https://docs.github.com/en/billing/concepts/product-billing/github-actions
  - Self-hosted: https://docs.github.com/en/actions/reference/runners/self-hosted-runners

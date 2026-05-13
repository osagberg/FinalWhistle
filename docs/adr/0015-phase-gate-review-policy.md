# ADR-0015 — Phase-gate review policy

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** Claude (Codex full-project audit Lane B "missing ADR" driver) + Codex (pending T1-phase-gate audit when T1 closes)

---

## Context

`CLAUDE.md §6` ("Phase-gate review") established the v2 review cadence: "Codex reviews at phase boundaries only, not per task. Per-task self-review (§5) is the inner loop; Codex is the outer loop." This replaced FW v1's `scripts/agent-bus` per-slice protocol — which was retired in the v2 pivot (REFERENCES.md "What was dropped permanently").

The 2026-05-13 Codex full-project audit Lane E finding ("phase-only Codex review missed T1-1, which was a canonical schema change") + Lane B finding ("missing ADR for phase-gate review policy post-agent-bus") both point at the same gap. The phase-gate-only rule may be too coarse for load-bearing task-level work (like T1-1 schema lock).

This ADR formalises the v2 review cadence and adds an explicit **mid-phase targeted-audit** trigger for high-stakes task work that would otherwise sneak through self-review.

## Decision

### Three review tiers

The v2 workflow has THREE review tiers, in increasing cost:

**Tier 1 — Per-task self-review (mandatory on ≥100 LoC of code change).** Per `CLAUDE.md §5` + `/next` Step 6. Runs three subagents automatically: `pr-review-toolkit:silent-failure-hunter`, `pr-review-toolkit:type-design-analyzer`, `feature-dev:code-reviewer`. Fires on every commit that meets the threshold; findings fix in-place or defer to commit-body follow-ups. Cost: ~$0.50–2 per commit.

**Tier 2 — Mid-phase targeted Codex audit.** Invoked manually by the user (via `gh pr create` against a topic branch + a focused Codex prompt) when the work is load-bearing. Examples that qualify:
- Schema lock for the rest of the project (T1-1's `fw-content` schema)
- New crate boundary (T2-9 `fw-save` first ship)
- Save-format change (any T3+ schema bump)
- BT-runner first implementation (T1-2b)
- Signature catalogue freeze (T1-3 first 3 signatures)

The prompt is task-specific, not a full-project audit. Cost: ~30 min of the user's time + 1 Codex session.

**Tier 3 — Phase-boundary full audit.** Invoked at every `/done` close. The user runs `gh pr create` against the phase's accumulated commits; Codex audits the phase as a whole — architecture coherence, test coverage, doc consistency, scope-realism. Cost: ~1–2 hr of user time + 1 Codex session.

### When does Tier 2 fire?

The criteria for "load-bearing enough to warrant a Tier-2 audit":

1. **Schema lock** — a struct, enum, or wire format that downstream crates will read for the rest of the phase. T1-1 was the canonical example.
2. **New canonical-state surface** — any commit that adds a field to `MatchState`, `MatchEvent`, or `MemoryEvent`. Tier 2 is mandatory.
3. **First implementation of an ADR** — when an ADR moves Proposed → Accepted via implementation, the first implementation commit gets a Tier-2 audit.
4. **API surface to UI** — any IPC command added to `fw-tauri`. The contract crosses the canonical-state ↔ UI boundary; Tier-2 mandatory.
5. **Save migration adapter** — any `migrations/<N>_to_<N+1>.rs` commit. The four-test contract is structural; Tier-2 confirms the test contract holds.

The criteria are **explicit and listed**; the goal is to take judgment out of the loop. If a commit hits any of the five, Tier-2 fires.

### Tier 2 prompt template

Per the 2026-05-13 audit (which Codex executed against the T1-1 schema lock as a Tier-2 prompt), the template:

```
TARGETED CODEX AUDIT — <task ID + name>
Engagement: focused, read-only, multi-agent.

Repo: /Users/vibelogic/dev/football
Commits in scope: <SHA-range or list>
Spec the work implements: <ADR path>

Audit lanes (focused, NOT the 10-lane full-project shape):
- Lane A: the diff itself (P0/P1 bugs, encapsulation, error-handling)
- Lane B: ADR-conformance (does the code match the ADR?)
- Lane C: determinism floor (did anything leak past the contract?)
- Lane D: carry-forward debt (did we lose anything from v1's equivalent?)

Output: structured Markdown report with P0/P1 findings + recommended fixes.
Quality bar: brutal but specific. file:line anchors required.
DO NOT mutate code; read-only audit. Recommendations only.
```

The template lives at `docs/templates/tier-2-codex-prompt.md` (Tranche 4 deliverable).

### Tier 3 prompt template

Lives at `docs/templates/tier-3-codex-prompt.md` (Tranche 4 deliverable). The 2026-05-13 full-project audit prompt is the canonical reference shape.

### When does Tier 3 fire?

`/done` invokes Tier 3 at every phase close. NO exceptions. The phase-gate is the audit boundary.

### Solo-dev period vs post-EA

- **Solo-dev pre-EA period (now → T5):** Tier-1 + Tier-2 + Tier-3 as described. User authorises Tier-2 invocations manually based on the 5 criteria.
- **Post-EA period (T5+):** if Codex is still active, same protocol. If Codex is unavailable, Tier 2/3 collapse to "peer review by an experienced game-dev" or "LLM-as-a-service audit via Anthropic API." The cadence stays; the reviewer identity is the variable.

### Forbidden patterns

- Skipping Tier 1 because "the change is small" — the 100-LoC threshold is the auto-gate; the user does NOT override it case-by-case.
- Skipping Tier 2 on a commit that hits any of the 5 criteria — the criteria are listed exactly to remove judgment from the call.
- Skipping Tier 3 at `/done` — phase boundaries are the audit boundary; bypassing them is the failure mode that v1's agent-bus protocol prevented.

## Consequences

**Positive:**
- The audit cadence is structural, not judgment-call. Five criteria for Tier 2; one trigger (`/done`) for Tier 3.
- T1-1's gap (load-bearing schema work that Tier 1 alone wasn't strong enough for) is closed — future T1-1-like work fires Tier 2.
- The cost shape is predictable. Tier 1 runs constantly; Tier 2 fires ~once per phase (avg 1–3 invocations across T1–T5); Tier 3 fires 6 times (one per phase).
- Solo-dev workflow stays light — the user invokes Tier 2/3 manually, not via automated triggers.

**Negative:**
- Tier 2's manual invocation depends on the user remembering to fire it. The 5 criteria help; a pre-commit check that surfaces "this commit hits criterion N — Tier 2 recommended" would help more (Tranche 7 follow-up).
- Codex CLI session cost adds up. Realistic estimate: $5–20 per Tier-2 audit; $20–50 per Tier-3 audit. Phase-total ~$50–100 in Codex spend across the lifecycle.
- The 5 criteria might be too coarse or too narrow. Tunable; the ADR can be amended.

**Neutral:**
- This ADR is process-level. No code change required.

**Rollback path:**
- If Tier 2 produces low-value findings consistently, raise the criteria threshold (e.g. require BOTH a schema lock AND a new canonical-state surface, not either-or).
- If Tier 2 catches genuinely-missed P0/P1s repeatedly, tighten the criteria (add more triggers).

## Alternatives considered

- **Stay at "phase boundaries only" (the original CLAUDE.md §6 rule).** Rejected — the T1-1 audit found a P0 + multiple P1s that Tier 1 missed; phase-boundary-only would have shipped them.
- **Reintroduce v1's per-slice agent-bus protocol.** Rejected — the per-task overhead was ~$15 vs $4 with the v2 model. Tier 2 picks up the same value at a fraction of the cost because it fires selectively, not on every task.
- **Tier 2 fires on every commit ≥500 LoC.** Rejected — LoC is the wrong metric. T1-1 was ~1000 LoC but the *risk profile* was the schema lock, not the line count. A 200-LoC migration adapter is higher-risk than a 1500-LoC test corpus addition.
- **Auto-detect Tier-2 triggers in a hook.** Considered; partial implementation feasible via `.claude/hooks/tier-2-trigger-detector.sh` (pre-commit). Tracked as Tranche 7 follow-up — until landed, the user invokes Tier 2 manually.

## References

- `CLAUDE.md` §5 (Tier-1 self-review mandate)
- `CLAUDE.md` §6 (the original phase-gate-only rule this ADR amends)
- `.claude/skills/next/SKILL.md` Step 6 (Tier-1 implementation)
- `.claude/skills/done/SKILL.md` (Tier-3 trigger via `/done`)
- `docs/audits/codex-full-audit-2026-05-13.md` (the audit that surfaced this ADR's need)
- `docs/templates/tier-2-codex-prompt.md` (Tranche 4 deliverable)
- `docs/templates/tier-3-codex-prompt.md` (Tranche 4 deliverable)
- v1: `scripts/agent-bus` (the retired protocol this ADR's Tier 2 effectively replaces)
- Codex full-project audit Lane B "missing ADRs" + Lane E "phase-only review missed T1-1"

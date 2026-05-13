---
description: Close the current phase — verify gate, sync ledgers, open PR for Codex review.
---

# /done — phase completion

Run when the current phase's acceptance gate is believed to pass. NOT a per-task command — that's `/next`.

## Pipeline

### 1. Verify phase scope is complete

Read `docs/MASTER_PLAN.md`. Find the active phase. Confirm every task row is DONE (no TODO, no BLOCKED). If any are open, stop and list them.

### 2. Re-run full verify

```bash
scripts/fw verify
```

Must be green (fmt + clippy + cargo test --workspace + pnpm test + banned-terms + canonical-hash regression). Any failure = stop, report, do not proceed.

### 3. Verify the phase's acceptance gate

Each phase in MASTER_PLAN has an explicit acceptance gate. Read it. Execute every check it specifies. Examples:

- **T0 (foundation):** pinned canonical SHA matches on macOS + Windows + Linux CI; Tauri shell opens.
- **T1 (sim core):** 90-min match completes; replay round-trip byte-identical; proptest invariants hold over 10k matches.
- **Content phase:** `scripts/fw verify-content` green on seeded corpus; FW-VAL checks pass.
- **UI phase:** screenshot of tactical board + commentary feed attached; `pnpm test` + `pnpm lint` green.
- **Save schema bump:** all four migration tests present + green.

If any acceptance check fails, stop and report which.

### 4. Append CHANGELOG entry

Append a phase-summary block to `CHANGELOG.md`:

```
## Phase <N>: <name> — YYYY-MM-DD

- <bullet per shipped MASTER_PLAN item, grouped by crate/area>
- Canonical-state hash: <BLAKE3 short>
- Tests: <n new>, all green on [macos-14, windows-latest, ubuntu-22.04]
- Decisions logged: <count> (see docs/DECISIONS.md)
```

### 5. Rewrite STATUS.md

STATUS.md is a state pointer, NOT a diary. Replace body with:
- Current phase pointer → **next** phase
- Active task: `(none — awaiting Codex review of phase <N>)`
- Blockers: `(none)` or list
- Last green verify: timestamp
- Last canonical hash: BLAKE3 short

Stop hook auto-stamps the file timestamp.

### 6. Open PR for Codex phase-gate review

Print (DO NOT EXECUTE) the suggested commands for the user:

```bash
git push origin main
gh pr create \
  --title "Phase <N>: <name>" \
  --body "$(cat <<'EOF'
## Phase <N> gate review

**Scope:** <one line>
**Acceptance:** see docs/MASTER_PLAN.md Phase <N> gate — all checks green.
**Verify:** scripts/fw verify green on macOS dev + CI matrix.
**Canonical hash:** <BLAKE3 short>
**Stats:** <LoC added/removed>, <N commits>, <N tests>.

### Decisions logged this phase
- <bullet per DECISIONS.md entry added since last phase>

### Risks for next phase
- <bullet>

Handing to Codex for phase-gate review.
EOF
)"
```

The user runs these in a separate terminal. Codex reviews via filesystem against the PR URL.

### 7. Print phase-summary report (<300 words)

- Phase name + duration (commit timestamps first → last)
- LoC delta (crates touched)
- Commits count + most-impactful 3 by message
- Decisions added (count + topics)
- Risks carried into next phase (2-3 bullets, sourced from MEMORY.md + open BLOCKED rows)
- Suggested first task for next phase (from MASTER_PLAN)

Then stop. Next steps belong to Codex review + user merge decision.

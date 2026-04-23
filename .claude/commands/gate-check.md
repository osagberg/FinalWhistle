---
description: Validate phase-gate conditions — PASS / CONCERNS / FAIL with blockers listed
argument-hint: "[target-phase]"
---

# /gate-check — phase-gate validation

Validates whether the project is ready to advance to the next phase. Checks for required artifacts, quality standards, and blockers. Distinct from `/status` (diagnostic) — this is prescriptive with a formal verdict.

**Phase:** any. Run at phase transitions. Also triggered implicitly by `/done` when closing the last task of a phase.

## Procedure

1. **Parse target phase.**
   - `<phase-number>` or `<phase-name>` — validate that specific gate
   - No arg — auto-detect current phase from `SPEC.md`; validate the next transition; confirm with user via `AskUserQuestion`
2. **Load** `SPEC.md` (phase list + gate conditions) + `STATUS.md`.
3. **Run gate checks for the target phase.** Gate definitions live in `SPEC.md` under each phase heading. Generic gates to always check:
   - All tasks in prior phase marked `[x]`
   - `CHANGELOG.md` entries exist for each `[x]` (drift check)
   - No template placeholder or TODO markers in any doc touched by this phase
   - Decisions-log entries for major phase decisions
4. **Phase-specific gates** (examples — adapt to SPEC.md):
   - **Concept → Systems Design:** `design/game-concept.md` exists + reviewed; pillars ranked
   - **Systems Design → Technical Setup:** `design/systems-index.md` exists; all MVP GDDs reviewed (`/review-all-gdds` PASS)
   - **Technical Setup → Pre-Production:** `docs/architecture/architecture.md` exists; all Required ADRs authored; `/architecture-review` PASS
   - **Pre-Production → Production:** All Foundation + Core epics + stories exist; `/story-readiness sprint` returns all READY
   - **Production → Polish:** Feature-layer MVP stories all Complete; `/smoke-check` PASS; `/balance-check` BALANCED or ADJUSTMENTS RECOMMENDED
   - **Polish → Release:** `/regression-suite audit` PASS; zero S1/S2 bugs open; `/milestone-review` go
5. **Spawn Director subagents** based on review mode:
   - `lean` (default) — spawn relevant director for this gate (e.g., Technical Director for Pre-Prod → Production)
   - `full` — spawn Creative + Technical + Producer + Art directors
   - `solo` — skip director spawns, artifact-existence checks only
6. **Verdict:**
   - **PASS** — all gates green; phase can advance
   - **CONCERNS** — non-blocking findings; user may override
   - **FAIL** — blockers present; advancement not recommended
7. **If PASS**, offer to execute transition (flip prior phase `✅ COMPLETE`, promote next phase to `🟡 ACTIVE`, flesh out placeholder tasks).
8. **Write report** to `reviews/gate-check-<phase>-<date>.md`

## If args provided

- `<phase-number>` or `<phase-name>` — scoped gate

## If no active phase in SPEC

Fail: "No active phase in `SPEC.md`. Project state undefined — run `/status` or review SPEC."

## Output

- `reviews/gate-check-<phase>-<date>.md`
- Console: verdict + blockers + proposed next action

## Related

- Typical follow-ups (PASS): `/done` to transition; `/next` to start new phase
- Typical follow-ups (FAIL): remediate blockers, re-run
- Invokes agents: director subagents per review mode (`creative-director`, `technical-director`, `producer`, `art-director`)
- Invokes skills: may cross-check with `/audit`, `/refresh-docs`
- Reads files: `SPEC.md`, `STATUS.md`, `CHANGELOG.md`, phase-specific artifacts
- Writes files: `reviews/gate-check-<phase>-<date>.md`, optionally SPEC.md (phase transition)

## Phase

T<id>: <phase name>

<!-- Examples:
T0-7: GitHub Actions matrix CI
T1-2: fw-match-sim ball physics + 22-player BT runner
T2-9: fw-save bincode-based save format + version migration
-->

## Summary

<!-- 1-3 sentences. What this PR ships. The reader should be able to
parse this without opening the file diff. -->

## Acceptance criteria

<!-- Pulled verbatim from docs/MASTER_PLAN.md "Done Criteria" column.
Every criterion must be checkable. If any criterion turns out to be
unverifiable mid-implementation, raise it in the PR description and
update MASTER_PLAN.md alongside the merge. -->

- [ ] criterion 1
- [ ] criterion 2

## Tests

<!-- Be specific. "tests pass" is not enough; name the test count + which
canonical-hash status the gate landed in. -->

- `cargo test --workspace --release`: __/__  passing
- `cargo clippy --workspace --all-targets -- -D warnings`: clean / N warnings
- `cargo fmt --all -- --check`: clean
- `pnpm test` + `pnpm typecheck` + `pnpm lint`: clean
- Canonical hash gate: __ all 3 OSes  /  __ drift authorized (rebaseline reason: ...)

<!-- If the canonical hash drifted intentionally, link the DECISIONS.md
entry that authorized the rebaseline:

  authorized by: docs/DECISIONS.md#YYYY-MM-DD-...
-->

## Self-review (CLAUDE.md §5 mandatory triple for ≥100 LoC of code)

<!-- The three subagents below MUST run before commit on any change
≥100 LoC of code. Skipping requires a one-liner in the commit body
explaining why. -->

- silent-failure-hunter: __ clean / __ findings: <summary or N/A>
- type-design-analyzer:  __ clean / __ findings: <summary or N/A>
- feature-dev:code-reviewer: __ clean / __ findings: <summary or N/A>

## Codex review (phase-gate only — see CLAUDE.md §6)

<!-- Phase-gate PRs only. For mid-phase per-task commits, leave as N/A. -->

- Requested at: <timestamp>
- Result: __ ack  /  __ counter  /  __ pending

## Pitfalls / next-up

<!-- Known follow-ups, deferred items, breadcrumbs for future sessions.
This is where future-Claude / future-me find out why a thing was
half-finished and what was intentional vs. accidental. -->

- 

---

<!-- Mechanical checklist (delete from final PR body once verified):

- [ ] STATUS.md updated (auto-stamped by Stop hook on /done)
- [ ] CHANGELOG.md line appended
- [ ] MASTER_PLAN.md status flipped to DONE with commit SHA + evidence
- [ ] No HashMap / HashSet in canonical-state crates (clippy-enforced)
- [ ] No f32/f64 arithmetic in fw-match-sim/fw-memory/fw-replay/fw-save/fw-content
- [ ] No SystemTime::now() / Instant::now() / thread_rng() in sim/content/memory
- [ ] Banned-terms lint green (scripts/fw banned-terms)
- [ ] If save schema bumped: four-tests-per-bump fixtures committed
-->

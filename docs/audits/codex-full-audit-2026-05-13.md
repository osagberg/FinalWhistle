# Codex Full-Project Audit — 2026-05-13

**Engagement:** read-only multi-agent audit after T1-1 schema-lock shipped (commits `69f900b9` + `821d387`).
**Verdict:** NEEDS FIXES before T1-2b. T1-2a can proceed only after status/doc drift is cleaned up.
**Findings:** 1 P0 + 11 headline P1 + ~30 lower-severity in lane reports.

## Triage (Claude's assessment, 2026-05-13)

### Tranche 1 — fix immediately (P0 + cheap doc drift)
1. **P0 — pinned-hash test can be `#[ignore]`-disabled.** CI grep + meta-test + hook coverage. ~30 min.
2. **P1 — STATUS / MEMORY / CHANGELOG / MASTER_PLAN drift** (claims "commit pending" / "browser-dev wiring confirmed" / Phase T0 / etc. when reality is different). Single doc-cleanup pass. ~45 min.
3. **P1 — `.claude/launch.json` untracked while docs claim it's wired.** Commit it or remove claim. ~5 min.
4. **P1 — local `main` ahead of `origin/main` by 2 commits.** Push to confirm CI on HEAD. ~5 min.

### Tranche 2 — schema / encapsulation fixes (T1-1 follow-ups, code-side)
5. **P1 — `AbilityCeiling::new` is unchecked.** Add `try_new` + `validate_unit_range`. Tests reject negative / >1 / current > potential.
6. **P1 — CA weights accept hidden/durability fields.** Split `VISIBLE_ATTRIBUTE_NAMES` from `KNOWN_ATTRIBUTE_NAMES`; validate `RoleWeights` against visible-only per ADR-0002 §"Choices" item 6.
7. **P2 — `RoleId::new` accepts empty/whitespace in release.** Promote `debug_assert` to runtime validation or document the policy.
8. **P2 — `schema_version` is marker-only.** Either implement load-gate or document that it's a placeholder until T2-3.
9. **P2 — `Q32Inner` is re-exported.** Hide it (bypasses panic-on-overflow operator policy).

### Tranche 3 — ADR drift + missing ADRs (design work, no code)
10. **P1 — RNG seed contract inconsistent across 4 docs.** New ADR-0009: one canonical `seed_fn(match_seed, tick, layer_tag, decision_id)` API. All random sites call it.
11. **P1 — Personality vector drift: ADR-0001/0003 say 8-element, ADR-0002 says 14.** Reconcile in ADR-0001 + ADR-0003 amendments.
12. **P1 — 8Hz cadence math: 60Hz / 8Hz = 7.5 ticks.** Pick: 4Hz / 7.5Hz with 8-tick window / explicit accumulator. Amend ADR-0001.
13. **P2 — Memory event count says both 28 and 29.** Reconcile ADR-0005.
14. **Missing ADRs (promote folklore → ADR):** save format (bincode 2 + zstd + migration), signature system, hash rebaseline policy, licensed-data policy, runtime AI/content boundary, phase-gate review policy (post-agent-bus retirement).

### Tranche 4 — missing T1-2b companion specs (must precede T1-2b code)
15. **P1 — Missing:** `docs/specs/tactic-fsm.md`, `docs/specs/bt-attribute-binding.md`, `docs/specs/decision-cadence-stagger.md`, `docs/design/xg-coefficients.md`, `docs/design/personality-bias-weights.md`. Without `bt-attribute-binding.md` the 55-attribute model has no consumption contract.

### Tranche 5 — MASTER_PLAN restructuring (producer)
16. **P1 — T1-2b is still too broad.** Split into smaller rows (ball physics / FSM-of-BTs / steering / events / hash pinning).
17. **P1 — PlayerSeparation acceptance.** Add concrete invariants to T1-2b/T1-9: min distance, deterministic pair order, velocity preservation, ball non-mutation, zero-distance fallback, runner-order regression.
18. **P2 — T2-1 "all 20-30 archetypes" is secretly huge.** Split.
19. **P2 — `cargo audit` + `cargo deny` deferred but trigger has fired.** Schedule.

### Tranche 6 — runtime + validation (must precede T2 content baking)
20. **P1 — `ContentStore::load_baked` returns `Ok(Self::default())`.** Implement minimal loader OR rename stub. Block runtime use until then.
21. **P1 — FW-VAL is fail-open.** Real validator before T2-3 baking. `||echo` masking in Justfile is hiding failure.

### Tranche 7 — workflow + tooling
22. **P1 — Hooks are Claude-only, not git-scoped.** Either install git hooks too or document as convenience-only.
23. **P2 — `/next` still references retired `/phase-gate` and `/duo-debate`.** Clean up SKILL.md.
24. **P2 — Unity MCP still in global `claude mcp list`.** Remove (Rust project now).
25. **P2 — Frontend `@apply` in shared CSS violates Frontend/RULES.md §2.** Either fix CSS or amend rule.
26. **P2 — Culture fixtures violate Content/RULES.md (no `schema_version`, wrong ID shape).** Fix fixtures or amend rule.

### Tranche 8 — pillar gaps (longer-term tracking)
27. **Pillar 4 (scouting) is vapor.** `fw-scouting` still has stale `gene_index`. Schedule pre-T3-5 spec.
28. **Pillar 5 (signature identity) is vapor.** No v2 catalogue. Port v1's 24-signature design into a v2 source-of-truth doc before T1-3.

### Tranche 9 — research waves (forward-looking)
29. **Pre-T1-2b sim audit:** verify cadence + seed API + attribute binding + separation before code.
30. **Deterministic ball physics in Rust deep dive:** Q32 semi-implicit Euler, drag, bounce, friction, collision order, golden vectors.
31. **Licensed-data validator research:** name overlap, club blocklists, false positives, mod policy.
32. **Q32 authoring format:** decimal-string RON wrapper vs raw `(bits: N)`.
33. **Stat-distribution calibration:** OOTP-style aggregate gates, Dixon-Coles reference sim.
34. **Signature catalogue port:** v1's 24-signature design → v2 source-of-truth doc.

---

## Original Codex report (verbatim)

[See conversation transcript 2026-05-13 — paste below if needed for offline reference.]

## Sequence Claude proposes (pending user approval)

**Today:**
- Tranche 1 (P0 + doc drift + push + commit launch.json). ~90 min total.

**Next session(s) before T1-2a starts:**
- Tranche 2 (T1-1 schema follow-ups). One `/next`-style cycle, single commit.
- Tranche 3 (ADR-0009 RNG seed, ADR amendments for personality vector + cadence + memory event count, promote folklore-to-ADR for save / signature / hash-rebaseline / licensed-data / runtime-LLM / phase-gate). Sequential `/log-decision` cycles.
- Tranche 4 (companion specs for T1-2b).

**Before T1-2b code:**
- Tranche 5 (MASTER_PLAN restructure: split T1-2b + add PlayerSeparation acceptance).
- Tranche 6 (real ContentStore loader + real FW-VAL).
- Tranche 7 (workflow + tooling cleanup).

**Long-term tracking:**
- Tranches 8 + 9 (pillar gaps + research waves) added to MASTER_PLAN at appropriate rows.

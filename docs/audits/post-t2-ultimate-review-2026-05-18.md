# Post-T2 Ultimate Review — 2026-05-18

> Multi-track adversarial review at the Phase T2 → T3 boundary. Per the /done
> skill Step 5.5: dispatched when a phase ships ≥5 commits. T2 shipped 10.
>
> **Phase scope reviewed:** T2 commits `b2d41f9b..26540dcd` (10 commits;
> +9844 / -276 LoC across 77 files; 90+ new tests). All 10 T2 MVP rows
> landed; 3 rolled to T3 (T2-4 PlayerBio + T2-7 Squad blocked on missing
> design doc; T2-1d2 utility_shoot deferred per "wait for BT to mature").
>
> **Track split rationale** (per `/done` Step 5.5): Claude is strong on
> implementation-drift / test-quality / systemic-pattern detection; Codex is
> strong on adversarial red-team + property explosion. Combined ≈ 2× the
> findings of either alone (per the 2026-05-16 audit convergence patterns).
>
> **Subagent discipline**: every Claude track is read-only. No file edits
> outside this shared review file. No git commits. Findings land here +
> the main thread consolidates into a verdict section at the end.

---

## Track A — Mutation-test analysis (Claude `feature-dev:code-explorer`)

*Lens: "If I mutated the constants / flipped the team / removed the guard / always returned default, would a test fail?" Identify MUTATION SURVIVORS across the T2 diff.*

**Owner:** Claude `feature-dev:code-explorer`
**Status:** _(pending — track populated by subagent on dispatch)_

### Findings

_To be filled by Track A subagent._

---

## Track B — Architectural drift (Claude `pr-review-toolkit:code-reviewer`)

*Lens: docs vs code, both directions. Where does code drift from the design doc that authored it? Where does a design doc lie about what the code actually does? Where does a comment claim an invariant the code doesn't enforce?*

**Owner:** Claude `pr-review-toolkit:code-reviewer`
**Status:** _(pending — track populated by subagent on dispatch)_

### Findings

_To be filled by Track B subagent._

---

## Track C — Whole-codebase silent-failure sweep (Claude `pr-review-toolkit:silent-failure-hunter`)

*Lens: NOT commit-diff-scoped — walk the whole codebase looking for silent-failure surfaces, including ones that existed pre-T2 but weren't caught. Especially: `unwrap_or_default`, `if let Err(_) {}`, `Result` swallowing, `saturating_*` on sim-bearing fields without justification, `debug_assert!` for canonical invariants.*

**Owner:** Claude `pr-review-toolkit:silent-failure-hunter`
**Status:** _(pending — track populated by subagent on dispatch)_

### Findings

_To be filled by Track C subagent._

---

## Track D — Test-the-tests (Claude `qa-lead`)

*Lens: vacuous-test patterns, redundancy across the suite, coverage holes that test NAMES imply but the test bodies don't actually cover, insta snapshot quality. Especially: tests that read named constants at assert-time (where mutating the constant doesn't fail the test), tests that assert default-shape only, integration tests that re-implement the production code instead of asserting against it.*

**Owner:** Claude `qa-lead`
**Status:** _(pending — track populated by subagent on dispatch)_

### Findings

_To be filled by Track D subagent._

---

## Track E — Adversarial red-team (Codex CLI; user-driven)

*Lens: 4 attack goals.*
1. **Break canonical hash silently** — find a code change that produces drift bytes the existing pinned-hash test doesn't catch (e.g. fixture seed not in the corpus; per-OS divergence not in the matrix).
2. **Make a content pack pass validation while semantically invalid** — find a RON fixture that passes `fw-content-baker validate-structural` but produces wrong behavior at sim time.
3. **Malicious mod overlay** — author a mod-pack RON that exploits the load order / overlay rules per `Content/RULES.md §6` to inject banned terms / licensed data / dangling refs.
4. **Find a determinism leak** — identify a Rust pattern in the T2 diff that introduces non-determinism (hidden HashMap iteration, `Instant::now()`, OS-time read, thread-RNG, etc.) that the existing clippy bans don't catch.

**Owner:** Codex CLI (separate parallel session — paste prompt below)
**Status:** _(pending — user pastes prompt into Codex CLI)_

### Codex CLI prompt (copy-paste verbatim into Codex)

```
Adversarial red-team review at Phase T2 close, Final Whistle (Rust + Tauri 2 + SolidJS football-management sim).
Commit range: b2d41f9b..26540dcd on main (10 commits, +9844 / -276 LoC, 77 files).

You are CODEX. Your job is to BREAK the project's invariants. Be specific + reproducible.

4 attack goals (separate findings per goal):

1. CANONICAL-HASH BYPASS. Find a code change in the T2 diff (or a code change you can author) that produces canonical-state drift bytes the existing pinned-hash test does NOT catch. The pin is `crates/fw-replay/tests/canonical_hash.rs` with two seeds (60-tick `blake3:eaf842ac…ad46` + 600-tick `blake3:aa7efe9b…5ae`). Per-OS divergence is checked via CI matrix [macos-14, windows-latest, ubuntu-22.04]. Look for: fixture seeds not in the corpus that would show drift; per-OS f64 calls that snuck back in; SeedLayer collisions; encoder field-order drift; #[serde(skip)] on a field that SHOULD be in canonical state.

2. CONTENT-PACK SEMANTIC POISONING. Author a RON fixture that passes `cargo run -p fw-content-baker -- validate-structural` (the T2-3 validator-as-one-class refactor) BUT produces wrong behavior at sim time. E.g. a TacticalArchetype with formation slots that pass the structural check (11 slots, no duplicate roster_slots, buildup_speed_factor in range) but breaks `tick_match`. Or a Culture whose name banks pass the ≥20-entry check but produces banned-term output via Markov chain composition.

3. MALICIOUS MOD OVERLAY. Per `Content/RULES.md §6`, mod overlays load lex-sorted by mod-id + can override existing IDs via `overrides: "fwh.core:..."`. Author a mod-pack RON that:
   (a) injects banned terms via override (silently substituting a clean fixture's display_name with a banned one — does the load-time banned-terms check catch the override?)
   (b) creates a dangling cross-ref via override (modify a ManagerArchetype's `tactical_archetype_id` to point at a removed archetype — does the DanglingReference check catch the post-override state?)
   (c) bypasses the sentinel-block restriction (per T1-20 the `ui-lint:ignore-start/end` is restricted to `docs/` / `crates/` / `scripts/` + `.md` paths — find a path that the restriction misses)

4. DETERMINISM LEAK. Walk the T2 diff (especially T2-5 `fw-tauri::season` + T2-3 `fw-content-baker/bake.rs` + T2-2 `fw-content::league.rs`) for any pattern that introduces non-determinism without tripping the existing clippy bans. Look for: hidden HashMap iteration via library trait impls (e.g. `std::collections::HashMap::iter` invoked indirectly); `tokio::time::Instant` in fw-tauri command handlers that bleeds back into a canonical-state DTO; `std::collections::hash_map::DefaultHasher` invoked by serde; OS-time bleeding through `cargo audit`'s downloads timestamp into a build script.

For each goal:
- Describe the exploit in 3-5 sentences (reproducible — no hand-waving)
- Provide the EXACT file path + line numbers + minimal change to demonstrate
- State the SEVERITY of the silent-failure / break (gate-blocker vs pre-next-phase recommended vs doc-only)
- If you can't find an exploit for a goal, say so explicitly (negative result is useful)

Write your findings into `docs/audits/post-t2-ultimate-review-2026-05-18.md` under the "Track E — Adversarial red-team" section. Do NOT commit. Hand back to main thread when done.
```

### Findings

_To be filled by Codex CLI when the user runs the prompt above._

---

## Track F — Property explosion (Codex CLI; user-driven)

*Lens: bump PROPTEST_CASES from default 256 → 10,000 on key invariants; bump intra-process determinism count; run for ~15-20 min wall-clock.*

**Owner:** Codex CLI (separate parallel session — paste prompt below)
**Status:** _(pending — user pastes prompt into Codex CLI)_

### Codex CLI prompt (copy-paste verbatim into Codex)

```
Property-explosion run at Phase T2 close, Final Whistle.

You are CODEX. Your job is to find proptest failures that the default 256-case shrink misses. Use a higher case count + a different seed.

Pre-flight:
- `cd /Users/vibelogic/dev/football`
- `git status` should show clean tree on main + 11 commits ahead of origin
- Run `scripts/fw verify` first; must be green

Then:

1. Identify every proptest in the workspace: `grep -rn "#\[proptest\|proptest!" crates/ | head -30`
2. For each proptest, run with `PROPTEST_CASES=10000` (bump from default 256 -> 10000):
   `PROPTEST_CASES=10000 cargo test -p <crate> --release -- <test_name>`
   Use --release so 10k cases finish in reasonable wall-clock.
3. Also bump intra-process determinism: where a test runs N matches and compares hashes, bump N from 10 to 100 if the test allows.
4. Specifically target these high-value invariants:
   - fw-match-sim proptest: canonical-state determinism across seeds + ticks
   - fw-content proptest: League fixture pair-coverage symmetry (each (home, away) pair exactly once + per-club 19h/19a)
   - fw-replay proptest: encoder field-order stability + roundtrip
   - fw-save proptest: encode -> decode -> re-encode byte identity (if a proptest exists; if only the unit-test version, author a quick proptest with random SaveV1 fixtures + assert round-trip)
5. Time-box: ~15-20 min wall-clock. If a test hangs or panics, kill it + report the seed that triggered.

For each finding:
- Test name + crate
- PROPTEST_CASES that surfaced the failure
- Minimal-shrunk test case (proptest's shrink output)
- Expected vs actual behavior
- Severity classification (gate-blocker / pre-T3-recommended / opportunistic)

Write findings into `docs/audits/post-t2-ultimate-review-2026-05-18.md` under "Track F — Property explosion". Do NOT commit. Hand back to main thread when done.
```

### Findings

_To be filled by Codex CLI when the user runs the prompt above._

---

## Consolidated verdict

_To be filled by main thread after all 6 tracks land. Format per /done Step 5.5:_
- _Severity-sorted findings_
- _Cross-track convergence patterns (highest-value signal)_
- _Recommended new MASTER_PLAN rows (gate-blocking vs opportunistic)_
- _ACCEPT / REVISE / REJECT for the phase close_

**Status:** _(pending consolidation)_

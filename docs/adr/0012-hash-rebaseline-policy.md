# ADR-0012 — Pinned-hash rebaseline policy

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** Claude (Codex full-project audit Lane B "missing ADR" driver) + Codex (pending pre-T1-2b audit)

---

## Context

The pinned BLAKE3 canonical-state hash in `crates/fw-replay/tests/canonical_hash.rs` is the Phase-0 acceptance gate and the load-bearing determinism contract. The hash drift contract — when re-baselining is allowed, who authorises, what CI proof is required — lives in scattered comments + `docs/specs/determinism-gate.md` §9 references, but doesn't have an authoritative ADR. The Codex full-project audit Lane B flagged this as a missing ADR.

The Phase-0 P0 fix (commit `eb0b952e`) installed a three-layer guard against silent `#[ignore]` disable. This ADR formalises the **legitimate** drift path — when drift is intentional, what gates does it pass through?

## Decision

### Rebaseline triggers (the only legitimate cases)

A pinned hash drift is **authorised** only when one of these triggers fires:

1. **Canonical schema bump** — `MatchState` adds, removes, or reorders a field. Always invalidates the pinned hash by construction; the same commit MUST update the constant + the RON fixture's `expected_hash` field.
2. **Canonical encoder change** — `CanonicalEncoder::encode_match_state` changes byte layout (new section, reordered field write, version-prefix bump). Always invalidates; same lockstep update required.
3. **Sim behavior change with documented intent** — a bug fix or feature change to `tick_match` that intentionally shifts the post-60-tick canonical state. Author cites the design driver in the commit body (ADR / SPEC / DECISIONS.md reference) AND amends `docs/MASTER_PLAN.md` task done-criteria to acknowledge the drift.
4. **Cross-OS divergence repair** — if the CI matrix surfaces a real platform-specific drift, the fix is a code-level patch that brings all platforms back into agreement; the pinned hash captures the post-fix state. Author documents which platform was wrong + why in the commit body.

Any drift that doesn't match one of these triggers is a **regression**. The canonical-hash regression test fails the build; the developer investigates BEFORE proposing a rebaseline.

### Authorization

- **Solo-dev pre-Steam-EA period (now → T5):** osagberg authorises rebaselines via the commit body. Format: `canonical hash: REBASELINED (trigger: <1-4 above>; old: blake3:abc...; new: blake3:def...; reason: <one line>)`. The `validate-commit.sh` hook surfaces the marker.
- **Post-EA period (T5+):** rebaselines require a `docs/DECISIONS.md` entry citing the trigger, plus a green CI matrix on all three OSes BEFORE merge. The protect-decisions append-only hook enforces the audit trail.
- **Codex review at phase boundaries** explicitly inspects every commit body's `canonical hash:` line during the phase-gate audit. Drift without trigger documentation is a P0 finding.

### CI proof requirement

Every rebaseline commit MUST produce green CI matrix results on `[macos-14, windows-latest, ubuntu-22.04]` before merge. The pinned hash + RON-fixture `expected_hash` MUST agree across all three platforms — if any platform produces a different hash, the rebaseline is REJECTED and the divergence is the real bug (trigger #4).

The CI step that proves this is `determinism-gate.yml`'s "Run canonical-hash regression test" — runs on every push to a branch that targets `main`.

### Three-layer guard from Codex P0 fix

Per commit `eb0b952e`:
1. **In-process meta-test** `bedrock_pinned_test_is_not_ignored` (`canonical_hash.rs`) — fails `cargo test` if the bedrock has `#[ignore]`. **DURABLE**: runs on every `cargo test` invocation locally + in CI.
2. **CI workflow step** `Bedrock-test ignore-attr guard` (`determinism-gate.yml`) — fails CI if the bedrock has `#[ignore]`. **DURABLE**: enforced for every push.
3. **Commit hook** `canonical-hash-guard.sh` — refuses to commit when the staged diff touches canonical-state code OR the bedrock test file OR the corpus fixture, unless the test passes. **CONVENIENCE-ONLY**: this is a Claude-Code PreToolUse hook wired in `.claude/settings.json`, not a git hook (`core.hooksPath` is unset). It fires for `Bash(git commit*)` invocations made through Claude Code's Bash tool. Commits made via the OS shell (`git commit` from a terminal, IDE git integrations, CI bots) bypass it. **The durable protection is layers 1 + 2** — both fire regardless of how the commit is made.

These three layers are NOT subject to rebaseline. The hash itself rebaselines; the guard infrastructure stays untouched.

**Codex pre-T1-2b re-audit P1 reconciliation (2026-05-13):** the original ADR text treated all three layers as equally durable. They are not — layer 3 is convenience-only. The text above is the post-reconciliation framing. Optional future tightening: install repo-local `.githooks/pre-commit` that calls the same script, switching layer 3 from convenience-only to durable; tracked as a follow-up in `MEMORY.md` "Queued user actions" but NOT required because layers 1 + 2 are already adequate.

### Re-baselining workflow

The legitimate workflow (matching `docs/specs/determinism-gate.md` §9):

1. Make the canonical-state-affecting change. Stage it.
2. Run `cargo test --release -p fw-replay --test canonical_hash` locally — capture the failure message's "Expected: ... Actual: ..." pair.
3. Update `PINNED_60_TICK` in `canonical_hash.rs` AND `expected_hash` in `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron` to the new value. Both in the same commit.
4. Commit with body marker: `canonical hash: REBASELINED (trigger: <N>; old: blake3:...; new: blake3:...; reason: ...)`.
5. Push to a topic branch. CI runs on all three OSes; if all green AND all three produce the SAME new hash, the rebaseline is verified.
6. Merge.

If step 5 returns mismatched hashes across platforms, ROLL BACK. The real drift is platform-specific; the fix is code, not a rebaseline.

### Forbidden patterns

- `--no-verify` on a commit that touches the canonical-state-bearing crates' src/ OR the test file OR the corpus fixture. The hook is the convenience-tier guard (per §"Three-layer guard" — layers 1 + 2 are the durable enforcement; layer 3 is best-effort against Claude Code commits). Bypassing the hook isn't catastrophic in isolation — layers 1 + 2 still catch drift at CI — but it's a smell, and any commit that does it should be accompanied by the documented commit-body marker for layers 1 + 2 to read.
- `#[ignore]` on the bedrock test. Triple-guarded per the P0 fix.
- Mutating a `docs/DECISIONS.md` historical entry to retro-authorise a rebaseline. Decisions are append-only; supersede via new entry.

## Consequences

**Positive:**
- The rebaseline gate has a single authoritative spec. Future drift is auditable via commit-body grep.
- The four-trigger taxonomy makes "is this drift legitimate?" a decidable question, not a judgment call.
- The CI matrix proof requirement makes platform-specific drift loudly visible.
- Solo-dev workflow stays unblocking (commit-body authorization) while post-EA workflow tightens.

**Negative:**
- Every canonical-state change costs a rebaseline cycle. This is the determinism contract working as intended — the cost is the point.
- Codex phase-gate audits MUST grep commit bodies for `canonical hash:` markers. Adds review overhead.
- The post-EA Codex requirement assumes Codex is still active at that point. If not, replace with a peer reviewer or an LLM-as-a-service audit.

**Neutral:**
- This ADR is process-level, not code-level. No source change required; the hooks + CI + tests already implement the structural guard. The ADR documents the legitimate workflow on top.

**Rollback path:**
- If the four triggers turn out to miss a legitimate case in practice, append a fifth trigger via a new ADR that supersedes this one's §"Rebaseline triggers" section. The structural guard infrastructure stays.

## Alternatives considered

- **No formal policy — author judges case by case.** Rejected on auditability grounds. Solo-dev right now, but Codex / future-collaborators / future-me need a clear contract.
- **Forbid all rebaselines; canonical state never changes.** Rejected because canonical schema legitimately bumps (T2-9 save format + canonical-state-version coexistence; future signature system might add fields).
- **Rebaselines require a DECISIONS.md entry from day one.** Rejected for solo-dev period (overhead without proportional benefit when there's one author). Tightens post-EA.
- **Pinned hash lives in a separate "golden" repo.** Rejected on infrastructure-weight grounds — the in-repo regression test + RON fixture form is simpler and the audit trail is just `git log`.

## References

- `docs/specs/determinism-gate.md` §9 (the existing rebaseline procedure this ADR formalises)
- `crates/fw-replay/tests/canonical_hash.rs` (the bedrock test)
- `crates/fw-replay/fixtures/0xdeadbeefdeadbeef.ron` (the corpus fixture)
- `.claude/hooks/canonical-hash-guard.sh` (the commit-time guard)
- `.github/workflows/determinism-gate.yml` (the CI guard)
- Commit `eb0b952e` (the P0 three-layer fix)
- `.claude/hooks/validate-commit.sh` (surfaces `canonical hash:` markers)
- ADR-0010 §"Migration: forward-only" (save format coexists with canonical schema version)
- Codex full-project audit Lane B "missing ADRs"

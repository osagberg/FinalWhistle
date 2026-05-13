---
description: Read-only project health sweep — STATUS staleness, plan integrity, sim invariants, decisions log, banned terms, verify.
---

# /audit — read-only health check

Read-only. Do not edit anything. Output a checklist report with PASS/FAIL per check + suggested follow-ups.

## Checks

### 1. STATUS.md freshness
- Read `STATUS.md` timestamp.
- PASS if <24h since last update.
- FAIL otherwise → suggest `/status` then `/next` to refresh.

### 2. MASTER_PLAN row integrity
- Read `docs/MASTER_PLAN.md`.
- Every task row should match `DONE` | `TODO` | `BLOCKED` (case-sensitive).
- FAIL if any row is ambiguous, missing status, or uses other words.

### 3. Canonical-state crate touches without hash re-pin
```bash
git log --since="7 days ago" --name-only --pretty=format: -- 'crates/fw-match-sim/**' 'crates/fw-memory/**' 'crates/fw-replay/**' 'crates/fw-save/**'
```
For each commit touching those paths, check if the same commit also touched the pinned-hashes fixture (`crates/fw-replay/fixtures/*.ron` or wherever `scripts/fw verify` looks). FAIL if any sim/memory/replay/save commit shipped without a paired hash check.

### 4. HashMap / HashSet in sim crates
```bash
rg -n '\bHashMap\b|\bHashSet\b' crates/fw-match-sim crates/fw-memory crates/fw-replay crates/fw-save crates/fw-content crates/fw-scouting crates/fw-core
```
PASS if zero matches (clippy enforces, but verify).

### 5. f32 / f64 in canonical state
```bash
rg -n '\bf32\b|\bf64\b' crates/fw-match-sim/src crates/fw-memory/src crates/fw-replay/src crates/fw-save/src --type rust
```
Manual eyeball needed — distinguish canonical struct fields (FAIL) from test/doc paths (allowed).

### 6. tokio / async in sim layer
```bash
rg -n '\btokio\b|async fn|\.await\b' crates/fw-match-sim crates/fw-memory
```
FAIL if any match.

### 7. DECISIONS.md mutation check
```bash
git log -p docs/DECISIONS.md | rg '^-\s*-\s*\*\*\d{4}-'
```
Lines deleted that were dated bullets. PASS if zero.

### 8. Banned terms (UI vocabulary)
```bash
python3 scripts/lint-banned-terms.py content/ docs/design/ frontend/src/
```
PASS if zero violations. Sentinel-comment exemptions (`// ui-lint:allow ...`) are valid.

### 9. scripts/fw verify
```bash
scripts/fw verify
```
Report exit code + first failing step if non-zero.

### 10. Crate boundary direction
```bash
cargo tree -e normal --workspace 2>&1 | head -200
```
`fw-core` should have no workspace deps. `fw-match-sim` should not depend on `fw-tauri` or frontend.

## Output format

```
# /audit report — YYYY-MM-DD HH:MM

  [PASS] 1. STATUS freshness — 3h since update
  [FAIL] 2. MASTER_PLAN row integrity — 2 rows ambiguous:
           - Phase T1 row 4: "wip" (use TODO)
  [PASS] 3. Canonical-state hash pinning — 4 sim commits, all paired
  ...

## Follow-ups
- Fix MASTER_PLAN rows 4 + 7 (status spelling)
- Re-pin hash for commit abc1234 (touched fw-replay/src/reader.rs)
```

Stop after printing. Do not auto-fix. Audit is read-only.

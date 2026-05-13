# /next — worked examples

Four concrete walk-throughs of the 9-step pipeline for the most common Final Whistle task classes. Read this when you're new to the loop or unsure how to spec a task.

---

## Example A — sim-rust task (T0-5: implement Q32 saturating ops)

**Step 1** — STATUS shows no active task. MASTER_PLAN T0-5 row: `TODO` with `deps: T0-1, T0-3` both DONE.

**Step 2** — Pick T0-5.

**Step 3** — Spec:
```
task_id: T0-5
acceptance:
  - fw-core::q32 module exposes Q32::saturating_{add,sub,mul,div}
  - Unit tests cover: overflow saturates at Q32::MAX, underflow at Q32::MIN, div-by-zero returns Q32::MAX (signed) or Q32::ZERO (per spec)
  - proptest invariant: saturating_add(a,b) == saturating_add(b,a) for all (a, b)
  - cargo clippy --workspace -- -D warnings clean
files_in_scope:
  - crates/fw-core/src/q32.rs
  - crates/fw-core/src/lib.rs
files_out_of_scope: <defaults>
task_class: sim-rust
required_agent: gameplay-programmer
estimated_LoC: M (~200)
```

**Step 4** — `gameplay-programmer`.

**Step 5** — Spawn agent with spec. It implements + proptest invariant.

**Step 6** — `scripts/fw verify` green.

**Step 7** — ≥100 LoC → parallel three-way self-review. Silent-failure-hunter flags one MED: "saturating_div on signed Q32 with negative LHS and zero RHS — clarify behavior."

**Step 8** — Apply MED fix inline (clarifying comment + test). Commit `feat(core): saturating Q32 arithmetic`. Mark T0-5 DONE. CHANGELOG line. STATUS update.

**Step 9** — Continue to T0-6.

---

## Example B — content-narrative task (write 2 new culture RON files)

**Step 1-2** — MASTER_PLAN content phase row: TODO `seed 2 anglo-derivative cultures + 1 fantasy-elvish variant in content/sources/cultures/`.

**Step 3** — Spec:
```
task_id: T2-4
acceptance:
  - 3 new RON files under content/sources/cultures/, all schema_version: 1
  - Pack-qualified IDs in form fwh.core:culture_<5digit>
  - Each culture has ≥40 first names, ≥30 surnames, ≥5 club-name fragments
  - scripts/fw banned-terms clean
  - cargo run --bin fw-content-baker -- validate succeeds
files_in_scope:
  - content/sources/cultures/anglo-northern.ron
  - content/sources/cultures/anglo-isles.ron
  - content/sources/cultures/elvish-tirvane.ron
files_out_of_scope: <defaults>
task_class: content-narrative
required_agent: narrative-director
estimated_LoC: ~600 lines of RON
```

**Step 4** — `narrative-director`.

**Step 5** — Spawn narrative-director with spec + reference to `docs/design/ui-vocabulary.md`.

**Step 6** — `scripts/fw verify` + `scripts/fw banned-terms` green. `cargo run --bin fw-content-baker -- validate` confirms schema + ID uniqueness.

**Step 7** — Not ≥100 LoC of code (RON is data, not code). Self-review optional; sample 5 names per culture for tone check.

**Step 8** — Commit `feat(content): seed 3 cultures (anglo-northern, anglo-isles, elvish-tirvane)`.

**Step 9** — Continue.

---

## Example C — frontend-ui task (Squad table view with TanStack Table)

**Step 1-2** — MASTER_PLAN T3-2: `TODO` Squad screen — virtualized table of 25-player roster.

**Step 3** — Spec:
```
task_id: T3-2
acceptance:
  - Route /squad renders TanStack Table v8 with columns: name, position, age, signature, fitness, form
  - Sortable on every column; row virtualization for ≥50 rows
  - IPC command get_squad(club_id) added to fw-tauri returning SquadDTO
  - pnpm test green; pnpm lint green
  - Screenshot attached to commit
files_in_scope:
  - frontend/src/routes/Squad.tsx
  - frontend/src/lib/columns/squad.columns.ts
  - frontend/src/lib/types.ts (SquadDTO addition)
  - crates/fw-tauri/src/lib.rs (get_squad command)
files_out_of_scope: <defaults + crates/fw-match-sim/**>
task_class: frontend-ui
required_agent: ui-programmer
estimated_LoC: L (~350)
```

**Step 4** — `ui-programmer`.

**Step 5** — Spawn ui-programmer. They implement DTO in Rust + TS, command handler, route, columns, virtualization.

**Step 6** — `scripts/fw verify` green. Manual look-see in `just dev`: table renders, sort works, scroll smooth.

**Step 7** — ≥100 LoC → three-way self-review. Type-design-analyzer flags MED: "SquadDTO duplicates fields from PlayerDTO; consider extends." → Inline fix.

**Step 8** — Commit `feat(ui): Squad screen with virtualized TanStack table`. Attach screenshot path.

**Step 9** — Continue.

---

## Example D — pause-trigger fires (would touch DESIGN_DOC)

**Step 1-3** — Picked T1-7 (BT-runner change). Drafting spec, realize the design doc for behavior-trees doesn't cover the case the task asks for ("how do BT nodes share state across ticks?"). The implementation needs a design choice first.

**PAUSE — trigger #3** (DESIGN_DOC change implied).

Output:
```
PAUSED — trigger #3 (design choice needed for cross-tick BT state)

What happened: T1-7 asks for a BT node that remembers its previous tick's outcome. design/match-engine.md doesn't specify whether shared state lives on the player, the BT context, or a separate ledger.

Files touched so far: none — paused at spec stage.
Verify status: n/a.

Proposed next step:
  Option A: open /log-decision now, propose "BT state lives on PlayerState as a typed slot"
  Option B: defer T1-7, pick T1-8 next, come back when design decision lands

Decision needed: A or B?
```

User answers. Loop resumes per answer.

---

## Cross-references

- `.claude/skills/next/SKILL.md` — authoritative 9-step pipeline
- `.claude/commands/next.md` — thin slash-command wrapper
- `.claude/agents/README.md` — agent dispatch table

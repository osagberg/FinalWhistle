# Dev verification surface

**Status:** Accepted 2026-05-13
**Owning agents:** qa-lead (test strategy) + gameplay-programmer (instrumentation) + ui-programmer (tactical board)
**Affected phase:** T1 (and forward)

---

## Problem

`docs/DESIGN_DOC.md` commits the *shipped game* to a text-first surface: 2D tactical board + commentary, no 3D viewer. Players read about the match.

The shipped surface is not the development surface. A football sim has emergent behavior across 22 agents + ball — small attribute changes cascade into wildly different match outcomes, and the only way to know whether the new outcome is "better football" or "different random walk" is to watch a match. FW v1 surfaced its worst behavioral bugs (static-ball convergence, brain-dead pressing, goalkeeper-wanders-to-midfield) only when the dots viewer made them visible.

Without a developer-tier verification surface, T1 cannot be reviewed for correctness. We'd ship "two teams play a match" with no defensible answer to "is that match football?"

## The three layers

### Layer 1 — Diagnostic commentary

Rich event-by-event log. Not "Goal at 67:23" — closer to:

> 67:23 — Jones (CB) intercepts at own 18-yard box, plays 12m pass to Brown (DM); Brown turns under pressure from Müller (#9), switches play left to Davies (LB); Davies takes 3 touches, crosses early.

Enough detail in pure text to spot "the GK is wandering to midfield" or "the defenders never close down a 1-v-1" *without seeing the pitch*. Cheap; just commentary scaffolding around the existing event stream.

Lives in: `fw-memory` event readers + commentary template bank (`narrative-director` owns voice).

### Layer 2 — 2D tactical board (dev-tier, always available)

Same PixiJS rendering planned for the shipped UI, built minimally and earlier. Top-down pitch, 22 dots, the ball. Hit play. Watch the dots move.

This is what FW v1 had. It's the single most cost-effective sim-correctness tool we can build. The shipped game still leads with text — the board is a separate developer route (or a debug-toggle inside the Match page) that's always on for the dev and hidden for the player.

Lives in: `frontend/src/routes/Dev/TacticalBoard.tsx` (dev-only route) consuming the same `MatchFrameDTO` stream that the eventual ship-quality viewer in T4 will use.

T1-2a (this phase) ships the minimal version: dots + ball + tick scrubber. T4 polishes it to shipped quality: trails, role colours, signature-move highlights, etc.

### Layer 3 — Behavioral assertions in property tests

Things you'd notice visually but can also encode as invariants:

- "GK is within 30m of own goal in 95%+ of ticks across a 90-minute match."
- "Team width during in-possession phases is 35-65m, in 90%+ of windows."
- "No player sustains >12m/s for >4 consecutive seconds (cap is sprint physiology)."
- "Average defender depth during opponent in-possession is within 8m of the line height set by tactical archetype."

Runs in CI on every push. Catches behavioral regressions without anyone watching. Cheap per assertion; the cost is *authoring* them — and you author them best while sitting in front of Layer 2 watching what "good football" actually looks like in the sim.

Lives in: `crates/fw-match-sim/tests/behavior_proptest.rs` (added in T1-9).

## Sequencing

The order matters:

1. **T1-2a (board first)** — before any BT runner work. The board renders the placeholder 22-player stationary state from T0; it proves the rendering pipeline works on dummy data. Adding moving dots later costs nothing.
2. **T1-2b (BT runner)** — the actual XL work. Verified via the board landing in 2a + manual eyeballing. No assertions yet — too early; we don't know what "good" looks like until we've watched it.
3. **T1-4 commentary** — author the diagnostic event templates *while* watching matches play out in the board. The templates write themselves: you see something weird, you add a commentary line that would have surfaced it from text alone.
4. **T1-9 behavioral assertions** — author after T1-2b has been watched enough that "what's normal" is intuitive. Encode the invariants whose violation we'd notice visually. CI catches regressions from then on.

## What this is NOT

- **Not the shipped game's UI.** The shipped game leads with text + a polished 2D board (T4). The dev-tier board is minimal — it's a debug surface, not a product surface.
- **Not a replacement for the canonical-hash regression test.** That test still owns "did the simulation drift bit-for-bit across platforms / commits?" The verification layers here own "is the behavior football-shaped?" — two different questions.
- **Not optional.** Without these, T1's exit gate ("two teams play a match and it makes sense") cannot be verified. The minimum bar for T1 close is: matches play; the board renders them; three behavioral invariants hold over 100 random seeds.

## Costs

- Layer 1 (commentary): ~1-2 days inside T1-4.
- Layer 2 (board, dev-tier): ~3 days as a new T1-2a row. Reuses PixiJS + Tauri IPC infrastructure already scaffolded.
- Layer 3 (proptest assertions): ~1-2 days as a new T1-9 row. Author 5-10 invariants; iterate as we learn what "normal" looks like.

Total: ~6 days of work pulled out of T4 / added to T1. T4 retains the "polish to shipped quality" rows.

## Cross-references

- `docs/DESIGN_DOC.md` §6 (presentation rules — text-first for shipped game)
- `docs/MASTER_PLAN.md` Phase T1 (where these land)
- `docs/postmortems/phase-T0.md` (the "we couldn't verify behavior" lesson from FW v1 that drove this)
- `.claude/agents/qa-lead.md` (acceptance criteria authoring)
- `.claude/agents/ui-programmer.md` (PixiJS implementation)

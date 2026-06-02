# External dual review — Claude (Opus 4.8) + Codex — 2026-06-02

**Provenance:** Two independent AI reviewers, run *outside* and *alongside* the main `/next`
implementation session, read the live tree read-only (no edits to the canonical stream, no
ledger mutations, no tests run by the reviewers). This memo is the merged output. It is an
**advisory artifact** — the human + main session decide what becomes work. Convert findings to
`MASTER_PLAN.md` rows / `DECISIONS.md` entries through the normal flow; do not bypass hooks.

**Verification legend:**
- `[CONFIRMED]` — a reviewer personally read the cited code and verified the claim.
- `[RELAYED]` — cited with file:line by Codex; plausible + consistent, not independently re-read here. Verify against the lines before acting.

**Headline:** the highest-value findings are NOT in the match engine (the area under active
focus) — they are in the **career / save / scouting data layer**, which is the actual product
differentiator and is currently buggy, un-persisted, and not player-visible. Recommended
re-prioritisation at the bottom.

---

## F1 — Breakthrough RNG seeded by renumbered event ids (correctness; pillar 3) `[CONFIRMED]`

`evaluate()` seeds the breakthrough-delta RNG with `tick_for_rng = event.event_id.0`
(`crates/fw-memory/src/breakthrough.rs:984`). But the per-player event batch fed to `evaluate()`
is built by `filter_new_events_for_player`, which appends clones into a **fresh**
`MemoryLedger::new()` (`crates/fw-tauri/src/season.rs:655,662`), and `MemoryLedger::append`
**overwrites** `event_id` with a fresh monotonic counter starting at 0
(`crates/fw-memory/src/ledger.rs:171`). The comment at `season.rs:645-648` claims "we clone the
events without renumbering" — **this is false; the code renumbers.**

**Consequence:** the RNG `tick` is the within-season batch index (`0,1,2,…`), not the globally
unique career `EventId`. Because the incremental watermark feeds a fresh `0,1,2,…` batch every
season, the breakthrough magnitude for a given `(player, family)` at batch-position *k* draws the
**identical** `raw_delta` every season it occurs (same `career_seed`, same `tick`, same `site`).
The design intent (unique RNG site per career event) is silently defeated → breakthrough outcomes
become patterned across a long career. Not a determinism-gate break (fw-memory is outside the
canonical-hash path — which is why CI never caught it); it is a statistical-correctness defect.

**Action:** make the RNG `tick` the player's *global* career `EventId`, not the renumbered local
id (e.g. carry the original `EventId` through the filter without re-`append`-ing, or pass the
career-ledger id explicitly into `evaluate()`). Fix the lying comment. **Sequencing: do this
BEFORE SaveV4 (T4-2.5g) persists `BreakthroughState`** — otherwise live careers bake in buggy
progression and need a migration to undo.

---

## F2 — Scout reports not disambiguated by player; identity field mismatch (correctness; pillar 4) `[CONFIRMED]`

`observe_player` seeds with `seed_fn(career_seed, observation_id, SeedLayer::ScoutObservation, 0)`
(`crates/fw-scouting/src/observe.rs:39`) — `site` is hardcoded **0**, so the scout-noise stream is
keyed only by `observation_id`, not by the player. Combined with the roster's round-robin reuse of
a small bio pool (`crates/fw-tauri/src/season.rs:707-722`, with the gene-snapshot assert), two
distinct roster instances that map to the **same bio** AND share an `observation_count` produce
**byte-identical `ScoutReport`s** (the fn is documented as identical-inputs → identical-output).
That directly undermines the scouting-uncertainty pillar (reports are supposed to differ).

Separately, `ScoutReport.player_id` is set to `player_bio.player_id` (a **content-bio** id,
`observe.rs:57`) while the career/DTO layer uses the **roster** `PlayerId`
(`crates/fw-tauri/src/roster_dto.rs:133` `[RELAYED]`). Persisting reports verbatim freezes that
mismatch.

**Action:** thread the player identity into the scout-observation RNG `site` so noise is
per-player-independent; set/translate `ScoutReport.player_id` to the roster `PlayerId`. **Do this
BEFORE SaveV4 persists scout reports.**

---

## F3 — Career state does not persist; no production save/load (pillar 2/3/4) `[RELAYED]`

`SaveEnvelope` stops at V3 and `load_envelope` returns `SaveV3`
(`crates/fw-save/src/lib.rs:92,338`); no `save_career`/`load_career` commands are registered
(`crates/fw-tauri/src/lib.rs:42`, `src-tauri/src/main.rs:65`). So roster, scout reports,
breakthrough state, and the eval watermark are **restart-ephemeral** — "careers that remember"
does not survive closing the app. SaveV4 is already planned (T4-2.5g); this finding elevates it
and adds requirements:
- non-empty roster / scout / breakthrough fixtures in the migration tests;
- explicit V3→V4 identity mapping (V3 `breakthrough_states` are keyed by `PlayerId`; roster now
  uses the `ROSTER_PLAYER_ID_BASE` range — old content-bio keys won't attach without an explicit
  map) `[RELAYED]`;
- land F1 + F2 first so persisted data is correct.

Minor, related `[RELAYED]`: `breakthrough_eval_watermark` is set before compaction
(`crates/fw-tauri/src/commands.rs:1284,1300`); harmless today (compaction events have no player
subject) but off-by-one-prone if compaction ever gains participants.

---

## F4 — Pillars are backend-wired but NOT player-visible `[RELAYED]`

STATUS claims "all 5 pillars now produce player-visible career output" — too strong for the UI.
`get_squad` returns the 22 content bios, not `CareerState.roster`
(`crates/fw-tauri/src/commands.rs:728`, `frontend/src/lib/api/squad.ts:16`). No frontend
`get_roster_for_club` / `get_scout_report` wrappers. Rust has a `NotYetObserved` error variant
that TS `IpcError` lacks (`crates/fw-tauri/src/error.rs:84` vs `frontend/src/lib/types.ts:109`).
The Player page still renders old phenotype "Scout traits" and marks career-roster details
deferred (`frontend/src/routes/Player.tsx:176,214`). Signature commentary/name routing is still
TODO (`docs/MASTER_PLAN.md:332`).

**Action:** surface the roster/scout/breakthrough data in Squad + Player + scouting UI. The
differentiator currently exists only in Rust; a player can't see it.

---

## F5 — Match engine: analytics built but unwired; off-ball on fixed slots; full match over-scores `[CONFIRMED]`

Context for prioritisation (this is the long valley, lower priority than F1-F4 right now).
The analytics primitives are real and good (`utility/pitch_control.rs` Spearman model, `xg.rs`,
`xt.rs`), and **softmax selection is wired** (`subtree_library.rs:175,260`) — but it samples
**proxy** utilities: `utility_shoot` is an attribute-product stub (`bt/on_ball.rs:191,238`), pass
uses a `vision` proxy, pressing uses `aggression×anticipation`, and spatial inputs aren't threaded
into the BT context (`bt/off_ball.rs:20`). Off-ball targets are static formation slots
(`subtree_library.rs:82`; press→opponent-GK-slot `off_ball.rs:173`); no influence-map module
exists (`lib.rs` module list). Set-piece **taxonomy** exists (`tactic_fsm.rs:59`) but **restart
mechanics** don't (`lib.rs:1470,1235`); no offside/foul/card event variants.

**Calibration blind spot worth noting:** the canonical-hash pins are on **short** fixtures
(60/600-tick), whose goal envelope (2-5) looks sane (`canonical_hash.rs:964`) — so the gate is
green while a real **5400-tick** match over-scores (STATUS:15, deferred to T5-5b). CI proves
"stable," not "football-shaped."

**Critical path when this becomes priority:** thread spatial context → build influence maps →
wire xG/xT/pitch-control into selection → real off-ball movement → close the `calibrate.rs` loop
(coefficients are currently fitted-to-JSON but never applied).

---

## F6 — Effort skew: every fixture runs the full engine `[RELAYED]`

`advance_week_inner` runs `play_one_match` for **every** fixture
(`crates/fw-tauri/src/commands.rs:411`), while the design says only the player's club should use
the real engine and AI fixtures should be seeded-procgen
(`docs/design/career-roster-layer.md:98`). Performance + scope signal; aligns the codebase to the
intended policy.

---

## C1 — CI posture: do NOT drop macOS from the determinism gate `[RELAYED]` (corrects an earlier Claude rec)

Earlier advice to "drop macOS from both CI matrices" rested on a false premise: that a local hook
gates the canonical hash. It does not — the pre-commit hook runs only `just lint` + `just
test-fast` (`pre-commit:43`), and `canonical-hash-guard.sh` is convenience-only, bypassable, and
fails-open if `jq`/`cargo` are missing (`canonical-hash-guard.sh:34,91`; ADR-0012:47). The repo's
own policy requires all three OSes to agree on the hash (DESIGN_DOC:60, ADR-0012:38).

**Recommended posture (cheaper, keeps the safety):**
- KEEP the macOS leg on `determinism-gate.yml` (canonical-hash only) — optionally path-filter it
  to canonical-affecting paths + nightly if Actions minutes are tight on a private repo.
- MOVE the expensive macOS `ci.yml` full build-test (the ~10× cost driver) to path-filtered /
  nightly / phase-gate, not always-on.
- ADD a scheduled/manual macOS Tauri packaging dry-run before T5 (release packaging only runs on
  tags today and Mac signing is still placeholder-gated, `release.yml:116`).

---

## I1 — Infra: dev-only fixture `invoke` shim to unblock AI/browser vision `[RELAYED + design]`

The AI sessions cannot see the real game: Claude Preview drives plain Chrome where Tauri
`invoke()` is undefined, so backend routes (Squad/Career/Match) error. All frontend IPC funnels
through one `safeInvoke()` chokepoint (`runtime-validators.ts:40,399`), which makes a fixture shim
cheap and low-blast-radius. Capabilities are `core:default`+`log:default` only and CSP
`connect-src` is locked (`default.json:6`, `tauri.conf.json:31`) — which argues *against* an
HTTP-backend-first approach.

**Recommendation:** add a DEV-gated, explicit-opt-in (`?backend=fixtures` or
`VITE_FW_BROWSER_BACKEND=fixtures`), **fail-loud** fixture shim at `safeInvoke` for read paths
(`get_settings`, `get_squad`, `get_player_detail`, `get_standings`, `get_fixtures`,
`get_career_overview`, `play_match`, `match_frames`), validated by the existing guards (throw on
missing/bad shape — no silent fallback). Don't make `isTauri()` lie — add a separate "has backend"
concept. Store fixtures under `frontend/public/dev-fixtures/`.
- The 2D **board** is already fully verifiable today via real deterministic `dump_frames` data.
- The shim gives realistic *static* UI (visual iteration + screenshots), **not** stateful flows.
- HTTP-backend dev mode and `tauri-driver` (real-data + native smoke) are deliberate later steps.

---

## Recommended re-prioritisation

The differentiator (the career layer) is currently incorrect, un-persisted, and invisible. Make it
**correct → persistent → visible** before perfecting the match engine.

1. **F1** — fix breakthrough RNG event-id keying (+ comment). *Blocks SaveV4.*
2. **F2** — fix scout `site` disambiguation + `ScoutReport.player_id`. *Blocks persisting reports.*
3. **F3** — SaveV4 with correct, non-empty roster/scout/breakthrough state + explicit identity map.
4. **F4** — surface roster/scout/breakthrough in Squad + Player UI (make the pillars visible).
5. **I1** — fixture invoke shim (so 1-4 are visible/iterable in the browser), in parallel.
6. **F5/F6** — match-engine integration + calibration, and the per-fixture-sim policy: the
   long-term moat, but second to the pillar layer right now.
7. **C1** — apply the CI posture when flipping the repo private.

*Reviewers ran read-only; no tests executed by them. Re-verify `[RELAYED]` items against the cited
lines before committing work.*

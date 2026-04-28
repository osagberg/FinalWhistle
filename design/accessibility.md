---
description: Target accessibility features for EA + how they wire to already-seeded hooks across ADR-0001 / 0002 / 0006, semantic-cinema, ui-vocabulary, and the golden replay corpus. Five-feature EA surface; Phase-7 implementation.
last_verified: 2026-04-24
status: Phase 2 authoring pass — EA feature set locked to the 5 items named in SPEC.md Phase 7 (reduce-motion / colorblind / remappable controls / large-text / subtitles). Per-feature semantics committed here. Mix of synthesis (reduce-motion already fully wired across ADRs 0001/0002; default-off advanced details already locked in ADR-0006) and fresh commitment (colorblind palette policy, subtitle-timing rules, text-scale factors, input-remap surface).
---

# Accessibility — EA target feature set

## Purpose

Answer "what accessibility features ship at EA, what does each one actually do in the codebase, and which hooks already exist to implement them cleanly instead of bolted on?"

Framing: accessibility is not a pre-release polish pass. It's a posture every system already respects. `ShotTypeSO` ships `reduce_motion_variant` from ADR-0001 day one; the golden replay corpus already has a `reduce_motion` field; the banned-terms lint already stops unpredictable vocabulary; the advanced scout tooltip is already default-OFF per ADR-0006. This doc commits the EA surface and wires the remaining semantics (colorblind palette, subtitles, text scale, input remap) to the same architectural posture.

## Locked decisions

See SPEC.md Phase 7 (2026-04-22): *"Accessibility: subtitles, colorblind, remappable controls, large-text UI, reduce-motion toggle"* — five items. This doc locks what each one means + how it implements.

**Cross-system hooks already seeded (synthesis):**

- **Reduce-motion** — `ShotTypeSO.reduce_motion_variant` per ADR-0001 + `EffectiveShotTypeId` resolution per ADR-0008 (the bridge resolves `BaseShotTypeId` → `EffectiveShotTypeId` once at viewer-event derivation; the `ReduceMotionApplied` flag travels on the `ViewerEvent`). Adapter-owned presentation behavior decides what the substitution looks like (the dots-phase ADR-0009 adapter currently disables motion-line / impact-flash equivalents at scene-load; future 3D adapter would make its own choice). Canonical MatchSim hash + `key_event_hashes` are identical with/without reduce-motion (presentation-only); adapter-keyed `pass_activation_log_hashes` MAY differ. `reduce_motion` field already in `golden-replay-corpus.md` JSON schema. (Earlier ADR-0002 references in this doc are historical; ADR-0002 is superseded by ADR-0008/0009 per SPEC 2026-04-26 decisions log — old shot-effect examples below stay as examples, not implementation commitments.)
- **Default-off advanced details** — ADR-0006 `§Q3` advanced tooltip opt-in; never exposes raw `InternalGeneSnapshot`; only category-level `GeneCategoryEstimate` ranges.
- **Predictable vocabulary** — banned-terms lint Category A hard-ban + Category B audited exemption per `design/ui-vocabulary.md`; British-football vernacular default; no capitalized mystical state nouns.
- **Typography readability** — Anton display / JetBrains Mono data / Rajdhani body per `design/semantic-cinema.md` 2026-04-24 resolution; scoreline uses Rajdhani SemiBold or JetBrains Mono (never Anton — Anton is decorative, not OCR-friendly).
- **Breakthrough-moment cinema timing** — 3-5s duration range per `design/breakthrough-moments.md` resolution; reduce-motion path collapses to a static post-match surface, not silent (next section spells this out).
- **Callback-deferral on high-leverage play** — pillar tiebreaker locked in `design/overview.md` Q4 resolution (callbacks defer to next natural surface during high-leverage sequences); accessibility angle is that live play is never interrupted, so screen-reader-friendly quiet moments are predictable.

**Fresh commitments in this pass (new):**

- Colorblind palette policy + CI validation step
- Subtitle-timing rules + post-match accumulator
- Text-scale factors + minimum readable sizes
- Input-remap surface + default scheme
- Accessibility replay-fixture pattern leveraging the existing `reduce_motion` corpus field

## The EA accessibility feature set — five items, locked scope

Each feature below is IN at EA. Nothing listed under §MVP boundary or §Deferred is in at EA.

### 1. Reduce-motion toggle

**Consolidation of already-locked behavior.** Single on/off toggle in the accessibility settings panel. When ON:

| Subsystem | ON behavior | Source |
|---|---|---|
| Shot-type resolution | Bridge resolves `BaseShotTypeId` → `EffectiveShotTypeId` once via `ShotTypeSO.reduce_motion_variant`; `ReduceMotionApplied` flag travels on the `ViewerEvent`; sim-side state untouched | ADR-0008 `ShotPresentationContract` + ADR-0001 §Schema + §Deterministic selection |
| Adapter-owned motion-effects | Each renderer adapter decides what reduce-motion means for its presentation (e.g. dots-phase ADR-0009 disables motion-line / impact-flash equivalents at scene-load; 3D adapter would make its own choice). Old ADR-0002 examples below are illustrative, not implementation commitments | ADR-0009 §Polish-bar + adapter-specific config |
| Screen-tone pass *(historical example, ADR-0002 superseded)* | Static overlay at fixed intensity; `_Time`-driven hatching disabled — illustrative of the kind of motion-effect an adapter might disable | (superseded — ADR-0002) |
| Motion-line trails *(historical example, ADR-0002 superseded)* | Feature unregistered at scene-load; no per-player mesh — illustrative | (superseded — ADR-0002) |
| Impact flash *(historical example, ADR-0002 superseded)* | Feature unregistered at scene-load; no fullscreen white-flash pulse — illustrative | (superseded — ADR-0002) |
| `aftermath-freeze` holds | Extended hold duration (+30% at stakes=1.0) so the player has more time to read post-event text | new commitment |
| Breakthrough cinema (3-5s) | Collapses to a static post-match surface: a stat-card entry with the same two-tier observational text; no mid-match cinematic | synthesis: `design/breakthrough-moments.md` resolution + reduce-motion variant rule |
| `crowd-reaction` cutaways | Replaced with `aftermath-freeze` substitute shot (chained via `ShotTypeSO.reduce_motion_variant`) | ADR-0001 |
| Persistent-scoreline typography | Unchanged (already Rajdhani SemiBold or JetBrains Mono, not Anton) | `design/semantic-cinema.md` |

**Reduce-motion is NOT silent mode.** Game still plays; sim runs; match narrates via text + audio. What changes is the visual-motion surface.

**Reduce-motion is scene-load-time, not per-frame.** Toggling mid-match requires scene reload to a match-viewer re-init; UI confirms (*"Change will apply on next match start"*). The structural posture is owned by the active renderer adapter (ADR-0009 for dots; future 3D adapter would respect the same boundary); revisiting it for in-match toggling is deferred post-EA. (Historical: this discipline originated in ADR-0002, superseded by ADR-0008/0009.)

**Validator invariant (owned by the Phase-6 content-pack validator):** any `ShotTypeSO` whose base variant uses an impact-flash or motion-line feature MUST declare a `reduce_motion_variant`. During Phase-3 shot authoring this may run as an authoring warning while the 7-shot vocabulary is still moving, but by Phase-6 content-pack v1 / EA lock it is a Tier-A blocking check (`FW-VAL-A-021`). Shipping content cannot omit reduce-motion coverage for impact-flash / motion-line shots.

### 2. Colorblind-safe palette

**Fresh commitment.** Three validated palettes selectable in the settings panel:

- `default` — project palette tuned for `Anton` + `Rajdhani` type contrast; contrast ratio ≥ 4.5:1 for body text over every background state (routine play / stakes elevated / post-match) per WCAG AA.
- `deuteranopia` — red / green axis remapped; team-color-coded UI elements (home vs away kit swatches, ledger callback severity, scout-archetype indicators) use shape + position redundancy in addition to color.
- `protanopia` — same axis remap, brightness-balanced differently than deuteranopia because the cone-response profile differs.
- `tritanopia` — blue / yellow axis remapped; rarer but cheap to include once the redundancy-coding pass is done for the above two.

**Color is never the sole carrier of information.** Every UI state that uses color to discriminate (team kits in match-view, ledger callback severity, scout-archetype visual tag, stakes-elevation intensity) also uses one of: icon shape, label text, position, pattern/hatching. Tested against Sim Daltonism + Color Oracle filters on a CI-uploaded settings-panel screenshot in Tier-C (Phase 7).

**Validator invariant (Phase-7 SPEC owed):** `colorblind-contrast-audit` runs the settings-panel + match-view + scout-report screenshots through a CLI colorblind simulator, asserts each discriminating element remains distinguishable. Fails the RC gate if any element becomes indistinguishable.

**Stakes-modulated colour grading interaction** — per `design/semantic-cinema.md`, colour grading shifts with stakes (cool desaturation → warm saturation as stakes rise). Under colorblind palettes, the saturation shift remains (it's a lightness/vividness cue, not hue-dependent), but the warm/cool hue shift is dampened. This keeps stakes legible without relying on hue discrimination.

### 3. Remappable controls

**Fresh commitment.** The Unity Input System owns all input. Every action is a declared input action, never a raw key poll. Default scheme + remap surface locked below; rebinding UI ships in Phase 7.

**Default input scheme (keyboard-first, since this is a management sim):**

| Action category | Default binding | Remappable? | Notes |
|---|---|---|---|
| Menu navigation | Arrow keys + Tab + Enter | Yes | Tab order is explicit per UI Toolkit best practice; no focus traps |
| Match-viewer controls (pause, speed, pip-in-pip) | Space / +/- / Shift+P | Yes | |
| Quick-access hotkeys (squad / tactics / inbox / fixtures) | 1-4 | Yes | 4 primary screens; hotkeys optional, menus fully mouse-drivable |
| Screenshot / replay-save | F12 / Ctrl+R | Yes | |
| Accessibility menu | F1 | **No** (fixed; always discoverable) | Accessibility settings must be reachable without first completing any other remap |

**Gamepad support:** best-effort at EA (Xbox + DualShock via Unity Input System auto-profiles). Full gamepad parity — especially for dense management screens — is a Phase-7-only-if-time target; deferred realistically to post-EA.

**No QTE / timing-sensitive inputs anywhere in the game.** Every decision is deliberative; no "press X within 1.5s." Breakthrough moments and signature executions resolve deterministically from prior decisions (per `design/breakthrough-moments.md` — match-flow cinematic, not pause-QTE).

**Mouse-only parity:** every action achievable via keyboard is also achievable via mouse. No keyboard-only paths. This keeps the input surface triply redundant (keyboard / mouse / gamepad-best-effort).

### 4. Large-text UI

**Fresh commitment.** Three text-scale settings:

| Scale | Factor | Minimum body-text size | Target |
|---|---|---|---|
| `small` | 0.85× | 12px | Dense-info display (power users, big monitors) |
| `default` | 1.0× | 14px | Baseline |
| `large` | 1.25× | 18px | Accessibility-forward; readable at normal viewing distance on a 1080p 24" monitor |

**No `xlarge` at EA.** Going beyond 1.25× forces UI-reflow layouts that don't fit the dense management-screen discipline without a full Phase-7 reflow pass. Revisit post-EA per § Deferred.

**Text-density interaction:** management screens are information-dense by design (anti-FM26-regression posture from `design/overview.md`). Large text reduces items-per-screen. The Phase-7 large-text pass verifies each management screen has a sensible large-text layout — columns reflow or collapse, no horizontal scrolling, pagination thresholds tightened. This is UI work, not architecture work.

**Monospace data cells stay monospace at all scales.** Numbers align. Tables remain scannable. This is why JetBrains Mono is the data/stat font per `design/semantic-cinema.md`.

**Subtitle / overlay text is always rendered at `default` or `large` scale** — never `small`, since the semantic-cinema overlay text is already time-limited (3-5s per breakthrough moment, ≤2.5s per `aftermath-freeze`) and small-scale would make it unreadable during its short window. Settings-panel subtitle-scale picker offers default + large only; small is management-screens-only.

### 5. Subtitles / text-first mode

**Fresh commitment.** All audio-carried information has a text equivalent. Since the game ships with NO voiced commentary at MVP (text commentary only, per `PROJECT_CONTEXT.md §5`), "subtitles" here means something slightly different from a typical audio-subtitle pairing — it means:

**What has subtitles:**

- **Crowd-audio cues** — chants, stadium hush moments, cup-final roar, relegation silence. Each tagged audio cue has a one-line text overlay (off by default; toggleable). Example: a home-goal roar-up plays alongside an optional `(Home crowd erupts)` overlay.
- **Match-state stings** — kickoff / half-time / full-time / goal / red card audio stings each map to a textual overlay when subtitles are ON.
- **Tutorial audio callouts** (if any ship at EA — currently tutorial is text-woven-into-viewer per `PROJECT_CONTEXT.md §6`, so this is likely empty at EA but the hook exists).

**Subtitle timing rules:**

- Display duration: min(max(word_count × 0.25s, 1.5s), 4.0s).
- Max 2 lines on screen simultaneously — older subtitles fade out when a third enters.
- Position: bottom-center by default, offset up when the `aftermath-freeze` text overlay is active (collision rule).
- Background: semi-transparent dark band (rgba 0,0,0,0.6) for WCAG AA contrast on any crowd / pitch background.

**Post-match audio-log accumulator.** Every audio-carried moment during a match accumulates to a post-match "what happened" text log, regardless of subtitle-toggle state. Players who join mid-match, step away, or play with sound off can reconstruct the match's audio story from text alone. This is a free win from the event-sourced memory posture — match events already emit; subtitles are just an additional render channel.

**Commentary text remains the primary narrative.** Subtitles don't replace commentary; they augment it by surfacing the audio-only cues commentary typically doesn't narrate ("the crowd has gone quiet" is commentary prose; a `(Stadium hush)` subtitle is the raw event).

## Cross-system accessibility discipline

### Default-OFF advanced details (already locked)

Per ADR-0006 `§Q3`, the advanced scout-report tooltip is default-OFF. When OFF, players see phenotype labels + scout prose only (`"Composed in traffic. Late bloomer. Awkward in the air."`). When ON, the tooltip reveals category-level `GeneCategoryEstimate` ranges (`Physical: 0.55-0.72 · Mental: 0.61-0.84 · Technical: 0.48-0.66`) — never raw gene values, never the walled-off `InternalGeneSnapshot`. This IS an accessibility posture: the default surface is narrative + qualitative, and numeric depth is opt-in, not required to play.

### Predictable vocabulary (already locked)

The banned-terms lint (`design/ui-vocabulary.md` + `scripts/lint-banned-terms.py`) is an accessibility tool. Consistent football-native vocabulary means screen readers produce predictable output, translation quality stays high, and cognitive load for new players is bounded to real-football concepts. No capitalized mystical state nouns, no RPG progression vocabulary, no genetics terminology. British-football vernacular default; locale-specific idiom via locale-specific banned-term lists.

### Focus / keyboard-navigation discipline (UI Toolkit)

Every UI Toolkit element declared focusable receives a visible focus ring (contrast ≥ 3:1 vs adjacent background). Focus order follows reading order. No focus traps; Escape always exits modals. This is UI-programmer discipline enforced during Phase-3+ screen authoring; validator-style enforcement is deferred (§Open questions).

### Screen-reader compatibility

Not in scope at EA. Unity UI Toolkit's screen-reader support on Windows / macOS / Linux is uneven (as of 2026-04 reference). Planned posture: the post-match text log + accessible label metadata on dense controls form a foundation; a dedicated screen-reader pass happens post-EA contingent on audience signal. Out at EA, seeded now.

## Replay / viewer test expectations

The golden replay corpus already pins a `reduce_motion: false|true` field per fixture (`design/specs/golden-replay-corpus.md §Schema v1`). Accessibility test expectations ride this:

- **Paired corpus fixtures at Phase 3.** Each Tier-A smoke seed gets two fixtures: `<seed>.json` (`reduce_motion: false`) + `<seed>.reduce-motion.json` (`reduce_motion: true`). Both must produce identical MatchSim canonical-state hashes and identical `key_event_hashes`; reduce-motion is presentation-only and cannot perturb the event stream. Adapter-keyed `pass_activation_log_hashes` (e.g. `pass_activation_log_hashes["dots"]`) may differ when shot-type selection routes through `reduce_motion_variant`, and both pass-activation hashes are pinned per adapter so render-path drift is still reviewable.
- **Colorblind-mode rendering parity (Phase 7 Tier-C):** rendered settings-panel + match-view + scout-report screenshots pass the colorblind simulator audit for deuteranopia / protanopia / tritanopia. Per-element distinguishability gates the RC.
- **Text-scale layout parity (Phase 7 Tier-C):** each management screen is captured at `small` / `default` / `large` and inspected for overflow, truncation, and unreachable controls. Tier-C because Tier-A can't afford 3× the screenshot inventory.
- **Subtitle-toggle regression (Phase 6+):** match-replay with subtitles ON produces a subtitle event log; re-run identical seed with subtitles OFF produces no subtitle events but identical MatchSim canonical-state hash. Subtitles are a pure presentation channel with no sim coupling.

## MVP boundary

**In at EA (Phase 7 polish target):**
1. Reduce-motion toggle (feature-disable-at-scene-load; `ShotTypeSO.reduce_motion_variant` substitution; aftermath-hold extension).
2. Colorblind palette — default + deuteranopia + protanopia + tritanopia. Color-never-sole-carrier discipline CI-audited.
3. Remappable controls via Unity Input System; keyboard + mouse parity; gamepad best-effort.
4. Large-text UI — small / default / large scales; Phase-7 reflow pass.
5. Subtitles — on/off toggle; crowd / stings / tutorial (if any) covered; post-match text log always-on.

**Underlying postures IN at EA (already architecturally locked):**
- Predictable banned-terms vocabulary.
- Default-OFF advanced scout numeric details.
- No QTE / timing-sensitive inputs.
- Live play never interrupted mid-sequence by callbacks (pillar tiebreaker).
- Canonical-JSON content packs + structured-data-rendered prose (no runtime LLM text = deterministic subtitle-log behavior).

**Out at EA (feature deferred; hook present where applicable):**
- Full gamepad parity on dense management screens.
- `xlarge` text scale (needs full reflow).
- Screen-reader (platform inconsistency + scope).
- In-match reduce-motion toggle (requires scene re-init, which is Phase-3 architectural posture).
- Closed-captions beyond the above-enumerated audio cues (no voiced commentary at MVP).
- Dyslexia-friendly font option (OpenDyslexic evaluation — post-EA trigger on player request).

## Deferred

Seeded now; surfaces post-EA contingent on audience signal:

- **Screen-reader support** across management screens. Needs accessible-label metadata pass (seeded via UI-programmer discipline) + platform-specific testing.
- **Dyslexia-friendly font variant** (OpenDyslexic Pro or similar). Typography pipeline supports alternate fonts; rendering stack respects font per-element. No architectural work; content + QA.
- **Cognitive-load presets** — "simplified UI" that hides advanced management surfaces (finances, staff contracts, youth-pool details) for new / cognitively-fatigued players. Phase-9+ scope. Data layer already supports progressive disclosure.
- **Full gamepad parity** across every management screen. Requires per-screen input-design pass.
- **Subtitle-positioning customization** (top vs bottom; size-in-subtitle).
- **Motion-sickness-reduced mode** beyond reduce-motion (e.g. disable stakes-elevated colour-grading shifts entirely). Hook is `ViewerState.ColourGradeIntensity` — post-EA UI.
- **Audio accessibility beyond subtitles** — mono audio mix, visualized low-frequency cues, frequency-shifted audio profile for hearing-range variance.

## Open questions

Deferred to Phase 3+ with the trigger condition named:

1. **Subtitle-event payload shape.** What exact fields does a subtitle event emit (event kind, text template ID, slot-fill data, display duration, position hint)? Pairs with the Phase-3 subtitle-event schema under `Memory.Contracts` (which also owns commentary events). Currently specified as "has subtitles" / "has post-match text log" — the structured-data shape lands Phase 3 when crowd-audio cue authoring starts.

2. **Focus-ring contrast validator.** UI Toolkit doesn't enforce focus-ring contrast; easy to author a screen where the focus ring is invisible on a dark background. A `fw focus-ring-audit` CLI tool that renders each screen and samples focus-ring contrast ratios is in the same family as `fw shader-audit` from ADR-0002. Trigger: Phase-7 UI polish pass, OR earlier if a screen ships with an unreadable focus ring. Not in the current Phase-2 SPEC as a task — would be added at Phase 7 when the list of management screens is known.

3. **Localization accessibility parity.** Each locale's banned-term list + font readability + subtitle line-length rules are locale-specific. Phase-7 localization pass covers EN-GB at EA; JP / ES / PT / DE post-EA. For now the spec treats EN-GB as the baseline; locale-specific accessibility spec lives alongside each locale's banned-term list.

## Prototype gate

Three gates, one per phase where accessibility surfaces are exercised:

**Phase 3 — reduce-motion determinism proof.** At Month-3 slice the paired corpus fixtures (`<seed>.json` + `<seed>.reduce-motion.json`) exist, both pass, and `ShotTypeSO.reduce_motion_variant` substitution demonstrably changes the rendered output (visual inspection of one match capture in each mode) without changing sim canonical state hash. Failure = reduce-motion is not architecturally real, revisit before Phase 4.

**Phase 6 — colorblind + text-scale content-pack coverage.** When content pack v1 compiles at Phase 6, the 7-shot + 24-signature + 46-phenotype-label surface ships with: reduce-motion variants declared and enforced by blocking `FW-VAL-A-021`; colorblind-mode screenshots audited; large-text layout verified for at least the 10 most-used management screens. Failure = Phase 7 polish carries the debt.

**Phase 7 — full accessibility pass gate.** All five EA features live, settings-panel discoverable, all four corpus fixture families passing (normal / reduce-motion × default / colorblind / large-text where capture applies). Colorblind-contrast-audit CI step green on RC candidate builds. Gate condition = *"Can a player with one of {reduce-motion need, one colorblind type, large-text requirement, subtitle requirement, remap requirement} complete a full match without hitting a readability / discrimination / operability blocker?"* Five testers, one per need. Pass = 5/5 complete; Fail = RC blocked, fix before Phase 8 EA lock.

## Cross-references

- **SPEC Phase 7:** canonical EA accessibility feature list (five items)
- **ADR-0001 ShotTypeSO:** `reduce_motion_variant` field + substitution rules
- **ADR-0002 Viewer rendering pipeline:** `ViewerState.ReduceMotion` scene-load-time feature disable + no-per-frame-branching posture
- **ADR-0006 IdentityPacket / AI Content Compiler:** `§Q3` advanced tooltip default-OFF + category-level-only range exposure + walled-off `InternalGeneSnapshot`
- **`design/semantic-cinema.md`:** typography stack (Anton display / JetBrains Mono data / Rajdhani body / scoreline override) + shot-type vocabulary + timing
- **`design/breakthrough-moments.md`:** 3-5s cinema duration; reduce-motion path collapses to post-match static surface
- **`design/ui-vocabulary.md`:** banned-terms lint + British-football vernacular + Category B audited-exemption
- **`design/overview.md` Q4 resolution:** pillar tiebreaker — callbacks defer to next natural surface during high-leverage play (accessibility angle: live play is never interrupted)
- **`design/specs/golden-replay-corpus.md`:** `reduce_motion` field already present in JSON schema v1
- **`design/specs/content-pack-validation-contract.md`:** existing checks that intersect accessibility (`FW-VAL-A-015` Category-A vocabulary, `FW-VAL-A-019` locale coverage); potential additions flagged in §Open Question 1

## Changelog within this doc

- **2026-04-24** — Authored as Phase-2 scope commitment. Five-item EA accessibility surface locked to the Phase-7 SPEC list; per-feature semantics committed. Reduce-motion is synthesis (already wired across ADR-0001 + ADR-0002 + corpus spec); default-OFF advanced details is synthesis (ADR-0006). Fresh commitments: colorblind palette policy (default + 3 validated palettes, color-never-sole-carrier discipline), subtitle-timing rules + post-match text-log accumulator, text-scale factors (0.85× / 1.0× / 1.25×, no xlarge at EA), input-remap surface (Unity Input System; keyboard + mouse parity; gamepad best-effort). Three prototype gates owed at Phase 3 / 6 / 7. Three open questions flagged. Replay/viewer test expectations ride the existing corpus `reduce_motion` field.

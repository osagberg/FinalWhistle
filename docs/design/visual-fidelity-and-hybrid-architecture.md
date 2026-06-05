# Visual fidelity & hybrid-render architecture — exploratory research

> **STATUS: EXPLORATORY RESEARCH, NOT A COMMITTED DECISION.**
> This doc captures recon findings + research on what it would take to let Final
> Whistle's match view grow from the current 2D dot board toward 2.5D and
> (optionally) 3D *without ever touching the deterministic sim*. It is written so
> the owner can make the product-vision call in §6 from a grounded position. The
> only thing here proposed for near-term action is the small contract-hygiene set
> in §2 — and even that is presented for sign-off, not auto-adopted. Nothing in
> this doc changes the locked tech stack (CLAUDE.md §3) or the design contract
> (`docs/DESIGN_DOC.md`). The text-first/2D-board direction remains the standing
> contract until §6 is decided.

---

## 1. Current-contract assessment — what the frame stream carries today

The match view is fed by two clean, disjoint, **unversioned** contracts. The seam
is sound — the frontend imports zero sim crates, every DTO is a documented one-way
Q32→f64 projection, the renderer interpolates screen positions only and never
re-simulates. The gaps below are about *signal carried*, not about leakage.

### 1.1 The position stream — `MatchFrameDto`

Single source of truth in `crates/fw-match-sim/src/dto.rs`, mirrored in
`frontend/src/lib/types.ts`. Both `dump_frames` (stdout JSON) and the
`match_frames` Tauri command emit it. JSON round-trip tested.

| Carried today | Per frame | Per player (`PlayerFrameDto`) | Per ball (`BallFrameDto`) |
|---|---|---|---|
| Present | `seedHex`, `tick`, `homeScore`, `awayScore`, `possession` (slot or null) | `slot`, `posX`, `posY`, `velX`, `velY` | `posX`, `posY`, **`posZ`**, `velX`, `velY`, **`velZ`** |
| Carried canonically but dropped at DTO | — | — | `spinX/Y/Z` (currently 0, so no loss yet) |
| **Missing at the canonical level too** | no `phase`, no `period` | **no body orientation / facing** | — |
| **No version stamp anywhere** | `MatchFrameDto`, `MatchEventDto`, the live-match DTOs all lack a `schemaVersion` field | | |

The board already wins over the real-world tracking standards (FIFA EPTS, the 2025
Common Data Format) on ball height: `posZ`/`velZ` are projected. The one field that
a 2.5D/3D view genuinely cannot fake — **player body orientation / facing** — is
absent in canonical `PlayerState`, not merely dropped at the boundary. The only
`heading` in canonical state is `technical.heading`, the *attribute*, not a
body-facing angle.

### 1.2 The event stream — `MatchEventDto` (separate contract)

`crates/fw-tauri/src/result.rs`: flat `{tick, minute, kind, description?}`. `kind`
is the closed PascalCase discriminant union (11 kinds, enforced as a TS compile
error on a new Rust variant). `description` is always `None` today. The live-match
`MatchSnapshot` is scoreboard/feed-grade — score, possession %, coarse 5-bucket
`ballZone`, last-16 events — and carries **no positions**.

So positions and discrete events are two disjoint streams with **nothing fusing
them on a shared timeline**. This is why `inspect_frames.rs` documents its phasing
detector as a *lower bound*: the board is blind to goals/shots/touches because that
signal lives only in the event stream and the events carry no pitch location
(`posX/posY`, `endX/endY`) or actor/target slots to draw with.

### 1.3 Versioning today

There is no view-facing contract version. The canonical encoder is at
`VERSION: u16 = 12` (`canonical.rs:243`) and a stale `session.rs` comment references
"encoder VERSION 7" — but that is the *canonical-state* version, a different number
from the *save-envelope* `SaveV-N` chain, and neither is the DTO/IPC contract
version. TS drift is caught structurally at runtime (`isMatchFrameDTOArray`) and at
compile time (the closed event union) — never by a negotiated version number.

### 1.4 The board itself

Minimal but correct. Two near-identical PixiJS v8 boards (production
`TacticalBoard.tsx`, ticker-LERP playback; dev `Dev/TacticalBoard.tsx`, frame-snap +
`window.fwDev.scrubTo` for headless screenshots). Both draw: boundary, halfway
line, centre circle/spot, two penalty boxes — and **no goal frames, no 6-yard box,
no penalty arcs, no corner/centre arcs**. Entities: 22 dots + 1 ball dot. The board
renders *none* of the richer signal already in the frame — possession is carried but
no dot is highlighted, velocity is carried but unused, ball `posZ` is flattened to a
fixed 4px dot. The dev board also inlines its own copies of the pitch constants
(drift risk vs `pitch-coords.ts`).

**Summary:** the seam is clean and the position stream is already richer than the
industry exchange formats. The genuine gaps are three: (a) no version envelope,
(b) no body orientation in canonical state, (c) no fusion of events onto the frame
timeline. Everything else the richer views want is *derivable downstream* and is a
renderer TODO, not a contract gap.

---

## 2. Recommended BUILD-TODAY changes (the important section)

The discipline, stated once: **add only the fields whose source of truth is the sim
and which a pure consumer cannot recover from what is already streamed.** The test
for "must be in the contract" is: *can a read-only consumer recover this from the
positions/events already sent?* If yes, it is the renderer's job and must NOT enter
the contract. If no, it has to come from the sim — and *that* is the thing that
forces a future sim change (and a canonical-hash re-pin) if skipped now.

By that test the must-add-today set is small, additive, and mostly already-present
data that is being dropped at the boundary.

### 2.1 The add-today set

| Add | Where | Why it must come from the sim (not derivable) | Sim risk |
|---|---|---|---|
| **`schemaVersion` envelope** — one per stream (`frameSchemaVersion`, `eventSchemaVersion`, `metaSchemaVersion`), distinct from canonical `VERSION` and `SaveV-N` | `dto.rs`, `result.rs`, live-match DTOs | A consumer cannot retrofit a version it was never told; adding it later is itself a breaking change. Per-stream because frames will churn far faster than the event kinds. | None — pure DTO |
| **Player `facing` / orientation** | canonical `PlayerState` → DTO | Body heading is sim truth; velocity direction is a poor proxy (a shielding/back-to-goal player has velocity perpendicular to facing). 2.5D/3D cannot fake it; FM26 made "on the half-turn" its headline. | **Real** — new canonical field + hash re-pin. This is the one decision that is expensive to defer. |
| **Ball `spinX/Y/Z` projection** | DTO only (canonical already has it) | The only believable-curl input 3D needs; it already exists canonically. This is purely un-dropping it at the boundary. | None — already canonical |
| **Event `primarySlot`/`secondarySlot`, `posX/posY`, `endX/endY`, `outcome`** | `result.rs` | The sim knows actor/target/locations at the instant of the event; reconstructing "who passed to whom from where" by scanning positions around the tick is exactly the lossy "lower bound" guess `inspect_frames.rs` already flags. This is what fuses the two streams. | Low — projection of data the sim already holds |
| **`phase` enum** on frames (in-possession / out / transition / dead-ball) | canonical → DTO | Already drives BT selection internally; analytics (xT/EPV) and a phase-aware renderer want it emitted, not re-derived. | Low — projection of internal truth |
| **`period` field** on frames + events | DTO | Trivial now, structurally required once stoppage/ET/shootout exist. | None |

### 2.2 Reserve-but-don't-fill (declare now, populate later non-breakingly)

Declare these fields/enum variants now so the *first time data lands in them* is a
value change a tolerant consumer ignores, not a schema break: player `actionState`
(idle/dribbling/passing/shooting/tackling/recovering — lets a future 2.5D/3D viewer
pick a clip without re-deriving intent), player `accel`, event `subType`
(header/through-ball/switch), ball `spin` precision. Do not build the producers
yet; just hold the slots.

### 2.3 Make the projection renderer-neutral (the lock-in avoider)

Treat the **canonical→frame projection as the real contract, with the TS DTO as one
downstream serialization of it.** Concretely: keep the projection a stable, named,
versioned Rust type in a sim-adjacent location, so that a future native renderer
reads the same versioned frame *in Rust* — before the f64/camelCase/JSON step —
instead of re-deriving the projection or scraping JSON. With exactly one consumer
today, this migration is free; with three it is not.

### 2.4 Versioning rules (port from protobuf/schema-evolution practice)

1. **Per-stream version, not one global number.** Frames churn; event kinds are
   stable. Version them on separate clocks (the CDF standard does exactly this).
2. **Additive-only at the boundary; tolerate-unknown on the consumer.** New fields
   are always optional/defaulted; existing fields are never removed or retyped
   (retyping/meaning-change is the break-everything mistake). The TS frame parser
   should consume any `frameSchemaVersion` ≤ what it understands, ignoring unknown
   fields — i.e. *forward* compat, the mode you actually want since the sim leads
   and views lag. Note the deliberate asymmetry: keep the **event** union
   compile-breaking on a new kind (you *want* to be forced to handle it), but make
   the **frame** stream tolerant and never compile-breaking on a new field.
3. **A `features: [...]` capability list in the static metadata header beats version
   arithmetic.** A renderer checks `features.contains("ball_spin")` and degrades
   gracefully if absent, instead of branching on `if version >= 5`. Lets a
   partially-shipped future field strand no consumer, and lets `dump_frames`
   fixtures self-describe to the screenshot harness.
4. **Golden fixtures are the regression net.** A committed golden frame per
   `frameSchemaVersion` + the existing JSON round-trip test trips on any field
   reorder/retype the way the canonical-hash test trips on sim drift (serde field
   order is load-bearing for Bincode bytes).

### 2.5 Explicitly NOT in the add-today set

Trails, motion-vector lengths, ball shadows, elevation cues, possession-carrier
highlight, ball zone, smoothed interpolation, depth sorting, camera. All derivable
from positions/events already streamed. These are renderer work (§3, §5) and need
zero sim or contract change.

One structural note, not a contract change: `match_frames`/`play_match` re-run the
sim from seed per call rather than reading a committed snapshot. For a deterministic
sim this *is* the correct replay primitive (store seed, recompute), not debt. The
only later refinement is periodic keyframe snapshots so seeking is O(N from nearest
keyframe). The live match should move to the Tauri **Channel API** (stream each tick,
project once) rather than re-running per command — but the contract above already
supports both without change.

---

## 3. The 2D → 2.5D → (maybe) 3D progression — one contract, sim untouched

Every step below reads the *same* frame+event contract from §2. The sim is never
touched again after §2.1 lands. Each step is pure additive render work on the
existing PixiJS board, ordered by felt-impact-per-effort.

### 3.1 Rich 2D (the cheap "oomph" slice — ~2-5 days, no contract change)

Reads only fields already carried. Build order by feel-per-effort:

1. **Possession highlight + carrier→ball tether** (~1-2h). Ring/brighten the
   carrier dot from `possession`; neutral "loose ball" state on null. Cheapest
   line-item, ties for most impact — "who's on the ball" is the first question a
   viewer asks, and nothing renders it today.
2. **Ball height: lift + ground shadow** (~half day). Draw the ball dot at
   `screenY - posZ * K` and a ground-shadow ellipse at the true `(x, y)` that
   shrinks+fades as `posZ` grows. The gap between shadow and ball *is* the height
   read. This is the canonical top-down 2.5D trick and is most of the perceived
   oomph — crosses, lofted through-balls and clearances suddenly read as airborne
   instead of teleporting flat. Uses the `posZ` already projected and currently
   thrown away.
3. **Depth sorting** (~half day). PixiJS v8 Render Layers or `zIndex = screenY` so a
   grounded ball sits under feet while a lofted ball passes over players. Trivial at
   23 objects; makes the lift in (2) look correct.
4. **Sprite billboards + velocity-facing** (~1 day). Replace dots with kit chips /
   shirt-number chips / a small facing triangle oriented by `velX/velY`. The jump
   from "dots" to "players." Velocity-facing is the honest cheap stand-in until/if
   true `facing` (§2.1) lands.
5. **Camera (pan/zoom/follow-the-ball)** (~half-1 day). Native PixiJS v8 render
   groups for a fixed tactical zoom (zero deps); reach for `pixi-viewport` only if
   full drag/pinch/follow is wanted. Don't add the dep speculatively.
6. **Finish the pitch furniture** (~few hours). Goal frames, 6-yard box, penalty
   arcs, corner/centre arcs in `pitch-coords.ts`; also kill the dev/prod constant
   duplication. Goal frames matter once (2) lands — a lofted ball dropping toward an
   actual goal mouth reads as a chance.

Items 1-3 alone are a ~2-day slice that already "shows what's happening." This is
the highest-ROI visual work available and needs **none** of §2's sim changes — only
the dropped-but-carried fields.

### 3.2 2.5D (tilted/perspective board — optional, Three.js layer)

A perspective-tilted pitch with billboarded players over a 3D ground plane, still
reading the same frame contract. This is the step where player `facing` (§2.1) and
`actionState` (§2.2 reserved slot) start to pay off — a tilted view makes the lack of
body orientation visible in a way the flat board hides. Three.js reserved for this;
PixiJS and Three can co-exist (2D HUD over a 3D pitch). Still a puppet show:
lerp/slerp between ticks, never extrapolate past the latest frame.

### 3.3 3D (full match-day viewer — only if §6 opens the lane)

Mesh players with clips driven by `actionState`, ball curl from `spinX/Y/Z`,
free camera. Consumes the full §2 field set + reserved slots. This is the only step
that might exceed the in-webview ceiling on the weakest platform (§4) and the only
one that touches the product-vision fork (§6). Explicitly **not** PixiJS
`PerspectiveMesh`/`pixi3d` — those are a rewrite for 80%-less felt depth than the
fake-Z approach, and violate the text-first direction unless §6 changes it.

The point: 2D-rich and 2.5D are reachable today on the committed stack reading one
contract; only 3D forces both a renderer-architecture question (§4) and a
product-vision question (§6).

---

## 4. Hybrid-rendering feasibility verdict

**Verdict: stay in the webview. Default renderer PixiJS v8 on the WebGL floor,
WebGPU as opportunistic upgrade. Treat a native wgpu/Bevy match surface as
research-only, not a near-term build.** The deciding factor is not graphics power —
23 entities are nowhere near any engine's envelope — it is the cross-platform
fragility of embedding a native GPU surface in a Tauri window in 2026.

### 4.1 Determinism is preserved either way — it is orthogonal to the renderer

The puppet-show invariant ("the match view reads sim frames and never integrates
physics") is an architectural contract, not a property a renderer grants or removes.
No candidate changes it: PixiJS/Three/Babylon in the webview only ever see the f64
projection and *cannot* simulate canonical state. A native Bevy surface is *more*
dangerous, not less — its whole reason to exist (ECS, transform hierarchy,
`bevy_rapier`/`avian`) tempts an accidental second physics layer the moment a system
integrates velocity to smooth between ticks. It is keepable-deterministic only by
deliberately not using the engine. The hard rule that keeps any renderer honest:
*consume the frame stream as sole truth; interpolation is display-only, bounded,
never extrapolated past the latest received frame; no renderer-side physics.* For a
deterministic puppet show fed from local IPC you need no dead-reckoning at all.

### 4.2 Why the webview (option a)

PixiJS v8 is already in the tree, mature, WebGPU-with-WebGL-fallback, best-in-class
2D batching. A football match is 23 sprites + overlays — trivially inside the WebGL
floor even on the weakest platform. Three.js is the reserved 2.5D upgrade;
Babylon.js only enters if full 3D is committed. The catch: **WebGPU in the webview
is platform-fractured.** Tauri uses the *system* webview — WebView2/Chromium
(Windows, self-updating, most reliable), WKWebView (macOS, WebGPU by default since
Safari 26), WebKitGTK (Linux, the weak link, WebGPU not assumable across
distro-variable versions). **And Linux is Steam Deck.** So: never *depend* on webview
WebGPU; WebGL2 is the real floor and is the ceiling on the platform you most want to
wow. IPC bandwidth is a non-problem (a few hundred bytes/frame); the live match
should stream via the Channel API rather than recompute per call.

### 4.3 Why not a native wgpu/Bevy surface (option b), yet

Two ways, both rough in 2026. (1) Embed a wgpu surface in the *same* Tauri window
(raw-window-handle): the webview and wgpu context fight for the surface and flicker —
a known, repeatedly-reported failure (tauri#9220, closed not-planned). The one
documented crack (May 2026, `tauri-plugin-steam-overlay`) is **macOS-only**. (2) A
separate native window/sidecar: sidesteps the surface fight but breaks "the player
barely notices the switch" — a second OS window with its own decorations, focus,
z-order, fullscreen and DPI to reconcile, plus a second renderer's binary and IPC
channel. Bevy itself is production-credible (0.18, Tiny Glade) but taxes 1-3 days of
migration every 3-4 months until an undated 1.0 — a large standing cost for one
screen.

### 4.4 Seamless switching — what "swap" can and cannot mean

**Yes**, if "swap" means swapping the *renderer component behind a stable, versioned
frame contract, inside the one webview*: a SolidJS match route owns a `<canvas>` +
`FrameSource`; Pixi today, a Three.js 2.5D board tomorrow, chosen by setting or
capability check. Same route, same DOM, same window, same IPC — genuinely seamless.
**No**, if "swap" means hot-switching between a webview board and a *native OS
window* — the window-management/focus/compositing seams are exactly what the Tauri
ecosystem has not solved portably in 2026.

### 4.5 The escape hatch (additive, not a rewrite)

If 3D is ever committed (§6) and the webview ceiling is genuinely hit on a target,
the right hatch is **a separate native wgpu match window fed by the same canonical
projection** — management UI stays SolidJS, only the match view goes native. wgpu is
pure-Rust, reads the §2.3 renderer-neutral projection directly with no JSON hop, and
is the GPU layer inside Firefox/Servo/Deno (battle-tested). This is structurally how
FM separates its match engine from its viewer. It is purely additive *provided* §2.3
(renderer-neutral versioned projection) and the §2.1 orientation decision are done
now. The compositing-in-one-window variant is rough; the separate-window variant is
mundane engineering. Re-evaluate when a portable Tauri compositing primitive exists.

---

## 5. Game-FEEL leverage list (presentation-only, ordered, cheap-now first)

None of this reads, drives, or simulates canonical state — the puppet-show
constraint holds throughout. The cross-cutting insight: the things that scream
"lightweight browser app" and the things that scream "generic AI frontend" are the
*same defaults* (native title bar, Inter, indigo accent, soft 0.1-opacity card
shadows, instant state swaps). Killing the browser tells and escaping the AI look is
largely one body of work.

| # | Lever | Effort | Touches sim? | Felt impact |
|---|---|---|---|---|
| 1 | **Borderless window + custom chrome.** macOS `titleBarStyle: "Overlay"` + `trafficLightPosition` (keeps native traffic lights/snapping, avoids the `decorations:false` resize bug #8519); Windows `decorations:false` + `data-tauri-drag-region`. A custom header strip carrying club crest, save name, in-world date and a persistent Continue action — the FM/Paradox signature, chrome as part of the game. | Low | No | Highest "this is an app, not Chrome" signal |
| 2 | **Visual identity — escape the generic-AI look.** Drop Inter for a high-contrast pairing (condensed/broadcast-graphic numerals for tables/scoreboards + an editorial face for prose), extreme weight contrast, `tabular-nums` non-negotiable. A committed non-purple palette anchored to football's own visual history (matchday-programme newsprint, classic broadcast graphics); drive via CSS vars and let each procedural club tint its own chrome — an identity moat a generic app can't fake. Density over whitespace; crisp 1px rules over soft shadows. | Low | No | Highest aesthetic ROI |
| 3 | **Motion language.** `solid-motionone` (~5.8KB, Solid-native; not React-coupled Framer). Three durations (~120/220/400ms), two easings (ease-out for entrances, a spring for physical things). Continue/advance-time *feels* like time passing; numbers animate (score ticks, table positions slide). Respect `prefers-reduced-motion`. Do NOT JS-hijack native scroll — adds jank, reads as try-hard. | Low-Med | No | "Has mass" vs "snappy webpage" |
| 4 | **Cohesive UI sound kit.** One shared `AudioContext`, gesture-gated `resume()`. A *family* of sounds (soft tick for toggles, gentle confirm for commits, low whoosh for transitions) — NOT sound on hover/scroll/every click. Off-by-default + a real mute in the header (#1). FM ships *no* UI sounds at all — a small tasteful kit out-classes the market leader for almost nothing. Match-day crowd ambience keyed to the event stream is a separate later tier. | Low | No | Disproportionate weight-per-byte |
| 5 | **Boot ceremony.** A Tauri splash window that *masks the real load* (content-pack load, save deserialize), dismissed on a ready event not a timer. One strong brand frame + one motion beat + the boot sting; a flavor loading line ("Compiling the lower leagues…") that reinforces pillar 1. Lower because it's seen once per session. | Low | No | Sets the frame |

Two explicit don'ts: don't JS-hijack scroll, and don't reach for the
indigo/Inter/soft-card defaults the toolchain hands you — that is the exact
generic-AI signature to escape.

---

## 6. PRODUCT-VISION FORK (owner decision)

> **This is the owner's call, not auto-adopted.** Per delegation discipline this is
> a product-vision fork, not an implementation detail — it changes a pillar-level
> contract (`docs/DESIGN_DOC.md` rules out 3D and commits to text-first/2D). It must
> go through `/log-decision` before any code or doc reflects a change. The §2
> contract-hygiene set below is recommended *regardless of which fork is chosen* —
> it is cheap, additive, and the thing that makes the fork reversible later.

The two forks:

### Fork A — Keep text-first / 2D-board as the locked contract

**What it means:** the match view stays a 2D tactical board forever. We invest the
visual budget in the §3.1 rich-2D slice (possession, ball height, depth, billboards,
camera, full pitch) and the §5 game-feel work. 3D is explicitly off the table; the
`DESIGN_DOC.md` ruling stands unchanged.

**Cost:** none beyond what's already planned. **Benefit:** maximal focus on the
pillars (procedural world, memory, breakthroughs, scouting, signature identity) that
are the actual product moat; zero exposure to the cross-platform native-render
problem; the cheapest path to a board that already "shows what's happening."
**Risk:** if the market later rewards a richer match-day, retrofitting orientation
into canonical state is a sim change + hash re-pin (mitigated entirely by doing §2.1
now).

### Fork B — Deliberately open a richer-visual lane (2.5D, optionally 3D)

**What it means:** commit to 2.5D now and reserve 3D as a real (not theoretical)
future, on the same one contract. The match view becomes a headline feature, not a
schematic.

**Cost:** sustained renderer investment (Three.js 2.5D layer; if 3D, art/animation
pipeline + the §4.5 native-window escape hatch on the weakest platform); a pillar-doc
amendment; ongoing maintenance of a richer surface. **Benefit:** a differentiated
match-day that competes on feel with FM's viewer; the procedural-world identity
rendered, not just described. **Risk:** scope gravity — a 3D match-day is a
content-and-engineering sink that can starve the pillars; the in-webview ceiling on
Linux/Steam Deck (§4.2) makes 3D-specifically a platform-gated bet.

### Recommendation

**Adopt the §2 contract-hygiene set now (it serves both forks), build the §3.1
rich-2D slice + §5 game-feel, and default to Fork A — text-first/2D as the locked
contract — while keeping Fork B genuinely reachable rather than committed.** The
reasoning: the rich-2D slice + game-feel work delivers almost all the felt "oomph"
for a fraction of Fork B's cost and zero platform risk, and §2 (especially the
orientation decision in §2.1 and the renderer-neutral versioned projection in §2.3)
makes Fork B an *additive* move later rather than a rewrite. The one decision that is
expensive to defer is whether player **body orientation** enters canonical state now;
doing it now costs one field + one hash re-pin, doing it after Fork B is chosen is a
larger contract renegotiation. Recommend deciding *that field* in the owner's favour
(add it) even under Fork A, precisely to keep Fork B cheap.

The fork itself — A vs B — is the owner's to make, and this doc exists to make that
call from a grounded position, not to make it for them.

---

*Authored as exploratory research. No DECISIONS.md entry, no DESIGN_DOC.md change, no
code change is implied by this doc until the owner rules on §6 and (if §2 is adopted)
a task is drafted via `/next`.*

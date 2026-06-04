> Cross-refs: `docs/design/gene-attribute-compiler.md` (real attribute spread that makes the identity descriptor meaningful), `docs/design/football-fidelity-audit.md`, the scouting types in `crates/fw-scouting`. Owners: systems-designer (classifier + gating) + narrative-director (verdict/tier Tracery) + ui-programmer (the 3 surfaces). §8 open questions await owner sign-off.

# Player Stats Presentation — Design Spec

**Target file:** `docs/design/player-stats-presentation.md`
**Status:** Proposed (Phase-N tuning values — not in SPEC or DECISIONS)
**Authored:** 2026-06-04

---

## 1. Overview and surface map

Three surfaces, one coherent data contract. Every element on every surface is read from a single `PlayerAssessmentDTO`. The DTO is built in `fw-tauri` from canonical types; the UI never touches canonical state.

| Surface | Primary voice | When visible |
|---|---|---|
| Squad table row | Star summary (filled/hollow) + identity chip | Always — every player in any list |
| Player detail screen | Verdict bullets + tier-words per group + radar silhouette | Click through from squad row |
| Power-user toggle | 1–20 numeric ranges per group, tightening with observation | User opt-in, default OFF |

The knowledge state — `UncertaintyBand` derived from the merged `GeneCategoryEstimate` bands across all scout reports — governs exactly how sharp or fuzzy every surface renders. Sharp and fuzzy are not cosmetic modes; they emerge from the same DTO fields read at different resolutions.

---

## 2. Identity descriptor — deterministic classification

> **Superseded (2026-06-04):** the authoritative descriptor set now lives in `docs/design/player-identity-descriptors.md` — 16 football-fan playing identities + 5 trait tags, with the precedence / quality-tier / identity-vs-role refinements. That doc SUPERSEDES the placeholder five enumerated in §2.1 below; the §2 classification mechanics (deterministic, DTO-build-time, knowledge-gated) still hold, but the descriptor list and its rules are the player-identity-descriptors.md set.

### 2.1 Five descriptors

| Descriptor | Player-facing label |
|---|---|
| `TechnicalArtist` | "technical artist" |
| `PhysicalSpecimen` | "physical specimen" |
| `MentalityMonster` | "mentality monster" |
| `LeftFootedWand` | "left-footed wand" |
| `LateBloomer` | "late bloomer" |

These are internal classification tokens. The UI surfaces the label string, never the enum name.

### 2.2 Input quantities

The classifier operates on three group means, gene modifier flags, and one `NarrativeFlag`. All computed from the canonical `PlayerAttributes` and `GeneSnapshot`. Computation runs at DTO build time in `fw-tauri`, deterministically, with no random draw.

**Group means** (Q32, all in [0, 1], derived from canonical attributes):

```
technical_mean = arithmetic mean of 14 TechnicalAttributes fields
mental_mean    = arithmetic mean of 10 MentalAttributes fields
physical_mean  = arithmetic mean of 8 PhysicalAttributes fields
```

The goalkeeper group is excluded from identity classification — it would swamp outfielders' reading.

**Gene modifier flags** (boolean, derived from GeneSnapshot thresholds):

```
is_left_dominant  = technical.left_foot > 0.65 (Q32 raw > 2_793_155_430)
is_physical_frame = physical.frame_density > 0.68 OR physical.fast_twitch_ratio > 0.72
                    i.e. frame_density.to_bits() > 2_920_577_761 OR
                         fast_twitch_ratio.to_bits() > 3_092_376_533
is_late_bloomer   = narrative_flags.contains(NarrativeFlag::LateBloomer)
                    OR physical.growth_curve > 0.40 (signed field; > 0.40 means later-than-average peak)
                    i.e. growth_curve.to_bits() > 1_717_986_918
```

### 2.3 Classification rules (strict priority order)

Evaluated top to bottom; first match wins. BTreeMap ordering is irrelevant here because the rules are sequential `if/else if`, not a map lookup — the order is the invariant.

**Rule 1 — LateBloomer** (highest narrative salience, checked first)

```
if is_late_bloomer:
    descriptor = LateBloomer
```

A player with `NarrativeFlag::LateBloomer` or a `growth_curve > 0.40` is always labelled late bloomer regardless of their current group means. The label refers to a career arc, not current quality, so it wins over all quality-based descriptors. This also prevents a mediocre-but-late-peak player from being called a "mentality monster" simply because their technical and physical means are both mid.

**Rule 2 — LeftFootedWand** (technical with clear foot dominance)

```
if is_left_dominant AND technical_mean > 0.60:
    descriptor = LeftFootedWand
```

`left_dominant` alone is not enough — a left-footed but technically limited player is a physical specimen or mentality monster, not a wand. The `technical_mean > 0.60` threshold (Q32 raw > 2_576_980_378, which is also the Tentative/Confident threshold boundary — intentionally aligned so the descriptor only fires once scouting finds something worth remarking on) keeps this accurate.

Worked example A: `left_foot = 0.78, technical_mean = 0.71` → `is_left_dominant` = true, `technical_mean > 0.60` = true → **LeftFootedWand**. Correct — this is the Marcos Vidal archetype.

Worked example B: `left_foot = 0.71, technical_mean = 0.44` → `is_left_dominant` = true but `technical_mean < 0.60` → falls through to next rule.

**Rule 3 — PhysicalSpecimen**

```
if is_physical_frame AND physical_mean > mental_mean AND physical_mean > technical_mean:
    descriptor = PhysicalSpecimen
```

The `is_physical_frame` gene flag (high `frame_density` or high `fast_twitch_ratio`) ensures the label is grounded in physical gene architecture, not just temporarily high physical attributes from youth. The dominance check (`physical_mean` beats both other groups) prevents mislabelling a technically-dominant player who also happens to be fit.

Worked example C: `frame_density = 0.74, fast_twitch_ratio = 0.61, physical_mean = 0.73, technical_mean = 0.51, mental_mean = 0.48` → `is_physical_frame` = true (frame_density > 0.68), `physical_mean` is the highest → **PhysicalSpecimen**.

Worked example D: `frame_density = 0.74, physical_mean = 0.68, technical_mean = 0.71` → `is_physical_frame` = true but `physical_mean < technical_mean` → falls through.

**Rule 4 — MentalityMonster**

```
if mental_mean > physical_mean AND mental_mean > technical_mean AND mental_mean > 0.63:
    descriptor = MentalityMonster
```

The absolute floor (0.63, Q32 raw ≈ 2_706_956_661) prevents the label from firing on a squad of mediocre players who just happen to have mental as their least-bad group. A "mentality monster" should be visibly strong in the mental group, not merely least-weak.

**Rule 5 — TechnicalArtist** (default for technically-dominant players without foot gene)

```
if technical_mean > mental_mean AND technical_mean > physical_mean:
    descriptor = TechnicalArtist
```

**Rule 6 — fallback**

```
else:
    descriptor = TechnicalArtist
```

A broadly balanced player (no dominant group, no special gene) defaults to TechnicalArtist. This is the safest fallback because technical skill is the most observable quality and the most readable identity at squad-table level. A genuinely balanced midfielder is better described as "technical artist" than having no identity at all.

### 2.4 Gene-to-descriptor influence summary

| Gene field | Effect |
|---|---|
| `growth_curve > 0.40` | Forces LateBloomer regardless of means |
| `NarrativeFlag::LateBloomer` | Forces LateBloomer |
| `left_foot > 0.65` | Unlocks LeftFootedWand if technical_mean also high |
| `frame_density > 0.68` or `fast_twitch_ratio > 0.72` | Unlocks PhysicalSpecimen if physical dominates |
| `height_ceiling` | Does not directly tip descriptor (affects radar shape) |
| `dead_ball` | Does not tip descriptor (surfaces in verdict bullets) |

### 2.5 Knowledge-gating of the identity chip

The identity chip is shown on the squad table row and at the top of the detail screen. Its display depends on `UncertaintyBand`:

| Band | Squad row chip | Detail screen chip |
|---|---|---|
| Hunch | Hidden entirely | `"getting a read on him"` (grey, italic) |
| Tentative | `"probably [descriptor]?"` | Descriptor label + question mark |
| Confident | Descriptor label | Descriptor label |
| Settled | Descriptor label | Descriptor label |

The underlying computation always runs — the chip is suppressed or hedged, not swapped for a different answer.

---

## 3. Knowledge-gating map

One coherent rule set. All five display surfaces derive their fog level from the same `UncertaintyBand` value in the DTO.

The DTO carries a single `band: UncertaintyBand` which represents the **worst-band across the three `GeneCategoryEstimate`s** in the aggregated scout picture. The aggregated estimates themselves come from merging all `ScoutReport`s on file for this player (the merge strategy is the `fw-scouting` crate's responsibility at the aggregation step; the DTO builder reads whatever merged estimate is current). Using the worst band — not the mean — means a player where physical is Settled but mental is Hunch still reads as Hunch-tier. This is correct for Pillar 4: uncertainty is not averaged away.

### 3.1 Star fill/hollow split (squad table row)

Stars represent the aggregated `AbilityCeiling.current()` (for filled stars) and the gap toward `AbilityCeiling.potential()` (for hollow/reach stars). Both are projected from Q32 [0, 1] → 0..5 star scale at the DTO boundary (multiply by 5, round to nearest half, clamp to [0, 5]).

The filled/hollow split additionally narrows based on band:

| Band | Filled star floor | Hollow star reach | Split rule |
|---|---|---|---|
| Hunch | `floor(ca_stars - 2.0)` clamped ≥ 0 | `ceil(pa_stars + 1.0)` clamped ≤ 5 | Wide possible range; very few filled |
| Tentative | `floor(ca_stars - 1.0)` clamped ≥ 0 | `ceil(pa_stars + 0.5)` clamped ≤ 5 | Moderate uncertainty |
| Confident | `round(ca_stars)` | `round(pa_stars)` | DTO values as-is |
| Settled | `round(ca_stars)` | `round(pa_stars)` | DTO values as-is; no additional spread |

Example: Marcos Vidal, CA ≈ 0.78, PA ≈ 0.85.
- `ca_stars = round(0.78 × 5) = 4`, `pa_stars = round(0.85 × 5) = 4`
- Settled: 4 filled, 4 reach (identical — a settled read of 4-star player)
- Tentative: floor(4−1)=3 filled, ceil(4+0.5)=5 reach → 3 filled + 2 hollow
- Hunch: floor(4−2)=2 filled, ceil(4+1)=5 reach → 2 filled + 3 hollow

A player who solidifies from Hunch to Settled visibly gains filled stars and loses hollow ones. That is the mechanic the owner asked for.

### 3.2 Tier-word width (detail screen group labels)

Each attribute group (Technical / Mental / Physical) is rendered as a tier-word. The tier-word is derived from the `GeneCategoryEstimate.low` and `.high` midpoint, projected to a 5-tier scale:

| Q32 midpoint range | Single-tier word | Adjacent-tier word (if high straddles) |
|---|---|---|
| [0.00, 0.25) | "poor" | "poor to average" |
| [0.25, 0.45) | "average" | "average to solid" |
| [0.45, 0.62) | "solid" | "solid to good" |
| [0.62, 0.80) | "good" | "good to excellent" |
| [0.80, 1.00] | "excellent" | — (ceiling) |

The tier-word displayed to the player is determined by how many tiers the band straddles:

| Band | Display rule |
|---|---|
| Hunch | Full phrase "poor to excellent" if high-low > 0.50; otherwise adjacent-tier phrasing |
| Tentative | Adjacent-tier phrasing always |
| Confident | Single-tier word if band is tight (high-low < 0.18); adjacent-tier otherwise |
| Settled | Single-tier word (band is ≤ 0.24 by construction given BASIC_SCOUT_BAND_HALF_WIDTH = 0.12) |

The band half-width of 0.12 at the current tuning seed means a Settled single-observation band spans 0.24 at most. Across multiple observations the band narrows further (aggregation logic in `fw-scouting`). At 0.24 width it spans at most two adjacent tiers, so even Settled reads occasionally get an adjacent qualifier — which is correct. "Solid to good" is more honest than "solid" for a player at the tier boundary.

### 3.3 Verdict bullet count and hedging

The detail screen leads with verdict bullets. Their count and language hedge per band:

| Band | Strength bullets | Weakness bullets | Section header | Language style |
|---|---|---|---|---|
| Hunch | 1–2 max | 1 max | "First impressions" / "Too soon to say" | Hedged: "looks quick", "probably physical" |
| Tentative | 2–3 | 2 | "Early signs" / "Concerns" | Partial confidence: "shows good touch", "tends to struggle" |
| Confident | 3–4 | 2–3 | "Strengths" / "Weaknesses" | Assertive: "excellent range of passing" |
| Settled | 4–5 | 2–4 | "Strengths" / "Weaknesses" | Full verdict: "reads space a step ahead of most at this level" |

The bullet text comes from Tracery templates (see Section 6 — narrative-director deliverables). The triggers for which bullets appear are deterministic thresholds on the group means and gene flags; the template selects the prose. A bullet does not appear if the evidence threshold for its template key is not met. Sparse panels for Hunch-tier players are the correct result — the player gets 2 bullets because there are only 2 bullets worth of evidence, not because we arbitrarily capped them.

### 3.4 Radar dot vs. shape (detail screen)

The radar has 5 axes: Technical, Mental, Physical, Goalkeeping (collapsed for outfielders), and a fifth context-dependent axis (Stamina/Engine for outfielders; Reflexes for keepers). Each axis value is the `GeneCategoryEstimate` midpoint scaled 0..1, rendered as a fraction of the maximum axis length.

| Band | Rendering |
|---|---|
| Hunch | Dots only — axis points rendered as disconnected dots, no connecting lines. Axis opacity 0.4. |
| Tentative | Dashed partial polygon — lines connect but with dashes; the gap between dash endpoints signals uncertainty. |
| Confident | Solid polygon, thin stroke, semi-transparent fill. |
| Settled | Solid polygon, full opacity, defined fill colour per descriptor archetype. |

An unscouted player's radar is a ring of faint dots. A watched regular's radar is a solid shape with a readable silhouette. The progressive solidification from dots to shape is the mechanic; it requires no additional computation — the same axis values are rendered differently based on the single `band` field in the DTO.

Radar shape colour by identity descriptor (for Confident/Settled only):
- TechnicalArtist: slate blue
- PhysicalSpecimen: burnt orange
- MentalityMonster: deep green
- LeftFootedWand: slate blue + left-foot accent mark on the Technical axis
- LateBloomer: amber (signals trajectory, not current peak)

These are design intent for the UI programmer; exact hex values live in the Tailwind design token file, not here.

### 3.5 Numeric range width (power-user toggle)

The 1–20 range is derived from `GeneCategoryEstimate.low` and `.high`, projected linearly: `round(q32_value × 20)`, clamped to [1, 20].

Per-group displayed range: `low_int–high_int`. Example: `low = 0.70, high = 0.82` → `14–17`.

The toggle is OFF by default. When ON it replaces the tier-word for each group. It never shows individual attribute values, only the group-level range (matching the `GeneCategoryEstimate` granularity).

Additional gating by band:

| Band | Numeric display |
|---|---|
| Hunch | Group level only; individual attribute rows show "?" |
| Tentative | Group level; individual attributes show "?" |
| Confident | Group level shown; individual attributes show their own narrowed range if available |
| Settled | Full expansion; narrowest ranges |

The range never collapses to a single number. The minimum display width is `N–N+1` (two adjacent integers). A range of `N–N` is treated as `N–N+1` at display time. This preserves the design rule: no false precision. The player sees we have a very confident view, not an omniscient one.

---

## 4. PlayerAssessmentDTO

Built in `fw-tauri` from canonical state. Read-only projection. `f64` is used freely (per `Tauri/RULES.md §3`). `#[serde(rename_all = "camelCase")]` on all fields.

```rust
/// Read-only projection for the player stats panel (squad row + detail screen + toggle).
/// Built deterministically from PlayerAttributes, GeneSnapshot, aggregated ScoutReports,
/// and AbilityCeiling. Never serialized back into canonical state.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerAssessmentDTO {
    // --- Identity ---
    pub player_id: u32,                       // PlayerId.raw()
    pub display_name_short: String,
    pub role_label: String,                   // from PlayerBio.role_family display_label

    // --- Knowledge state ---
    /// Worst-band across the three GeneCategoryEstimates. Drives all fog rendering.
    pub band: UncertaintyBandDTO,

    // --- Star summary (squad row) ---
    /// Filled star count [0.0, 5.0] in 0.5 steps. Band-adjusted floor.
    pub stars_filled: f64,
    /// Hollow (reach) star count [0.0, 5.0] in 0.5 steps. Band-adjusted ceiling.
    pub stars_reach: f64,

    // --- Identity chip ---
    /// null = hidden (Hunch-tier). Otherwise the hedged or firm descriptor string.
    pub identity_chip: Option<String>,
    /// Internal classifier token for colour/icon selection.
    pub identity_descriptor: Option<IdentityDescriptorDTO>,

    // --- Tier-words per group (detail screen) ---
    pub tier_technical: Option<TierWordDTO>,  // None = Hunch with no evidence
    pub tier_mental: Option<TierWordDTO>,
    pub tier_physical: Option<TierWordDTO>,

    // --- Verdict bullets (detail screen) ---
    /// Section heading for strength bullets (changes per band).
    pub strengths_heading: String,
    /// Strength bullet strings. Tracery-rendered, band-count-gated.
    pub strength_bullets: Vec<String>,
    /// Section heading for weakness bullets.
    pub weaknesses_heading: String,
    /// Weakness bullet strings. Tracery-rendered, band-count-gated.
    pub weakness_bullets: Vec<String>,

    // --- Radar (detail screen) ---
    pub radar: RadarDTO,

    // --- Numeric ranges (power-user toggle, only populated when requested) ---
    /// None when toggle is OFF. Populated on explicit request to avoid DTO bloat.
    pub numeric_ranges: Option<NumericRangesDTO>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum UncertaintyBandDTO {
    Hunch,
    Tentative,
    Confident,
    Settled,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum IdentityDescriptorDTO {
    TechnicalArtist,
    PhysicalSpecimen,
    MentalityMonster,
    LeftFootedWand,
    LateBloomer,
}

/// A tier-word for one attribute group.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TierWordDTO {
    /// e.g. "solid", "solid to good", "poor to excellent"
    pub label: String,
    /// Midpoint of the estimate [0.0, 1.0] for bar fill or radar scaling.
    pub midpoint: f64,
    /// Width of the uncertainty band [0.0, 1.0].
    pub band_width: f64,
}

/// Radar axis values for the 5-axis silhouette.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RadarDTO {
    pub technical: f64,    // GeneCategoryEstimate midpoint, [0, 1]
    pub mental: f64,
    pub physical: f64,
    /// Rendered at full opacity only for goalkeepers; collapsed to 0 for outfielders.
    pub goalkeeping: f64,
    /// "Stamina / Engine" for outfielders (physical.stamina midpoint),
    /// "Reflexes" for keepers (goalkeeper.reflexes midpoint).
    pub context_axis: f64,
    pub context_axis_label: String,
    /// Drives rendering mode (dots / dashed / solid / coloured).
    pub rendering_mode: RadarRenderingModeDTO,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RadarRenderingModeDTO {
    Dots,
    DashedPartial,
    SolidThin,
    SolidFull,
}

/// 1–20 integer ranges per group. Only sent when toggle is ON.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NumericRangesDTO {
    pub technical_low: u8,   // [1, 20]
    pub technical_high: u8,
    pub mental_low: u8,
    pub mental_high: u8,
    pub physical_low: u8,
    pub physical_high: u8,
}
```

### 4.1 DTO construction responsibilities

The Tauri command `get_player_assessment(player_id: u32, include_ranges: bool) -> Result<PlayerAssessmentDTO, IpcError>` builds the DTO as follows:

1. Load `PlayerAttributes`, `AbilityCeiling`, `GeneSnapshot`, and the current aggregated `ScoutReport` set for the player.
2. Compute group means (arithmetic mean of each attribute group's Q32 fields).
3. Classify identity descriptor (Section 2 rules — pure function, no RNG).
4. Derive `UncertaintyBand` from the worst `GeneCategoryEstimate.band()` across the three categories.
5. Apply band-gating rules (Section 3) to produce stars, chip, tier-words, radar values, and rendering mode.
6. Select Tracery template keys and bullet count (Section 6); render bullets via the Tracery engine (sync call, no LLM).
7. Project Q32 → f64 for all DTO numeric fields. Project to 1–20 integer ranges only if `include_ranges = true`.

Step 3 is purely `if/else if` over Q32 comparisons — no allocations, no random draws.

### 4.2 What narrative-director owes

The DTO's `strength_bullets` and `weakness_bullets` are rendered strings. The content pipeline for them is:

- Template key selection: deterministic, driven by which attribute thresholds are met and which phenotype labels are confirmed above the band's minimum confidence (see Section 3.3). This is a table lookup, not prose generation.
- Prose output: Tracery templates authored by narrative-director, keyed by a `BulletKey` enum (e.g. `BulletKey::HighTechnical`, `BulletKey::LeftFootDominance`, `BulletKey::LowPhysical`, `BulletKey::PressureStrength`).

Narrative-director deliverables owed to make this spec complete:

- Tracery template bank: one template set per `BulletKey` × `UncertaintyBand` (hedged vs. assertive phrasing). Minimum 3 variants per slot.
- `BulletKey` enumeration — the full list of trigger-able bullets. Proposed starting set:
  - Strengths: `HighTechnical`, `HighMental`, `HighPhysical`, `LeftFootDominance`, `DeadBallAffinity`, `ComposureUnderPressure`, `RelentlessEngine`, `ReadsTheGame`
  - Weaknesses: `LowPhysical`, `LowMental`, `LowTechnical`, `StrugglesUnderScrutiny`, `SlowStarter`, `AerialWeakness`
- The identity sentence (the verbose form used in the detail screen header at Confident/Settled) for each `IdentityDescriptorDTO` variant × `UncertaintyBand`.

The DTO carries the rendered strings; the template authoring and key-to-string rendering live in the content layer. The DTO builder calls a sync `render_bullet(key, band) -> String` function (already-compiled Tracery templates, no network, no LLM).

---

## 5. Per-surface layout

### 5.1 Squad table row

Fields used: `displayNameShort`, `starsFilled`, `starsReach`, `identityChip`, `band`.

Layout (single row, left-to-right):

```
[Name]   [Stars: ★★★★◐◐☆☆☆]   [Chip: "technical artist"]
```

- Star glyphs: filled (★), half-hollow (◐), hollow (☆). Rendered from `starsFilled` and `starsReach` — fill the first `starsFilled` stars, then extend hollow to `starsReach`, then remainder empty.
- Chip: absent when `identityChip` is null (Hunch). Tentative chips carry the `?` suffix. Chip background colour is keyed on `identityDescriptor`.
- Row height: single line. The star row and chip fit inline. No expand/collapse on the row itself.

### 5.2 Player detail screen

Layout: three sections stacked vertically.

**Header block**
- `displayNameShort` + role + age (age comes from a separate player snapshot DTO, not `PlayerAssessmentDTO`)
- Identity chip (same rendering as squad row, but larger)
- Identity sentence (Confident/Settled only; from Tracery template rendered at DTO build time — add `identity_sentence: Option<String>` to DTO)

**Radar + verdict (side by side on wide viewport, stacked on narrow)**

Left: `RadarDTO` → ECharts radar component. Axis values from `technical`, `mental`, `physical`, `goalkeeping` (collapsed for outfielders), `contextAxis`. Rendering mode from `renderingMode`. Legend explains fill state.

Right: Verdict bullets.
- `strengthsHeading` (h3)
- `strengthBullets` as unordered list items
- `weaknessesHeading` (h3)
- `weaknessBullets` as unordered list items

**Tier-words bar**

Horizontal row below radar+verdict:
```
Technical   [tier label]   [uncertainty bar fill]
Mental      [tier label]   [uncertainty bar fill]
Physical    [tier label]   [uncertainty bar fill]
```

The uncertainty bar is a thin progress bar; fill = `tierTechnical.midpoint`; translucent extension fill = `tierTechnical.midpoint + tierTechnical.bandWidth / 2`, clamped to 1.0. Never renders a number.

**Power-user toggle** (bottom, inline toggle control)

When toggled ON: tier-words are replaced with `numericRanges` formatted as "14–17" per group. The toggle state is client-side only (a SolidJS signal, not persisted).

### 5.3 Power-user toggle

DTO field: `numericRanges: Option<NumericRangesDTO>`. The Tauri command is called with `include_ranges: true` when the toggle is first enabled (lazy fetch — the ranges are not in the default DTO response). The DTO returns `numericRanges: null` by default.

Display format when ON: replace each tier-word with `"[low]–[high]"`. Keep the uncertainty bar below it. Never show a single value.

---

## 6. Pillar alignment

### Pillar 4 — scouting uncertainty

Every surface element is derived from `GeneCategoryEstimate { low, high }` and `UncertaintyBand`. The rule that the `band` field represents the worst band (not the mean) means uncertainty propagates conservatively. A player with one unknown dimension presents as uncertain everywhere — exactly what Pillar 4 requires.

Crucially, Hunch-tier players still give usable information: one or two strength bullets appear (the evidence that is there, rendered), a partially-filled star row appears, and the radar shows a few dots where physical or technical genes are detectable (scouts observe physical by `height_ceiling`, `fast_twitch_ratio` — these show up as data even on a first watch). The design avoids the failure mode where an unscouted player shows a completely blank panel, which reads as a bug to the player rather than honest uncertainty.

### Pillar 5 — signature identity

The identity descriptor is the primary identity expression in the squad row. A player's signature move does not appear in this panel (signatures are a match-day surface). But the identity descriptor is deliberately aligned with signature categories: a `TechnicalArtist` will tend to have technical-affinity signatures; a `PhysicalSpecimen` will have physical signatures. The chip creates the expectation; the match watching delivers it.

The radar shape is the visual identity read. A physical specimen has a physically-dominant silhouette. That shape, read at a glance across a squad list, is Pillar 5 made visual — you are scanning player identities, not stat lines.

### No omniscient raw stat rule

No raw attribute value from `PlayerAttributes` ever appears in the player-facing UI. The Q32 [0, 1] values stay in the sim layer. The DTO boundary converts to:
- f64 group means for radar axis scaling
- Integer 1–20 ranges (power-user toggle, which is the only place a number appears, and it is always a range, never a point)
- Tier-word strings
- Tracery-rendered bullet prose

The `internal_gene_snapshot` field on `PlayerBio` is explicitly never serialized to any DTO. Gene values are invisible; phenotype labels and their derived tier-words are the player-facing surface.

---

## 7. Tuning values (Phase-N, subject to balance revision)

These live here in the design doc, not in SPEC, not in DECISIONS.

| Constant | Value | Rationale |
|---|---|---|
| LateBloomer growth_curve threshold | 0.40 (Q32 raw 1_717_986_918) | Coincides with TENTATIVE band floor — a late-bloomer flag should be detectable with some scouting work but not given away at first look |
| LeftFootedWand left_foot threshold | 0.65 (Q32 raw 2_793_155_430) | Comfortably clear of the midpoint; a 0.51 left-foot dominance is not a "wand" |
| LeftFootedWand technical_mean floor | 0.60 (Q32 raw 2_576_980_378) | Aligned to TENTATIVE_MAX — foot dominance without technical quality is not a wand |
| PhysicalSpecimen frame_density threshold | 0.68 (Q32 raw 2_920_577_761) | Notably high frame_density; avoids firing on ordinary fit players |
| PhysicalSpecimen fast_twitch_ratio threshold | 0.72 (Q32 raw 3_092_376_533) | Distinctly fast-twitch dominated |
| MentalityMonster mental_mean floor | 0.63 (Q32 raw 2_706_956_661) | Meaningfully above the solid-to-good boundary |
| Hunch star floor penalty | −2.0 stars | Wide spread for genuinely unknown players |
| Tentative star floor penalty | −1.0 star | Moderate uncertainty |
| Tier adjacency threshold | high_minus_low > 0.18 | Trigger adjacent-tier label rather than single-tier |
| Minimum numeric range display width | 2 integer steps | No false precision at any band |

---

## 8. Open questions for the owner

**Q1 — Descriptor count.** Five descriptors cover most archetypes. The gap is the genuinely well-rounded player (all groups 0.55–0.62, no gene flags) who defaults to "technical artist." Should there be a "complete footballer" descriptor for high-CA balanced players, or is the default fallback acceptable?

**Q2 — Left-footed wand for right-dominant players.** The current spec only classifies left-foot dominance. Should `left_foot < 0.25` trigger a "right-footed specialist" chip, or is this a second-phase addition after seeing how distinctive left-foot feels in play?

**Q3 — Radar axis 5.** For outfielders the fifth axis is "engine/stamina" (`physical.stamina` midpoint). Should this instead be a combined "fitness" axis (mean of `stamina` + `natural_fitness` + `stamina_recovery_gene`)? Stamina alone may under-represent what the radar axis is trying to say.

**Q4 — Band aggregation.** Using the worst band across three categories is conservative. An alternative is the mean band (majority of categories is what drives the label). The worst-band rule is safer for Pillar 4 integrity but may make mid-game players feel more uncertain than they should be. Worth playtesting before locking.

**Q5 — Numeric toggle discoverability.** Default OFF is correct for the clean launch experience. Should there be a visual hint (a small icon, an affordance in the detail screen) that the numeric view exists, or does it surface through settings only?

---

## 9. UI prototype work that follows

For the ui-programmer to build a mockup against a player fixture (no backend required):

1. Author two static JSON fixtures in `frontend/public/dev-fixtures/` matching the `PlayerAssessmentDTO` shape above — one for a Settled player (the Marcos Vidal archetype), one for a Hunch player (Dario Kessler archetype). These can be hand-crafted from the DTO shape; no Rust build needed.
2. Implement `PlayerAssessmentDTO` type in `frontend/src/lib/types.ts` from the spec above.
3. Build `SquadRow` component (stars + chip, reads `starsFilled`, `starsReach`, `identityChip`).
4. Build `PlayerDetail` screen with three sections (header, radar+verdict, tier-words bar). Use the ECharts radar component for the radar; the `RadarDTO.renderingMode` field controls opacity and dash style.
5. Build the power-user toggle as a SolidJS `createSignal(false)` signal; when `true`, replace tier-word text with `numericLow–numericHigh` strings from a second fixture that includes `numericRanges`.
6. Run `node frontend/scripts/board-shots.mjs` (or the screenshot harness) to produce PNGs of both fixture states (Settled and Hunch) for review.

The Tracery-rendered bullet strings can be static placeholder prose in the dev fixtures — the template engine wiring is a separate narrative-director task. The UI prototype only needs the string to be present; the templates do not need to be live for the mockup review.

---

**Relevant source files grounding this spec:**

- `/Users/vibelogic/dev/football/crates/fw-scouting/src/band.rs` — `UncertaintyBand`, thresholds, `GeneCategoryEstimate::band()`
- `/Users/vibelogic/dev/football/crates/fw-scouting/src/report.rs` — `GeneCategoryEstimate { low, high }`, `ScoutReport`, `LabelEstimate`
- `/Users/vibelogic/dev/football/crates/fw-scouting/src/scout.rs` — `BASIC_SCOUT_BAND_HALF_WIDTH = 0.12`, `BASIC_SCOUT_OBSERVATION_NOISE = 0.10`
- `/Users/vibelogic/dev/football/crates/fw-core/src/player_attributes.rs` — `TechnicalAttributes (14)`, `MentalAttributes (10)`, `PhysicalAttributes (8)`, `AbilityCeiling { current, potential }`, `PlayerCondition`
- `/Users/vibelogic/dev/football/crates/fw-content/src/gene.rs` — `GeneSnapshot`, `PhysicalGenes.growth_curve` (signed [-1,+1]), `TechnicalAffinities.left_foot`, `PhysicalGenes.frame_density`, `PhysicalGenes.fast_twitch_ratio`, `NarrativeFlag::LateBloomer`
- `/Users/vibelogic/dev/football/crates/fw-content/src/player_bio.rs` — `PlayerBio.scout_labels: BTreeSet<PhenotypeLabelId>`, 46 `PhenotypeLabelId` variants with `display_label()`, `CommentaryHandles`
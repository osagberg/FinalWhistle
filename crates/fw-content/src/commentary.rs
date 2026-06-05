//! Commentary renderer — T1-4b.
//!
//! Deterministic in-match commentary variant-pick via Tracery grammars.
//! `CommentaryGrammarBank` holds one Tracery grammar (as a raw-rules map) per
//! `MatchEventDiscriminant`. `render_event` seeds a `ChaCha8Rng` from
//! `seed_fn(match_seed, tick, SeedLayer::Commentary, site)` and calls
//! `Grammar::flatten` — no `thread_rng` anywhere in the render path.
//!
//! ## Site formula (ADR-0009 amendment 2026-05-16)
//!
//! `site = ((player_slot as u32) << 16) | (event_class_discriminant as u32)`
//!
//! For events without a natural player slot (KickOff / FullTime) the sentinel
//! `player_slot = 0xFF` (u8 max) is used. In the site u32 this becomes
//! `0x00FF_0000`, distinct from any real slot (0..=21 → `0x0000_0000..0x0015_0000`).
//!
//! ## Substitution variables
//!
//! Variables are injected as single-entry rules into a per-render Grammar clone,
//! shadowing any template rule with the same name. Each event class exposes:
//!
//! | Event class        | Variables                                              |
//! |--------------------|--------------------------------------------------------|
//! | KickOff            | `tick`, `isSecondHalf`                                 |
//! | FullTime           | `tick`, `homeScore`, `awayScore`                       |
//! | Goal               | `tick`, `scorerSlot`, `homeScore`, `awayScore`         |
//! | Shot               | `tick`, `shooterSlot`, `targetX`, `targetY`, `onTarget`|
//! | Pass               | `tick`, `fromSlot`, `toSlot`, `passKind`, `completed`  |
//! | SignatureFirstFired | `tick`, `playerSlot`, `signatureId`                   |
//!
//! Numeric Q32 fields → f64 strings with 1 decimal place. These strings do NOT
//! enter canonical state — read-side projection only (mirror of `fw-match-sim::dto`).

use std::collections::BTreeMap;

use fw_core::{PlayerSlot, Q32, seed::SeedLayer, seed::seed_fn};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::event::{MatchEvent, PassKind};

// ---------------------------------------------------------------------------
// MatchEventDiscriminant — re-export (moved to event module at T1-11 fix-pass)
// ---------------------------------------------------------------------------

/// Re-export of [`crate::event::MatchEventDiscriminant`].
///
/// **Located in `event.rs` post Codex T1-11 type-design P1 fix-pass**: this
/// enum was originally defined here, but `MatchEvent::discriminant()` returns
/// it (typed enum, not raw `u8`) — and event.rs cannot import from
/// commentary.rs (cyclic). Moved to event.rs as the canonical home; the
/// re-export here preserves the public API for downstream consumers
/// (`fw_content::commentary::MatchEventDiscriminant` still resolves to the
/// same type).
pub use crate::event::MatchEventDiscriminant;

// ---------------------------------------------------------------------------
// CommentaryGrammarBank
// ---------------------------------------------------------------------------

/// Holds the raw Tracery rule maps (one per `MatchEventDiscriminant`).
///
/// We store `BTreeMap<String, Vec<String>>` rather than a compiled
/// `tracery::Grammar` because:
/// 1. `Grammar` is not `Serialize` — we can't extract its rules after build.
/// 2. Per-render we need to inject substitution variables as additional rules
///    (e.g. `"tick" → ["42"]`), which requires rebuilding the grammar from
///    the merged rule set. Storing the raw rules makes this cheap.
///
/// **Invariant:** `rules` has exactly one entry per `MatchEventDiscriminant`
/// variant. Enforced by `try_from_map`; callers cannot break it.
///
/// `signature_banks` is an optional sub-keyed override for
/// `SignatureFirstFired` — keyed by the signature SLUG (the final
/// dot-separated component of the signature ID, e.g. `"long-range-strike"`).
/// When a slug is present, `render_event` routes `SignatureFirstFired` to
/// that sub-bank; otherwise it falls back to the generic
/// `MatchEventDiscriminant::SignatureFirstFired` rules.
///
/// BTreeMap — deterministic iteration order for any future validation pass.
pub struct CommentaryGrammarBank {
    rules: BTreeMap<MatchEventDiscriminant, BTreeMap<String, Vec<String>>>,
    /// Per-signature sub-banks. Keyed by signature slug (e.g. `"long-range-strike"`).
    /// Each sub-bank must have a non-empty `origin` rule with ≥1 non-empty variant.
    signature_banks: BTreeMap<String, BTreeMap<String, Vec<String>>>,
}

impl CommentaryGrammarBank {
    /// Construct from a map of discriminant → raw rule map.
    ///
    /// Returns `Err` if:
    /// - any `MatchEventDiscriminant` variant is absent (`MissingGrammarError`), OR
    /// - any grammar lacks a non-empty `origin` rule (`EmptyOriginRule`).
    ///
    /// The empty-origin check (Codex Tier-2 type-design P1 on T1-4b 2026-05-16)
    /// catches the construction-time hole where `Goal -> { "origin": vec![] }`
    /// or `Goal -> {}` passed `try_from_map` and `render_event` later returned
    /// silent-empty. Aligns construction-time guard with the actual
    /// content-pack contract: a usable grammar per discriminant means a
    /// non-empty `origin` rule with ≥1 non-empty variant.
    ///
    /// Fail-loud per T1-12 content-validation hardening pattern.
    pub fn try_from_map(
        map: BTreeMap<MatchEventDiscriminant, BTreeMap<String, Vec<String>>>,
    ) -> Result<Self, CommentaryBankBuildError> {
        for disc in MatchEventDiscriminant::all() {
            let Some(rules) = map.get(&disc) else {
                return Err(CommentaryBankBuildError::MissingGrammar(disc));
            };
            // The `origin` rule is Tracery's entry point — required by the
            // renderer's `Grammar::flatten("#origin#", ...)` call. Without it,
            // every render for this discriminant would error at runtime.
            let Some(origin_variants) = rules.get("origin") else {
                return Err(CommentaryBankBuildError::MissingOriginRule(disc));
            };
            if origin_variants.is_empty() {
                return Err(CommentaryBankBuildError::EmptyOriginRule(disc));
            }
            // At least one non-empty variant — otherwise Tracery renders to
            // empty string. Codex T1-4b type-design P1 acceptance criterion.
            if origin_variants.iter().all(|v| v.is_empty()) {
                return Err(CommentaryBankBuildError::AllEmptyOriginVariants(disc));
            }
        }
        Ok(Self {
            rules: map,
            signature_banks: BTreeMap::new(),
        })
    }

    /// Attach a per-signature sub-bank keyed by slug.
    ///
    /// Called by the content loader after the primary `try_from_map`
    /// construction, once per `signature_first_fired.<slug>.tracery.json`
    /// file it encounters. Validation mirrors `try_from_map`'s origin-rule
    /// discipline: the sub-bank must have a non-empty `origin` with ≥1
    /// non-empty variant, else `CommentaryBankBuildError` is returned.
    ///
    /// Inserting the same slug twice is last-writer-wins. This is unreachable
    /// today: `load_commentary_grammars` walks a single directory and two
    /// files there cannot share a name, so two `signature_first_fired.<slug>`
    /// files cannot yield the same slug. There is intentionally NO
    /// duplicate-slug guard here yet (it would guard a code path that does not
    /// exist — see Rust/RULES "no speculative abstractions"). When commentary
    /// loading spans mod overlays (multiple directories, post-T2-3), a
    /// duplicate-slug check MUST be added in the loader before this call —
    /// mirroring the RON loader's `insert_unique`/`DuplicateId` discipline —
    /// so a mod cannot silently clobber a core sub-bank. Tracked as a
    /// follow-up; see the T4-2.5i self-review note.
    pub fn insert_signature_bank(
        &mut self,
        slug: String,
        rules: BTreeMap<String, Vec<String>>,
    ) -> Result<(), CommentaryBankBuildError> {
        // Re-use the same origin-rule discipline as try_from_map. The
        // discriminant used in error variants is SignatureFirstFired — the
        // only discriminant these sub-banks belong to.
        let disc = MatchEventDiscriminant::SignatureFirstFired;
        let Some(origin_variants) = rules.get("origin") else {
            return Err(CommentaryBankBuildError::MissingOriginRule(disc));
        };
        if origin_variants.is_empty() {
            return Err(CommentaryBankBuildError::EmptyOriginRule(disc));
        }
        if origin_variants.iter().all(|v| v.is_empty()) {
            return Err(CommentaryBankBuildError::AllEmptyOriginVariants(disc));
        }
        self.signature_banks.insert(slug, rules);
        Ok(())
    }

    /// Return the number of `origin` variants in the named signature sub-bank.
    ///
    /// Returns 0 if the slug is not present or the `origin` rule is absent.
    /// Used by tests to assert the ≥3 variant requirement without reaching
    /// into internal fields directly.
    #[must_use]
    pub fn signature_bank_origin_len(&self, slug: &str) -> usize {
        self.signature_banks
            .get(slug)
            .and_then(|rules| rules.get("origin"))
            .map(|v| v.len())
            .unwrap_or(0)
    }

    /// Look up the raw rules for a given discriminant.
    ///
    /// Panics if the discriminant is missing — the `try_from_map` invariant
    /// guarantees every discriminant is present; this is unreachable after a
    /// valid construction.
    fn get_rules(&self, disc: MatchEventDiscriminant) -> &BTreeMap<String, Vec<String>> {
        self.rules
            .get(&disc)
            .expect("CommentaryGrammarBank invariant: all discriminants present at construction")
    }
}

impl std::fmt::Debug for CommentaryGrammarBank {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CommentaryGrammarBank")
            .field(
                "loaded_discriminants",
                &self.rules.keys().collect::<Vec<_>>(),
            )
            .field(
                "signature_slugs",
                &self.signature_banks.keys().collect::<Vec<_>>(),
            )
            .finish()
    }
}

impl Clone for CommentaryGrammarBank {
    fn clone(&self) -> Self {
        Self {
            rules: self.rules.clone(),
            signature_banks: self.signature_banks.clone(),
        }
    }
}

/// Error returned by `CommentaryGrammarBank::try_from_map` when the bank
/// cannot be constructed cleanly.
///
/// Codex Tier-2 type-design P1 on T1-4b 2026-05-16: replaced single-purpose
/// `MissingGrammarError(MatchEventDiscriminant)` with a multi-variant enum
/// so callers can distinguish absent-grammar (loader-side bug — file is
/// missing) from malformed-grammar (content-authoring bug — file exists
/// but lacks origin OR has only empty variants). The runtime loader maps
/// all four variants to `ContentLoadError::MissingCommentaryGrammar` for
/// now (uniform fail-loud); future T1-12 hardening can distinguish.
#[derive(Debug)]
pub enum CommentaryBankBuildError {
    /// Map does not contain any entry for this discriminant.
    MissingGrammar(MatchEventDiscriminant),
    /// Grammar exists but has no `origin` rule (Tracery's entry symbol).
    MissingOriginRule(MatchEventDiscriminant),
    /// Grammar's `origin` rule is an empty `Vec` — no variants to pick from.
    EmptyOriginRule(MatchEventDiscriminant),
    /// Grammar's `origin` rule has variants but all of them are empty strings.
    /// Tracery would render to "" → silent commentary line at runtime.
    AllEmptyOriginVariants(MatchEventDiscriminant),
}

impl CommentaryBankBuildError {
    /// The discriminant the error pertains to (all variants carry one).
    pub fn discriminant(&self) -> MatchEventDiscriminant {
        match self {
            Self::MissingGrammar(d)
            | Self::MissingOriginRule(d)
            | Self::EmptyOriginRule(d)
            | Self::AllEmptyOriginVariants(d) => *d,
        }
    }
}

impl std::fmt::Display for CommentaryBankBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingGrammar(d) => {
                write!(f, "missing commentary grammar for {d:?}")
            }
            Self::MissingOriginRule(d) => write!(
                f,
                "commentary grammar for {d:?} has no `origin` rule (required by the renderer)"
            ),
            Self::EmptyOriginRule(d) => write!(
                f,
                "commentary grammar for {d:?} has an empty `origin` rule (Vec::new())"
            ),
            Self::AllEmptyOriginVariants(d) => write!(
                f,
                "commentary grammar for {d:?} has only empty-string `origin` variants \
                 (Tracery would render to empty)"
            ),
        }
    }
}

impl std::error::Error for CommentaryBankBuildError {}

// ---------------------------------------------------------------------------
// CommentaryRenderError — render-time failures
// ---------------------------------------------------------------------------

/// Render-time failure for `render_event`.
///
/// Codex Tier-2 silent-failure P0 on T1-4b 2026-05-16: prior `render_event`
/// returned `String` and silently converted every `tracery::Error` (parse
/// failure, `MissingKeyError`, modifier-not-found, infinite-recursion guard)
/// into `String::default()` via `unwrap_or_default`. A template typo like
/// `#scorerSlotx#` (missing key) would silently produce an empty commentary
/// line at runtime with NO log, NO telemetry, NO test signal — the hot test
/// suite happens to use multi-variant grammars that always render, so the
/// bug ships green. This is the canonical silent-failure anti-pattern the
/// `pr-review-toolkit:silent-failure-hunter` agent is named after.
///
/// New surface: `render_event` returns `Result<String, CommentaryRenderError>`.
/// Callers (T1-5 Tauri play_match, T1-6 frontend Match recap) decide how to
/// degrade: log + display "(commentary unavailable)" / fall back to a generic
/// event description / swallow + return empty — but the decision is explicit.
#[derive(Debug)]
pub enum CommentaryRenderError {
    /// Tracery raised an error during render. Most common cause: template
    /// references a variable that the renderer doesn't inject (typo in the
    /// `.tracery.json` file).
    Tracery {
        event_class: MatchEventDiscriminant,
        source: tracery::Error,
    },
    /// Tracery returned a successful render but the output is empty. Most
    /// common cause: a grammar's `origin` rule has an empty-string variant
    /// (`{"origin": [""]}`) or the variant-pick landed on an empty entry.
    /// Distinct from Tracery error — useful for content-authoring follow-up.
    EmptyOutput { event_class: MatchEventDiscriminant },
}

impl std::fmt::Display for CommentaryRenderError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Tracery {
                event_class,
                source,
            } => write!(f, "commentary render failed for {event_class:?}: {source}"),
            Self::EmptyOutput { event_class } => write!(
                f,
                "commentary render produced empty string for {event_class:?} \
                 (content-authoring bug: check `origin` rule has non-empty variants)"
            ),
        }
    }
}

impl std::error::Error for CommentaryRenderError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Tracery { source, .. } => Some(source),
            Self::EmptyOutput { .. } => None,
        }
    }
}

// ---------------------------------------------------------------------------
// render_event — public surface
// ---------------------------------------------------------------------------

/// Render a commentary line for `event`.
///
/// Determinism: same `(match_seed, event, slot_names)` → same output, every platform.
/// `ChaCha8Rng` is seeded from `seed_fn(match_seed, tick, SeedLayer::Commentary,
/// site)` — no `thread_rng`, no `OsRng`.
///
/// `slot_names` maps `PlayerSlot` → display name for the current match roster.
/// For `SignatureFirstFired`, the `playerName` substitution variable is
/// populated from this map if the slot is present; otherwise a deterministic
/// positional label is used ("a forward", "a midfielder", etc.). Pass
/// `&BTreeMap::new()` when no roster is available (dev/test paths).
///
/// **Per-signature routing (T4-2.5i):** for `SignatureFirstFired`, if the
/// bank contains a sub-bank keyed by the signature slug (the final
/// dot-separated component of `signature_id`, e.g. `"long-range-strike"`),
/// that sub-bank is used in preference to the generic
/// `SignatureFirstFired` rules. Fall-through to the generic bank if no
/// sub-bank exists for the slug.
///
/// **Returns `Result<String, CommentaryRenderError>`** (changed from `String`
/// at T1-4b fix-pass per Codex silent-failure P0 — see `CommentaryRenderError`
/// docs). The Tracery render path or empty-output check produce typed errors
/// that callers handle explicitly: log + fall back to a generic line, OR
/// surface "(commentary unavailable)" in the UI. The prior `unwrap_or_default`
/// silently masked template typos with empty strings.
///
/// **Invariant guaranteed by `CommentaryGrammarBank::try_from_map`**: the
/// bank always contains a grammar for `MatchEventDiscriminant::from_event(event)`,
/// so the grammar-lookup path is infallible — only Tracery render failures
/// or empty-output bugs reach the error path.
pub fn render_event(
    event: &MatchEvent,
    match_seed: u64,
    bank: &CommentaryGrammarBank,
    slot_names: &BTreeMap<PlayerSlot, String>,
) -> Result<String, CommentaryRenderError> {
    let disc = MatchEventDiscriminant::from_event(event);
    let (tick_raw, player_slot) = event_tick_and_slot(event);

    let site = ((player_slot as u32) << 16) | (disc as u32);
    let derived = seed_fn(match_seed, tick_raw, SeedLayer::Commentary, site);
    let mut rng = ChaCha8Rng::seed_from_u64(derived);

    // For SignatureFirstFired: try slug-keyed sub-bank first, fall back to
    // the generic rules.
    let base_rules: &BTreeMap<String, Vec<String>> = match event {
        MatchEvent::SignatureFirstFired { signature_id, .. } => {
            let slug = signature_slug(signature_id.as_str());
            bank.signature_banks
                .get(slug)
                .unwrap_or_else(|| bank.get_rules(disc))
        }
        _ => bank.get_rules(disc),
    };

    let mut vars = build_vars(event);

    // Inject playerName for SignatureFirstFired.
    if let MatchEvent::SignatureFirstFired { player_slot, .. } = event {
        let name = slot_names
            .get(player_slot)
            .cloned()
            .unwrap_or_else(|| slot_positional_label(*player_slot).to_string());
        vars.push(("playerName".into(), name));
    }

    let output = render_with_vars(base_rules, &vars, &mut rng).map_err(|source| {
        CommentaryRenderError::Tracery {
            event_class: disc,
            source,
        }
    })?;

    if output.is_empty() {
        return Err(CommentaryRenderError::EmptyOutput { event_class: disc });
    }

    Ok(output)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

/// Extract the slug from a signature ID string.
///
/// The slug is the final dot-separated component of the ID. For
/// `"fwh.core:signature.long-range-strike"` the slug is
/// `"long-range-strike"`. For a malformed ID with no dot the full
/// string is returned (safe fallback — just won't match any sub-bank).
fn signature_slug(id: &str) -> &str {
    id.rsplit('.').next().unwrap_or(id)
}

/// Return a football-native positional label for a player slot.
///
/// Slot layout (22-player match, 0-based):
///   - Home side: 0=GK, 1-4=DEF, 5-7=MID, 8-10=FWD
///   - Away side: 11=GK, 12-15=DEF, 16-18=MID, 19-21=FWD
///
/// Slots outside the known range produce "a player" (safe fallback that
/// doesn't expose a raw number to commentary output).
fn slot_positional_label(slot: PlayerSlot) -> &'static str {
    match slot {
        0 | 11 => "the goalkeeper",
        1..=4 | 12..=15 => "a defender",
        5..=7 | 16..=18 => "a midfielder",
        8..=10 | 19..=21 => "a forward",
        _ => "a player",
    }
}

/// Sentinel player slot for events with no natural player (KickOff / FullTime).
const SLOT_SENTINEL: PlayerSlot = 0xFF;

/// Extract `(tick_as_u32, player_slot_for_site)` from an event.
fn event_tick_and_slot(event: &MatchEvent) -> (u32, PlayerSlot) {
    match event {
        MatchEvent::KickOff { tick, .. } => (tick.to_raw() as u32, SLOT_SENTINEL),
        MatchEvent::FullTime { tick, .. } => (tick.to_raw() as u32, SLOT_SENTINEL),
        MatchEvent::Goal {
            tick, scorer_slot, ..
        } => (tick.to_raw() as u32, *scorer_slot),
        MatchEvent::Shot {
            tick, shooter_slot, ..
        } => (tick.to_raw() as u32, *shooter_slot),
        MatchEvent::Pass {
            tick, from_slot, ..
        } => (tick.to_raw() as u32, *from_slot),
        MatchEvent::SignatureFirstFired {
            tick, player_slot, ..
        } => (tick.to_raw() as u32, *player_slot),
        MatchEvent::Offside {
            tick,
            offending_slot,
        } => (tick.to_raw() as u32, *offending_slot),
        // FUN-CB1: PassIncomplete — from_slot is the primary actor.
        MatchEvent::PassIncomplete {
            tick, from_slot, ..
        } => (tick.to_raw() as u32, *from_slot),
    }
}

/// Build the substitution variable list for a given event.
///
/// Returns `Vec<(name, value_string)>`. Each pair becomes a single-entry rule
/// in the merged grammar, shadowing any template rule with the same name.
fn build_vars(event: &MatchEvent) -> Vec<(String, String)> {
    match event {
        MatchEvent::KickOff {
            tick,
            is_second_half,
        } => vec![
            ("tick".into(), tick.to_raw().to_string()),
            ("isSecondHalf".into(), is_second_half.to_string()),
        ],
        MatchEvent::FullTime {
            tick,
            home_score,
            away_score,
        } => vec![
            ("tick".into(), tick.to_raw().to_string()),
            ("homeScore".into(), home_score.to_string()),
            ("awayScore".into(), away_score.to_string()),
        ],
        MatchEvent::Goal {
            tick,
            scorer_slot,
            score_home_after,
            score_away_after,
        } => vec![
            ("tick".into(), tick.to_raw().to_string()),
            ("scorerSlot".into(), scorer_slot.to_string()),
            ("homeScore".into(), score_home_after.to_string()),
            ("awayScore".into(), score_away_after.to_string()),
        ],
        MatchEvent::Shot {
            tick,
            shooter_slot,
            target_x,
            target_y,
            on_target,
        } => vec![
            ("tick".into(), tick.to_raw().to_string()),
            ("shooterSlot".into(), shooter_slot.to_string()),
            ("targetX".into(), q32_to_str(*target_x)),
            ("targetY".into(), q32_to_str(*target_y)),
            ("onTarget".into(), on_target.to_string()),
        ],
        MatchEvent::Pass {
            tick,
            from_slot,
            to_slot,
            kind,
            completed,
        } => vec![
            ("tick".into(), tick.to_raw().to_string()),
            ("fromSlot".into(), from_slot.to_string()),
            ("toSlot".into(), to_slot.to_string()),
            ("passKind".into(), pass_kind_str(*kind).into()),
            ("completed".into(), completed.to_string()),
        ],
        MatchEvent::SignatureFirstFired {
            tick,
            player_slot,
            signature_id,
        } => vec![
            ("tick".into(), tick.to_raw().to_string()),
            ("playerSlot".into(), player_slot.to_string()),
            ("signatureId".into(), signature_id.as_str().to_string()),
        ],
        // FUN-TS2b: Offside event variables.
        MatchEvent::Offside {
            tick,
            offending_slot,
        } => vec![
            ("tick".into(), tick.to_raw().to_string()),
            ("offendingSlot".into(), offending_slot.to_string()),
        ],
        // FUN-CB1: PassIncomplete event variables.
        MatchEvent::PassIncomplete {
            tick,
            from_slot,
            to_slot,
            kind,
        } => vec![
            ("tick".into(), tick.to_raw().to_string()),
            ("fromSlot".into(), from_slot.to_string()),
            ("toSlot".into(), to_slot.to_string()),
            ("passKind".into(), pass_kind_str(*kind).into()),
        ],
    }
}

/// Project a `Q32` as a 1-decimal-place `f64` string.
///
/// The f64 does NOT enter canonical state — it's prose only (mirror of
/// `fw-match-sim::dto` q32_to_f64 projection pattern: `raw_bits as f64 / 2^32`).
/// The `#[allow]` is justified: Q32→f64 for UI/commentary text is the
/// explicitly sanctioned float use (Tauri/RULES.md §3 "DTOs use f64 freely").
#[allow(clippy::float_arithmetic)]
fn q32_to_str(v: Q32) -> String {
    const Q32_SCALE: f64 = 4_294_967_296.0; // 2^32
    let f: f64 = v.to_bits() as f64 / Q32_SCALE;
    format!("{f:.1}")
}

fn pass_kind_str(kind: PassKind) -> &'static str {
    match kind {
        PassKind::Short => "short",
        PassKind::Long => "long",
        PassKind::Cross => "cross",
        PassKind::LayOff => "layoff",
    }
}

/// Merge `base_rules` with variable overrides, build a `Grammar`, and flatten.
///
/// Variables are injected as single-entry rules; they shadow any same-named
/// key in the template, so `#tick#` resolves to the event's tick string.
///
/// Per-render Grammar construction is intentional — `Grammar` is `Clone` but
/// the only mutation API (`push_rule`) is `pub(crate)`. Storing raw rules and
/// rebuilding is the clean external-API path.
fn render_with_vars(
    base_rules: &BTreeMap<String, Vec<String>>,
    vars: &[(String, String)],
    rng: &mut ChaCha8Rng,
) -> Result<String, tracery::Error> {
    // Codex Tier-2 code-reviewer P2 on T1-4b 2026-05-16: pre-filter base
    // rules to remove any key that's about to be overridden by a var, then
    // append the var entries. Prior implementation relied on the undocumented
    // "last-wins for duplicate keys" behavior of `tracery::Grammar::from_map`
    // — that behavior happens to work today because the crate's internal
    // accumulation does keep the last insert, but the crate's API doesn't
    // promise it. Pre-filtering removes the ordering dependency entirely:
    // a template author who adds a rule named `tick` to a `.tracery.json`
    // file will see the renderer's var win deterministically, regardless of
    // how `Grammar::from_map`'s internals evolve in a future tracery release.
    let var_names: BTreeMap<&str, ()> = vars.iter().map(|(k, _)| (k.as_str(), ())).collect();
    let mut merged: Vec<(String, Vec<String>)> = base_rules
        .iter()
        .filter(|(k, _)| !var_names.contains_key(k.as_str()))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();

    for (k, v) in vars {
        // Single-variant override per substitution var. Each var is the
        // only entry under its key because base_rules were pre-filtered
        // above; from_map's duplicate-key behavior is now irrelevant.
        merged.push((k.clone(), vec![v.clone()]));
    }

    let grammar = tracery::Grammar::from_map(merged)?;
    grammar.flatten(rng)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::SignatureId;
    use fw_core::Tick;

    // ---- helpers ----

    /// Build a minimal CommentaryGrammarBank with 2-variant grammars for each
    /// event class. 2 variants ensures the RNG-pick test is non-vacuous
    /// (anti-vacuousness discipline per spec §"Anti-vacuousness").
    fn two_variant_bank() -> CommentaryGrammarBank {
        let mut map = BTreeMap::new();
        for disc in MatchEventDiscriminant::all() {
            let key = format!("{disc:?}");
            let mut rules: BTreeMap<String, Vec<String>> = BTreeMap::new();
            rules.insert(
                "origin".into(),
                vec![
                    format!("{key} variant-A tick #tick#"),
                    format!("{key} variant-B tick #tick#"),
                ],
            );
            map.insert(disc, rules);
        }
        CommentaryGrammarBank::try_from_map(map).expect("all discriminants present")
    }

    fn kickoff_event() -> MatchEvent {
        MatchEvent::KickOff {
            tick: Tick::from_raw(0),
            is_second_half: false,
        }
    }

    fn fulltime_event() -> MatchEvent {
        MatchEvent::FullTime {
            tick: Tick::from_raw(60),
            home_score: 1,
            away_score: 0,
        }
    }

    fn goal_event() -> MatchEvent {
        MatchEvent::Goal {
            scorer_slot: 9,
            tick: Tick::from_raw(30),
            score_home_after: 1,
            score_away_after: 0,
        }
    }

    fn shot_event() -> MatchEvent {
        MatchEvent::Shot {
            shooter_slot: 9,
            tick: Tick::from_raw(25),
            target_x: Q32::from_int(52),
            target_y: Q32::ZERO,
            on_target: true,
        }
    }

    fn pass_event() -> MatchEvent {
        MatchEvent::Pass {
            from_slot: 5,
            to_slot: 7,
            tick: Tick::from_raw(10),
            kind: PassKind::Short,
            completed: true,
        }
    }

    fn sig_event() -> MatchEvent {
        let id = SignatureId::try_new("fwh.core:signature.long-range-strike").unwrap();
        MatchEvent::SignatureFirstFired {
            player_slot: 9,
            signature_id: id,
            tick: Tick::from_raw(50),
        }
    }

    // ---- RED test 1: render_event returns non-empty for all 6 variants ----

    #[test]
    fn render_event_kickoff_is_non_empty() {
        let bank = two_variant_bank();
        let result = render_event(&kickoff_event(), 0xDEAD_BEEF, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        assert!(
            !result.is_empty(),
            "render_event(KickOff) returned empty string"
        );
    }

    #[test]
    fn render_event_fulltime_is_non_empty() {
        let bank = two_variant_bank();
        let result = render_event(&fulltime_event(), 0xDEAD_BEEF, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        assert!(
            !result.is_empty(),
            "render_event(FullTime) returned empty string"
        );
    }

    #[test]
    fn render_event_goal_is_non_empty() {
        let bank = two_variant_bank();
        let result = render_event(&goal_event(), 0xDEAD_BEEF, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        assert!(
            !result.is_empty(),
            "render_event(Goal) returned empty string"
        );
    }

    #[test]
    fn render_event_shot_is_non_empty() {
        let bank = two_variant_bank();
        let result = render_event(&shot_event(), 0xDEAD_BEEF, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        assert!(
            !result.is_empty(),
            "render_event(Shot) returned empty string"
        );
    }

    #[test]
    fn render_event_pass_is_non_empty() {
        let bank = two_variant_bank();
        let result = render_event(&pass_event(), 0xDEAD_BEEF, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        assert!(
            !result.is_empty(),
            "render_event(Pass) returned empty string"
        );
    }

    #[test]
    fn render_event_signature_first_fired_is_non_empty() {
        let bank = two_variant_bank();
        let result = render_event(&sig_event(), 0xDEAD_BEEF, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        assert!(
            !result.is_empty(),
            "render_event(SignatureFirstFired) returned empty string"
        );
    }

    // ---- RED test 2: determinism — same seed + event = byte-identical output ----

    #[test]
    fn render_event_is_deterministic_kickoff() {
        let bank = two_variant_bank();
        let ev = kickoff_event();
        let a = render_event(&ev, 0xCAFE_BABE, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        let b = render_event(&ev, 0xCAFE_BABE, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        assert_eq!(
            a, b,
            "render_event(KickOff) produced different outputs for same seed"
        );
    }

    #[test]
    fn render_event_is_deterministic_shot() {
        let bank = two_variant_bank();
        let ev = shot_event();
        let a = render_event(&ev, 0x1234_5678, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        let b = render_event(&ev, 0x1234_5678, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        assert_eq!(
            a, b,
            "render_event(Shot) produced different outputs for same seed"
        );
    }

    #[test]
    fn render_event_is_deterministic_pass() {
        let bank = two_variant_bank();
        let ev = pass_event();
        let a = render_event(&ev, 0xABCD_EF01, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        let b = render_event(&ev, 0xABCD_EF01, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        assert_eq!(
            a, b,
            "render_event(Pass) produced different outputs for same seed"
        );
    }

    // ---- RED test 3: anti-vacuousness — different seeds pick different variants ----
    //
    // With a 2-variant grammar, 20 different seeds should hit both variants.
    // If the RNG is never consulted (e.g. single-variant grammar, or RNG path
    // accidentally skipped), this test fails.

    #[test]
    fn render_event_different_seeds_pick_different_variants_shot() {
        let bank = two_variant_bank();
        let ev = shot_event(); // Shot has a real player slot — standard site formula path
        let results: Vec<String> = (0u64..20)
            .map(|s| {
                render_event(&ev, s, &bank, &BTreeMap::new())
                    .expect("render_event must succeed for test bank")
            })
            .collect();
        let unique: BTreeSet<&str> = results.iter().map(String::as_str).collect();
        assert!(
            unique.len() >= 2,
            "render_event(Shot) produced only 1 unique variant across 20 seeds — \
             RNG path may not be exercised. Got: {unique:?}"
        );
    }

    #[test]
    fn render_event_different_seeds_pick_different_variants_kickoff() {
        // KickOff uses the sentinel slot — exercise that code path too.
        let bank = two_variant_bank();
        let ev = kickoff_event();
        let results: Vec<String> = (0u64..20)
            .map(|s| {
                render_event(&ev, s, &bank, &BTreeMap::new())
                    .expect("render_event must succeed for test bank")
            })
            .collect();
        let unique: BTreeSet<&str> = results.iter().map(String::as_str).collect();
        assert!(
            unique.len() >= 2,
            "render_event(KickOff) produced only 1 variant across 20 seeds — \
             sentinel slot path may not consult RNG. Got: {unique:?}"
        );
    }

    // ---- missing grammar error ----

    #[test]
    fn missing_grammar_returns_error() {
        let empty: BTreeMap<MatchEventDiscriminant, BTreeMap<String, Vec<String>>> =
            BTreeMap::new();
        let result = CommentaryGrammarBank::try_from_map(empty);
        assert!(
            result.is_err(),
            "try_from_map should reject a bank missing all grammars"
        );
    }

    // ---- discriminant derivation ----

    #[test]
    fn from_event_matches_all_discriminants() {
        assert_eq!(
            MatchEventDiscriminant::from_event(&kickoff_event()),
            MatchEventDiscriminant::KickOff
        );
        assert_eq!(
            MatchEventDiscriminant::from_event(&fulltime_event()),
            MatchEventDiscriminant::FullTime
        );
        assert_eq!(
            MatchEventDiscriminant::from_event(&goal_event()),
            MatchEventDiscriminant::Goal
        );
        assert_eq!(
            MatchEventDiscriminant::from_event(&shot_event()),
            MatchEventDiscriminant::Shot
        );
        assert_eq!(
            MatchEventDiscriminant::from_event(&pass_event()),
            MatchEventDiscriminant::Pass
        );
        assert_eq!(
            MatchEventDiscriminant::from_event(&sig_event()),
            MatchEventDiscriminant::SignatureFirstFired
        );
    }

    // ---- variable substitution ----

    #[test]
    fn render_event_shot_contains_tick_value() {
        // The 2-variant grammar uses `#tick#` — verify substitution fires.
        let bank = two_variant_bank();
        let ev = MatchEvent::Shot {
            shooter_slot: 9,
            tick: Tick::from_raw(42),
            target_x: Q32::ZERO,
            target_y: Q32::ZERO,
            on_target: false,
        };
        let result = render_event(&ev, 0, &bank, &BTreeMap::new())
            .expect("render_event must succeed for test bank");
        assert!(
            result.contains("42"),
            "expected tick=42 to appear in rendered string; got: {result:?}"
        );
    }
}

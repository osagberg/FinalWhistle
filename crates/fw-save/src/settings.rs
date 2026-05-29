//! Settings persistence — versioned `SettingsEnvelope` separate from the
//! game-save `SaveEnvelope`.
//!
//! ## Design
//!
//! Settings are app-global, not per-save-game. They live in a dedicated
//! `settings.fwcfg` file in the Tauri app-config directory, NOT alongside
//! game saves. The `SettingsEnvelope` enum mirrors the `SaveEnvelope` shape:
//! a leading version tag byte + bincode-encoded payload, with a forward-
//! migration chain so old settings files remain loadable after schema bumps.
//!
//! ## Wire-byte lock (LOAD-BEARING FOREVER)
//!
//! `SettingsEnvelope::V0` → wire tag `0x00`. This byte is LOCKED FOREVER by
//! the `settings_v0_wire_byte_is_0x00` regression test. New variants append
//! AT THE END (`V1 = 1`, `V2 = 2`, …). Never reorder, never reuse a tag.
//!
//! ## Migration chain (today: V0-only)
//!
//! `load_settings_envelope(bytes)` decodes + migrates to the latest schema
//! (`SettingsV0` today). T4-6b can append a `V1` variant here; the chain
//! expands naturally without touching existing arms.

use serde::{Deserialize, Serialize};
use thiserror::Error;

// ---------------------------------------------------------------------------
// ThemePref
// ---------------------------------------------------------------------------

/// User's preferred colour scheme.
///
/// Default is `Light`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ThemePref {
    #[default]
    Light,
    Dark,
}

// ---------------------------------------------------------------------------
// SettingsV0
// ---------------------------------------------------------------------------

/// Settings payload — schema V0.
///
/// Ships with the T4-6a foundation:
///   - `theme` — light / dark colour scheme.
///   - `reduce_motion` — disables CSS transitions / animations.
///
/// T4-6b will add `text_scale`, `colorblind_palette`, key-rebind map,
/// save-folder override. Those land as `SettingsV1` via the migration chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SettingsV0 {
    /// Preferred colour scheme.
    pub theme: ThemePref,
    /// When `true` the frontend applies `.reduce-motion` to suppress all CSS
    /// transitions and animations for users who have vestibular-disorder
    /// sensitivity.
    pub reduce_motion: bool,
}

impl Default for SettingsV0 {
    fn default() -> Self {
        SettingsV0 {
            theme: ThemePref::Light,
            reduce_motion: false,
        }
    }
}

// ---------------------------------------------------------------------------
// SettingsEnvelope
// ---------------------------------------------------------------------------

/// The settings-file envelope. Versioned so settings files authored against
/// an older build remain loadable after schema bumps.
///
/// ## Variant-tag stability (LOAD-BEARING FOREVER)
///
/// Explicit `= N` discriminants prevent any future sort/reorder from silently
/// swapping wire tags. The wire byte for `V0` (`0x00`) is pinned by
/// `settings_v0_wire_byte_is_0x00` — if bincode changes its varint encoding
/// for discriminant 0 the test fails loudly.
///
/// New variants append AT THE END with the next integer. NEVER reorder,
/// NEVER reuse a discriminant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u32)]
pub enum SettingsEnvelope {
    /// Schema V0 — T4-6a foundation (theme + reduce_motion).
    ///
    /// Wire tag `0x00` is LOCKED FOREVER.
    V0(SettingsV0) = 0,
}

// ---------------------------------------------------------------------------
// Errors
// ---------------------------------------------------------------------------

/// Errors the settings codec can raise.
#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("bincode encode failure: {0}")]
    Encode(#[from] bincode::error::EncodeError),

    #[error("bincode decode failure: {0}")]
    Decode(#[from] bincode::error::DecodeError),

    /// Trailing bytes past the valid encoding — corruption or splice.
    #[error(
        "settings file has trailing bytes: decoded {consumed} of {total} bytes \
         (corruption or splice)"
    )]
    TrailingBytes { consumed: usize, total: usize },
}

// ---------------------------------------------------------------------------
// encode / decode
// ---------------------------------------------------------------------------

/// Encode a `SettingsEnvelope` to bincode bytes.
#[must_use = "encoded bytes are discarded; did you mean to persist via std::fs::write?"]
pub fn encode_settings(envelope: &SettingsEnvelope) -> Result<Vec<u8>, SettingsError> {
    let cfg = bincode::config::standard();
    Ok(bincode::serde::encode_to_vec(envelope, cfg)?)
}

/// Decode the raw envelope from bytes. Rejects trailing bytes (same discipline
/// as `SaveEnvelope::decode` — trailing bytes indicate corruption or a splice).
fn decode_settings_envelope(bytes: &[u8]) -> Result<SettingsEnvelope, SettingsError> {
    let cfg = bincode::config::standard();
    let (envelope, consumed) = bincode::serde::decode_from_slice(bytes, cfg)?;
    if consumed != bytes.len() {
        return Err(SettingsError::TrailingBytes {
            consumed,
            total: bytes.len(),
        });
    }
    Ok(envelope)
}

/// Production load entry point. Decode bytes → run the forward-migration
/// chain → return the latest-schema payload (`SettingsV0` today).
///
/// Forward-migration chain:
///   - `V0(v0)` → returned as-is (current schema, no migration needed).
///   - Unknown discriminant → `SettingsError::Decode(...)` (bincode
///     `OtherString` — fail loud, same pattern as `SaveEnvelope`).
///
/// T4-6b extending to V1: add `V1(v1)` arm with
/// `V0(migrate_settings_v0_to_v1(v0))` → `V1(v1)` → return.
#[must_use = "the loaded SettingsV0 is discarded; load_settings_envelope runs the full migration chain"]
pub fn load_settings_envelope(bytes: &[u8]) -> Result<SettingsV0, SettingsError> {
    match decode_settings_envelope(bytes)? {
        SettingsEnvelope::V0(v0) => Ok(v0),
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip-byte-identical: `encode(decode(bytes)) == bytes`.
    /// Mutation removing serde determinism would fail this.
    #[test]
    fn settings_v0_round_trips_byte_identical() {
        let env = SettingsEnvelope::V0(SettingsV0 {
            theme: ThemePref::Dark,
            reduce_motion: true,
        });
        let bytes_1 = encode_settings(&env).expect("encode 1");
        let v0 = load_settings_envelope(&bytes_1).expect("load");
        let bytes_2 = encode_settings(&SettingsEnvelope::V0(v0)).expect("encode 2");
        assert_eq!(
            bytes_1, bytes_2,
            "encode(decode(encode(x))) must be byte-identical"
        );
    }

    /// The V0 wire tag is locked at `0x00` — regression pin.
    #[test]
    fn settings_v0_wire_byte_is_0x00() {
        let env = SettingsEnvelope::V0(SettingsV0::default());
        let bytes = encode_settings(&env).expect("encode");
        assert_eq!(
            bytes[0], 0x00,
            "V0 wire tag is LOCKED at 0x00 — schema-lock invariant"
        );
    }

    /// `SettingsV0::default()` is Light theme + no reduce-motion.
    #[test]
    fn default_settings_are_light_no_reduce_motion() {
        let s = SettingsV0::default();
        assert_eq!(s.theme, ThemePref::Light);
        assert!(!s.reduce_motion);
    }

    /// A missing settings file (empty bytes / first-run) must NOT use
    /// `load_settings_envelope` — callers return `SettingsV0::default()`
    /// instead. Verify `load_settings_envelope` on an empty slice returns a
    /// decode error (not a default), confirming callers need the explicit
    /// first-run guard.
    #[test]
    fn load_settings_envelope_on_empty_bytes_errors_not_silently_defaults() {
        let result = load_settings_envelope(&[]);
        assert!(
            result.is_err(),
            "empty bytes must not silently produce default settings; \
             callers must handle first-run via SettingsV0::default()"
        );
    }

    /// Trailing bytes past a valid encoding MUST fail with `TrailingBytes`.
    #[test]
    fn load_settings_envelope_rejects_trailing_bytes() {
        let env = SettingsEnvelope::V0(SettingsV0::default());
        let mut bytes = encode_settings(&env).expect("encode");
        let original_len = bytes.len();
        bytes.push(0xFF);

        let err = load_settings_envelope(&bytes).expect_err("trailing bytes must fail");
        match err {
            SettingsError::TrailingBytes { consumed, total } => {
                assert_eq!(consumed, original_len);
                assert_eq!(total, original_len + 1);
            }
            other => panic!("expected TrailingBytes; got {other:?}"),
        }
    }

    /// Dark + reduce_motion=true round-trips cleanly (non-default values).
    #[test]
    fn settings_v0_non_default_values_round_trip() {
        let v0 = SettingsV0 {
            theme: ThemePref::Dark,
            reduce_motion: true,
        };
        let bytes = encode_settings(&SettingsEnvelope::V0(v0.clone())).expect("encode");
        let loaded = load_settings_envelope(&bytes).expect("load");
        assert_eq!(loaded, v0);
    }
}

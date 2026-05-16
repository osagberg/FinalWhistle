//! Post-LLM validation lints.
//!
//! Every baker subcommand pipes generated fragments through these validators
//! before writing RON. A Category-A hit rejects the fragment outright. A
//! Category-B / cliché hit is logged and surfaces in the bake review report.
//!
//! Three layers:
//!   1. `banned_terms` — shells out to `scripts/lint-banned-terms.py` with
//!      `--scope <generated-tmpfile>`. The Python is the single source of
//!      truth; we do not reimplement the catalog in Rust.
//!   2. `licensed_data` — regex match against a curated list of real-world
//!      Premier League / La Liga / Bundesliga / Serie A clubs + canonical
//!      player surnames. Reject on any hit (Category A — no exemption).
//!   3. `cliche_detector` — reject bio sentences that match common LLM tells.
//!      Override per-fragment with a sentinel comment.
//!
//! T1-12 audit-triage hardening: validators previously returned `Ok(())` while
//! unimplemented, masking future wiring errors. They now return
//! `ValidationError::NotImplemented` so any caller that mistakenly invokes them
//! before T2-3 fails loudly rather than silently passing. The `validate`
//! subcommand in `main.rs` does NOT call these functions (it uses the
//! structural validators on `ContentStore` directly), so `cargo run -p
//! fw-content-baker -- validate` continues to pass unchanged.

// Validators are wired to bake subcommands that land at T2-3. Suppress
// dead_code lint until the first bake subcommand consumer arrives.
#![allow(dead_code)]

use std::path::Path;

#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("banned-term violation (Category A): {0}")]
    BannedTerm(String),
    #[error("licensed-data hit: {0}")]
    LicensedData(String),
    #[error("cliché detected: {0}")]
    Cliche(String),
    #[error("validator I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Returned by any validator that has not yet been implemented.
    ///
    /// This variant replaces the prior `Ok(())` stub so callers that
    /// mistakenly invoke unimplemented validators fail loudly rather than
    /// silently passing. The `validator` field names the check function;
    /// `defer_to` names the MASTER_PLAN milestone at which real
    /// implementation lands.
    #[error(
        "validator '{validator}' is not yet implemented \
         (deferred to {defer_to}); \
         fail-closed per T1-12 audit-triage hardening"
    )]
    NotImplemented {
        /// The function name of the unimplemented validator.
        validator: &'static str,
        /// The MASTER_PLAN milestone string where real implementation lands.
        defer_to: &'static str,
    },
}

/// Run the banned-terms lint against a generated content fragment.
///
/// Real implementation (T2-3): spawns
/// `scripts/lint-banned-terms.py <tmpfile>` and parses exit code + stderr.
///
/// Until T2-3 wires `bake-names` as the first consumer, this returns
/// `ValidationError::NotImplemented` so any premature caller fails loudly.
pub fn check_banned_terms(_fragment_path: &Path) -> Result<(), ValidationError> {
    Err(ValidationError::NotImplemented {
        validator: "check_banned_terms",
        defer_to: "T2-3",
    })
}

/// Reject any text containing a real-world licensed club / surname.
///
/// Curated list lives in `data/licensed-blocklist.txt` (gitignored from
/// shipped build; lives alongside this crate for dev-only use).
///
/// Until T2-3 wires `bake-names` as the first consumer, this returns
/// `ValidationError::NotImplemented` so any premature caller fails loudly.
pub fn check_licensed_data(_text: &str) -> Result<(), ValidationError> {
    Err(ValidationError::NotImplemented {
        validator: "check_licensed_data",
        defer_to: "T2-3",
    })
}

/// Cliché detector — reject obvious LLM tells.
///
/// Default patterns (override per-fragment via sentinel):
/// - "passionate about"
/// - "exceptional ability to"
/// - "rising star with bright future"
/// - "the world of football"
/// - "wears his heart on his sleeve" (acceptable as football vernacular but
///   over-used by LLMs; soft-rejected and devs decide)
///
/// Devs override by appending `// bake-baker:allow-cliche reason="..."` to
/// the bake log entry for that fragment.
///
/// Until T2-3 wires `bake-names` as the first consumer, this returns
/// `ValidationError::NotImplemented` so any premature caller fails loudly.
pub fn check_cliche(_text: &str) -> Result<(), ValidationError> {
    Err(ValidationError::NotImplemented {
        validator: "check_cliche",
        defer_to: "T2-3",
    })
}

/// Run all three validators in order. Returns the first error encountered.
///
/// Because all three inner validators currently return `NotImplemented`,
/// `validate_fragment` will return the `check_banned_terms` error first —
/// maintaining the documented "first error wins" ordering so callers can
/// reason about which validator fired.
pub fn validate_fragment(text: &str, fragment_path: &Path) -> Result<(), ValidationError> {
    check_banned_terms(fragment_path)?;
    check_licensed_data(text)?;
    check_cliche(text)?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tests — chunk 4 (T1-12)
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn check_banned_terms_returns_not_implemented_with_correct_fields() {
        let err = check_banned_terms(Path::new("/fake/fragment.ron"))
            .expect_err("check_banned_terms must return NotImplemented");
        match err {
            ValidationError::NotImplemented {
                validator,
                defer_to,
            } => {
                assert_eq!(validator, "check_banned_terms");
                assert_eq!(defer_to, "T2-3");
            }
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    #[test]
    fn check_licensed_data_returns_not_implemented_with_correct_fields() {
        let err = check_licensed_data("some generated text")
            .expect_err("check_licensed_data must return NotImplemented");
        match err {
            ValidationError::NotImplemented {
                validator,
                defer_to,
            } => {
                assert_eq!(validator, "check_licensed_data");
                assert_eq!(defer_to, "T2-3");
            }
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    #[test]
    fn check_cliche_returns_not_implemented_with_correct_fields() {
        let err = check_cliche("some generated text")
            .expect_err("check_cliche must return NotImplemented");
        match err {
            ValidationError::NotImplemented {
                validator,
                defer_to,
            } => {
                assert_eq!(validator, "check_cliche");
                assert_eq!(defer_to, "T2-3");
            }
            other => panic!("expected NotImplemented, got {other:?}"),
        }
    }

    #[test]
    fn validate_fragment_returns_banned_terms_not_implemented_first() {
        // All three validators are NotImplemented; validate_fragment must
        // propagate check_banned_terms' error (first in the chain) rather than
        // a different validator's error.
        let err = validate_fragment("text", Path::new("/fake/x.ron"))
            .expect_err("validate_fragment must return NotImplemented");
        match err {
            ValidationError::NotImplemented {
                validator,
                defer_to,
            } => {
                assert_eq!(
                    validator, "check_banned_terms",
                    "validate_fragment must return banned_terms error first (ordering contract)"
                );
                assert_eq!(defer_to, "T2-3");
            }
            other => panic!("expected NotImplemented from banned_terms, got {other:?}"),
        }
    }
}

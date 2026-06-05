//! Procedural team generation — `generate_team` orchestrator.
//!
//! Entry point for career-init procgen. Given a content store + culture ID +
//! tactical archetype ID + manager archetype ID + deterministic seed, produces
//! a `ProcGenTeam` with 22 player names, a manager name, and a team name.
//!
//! ## Determinism contract
//!
//! All randomness flows through `ChaCha8Rng` seeded via `seed_fn` per
//! ADR-0009. The `SeedLayer::ContentBake` lane is used throughout this
//! module. Site discriminants are fixed constants (see below) so the output
//! is bit-identical across platforms + runs for the same inputs.
//!
//! ## Site discriminants (ContentBake lane)
//!
//! Per MEMORY.md T1-7 design, the module uses a single sequential RNG
//! instance rather than per-site seeds. The RNG is seeded once via
//! `seed_fn(seed.to_u64(), 0, SeedLayer::ContentBake, 0)` and advanced
//! linearly through:
//!
//!   1. Team-name index pick (1 draw).
//!   2. Manager first name (Markov walk — variable draws).
//!   3. Manager last name (Markov walk — variable draws).
//!   4. Player slot 0..21 first names (Markov walk each — variable draws).
//!   5. Player slot 0..21 last names (Markov walk each — variable draws).
//!
//! This is simpler than 50+ per-site seeds and equally deterministic because
//! the RNG sequence is a pure function of the seed.

use std::collections::BTreeSet;

use rand::Rng;
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use fw_core::{Seed, SeedLayer, seed_fn};

use crate::ContentStore;
use crate::markov::{MarkovError, MarkovNameChain};

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// A single player name (first + last).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerName {
    pub first: String,
    pub last: String,
}

impl PlayerName {
    /// Full display name in the culture's naming pattern.
    /// At T1-7 this is always `"{first} {last}"` regardless of the
    /// culture's `naming_pattern` field; T2 can honour it.
    pub fn display(&self) -> String {
        format!("{} {}", self.first, self.last)
    }
}

/// A manager name (first + last).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManagerName {
    pub first: String,
    pub last: String,
}

impl ManagerName {
    pub fn display(&self) -> String {
        format!("{} {}", self.first, self.last)
    }
}

/// The output of `generate_team`: a team name, a manager name, and 22
/// player names. Name-only at T1-7; attributes come from `PlayerTemplate`
/// consumption at T2 procgen-player row.
#[derive(Debug, Clone)]
pub struct ProcGenTeam {
    pub team_name: String,
    pub manager: ManagerName,
    pub players: [PlayerName; 22],
}

/// Errors from `generate_team`.
///
/// `#[non_exhaustive]` per T1-7 fix-pass (type-design audit P2): variants
/// WILL grow at T2+ (e.g. `NamingPatternMalformed`, `CultureCorpusTooSmall`).
/// Marking non-exhaustive now means downstream consumers handle future
/// variants via a catch-all arm without breaking on a SemVer-compatible
/// variant addition.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProcGenError {
    #[error("culture {0:?} not found in content store")]
    MissingCulture(String),
    #[error("tactical archetype {0:?} not found in content store")]
    MissingTacticalArchetype(String),
    #[error("manager archetype {0:?} not found in content store")]
    MissingManagerArchetype(String),
    #[error("culture {0:?} has an empty team_name_bank; cannot pick a team name")]
    EmptyTeamNameBank(String),
    #[error("Markov training failed for culture {culture_id:?}: {source}")]
    MarkovTrainingFailed {
        culture_id: String,
        #[source]
        source: MarkovError,
    },
    #[error("Markov sampling failed: {0}")]
    MarkovSamplingFailed(#[source] MarkovError),
}

/// Named inputs for [`generate_team`]. Replaces the prior 4-positional-`&str`
/// signature per T1-7 type-design audit P1-3 — three of the four arguments
/// were the same type (`&str`), inviting argument-order bugs at call sites.
/// Cheaper than a full newtype-symmetry refactor (which would touch the
/// existing `TacticalArchetype` struct + fixtures + tests); the newtype
/// extension is deferred to a dedicated row.
#[derive(Debug, Clone)]
pub struct ProcGenInputs<'a> {
    pub culture_id: &'a str,
    pub tactical_archetype_id: &'a str,
    pub manager_archetype_id: &'a str,
    pub seed: Seed,
    /// Team names already taken within this league (from clubs sharing the
    /// same culture). `generate_team` will re-draw on the same ChaCha8
    /// stream to avoid a collision; see the team-name-dedup section below.
    ///
    /// Pass `None` when generating a single isolated team (e.g. tests,
    /// career-init single-team path) — no dedup is attempted.
    pub used_team_names: Option<&'a BTreeSet<String>>,
}

// ---------------------------------------------------------------------------
// generate_team
// ---------------------------------------------------------------------------

/// Generate a full team (22 player names + manager name + team name)
/// deterministically from a single seed.
///
/// # Arguments
///
/// - `content` — the loaded content store.
/// - `inputs` — [`ProcGenInputs`] bundling the 3 reference IDs + seed.
///   T1-7 fix-pass per type-design P1-3: was 4 positional `&str` args; 3
///   of them same-typed invited argument-order bugs.
///
/// # Determinism
///
/// All draws use a single `ChaCha8Rng` seeded via
/// `seed_fn(seed.to_u64(), 0, SeedLayer::ContentBake, 0)`. The sequence is
/// fully determined by `seed` + the content store contents.
pub fn generate_team(
    content: &ContentStore,
    inputs: ProcGenInputs<'_>,
) -> Result<ProcGenTeam, ProcGenError> {
    let ProcGenInputs {
        culture_id,
        tactical_archetype_id,
        manager_archetype_id,
        seed,
        used_team_names,
    } = inputs;

    // --- Validate references ---
    let culture = content
        .cultures
        .get(culture_id)
        .ok_or_else(|| ProcGenError::MissingCulture(culture_id.to_owned()))?;

    if !content
        .tactical_archetypes
        .contains_key(tactical_archetype_id)
    {
        return Err(ProcGenError::MissingTacticalArchetype(
            tactical_archetype_id.to_owned(),
        ));
    }

    if !content.managers.contains_key(manager_archetype_id) {
        return Err(ProcGenError::MissingManagerArchetype(
            manager_archetype_id.to_owned(),
        ));
    }

    if culture.team_name_bank.is_empty() {
        return Err(ProcGenError::EmptyTeamNameBank(culture_id.to_owned()));
    }

    // --- Train Markov chain on culture's combined name corpus ---
    // Combine first_name_bank + last_name_bank as one training set.
    // The chain is used for both first and last name generation; the
    // statistical character of first vs last names in English is different
    // enough that a combined chain still produces reasonable outputs for
    // a T1 stub. T2 can split into two chains if quality requires it.
    let mut combined_corpus: Vec<String> =
        Vec::with_capacity(culture.first_name_bank.len() + culture.last_name_bank.len());
    for name in &culture.first_name_bank {
        // Lowercase normalisation happens inside MarkovNameChain::train;
        // we pass as-is.
        combined_corpus.push(name.clone());
    }
    for name in &culture.last_name_bank {
        combined_corpus.push(name.clone());
    }

    let chain = MarkovNameChain::train(&combined_corpus).map_err(|e| {
        ProcGenError::MarkovTrainingFailed {
            culture_id: culture_id.to_owned(),
            source: e,
        }
    })?;

    // --- Seed a single RNG for all draws in this call ---
    let rng_seed = seed_fn(seed.to_u64(), 0, SeedLayer::ContentBake, 0);
    let mut rng = ChaCha8Rng::seed_from_u64(rng_seed);

    // --- Pick team name (index into team_name_bank; NOT Markov) ---
    //
    // S9 dedup: if `used_team_names` is supplied, re-draw on the same RNG
    // stream until we find a name not in the used set.
    //
    // Within-league club-name dedup (terminates even when a culture's bank is
    // smaller than the number of clubs sharing it):
    //   Draw one initial index. If it is free (or no used-set was supplied),
    //   take it. Otherwise scan the bank DETERMINISTICALLY forward from the
    //   next index (wrapping) and take the first free name. The scan consumes
    //   NO extra RNG, so downstream draws (manager + players) stay a pure
    //   function of the seed regardless of how many names were already taken,
    //   and the bank is treated as exhausted only when ALL bank_len entries
    //   are genuinely taken (a random re-draw could suffix prematurely while a
    //   free name still existed).
    //   On true exhaustion, fall back to a deterministic " (N)" disambiguator
    //   on the initial name (N = used.len() + 1), guaranteeing uniqueness.
    let bank_len = culture.team_name_bank.len();
    let team_name_idx = rng.gen_range(0..bank_len);
    let team_name = {
        let initial_name = &culture.team_name_bank[team_name_idx];
        match used_team_names {
            None => initial_name.clone(),
            Some(used) if !used.contains(initial_name.as_str()) => initial_name.clone(),
            Some(used) => {
                // Deterministic forward scan over the rest of the bank (NO
                // extra RNG): the first free name wins; the bank is exhausted
                // only if every one of its bank_len entries is taken.
                let mut found: Option<String> = None;
                for offset in 1..bank_len {
                    let candidate_idx = (team_name_idx + offset) % bank_len;
                    let candidate = &culture.team_name_bank[candidate_idx];
                    if !used.contains(candidate.as_str()) {
                        found = Some(candidate.clone());
                        break;
                    }
                }
                match found {
                    Some(name) => name,
                    None => {
                        // All bank_len candidates collided.  Append a
                        // deterministic suffix to the initial name to
                        // guarantee uniqueness without further RNG draws.
                        // N = number of clubs already using this culture =
                        // used.len() + 1 (the current club would be the
                        // (used.len()+1)-th club assigned to this culture).
                        // Using `used.len() + 1` is deterministic for a
                        // given (seed, league-generation-order) pair.
                        let ordinal = used.len() + 1;
                        format!("{} ({})", initial_name, ordinal)
                    }
                }
            }
        }
    };

    // --- Sample manager name ---
    let manager_first = chain
        .sample(&mut rng)
        .map_err(ProcGenError::MarkovSamplingFailed)?;
    let manager_last = chain
        .sample(&mut rng)
        .map_err(ProcGenError::MarkovSamplingFailed)?;
    let manager = ManagerName {
        first: manager_first,
        last: manager_last,
    };

    // --- Sample 22 player names ---
    // Build an array via a collect into a Vec, then try_into.
    let mut players_vec: Vec<PlayerName> = Vec::with_capacity(22);
    for _ in 0..22 {
        let first = chain
            .sample(&mut rng)
            .map_err(ProcGenError::MarkovSamplingFailed)?;
        let last = chain
            .sample(&mut rng)
            .map_err(ProcGenError::MarkovSamplingFailed)?;
        players_vec.push(PlayerName { first, last });
    }

    // SAFETY: we pushed exactly 22 elements above.
    let players: [PlayerName; 22] = players_vec
        .try_into()
        .expect("pushed exactly 22 PlayerNames");

    Ok(ProcGenTeam {
        team_name,
        manager,
        players,
    })
}

// ---------------------------------------------------------------------------
// Build a per-culture Markov chain (public helper for tests + T2 reuse)
// ---------------------------------------------------------------------------

/// Train a `MarkovNameChain` on the combined name corpus of a culture.
///
/// Combines `first_name_bank` + `last_name_bank` as the training set.
/// Public for integration tests + future T2 consumers.
pub fn train_culture_chain(culture: &crate::Culture) -> Result<MarkovNameChain, ProcGenError> {
    let mut corpus: Vec<String> =
        Vec::with_capacity(culture.first_name_bank.len() + culture.last_name_bank.len());
    corpus.extend_from_slice(&culture.first_name_bank);
    corpus.extend_from_slice(&culture.last_name_bank);
    MarkovNameChain::train(&corpus).map_err(|e| ProcGenError::MarkovTrainingFailed {
        culture_id: culture.id.clone(),
        source: e,
    })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{ContentStore, Culture, CultureWeights};
    use fw_core::{Q32, Seed};

    fn minimal_store() -> ContentStore {
        use crate::manager::ManagerArchetype;
        use crate::runtime::TacticalArchetype;

        let mut store = ContentStore::default();

        // Add a minimal culture with team_name_bank
        let culture = Culture {
            id: "fwh.core:culture.test".to_string(),
            name: "Test".to_string(),
            first_name_bank: vec![
                "James".into(),
                "William".into(),
                "Oliver".into(),
                "Henry".into(),
                "George".into(),
                "Thomas".into(),
                "Edward".into(),
                "Arthur".into(),
                "Charles".into(),
                "Frederick".into(),
            ],
            last_name_bank: vec![
                "Smith".into(),
                "Jones".into(),
                "Brown".into(),
                "Taylor".into(),
                "Wilson".into(),
                "Davies".into(),
                "Evans".into(),
                "Thomas".into(),
                "Roberts".into(),
                "Johnson".into(),
            ],
            team_name_bank: vec![
                "Northbrook United".into(),
                "Riverside Town".into(),
                "Eastfield City".into(),
            ],
            naming_pattern: "{first} {last}".to_string(),
            weights: CultureWeights::default(),
        };
        store.cultures.insert(culture.id.clone(), culture);

        // Add a tactical archetype
        let archetype = TacticalArchetype {
            id: "fwh.core:archetype.test".to_string(),
            formation: vec![],
            press_radius_metres: 15,
            line_height_metres: None,
            buildup_speed_factor_bps: 10_000,
        };
        store
            .tactical_archetypes
            .insert(archetype.id.clone(), archetype);

        // Add a manager archetype via try_new (T1-7 fix-pass: struct fields
        // are now newtype + validators, no longer raw-String construction).
        let manager = ManagerArchetype::try_new(
            1,
            crate::manager::ManagerArchetypeId::try_new("fwh.core:manager.test").expect("test id"),
            "Test Manager".to_string(),
            "fwh.core:archetype.test".to_string(),
            Q32::from_raw(1_288_490_189),
            Q32::from_raw(1_717_986_918),
        )
        .expect("test manager validates");
        store
            .managers
            .insert(manager.id.as_str().to_owned(), manager);

        store
    }

    #[test]
    fn generate_team_succeeds_with_valid_inputs() {
        let store = minimal_store();
        let result = generate_team(
            &store,
            ProcGenInputs {
                culture_id: "fwh.core:culture.test",
                tactical_archetype_id: "fwh.core:archetype.test",
                manager_archetype_id: "fwh.core:manager.test",
                seed: Seed::from_u64(0x1234),
                used_team_names: None,
            },
        );
        assert!(result.is_ok(), "generate_team should succeed: {:?}", result);
        let team = result.unwrap();
        assert!(!team.team_name.is_empty());
        assert!(!team.manager.first.is_empty());
        assert!(!team.manager.last.is_empty());
        assert_eq!(team.players.len(), 22);
        for (i, p) in team.players.iter().enumerate() {
            assert!(!p.first.is_empty(), "player {i} first name empty");
            assert!(!p.last.is_empty(), "player {i} last name empty");
        }
    }

    #[test]
    fn same_seed_produces_identical_team() {
        let store = minimal_store();
        let seed = Seed::from_u64(0xABCD_1234);
        let a = generate_team(
            &store,
            ProcGenInputs {
                culture_id: "fwh.core:culture.test",
                tactical_archetype_id: "fwh.core:archetype.test",
                manager_archetype_id: "fwh.core:manager.test",
                seed,
                used_team_names: None,
            },
        )
        .expect("generate a");
        let b = generate_team(
            &store,
            ProcGenInputs {
                culture_id: "fwh.core:culture.test",
                tactical_archetype_id: "fwh.core:archetype.test",
                manager_archetype_id: "fwh.core:manager.test",
                seed,
                used_team_names: None,
            },
        )
        .expect("generate b");
        assert_eq!(a.team_name, b.team_name);
        assert_eq!(a.manager.first, b.manager.first);
        assert_eq!(a.manager.last, b.manager.last);
        for i in 0..22 {
            assert_eq!(
                a.players[i].first, b.players[i].first,
                "player {i} first mismatch"
            );
            assert_eq!(
                a.players[i].last, b.players[i].last,
                "player {i} last mismatch"
            );
        }
    }

    #[test]
    fn missing_culture_returns_error() {
        let store = minimal_store();
        let result = generate_team(
            &store,
            ProcGenInputs {
                culture_id: "fwh.core:culture.nonexistent",
                tactical_archetype_id: "fwh.core:archetype.test",
                manager_archetype_id: "fwh.core:manager.test",
                seed: Seed::from_u64(0),
                used_team_names: None,
            },
        );
        assert!(
            matches!(result, Err(ProcGenError::MissingCulture(_))),
            "expected MissingCulture, got: {:?}",
            result
        );
    }

    #[test]
    fn missing_tactical_archetype_returns_error() {
        let store = minimal_store();
        let result = generate_team(
            &store,
            ProcGenInputs {
                culture_id: "fwh.core:culture.test",
                tactical_archetype_id: "fwh.core:archetype.nonexistent",
                manager_archetype_id: "fwh.core:manager.test",
                seed: Seed::from_u64(0),
                used_team_names: None,
            },
        );
        assert!(
            matches!(result, Err(ProcGenError::MissingTacticalArchetype(_))),
            "expected MissingTacticalArchetype, got: {:?}",
            result
        );
    }

    #[test]
    fn missing_manager_archetype_returns_error() {
        let store = minimal_store();
        let result = generate_team(
            &store,
            ProcGenInputs {
                culture_id: "fwh.core:culture.test",
                tactical_archetype_id: "fwh.core:archetype.test",
                manager_archetype_id: "fwh.core:manager.nonexistent",
                seed: Seed::from_u64(0),
                used_team_names: None,
            },
        );
        assert!(
            matches!(result, Err(ProcGenError::MissingManagerArchetype(_))),
            "expected MissingManagerArchetype, got: {:?}",
            result
        );
    }

    #[test]
    fn empty_team_name_bank_returns_error() {
        let mut store = minimal_store();
        store
            .cultures
            .get_mut("fwh.core:culture.test")
            .unwrap()
            .team_name_bank
            .clear();
        let result = generate_team(
            &store,
            ProcGenInputs {
                culture_id: "fwh.core:culture.test",
                tactical_archetype_id: "fwh.core:archetype.test",
                manager_archetype_id: "fwh.core:manager.test",
                seed: Seed::from_u64(0),
                used_team_names: None,
            },
        );
        assert!(
            matches!(result, Err(ProcGenError::EmptyTeamNameBank(_))),
            "expected EmptyTeamNameBank, got: {:?}",
            result
        );
    }
}

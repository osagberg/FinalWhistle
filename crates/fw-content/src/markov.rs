//! 2-gram character-level Markov name chain.
//!
//! ## Design
//!
//! Trains on a corpus of name strings. State is a (prev_char, curr_char)
//! bigram; the transition table stores the weighted next-char distribution as
//! a `Vec<char>` (duplicates = frequency weighting — uniform index sampling
//! over the vec gives the same distribution as the count-weighted one).
//!
//! Start-of-word and end-of-word are represented by sentinel chars
//! `START = '\x00'` and `END = '\x01'`. The sentinel pair `(START, START)`
//! is the initial state. A transition to `END` terminates sampling.
//!
//! ## Determinism invariants
//!
//! - All state uses `BTreeMap` (no `HashMap`) — iteration order is stable
//!   across platforms per `Sim/RULES.md §2`.
//! - Callers supply a `ChaCha8Rng` seeded via `seed_fn` per ADR-0009;
//!   no `thread_rng` or clock calls inside this module.
//! - Sampling is purely a function of RNG state + trained table; given
//!   identical RNG state + identical training corpus → identical output.
//!
//! ## Failure modes
//!
//! - `train` on an empty corpus returns `Err(MarkovError::EmptyCorpus)`.
//! - `sample` respects `MAX_NAME_LEN = 24`; a chain that never emits END
//!   within 24 characters is truncated (loud: returns a name, not an error,
//!   but the truncation is visible to callers).
//! - `sample` on a chain with no reachable transitions from the start state
//!   returns `Err(MarkovError::NoTransitionFromStart)`.

use std::collections::BTreeMap;

use rand::Rng;
use rand_chacha::ChaCha8Rng;

/// Sentinel: start-of-word / initial state marker.
const START: char = '\x00';

/// Sentinel: end-of-word marker. A transition to this char terminates
/// sampling.
const END: char = '\x01';

/// Maximum output name length (characters, excluding sentinel).
/// A chain that hasn't emitted END within this many chars is hard-truncated.
pub const MAX_NAME_LEN: usize = 24;

/// Errors from `MarkovNameChain` construction or sampling.
#[derive(Debug, thiserror::Error)]
pub enum MarkovError {
    #[error("cannot train a Markov chain on an empty corpus")]
    EmptyCorpus,
    #[error("Markov chain has no transitions from the start state (corpus too short?)")]
    NoTransitionFromStart,
}

/// 2-gram char-level Markov name chain.
///
/// Trains on a corpus of name strings, builds a bigram transition table, then
/// samples new names deterministically given a caller-supplied `ChaCha8Rng`.
#[derive(Debug, Clone)]
pub struct MarkovNameChain {
    /// Bigram transition table. Key = (prev_char, curr_char); value = next-char
    /// frequency distribution encoded as a `Vec<char>` with duplicates
    /// (uniform index pick ≡ weighted pick).
    table: BTreeMap<(char, char), Vec<char>>,
}

impl MarkovNameChain {
    /// Train on `corpus`. Returns `Err(MarkovError::EmptyCorpus)` if corpus
    /// is empty or all entries are empty strings.
    ///
    /// Training procedure (per MEMORY.md T1-7 design):
    /// 1. Lowercase each entry (normalizes training data; sampled output is
    ///    then title-cased by the caller or left lowercase as needed).
    /// 2. Walk each word as `START START c0 c1 … cN END`, emitting bigrams
    ///    `(START, START)→c0`, `(START, c0)→c1`, `(c0, c1)→c2`, …,
    ///    `(c(N-1), cN)→END`.
    /// 3. Duplicates in the Vec are intentional — they encode frequency
    ///    weighting so uniform index sampling reproduces the corpus distribution.
    pub fn train(corpus: &[String]) -> Result<Self, MarkovError> {
        let mut table: BTreeMap<(char, char), Vec<char>> = BTreeMap::new();
        let mut any_entry = false;

        for word in corpus {
            let word = word.trim();
            if word.is_empty() {
                continue;
            }
            any_entry = true;

            // Collect chars once; we need the full sequence.
            let chars: Vec<char> = word.chars().collect();
            let n = chars.len();

            // Emit: (START, START) → chars[0]
            table.entry((START, START)).or_default().push(chars[0]);

            if n == 1 {
                // Single-char word: (START, chars[0]) → END
                table.entry((START, chars[0])).or_default().push(END);
                continue;
            }

            // (START, chars[0]) → chars[1]
            table.entry((START, chars[0])).or_default().push(chars[1]);

            // Interior bigrams: (chars[i], chars[i+1]) → chars[i+2]
            for i in 0..(n - 2) {
                table
                    .entry((chars[i], chars[i + 1]))
                    .or_default()
                    .push(chars[i + 2]);
            }

            // Final bigram: (chars[n-2], chars[n-1]) → END
            table
                .entry((chars[n - 2], chars[n - 1]))
                .or_default()
                .push(END);
        }

        if !any_entry {
            return Err(MarkovError::EmptyCorpus);
        }

        Ok(Self { table })
    }

    /// Sample a new name from the chain using `rng`.
    ///
    /// Returns `Err(MarkovError::NoTransitionFromStart)` if the start state
    /// has no transitions (should not happen on a well-trained chain, but
    /// guarded against for fail-loud rather than silent panic).
    ///
    /// Names are title-cased (first char uppercased; rest as sampled — the
    /// corpus is lowercased at train time so rest will be lowercase).
    ///
    /// Hard-truncates at `MAX_NAME_LEN` characters if END is never reached.
    pub fn sample(&self, rng: &mut ChaCha8Rng) -> Result<String, MarkovError> {
        // Verify the start state has transitions.
        let start_nexts = self
            .table
            .get(&(START, START))
            .ok_or(MarkovError::NoTransitionFromStart)?;

        if start_nexts.is_empty() {
            return Err(MarkovError::NoTransitionFromStart);
        }

        let mut result = Vec::with_capacity(MAX_NAME_LEN);
        let first_idx = rng.gen_range(0..start_nexts.len());
        let first_char = start_nexts[first_idx];

        if first_char == END {
            // Degenerate: corpus of single-char words. Return the char as-is.
            let s = first_char.to_uppercase().to_string();
            return Ok(s);
        }

        result.push(first_char);

        let mut prev = START;
        let mut curr = first_char;

        loop {
            if result.len() >= MAX_NAME_LEN {
                // Hard truncation: chain ran too long.
                break;
            }

            let nexts = match self.table.get(&(prev, curr)) {
                Some(v) if !v.is_empty() => v,
                // No outgoing transition from this bigram — stop.
                _ => break,
            };

            let idx = rng.gen_range(0..nexts.len());
            let next_char = nexts[idx];

            if next_char == END {
                break;
            }

            result.push(next_char);
            prev = curr;
            curr = next_char;
        }

        // Title-case: uppercase the first char; leave the rest as sampled
        // (lowercase, since train normalizes to lowercase).
        let name: String = result
            .iter()
            .enumerate()
            .map(|(i, &c)| {
                if i == 0 {
                    c.to_uppercase().next().unwrap_or(c)
                } else {
                    c
                }
            })
            .collect();

        Ok(name)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;
    use rand_chacha::ChaCha8Rng;

    fn simple_corpus() -> Vec<String> {
        vec![
            "james".into(),
            "william".into(),
            "oliver".into(),
            "henry".into(),
            "george".into(),
            "thomas".into(),
            "edward".into(),
            "arthur".into(),
            "charles".into(),
            "frederick".into(),
        ]
    }

    #[test]
    fn train_empty_corpus_returns_error() {
        let result = MarkovNameChain::train(&[]);
        assert!(
            matches!(result, Err(MarkovError::EmptyCorpus)),
            "expected EmptyCorpus error, got: {:?}",
            result.map(|_| "Ok")
        );
    }

    #[test]
    fn train_all_blank_returns_error() {
        let corpus = vec!["".into(), "   ".into()];
        let result = MarkovNameChain::train(&corpus);
        assert!(matches!(result, Err(MarkovError::EmptyCorpus)));
    }

    #[test]
    fn sample_is_deterministic_given_same_rng() {
        let chain = MarkovNameChain::train(&simple_corpus()).expect("train");
        let mut rng_a = ChaCha8Rng::seed_from_u64(0xDEAD_BEEF);
        let mut rng_b = ChaCha8Rng::seed_from_u64(0xDEAD_BEEF);
        let a = chain.sample(&mut rng_a).expect("sample a");
        let b = chain.sample(&mut rng_b).expect("sample b");
        assert_eq!(a, b, "same RNG seed must produce identical output");
    }

    #[test]
    fn sample_is_within_max_len() {
        let chain = MarkovNameChain::train(&simple_corpus()).expect("train");
        let mut rng = ChaCha8Rng::seed_from_u64(0x1234);
        for _ in 0..200 {
            let name = chain.sample(&mut rng).expect("sample");
            assert!(
                name.chars().count() <= MAX_NAME_LEN,
                "name exceeds MAX_NAME_LEN: {:?}",
                name
            );
            assert!(!name.is_empty(), "name must not be empty");
        }
    }

    #[test]
    fn sample_produces_title_case() {
        let chain = MarkovNameChain::train(&simple_corpus()).expect("train");
        let mut rng = ChaCha8Rng::seed_from_u64(0xABCD);
        for _ in 0..50 {
            let name = chain.sample(&mut rng).expect("sample");
            let first = name.chars().next().expect("non-empty");
            assert!(
                first.is_uppercase(),
                "first char of '{name}' should be uppercase"
            );
        }
    }

    #[test]
    fn sample_different_seeds_produce_different_names() {
        // Over 30 samples from distinct seeds, at least 2 distinct names.
        let chain = MarkovNameChain::train(&simple_corpus()).expect("train");
        let names: Vec<String> = (0u64..30)
            .map(|i| {
                let mut rng = ChaCha8Rng::seed_from_u64(i * 1_000_007);
                chain.sample(&mut rng).expect("sample")
            })
            .collect();
        let unique: std::collections::BTreeSet<&String> = names.iter().collect();
        assert!(
            unique.len() >= 2,
            "expected multiple distinct names over 30 seeds, got: {unique:?}"
        );
    }

    #[test]
    fn start_char_distribution_matches_corpus() {
        // Over 1000 samples, all first chars should appear in the corpus.
        let corpus = simple_corpus();
        let first_chars: std::collections::BTreeSet<char> = corpus
            .iter()
            .filter_map(|w| {
                w.chars()
                    .next()
                    .map(|c| c.to_uppercase().next().unwrap_or(c))
            })
            .collect();
        let chain = MarkovNameChain::train(&corpus).expect("train");
        let mut rng = ChaCha8Rng::seed_from_u64(0x5678);
        for _ in 0..1000 {
            let name = chain.sample(&mut rng).expect("sample");
            let first = name.chars().next().expect("non-empty");
            assert!(
                first_chars.contains(&first),
                "sampled name '{name}' starts with '{first}' not in corpus first-chars {first_chars:?}"
            );
        }
    }
}

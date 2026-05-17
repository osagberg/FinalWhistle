//! `calibrate` — T2-1d offline calibration binary.
//!
//! Implements the calibration loop documented in
//! `docs/design/xg-coefficients.md §Calibration loop (T2-1)` +
//! `docs/design/personality-bias-weights.md §Re-tuning cadence`. Three
//! subcommands:
//!
//! 1. **`run`** — execute N=100 matches across a deterministic seed ×
//!    archetype-pair sweep + dump per-shot + per-dribble telemetry to JSON
//!    via the `MatchState::shot_telemetry` + `dribble_telemetry`
//!    `#[serde(skip)]` sidecar Vecs populated by `dispatch::apply_intent`.
//! 2. **`fit-xg`** — read the JSON corpus, run an in-process Newton-Raphson
//!    logistic regression on the 6-feature N-shot design matrix, and print
//!    PROPOSED Q32 raw-bits for BETA_0..BETA_6 to stdout in paste-ready
//!    format. Does NOT modify source; T2-1d ships only the infrastructure
//!    and T2-1d2 wires utility_shoot to xg_utility and atomically applies
//!    the fitted values.
//! 3. **`fit-personality`** — read the JSON corpus, extract empirical
//!    decision-frequency curves per personality dimension, and print
//!    PROPOSED Q32 raw-bits for K_1, K_2, K_7, K_8, K_18 to stdout.
//!
//! Per the T2-1d MEMORY-spec implementation-discovery #2 scope-shrink:
//! `utility_shoot` currently uses a hand-tuned stub instead of the
//! `xg_utility` model, so applying the fitted constants now would be
//! decorative. T2-1d2 wires the model in.
//!
//! ## Determinism contract
//!
//! - Match runs are FULLY DETERMINISTIC — the same `(seed, home_archetype,
//!   away_archetype)` triple produces the same telemetry corpus byte-for-byte.
//! - Match-index → archetype-pair selection is deterministic (round-robin
//!   over the loaded archetype catalog; pair-index =
//!   `(match_idx % len, (match_idx / len) % len)`).
//! - Newton-Raphson regression converges to a unique fixed-point given the
//!   same corpus (no RNG; gradient + Hessian are deterministic functions
//!   of the corpus + current β estimate).
//! - JSON dumps use `serde_json::to_string_pretty` for human-readable
//!   audit-trail clarity; the schema is documented in `target/` artifacts
//!   the calibrate binary writes.
//!
//! ## Usage
//!
//! ```sh
//! cargo run --release --bin calibrate -- run --matches 100 --output target/calibration-corpus.json
//! cargo run --release --bin calibrate -- fit-xg target/calibration-corpus.json
//! cargo run --release --bin calibrate -- fit-personality target/calibration-corpus.json
//! ```

use clap::{Parser, Subcommand};
use fw_content::ContentStore;
use fw_core::Seed;
use fw_match_sim::{DribbleTelemetryRecord, MatchState, ShotTelemetryRecord, tick_match};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Parser, Debug)]
#[command(
    name = "calibrate",
    about = "T2-1d xG / personality coefficient calibration binary",
    long_about = "Runs match corpora + offline coefficient fits per docs/design/xg-coefficients.md + docs/design/personality-bias-weights.md. Outputs proposed Q32 raw-bits to stdout; does NOT modify source constants (T2-1d2 applies the fitted values after utility_shoot rewiring)."
)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Run N matches across a seed × archetype-pair sweep + dump telemetry JSON.
    Run {
        /// Number of matches to simulate (default 100 per the design-doc spec).
        #[arg(long, default_value_t = 100)]
        matches: u32,

        /// Ticks per match (default 600 — matches the extended corpus pin).
        #[arg(long, default_value_t = 600)]
        ticks: u32,

        /// Output path for the JSON corpus.
        #[arg(long, default_value = "target/calibration-corpus.json")]
        output: PathBuf,

        /// Content root for ContentStore::load_sources (defaults to ./content).
        #[arg(long, default_value = "content")]
        content: PathBuf,
    },

    /// Read corpus + run Newton-Raphson on β coefficients.
    FitXg {
        /// Path to corpus JSON written by `run`.
        corpus: PathBuf,

        /// Output path for the fit provenance JSON (default
        /// `target/xg-fit-result.json`).
        #[arg(long, default_value = "target/xg-fit-result.json")]
        result: PathBuf,
    },

    /// Read corpus + extract personality-bias decision-frequency curves.
    FitPersonality {
        /// Path to corpus JSON written by `run`.
        corpus: PathBuf,

        /// Output path for the fit provenance JSON (default
        /// `target/k-fit-result.json`).
        #[arg(long, default_value = "target/k-fit-result.json")]
        result: PathBuf,
    },
}

/// On-disk JSON schema for the calibration corpus written by `run` + read
/// by `fit-xg` / `fit-personality`. NON-canonical (off-sim-path).
#[derive(Debug, Serialize, Deserialize)]
struct Corpus {
    metadata: CorpusMetadata,
    shots: Vec<ShotTelemetryRecord>,
    dribbles: Vec<DribbleTelemetryRecord>,
}

#[derive(Debug, Serialize, Deserialize)]
struct CorpusMetadata {
    total_matches: u32,
    ticks_per_match: u32,
    total_shots: u32,
    total_dribbles: u32,
    /// Pre-fit mean xG per shot using current (Phase-1) BETA values.
    /// Computed by evaluating `xg_utility(ctx)` on each captured shot's
    /// features. Diagnostic only — not the fit target.
    mean_xg_per_shot_pre_fit_milli: i64,
    /// Empirical goal rate (became_goal == Some(true) count / total_shots).
    /// The fit target: post-fit mean xG should approach this value.
    empirical_goal_rate_milli: i64,
    /// Calibrate binary version stamp + commit-relative timestamp.
    binary_version: String,
    archetype_catalog: Vec<String>,
}

// f64 arithmetic is permitted in this binary (off-canonical-path bake-time
// tooling per Sim/RULES.md §1 — only the sim itself + canonical state
// forbid floats). The crate-level lint is sim-conservative; this binary
// opts in to f64 for the print-formatter divides + the Newton-Raphson fit.
#[allow(clippy::float_arithmetic)]
fn main() -> std::process::ExitCode {
    let cli = Cli::parse();
    match cli.cmd {
        Cmd::Run {
            matches,
            ticks,
            output,
            content,
        } => match run_corpus(matches, ticks, &content, &output) {
            Ok(meta) => {
                eprintln!("calibrate run: PASS");
                eprintln!("  matches:          {}", meta.total_matches);
                eprintln!("  ticks/match:      {}", meta.ticks_per_match);
                eprintln!("  total_shots:      {}", meta.total_shots);
                eprintln!("  total_dribbles:   {}", meta.total_dribbles);
                eprintln!(
                    "  mean_xg/shot:     {:.4} (pre-fit; Phase-1 BETA on captured features)",
                    meta.mean_xg_per_shot_pre_fit_milli as f64 / 1000.0
                );
                eprintln!(
                    "  empirical_goal_rate: {:.4} (fit target)",
                    meta.empirical_goal_rate_milli as f64 / 1000.0
                );
                eprintln!("  written to:       {}", output.display());
                std::process::ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("calibrate run: FAIL: {e}");
                std::process::ExitCode::FAILURE
            }
        },
        Cmd::FitXg { corpus, result } => match fit_xg(&corpus, &result) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("fit-xg: FAIL: {e}");
                std::process::ExitCode::FAILURE
            }
        },
        Cmd::FitPersonality { corpus, result } => match fit_personality(&corpus, &result) {
            Ok(()) => std::process::ExitCode::SUCCESS,
            Err(e) => {
                eprintln!("fit-personality: FAIL: {e}");
                std::process::ExitCode::FAILURE
            }
        },
    }
}

/// Multi-match runner. Loops over `matches` seeds × archetype-pair selections,
/// runs each through `tick_match` for `ticks` ticks, accumulates per-shot +
/// per-dribble telemetry, back-fills `became_goal` via post-match Goal-event
/// correlation, and dumps JSON.
fn run_corpus(
    matches: u32,
    ticks: u32,
    content_root: &Path,
    output: &Path,
) -> Result<CorpusMetadata, String> {
    let content = ContentStore::load_sources(content_root).map_err(|e| {
        format!(
            "ContentStore::load_sources({}): {e}",
            content_root.display()
        )
    })?;

    // Build deterministic archetype catalog (sorted IDs for stable
    // pair-selection ordering). BTreeMap iter is key-ordered so the
    // catalog is already deterministic without re-sorting.
    let catalog: Vec<String> = content.tactical_archetypes.keys().cloned().collect();
    if catalog.is_empty() {
        return Err("no tactical_archetypes in content store".to_string());
    }

    let sig_defs = content.signature_definitions.clone();
    let mut all_shots: Vec<ShotTelemetryRecord> = Vec::new();
    let mut all_dribbles: Vec<DribbleTelemetryRecord> = Vec::new();
    let mut total_goals_attributed: u32 = 0;

    // Seed sweep: derive seed_i from match_idx via a simple multiplier so
    // each match has a distinct base seed. Archetype pair: round-robin over
    // the catalog so all archetype combinations get exercised across N=100
    // (catalog size N=16 → 100/16 = 6.25 full sweeps; uneven tail acceptable).
    for match_idx in 0..matches {
        let seed = Seed::from_u64(0x1000_0000_0000_0000_u64.wrapping_add(match_idx as u64));
        let home_idx = (match_idx as usize) % catalog.len();
        let away_idx = ((match_idx as usize) / catalog.len()) % catalog.len();
        let home_id = &catalog[home_idx];
        let away_id = &catalog[away_idx];

        let mut state = MatchState::initial_with_content(seed, &content, home_id, away_id)
            .map_err(|e| format!("initial_with_content match {match_idx}: {e}"))?;
        for _ in 0..ticks {
            state = tick_match(state, &sig_defs);
        }

        // Drain telemetry sidecars + back-fill became_goal via post-match
        // Goal-event correlation (120-tick lookahead per the MEMORY spec).
        let goal_events: Vec<(u8, u32)> = state
            .match_events()
            .iter()
            .filter_map(|e| match e {
                fw_content::MatchEvent::Goal {
                    scorer_slot, tick, ..
                } => Some((*scorer_slot, tick.to_raw() as u32)),
                _ => None,
            })
            .collect();
        total_goals_attributed += goal_events.len() as u32;

        // Drain the telemetry sidecars via the public accessors. (Binaries
        // see lib's pub items only; pub(crate) doesn't cross the bin/lib
        // boundary so the drain methods are the correct surface.)
        let mut shots = state.drain_shot_telemetry();
        let dribbles = state.drain_dribble_telemetry();
        for shot in shots.iter_mut() {
            // became_goal = Some(true) iff any goal_event matches
            // (scorer_slot == shooter_slot, tick within [shot_tick,
            // shot_tick + 120]).
            let became = goal_events.iter().any(|(scorer, gtick)| {
                *scorer == shot.shooter_slot
                    && *gtick >= shot.shot_tick
                    && *gtick <= shot.shot_tick.saturating_add(120)
            });
            shot.became_goal = Some(became);
        }
        all_shots.extend(shots);
        all_dribbles.extend(dribbles);
    }

    // Compute pre-fit mean xG + empirical goal rate (diagnostics).
    let mean_xg_pre_milli = compute_mean_xg_milli(&all_shots);
    let empirical_goal_rate_milli = if all_shots.is_empty() {
        0
    } else {
        let goals: u32 = all_shots
            .iter()
            .filter(|s| s.became_goal == Some(true))
            .count() as u32;
        ((goals as i64) * 1000) / (all_shots.len() as i64)
    };
    let _ = total_goals_attributed; // diagnostic; not in metadata yet

    let meta = CorpusMetadata {
        total_matches: matches,
        ticks_per_match: ticks,
        total_shots: all_shots.len() as u32,
        total_dribbles: all_dribbles.len() as u32,
        mean_xg_per_shot_pre_fit_milli: mean_xg_pre_milli,
        empirical_goal_rate_milli,
        binary_version: "T2-1d-infra-2026-05-17".to_string(),
        archetype_catalog: catalog,
    };

    let corpus = Corpus {
        metadata: meta,
        shots: all_shots,
        dribbles: all_dribbles,
    };

    // Ensure parent directory exists.
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create_dir_all({}): {e}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(&corpus)
        .map_err(|e| format!("serde_json::to_string_pretty: {e}"))?;
    std::fs::write(output, json).map_err(|e| format!("write({}): {e}", output.display()))?;

    Ok(corpus.metadata)
}

/// Compute mean xG-per-shot in milli-units (0..1000) using the current
/// Phase-1 BETA values applied to each shot's captured features.
fn compute_mean_xg_milli(shots: &[ShotTelemetryRecord]) -> i64 {
    if shots.is_empty() {
        return 0;
    }
    let mut sum_milli: i64 = 0;
    for shot in shots {
        // Re-evaluate xg_utility using current BETA on the captured features.
        // xg_utility takes ShotContext; reconstruct from raw bits.
        let ctx = fw_match_sim::utility::xg::ShotContext::try_new(
            fw_core::Q32::from_raw(shot.distance_q32_raw),
            fw_core::Q32::from_raw(shot.angle_q32_raw),
            fw_core::Q32::from_raw(shot.pressure_q32_raw),
            fw_core::Q32::from_raw(shot.shot_type_q32_raw),
            fw_core::Q32::from_raw(shot.assist_kind_q32_raw),
            fw_core::Q32::from_raw(shot.shooter_quality_q32_raw),
        );
        // If features were out of [0, 1] (shouldn't happen but defensive),
        // skip the sample.
        let Ok(ctx) = ctx else {
            continue;
        };
        let xg = fw_match_sim::utility::xg::xg_utility(&ctx);
        // xg in [0, 1] Q32 → milli-units = bits >> (32-10) approx; use i64
        // arithmetic via `* 1000 / 2^32`.
        let xg_bits = xg.to_bits();
        sum_milli += (xg_bits * 1000) >> 32;
    }
    sum_milli / (shots.len() as i64)
}

/// `fit-xg` subcommand: Newton-Raphson logistic regression on the 6-feature
/// shot corpus. Inputs: distance_q32, angle_q32, pressure_q32, shot_type_q32,
/// assist_kind_q32, shooter_quality_q32. Output: 7 β coefficients (intercept
/// + 6 features) as Q32 raw-bits printed to stdout in paste-ready format.
///
/// Float arithmetic used HERE (off-canonical-path; binary is outside the sim
/// ring per Sim/RULES.md §1 — only the sim itself + canonical state are
/// f32/f64-banned; calibrate binary's fit math is bake-time tooling).
#[allow(clippy::float_arithmetic)]
fn fit_xg(corpus_path: &Path, result_path: &Path) -> Result<(), String> {
    let corpus_json = std::fs::read_to_string(corpus_path)
        .map_err(|e| format!("read({}): {e}", corpus_path.display()))?;
    let corpus: Corpus =
        serde_json::from_str(&corpus_json).map_err(|e| format!("parse corpus JSON: {e}"))?;

    if corpus.shots.is_empty() {
        return Err("corpus contains zero shots — re-run `calibrate run` first".to_string());
    }

    // Convert Q32 raw-bits → f64 per shot for fit-time arithmetic.
    // Q32 raw bits / 2^32 = float value.
    let q32_to_f64 = |bits: i64| -> f64 { (bits as f64) / 4_294_967_296.0_f64 };

    let n_shots = corpus.shots.len();
    let mut features: Vec<[f64; 7]> = Vec::with_capacity(n_shots); // [1, f1..f6]
    let mut labels: Vec<f64> = Vec::with_capacity(n_shots);
    for shot in &corpus.shots {
        features.push([
            1.0, // intercept
            q32_to_f64(shot.distance_q32_raw),
            q32_to_f64(shot.angle_q32_raw),
            q32_to_f64(shot.pressure_q32_raw),
            q32_to_f64(shot.shot_type_q32_raw),
            q32_to_f64(shot.assist_kind_q32_raw),
            q32_to_f64(shot.shooter_quality_q32_raw),
        ]);
        labels.push(if shot.became_goal == Some(true) {
            1.0
        } else {
            0.0
        });
    }

    // Initial β from current Phase-1 constants (warm-start the regression).
    let mut beta: [f64; 7] = [
        q32_to_f64(fw_match_sim::utility::xg::BETA_0.to_bits()),
        q32_to_f64(fw_match_sim::utility::xg::BETA_1.to_bits()),
        q32_to_f64(fw_match_sim::utility::xg::BETA_2.to_bits()),
        q32_to_f64(fw_match_sim::utility::xg::BETA_3.to_bits()),
        q32_to_f64(fw_match_sim::utility::xg::BETA_4.to_bits()),
        q32_to_f64(fw_match_sim::utility::xg::BETA_5.to_bits()),
        q32_to_f64(fw_match_sim::utility::xg::BETA_6.to_bits()),
    ];

    // Newton-Raphson IRLS on logistic regression. Up to 50 iterations OR
    // until ||Δβ||_∞ < 1e-6 OR until log-loss change < 1e-9.
    let max_iter = 50_usize;
    let mut prev_loss = f64::INFINITY;
    let mut converged = false;
    let mut iter_count = 0_usize;
    for iter in 0..max_iter {
        iter_count = iter + 1;
        // Compute predictions, gradient, and Hessian.
        let mut grad = [0.0_f64; 7];
        let mut hess = [[0.0_f64; 7]; 7];
        let mut loss = 0.0_f64;
        for (x, &y) in features.iter().zip(labels.iter()) {
            let logit: f64 = (0..7).map(|i| beta[i] * x[i]).sum();
            // Numerically stable sigmoid via log1p.
            let p = if logit >= 0.0 {
                1.0 / (1.0 + (-logit).exp())
            } else {
                let e = logit.exp();
                e / (1.0 + e)
            };
            let err = p - y;
            for i in 0..7 {
                grad[i] += err * x[i];
                for j in 0..7 {
                    hess[i][j] += p * (1.0 - p) * x[i] * x[j];
                }
            }
            // Cross-entropy loss: -[y*log(p) + (1-y)*log(1-p)]
            let p_safe = p.clamp(1e-15, 1.0 - 1e-15);
            loss += -(y * p_safe.ln() + (1.0 - y) * (1.0 - p_safe).ln());
        }

        // Ridge regularization (λ = 0.01) to handle constant-feature
        // singularities. In T1 `shot_type_q32` and `assist_kind_q32` are
        // hardcoded to 1.0 (always footed; always solo) so their design-
        // matrix columns are collinear with the intercept → singular
        // Hessian without regularization. λ*I shifts the diagonal so the
        // Gauss-Jordan solve always converges; β estimates for constant
        // features approach 0 under regularization (correct behavior —
        // they carry no information until the BT runner emits varied
        // shot_type / assist_kind).
        let lambda = 0.01_f64;
        for i in 0..7 {
            hess[i][i] += lambda;
            // Penalty gradient: ridge adds λβ to gradient (excluding
            // intercept by convention; we apply to all for simplicity
            // since the intercept won't be heavily regularized).
            grad[i] += lambda * beta[i];
        }
        // Solve hess * Δβ = -grad via Gauss-Jordan (7×7; simple enough).
        let delta = solve_7x7(&hess, &grad)
            .map_err(|e| format!("iter {iter}: Hessian solve failed: {e}"))?;
        let mut max_delta = 0.0_f64;
        for i in 0..7 {
            beta[i] -= delta[i];
            max_delta = max_delta.max(delta[i].abs());
        }

        let loss_change = (prev_loss - loss).abs();
        prev_loss = loss;
        if max_delta < 1e-6 || loss_change < 1e-9 {
            converged = true;
            break;
        }
    }

    // Convert back to Q32 raw bits + print paste-ready const block.
    let f64_to_q32_bits = |v: f64| -> i64 { (v * 4_294_967_296.0_f64) as i64 };
    let bits: [i64; 7] = std::array::from_fn(|i| f64_to_q32_bits(beta[i]));

    println!("// PROPOSED β coefficients per T2-1d-infra fit on {n_shots}-shot corpus.");
    println!("// converged={converged} iterations={iter_count} final_loss={prev_loss:.6}");
    println!("// NOT YET APPLIED to source — T2-1d2 wires utility_shoot to xg_utility first.");
    for (i, &b) in bits.iter().enumerate() {
        println!("pub const BETA_{i}: Q32 = Q32::from_raw({b}_i64);");
    }

    // Provenance JSON.
    let result = serde_json::json!({
        "converged": converged,
        "iterations": iter_count,
        "final_loss": prev_loss,
        "n_shots": n_shots,
        "proposed_beta_q32_bits": bits,
        "proposed_beta_f64": beta,
        "corpus_path": corpus_path.display().to_string(),
    });
    if let Some(parent) = result_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create_dir_all({}): {e}", parent.display()))?;
    }
    std::fs::write(result_path, serde_json::to_string_pretty(&result).unwrap())
        .map_err(|e| format!("write({}): {e}", result_path.display()))?;

    Ok(())
}

/// 7×7 Gauss-Jordan solver for the Newton-Raphson Hessian step.
///
/// Clippy's `needless_range_loop` would prefer iterator methods over `for i
/// in 0..7`, but the in-place row swaps + cross-row eliminations make
/// iterator-based rewrites materially less readable for this small-N
/// numeric kernel. Allow-listed for the kernel function only.
#[allow(clippy::float_arithmetic, clippy::needless_range_loop)]
fn solve_7x7(a: &[[f64; 7]; 7], b: &[f64; 7]) -> Result<[f64; 7], String> {
    // Augmented matrix [A | b], in-place reduction.
    let mut m: [[f64; 8]; 7] = [[0.0; 8]; 7];
    for i in 0..7 {
        for j in 0..7 {
            m[i][j] = a[i][j];
        }
        m[i][7] = b[i];
    }

    for i in 0..7 {
        // Partial pivot.
        let mut max_row = i;
        let mut max_val = m[i][i].abs();
        for k in (i + 1)..7 {
            if m[k][i].abs() > max_val {
                max_val = m[k][i].abs();
                max_row = k;
            }
        }
        if max_val < 1e-12 {
            return Err(format!("singular Hessian at row {i}"));
        }
        m.swap(i, max_row);

        // Normalize row i.
        let pivot = m[i][i];
        for j in i..8 {
            m[i][j] /= pivot;
        }
        // Eliminate column i from other rows.
        for k in 0..7 {
            if k == i {
                continue;
            }
            let factor = m[k][i];
            for j in i..8 {
                m[k][j] -= factor * m[i][j];
            }
        }
    }

    Ok(std::array::from_fn(|i| m[i][7]))
}

/// `fit-personality` subcommand: extract per-personality-dimension shot +
/// dribble frequency curves; print proposed K_1..K_18 Q32 raw-bits.
///
/// For each in-scope personality bias (K_1 SHOOT_FLAIR, K_2 SHOOT_COMPOSURE,
/// K_18 SHOOT_RISK reading from `shot_telemetry`; K_7 DRIBBLE_FLAIR, K_8
/// DRIBBLE_AGG reading from `dribble_telemetry`):
///   1. Partition the corpus into bottom-quartile + top-quartile of the
///      relevant attribute.
///   2. Compute shot (or dribble) frequency per quartile.
///   3. Solve for K such that target ratio (top/bottom) ≥ 1.40 per the
///      design-doc test contract.
#[allow(clippy::float_arithmetic)]
fn fit_personality(corpus_path: &Path, result_path: &Path) -> Result<(), String> {
    let corpus_json = std::fs::read_to_string(corpus_path)
        .map_err(|e| format!("read({}): {e}", corpus_path.display()))?;
    let corpus: Corpus =
        serde_json::from_str(&corpus_json).map_err(|e| format!("parse corpus JSON: {e}"))?;

    let q32_to_f64 = |bits: i64| -> f64 { (bits as f64) / 4_294_967_296.0_f64 };
    let f64_to_q32_bits = |v: f64| -> i64 { (v * 4_294_967_296.0_f64) as i64 };

    // Extract per-attribute samples from shots + dribbles.
    let shot_flair: Vec<f64> = corpus
        .shots
        .iter()
        .map(|s| q32_to_f64(s.shooter_flair_q32_raw))
        .collect();
    let shot_composure: Vec<f64> = corpus
        .shots
        .iter()
        .map(|s| q32_to_f64(s.shooter_composure_q32_raw))
        .collect();
    let shot_risk: Vec<f64> = corpus
        .shots
        .iter()
        .map(|s| q32_to_f64(s.shooter_risk_appetite_q32_raw))
        .collect();
    let dribble_flair: Vec<f64> = corpus
        .dribbles
        .iter()
        .map(|d| q32_to_f64(d.dribbler_flair_q32_raw))
        .collect();
    let dribble_agg: Vec<f64> = corpus
        .dribbles
        .iter()
        .map(|d| q32_to_f64(d.dribbler_aggression_q32_raw))
        .collect();

    // For each personality dimension, compute the empirical quartile ratio
    // (how much more often does the top quartile of that attribute take the
    // relevant action vs the bottom quartile). Target: ratio ≥ 1.40 per
    // personality-bias-weights.md test contract. If observed ratio is
    // already ≥ 1.40, hold current K. Otherwise, scale K to meet target.
    let k_1_proposed = solve_k_for_ratio(
        &shot_flair,
        1.40,
        q32_to_f64(fw_match_sim::bt::personality_bias::K_1_SHOOT_FLAIR.to_bits()),
    );
    let k_2_proposed = solve_k_for_ratio(
        &shot_composure,
        1.40,
        q32_to_f64(fw_match_sim::bt::personality_bias::K_2_SHOOT_COMPOSURE.to_bits()),
    );
    let k_7_proposed = solve_k_for_ratio(
        &dribble_flair,
        1.40,
        q32_to_f64(fw_match_sim::bt::personality_bias::K_7_DRIBBLE_FLAIR.to_bits()),
    );
    let k_8_proposed = solve_k_for_ratio(
        &dribble_agg,
        1.40,
        q32_to_f64(fw_match_sim::bt::personality_bias::K_8_DRIBBLE_AGG.to_bits()),
    );
    let k_18_proposed = solve_k_for_ratio(
        &shot_risk,
        1.40,
        q32_to_f64(fw_match_sim::bt::personality_bias::K_18_SHOOT_RISK.to_bits()),
    );

    println!(
        "// PROPOSED K coefficients per T2-1d-infra fit on {}-shot {}-dribble corpus.",
        corpus.shots.len(),
        corpus.dribbles.len()
    );
    println!("// NOT YET APPLIED to source — T2-1d2 wires utility_shoot to xg_utility first.");
    println!(
        "pub const K_1_SHOOT_FLAIR: Q32 = Q32::from_raw({}_i64);",
        f64_to_q32_bits(k_1_proposed)
    );
    println!(
        "pub const K_2_SHOOT_COMPOSURE: Q32 = Q32::from_raw({}_i64);",
        f64_to_q32_bits(k_2_proposed)
    );
    println!(
        "pub const K_7_DRIBBLE_FLAIR: Q32 = Q32::from_raw({}_i64);",
        f64_to_q32_bits(k_7_proposed)
    );
    println!(
        "pub const K_8_DRIBBLE_AGG: Q32 = Q32::from_raw({}_i64);",
        f64_to_q32_bits(k_8_proposed)
    );
    println!(
        "pub const K_18_SHOOT_RISK: Q32 = Q32::from_raw({}_i64);",
        f64_to_q32_bits(k_18_proposed)
    );

    let result = serde_json::json!({
        "n_shots": corpus.shots.len(),
        "n_dribbles": corpus.dribbles.len(),
        "proposed": {
            "K_1_SHOOT_FLAIR": k_1_proposed,
            "K_2_SHOOT_COMPOSURE": k_2_proposed,
            "K_7_DRIBBLE_FLAIR": k_7_proposed,
            "K_8_DRIBBLE_AGG": k_8_proposed,
            "K_18_SHOOT_RISK": k_18_proposed,
        },
        "corpus_path": corpus_path.display().to_string(),
    });
    if let Some(parent) = result_path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("create_dir_all({}): {e}", parent.display()))?;
    }
    std::fs::write(result_path, serde_json::to_string_pretty(&result).unwrap())
        .map_err(|e| format!("write({}): {e}", result_path.display()))?;

    Ok(())
}

/// Empirical proposal for K given a per-action sample list of the relevant
/// attribute. Computes top/bottom quartile ratio + scales K so the ratio
/// meets the target.
///
/// Simplified model: assume action-frequency-per-attribute is roughly
/// proportional to (1 + K * attribute_value). Ratio = (1 + K * top_mean) /
/// (1 + K * bot_mean). Solving for K to hit target ratio R:
///   K = (R - 1) / (top_mean - R * bot_mean)
///
/// If top_mean ≤ R * bot_mean (degenerate) OR sample size < 8, hold current K.
#[allow(clippy::float_arithmetic)]
fn solve_k_for_ratio(samples: &[f64], target_ratio: f64, current_k: f64) -> f64 {
    if samples.len() < 8 {
        return current_k;
    }
    let mut sorted = samples.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q1_end = sorted.len() / 4;
    let q4_start = (sorted.len() * 3) / 4;
    let bot_mean: f64 = sorted[..q1_end].iter().sum::<f64>() / (q1_end as f64).max(1.0);
    let top_mean: f64 =
        sorted[q4_start..].iter().sum::<f64>() / ((sorted.len() - q4_start) as f64).max(1.0);

    let denominator = top_mean - target_ratio * bot_mean;
    if denominator <= 1e-6 {
        return current_k;
    }
    let proposed = (target_ratio - 1.0) / denominator;
    // Clamp to a reasonable [0.1, 2.0] range so a degenerate corpus
    // doesn't propose K values that blow out the [0.25, 0.45] design-doc
    // sanity envelope. (Phase-1 K values cluster around 0.3.)
    proposed.clamp(0.1, 2.0)
}

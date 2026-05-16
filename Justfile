# Final Whistle — dev-task automation (T0-8)
#
# Just is the dev-task front-door. `scripts/fw` shells out to most of these
# for a stable CLI alias. CI workflows call cargo/pnpm directly (NOT just)
# so the CI surface area is independent of just availability on runners.
#
# `just --list` lists every available recipe with its summary comment.

# default: list available recipes
default:
    @just --list

# ----------------------------------------------------------------
# Dev loop
# ----------------------------------------------------------------

# Launch Tauri dev mode (Rust backend + SolidJS frontend with HMR).
dev:
    cd frontend && pnpm install
    cargo tauri dev

# ----------------------------------------------------------------
# Tests
# ----------------------------------------------------------------

# Full workspace test pass (matches the ci.yml main test step).
test:
    cargo test --workspace --release

# T1-13: Frontend Vitest suite. Runs the Vitest substrate activated at T1-6
# + broadened at T1-13 (FrameSource + TacticalBoard lifecycle + window.fwDev).
# Wired into `ci-local` (and therefore `scripts/fw verify`) so a frontend
# test regression blocks the pre-commit gate. Runtime budget: <10s on dev box
# (currently ~800ms for 34 tests across 3 files).
frontend-test:
    cd frontend && pnpm test

# Library-only fast iteration (skips integration tests + tauri build).
test-fast:
    cargo test --workspace --lib

# The Phase-0 determinism gate in isolation. Mirrors the
# .github/workflows/determinism-gate.yml job exactly.
test-determinism:
    cargo test --release -p fw-replay --test canonical_hash

# ----------------------------------------------------------------
# Lint + format
# ----------------------------------------------------------------

# All gates that fail-fast in CI: fmt + clippy + frontend lint + typecheck.
lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cd frontend && pnpm lint
    cd frontend && pnpm typecheck

# Auto-fix where possible (does NOT run clippy --fix; clippy fixes get
# reviewed by a human).
fmt:
    cargo fmt --all
    cd frontend && pnpm format

# ----------------------------------------------------------------
# Build
# ----------------------------------------------------------------

build-debug:
    cargo build --workspace

build-release:
    cargo build --workspace --release

# Full Tauri bundle for the current platform (DMG / MSI / AppImage).
bundle:
    cd frontend && pnpm install --frozen-lockfile
    cd frontend && pnpm build
    cargo tauri build

# ----------------------------------------------------------------
# Local CI mirror
# ----------------------------------------------------------------

# Reproduces the per-PR CI matrix as closely as `just` can on a single
# host. Use before pushing to `main` direct-commit per CLAUDE.md §4.5.
# This is what `/next` step 6 (verify) invokes via `scripts/fw verify`.
#
# T1-13: extended with `frontend-test` (Vitest), `audit` (cargo-audit
# vulnerability scan), `deny` (cargo-deny license + advisory check). The
# audit/deny gates run on the workspace root; both tools must be
# `cargo install`-ed locally (see `cargo install cargo-audit cargo-deny`).
ci-local: lint test frontend-test banned-terms test-determinism determinism-audit verify-content audit deny

# T1-13: RustSec vulnerability scan via cargo-audit. Fails on any active
# advisory; baseline (T1-13 commit time) is zero vulnerabilities + 19
# transitive unmaintained warnings from Tauri's GTK3 stack on Linux.
#
# Tool install (one-time): `cargo install cargo-audit`. CI does this in
# the dedicated cargo-audit job per ci.yml.
audit:
    cargo audit

# T1-13: cargo-deny multi-gate (advisories + licenses + bans + sources)
# per `deny.toml` at repo root. License allowlist is permissive +
# LGPL-compatible; advisories deny vulnerabilities + warn on unmaintained
# (the bincode `RUSTSEC-2025-0141` ignore is documented inline in deny.toml).
#
# Tool install (one-time): `cargo install cargo-deny`.
deny:
    cargo deny check

# ----------------------------------------------------------------
# Banned-terms lint (football-native vocabulary)
# ----------------------------------------------------------------

# Scan content + design docs + frontend strings for banned mystical
# state-nouns. Catalog in docs/design/ui-vocabulary.md. Sentinel
# exemption via `// ui-lint:allow term="..." reason="..." reviewer="..."`.
banned-terms:
    python3 scripts/lint-banned-terms.py content/ docs/design/ frontend/src/
    # T1-20: regression test that the lint detects banned terms under
    # content/baked/ (post-exclusion-drop) + ignores sentinel blocks outside
    # the docs/Rust scope (post-sentinel-scope-restriction).
    python3 scripts/test-lint-banned-terms.py

# ----------------------------------------------------------------
# Content-pack validation (FW-VAL)
# ----------------------------------------------------------------

# Run FW-VAL checks per docs/specs/content-pack-validation-contract.md.
#
# Codex full-project audit P1 (2026-05-13) Tranche 6 fix:
# - Was: `cargo run -p fw-content-baker -- validate-content || echo
#        "fw-content-baker validator not yet implemented (T2-3); skipping"`.
#   Two bugs there: (a) `validate-content` isn't a valid subcommand (the
#   CLI's command is `validate`); (b) `|| echo` masked any failure, so the
#   recipe silently passed.
# - Now: invokes the real `validate-structural` subcommand. Exits non-zero on
#   any validation error (fail-closed). No silent skip.
#
# T1-20 (post-T1-close ultimate-review Track E #1): the subcommand was renamed
# `validate` → `validate-structural` so the CLI surface stops promising "all
# validators passed" when only structural validators actually run.
# `validate-semantic` + `validate-content-pack` land at T2-3 alongside the real
# bake pipeline per Codex workflow improvement #4's 3-way split.
#
# Runtime content validators (RoleAffinityTable weight sums + PlayerAttributes
# Q32 range + manager → tactical_archetype cross-ref + player_template →
# signature_definition cross-ref) are real as of T1-20. The bake-time semantic
# validators (banned-terms shell-out + licensed-data corpus + cliché detection)
# are still NOT IMPLEMENTED — they land at T2-3 when their consumer (bake-names)
# lands.
verify-content:
    cargo run -p fw-content-baker -- validate-structural

# ----------------------------------------------------------------
# Determinism audit (Codex pre-T0 audit follow-up)
# ----------------------------------------------------------------

# Static audit of the determinism bans in Sim/RULES.md (HashMap, tokio,
# clocks, system RNG, f32/f64). Catches violations that clippy lints
# don't cover and that may appear via transitive paths. The Python script
# strips line + block comments before matching, so the rule docs that
# discuss the bans don't false-positive. Fail-closed: any match exits
# non-zero so `ci-local` blocks.
determinism-audit:
    python3 scripts/determinism-audit.py

# ----------------------------------------------------------------
# Workspace housekeeping
# ----------------------------------------------------------------

clean:
    cargo clean
    rm -rf frontend/dist frontend/node_modules

# ----------------------------------------------------------------
# Content baker (T2+ — see MASTER_PLAN T2-3)
# ----------------------------------------------------------------

# Regenerate the content corpus from base templates. Runs the bake-time
# Claude API loop; output is reviewed + committed manually per CLAUDE.md
# §3 (bake-time only, no runtime LLM).
bake-content:
    cargo run -p fw-content-baker -- bake --output content/baked

# ----------------------------------------------------------------
# Snapshot maintenance
# ----------------------------------------------------------------

# Review pending insta snapshot drifts interactively. Use cautiously —
# accepting a canonical-hash drift here without a SPEC entry is exactly
# the kind of thing canonical-hash-guard.sh exists to catch.
review-snapshots:
    cargo insta review

# ----------------------------------------------------------------
# Cross-compile sanity check (Mac dev host → Win/Linux targets)
# ----------------------------------------------------------------

# Compile checks ONLY (no link); proves Win/Linux targets stay buildable
# from the Mac dev box without spinning up Windows / Linux VMs. Requires
# `cargo install cargo-zigbuild` + `brew install zig`. Used as a smoke
# step before tagging a release; the real cross-platform build happens
# in release.yml on actual GitHub-hosted runners for each OS.
cross-check:
    cargo zigbuild --target x86_64-pc-windows-msvc
    cargo zigbuild --target x86_64-unknown-linux-gnu

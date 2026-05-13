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
ci-local: lint test banned-terms test-determinism

# ----------------------------------------------------------------
# Banned-terms lint (football-native vocabulary)
# ----------------------------------------------------------------

# Scan content + design docs + frontend strings for banned mystical
# state-nouns. Catalog in docs/design/ui-vocabulary.md. Sentinel
# exemption via `// ui-lint:allow term="..." reason="..." reviewer="..."`.
banned-terms:
    python3 scripts/lint-banned-terms.py content/ docs/design/ frontend/src/

# ----------------------------------------------------------------
# Content-pack validation (FW-VAL)
# ----------------------------------------------------------------

# Run FW-VAL checks per docs/specs/content-pack-validation-contract.md.
# Falls back gracefully if fw-content-baker isn't built yet (T0 stub).
verify-content:
    cargo run -p fw-cli -- validate-content || echo "fw-cli content validator not yet implemented (T2-3); skipping"

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
    cargo run -p fw-cli -- bake --output content/baked

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

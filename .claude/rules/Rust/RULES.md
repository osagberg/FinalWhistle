---
description: General Rust style for all Final Whistle crates. Determinism rules live in Sim/RULES.md; this file is the cross-cutting baseline.
applies_to:
  - crates/**/*.rs
  - src-tauri/**/*.rs
auto_load: when_editing_matching_path
---

# Rust general rules

## Edition + toolchain

- Edition **2024**.
- Rust **1.95+** (pinned via `rust-toolchain.toml`). Bumped from 1.85 at T0-12 because `fixed@1.31` (Q32.32 backing crate) requires rustc 1.93+.
- `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings` clean before commit.

## Abstractions

- **No speculative abstractions.** Three similar lines beats a premature trait. Three similar call sites is the threshold for extracting a function. Don't generalize on two.
- Prefer **monomorphic generics** over `Box<dyn Trait>` in hot paths. `Box<dyn Trait>` is fine at IPC and serde boundaries.
- Avoid `Rc` / `Arc` unless ownership genuinely shares across threads. Pass `&T` when the borrow checker permits.
- Newtypes for typed IDs (`PlayerId(u32)`, `MatchId(u32)`, `ClubId(u32)`). Never a raw `u32` field whose meaning depends on context.

## Errors

- **Library crates:** `thiserror` for typed errors. One `Error` enum per crate.
- **Binary crates** (`fw-content-baker`, `src-tauri`): `anyhow` is fine.
- `Result<T, E>` for fallible operations with meaningful failure context. `Option<T>` only when "absent" is the failure mode.
- Never `unwrap()` outside tests. `expect("...")` with a reason is acceptable when the invariant is genuinely impossible to violate (then prove it in a comment).
- `?` operator for propagation. No nested `match` over `Result` when `?` works.

## Unsafe

- Avoid. If genuinely required, every `unsafe` block has a `// SAFETY:` comment naming the invariant.

## API surface

- `#[must_use]` on every `Result`-returning API and every builder.
- `pub(crate)` over `pub` unless cross-crate use is intended.
- Doc comments (`///`) on every `pub` item. Top-of-file `//!` for crate-level intent.
- Internal items: no comments unless WHY is non-obvious. Don't narrate WHAT.

## Cloning

- Minimal `clone()`. Pass references; clone only at ownership boundaries.
- `Cow<'a, str>` over `String` when the borrow vs. owned choice is real.

## Iterator + collection style

- Iterator chains preferred over loop+index. Stop the chain when readability tanks.
- `collect::<Vec<_>>()` only when downstream needs `Vec`. Otherwise `.collect()` with target type inferred.
- See `Sim/RULES.md` for the BTreeMap / HashMap rule (determinism-driven).

## Tests

- `#[cfg(test)] mod tests { ... }` at the bottom of the module under test.
- Integration tests in `crates/<crate>/tests/`.
- `insta` snapshots for stable readable outputs.
- `proptest` invariants for logic-heavy code.

## Formatting

- `cargo fmt` style. No manual line breaks vs. the formatter's choice. Disagree? Run it once + accept.
- Comments above the line they refer to, not trailing.
- No emojis in source unless explicitly requested.

## Lints

- No `#[allow(...)]` without a comment explaining why.
- `#[deny(clippy::float_arithmetic)]` and other determinism guards are crate-level on sim crates (see `Sim/RULES.md`).

## Cross-references

- `Sim/RULES.md` — determinism overrides for sim crates (stricter than this file)
- `Tauri/RULES.md` — IPC + async patterns
- `CLAUDE.md` §7 — project code style

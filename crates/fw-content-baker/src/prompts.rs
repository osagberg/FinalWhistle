//! Embedded prompt templates for each bake subcommand.
//!
//! Markdown sources live under `src/prompts/`. We embed them at compile time
//! via `include_str!` so the binary doesn't depend on the working directory
//! at runtime. Each prompt is paired with a JSON-Schema (see `schemas.rs`).
//!
//! Prompts use the `{placeholder}` convention for slot substitution at bake
//! time — culture_id, archetype_id, count_per_bank, etc.

// Prompts authored ahead of the bake subcommands that consume them.
// T2-3 wires bake-names; T2-4 wires bake-bios; T3-3 wires bake-headlines;
// T3-5 wires bake-scout-phrases. Each subcommand will reach into this
// module via its const name. Until then, the consts are dead code from
// the compiler's view — silence the lint at the module root rather than
// per-const to keep additions painless.
#![allow(dead_code)]

pub const NAMES_PROMPT: &str = include_str!("prompts/names.md");
pub const BIOS_PROMPT: &str = include_str!("prompts/bios.md");
pub const HEADLINES_PROMPT: &str = include_str!("prompts/headlines.md");
pub const SCOUT_PHRASES_PROMPT: &str = include_str!("prompts/scout-phrases.md");

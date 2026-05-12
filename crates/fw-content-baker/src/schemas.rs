//! JSON-Schema definitions for LLM output validation.
//!
//! Every baker subcommand has a target JSON shape it expects the Claude API
//! to return. We send the schema as part of the prompt (anthropic
//! structured-output / "your response MUST validate against this schema"
//! discipline) AND validate post-response. Anything that fails schema
//! validation is rejected, logged, and retried with a tighter prompt — or
//! escalated to the dev for manual review.
//!
//! These schemas are intentionally strict: extra properties are rejected,
//! types are pinned, string lengths bounded. The point of bake-time is to
//! catch LLM weirdness before it ever reaches a shipped build.
//!
//! Stub at T0; real schemas land alongside each `bake-*` subcommand.

// JSON-Schema for a per-culture name-bank response.
//
// Real implementation will use the `jsonschema` crate to compile this at
// startup. Pattern at T0 is illustrative; do not consume yet.
pub const NAMES_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "additionalProperties": false,
  "required": ["culture_id", "first_names", "last_names"],
  "properties": {
    "culture_id":  { "type": "string", "pattern": "^[a-z][a-z0-9-]{2,30}$" },
    "first_names": {
      "type": "array",
      "minItems": 20,
      "maxItems": 200,
      "items": { "type": "string", "minLength": 2, "maxLength": 40 }
    },
    "last_names": {
      "type": "array",
      "minItems": 20,
      "maxItems": 200,
      "items": { "type": "string", "minLength": 2, "maxLength": 40 }
    }
  }
}
"#;

// JSON-Schema for a player biography template response.
pub const BIO_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "additionalProperties": false,
  "required": ["culture_id", "archetype_id", "templates"],
  "properties": {
    "culture_id":   { "type": "string", "pattern": "^[a-z][a-z0-9-]{2,30}$" },
    "archetype_id": { "type": "string", "pattern": "^[a-z][a-z0-9-]{2,40}$" },
    "templates": {
      "type": "array",
      "minItems": 10,
      "maxItems": 300,
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["body"],
        "properties": {
          "body":     { "type": "string", "minLength": 40, "maxLength": 400 },
          "tone":     { "type": "string", "enum": ["positive", "neutral", "negative"] }
        }
      }
    }
  }
}
"#;

// JSON-Schema for a Tracery grammar response (headlines, manager-quotes,
// fan-reactions).
pub const TRACERY_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "required": ["origin"],
  "properties": {
    "origin": { "type": "array", "items": { "type": "string" }, "minItems": 1 }
  },
  "additionalProperties": {
    "type": "array",
    "items": { "type": "string", "minLength": 1, "maxLength": 200 }
  }
}
"#;

// JSON-Schema for scout-report phrase templates.
pub const SCOUT_PHRASES_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "additionalProperties": false,
  "required": ["archetype_id", "phrases"],
  "properties": {
    "archetype_id": { "type": "string", "pattern": "^[a-z][a-z0-9_-]{2,40}$" },
    "phrases": {
      "type": "array",
      "minItems": 10,
      "maxItems": 200,
      "items": {
        "type": "object",
        "additionalProperties": false,
        "required": ["text", "valence"],
        "properties": {
          "text":    { "type": "string", "minLength": 8, "maxLength": 200 },
          "valence": { "type": "string", "enum": ["positive", "neutral", "negative"] }
        }
      }
    }
  }
}
"#;

// JSON-Schema for match-commentary phrase banks.
pub const COMMENTARY_SCHEMA: &str = r#"
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "additionalProperties": false,
  "required": ["event_type", "lines"],
  "properties": {
    "event_type": {
      "type": "string",
      "enum": ["goal", "save", "miss", "foul", "card", "sub", "kick-off", "full-time"]
    },
    "lines": {
      "type": "array",
      "minItems": 10,
      "maxItems": 100,
      "items": { "type": "string", "minLength": 8, "maxLength": 200 }
    }
  }
}
"#;

# patterns/ — Final Whistle pattern library

Reference docs for recurring solutions. Cited from design docs, ADRs, and CLAUDE.md. Reading order is least-to-most domain-specific.

| Pattern | Why it matters | Owning crate(s) |
|---|---|---|
| `behavior-trees.md` | The match-sim AI runner. 22 players × 60Hz × N-tick matches. | `fw-match-sim` |
| `event-driven.md` | Append-only career memory ledger (pillar 2). | `fw-memory` |
| `fsm.md` | Match phase, player state, ball state — typed enums + match. | `fw-match-sim` |
| `save-load.md` | Bincode 2 versioned saves; forward migration only. | `fw-save` |
| `unit-testing.md` | cargo test + insta + proptest + canonical-hash regression. | all crates |
| `phase-gate-workflow.md` | Codex review at phase boundaries via PR. | tooling |
| `dependency-injection.md` | Trait + generics; minimal Box<dyn>. | all crates |
| `bake-time-llm-content-pipeline.md` | Pillar 1 — procedural world via offline LLM corpus. | `fw-content-baker` |
| `narrative-subagent-pattern.md` | When + how to invoke the `narrative-director` agent. | tooling |

Slimmed from the blueprint's 18 patterns — Unity-specific patterns (Addressables, DOTS ECS intro, Unity object pooling, Unity spatial partitioning) dropped. Cross-cutting non-game patterns (accessibility, analytics-opt-in, community-feedback) deferred to a future "operations" subdirectory if needed.

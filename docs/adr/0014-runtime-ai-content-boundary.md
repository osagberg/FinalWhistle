# ADR-0014 — Runtime AI / content boundary

**Status:** Proposed

**Date:** 2026-05-13

**Decider:** Claude (Codex full-project audit Lane B "missing ADR" driver) + Codex (pending pre-T2-3 audit)

---

## Context

`CLAUDE.md §3` + `DESIGN_DOC.md §3` both state "No runtime LLM calls. All generated text baked at content-pack build time" as a core architectural rule. The rule lives in multiple docs but has no authoritative ADR. The Codex full-project audit Lane B flagged this as a missing ADR.

This ADR formalises:
1. What "runtime AI" specifically forbids
2. How bake-time content + manifest works structurally
3. Rebake policy (when can we regenerate content; how is the regeneration captured)
4. The single allowed runtime LLM exception path (currently: NONE — but the policy must be explicit)

## Decision

### Forbidden at runtime

Inside the shipped game binary, while a user is playing:

1. **No outbound HTTP / TCP / UDP** to any LLM provider (Anthropic, OpenAI, Google, etc.). The shipped game makes ZERO LLM API calls.
2. **No embedded model inference.** No `llama.cpp` / `candle` / `tch` / `ort` binary blob shipped in the game directory. The game does not run inference, period.
3. **No "request a fresh LLM call" code path.** The codebase does not contain a function that, if called, would issue an LLM request. The clippy ban for `reqwest` / `tokio` in sim crates partially enforces this; the broader contract is "no LLM SDK dependency in any shipped crate."
4. **No deferred AI work.** "Generate the content when the user first asks" is also forbidden — content is generated AT BAKE TIME, committed to disk as RON, and shipped with the binary.

### Allowed at bake-time

`fw-content-baker` (a CLI tool, NOT part of the shipped game):

1. **Outbound Anthropic API calls** are allowed during `just bake-content`. The tool runs in the dev environment, reads `ANTHROPIC_API_KEY` from env, makes paid API calls, persists output to `content/baked/<kind>/<id>.ron`.
2. **All generated output is committed to git.** The bake's output is reproducible: rerunning bake with the same `match_seed` + `prompt_hash` + `model_id` MUST produce identical RON. Determinism at bake-time is captured by the manifest (below).
3. **Bake manifest** at `content/baked/manifest.ron`:

```ron
ContentBakeManifest(
    schema_version: 1,
    baked_at: "2026-05-13T14:23:00Z",  // ISO-8601; informational only
    model_id: "claude-opus-4-7-20260301",  // exact API identifier
    prompt_hash: "blake3:...",  // BLAKE3 of the bake-prompt template
    prompt_template_version: "v1.3",  // dev-readable handle
    corpus_version: 7,  // bumps on every regen
    seed: 0xdeadbeef,  // master seed for procedural fill
    licensed_data_corpus_version: 4,  // per ADR-0013
    counts: {
        "culture": 12,
        "player_template": 1500,
        "scout_phrase": 800,
        "news_headline": 2400,
        // ...
    },
)
```

The manifest is committed alongside the baked RON. Loading content compares `manifest.corpus_version` against the runtime's compatible-version range; mismatched corpora surface a warning to the user but are not fatal.

### Rebake policy

A rebake is **valid** when:

1. **Model version changes** — `claude-opus-4-7` → `claude-opus-4-8`. The manifest's `model_id` shifts; corpus_version bumps; the dev re-baselines + commits.
2. **Prompt template changes** — wording, structure, or system-prompt updates. `prompt_template_version` + `prompt_hash` shift; corpus_version bumps; full regeneration required.
3. **Seed change** — same template, different seed (e.g. expanding name banks). Corpus_version bumps; old + new entries coexist in the baked output (no deletion).
4. **Licensed-data corpus refresh** — new corpus version detected; previously-passing names may now collide; rebake + manual review per ADR-0013.

A rebake is **invalid** when:

1. **Mid-EA "we want different names" cosmetic regeneration.** Names that have shipped to players in saved careers MUST persist. The save-mod-fingerprint mechanism (ADR-0010) handles the case where a save references content whose ID is gone, but the policy is: ship-once, persist-forever.
2. **"Fix one name" without manifest bump.** A single-name edit (ADR-0013's collision fix flow) IS allowed without rebake, but the manifest's `counts` MUST update + commit alongside.

### Audit trail

Every bake produces:
- `content/baked/manifest.ron` (the manifest above)
- `content/baked/<kind>/*.ron` (the per-kind output)
- A git commit body line: `corpus bake: <kind> count=N model=<id> prompt_version=<v> seed=0x... → corpus_version=N+1`

The `fw verify-content` umbrella command (Justfile) re-validates the manifest against the on-disk corpus + asserts the licensed-data-validator passed at last bake.

### One allowed exception: bake-time review

`fw-content-baker --review-output` is allowed to issue NEW LLM calls during the manual-review step — a developer running the baker locally can ask for a "is this name plausibly real?" double-check via a separate Anthropic API call. The review-result is INFORMATIONAL ONLY — it doesn't mutate the baked output; the developer makes the final call. This is the human-in-the-loop quality gate.

### Future-proofing — when (if) this rule changes

This rule is non-negotiable for v1.0 EA + the first ~year post-launch. If at some future point a serious case emerges (e.g. dynamic commentary generation that genuinely needs LLM), the path is:

1. New ADR superseding this one, explaining the case + the new boundary.
2. User-facing opt-in: "Online narrative" toggle in Settings; default OFF.
3. Privacy + data-usage disclosure per Steam content policies.
4. Network-failure fallback that degrades to baked content gracefully.

This ADR explicitly forbids the change for v1.0; the future-proofing is documentation, not allowance.

## Consequences

**Positive:**
- Pillar 1's "no real licensed data" coexists with the broader runtime-LLM ban — the game's output is fully reproducible from committed RON + game version.
- Players get consistent content across save-loads (the LLM doesn't drift). Pillar 2 ("careers that remember") relies on consistent text being there to remember.
- Cost predictability — Anthropic API spend is bounded by bake-time runs, not by user-count × playtime.
- Privacy — the shipped game makes no outbound network calls to LLM providers; no user data leaves their machine on the LLM dimension.
- Steam content policy compliance — the shipped game doesn't depend on cloud services.

**Negative:**
- All content is committed RON. The repo grows over time; `content/baked/` is gitignored by default (regenerable from `just bake-content`) but the manifest + the per-kind sources are committed. ~10–50 MB per major bake.
- Bake-time cost: Anthropic API spend per major content release. Budgetable in advance; no surprises.
- No "dynamic" commentary in v1.0 — every line in the commentary feed is from a pre-baked template bank. The Tracery-style grammar (ADR-0005-adjacent) provides per-event substitution variety, but the bank is fixed.
- Mod authors who want LLM-generated content must run their own bake-time tooling; the game doesn't help.

**Neutral:**
- The rule applies to TEXT content. Procedural generation of player attributes / match outcomes / etc. happens at runtime via `ChaCha8Rng` + Q32 math (deterministic). That's not LLM.

**Rollback path:**
- If a real case for runtime LLM emerges, the future-proofing path (above) is the rollback. Each step is gated; the user explicitly opts in.

## Alternatives considered

- **Allow runtime LLM with player opt-in from day one.** Rejected — adds complexity, network-failure-mode handling, cost-shifting to player, privacy-disclosure overhead. None of these earn their keep at v1.0.
- **Embedded small-model inference (e.g. a 4B-parameter Llama distilled for commentary).** Rejected — model weights add ~3 GB to the game install; inference performance on consumer hardware is unreliable; quality is meaningfully worse than the Anthropic baseline that powers the baker.
- **Bake-time + runtime hybrid: bake the bulk, runtime-generate the rest.** Rejected — the "rest" is hard to scope and the determinism contract (Pillar 2) requires reproducible content across save-loads.
- **"Local LLM proxy" the user runs themselves (Ollama).** Rejected — adds a system-dependency for a feature most users won't have set up; degrades to the baked content anyway.

## References

- CLAUDE.md §3 (the stack contract)
- DESIGN_DOC.md §3 (the pillar)
- `docs/CONTENT_PIPELINE.md` (the bake-time pipeline spec)
- `crates/fw-content-baker/src/main.rs` (the CLI implementation; T2-3 fills the real work)
- ADR-0010 (save format; mod-fingerprint mechanism)
- ADR-0013 (licensed-data policy — composes with this ADR)
- `.claude/rules/Sim/RULES.md` §5 (no tokio / async in sim crates — partial enforcement of the "no runtime LLM" rule at the dependency layer)
- Codex full-project audit Lane B "missing ADRs"

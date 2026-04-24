<!--
Final Whistle PR template.
Fill every section. Delete sections only if they genuinely don't apply.
-->

## Summary

<!-- 1-2 sentences: what changes, in plain football-native English where possible. -->

## Why

<!-- The WHY, not the WHAT.
    Link the SPEC task / ADR / decisions-log entry that drives this.
    Example: "Closes Phase 1 task 'scripts/fw skeleton' from SPEC.md §Phase 1." -->

## Linked

- SPEC task:
- ADR (if any):
- Design doc(s) touched:
- Previous decision superseded (if any):

## Test plan

<!-- Bulleted checklist of what was verified. -->

- [ ] `scripts/fw verify-docs` clean
- [ ] `scripts/fw test` green (if MatchSim touched)
- [ ] `scripts/fw replay <seed>` deterministic across Win/Mac/Linux (if sim touched)
- [ ] Banned-term lint clean (if UI / content touched)
- [ ] Content-pack validator clean (if content touched)
- [ ] Manual Unity smoke (if viewer / editor code touched)

## Breaking changes

<!-- Any of these? Explicitly call out; never silent.
    - Schema bump (content-pack version OR MemoryEvent version OR IdentityPacket version)
    - Save-compat breakage
    - New banned-term category
    - New Phase-2 ADR candidate
    - New external dependency
    If none, write "None." -->

## Cinematic / gameplay feel (if applicable)

<!-- For viewer / signature / breakthrough / semantic-cinema work:
    describe the intended feel change + include a short recording / screenshot diff if possible. -->

## Checklist before requesting review

- [ ] Design doc consulted; code matches design intent
- [ ] No new banned-term strings introduced (or sentinel-exempted with reviewer handle)
<!-- ui-lint:ignore-start reason="checklist meta-reference to placeholder tokens" -->
- [ ] No `{{PROJECT_NAME}}` / `TODO:` / `FIXME` leaks in shipped content
<!-- ui-lint:ignore-end -->
- [ ] Decisions log left append-only (no mutations to past entries)
- [ ] CHANGELOG line drafted (will be landed on merge)

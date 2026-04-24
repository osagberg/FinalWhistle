---
name: Bug report
about: Report a defect in Final Whistle (sim, viewer, UI, content, or pipeline)
title: "[BUG] "
labels: bug
---

## What happened

<!-- ui-lint:ignore-start reason="bug-report guidance showing banned vs non-banned example phrasings" -->
<!-- One-sentence description in football-native terms where possible.
     Bad: "The XP counter is wrong."
     Better: "Striker signature unlocked without the 0.85 readiness threshold being crossed." -->
<!-- ui-lint:ignore-end -->

## Expected vs actual

- **Expected:**
- **Actual:**

## Repro

- Build channel: <!-- dev / tester-closed / demo / ea / hotfix -->
- Build version / commit: <!-- paste from build-info or about-screen -->
- Content-pack version: <!-- paste from save or pack manifest -->
- Save slot seed: <!-- if relevant -->
- Match seed: <!-- if relevant; enables `scripts/fw replay <seed>` -->

### Steps

1.
2.
3.

## Diagnostics bundle

<!-- If the build has the in-build "Export bug bundle" button, attach the zip.
     Bundle contains: save + replay seeds + rolling logs + settings + content-pack version manifest. -->

- [ ] Diagnostics bundle attached (or path below)

## Platform

- OS:
- Unity version (from build-info):
- Resolution / display scaling:

## Severity (reporter's best guess — triager may reclassify)

- [ ] Blocker — crashes, corrupts save, prevents launch / gate-check
- [ ] High — breaks a locked system (determinism, memory pillar, signature behaviour)
- [ ] Medium — feature behaves incorrectly but has workaround
- [ ] Low — cosmetic, text, tone

## Related

- SPEC task:
- Design doc:
- Prior issue:

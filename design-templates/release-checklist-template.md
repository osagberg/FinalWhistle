---
description: Per-release checklist — build / quality / content / platform / store. Copied into `docs/release/<version>.md` and edited for the specific release.
---

<!-- USAGE
Project-local reusable checklist. Copy this into `docs/release/<version>.md`
for each release and mark items as they complete. Unlike a blueprint-level
launch reference (which is advisory), this is the hard gate for the specific
release — GO/NO-GO decision lives here.

Delta from blueprint's launch-checklist.md: that reference describes the
full Steam release process once; this template is per-project and per-release,
customized for your build + platforms + scope.

Cross-refs:
  - design-templates/release-notes.md           (player-facing counterpart)
  - design-templates/test-plan.md               (quality gates trace here)
  - design-templates/architecture-traceability.md (cross-ADR conflicts must be 0)
  - SPEC.md final phase                         (release phase is the parent)
-->

# Release Checklist: {{PROJECT_NAME}} v<fill-in: X.Y.Z> — <fill-in: Platform>

**Release Date**: <fill-in: YYYY-MM-DD>
**Release Manager**: <fill-in: self>
**Status**: [ ] GO  /  [ ] NO-GO

---

## Build Verification

- [ ] Clean build succeeds on all target platforms
- [ ] Zero compiler warnings (CSharp/RULES.md zero-warning policy)
- [ ] Version number set in Player Settings: `<fill-in>`
- [ ] Build reproducible from tagged commit `<fill-in: git sha>`
- [ ] Build size within budget: <fill-in: actual>MB / <fill-in: budget>MB
- [ ] No `DEVELOPMENT_BUILD` or `UNITY_EDITOR` code paths in Release build
- [ ] `DebugManager` stripped (verify: `#if UNITY_EDITOR || DEVELOPMENT_BUILD` guards hold)
- [ ] Addressables built and catalog shipped (if using remote content)
- [ ] No orphaned scene meta, no missing SO refs — run /audit

---

## Quality Gates

### Critical Bugs

- [ ] Zero S1 (Critical) bugs open
- [ ] Zero S2 (Major) bugs — or documented exceptions below:

| Bug ID | Description | Exception Rationale | Approved By |
|---|---|---|---|
| <fill-in> | <fill-in> | <fill-in> | <fill-in> |

### Test Coverage

- [ ] All critical-path features tested + signed off
- [ ] Full regression suite passed: <fill-in>% pass rate
- [ ] EditMode tests green
- [ ] PlayMode tests green
- [ ] Performance tests within budget
- [ ] Soak test passed (4+ hours continuous play without crash / leak)

### Architecture

- [ ] [architecture-traceability.md](architecture-traceability.md) has zero Unresolved Cross-ADR Conflicts
- [ ] Zero Foundation Layer gaps in traceability

### Performance

- [ ] Target framerate met on min spec: <fill-in>fps / <fill-in>fps target
- [ ] Memory within budget: <fill-in>MB / <fill-in>MB
- [ ] Load times within budget: <fill-in>s / <fill-in>s
- [ ] No memory leaks over 1hr play (Unity Memory Profiler confirms)
- [ ] No frame drops below <fill-in>fps in normal gameplay

---

## Content Complete

- [ ] All placeholder assets replaced with final versions
- [ ] All player-facing text proofread
- [ ] All text driven by localization strings — zero hardcoded UI text
- [ ] Localization complete for: <fill-in: locales or "English-only V1">
- [ ] Audio mix finalized + approved
- [ ] Credits complete + accurate
- [ ] Legal notices + third-party attributions (lilToon, UniTask, Odin, etc.)
- [ ] No debug UI visible (god-mode, FPS counter, state-dump buttons)

---

## Platform: PC / Steam

- [ ] Steam SDK integrated (`Steamworks.NET`) and tested
- [ ] AppID configured in Steam partner dashboard
- [ ] Achievements functional (if any)
- [ ] Cloud saves functional (if any)
- [ ] Steam Input controller profile configured
- [ ] Min + recommended specs documented
- [ ] Keyboard+mouse fully functional
- [ ] Controller support tested (Xbox / PS / Steam Deck)
- [ ] Resolution scaling tested: 1080p / 1440p / 4K / ultrawide (21:9)
- [ ] Windowed / borderless / fullscreen modes working
- [ ] Graphics settings save + load across sessions
- [ ] Steam Deck verified: controller default, text legibility at 1280×800, battery usage

---

## Platform: Console (if applicable)

- [ ] TRC/TCR/Lotcheck requirements met
- [ ] Platform controller glyphs correct (PS / Xbox / Nintendo variants)
- [ ] Suspend/resume works
- [ ] User switching handled
- [ ] Network loss handled gracefully
- [ ] Storage full scenario handled
- [ ] Parental controls respected
- [ ] Cert submission prepared

---

## Store + Distribution

- [ ] Store page metadata complete + proofread
- [ ] Screenshots current + meet platform requirements (1920×1080 min)
- [ ] Trailer uploaded + verified
- [ ] Key art + capsule images final
- [ ] Age ratings: [ ] ESRB  [ ] PEGI  [ ] Other
- [ ] EULA / Privacy Policy / ToS current
- [ ] Pricing configured for all regions
- [ ] Release notes published (see [release-notes.md](release-notes.md))

---

## Launch Readiness

- [ ] Crash reporting configured (BugSplat / Sentry / Unity Cloud Diagnostics)
- [ ] Day-one patch prepared (if needed)
- [ ] Community announcements drafted
- [ ] Press / influencer keys prepared (if any)
- [ ] Support channel ready (Discord / email / Steam forums)
- [ ] Known issues list prepared for Community
- [ ] Rollback plan documented + tested (revert to `v<fill-in: previous>`)

---

## Sign-Off

| Role | Name | Status | Date |
|---|---|---|---|
| Developer | <fill-in> | [ ] Approved | <fill-in> |
| QA (self-review or external) | <fill-in> | [ ] Approved | <fill-in> |
| Release Manager | <fill-in> | [ ] Approved | <fill-in> |

---

## Final Decision

**GO / NO-GO**: <fill-in>

**Rationale**: <fill-in: summary. If NO-GO, list specific blocking items + ETA to resolve.>

**Conditions on release** (if any): <fill-in>

---

## Post-Release

- [ ] Tag release in git: `v<fill-in>`
- [ ] Trigger [release-notes.md](release-notes.md) publication
- [ ] Schedule 72-hour watch window (crash reports, user reviews)
- [ ] Schedule post-release [postmortem.md](postmortem.md) within 2 weeks

---
description: Load the Steam launch checklist + verify progress against it
argument-hint: "[platform: steam | mac | pc | all]"
---

# /release-checklist — pre-launch validation

Comprehensive pre-release checklist: build verification, Steam certification requirements, store metadata, launch readiness. Produces a ticked checklist — incomplete items block launch.

**Phase:** 7 (Release). Output: `reviews/release-checklist-<version>-<date>.md`.

## Procedure

1. **Parse args.** Platform (default `steam` for blueprint; `all` runs every platform's checks).
2. **Load** project version from `CLAUDE.md` or `ProjectSettings/ProjectSettings.asset` (Unity `bundleVersion`).
3. **Load** current milestone from `SPEC.md` + `production/milestones/` — establishes what should be in this release.
4. **Scan codebase:**
   - `TODO` / `FIXME` / `HACK` counts across `Assets/_Project/Scripts/**`
   - `Debug.Log` calls left in release code (flag for removal)
   - `UNITY_EDITOR`-only code referenced from runtime (flag)
5. **Generate checklist** covering:

   ### Codebase health
   - [ ] TODO count ≤ acceptable threshold (list top 5 if over)
   - [ ] FIXME count = 0 (every FIXME is a potential blocker)
   - [ ] HACK count = 0 or documented
   - [ ] No `Debug.Log` in runtime code paths

   ### Build verification
   - [ ] Clean build on target platforms
   - [ ] Zero compiler warnings
   - [ ] All addressables/assets included + load correctly
   - [ ] Build size within budget (from `TECH_APPROACH.md`)
   - [ ] Build version matches `CLAUDE.md` version string
   - [ ] Build reproducible from tagged commit

   ### Quality gates
   - [ ] Zero S1 bugs open
   - [ ] Zero S2 bugs open (or documented exceptions)
   - [ ] `/smoke-check` PASS on tagged build
   - [ ] `/regression-suite audit` PASS
   - [ ] All MVP stories Complete

   ### Steam-specific (if `steam` or `all`)
   - [ ] Steamworks SDK integrated + tested
   - [ ] App ID configured
   - [ ] Store page assets: capsules, screenshots, trailer (check `steam-release/store-assets/`)
   - [ ] Store description + tags complete
   - [ ] Age rating / content descriptors accurate
   - [ ] Depot configuration correct
   - [ ] Branch strategy: `default`, `beta`, `release`
   - [ ] Achievements registered in Steamworks
   - [ ] Cloud save config if applicable

   ### Legal + compliance
   - [ ] Asset-licensing tracker complete (every 3rd-party asset has a row)
   - [ ] EULA / privacy policy in build
   - [ ] Open-source attribution screen if applicable

   ### Accessibility
   - [ ] Declared accessibility items in `design/accessibility.md` all verified
   - [ ] Remappable controls
   - [ ] Scalable text / colorblind-friendly UI

   ### Launch ops
   - [ ] Day-one patch plan (if any)
   - [ ] Rollback procedure documented
   - [ ] Support contact / issue-intake channel defined

6. **Cross-check** against last `/milestone-review` for consistency.
7. **Write** `reviews/release-checklist-<version>-<date>.md` with all items ticked / unticked.
8. **Verdict:** READY / BLOCKED (unticked items list). If READY and auto mode is not active, require explicit user sign-off before recommending launch.

## If args provided

- `steam` / `mac` / `pc` / `all` — platform scope

## If project is not at Phase 7

Warn: "Project is at phase X. Release checklist is meaningful only near launch. Continue anyway?"

## Output

- `reviews/release-checklist-<version>-<date>.md`
- Console: READY / BLOCKED + unticked items

## Related

- Typical follow-ups (READY): tag release, user pushes to Steam default branch
- Typical follow-ups (BLOCKED): `/hotfix` for S1/S2; remediate checklist items
- Invokes agents: `release-manager` if installed; else `producer`; else `qa-lead`
- Invokes skills: cross-checks `/smoke-check`, `/regression-suite`, `/milestone-review`
- Reads files: `CLAUDE.md`, `SPEC.md`, `production/milestones/**`, `steam-release/**`, `asset-licensing-tracker.csv`, `design/accessibility.md`
- Writes files: `reviews/release-checklist-<version>-<date>.md`

# Backup & Restore — Final Whistle

Operational policy for what gets backed up where, and how to restore from each.

Authored 2026-04-24 (Phase 1). Updated when tooling or storage-destinations change.

---

## 1. What gets backed up — and where

| Asset class | Authoritative store | Local backup | Cloud backup | Notes |
|---|---|---|---|---|
| Code + design docs + scripts | GitHub (private repo) | Time Machine | n/a | Git history IS the backup; Time Machine catches uncommitted work |
| Content-pack JSON (generated + hand-authored) | GitHub | Time Machine | Periodic zip before destructive imports | Small; tracked in git |
| Content-pack compiler prompts + seeds | GitHub | Time Machine | n/a | Reproducible via the AI Content Compiler — see `design/player-generation.md` |
| Unity Library/ cache | **nothing — regenerable** | — | — | Explicitly excluded from backup |
| Unity binary assets (textures, audio, FBX) | GitHub via Git LFS | Time Machine | Optional cloud mirror if pipeline demands | Git LFS only turned on when a binary asset lands |
| Blender / Aseprite / DAW source files | Local workspace | Time Machine | Zip before destructive imports | Post-EA 3D push only; pre-EA mostly empty |
| AI prompt outputs (GPT Image 2 concepts, Suno/Udio stems) | Local `raw-inputs/` (gitignored) | Time Machine | Zip per milestone | Reproducible from the prompt + seed manifest |
| Secrets (API keys, Unity license) | 1Password | — | — | NEVER in git; NEVER in Time Machine readable path |
| Steam Direct receipts / contracts | 1Password + local encrypted archive | Time Machine (encrypted) | — | Phase-8 artifact |

---

## 2. Rules

1. **Git-trackable always wins.** If an artifact can live in git (text, small JSON, scripts), put it in git. Don't invent a new backup target.
2. **Regenerable ≠ backup-worthy.** `Library/`, `Temp/`, `Obj/`, and any artifact reproducible by re-running a script or compiler is NOT backed up.
3. **Secrets never leave 1Password.** No `.env`, no plaintext key dumps, no committed config with credentials. `.env.local` is gitignored; if it ever appears in `git status`, treat as a security incident.
4. **Pre-destructive-import snapshot.** Before any content-pack regeneration, schema-bumping migration script, or bulk asset re-import, create a local zip in `~/backups/finalwhistle/YYYYMMDD-HHMM-reason.zip`. Minimum-effort insurance; delete at end of phase if unused.
5. **Cloud mirror only when local is insufficient.** No default cloud backup beyond GitHub. If a specific asset class (e.g., Blender project files at Phase 9) outgrows Time Machine, document the new target here BEFORE turning it on.
6. **No cross-repo sync automation at MVP.** A second git remote at Phase 8 is the first acceptable mirror — not before.

---

## 3. Restore paths

### Clean-machine restore (full)

1. Install dev toolchain per `SETUP.md §4`.
2. `git clone git@github.com:osagberg/FinalWhistle.git`
3. `git lfs install && git lfs pull`
4. Restore secrets from 1Password into `.env.local` + Unity license activation + any Asset Store receipts.
5. Open in Unity Hub; let it regenerate `Library/`.
6. Run `scripts/fw verify-docs` to confirm doc integrity.
7. Run `scripts/fw test` (once `MatchSim.Tests` exists) to confirm determinism.

### Content-pack rollback

Roll back via `git` — every pack version is a commit. `git checkout <pack-version-tag> -- content/` restores the pack; re-run the compiler to confirm canonical artifact matches.

### Save-file restore (developer / tester side)

Save files live in the standard Unity persistent-data path per-platform. Save-slot layout and migration policy live in `design/event-sourced-memory.md` (load-time forward migration). Users can copy `*.sav` into `persistentDataPath/saves/` and the game migrates on load. No cloud save at MVP.

### Crash-bundle triage (Phase 5+)

In-build "Export bug bundle" produces a zip with save + replay seeds + rolling logs + settings + content-pack version. Developer unzips in a dedicated `triage/` workspace, reproduces via `scripts/fw replay <seed>` where applicable.

---

## 4. Time Machine configuration

- **Include:** `~/dev/football/` + `~/Applications/` + `/Library/Preferences/`
- **Exclude:** Any `Library/` / `Temp/` / `Obj/` / `node_modules/` / `Build/` / `Logs/` subdirectory within the project tree.
- **Frequency:** Hourly (Time Machine default).
- **Retention:** Default Time Machine rolling retention.

Add excludes via System Settings → General → Time Machine → Options. Audit quarterly; Unity regenerates `Library/` from version-pinned state.

---

## 5. Verification

Quarterly or before a major milestone (EA launch, 1.0 launch), perform a full clean-machine restore on a secondary Mac (or a fresh user account) following §3. Any step that requires manual recovery of data not in GitHub or 1Password is a backup-policy gap — document it here and close it before the milestone ships.

---

## 6. References

- `SETUP.md` — dev toolchain install procedure.
- `design/production-pipeline.md` §backup-policy — higher-level framing.
- `design/event-sourced-memory.md` §migration — save-file migration rules.
- 1Password vault: `Final Whistle` (secrets + receipts + Unity license).

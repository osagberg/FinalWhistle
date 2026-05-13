# .claude/rules — path-scoped rules

Each subdirectory holds a `RULES.md` that applies to a specific path scope. Claude loads the rule file when editing matching paths (Claude Code's path-scoped rules pattern, ported from the Unity blueprint with Rust/Tauri/SolidJS content).

| Rule file | Scope |
|---|---|
| `Rust/RULES.md` | All Rust source — applies to every crate |
| `Sim/RULES.md` | Sim crates (fw-core, fw-match-sim, fw-content, fw-scouting, fw-memory, fw-replay, fw-save). Determinism non-negotiables. |
| `Tauri/RULES.md` | `crates/fw-tauri/**` + `src-tauri/**`. IPC patterns + async-OK. |
| `Frontend/RULES.md` | `frontend/**`. SolidJS + Tailwind v3 + TanStack v8 + PixiJS v8 + ECharts. |
| `Content/RULES.md` | `content/**`. RON authoring + content-pack IDs + banned terms. |
| `design-docs/RULES.md` | `docs/**`. Design-doc writing conventions. |

## Activation

Rules use `applies_to:` glob patterns in their YAML frontmatter. The intent is opt-in lazy loading: a `Sim/RULES.md` is auto-cited by Claude when an edit lands in `crates/fw-match-sim/**`. Treat the frontmatter as advisory until Claude Code formalizes the auto-load contract.

## Why slim from blueprint

Blueprint had Unity-specific rule dirs: `CSharp/`, `Assemblies/`, `Addressables/`, `ScriptableObjects/`, `Scripts/`, `design-docs/`, `tests/`. Final Whistle drops all 5 Unity-specific dirs and replaces with the 6 above. The `Scripts/` dir from the blueprint is being retired — its general bash-script rules are absorbed into `Rust/RULES.md` since most automation scripts in this repo are bash front-doors to cargo/pnpm.

See `docs/BLUEPRINT_RECONCILE.md` for the full audit.

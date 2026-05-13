# templates/design-templates — Final Whistle doc templates

Copy a template into the appropriate destination, fill `{{PLACEHOLDER}}` slots, commit.

| Template | When to use | Destination |
|---|---|---|
| `architecture-decision-record.md` | One per architecture decision | `docs/adr/NNNN-<slug>.md` |
| `game-concept.md` | Greenlight gate for a new feature or pivot | `docs/design/<feature>-concept.md` |
| `game-design-document.md` | Full per-system spec | `docs/design/<system>.md` |
| `game-pillars.md` | Reaffirming or revising the 5 pillars | `docs/DESIGN_DOC.md` §3 |
| `systems-index.md` | TOC of in-game systems | `docs/design/systems-index.md` |
| `test-plan.md` | Test strategy per system | `docs/specs/<system>-test-plan.md` |
| `playtest-report.md` | Internal solo-dev playtest log | `docs/playtests/YYYY-MM-DD-<seed>.md` |
| `postmortem.md` | Phase boundary retrospective | `docs/postmortems/phase-T<N>.md` |

These templates are slimmed adaptations of the blueprint's `templates/design-templates/` (originally Unity-focused). All Unity / 3D / Addressables / ScriptableObjects references stripped.

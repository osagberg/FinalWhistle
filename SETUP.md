# SETUP.md — Final Whistle setup procedure

> Single authoritative procedure for setting up machine + accounts + project from scratch. Referenced by `CLAUDE.md` + Phase 1 tasks.
>
> Authored 2026-04-22. Updated whenever tooling / accounts change.

---

## 1. Pre-flight

**Accounts needed:**

| Account | Cost | When needed | Notes |
|---|---|---|---|
| GitHub | free | Phase 1 | source control + CI + Issues |
| Unity ID | free | Phase 1 | engine licensing + Asset Store |
| Steamworks | **$100 one-time Steam Direct** | Phase 8 | per-title submission fee |
| itch.io | free | Phase 4 (closed beta) | demo + tester distribution |

Accounts intentionally **not** required (despite blueprint default):
- Apple Developer ($99/yr) — no Mac App Store plan; Steam Mac builds don't need it
- Adobe / Mixamo — not part of the current 3D candidate stack; Phase-5 license audit decides any animation/asset-service accounts

**Hardware prerequisites:**
- Apple Silicon Mac (M1+) — user's primary dev box
- 16+ GB RAM (32+ recommended for full balance-harness sweeps)
- 250+ GB free disk (Unity + Library/ cache grows fast)

---

## 2. Machine inventory

Track installed tooling — gets out of sync otherwise.

| Tool | Version | Install path | Notes |
|---|---|---|---|
| Unity Editor | pin at Phase 3 | `/Applications/Unity/Hub/Editor/<version>/` | Use Unity Hub |
| Blender | pin at Phase 3 | `/Applications/Blender.app` | Deferred-3D pipeline; Phase 3 install only |
| VS Code / Rider | current | / | C# editor |
| .NET SDK | `10.0.202` via `global.json` roll-forward | `/usr/local/share/dotnet/` | MatchSim + xUnit test host |
| Claude Code | current | `/usr/local/bin/claude` | CLI |
| gh CLI | present | `/opt/homebrew/bin/gh` | GitHub workflow |
| git-lfs | present | `/opt/homebrew/bin/git-lfs` | Unity binary assets |
| jq | present | `/opt/homebrew/bin/jq` | JSON munging |
| node / npx | present | `/opt/homebrew/bin/node` | MCP servers |
| python3 | system | `/usr/bin/python3` | hook scripts |

`/audit` flags drift. Update when versions change.

---

## 3. Budget tier — current: Tier 1 (bootstrap)

Philosophy: **buy-on-pain.** Don't pre-purchase. Each tier escalates when a specific pain hits.

### Tier 1 — Bootstrap (current — Phase 0-4 default)

- **One-time spent:** $0 (Magica Cloth 2 @ $50 already owned pre-project)
- **Monthly:** $0
- Contents: free Unity + free URP + UI Toolkit (built-in) + FMOD free-indie + Steamworks.NET + GPT Image 2 (existing subscription used for concepts)
- Exits tier when: specific Phase-trigger pain hits

### Tier 2 — Pipeline horsepower (Phase 3-6 escalation)

- **One-time:** $200-400
- Contents by trigger:
  - Odin Inspector ($55) — Phase 3 SO-authoring pain at scale
  - Animancer Pro ($80) — Phase 4 Mecanim state-graph breakdown for signature authoring
  - DOTween Pro ($15) — Phase 5 UI juice
  - Additional Asset Store one-offs as pain hits

### Tier 3 — 3D-pipeline production (now Phase-5/6+ candidate-shipping path)

**2026-04-26 visual-target supersession** moved Tier 3 activation from "post-EA Phase 9 audience-signal gate" to **Phase-5/6 production-feasibility spike per `design/3d-pipeline.md`**. Cel-shaded 3D is a candidate shipping visual; tooling activates incrementally:

- **Phase-5 spike preparation** — license-audit ALL 3D tooling commercial-rights before any subscription activates. Per supersession §(e) + `FW-VAL-D-011` content-pack-validator check. Free-tier tools used for spike experimentation; commercial tier activates only when committed for shipping content.
- **Phase-5 spike subscriptions** (only if license-audit passes):
  - 3D-asset-generator candidate: e.g. Tripo Pro $20-30/mo (commercial-tier required; verify Steam-commercial coverage before activation)
  - AI-assisted animation tool candidate: e.g. Cascadeur Pro / Teams (Free is non-commercial + export-limited)
  - Reference / fallback model gen: e.g. Hunyuan3D (open-weights; commercial coverage needs license re-read)
- **Phase-6 production scaling** (only if spike-green outcome):
  - Add Rodin Gen-2 (Hyper3D) $20-60/mo credits for hero assets (stadium / trophy / cup-final-context)
  - Suno / Udio $10-30/mo for music pass (Phase-6 audio)
  - ElevenLabs $5-22/mo only if commentary VO becomes player-data-demanded (post-EA at earliest)
- **Magica Cloth 2** ($50 owned) activates at Phase-5 spike (cloth simulation for kit + hair).
- **Specific tooling names** above are current candidates; final stack locks at Phase-5 license-audit. Vendor-agnostic per supersession §(d) — substitutions allowed if license / quality / pipeline-fit warrants.

**Phase-9 R&D path** still exists as fallback — if Phase-5/6 spike fails terminally and dots ships at EA, post-EA 3D R&D evaluation runs at Phase 9 audience-signal gate (original 2026-04-22 framing).

**Hard rule revised:** no monthly subscriptions before Phase 5 license-audit completes unless explicitly signed off in a NEW decisions-log entry. Tier 1 + Tier 2 unchanged in their phase triggers.

See `~/dev/blueprint/patterns/budget-tier-trigger-table.md` for discipline.

---

## 4. Install list (Phase 1)

Run top-to-bottom:

1. **Unity Hub** — download from unity.com
2. **Unity Editor** — via Unity Hub, pin exact version at Phase 3 kickoff (latest 6 LTS at that moment)
3. **Build support** — Mac Build Support, Windows Build Support, Linux Build Support (WebGL deferred)
4. **Source editor** — VS Code with C# extension (or Rider)
5. **.NET SDK 10** — required for `scripts/fw test` / `scripts/fw verify`; repo pins SDK feature band in `global.json`
6. **Blender** — Phase 3 install only (deferred 3D pipeline); not needed for MVP
7. **Claude Code** — already installed
8. **gh CLI** — already installed; verify `gh auth status` is green
9. **git LFS** — already installed; `git lfs install` once per clone
10. **Version pin all of the above** in §2 machine inventory table

---

## 5. MCP + tooling install (Phase 1 — already done at bootstrap)

Installed (verified at bootstrap):

```
context7       ✓ already present
github         ✓ already present
blender-mcp    ✓ already present
```

**Intentionally skipped:**
- `chrome` — Claude Code has WebSearch + WebFetch; no genuine gap
- `desktop-commander` — Claude Code has Read/Write/Edit/Bash; no genuine gap

**Phase 3 — installed 2026-04-28:**
- `UnityMCP` (CoplayDev `com.coplaydev.unity-mcp`) — project-scoped; installed via `unity-project/Packages/manifest.json` (pinned to commit SHA) + Unity-side `Window → MCP for Unity → Configure` writes the `UnityMCP` (Pascal) entry to `.mcp.json` on `http://localhost:8080/mcp`. CoplayDev's `manage_script` tool already calls `AssetDatabase.ImportAsset` + `RequestScriptCompilation` internally, so no PostToolUse refresh hook is needed (the previous `refresh-unity-on-script.sh` was removed in commit `47997fc`).

Verify current state: `claude mcp list` — all three baseline MCPs green.

---

## 6. Project kickoff (Phase 1)

Once machine + tooling is green:

```bash
cd /Users/vibelogic/dev/football/

# 1. Create GitHub remote (private)
gh repo create osagberg/FinalWhistle --private --source=. --remote=origin
# NOTE: The `vibelogic` GitHub org exists (reserved as a potential future
# publisher namespace) but neither authenticated gh account is a member,
# so repo creation under that org is blocked. Personal-namespace repo is
# simpler and transferrable one-click at Phase 8 if a publisher namespace
# is wanted for Steam branding.

# 2. Smoke-test Claude Code
claude
# In fresh session:
# /plugin install feature-dev
# /plugin install pr-review-toolkit
# /plugin install hookify
# /status   (reads STATUS.md cleanly)
# /next     (picks up first Phase 0 design-question task)
```

---

## 7. Privacy / security posture

- `.gitignore` covers Unity Library/ + Temp/ + secrets
- Never commit API keys — use `.env.local` (gitignored)
- GitHub repo private until Phase 8 (marketing launch)
- Asset Store receipts / license keys stored in 1Password, NOT in repo
- Asset licensing tracker (`steam-release/asset-licensing-tracker.csv`) mandatory from first paid tool purchase
- AI-generated content: ship with Steam's disclosure metadata enabled per Valve's 2025 policy

---

## 8. Backup strategy

- Local: Time Machine (Mac)
- Remote: GitHub (code) + cloud storage (large binary assets not in LFS)
- Unity Library/ does NOT need backup
- Blender source files from the Phase-5/6 3D spike or any later 3D production DO need backup
- AI content pack inputs (prompts + seeds) version-controlled; reproducible via compiler

---

## 9. Smoke test (end of Phase 1)

Before transitioning to Phase 2, verify:

- [ ] `claude mcp list` — all baseline MCPs green
- [ ] `/status` in Claude Code — reads STATUS.md, reports Phase 0 / Phase 1 correctly
- [ ] `/next` — picks up the next unblocked Phase 1 or Phase 2 task
- [ ] `/log-decision test decision` — successfully appends to decisions log
- [ ] `git log -3` — see bootstrap + setup commits
- [ ] GitHub Actions CI stub passes on noop commit
- [ ] All Phase 1 tasks `[x]` in SPEC.md

---

## 10. Trigger table (deferred purchases + actions)

Deliberately NOT decided at Phase 1 — each has a specific trigger for re-evaluation:

| Item | Cost | Trigger | Phase | Tier |
|---|---|---|---|---|
| Odin Inspector | $55 | SO-authoring pain at Phase 3+ | 3 | 2 |
| Animancer Pro | $80 | Mecanim state-graph breakdown at signature-authoring | 4 | 2 |
| DOTween Pro | $15 | UI juice pain | 5 | 2 |
| FinalIK | $90 | Hand-contact / foot-IK needed | Phase-5 spike only if manual/AI-assisted rigging fails | 3 |
| Auto-Rig Pro (Blender) | $40 | Custom skeleton auto-binding for 3D | Phase-5 spike only if base Blender rigging is insufficient | 3 |
| Magica Cloth 2 | $50 (owned) | First cloth-sim scene (kit / hair flutter) | **Phase-5 spike** (was Post-EA; superseded 2026-04-26) | 3 |
| 3D-asset generator (e.g. Tripo Pro) | $20-30/mo | Bulk player/kit generation; commercial tier required | **Phase-5 spike + Phase-6 production** (was Post-EA; superseded 2026-04-26) | 3 |
| Hero-asset 3D generator (e.g. Rodin Gen-2 / Hyper3D) | $20-60/mo | Hero stadium/trophy/cup-final context | **Phase-6 production** (post-spike-green; superseded 2026-04-26) | 3 |
| Hunyuan3D (open-weights) | free | Reference / fallback model gen; commercial coverage needs license re-read | **Phase-5 spike + Phase-6 production** | 3 |
| Suno / Udio | $10-30/mo | Music pass | Phase 6 | 2 |
| AI-assisted animation tool (e.g. Cascadeur Pro/Teams) | $24/mo | AI-assisted signature-move animation; Free tier is non-commercial + export-limited | **Phase-5 spike + Phase-6 production** (was Post-EA; superseded 2026-04-26) | 3 |
| ElevenLabs | $5-22/mo | Commentary VO evaluation | Post-EA (conditional on player demand) | 3 |
| Amplify Shader Editor | $80 | Custom shader beyond Shader Graph | Phase-5/6 only if Shader Graph cannot hit cel-shader target | 3 |
| Substance Painter | $250 | PBR texture authoring | Post-spike only if generated/editable texture workflow fails | 4 |
| Steam Direct | $100 | Phase 8 — Steam submission | 8 | mandatory |
| Code-signing cert (Win) | $100-500/yr | Phase 8 if SmartScreen avoidance matters | 8 | optional |

Re-evaluate at each phase transition. Tier-3 subscriptions activate only after the Phase-5 commercial-license audit and only for the narrow spike task being tested; production subscriptions require a spike-green outcome.

---

*Authored 2026-04-22. Updated whenever tooling / accounts change.*

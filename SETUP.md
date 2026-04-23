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
- Adobe / Mixamo — no 3D at MVP; deferred indefinitely

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

### Tier 3 — Post-EA 3D push (only if audience-signal gate passes at Phase 9)

- **Monthly subscriptions (rotating during content phases):**
  - Tripo v3.0 $20-30/mo (bulk player meshes with auto-rig)
  - Rodin Gen-2 (Hyper3D) $20-60/mo credits (hero assets)
  - Suno / Udio $10-30/mo (music pass)
  - Cascadeur $24/mo (signature-move animation)
  - ElevenLabs $5-22/mo (only if player data demands commentary VO)

Tier 3 does NOT activate at EA. Explicitly gated behind audience-signal review at Phase 9.

**Hard rule:** no monthly subscriptions before Phase 9 unless explicitly signed off in a NEW decisions-log entry.

See `~/dev/blueprint/patterns/budget-tier-trigger-table.md` for discipline.

---

## 4. Install list (Phase 1)

Run top-to-bottom:

1. **Unity Hub** — download from unity.com
2. **Unity Editor** — via Unity Hub, pin exact version at Phase 3 kickoff (latest 6 LTS at that moment)
3. **Build support** — Mac Build Support, Windows Build Support, Linux Build Support (WebGL deferred)
4. **Source editor** — VS Code with C# extension (or Rider)
5. **Blender** — Phase 3 install only (deferred 3D pipeline); not needed for MVP
6. **Claude Code** — already installed
7. **gh CLI** — already installed; verify `gh auth status` is green
8. **git LFS** — already installed; `git lfs install` once per clone
9. **Version pin all of the above** in §2 machine inventory table

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

**Deferred to Phase 3:**
- `unity-mcp` (CoplayDev) — project-scoped; install after Unity project exists via `Packages/manifest.json` + `claude mcp add -s project unity-mcp <launcher-command>`

Verify current state: `claude mcp list` — all three baseline MCPs green.

---

## 6. Project kickoff (Phase 1)

Once machine + tooling is green:

```bash
cd /Users/vibelogic/dev/football/

# 1. Create GitHub remote (private)
gh repo create Vibelogic/FinalWhistle --private --source=. --push
# NOTE: --push pushes existing commits. Bootstrap deliberately did NOT push.

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
- Blender source files (post-EA 3D push) DO need backup
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
| FinalIK | $90 | Hand-contact / foot-IK needed | Post-EA 3D push only | 3 |
| Auto-Rig Pro (Blender) | $40 | Custom skeleton auto-binding for 3D | Post-EA 3D push only | 3 |
| Magica Cloth 2 | $50 (owned) | First cloth-sim scene | Post-EA 3D push | 3 |
| Tripo v3.0 | $20-30/mo | Bulk player/kit generation | Post-EA 3D push | 3 |
| Rodin Gen-2 (Hyper3D) | $20-60/mo | Hero stadium/trophy | Post-EA 3D push | 3 |
| Hunyuan3D (via Blender MCP) | free | Anime-stylized model prototyping | Post-EA 3D push | 3 |
| Suno / Udio | $10-30/mo | Music pass | Phase 6 | 2 |
| Cascadeur | $24/mo | AI-assisted signature-move animation | Post-EA 3D push | 3 |
| ElevenLabs | $5-22/mo | Commentary VO evaluation | Post-EA (conditional on player demand) | 3 |
| Amplify Shader Editor | $80 | Custom shader beyond Shader Graph | Post-EA 3D push | 3 |
| Substance Painter | $250 | PBR texture authoring | Post-EA 3D push | 4 |
| Steam Direct | $100 | Phase 8 — Steam submission | 8 | mandatory |
| Code-signing cert (Win) | $100-500/yr | Phase 8 if SmartScreen avoidance matters | 8 | optional |

Re-evaluate at each phase transition. Most Tier-3 subscriptions only activate if Post-EA 3D push is greenlit via audience-signal gate at Phase 9.

---

*Authored 2026-04-22. Updated whenever tooling / accounts change.*

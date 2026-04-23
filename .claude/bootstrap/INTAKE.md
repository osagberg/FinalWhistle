# Bootstrap intake — questionnaire

> Read by Claude during `/bootstrap` Phase A. Questions to ask the user, one at a time, conversationally.

## Interview style

- **One question at a time.** Never batch.
- **Accept vague answers.** Don't grill. "Not sure yet" + brief expansion is enough.
- **Use examples.** If a question lands flat, offer 2-3 reference games to calibrate.
- **Summarize at the end.** Recap in 5-10 lines before proceeding.

---

## Question 1 — Project name

> "What's the working title of the game? (Tentative is fine — it can change. But pick something Googleable and not already on Steam.)"

Examples to offer if user blanks:
- Evocative single words ("Hollow", "Verdant")
- Two-word compounds ("Shadow Garden", "Velvet Forge")
- Setting-anchored ("Ironhold Academy", "Ashbourne")

Record: `project_name`

---

## Question 2 — Studio / author name

> "What studio or author name do you want on the credits + Steam page? Your personal name is fine for solo."

Record: `studio_name`

---

## Question 3 — Genre / shape

> "What shape of game? Pick a combination of tags that fits:
>
> - **Core loop:** action / puzzle / narrative / sim / strategy / sandbox / roguelike / RPG
> - **Perspective:** first-person / third-person / top-down / side-scroll / isometric
> - **Tone:** horror / cozy / dark / comedic / melancholic / mystery / adventure
>
> And 2-3 reference games that give the clearest vibe (e.g., 'Disco Elysium meets Signalis'). Reference games matter more than genre tags."

Record:
- `genre_tags` (list)
- `reference_games` (list, 2-3)

---

## Question 4 — Character fidelity

> "How do characters look in-game?
>
> 1. **No characters** (abstract / puzzle / first-person-no-avatar)
> 2. **2D sprites** (pixel / hand-painted / vector)
> 3. **3D stylized low-poly** (A Short Hike / Sable)
> 4. **3D anime** (Genshin / Honkai / Under the Witch — VRoid pipeline)
> 5. **3D PBR realistic** (walking-sim aesthetic)
> 6. **Mix** (describe)"

Record: `character_fidelity` (one of: `none`, `2d`, `3d_lowpoly`, `3d_anime`, `3d_pbr`, `mix`)

If `3d_anime` — ask: "VRoid or bespoke characters?" → record `character_pipeline` = `vroid` | `bespoke` | `mixamo`.

---

## Question 5 — Narrative weight

> "How much dialogue / story?
>
> 1. **None** (pure mechanics — Tetris / Baba Is You style)
> 2. **Light** (item descriptions, collectible lore, no dialog runner needed)
> 3. **Medium** (scripted cutscenes, some branching)
> 4. **Heavy** (branching dialogue trees, many characters with distinct voices)"

Record: `narrative_weight` (one of: `none`, `light`, `medium`, `heavy`)

If `heavy` — ask: "Rough character count? 3-5 / 6-12 / 13+?" → record `character_count_estimate`.

---

## Question 6 — Scope

> "Honest scope — how big is this?
>
> 1. **Tiny** (1-2 scenes, game-jam-sized, <5 hrs playtime)
> 2. **Small** (few systems, one chapter, 2-6 hrs playtime)
> 3. **Medium** (full indie — one cohesive game, 8-20 hrs playtime)
> 4. **Large** (20+ hrs, multi-chapter, comparable to Hollow Knight scope)
>
> Medium is the right answer for most solo-dev first-shippables. Large is usually wrong for first project."

Record: `scope` (one of: `tiny`, `small`, `medium`, `large`)

If `large` — gently flag: "Large-scope solo projects have a high attrition rate. Confident?"

---

## Question 7 — Commercial intent

> "Where does this ship?
>
> 1. **Steam release** (primary target — most solo indies)
> 2. **itch.io only** (no Steam Direct fee; smaller audience)
> 3. **Steam + itch.io + other storefronts**
> 4. **Private / unlisted** (portfolio / gift / personal)
> 5. **Not sure yet** (default to 'Steam-release intent' — easier to back off than to retrofit)"

Record: `commercial_intent` (one of: `steam`, `itch`, `multi`, `private`, `unsure`)

If `steam` or `multi` — note that Phase 8 includes $100 Steam Direct prerequisite.

---

## Question 8 — Platforms

> "Which platforms at launch?
>
> - Windows (almost always yes)
> - Mac (yes by default on Apple Silicon; easy to ship)
> - Linux (easy via Unity, recommended for Steam Deck)
> - Steam Deck verified (worth pursuing for indie visibility)
> - Consoles (defer to post-launch unless you have specific reason)
> - Mobile (different design — defer)"

Record: `platforms` (list)

---

## Question 9 — Dev model

> "Solo or team?
>
> 1. **Solo** (you do everything)
> 2. **Solo + occasional contractors** (voice acting, music, art)
> 3. **Small team** (2-5 people collaborating)
> 4. **Remote collaboration via git** (everyone pushes to shared repo)"

Record: `dev_model` (one of: `solo`, `solo_plus_contractors`, `small_team`, `remote_collab`)

---

## Question 10 — Budget tolerance

> "Rough budget for tools + assets + services, one-time + over-lifetime:
>
> 1. **Bootstrap** ($0-50 total — free tools only)
> 2. **Lean** ($50-200 — a handful of essentials)
> 3. **Standard indie** ($200-500 — can buy quality plugins as pain arises)
> 4. **Comfortable** ($500-1500 — full toolkit including Animancer / FinalIK / quality assets)
> 5. **No constraint** ($1500+ — buy what's needed)
>
> Covers tool/plugin/asset cost — not labor / contractors. Steam Direct $100 at Phase 8 is separate."

Record: `budget_tier` (one of: `1_bootstrap`, `2_lean`, `3_standard`, `4_comfortable`, `5_unconstrained`)

---

## Question 11 — Content rating target

> "Age rating target for store pages?
>
> 1. **E / PEGI 3-7** (family-friendly — broadest audience)
> 2. **T / PEGI 12** (mild violence / language — core teen/adult market)
> 3. **M / PEGI 16-18** (stronger violence / language / mature themes)
> 4. **Adult-only** (explicit content — Steam-only realistically, ~40% smaller market)
>
> Honest answer — affects store page strategy and content-policy doc."

Record: `content_rating` (one of: `e`, `t`, `m`, `adult`)

---

## Question 12 — Context tier + default scope

> "What's your Claude Code context window capability?
>
> 1. **200K** (standard tier — most accounts)
> 2. **1M** (1M-context tier — recent rollout; pick if your account has it)
> 3. **Unsure** (I'll detect from `~/.claude/tier-capabilities.json` if present, otherwise default to 200K)"

**Pre-check:** before asking, read `~/.claude/tier-capabilities.json` if present; it may declare `context_window` already. If found, announce: "I see `~/.claude/tier-capabilities.json` says `<tier>` — keep or override?"

Record: `context_window` (one of: `200K`, `1M`, `unknown`)

Follow-up:
> "Default scope preference?
>
> - **minimal** — bare orientation (CLAUDE.md + SPEC.md + STATUS.md); for quick-fix sessions
> - **standard** — v1 blueprint behavior; core docs + 6 base slash commands (good default for 200K-tier)
> - **rich** — standard + 14-agent studio + 25 extended slash commands + 17 rule files + statusline (recommended default for 1M-tier)
> - **studio** — rich + extended agent roster + sprint management patterns (opt-in at Phase 6+ content scaling)
> - **research** — studio + full reference library loaded (Phase 0 kickoff / Phase 2 design bible / Phase 8 launch prep)
>
> Guidance: **1M-tier → rich**; **200K-tier → standard**; if you're doing Phase 0 or Phase 2 work on 1M, `research` is a better starting point. You can re-scope anytime via `/expand-studio` / `/deep-research` / `/contract-scope`."

Record: `default_scope` (one of: `minimal`, `standard`, `rich`, `studio`, `research`)

---

## Question 13 — Pitch in one sentence

> "If you had to pitch this in ONE sentence to a publisher / press contact, what would it say? It's OK if it's rough — we'll refine in Phase 0. Just a first swing."

Record: `pitch_one_liner`

---

## Summary message template

After all 13 questions, produce a message like:

```
Got it. Here's what I heard:

  Project: {project_name} by {studio_name}
  Genre: {genre_tags} — reference {reference_games}
  Pitch: {pitch_one_liner}

  Characters: {character_fidelity}{+" ("+character_pipeline+")" if set}
  Narrative: {narrative_weight}{+" with ~"+character_count_estimate+" characters" if set}
  Scope: {scope} — {playtime estimate derived from scope}

  Commercial: {commercial_intent}
  Platforms: {platforms}
  Rating: {content_rating}

  Dev model: {dev_model}
  Budget tier: {budget_tier}

  Context tier: {context_window}
  Default scope: {default_scope} (re-scope anytime via /expand-studio, /deep-research, /contract-scope)

Does that look right? Any corrections before I customize the project?
```

Wait for user confirmation or corrections, then proceed to Phase B.

---

## Five-question fast path

If user says "just figure it out" / "quick setup" / "I'll refine later":

1. Project name?
2. One-sentence pitch?
3. Closest reference game?
4. Scope (tiny / small / medium / large)?
5. Steam release intent (yes / no / unsure)?

From those, Claude makes best-guess defaults:
- Character fidelity: infer from reference game
- Narrative weight: infer from reference game
- Platforms: Windows + Mac + Linux (Steam-default trio)
- Dev model: solo
- Budget tier: `2_lean` (safe starting point)
- Content rating: `t` (safe default; user can revise)
- Context tier: read `~/.claude/tier-capabilities.json` if present; else `200K`
- Default scope: `rich` if tier is `1M`, else `standard`

Flag the defaults in the summary so user can override.

---

## Edge cases

**User says "I don't know yet" on many questions:**
Acceptable. Record `unknown` and proceed. The blueprint is Phase 0 scaffolding — unknowns get filled as the project is clarified.

**User's answers contradict:**
Example: "character-heavy" + "no narrative" — unusual. Flag politely: "Character-heavy with no dialogue is unusual — usually means combat/action focus (Hades-like). Is that right?"

**User pitches something problematic:**
- Real people / celebrities → flag; suggest fictional analogues
- Content that implies minors in sexual context → refuse, suggest aging up + making explicit
- Direct IP clones → flag; suggest original variation
Handle as a gentle redirect, not a lecture. If user insists, log the concern in SPEC.md decisions log and let their judgment stand (except for the minor-content safety floor, which is non-negotiable).

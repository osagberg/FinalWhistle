# open-football: data model + procedural generation

Snapshot of `ZOXEXIVO/open-football` focused on the `database/` crate, generators, and `core::club / core::league / core::transfers` shapes. Headline: this is a **real-data sim with light procedural padding**, not a procedural-world generator. Ships with a hand-curated DB of ~1,076 clubs across 73 leagues plus an external "ODB" player corpus, and only generates players when the ODB doesn't cover a club.

## Player data shape

Skills laid out as a flat `[f32; 37]` work array (`generators/player.rs:15-57`), projected into structured `PlayerSkills` (`player.rs:362-410`):
- **Technical (14):** corners, crossing, dribbling, finishing, first_touch, free_kicks, heading, long_shots, long_throws, marking, passing, penalty_taking, tackling, technique.
- **Mental (14):** aggression, anticipation, bravery, composure, concentration, decisions, determination, flair, leadership, off_the_ball, positioning, teamwork, vision, work_rate.
- **Physical (9):** acceleration, agility, balance, jumping, natural_fitness, pace, stamina, strength, match_readiness; plus `goalkeeping: Default`.

All `f32` on **1–20 scale** (FM-style). `PlayerAttributes { current_ability, potential_ability }` as `u8` on **15–200 CA/PA scale** (`player.rs:674,680`). Fitness state — `condition: i16`, `fitness: i16`, `jadedness: i16` — on 0–10000 internal scale (`player.rs:587-623`). Height/weight `u8` cm/kg. Morale lives in separate `PlayerHappiness`/`PlayerStatus` modules (`core/src/club/mod.rs:107-145`).

## Generation approach

Senior pipeline (`player.rs:706-876`):
1. Blend team/league/country rep → `rep_factor ∈ [0,1]` (weights 0.50/0.30/0.20, `player.rs:655-658`).
2. Pick age uniformly in team-type range (`generator/players.rs:276-283`: Main 17-35, U23 17-23).
3. Target CA = `rep_curve × role_factor × age_factor + Gaussian` (`player.rs:668-675`); Box-Muller normal at `player.rs:62-66`.
4. PA = CA + role/age headroom (`player.rs:677-681`; Prospects 24-55, Stars 4-12).
5. Pick exact position (DC, AMR, WBL…) from bucket biased by PA + team_type + role.
6. Generate skills group-by-group from `(group_mean, position_weight, noise)` with peak-timing + per-skill age cap (`player.rs:109-127`).
7. Apply `SkillAffinities` cross-correlations — passing→vision+first_touch, aggression→bravery (`player.rs:322-353`) — then a **role-archetype overlay** (Poacher / Target Man / Playmaker / Ball Winner — `player.rs:163-312`).
8. Apply **per-country additive bias** (`country_bias.rs:54-65`, e.g. `br` → +2.0 dribbling, +2.0 flair, +1.5 technique).
9. Rescale to target CA; clamp to age cap.

`PositionRoleQueue` pre-quotas Stars/Starters/Rotation/Backup/Prospect/Fringe per team type so quality can't clump on keepers (`generator/players.rs:21-152`). No Markov chains, no LLMs. Names sampled uniformly from per-country pools (`loaders/names.rs:1-15`).

## Real-data vs procedural — be specific

**Overwhelmingly real-data.** Compiled DB at `src/database/src/data/database.db` is a 2.2 MB gzip-compressed JSON doc embedded via `include_bytes!` (`loaders/compiled.rs:25,52-58`). Tests assert **73 enabled leagues and 1,076 enabled clubs** (`data_tree.rs:90-97`) and name actual licensed entities — **Spartak Moscow, Dinamo Moscow, Zenit, Real Sociedad, Real Sociedad B, Ural** — with real league IDs (`data_tree.rs:121-198`). `OdbPlayer` records carry real names, birthdays, country/club, CA/PA, contracts, prior-season history (`played`/`goals`/`rating`), reputation, loan placement (`loaders/players.rs:18-187`).

Procedural generation is the **fallback**: ODB drives senior teams when a club has records (`generator/clubs.rs:39-58`); U18/U19 + free-agent intake always generate (`generator/players.rs:172-273`); clubs without ODB coverage generate fully. Country bias tables (`country_bias.rs:69+`) bake in tropes — jogo bonito for Brazil, German discipline, etc.

## Club + league shapes

`ClubEntity` (`loaders/club.rs:3-32`): id, name, country_id, location.city_id, finance.balance, colors hex, `teams: Vec<ClubTeamEntity>`, `rivals: Vec<u32>`, optional `philosophy`, optional `facilities` (4 string levels), `average_attendance`, plus a `parent_club` pointer the compiler uses to fold satellites like "Ural 2" into the parent as a B team. `ClubTeamEntity` carries reputation (home/national/world `u16`), `team_type: String`, stamped `league_id`. Runtime `Club` (`core/src/club/club/mod.rs:46-71`) adds `board`, `academy`, `transfer_plan`, philosophy enum (DevelopAndSell / SignToCompete / LoanFocused / Balanced), `FacilityLevel`, `TeamCollection`.

Leagues (`loaders/league.rs:9-41`): id, slug, name, country_id, season settings, `reputation: u16`, `tier: u8`, `promotion_spots`, `relegation_spots`, `foreign_players: Vec<ForeignPlayerEntry { country_id, weight }>`, optional `league_group` for multi-group competitions (Serie C A/B/C). National competitions (Euro, World Cup) are separate `NationalCompetitionEntity` with qualifying zones + tournament configs (`generators/convert.rs:7-99`).

## Transfer system summary

`TransferMarket` (`market.rs:8-15`): `listings: Vec<TransferListing>`, `negotiations: HashMap<u32, TransferNegotiation>`, `transfer_window_open: bool`, `transfer_history: Vec<CompletedTransfer>`. Listings carry an `origin` discriminator (SellerListed / LoanOutListed / EndOfContract / SyntheticUnsolicited — `market.rs:47-60`) so synthetic listings for unsolicited approaches don't earn the seller's "willing-to-deal" bonus. Price decay rate-limited to one step per week. `TransferWindowManager` keyed by `country_id` (`window.rs:5-9`).

Most interesting for FW T2-8: the **scouting-region abstraction**. Scouts know regions (`WesternEurope`, `WestAfrica`, `EastAsia` — `scouting_region.rs:8-40`), not country lists. Right shape for FW's biased-scout pillar.

## Persistence + migration

Single embedded gzipped JSON blob parsed once via `OnceLock` (`loaders/compiled.rs:39-50`). Top-level version field, `pub const SUPPORTED_VERSION: &str = "0.01"`, hard panic on mismatch (`compiled.rs:23,59-64`). No migration code — incompatible versions fail to boot. No save-file system in scope; this crate is purely the **world-bootstrap** layer. `OdbPlayer` uses `#[serde(default)]` on optional fields (`players.rs:23-86`) for soft forward-compat against varying scraper output.

## Generation determinism

**Not deterministic.** Generator calls `rand::random::<f32>()` (`player.rs:65,169`) and `IntegerUtils::random(...)` directly with no per-world seed plumbing. Worse: `generate_clubs` is parallelized via `rayon::par_iter` (`generator/clubs.rs:39`, `countries.rs:30`) with thread-local RNG (`clubs.rs:36-38`). Same input → different roster every boot. Hard "do not adopt" — FW's `Sim/RULES.md` §3-§5 forbids `rand::random`, `rayon` in sim crates, and thread-local RNG.

## What's worth adopting for FW T1-1 / T2-3 / T2-4

- **Technical/Mental/Physical/GK grouping with flat-array index constants** (`player.rs:15-57`). Maps onto FW's 32-attribute proposal; keeps gen code readable.
- **Role-archetype overlay on position weights** (`player.rs:163-312`). Position → base weights; archetype roll → Poacher/Target Man flavor. Lift as design intent for FW's signature pillar.
- **Squad-role pre-quota queue** (`generator/players.rs:21-152`). Prevents a 5-star backup keeper from dice clumping.
- **Rep-blend + cubic salary curve** (`player.rs:806-841`). Avoids every-tier-pays-similar-wages.
- **Per-country additive skill bias** (`country_bias.rs`). FW cultures get a `[bias_per_attribute]` overlay layered before clamp. Fits RON trivially.
- **Scouting region abstraction** (`scouting_region.rs`). Regional knowledge beats per-country lists for the biased-scout pillar.
- **CA→PA→skills rescale pipeline** (`player.rs:747-779`). Decouples "how good now" / "how good eventually" / "what does it look like per-attribute."

## What's worth avoiding

- **`rand::random()` + `rayon::par_iter` in gen.** Veto per `Sim/RULES.md` §3-§5. Seed every roll through `ChaCha8Rng::seed_from_tuple(world_seed,…)`; gen stays sync.
- **Single embedded gzipped JSON blob.** Doesn't compose with mod-overlay (`Content/RULES.md` §6). Per-pack RON + lex load order.
- **`HashMap` in canonical state** (`players.rs:199`, `market.rs:11`, `window.rs:8`). `BTreeMap` per `Sim/RULES.md` §2.
- **Real licensed names + clubs.** Hard exclusion per pillar 1.
- **Hard-panic version check, no migrations** (`compiled.rs:59-64`). FW migration is forward-only + numbered.
- **`f32` in canonical types.** `Q32` newtype per `Sim/RULES.md` §1.
- **String enums on the wire** (`OdbContract.contract_type: Option<String>`, `ClubFacilitiesEntity.training: String`). Use typed enum variants.

## Open questions

1. **Durable CA/PA scalars vs derived.** Open-football anchors gen + dev ceiling on durable u8 CA/PA. FW better served by durable CA/PA nudged by breakthroughs, or per-attribute values where "potential" emerges from career events? The former is simpler but off-axis with the memory-ledger story.
2. **Baked role archetypes vs emergent signature.** Open-football's 20+ archetypes bake at gen. FW's signature pillar wants the archetype to *emerge* from played minutes. How early do we commit?
3. **Foreign-player league weighting.** `LeagueEntity.foreign_players: Vec<{country_id, weight}>` cleanly drives squad nationality mix without modeling migration. For procedural cultures, equivalent might be per-league "foreign-culture-mix" weights from the content baker.
4. **Player events / memory.** Out of read scope. `OdbHistoryItem`'s per-season `played`/`goals`/`rating` is bare minimum, nothing like a memory ledger — likely confirms pillar 2 has no field equivalent.
5. **Dual 1-20 + 15-200 scale.** FM convention, recognizable, but FW is text-first. Could collapse to one internal Q32 scale projected to commentary only.

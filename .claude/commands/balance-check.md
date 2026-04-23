---
description: Balance / stat / formula sanity check — outliers, broken progressions, dominant strategies
argument-hint: "[system-name | path-to-data-file]"
---

# /balance-check — balance sanity check

Analyze balance data files, formulas, and configuration. Identify outliers, broken progressions, dominant strategies, economy imbalances. Run after modifying any balance-related data.

**Phase:** 2-7 (anytime stat data exists). Output: `design/reviews/balance-<system>-<date>.md`.

## Procedure

1. **Parse arg.**
   - `<system-name>` — e.g., `combat`, `economy`, `progression`, `loot`
   - `<path>` — load that data file; infer domain from content
   - No arg — ask user which system
2. **Identify domain:**
   - **Combat** — weapon/ability DPS, TTK, damage types
   - **Economy** — resource faucets/sinks, rates, pricing
   - **Progression** — XP/power curves, dead zones, spikes
   - **Loot** — rarity, pity timers, inventory pressure
   - **Custom** — project-specific (stats, resources, etc.)
3. **Load** relevant data files from `Assets/_Project/Data/<System>/*.asset` (ScriptableObject) or `design/balance/` (CSV/YAML).
4. **Load** the governing GDD from `design/gdd/<system>.md` — establishes **intended** ranges + tuning knobs.
5. **Spawn Systems Designer subagent** (`systems-designer` if installed; else `economy-designer`; else `general-purpose` with a Systems Designer persona).
6. **Domain-specific analysis:**
   - **Combat:** DPS per tier, TTK per tier, strict-dominance scan, defense-stacking unkillable scan, damage-type resistance balance
   - **Economy:** faucet/sink flow map, resource accumulation projection, infinite-loop check, unused items, gold-sink scaling
   - **Progression:** XP curve + power curve plots, dead-zone detection, spike detection, content-gate alignment
   - **Loot:** rarity distribution, pity-timer effectiveness, inventory-pressure calc
7. **Formula-sanity checks:**
   - Divide-by-zero risk
   - Negative-output cases
   - Monotonic where intended (e.g., higher-level weapon ≥ lower-level)
   - Bounds respected (HP doesn't go negative, resistance ≤ 1.0, etc.)
8. **Report** to `design/reviews/balance-<system>-<date>.md`:
   - Data Sources (files read)
   - Findings (outliers, dominant strategies, broken curves) with severity
   - Proposed tuning fixes (deltas, not rewrites)
9. **Verdict:** BALANCED / ADJUSTMENTS RECOMMENDED / BROKEN.

## If args provided

- `<system-name>` or `<path>` — scope the check

## If no data exists

Fail: "No balance data found for `<system>`. Data should live under `Assets/_Project/Data/<System>/` (SOs) or `design/balance/`."

## If GDD missing

Warn but continue: "No GDD for `<system>` — using defaults as baseline. Recommend `/design-system <system>` for explicit tuning knobs."

## Output

- `design/reviews/balance-<system>-<date>.md`
- Console: verdict + top 3 findings

## Related

- Typical follow-ups: `/quick-design` to record tuning change → `/dev-story` to apply
- Invokes agents: `systems-designer` or `economy-designer`
- Invokes skills: none
- Reads files: `Assets/_Project/Data/<System>/*.asset`, `design/balance/**`, `design/gdd/<system>.md`
- Writes files: `design/reviews/balance-<system>-<date>.md`

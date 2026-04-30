using System;
using System.Collections.Generic;
using System.Collections.ObjectModel;
using FinalWhistle.MatchSim.Memory.Contracts;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// Phase-3 minimum sim → viewer translation per ADR-0008
    /// §"Viewer.EventBridge owns deterministic conversion." Consumes the
    /// canonical MatchSim KeyEvent stream + Phase-3
    /// <see cref="ShotTypeCatalog"/> + the per-event source seed and emits
    /// a deterministic <see cref="ViewerEvent"/> stream sorted by
    /// <c>(StartTick ascending, ViewerEventId ascending)</c> per ADR-0008
    /// §"Determinism contract" ordering rules.
    ///
    /// <para>
    /// <strong>Lives in <c>Viewer.Contracts</c></strong> (not
    /// <c>Viewer.Core</c>) per SPEC 2026-04-30 Codex round-4 entry +
    /// <c>.claude/rules/Scripts/Viewer/RULES.md</c>: the asmdef-level
    /// <c>noEngineReferences: true</c> flag makes a stray
    /// <c>using UnityEngine</c> in bridge code a compile error rather
    /// than a reviewer-discipline issue, architecturally enforcing
    /// ADR-0008's "no Unity APIs in deterministic conversion" contract.
    /// </para>
    ///
    /// <para>
    /// <strong>Phase-3 minimum scope</strong>:
    /// </para>
    /// <list type="bullet">
    ///   <item><description><see cref="KeyEventKind.Goal"/> →
    ///       <see cref="ShotTypeCatalog.ShotPassShotImpact"/> (no scorer
    ///       attribution at Phase 3; <see cref="ViewerEvent.FocalSubject"/>
    ///       null + <see cref="ViewerEvent.ParticipantPlayerIds"/> empty).</description></item>
    ///   <item><description><see cref="KeyEventKind.SignatureExecuted_LowCutback"/>
    ///       → <see cref="ShotTypeCatalog.ShotPlayerIsolation"/>
    ///       (carrier focal).</description></item>
    ///   <item><description><see cref="KeyEventKind.SignatureExecuted_BlindSideNearPostRun"/>
    ///       → <see cref="ShotTypeCatalog.ShotPassShotImpact"/>
    ///       (carrier focal — blind-side striker run resolves at the
    ///       cross-receive moment which IS a pass-shot impact).</description></item>
    ///   <item><description><see cref="KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch"/>
    ///       → <see cref="ShotTypeCatalog.ShotTacticalWide"/>
    ///       (the diagonal switch is a wide-framing tactical shot).</description></item>
    ///   <item><description><see cref="KeyEventKind.SignatureBreakthrough"/>
    ///       → <see cref="ShotTypeCatalog.ShotAftermathFreeze"/>
    ///       (cap-reach breakthrough = post-action freeze beat per
    ///       <c>design/breakthrough-moments.md</c> §"Trigger kinds" Kind 1).</description></item>
    ///   <item><description>Restart events
    ///       (<see cref="KeyEventKind.GoalKickRestart"/> /
    ///       <see cref="KeyEventKind.ThrowInRestart"/> /
    ///       <see cref="KeyEventKind.CornerKickRestart"/>) do NOT emit
    ///       <see cref="ViewerEvent"/>s — match telemetry only at Phase 3.
    ///       Phase-4+ may add brief tactical-wide ViewerEvents on
    ///       restarts.</description></item>
    /// </list>
    ///
    /// <para>
    /// <strong>MemoryHit derivation</strong> is Phase-4+ — Phase-3
    /// emission produces <see cref="ViewerEvent.MemoryHits"/> as
    /// empty arrays. The reader-callback infrastructure shipped in slice
    /// #3 (<c>PressFanReader</c>) and slice #4 (<c>BreakthroughReader</c>)
    /// queries memory readers per the bridge's resolution semantics
    /// without yet emitting <c>MemoryHit</c>s on the viewer events.
    /// Phase-4+ wires the bridge ↔ memory-reader bridge once the
    /// <c>Viewer.EventBridge</c> consumer surface in the dots adapter (SPEC
    /// line 149) shows what fields drive what visual.
    /// </para>
    /// </summary>
    public static class EventBridge
    {
        /// <summary>
        /// Derive the <see cref="ViewerEvent"/> stream from a completed
        /// match's canonical state. Pure function: same input produces
        /// the same byte-for-byte output stream.
        /// </summary>
        /// <param name="state">Match state with the canonical KeyEvent
        /// stream populated. Read-only; never mutated.</param>
        /// <param name="matchSeed">Match seed for per-event seed
        /// derivation per ADR-0001 / TECH_APPROACH §3.2.</param>
        /// <param name="reduceMotionEnabled">When true,
        /// <see cref="ShotTypeDefinition.ReduceMotionVariantId"/> is
        /// substituted exactly once at the bridge per ADR-0008
        /// §"Reduce-motion adapter-awareness" — adapters do NOT
        /// re-substitute.</param>
        public static IReadOnlyList<ViewerEvent> Derive(
            MatchSimulationState state,
            Seed matchSeed,
            bool reduceMotionEnabled = false)
        {
            if (state is null) throw new ArgumentNullException(nameof(state));

            // StartTick ordering enforcement per Codex round-1 P2 against
            // 40159bd: ADR-0008 §"Determinism contract" pins stream order
            // at (StartTick ascending, ViewerEventId ascending). Bridge
            // requires KeyEvents to already be StartTick-non-decreasing
            // (the MatchSim runtime invariant); a hand-built or future
            // orchestration state with [tick=300, tick=100] would
            // otherwise silently emit ViewerEvents out of order. Throw
            // loud at the bridge boundary rather than emit a malformed
            // stream.
            ValidateKeyEventsStartTickNonDecreasing(state.KeyEvents);

            // Build an index from KeyEventIndex → SignatureExecution so
            // each signature-execution KeyEvent is correlated with its
            // recipe per Codex round-1 P1 against 40159bd. The recipe
            // is the authored presentation metadata — RecipeKey drives
            // the shot selection (no longer hard-coded by KeyEventKind),
            // and SignatureId / SimBiasFieldId / SimBiasDeltaRawQ32
            // travel onto the ViewerEvent for the dots adapter.
            Dictionary<int, SignaturePresentationRecipe> recipesByIndex =
                BuildRecipeIndex(state);

            List<ViewerEvent> result = new();
            ulong viewerEventId = 0;
            for (int i = 0; i < state.KeyEvents.Count; i++)
            {
                KeyEvent ke = state.KeyEvents[i];
                if (!TryResolveBaseShot(ke, i, recipesByIndex, out string baseShotId))
                {
                    // Phase-3: restart events skip translation. The skip is
                    // the documented routine-band-telemetry behavior, not
                    // an error — restart KeyEvents stay in the canonical
                    // stream for replay verification but don't drive
                    // viewer presentation.
                    continue;
                }

                ShotTypeDefinition baseShot = ShotTypeCatalog.Get(baseShotId);
                (string effectiveShotId, bool reduceMotionApplied) =
                    ResolveEffectiveShot(baseShot, reduceMotionEnabled);
                // Derive the duration envelope from the EFFECTIVE shot per
                // pr-review-toolkit:type-design-analyzer 2026-04-30 finding
                // #2: a future reduce-motion variant with a different
                // duration would silently desync the envelope from the
                // resolved shot if we read baseShot.DurationTicks instead.
                ShotTypeDefinition effectiveShot = ShotTypeCatalog.Get(effectiveShotId);

                Tick startTick = ke.Tick;
                Tick endTick = new(startTick.Value + effectiveShot.DurationTicks);

                Seed eventSeed = Seed.Derive(
                    matchSeed: matchSeed.Value,
                    tick: ke.Tick,
                    eventId: viewerEventId);

                EventClass sourceClass = MapToSourceEventClass(ke.Kind);
                bool hasFocalSubject = ke.JerseyNumber != KeyEvent.JerseyUnspecified
                    && sourceClass != EventClass.GoalScored;
                string? focalSubject = hasFocalSubject
                    ? FormatFocalSubject(ke.Side, ke.JerseyNumber)
                    : null;
                string[] participantPlayerIds = focalSubject is null
                    ? Array.Empty<string>()
                    : new[] { focalSubject };

                ViewerEvent ev = new(
                    viewerEventId: viewerEventId,
                    sourceEventId: (ulong)i,
                    sourceEventOrdinal: 0,
                    baseShotTypeId: baseShot.Id,
                    effectiveShotTypeId: effectiveShotId,
                    reduceMotionApplied: reduceMotionApplied,
                    startTick: startTick,
                    endTick: endTick,
                    seed: eventSeed,
                    stakesNormalized: StakesFor(sourceClass),
                    memoryRelevance: Fixed.Zero,
                    focalSubject: focalSubject,
                    participantPlayerIds: participantPlayerIds,
                    memoryHits: Array.Empty<MemoryHit>(),
                    sourceEventClass: sourceClass,
                    sourceEntityId: focalSubject);

                result.Add(ev);
                viewerEventId++;
            }
            // Wrap in ReadOnlyCollection per Codex round-1 P2 against
            // 40159bd: the bare List<ViewerEvent> instance is castable
            // back to a mutable List<ViewerEvent>, letting consumers
            // reorder or append after the bridge has emitted what it
            // claims is a deterministic stream. Same defensive-wrap
            // pattern as MemoryEvent.Participants + CallbackTag.ConsumingReaders.
            return new ReadOnlyCollection<ViewerEvent>(result);
        }

        /// <summary>
        /// Build a (KeyEventIndex → recipe) dictionary AND validate the
        /// invariant: every <c>SignatureExecuted_*</c> KeyEvent has
        /// exactly one matching recipe in
        /// <c>state.SignatureRecipes</c>; every recipe maps to a real
        /// signature-execution KeyEvent. Per Codex round-1 P1: the
        /// recipe stream was specifically built for Viewer.EventBridge;
        /// silently dropping it loses the authored presentation metadata
        /// (SignatureId / RecipeKey / SimBiasFieldId / SimBiasDeltaRawQ32)
        /// the dots adapter is meant to consume.
        /// </summary>
        private static Dictionary<int, SignaturePresentationRecipe> BuildRecipeIndex(
            MatchSimulationState state)
        {
            Dictionary<int, SignaturePresentationRecipe> byIndex = new(state.SignatureRecipes.Count);
            for (int i = 0; i < state.SignatureRecipes.Count; i++)
            {
                SignatureExecution exec = state.SignatureRecipes[i];
                if (byIndex.ContainsKey(exec.KeyEventIndex))
                {
                    throw new InvalidOperationException(
                        $"SignatureRecipes contains duplicate entries for KeyEventIndex {exec.KeyEventIndex}. " +
                        "The recipe stream is parallel to the KeyEvents stream — each signature-execution " +
                        "KeyEvent must correspond to exactly one SignatureExecution entry.");
                }
                if ((uint)exec.KeyEventIndex >= state.KeyEvents.Count)
                {
                    throw new InvalidOperationException(
                        $"SignatureRecipes[{i}].KeyEventIndex={exec.KeyEventIndex} is out of range " +
                        $"for KeyEvents.Count={state.KeyEvents.Count}.");
                }
                if (!IsSignatureExecutionKind(state.KeyEvents[exec.KeyEventIndex].Kind))
                {
                    throw new InvalidOperationException(
                        $"SignatureRecipes[{i}].KeyEventIndex={exec.KeyEventIndex} points to a " +
                        $"KeyEvent of kind {state.KeyEvents[exec.KeyEventIndex].Kind}, which is not " +
                        "a signature-execution kind. Recipe-stream entries must mirror SignatureExecuted_* events.");
                }
                byIndex[exec.KeyEventIndex] = exec.Recipe;
            }
            // Symmetric check: every signature-execution KeyEvent has a
            // recipe entry. The pairing is what makes the two streams a
            // joint contract.
            for (int i = 0; i < state.KeyEvents.Count; i++)
            {
                if (IsSignatureExecutionKind(state.KeyEvents[i].Kind) && !byIndex.ContainsKey(i))
                {
                    throw new InvalidOperationException(
                        $"KeyEvents[{i}] is a {state.KeyEvents[i].Kind} signature-execution event " +
                        "but no matching SignatureRecipes entry exists. SignatureRules must emit a " +
                        "recipe alongside every signature-execution KeyEvent.");
                }
            }
            return byIndex;
        }

        /// <summary>
        /// Resolve the base shot ID for a KeyEvent. For
        /// signature-execution events, looks up the recipe and maps
        /// <see cref="SignaturePresentationRecipe.RecipeKey"/> → catalog
        /// shot ID — the recipe is the authored single-source-of-truth
        /// per Codex round-1 P1 fix. Goals + breakthroughs use static
        /// mappings (no recipe stream entry; their shot identity is
        /// pinned by the bridge contract). Restart events return false
        /// (caller skips them).
        /// </summary>
        private static bool TryResolveBaseShot(
            KeyEvent ke, int keyEventIndex,
            Dictionary<int, SignaturePresentationRecipe> recipesByIndex,
            out string shotId)
        {
            switch (ke.Kind)
            {
                case KeyEventKind.Goal:
                    shotId = ShotTypeCatalog.ShotPassShotImpact;
                    return true;
                case KeyEventKind.SignatureBreakthrough:
                    shotId = ShotTypeCatalog.ShotAftermathFreeze;
                    return true;
                case KeyEventKind.SignatureExecuted_LowCutback:
                case KeyEventKind.SignatureExecuted_BlindSideNearPostRun:
                case KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch:
                    SignaturePresentationRecipe recipe = recipesByIndex[keyEventIndex];
                    shotId = ShotIdForRecipeKey(recipe.RecipeKey);
                    return true;
                default:
                    shotId = string.Empty;
                    return false;
            }
        }

        /// <summary>
        /// Map a signature recipe key (the short slug authored on
        /// <see cref="SignaturePresentationRecipe.RecipeKey"/>) to the
        /// content-pack-qualified <see cref="ShotTypeDefinition.Id"/> in
        /// <see cref="ShotTypeCatalog"/>. The two layers carry the same
        /// vocabulary; this helper is the seam between them. Throws on
        /// unknown recipe keys so a future authoring drift surfaces
        /// loudly.
        /// </summary>
        private static string ShotIdForRecipeKey(string recipeKey)
        {
            return recipeKey switch
            {
                "tactical-wide" => ShotTypeCatalog.ShotTacticalWide,
                "player-isolation" => ShotTypeCatalog.ShotPlayerIsolation,
                "pass-shot-impact" => ShotTypeCatalog.ShotPassShotImpact,
                _ => throw new ArgumentOutOfRangeException(
                    nameof(recipeKey), recipeKey,
                    "EventBridge has no Phase-3 ShotTypeCatalog mapping for this recipe key. " +
                    "Add an entry here when a new signature recipe key enters the bridge."),
            };
        }

        private static bool IsSignatureExecutionKind(KeyEventKind kind) =>
            kind == KeyEventKind.SignatureExecuted_LowCutback
            || kind == KeyEventKind.SignatureExecuted_BlindSideNearPostRun
            || kind == KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch;

        private static void ValidateKeyEventsStartTickNonDecreasing(IReadOnlyList<KeyEvent> keyEvents)
        {
            // Allow equal ticks (multiple events on same tick → ordering
            // by ViewerEventId ascending per ADR-0008). Reject strictly
            // decreasing.
            for (int i = 1; i < keyEvents.Count; i++)
            {
                if (keyEvents[i].Tick.Value < keyEvents[i - 1].Tick.Value)
                {
                    throw new ArgumentException(
                        $"KeyEvents are not StartTick-non-decreasing: " +
                        $"KeyEvents[{i - 1}].Tick={keyEvents[i - 1].Tick.Value}, " +
                        $"KeyEvents[{i}].Tick={keyEvents[i].Tick.Value}. " +
                        "EventBridge requires the canonical KeyEvents stream to be in " +
                        "chronological order per ADR-0008 §Determinism contract " +
                        "(stream order is (StartTick, ViewerEventId)).",
                        nameof(keyEvents));
                }
            }
        }

        private static (string EffectiveId, bool Applied) ResolveEffectiveShot(
            ShotTypeDefinition baseShot, bool reduceMotionEnabled)
        {
            if (!reduceMotionEnabled || baseShot.ReduceMotionVariantId is null)
            {
                return (baseShot.Id, false);
            }
            return (baseShot.ReduceMotionVariantId, true);
        }

        private static EventClass MapToSourceEventClass(KeyEventKind kind)
        {
            // ADR-0004 cross-doc exact-match. Signature-execution KeyEvents
            // map to EventClass.SignatureExecuted (Phase-3 catalog
            // extension per SPEC 2026-04-30 decisions-log entry +
            // Codex round-1 P1 fix against 40159bd) — distinct from
            // EventClass.SignatureBreakthrough (the cap-reach permanent-
            // development event). Goals + breakthroughs map to their own
            // classes.
            return kind switch
            {
                KeyEventKind.Goal => EventClass.GoalScored,
                KeyEventKind.SignatureBreakthrough => EventClass.SignatureBreakthrough,
                KeyEventKind.SignatureExecuted_LowCutback
                    or KeyEventKind.SignatureExecuted_BlindSideNearPostRun
                    or KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch
                    => EventClass.SignatureExecuted,
                _ => throw new ArgumentOutOfRangeException(
                    nameof(kind), kind,
                    $"EventBridge has no Phase-3 EventClass mapping for KeyEventKind.{kind}."),
            };
        }

        private static Fixed StakesFor(EventClass sourceClass)
        {
            // Phase-3 placeholder stakes by event class. Phase-4+ derives
            // stakes from match context (cup final / derby / relegation
            // match). Pinned values keep routine signature fires cleanly
            // distinct from cap-reach breakthroughs per Codex round-1 P1
            // against 40159bd:
            //   GoalScored             — 0.95 (high stakes; goal events feed press-fan)
            //   SignatureExecuted      — 0.70 (moderate; routine signature fire)
            //   SignatureBreakthrough  — 1.00 (max; permanent player-development)
            //
            // Default arm THROWS per pr-review-toolkit:feature-dev:code-reviewer
            // 2026-04-30 finding #2: a future Phase-4 EventClass entry
            // added without a StakesFor case must fail loud rather than
            // silently emit a misleading default.
            return sourceClass switch
            {
                EventClass.GoalScored => Fixed.Parse("0.9500000000"),
                EventClass.SignatureExecuted => Fixed.Parse("0.7000000000"),
                EventClass.SignatureBreakthrough => Fixed.One,
                _ => throw new ArgumentOutOfRangeException(
                    nameof(sourceClass), sourceClass,
                    $"EventBridge has no Phase-3 stakes mapping for EventClass.{sourceClass}. " +
                    "Add a case here when a new EventClass enters the bridge translation."),
            };
        }

        private static string FormatFocalSubject(TeamSide side, byte jerseyNumber)
        {
            // Phase-3 minimum: a focal-subject identifier embeds team-side +
            // jersey rather than the real PlayerId because EventBridge
            // doesn't take IdentityPackets at Phase 3 (the dots adapter
            // resolves to PlayerId via its own roster wiring per ADR-0009).
            // Phase-4+ EventBridge takes the IdentityPacket arrays exactly
            // like MemoryEmissionRules does (per Codex round-1 P1 fix on
            // a2b9479) and emits the canonical fwh.core:player_NNNNN
            // identifier here. The bridge format below is stable +
            // adapter-resolvable + ordinal-comparable.
            return $"viewer.focal:{(side == TeamSide.Home ? "home" : "away")}.{jerseyNumber:D2}";
        }
    }
}

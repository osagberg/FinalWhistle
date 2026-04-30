using System;
using System.Collections.Generic;
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

            List<ViewerEvent> result = new();
            ulong viewerEventId = 0;
            for (int i = 0; i < state.KeyEvents.Count; i++)
            {
                KeyEvent ke = state.KeyEvents[i];
                if (!TryMapToShot(ke.Kind, out string baseShotId))
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
            return result;
        }

        private static bool TryMapToShot(KeyEventKind kind, out string shotId)
        {
            switch (kind)
            {
                case KeyEventKind.Goal:
                    shotId = ShotTypeCatalog.ShotPassShotImpact;
                    return true;
                case KeyEventKind.SignatureExecuted_LowCutback:
                    shotId = ShotTypeCatalog.ShotPlayerIsolation;
                    return true;
                case KeyEventKind.SignatureExecuted_BlindSideNearPostRun:
                    shotId = ShotTypeCatalog.ShotPassShotImpact;
                    return true;
                case KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch:
                    shotId = ShotTypeCatalog.ShotTacticalWide;
                    return true;
                case KeyEventKind.SignatureBreakthrough:
                    shotId = ShotTypeCatalog.ShotAftermathFreeze;
                    return true;
                default:
                    shotId = string.Empty;
                    return false;
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
            // ADR-0004 cross-doc exact-match: signature-execution KeyEvents
            // map to SignatureBreakthrough at Phase 3 as the closest
            // available EventClass (the SignatureExecuted EventClass entry
            // is Phase-4+ reserved per ADR-0004 + the SPEC 2026-04-30
            // EventClass.SignatureBreakthrough catalog-extension entry).
            // Goals + breakthroughs map to their own classes.
            return kind switch
            {
                KeyEventKind.Goal => EventClass.GoalScored,
                KeyEventKind.SignatureBreakthrough => EventClass.SignatureBreakthrough,
                KeyEventKind.SignatureExecuted_LowCutback
                    or KeyEventKind.SignatureExecuted_BlindSideNearPostRun
                    or KeyEventKind.SignatureExecuted_FirstTimeDiagonalSwitch
                    => EventClass.SignatureBreakthrough,
                _ => throw new ArgumentOutOfRangeException(
                    nameof(kind), kind,
                    $"EventBridge has no Phase-3 EventClass mapping for KeyEventKind.{kind}."),
            };
        }

        private static Fixed StakesFor(EventClass sourceClass)
        {
            // Phase-3 placeholder stakes by event class. Phase-4+ derives
            // stakes from match context (cup final / derby / relegation
            // match). Pinned values match the Memory layer's analogous
            // placeholders for cross-layer consistency.
            //
            // Default arm THROWS per pr-review-toolkit:feature-dev:code-reviewer
            // 2026-04-30 finding #2: a future Phase-4 EventClass entry
            // added without a StakesFor case must fail loud rather than
            // silently emit 0.5 (a misleading default). Mirrors the
            // exhaustive-throw posture of MapToSourceEventClass.
            return sourceClass switch
            {
                EventClass.GoalScored => Fixed.Parse("0.9500000000"),
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

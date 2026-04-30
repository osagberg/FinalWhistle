using System;
using System.Collections.Generic;
using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Contracts;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Scene-singleton MonoBehaviour that owns the dots-adapter lifecycle
    /// per the Phase-3 dots-adapter blueprint §B Slice 2/3. Slice-3 wires
    /// live sim playback: <see cref="FixedUpdate"/> calls
    /// <see cref="MatchSimulationRunner.RunTicks"/> with <c>ticks=1</c> at
    /// 60Hz, then <see cref="EventBridge.Derive"/> projects newly-emitted
    /// canonical events into <see cref="ViewerEvent"/>s, then a high-water
    /// id tracker dispatches new events through the
    /// <see cref="IShotPresentationAdapter"/>. Pinned 60-tick smoke-fixture
    /// hash <c>sha256:7e851976...50e</c> stays unchanged because the
    /// chunked-vs-single-call invariant is already guaranteed by
    /// <c>Match_ChunkedRunTicksWithSignatures_ProducesIdenticalHashAndEventStream</c>
    /// in <c>MatchSim.Tests</c> (Codex round-9 closure: cooldown lives on
    /// <c>MatchSimulationState</c>; runner reads-from-state instead of
    /// re-allocating per call).
    ///
    /// <para>
    /// <strong>Tick rate (Slice 3):</strong>
    /// <see cref="Time.fixedDeltaTime"/> is set to <c>1/60</c> in
    /// <see cref="OnEnable"/> + restored to its previous value in
    /// <see cref="OnDisable"/>. The matching-rate guarantees one
    /// <c>FixedUpdate</c> tick per canonical sim tick. Saving + restoring
    /// the prior value avoids leaking 60Hz into the editor across scene
    /// boundaries (gameplay-programmer Slice-3 consult Q2). Cache fields
    /// are <c>[NonSerialized]</c> so a Unity domain-reload during play
    /// mode doesn't poison the cached prior value (pr-review-toolkit
    /// silent-failure-hunter Slice-3 P2).
    /// </para>
    ///
    /// <para>
    /// <strong>Loud-fail discipline:</strong> missing inspector references
    /// throw rather than <c>Debug.LogError + return</c>. The earlier
    /// early-return path produced a half-built scene where Slice 3+ tick
    /// logic would NRE against null adapter references on every frame.
    /// </para>
    /// </summary>
    [DefaultExecutionOrder(-100)]
    public sealed class DotsMatchDirector : MonoBehaviour
    {
        [SerializeField] private DotPool dotPool;
        [SerializeField] private PitchQuad pitchQuad;
        [SerializeField] private DotsAdapterRoot adapterRoot;
        [SerializeField] private ShotCamera shotCamera;

        [Tooltip("Archetype slug for the home side; matches a YAML file stem in MatchSim/Content/archetypes/.")]
        [SerializeField] private string homeArchetypeName = "direct-pressing";

        [Tooltip("Archetype slug for the away side; matches a YAML file stem in MatchSim/Content/archetypes/.")]
        [SerializeField] private string awayArchetypeName = "low-block-counter";

        [Tooltip("Match seed for deterministic playback. 0xdeadbeefdeadbeef is the Phase-3 smoke-fixture seed per design/specs/golden-replay-corpus.md.")]
        [SerializeField] private string matchSeedHex = "0xdeadbeefdeadbeef";

        [Tooltip("If true, RunTicks runs every FixedUpdate. Toggle off for static-formation Slice-2-style debugging.")]
        [SerializeField] private bool driveSim = true;

        [Tooltip("Ball Z-velocity (m/s) magnitude above which the diagonal-attack-lane heuristic transitions framing. Default 8 m/s ≈ sustained-pace cross-pitch ball travel.")]
        [SerializeField, Min(0f)] private float diagonalAttackLaneZVelocityThreshold = 8f;

        // Canonical sim tick rate per `Tick.TicksPerSecond`. Hard-coded
        // (NOT a [SerializeField]) per Codex round-1 P2 follow-up against
        // 2a79529: an inspector-mutable knob is a load-bearing-determinism
        // footgun — accidentally editing this to 0.02 / 0 / NaN would
        // silently desync the FixedUpdate loop from the canonical 60Hz tick
        // rate without any loud boundary failure. Deriving from the
        // MatchSim const ensures it tracks the canonical rate if/when
        // `Tick.TicksPerSecond` ever changes.
        private const float CanonicalFixedDeltaTime = 1f / Tick.TicksPerSecond;

        private PitchView pitchView;
        private MatchSimulationState state;
        private BehaviorTreeArchetype homeArchetype;
        private BehaviorTreeArchetype awayArchetype;
        private IdentityPacket[] homePackets;
        private IdentityPacket[] awayPackets;
        private MatchSimulationConfig config;
        private Seed matchSeed;

        // Pre-allocated scratch buffers for the buffer-reusing RunTicks
        // overload per Codex round-1 P2 follow-up against 2a79529: the
        // FixedUpdate hot path no longer fresh-allocates 22 PlayerCommand
        // entries per tick; these buffers live on the director for the
        // match's lifetime + are reused across every RunTicks call. The
        // per-match SignatureCooldownState (round-9 closure) lives on
        // MatchSimulationState; these viewer-side scratch buffers live
        // on the director.
        private PlayerCommand[] homeCommandBuffer;
        private PlayerCommand[] awayCommandBuffer;

        // High-water mark for ViewerEvent dispatch. ulong? collapses the
        // prior two-field shape (lastProcessedViewerEventId + bool
        // firstViewerEventConsumed) into one self-documenting state per
        // pr-review-toolkit type-design-analyzer Slice-3 P1: null = "no
        // event consumed yet"; otherwise the highest dispatched id.
        private ulong? lastProcessedViewerEventId;
        private bool warnedAdapterRootMissing;

        // Cached canonical-state event-stream lengths per Codex round-1 P2
        // follow-up against 2a79529: skip EventBridge.Derive (which
        // allocates List<ViewerEvent> + Dictionary + ReadOnlyCollection
        // per call) on quiet ticks where neither the KeyEvents stream nor
        // the SignatureRecipes stream advanced. Phase-3 KeyEvent counts
        // are <10 per match, so most ticks are "quiet" — eliminating the
        // wasted allocation eliminates ~120 empty-container allocations
        // per second on the live viewer hot path.
        private int lastKeyEventCount;
        private int lastSignatureRecipeCount;

        // [NonSerialized] per pr-review-toolkit silent-failure-hunter
        // Slice-3 P2: a Unity domain-reload during play mode would
        // otherwise serialize the cached prior value (which is the
        // already-overridden 1/60), then OnEnable on re-load would treat
        // 1/60 as the "true editor default" and OnDisable would restore
        // the wrong value. Marking [NonSerialized] forces these to reset
        // across reload so OnEnable always captures the actual editor
        // default cleanly.
        [NonSerialized] private float priorFixedDeltaTime = -1f;
        [NonSerialized] private bool fixedDeltaTimeOverrideApplied;

        private void Awake()
        {
            if (dotPool == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(DotPool)} reference missing on {nameof(DotsMatchDirector)}; " +
                    "assign in the scene inspector.");
            }
            if (pitchQuad == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(PitchQuad)} reference missing on {nameof(DotsMatchDirector)}; " +
                    "assign in the scene inspector.");
            }
            // shotCamera is REQUIRED at Slice-4 — PresentShot needs it to
            // dispatch shot framings + the diagonal-attack-lane heuristic
            // needs it to apply transitions. The Slice-3 null-prop chain
            // was incoherent with the loud-fail discipline this director
            // declares (per pr-review-toolkit silent-failure-hunter Slice-4
            // P1-A: a missing reference produced a half-built scene whose
            // wiring bug only surfaced at the first ViewerEvent — sometimes
            // never, on smoke-fixture runs).
            if (shotCamera == null)
            {
                throw new InvalidOperationException(
                    $"{nameof(ShotCamera)} reference missing on {nameof(DotsMatchDirector)}; " +
                    "assign in the scene inspector.");
            }
            // adapterRoot is intentionally optional at Phase-3 — Slice-3 still
            // works without it (PresentShot is no-op anyway). FixedUpdate
            // emits a one-shot warning if the bridge ever produces events
            // while adapterRoot is null (per pr-review-toolkit
            // silent-failure-hunter Slice-3 P1: prevents Slice-4 from
            // shipping with a missing wiring + silent shot-camera failure).

            pitchView = new PitchView();
            pitchQuad.Initialize(pitchView);
            dotPool.Initialize(pitchView);
            adapterRoot?.Initialize(pitchView);
            shotCamera.Initialize(pitchView);

            homeArchetype = BehaviorTreeArchetypes.Load(homeArchetypeName);
            awayArchetype = BehaviorTreeArchetypes.Load(awayArchetypeName);
            homePackets = LoadPackets(homeArchetypeName);
            awayPackets = LoadPackets(awayArchetypeName);
            matchSeed = ParseMatchSeed(matchSeedHex);
            config = new MatchSimulationConfig(matchSeed);
            state = MatchSimulationState.FromArchetypeFormations(
                Tick.Zero, BallState.AtRest, homeArchetype, awayArchetype);
            // Pre-allocate scratch buffers once at match start for the
            // buffer-reusing RunTicks overload (per the field doc-comment
            // above; eliminates 22 PlayerCommand[] entries × 60Hz of
            // fresh allocations on the FixedUpdate hot path).
            homeCommandBuffer = new PlayerCommand[MatchCanonicalState.PlayersPerTeam];
            awayCommandBuffer = new PlayerCommand[MatchCanonicalState.PlayersPerTeam];
            lastProcessedViewerEventId = null;
            warnedAdapterRootMissing = false;
            lastKeyEventCount = state.KeyEvents.Count;
            lastSignatureRecipeCount = state.SignatureRecipes.Count;
        }

        private void Start()
        {
            // Seed the interpolation snapshot from the formation positions
            // so the first FixedUpdate's PushTickSnapshot has a sensible
            // "previous" frame to lerp from. Without this, dots would jump
            // from world-origin to tick-1 positions on the first frame.
            // Caches the archetypes on DotPool so PushTickSnapshot doesn't
            // need them per-call.
            dotPool.SetFormationPositions(homeArchetype, awayArchetype, Vector3Fixed.Zero);
        }

        private void OnEnable()
        {
            if (!fixedDeltaTimeOverrideApplied)
            {
                priorFixedDeltaTime = Time.fixedDeltaTime;
                Time.fixedDeltaTime = CanonicalFixedDeltaTime;
                fixedDeltaTimeOverrideApplied = true;
            }
        }

        private void OnDisable()
        {
            if (fixedDeltaTimeOverrideApplied)
            {
                Time.fixedDeltaTime = priorFixedDeltaTime;
                fixedDeltaTimeOverrideApplied = false;
            }
        }

        private void FixedUpdate()
        {
            if (!driveSim || state is null)
            {
                return;
            }

            // Cache the pre-advance tick BEFORE RunTicks mutates state.CurrentTick.
            // The fired event's StartTick == this pre-advance tick; ElapsedTicks
            // for a brand-new event must be 0 (no ticks have elapsed since
            // the event fired) per pr-review-toolkit feature-dev:code-reviewer
            // Slice-3 P1. Without this cache, elapsed = state.CurrentTick (now
            // N+1) - StartTick (N) = 1, off-by-one.
            long preAdvanceTick = state.CurrentTick.Value;

            // Advance the canonical sim by exactly one tick via the
            // buffer-reusing RunTicks overload (Codex round-1 P2 follow-up
            // against 2a79529). The chunked-vs-single-call invariant is
            // pinned by Match_ChunkedRunTicksWithSignatures_ProducesIdenticalHashAndEventStream
            // in MatchSim.Tests — RunTicks(ticks=1) × N produces byte-identical
            // canonical state to RunTicks(ticks=N) for the smoke fixture and
            // for the LowCutback trigger fixture (round-10 follow-up). The
            // per-match SignatureCooldownState lives on MatchSimulationState
            // (round-9 closure), so it persists across these chunked calls.
            // The buffer-reusing path is pinned byte-identical to the
            // fresh-allocation path by Match_RunTicks_BufferReusing_ProducesIdenticalCanonicalState
            // (Slice-3 follow-up).
            MatchSimulationRunner.RunTicks(
                state, homeArchetype, awayArchetype,
                homePackets, awayPackets,
                PlayerKinematics.Phase3Defaults,
                BallPhysicsCoefficients.Phase3Seeds,
                config,
                SignatureConfig.Phase3Defaults,
                homeCommandBuffer, awayCommandBuffer,
                ticks: 1);

            // Push the new canonical-state snapshot to the dot pool BEFORE
            // event derivation so the visual interpolation target is
            // up-to-date even if the bridge throws on a malformed event.
            dotPool.PushTickSnapshot(state.HomeTeam, state.AwayTeam, state.Ball);

            // Notify ShotCamera so it can auto-return to the default
            // framing when the active shot's duration envelope expires.
            shotCamera.OnSimTick(state.CurrentTick);

            DispatchNewViewerEvents(preAdvanceTick);

            // Slice-4 adapter-local heuristic per blueprint Decision 3:
            // diagonal-attack-lane is NOT bridge-emitted at Phase-3 — the
            // adapter watches ball Z-velocity and transitions framing
            // when wide ball motion suggests an attacking lane is opening.
            // Phase-4+ may flip this to bridge-emitted (KeyEventKind.ThroughBallLaunched);
            // until then it's a presentation-only heuristic that the
            // ShotCamera ignores when an event-driven shot is active
            // (BeginAdapterShot bails on hasActiveShot=true).
            CheckDiagonalAttackLaneHeuristic();
        }

        private void CheckDiagonalAttackLaneHeuristic()
        {
            if (adapterRoot == null)
            {
                return;
            }
            // diagonalAttackLaneZVelocityThreshold is now a SerializeField
            // (per pr-review-toolkit silent-failure-hunter Slice-4 P2-C):
            // Phase-3 observer feedback can tune the cutoff via inspector
            // without a code edit + recompile cycle. Default 8 m/s
            // ≈ sustained-pace cross-pitch ball travel. The conversion
            // from float → Fixed happens once per FixedUpdate; cheap.
            Fixed threshold = Fixed.FromRaw(
                (long)(diagonalAttackLaneZVelocityThreshold * Fixed.OneRaw));
            // Fixed.Abs handles the negation correctly (no Fixed.Zero - x
            // pattern that would silently overflow at Fixed.MinValue —
            // unreachable at Phase-3 ball speeds but worth using the
            // canonical helper for clarity per pr-review-toolkit
            // feature-dev:code-reviewer Slice-4 P2.4).
            if (Fixed.Abs(state.Ball.Velocity.Z) > threshold)
            {
                ShotTypeSO diagonalLane = adapterRoot.ResolveShot(ShotCategory.DiagonalAttackLane);
                shotCamera.BeginAdapterShot(diagonalLane);
            }
        }

        private void DispatchNewViewerEvents(long preAdvanceTick)
        {
            // Quiet-tick shortcut per Codex round-1 P2 follow-up against
            // 2a79529: skip the bridge entirely when neither the canonical
            // KeyEvents stream nor the SignatureRecipes stream advanced
            // since the last call. EventBridge.Derive allocates a List +
            // Dictionary + ReadOnlyCollection per call regardless of
            // event count; at Phase-3 KeyEvent counts (<10 per match),
            // most ticks are quiet and these allocations are pure waste.
            // Tracking the prior counts is a 2-int compare per FixedUpdate
            // — orders of magnitude cheaper than the alloc.
            int currentKeyEventCount = state.KeyEvents.Count;
            int currentRecipeCount = state.SignatureRecipes.Count;
            if (currentKeyEventCount == lastKeyEventCount
                && currentRecipeCount == lastSignatureRecipeCount)
            {
                return;
            }
            lastKeyEventCount = currentKeyEventCount;
            lastSignatureRecipeCount = currentRecipeCount;

            // Re-derive the full ViewerEvent list once per tick per blueprint
            // Decision 4: O(n) in KeyEvent count, negligible at Phase-3 counts
            // (<10 per 90-min match). Phase-4+ may add an incremental
            // DeriveFrom(state, seed, fromIndex, ...) overload when KeyEvent
            // counts grow (fouls / cards / subs).
            //
            // ViewerEventId stability across re-derives: bridge iterates
            // state.KeyEvents in stable order with viewerEventId++ per
            // emission, so re-derives at tick N+1 produce identical ids
            // 0..K-1 for previously-emitted events plus new ids K..K+M-1
            // for new events. The high-water-mark approach is therefore
            // sound (gameplay-programmer Slice-3 consult Q4).
            IReadOnlyList<ViewerEvent> events = EventBridge.Derive(
                state, matchSeed, reduceMotionEnabled: false);

            if (events.Count == 0)
            {
                return;
            }

            // adapterRoot null + events non-empty == silent-fail trap per
            // pr-review-toolkit silent-failure-hunter Slice-3 P1. Warn once
            // per session so the dev fixes the wiring; the dispatch silently
            // skips when adapterRoot is null (Slice-3 PresentShot is no-op
            // anyway, but Slice-4+ shot-camera dispatch will need adapterRoot
            // wired to fire).
            if (adapterRoot == null)
            {
                if (!warnedAdapterRootMissing)
                {
                    Debug.LogWarning(
                        $"{nameof(DotsMatchDirector)}: adapterRoot unwired but EventBridge produced " +
                        $"{events.Count} event(s) — Slice-4+ shot cameras will not fire. " +
                        "Assign DotsAdapterRoot in the scene inspector.", this);
                    warnedAdapterRootMissing = true;
                }
                // Still advance the high-water mark so we don't re-warn on
                // the same events next tick.
                lastProcessedViewerEventId = events[events.Count - 1].ViewerEventId;
                return;
            }

            for (int i = 0; i < events.Count; i++)
            {
                ViewerEvent ev = events[i];
                bool isNew = lastProcessedViewerEventId is null
                    || ev.ViewerEventId > lastProcessedViewerEventId.Value;
                if (!isNew)
                {
                    continue;
                }

                // Use preAdvanceTick (the tick the runner just executed),
                // not state.CurrentTick (which is preAdvanceTick + 1 after
                // RunTicks). For a brand-new event with StartTick ==
                // preAdvanceTick, elapsed = 0 — the correct "no ticks
                // elapsed since the event fired" reading per the
                // ActiveViewerEvent.ElapsedTicks contract.
                int elapsed = Math.Max(0, (int)(preAdvanceTick - ev.StartTick.Value));
                ActiveViewerEvent active = new(ev, elapsed);
                adapterRoot.PresentShot(active);

                lastProcessedViewerEventId = ev.ViewerEventId;
            }
        }

        private static IdentityPacket[] LoadPackets(string archetype)
        {
            IdentityPacket[] packets = new IdentityPacket[IdentityPackets.PlayersPerArchetype];
            for (byte jersey = 1; jersey <= IdentityPackets.PlayersPerArchetype; jersey++)
            {
                packets[jersey - 1] = IdentityPackets.Load(archetype, jersey);
            }
            return packets;
        }

        private static Seed ParseMatchSeed(string hex)
        {
            if (string.IsNullOrWhiteSpace(hex))
            {
                throw new InvalidOperationException(
                    $"matchSeedHex is empty; expected a value like 0xdeadbeefdeadbeef.");
            }
            return Seed.Parse(hex);
        }

#if UNITY_EDITOR
        // OnValidate per pr-review-toolkit feature-dev:code-reviewer
        // Slice-3 P3: surface a malformed matchSeedHex at inspector-edit
        // time rather than at play-mode entry. The Awake throw is still
        // the load-bearing loud-fail; OnValidate is dev-UX affordance.
        private void OnValidate()
        {
            if (string.IsNullOrWhiteSpace(matchSeedHex))
            {
                return;
            }
            try
            {
                _ = Seed.Parse(matchSeedHex);
            }
            catch (Exception ex)
            {
                Debug.LogError(
                    $"{nameof(DotsMatchDirector)}.{nameof(matchSeedHex)} is malformed: {ex.Message} " +
                    $"(value: \"{matchSeedHex}\"). Expected 0x-prefixed 16-hex-digit form (e.g. 0xdeadbeefdeadbeef).",
                    this);
            }
        }
#endif
    }
}

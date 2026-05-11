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

        [Tooltip("Optional — Slice-6 UGUI overlay (scoreboard / commentary / signature title-card). The Slice-6 round-1 implementation used UI Toolkit but UI Toolkit runtime panels did not composite on Unity 6.0.4 + URP 17.4 + Mac/Metal; UGUI is the fallback per .claude/rules/Scripts/Viewer/RULES.md. When unwired, the dots scene runs without overlay text; a one-shot warning fires if the bridge produces a signature-execution ViewerEvent + the overlay is null. The OverlayController hosts a Canvas in Screen Space - Overlay mode with sortingOrder=1 (renders above all camera output, above the Slice-5 renderer features). UI Toolkit migration tracked for Phase 4+ once the composition issue is diagnosed.")]
        [SerializeField] private OverlayController overlayController;

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

        [Tooltip("If true, EventBridge.Derive applies the reduce-motion shot-id substitution path per ADR-0001. Inspector toggle for L2 multi-state screenshot capture; player-facing UI lands Phase 4+. When true: impact-frame flash is suppressed; screen-tone strength clamps to 0.")]
        [SerializeField] private bool reduceMotionEnabled = false;

        [Tooltip("Stakes threshold above which a pass-shot-impact event triggers the impact-frame flash. Default 0.7 per dots-adapter blueprint Slice 5 acceptance criterion.")]
        [SerializeField, Range(0f, 1f)] private float impactFrameStakesThreshold = 0.7f;

        [Tooltip("Linear-decay window for the impact-frame flash, in canonical 60Hz ticks. Default 12 = 0.2s @ 60Hz. Visible window is [trigger, trigger+decayTicks).")]
        [SerializeField, Min(1)] private int impactFrameDecayTicks = 12;

        [Tooltip("Symmetric fade-in/fade-out length for the screen-tone overlay, in canonical ticks. Default 6 = 0.1s @ 60Hz.")]
        [SerializeField, Min(0)] private int screenToneFadeTicks = 6;

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
        // One-shot warn for unknown ShotTypeIds reaching the trigger
        // path (per pr-review-toolkit silent-failure-hunter Slice-5
        // P1 closure: a typo or content-pack drift in EffectiveShotTypeId
        // dropped the event into ShotCategory.None silently — entire match
        // produced zero anime-presentation triggers with no diagnostic).
        private bool warnedUnknownShotId;

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

        // Slice-5 anime-presentation state per blueprint §B Slice 5.
        // Globals driven from these by FixedUpdate after RunTicks; the
        // ScreenTone + ImpactFrame URP renderer features read the globals
        // each frame they render. Decay/strength math lives in
        // AnimePresentationUniforms — pure-C# + L1-testable.
        // -1 sentinel = "no flash active." impactFlashStartTick is read
        // back when an impact-frame trigger fires + computed against
        // state.CurrentTick on FixedUpdate.
        private long impactFlashStartTick = -1L;

        // Slice-6 title-card lifecycle — same -1L sentinel pattern as
        // impactFlashStartTick / screenToneStartTick. The director owns
        // the active title-card window's EndTick + retires the card via
        // OverlayController.HideTitleCard when preAdvanceTick passes
        // EndTick. Canonical-tick driven (Codex Slice-6 P2-3 + Slice-6
        // round-1 P1 closure): NO Time.time / Time.deltaTime feeds the
        // title-card lifetime; retirement uses `preAdvanceTick` (the
        // tick the dispatch loop just emitted events against), NOT
        // `state.CurrentTick.Value` (which is preAdvanceTick + 1 after
        // RunTicks advanced). Same off-by-one class as the Slice-3
        // ActiveViewerEvent.ElapsedTicks fix + Slice-5 round-1
        // UpdateAnimePresentationUniforms fix. [NonSerialized] mirrors
        // the Slice-3 P2 pattern on priorFixedDeltaTime so domain reload
        // can't leak this past the controller's OnDisable null-out.
        [NonSerialized] private long activeTitleCardEndTick = -1L;
        // One-shot warn for "signature event arrived but overlayController
        // is null" — same pattern as warnedAdapterRootMissing.
        private bool warnedOverlayMissing;
        // One-shot warn for the title-card branch swallowing a content-
        // pack drift error (per silent-failure-hunter P1 closure: a bad
        // SignatureId from a future content pack would otherwise crash
        // the dispatch loop and skip PresentShot for the same event).
        private bool warnedOverlayTriggerError;
        // One-shot warn for unknown TeamSide playerName-prefix per
        // silent-failure-hunter P3 closure: Phase-3's StartsWith("home")
        // placeholder silently routes anything-not-home to Away tinting.
        // A future fixture-rename would mask the drift; warn once on the
        // first miss so it surfaces.
        private bool warnedTitleCardSidePrefix;

        // Slice-7 pressure indicator: most-recent ViewerEvent's
        // StakesNormalized projected to float, persisted across ticks.
        // OverlayController.SetPressureTint reads this each tick from
        // the director's FixedUpdate hook. Per blueprint §B Slice 7:
        // "continuous (per-tick), not event-driven" — the indicator
        // holds the last event's stakes until the next event arrives.
        // Starts at 0 (transparent) on Awake.
        private float currentStakesForPressure;
        // Active screen-tone window [start, end). -1 sentinel = "no
        // tone active." Strength at the trigger-time stakes value, then
        // modulated by the symmetric fade envelope.
        private long screenToneStartTick = -1L;
        private long screenToneEndTick = -1L;
        private float screenToneBaseStrength = 0f;

        // Cached PropertyToIDs — Shader.SetGlobalFloat takes either a
        // string or an int; the int version skips a per-call hash.
        // Cached at Awake to avoid the first-call hash inside the hot path.
        private int flashIntensityId;
        private int screenToneStrengthId;
        private int elapsedTicksId;

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
            warnedOverlayMissing = false;
            warnedOverlayTriggerError = false;
            warnedTitleCardSidePrefix = false;
            activeTitleCardEndTick = -1L;
            currentStakesForPressure = 0f;
            lastKeyEventCount = state.KeyEvents.Count;
            lastSignatureRecipeCount = state.SignatureRecipes.Count;

            // Slice-5 shader-global ids cached once.
            flashIntensityId = Shader.PropertyToID(AnimePresentationUniforms.FlashIntensityName);
            screenToneStrengthId = Shader.PropertyToID(AnimePresentationUniforms.ScreenToneStrengthName);
            elapsedTicksId = Shader.PropertyToID(AnimePresentationUniforms.ElapsedTicksName);
            ResetAnimePresentationState();
        }

        private void ResetAnimePresentationState()
        {
            impactFlashStartTick = -1L;
            screenToneStartTick = -1L;
            screenToneEndTick = -1L;
            screenToneBaseStrength = 0f;
            warnedUnknownShotId = false;
            // Zero the globals at start so a previous play-session's
            // residual values don't bleed into this match's first frame.
            //
            // NOTE — Shader.SetGlobalFloat is process-global. This
            // assumes a single DotsMatchDirector instance per process at
            // Phase-3 (single-camera, single-scene playback). Multi-
            // director scenarios (additive scenes, split-screen, in-editor
            // scene reload mid-play) would need either singleton-enforce
            // here or per-instance MaterialPropertyBlocks on the renderer
            // features. Flagged for revisit if/when Slice-7+ adds those
            // (per pr-review-toolkit silent-failure-hunter Slice-5 P2).
            Shader.SetGlobalFloat(flashIntensityId, 0f);
            Shader.SetGlobalFloat(screenToneStrengthId, 0f);
            Shader.SetGlobalInt(elapsedTicksId, 0);
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
            // Symmetric with Awake's reset (per Codex round-1 closure of
            // 2b3460e: Shader.SetGlobal* are process-global; OnDisable
            // restored Time.fixedDeltaTime but not the anime uniforms,
            // so a director disabled mid-flash left the renderer features
            // applying the stale globals until another director cleared
            // them — observable in scene unload, driveSim toggling, or
            // play→edit transition).
            ClearAnimePresentationGlobals();
        }

        private void OnDestroy()
        {
            // Belt-and-braces alongside OnDisable: a destroyed director
            // can't run OnDisable in some teardown orderings, so clear
            // again here. Idempotent — second clear is a no-op.
            ClearAnimePresentationGlobals();
        }

        private void ClearAnimePresentationGlobals()
        {
            // PropertyToIDs may not have been cached yet if Awake
            // hasn't run (e.g. OnDisable from a prefab teardown). Re-
            // resolve by name in that case so the cleanup is safe to
            // call from any lifecycle phase.
            int flashId = flashIntensityId != 0
                ? flashIntensityId
                : Shader.PropertyToID(AnimePresentationUniforms.FlashIntensityName);
            int toneId = screenToneStrengthId != 0
                ? screenToneStrengthId
                : Shader.PropertyToID(AnimePresentationUniforms.ScreenToneStrengthName);
            int elapsedId = elapsedTicksId != 0
                ? elapsedTicksId
                : Shader.PropertyToID(AnimePresentationUniforms.ElapsedTicksName);
            Shader.SetGlobalFloat(flashId, 0f);
            Shader.SetGlobalFloat(toneId, 0f);
            Shader.SetGlobalInt(elapsedId, 0);
        }

        private void FixedUpdate()
        {
            if (!driveSim || state is null)
            {
                // driveSim toggled OFF mid-play (or sim never bootstrapped):
                // keep the anime globals zeroed so a previously-active flash
                // or screen-tone doesn't visually persist when the sim is
                // paused via the inspector toggle (per Codex round-1 P2).
                ClearAnimePresentationGlobals();
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

            // Slice-7 finding #1 closure: advance MotionLineEmitter fade
            // (alpha Lerp toward 0 over FadeTicks=18). DotsAdapterRoot
            // owns the fade-advance hook; canonical-tick driven against
            // preAdvanceTick to match every other Slice-3+ off-by-one
            // discipline (the runner has advanced state.CurrentTick to
            // preAdvanceTick + 1 by this point).
            adapterRoot?.OnSimTick((int)preAdvanceTick);

            DispatchNewViewerEvents(preAdvanceTick);

            // Slice-6 overlay: scoreboard + minute every tick (cheap
            // label writes; UI Toolkit batches DOM updates). Canonical-
            // tick driven — NO Time.time. Minute formula is the
            // ticks-to-seconds-to-minutes conversion clamped at the
            // controller side.
            if (overlayController != null)
            {
                overlayController.SetScore(state.HomeScore, state.AwayScore);
                int minute = (int)(preAdvanceTick / Tick.TicksPerSecond / 60);
                overlayController.SetMinute(minute);

                // Slice-7 finding #2 closure: continuous per-tick pressure
                // tint write proportional to the most-recent ViewerEvent's
                // StakesNormalized. Per blueprint §B Slice 7: continuous
                // (not event-driven). Persists last event's stakes until
                // a new event arrives (mutated in DispatchNewViewerEvents).
                overlayController.SetPressureTint(currentStakesForPressure);

                // Retire the title-card when the active event's window
                // ends. ev.EndTick is exclusive ([StartTick, EndTick))
                // so we hide the moment the current canonical tick
                // reaches EndTick. Per Slice-6 round-1 P1 closure
                // (feature-dev:code-reviewer): use `preAdvanceTick`
                // (the tick the dispatch loop just emitted events
                // against), NOT `state.CurrentTick.Value` (which is
                // preAdvanceTick + 1 after RunTicks advanced). Same
                // off-by-one class as the Slice-3 ElapsedTicks fix +
                // Slice-5 round-1 UpdateAnimePresentationUniforms fix
                // — staying consistent across all canonical-tick
                // consumers.
                if (activeTitleCardEndTick >= 0L
                    && preAdvanceTick >= activeTitleCardEndTick)
                {
                    overlayController.HideTitleCard();
                    activeTitleCardEndTick = -1L;
                }
            }

            // Slice-5 anime-presentation: drive the shader globals each
            // FixedUpdate against the canonical tick stream (NOT Time.time).
            // Use preAdvanceTick — the same tick the dispatch loop just
            // emitted events against — so a brand-new event at tick N
            // first renders at intensity 1.0 / fade-in tick 0, not at
            // 11/12 / fade-in tick 1 (per Codex round-1 closure of 2b3460e:
            // same off-by-one class as the Slice-3 ActiveViewerEvent
            // ElapsedTicks fix). Decay math lives in
            // AnimePresentationUniforms so it is pure-C# + EditMode-
            // testable — the renderer features read these globals on the
            // next render frame.
            UpdateAnimePresentationUniforms(preAdvanceTick);

            // Slice-4 adapter-local heuristic per blueprint Decision 3:
            // diagonal-attack-lane is NOT bridge-emitted at Phase-3 — the
            // adapter watches ball Z-velocity and transitions framing
            // when wide ball motion suggests an attacking lane is opening.
            // Phase-4+ may flip this to bridge-emitted (KeyEventKind.ThroughBallLaunched);
            // until then it's a presentation-only heuristic. ShotCamera
            // suppresses the heuristic shot while an event-driven shot is
            // active (records the desired shot for resume on event-end).
            CheckDiagonalAttackLaneHeuristic();
        }

        private void CheckDiagonalAttackLaneHeuristic()
        {
            // Silent early-return on missing adapterRoot is intentional:
            // mirrors the runtime contract elsewhere in FixedUpdate
            // (DispatchNewViewerEvents also tolerates a null adapterRoot
            // and warns-once via the bridge wiring path). shotCamera is
            // dereferenced earlier in FixedUpdate (OnSimTick), so a null
            // shotCamera at this point would have NRE'd before reaching
            // here — symmetric loud-fail isn't needed.
            if (adapterRoot == null)
            {
                return;
            }
            // diagonalAttackLaneZVelocityThreshold is a SerializeField
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
            //
            // SetAdapterShot is idempotent (Codex round-1 Slice-4 finding 1
            // closure): a sustained over-threshold condition produces a
            // single transition into diagonal-attack-lane (no per-FixedUpdate
            // restart), and a below-threshold condition transitions back to
            // the default framing. Both branches must be wired — calling
            // SetAdapterShot on only the over-threshold path would leave the
            // camera stuck in diagonal framing once the ball decelerates.
            ShotTypeSO desired = Fixed.Abs(state.Ball.Velocity.Z) > threshold
                ? adapterRoot.ResolveShot(ShotCategory.DiagonalAttackLane)
                : null;
            shotCamera.SetAdapterShot(desired);
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
                state, matchSeed, reduceMotionEnabled);

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

                // Slice-5 round-2 follow-up against d7faefc closes a P1
                // dispatch-ordering bug Codex caught: previously the
                // anime trigger fired AFTER PresentShot, so an event
                // whose category had no registered ShotTypeSO (Phase-3
                // scene caught only TacticalWide / DiagonalAttackLane /
                // PassShotImpact) would throw inside ResolveShot before
                // the screen-tone / impact-frame trigger ran. Anime
                // triggers must be defended from camera-resolution
                // failures: fire them FIRST so a SignatureBreakthrough
                // / LowCutback always opens the anime window even when
                // an authoring miss prevents the framing from rendering
                // (combined with the loud-fail in ResolveShot, the dev
                // sees both the anime effect AND the wiring-bug throw
                // — fully diagnostic).
                MaybeTriggerAnimePresentation(ev);
                MaybeTriggerOverlay(ev);
                adapterRoot.PresentShot(active);

                // Slice-7 finding #2 closure: update the pressure-indicator
                // input on every dispatched ViewerEvent. SaturateStakes
                // clamps the canonical [0, 1] projection to float-space
                // [0, 1]; the indicator holds this value across quiet
                // ticks until a new event mutates it.
                currentStakesForPressure = SaturateStakes(ev.StakesNormalized);

                lastProcessedViewerEventId = ev.ViewerEventId;
            }
        }

        /// <summary>
        /// Slice-6 commentary + signature title-card trigger per the
        /// CommentaryTemplates matrix. Fires for every dispatched
        /// ViewerEvent that has a registered (EventClass, ShotCategory)
        /// pool — silent for restart events / unknown categories
        /// (matrix-completeness test pins the contract).
        ///
        /// <para>
        /// Called BEFORE <c>adapterRoot.PresentShot</c> so a missing
        /// scene wiring on the camera side doesn't block the overlay
        /// trigger from firing — same defense-in-depth as the Slice-5
        /// round-2 dispatch reorder.
        /// </para>
        /// </summary>
        private void MaybeTriggerOverlay(ViewerEvent ev)
        {
            if (overlayController == null)
            {
                if (!warnedOverlayMissing)
                {
                    Debug.LogWarning(
                        $"{nameof(DotsMatchDirector)}: overlayController unwired but " +
                        $"the bridge dispatched ViewerEvent {ev.ViewerEventId} ({ev.SourceEventClass}). " +
                        "Slice-6 commentary + title-card will not render. Assign " +
                        "OverlayController in the scene inspector. This message logs once " +
                        "per Awake lifecycle.", this);
                    warnedOverlayMissing = true;
                }
                return;
            }

            // Whole-block try/catch per Slice-6 round-1 P1 closure
            // (silent-failure-hunter): an unknown SignatureId from a
            // future content-pack drift would otherwise propagate
            // KeyNotFoundException out of MaybeTriggerOverlay AND past
            // the caller's `lastProcessedViewerEventId` increment, so
            // the camera framing for THIS event is silently skipped
            // AND the high-water mark advances past it. Narrow the
            // damage: log loud once, no-op the overlay path for this
            // event, and let the dispatch loop continue to PresentShot.
            try
            {
                if (CommentaryTemplates.TryPickCommentary(ev, out string line)
                    && !string.IsNullOrEmpty(line))
                {
                    overlayController.PushCommentary(line);
                }

                ShotCategory category = ResolveCategory(ev);
                if (CommentaryTemplates.ShouldShowTitleCard(ev.SourceEventClass, category)
                    && ev.SignatureMetadata != null)
                {
                    string displayName = CommentaryTemplates.GetSignatureDisplayName(
                        ev.SignatureMetadata.SignatureId);
                    string playerName = ev.ParticipantPlayerIds.Count > 0
                        ? ev.ParticipantPlayerIds[0]
                        : string.Empty;
                    TeamSide side = ResolveTitleCardSide(playerName);
                    overlayController.ShowTitleCard(
                        displayName, playerName, side, ev.ReduceMotionApplied);
                    activeTitleCardEndTick = ev.EndTick.Value;
                }
            }
            catch (Exception ex) when (ex is KeyNotFoundException || ex is ArgumentException)
            {
                // One-shot — repeated misses in the same match are
                // expected to share the same root cause (content-pack
                // drift). Quiet after the first surface so a long
                // match doesn't flood the log.
                if (!warnedOverlayTriggerError)
                {
                    Debug.LogError(
                        $"{nameof(DotsMatchDirector)}: overlay trigger failed for ViewerEvent " +
                        $"{ev.ViewerEventId} ({ev.SourceEventClass}). The PresentShot dispatch " +
                        $"continues for this event — only the commentary + title-card overlay " +
                        $"is suppressed. Most likely cause: a content-pack SignatureId not " +
                        $"registered in CommentaryTemplates.SignatureDisplayNames. " +
                        $"Underlying: {ex.GetType().Name}: {ex.Message}", this);
                    warnedOverlayTriggerError = true;
                }
            }
        }

        /// <summary>
        /// Phase-3 placeholder side discrimination via the focal-subject
        /// string format. Per <c>Viewer.EventBridge.FormatFocalSubject</c>,
        /// the bridge emits participants as <c>viewer.focal:home.{NN}</c> /
        /// <c>viewer.focal:away.{NN}</c>. Phase-4+ adds an explicit
        /// <c>TeamSide</c> field on <see cref="ViewerEvent"/> that retires
        /// this string-prefix heuristic.
        ///
        /// <para>
        /// Per Slice-6 round-2 P2 closure (Codex review of 67c0905): the
        /// previous shape checked raw <c>"home"</c> / <c>"away"</c>
        /// prefixes, which never matched the actual <c>viewer.focal:home.</c>
        /// format — so EVERY home signature title-card silently fell
        /// through to Away tinting. Now matches the actual prefix; the
        /// warn-once still fires for unrecognised formats so a future
        /// rename surfaces.
        /// </para>
        /// </summary>
        private TeamSide ResolveTitleCardSide(string playerName)
        {
            if (playerName.StartsWith("viewer.focal:home.", StringComparison.Ordinal))
            {
                return TeamSide.Home;
            }
            if (playerName.StartsWith("viewer.focal:away.", StringComparison.Ordinal))
            {
                return TeamSide.Away;
            }
            if (!warnedTitleCardSidePrefix)
            {
                Debug.LogWarning(
                    $"{nameof(DotsMatchDirector)}: title-card playerName '{playerName}' " +
                    "does not match the expected 'viewer.focal:home.NN' / 'viewer.focal:away.NN' " +
                    "format. Phase-3 placeholder side discrimination defaulting to Away tinting. " +
                    "This message logs once per Awake lifecycle. Phase-4+ adds an explicit " +
                    "ViewerEvent.Side field that retires this heuristic.",
                    this);
                warnedTitleCardSidePrefix = true;
            }
            return TeamSide.Away;
        }

        /// <summary>
        /// Per-event hook for the anime-presentation surfaces (Slice 5).
        /// Triggered inside the dispatch loop so a new ViewerEvent can
        /// raise an impact-frame flash + a screen-tone window in the
        /// same tick that brought it through the bridge. Reduce-motion
        /// is honoured via the per-event <see cref="ViewerEvent.ReduceMotionApplied"/>
        /// flag (the bridge sets it; the adapter just reads).
        /// </summary>
        private void MaybeTriggerAnimePresentation(ViewerEvent ev)
        {
            ShotCategory category = ResolveCategory(ev);
            // Stakes-driven impact-frame flash on pass-shot-impact
            // events at high stakes per blueprint Slice 5 acceptance
            // criterion. ReduceMotionApplied suppresses the flash
            // entirely (per ADR-0001 + budget surface #1 reduce-motion
            // semantics — a flash is a strong vestibular cue).
            if (category == ShotCategory.PassShotImpact
                && !ev.ReduceMotionApplied
                && ev.StakesNormalized.RawValue >= ImpactStakesRaw())
            {
                impactFlashStartTick = ev.StartTick.Value;
            }
            // Aftermath-freeze drives the screen-tone overlay for the
            // event's duration. Reduce-motion fully suppresses the
            // surface — early-return so a reduce-motion event does NOT
            // overwrite an in-flight non-reduce-motion tone window
            // (per pr-review-toolkit silent-failure-hunter Slice-5
            // P1 closure: the prior write-then-clamp shape would have
            // killed an active overlay early). Symmetric with the
            // impact-frame branch above.
            if (category == ShotCategory.AftermathFreeze
                && !ev.ReduceMotionApplied)
            {
                screenToneStartTick = ev.StartTick.Value;
                screenToneEndTick = ev.EndTick.Value;
                screenToneBaseStrength = SaturateStakes(ev.StakesNormalized);
            }
        }

        /// <summary>
        /// Resolve a ViewerEvent's <see cref="ShotCategory"/> via the
        /// catalog. Returns <see cref="ShotCategory.None"/> on a miss
        /// + emits a one-shot warning (per pr-review-toolkit
        /// silent-failure-hunter Slice-5 P1 closure: previously the
        /// miss path was silent, so a content-pack typo or stale
        /// ShotTypeCatalog produced zero anime-presentation triggers
        /// with no diagnostic).
        /// </summary>
        private ShotCategory ResolveCategory(ViewerEvent ev)
        {
            if (ShotTypeCatalog.TryGet(ev.EffectiveShotTypeId, out ShotTypeDefinition def))
            {
                return def.Category;
            }
            if (!warnedUnknownShotId)
            {
                Debug.LogWarning(
                    $"{nameof(DotsMatchDirector)}: ViewerEvent {ev.ViewerEventId} carries " +
                    $"unknown EffectiveShotTypeId '{ev.EffectiveShotTypeId}' — anime-presentation " +
                    "trigger path will skip it. This message logs once per Awake lifecycle.",
                    this);
                warnedUnknownShotId = true;
            }
            return ShotCategory.None;
        }

        /// <summary>
        /// Q32.32 raw value for the impact-frame stakes threshold.
        /// Computed once-per-call from the inspector float so the
        /// FixedUpdate hot path stays allocation-free.
        /// </summary>
        private long ImpactStakesRaw()
        {
            float t = impactFrameStakesThreshold;
            if (t <= 0f) return 0L;
            if (t >= 1f) return Fixed.OneRaw;
            return (long)(t * Fixed.OneRaw);
        }

        private static float SaturateStakes(Fixed stakes)
        {
            // Stakes is canonical [0, 1]. Project to float via the same
            // double-precision intermediate ActiveViewerEvent uses so the
            // representation matches across the bridge boundary.
            const double oneRaw = (double)Fixed.OneRaw;
            float f = (float)(stakes.RawValue / oneRaw);
            if (f < 0f) return 0f;
            if (f > 1f) return 1f;
            return f;
        }

        /// <summary>
        /// Per-tick refresh of the anime-presentation shader globals.
        /// Decay/strength math lives in <see cref="AnimePresentationUniforms"/>
        /// so the arithmetic is L1-testable; this method just owns the
        /// bridge between the canonical Tick stream and the global
        /// uniforms.
        /// </summary>
        private void UpdateAnimePresentationUniforms(long currentTick)
        {
            float flashIntensity = AnimePresentationUniforms.ComputeFlashIntensity(
                currentTick, impactFlashStartTick, impactFrameDecayTicks);
            float toneStrength = AnimePresentationUniforms.ComputeScreenToneStrength(
                currentTick, screenToneStartTick, screenToneEndTick,
                screenToneBaseStrength, screenToneFadeTicks);

            Shader.SetGlobalFloat(flashIntensityId, flashIntensity);
            Shader.SetGlobalFloat(screenToneStrengthId, toneStrength);
            // Saturating cast: at >2^31 ticks (~414 days @60Hz) the int
            // pins to MaxValue rather than wrapping. No real Phase-3 match
            // hits this, but explicit saturation beats silent overflow.
            Shader.SetGlobalInt(elapsedTicksId, (int)Math.Min(currentTick, int.MaxValue));

            // Retire spent windows so a future event can re-trigger
            // cleanly without leaking residual state. Per pr-review-toolkit
            // feature-dev:code-reviewer Slice-5 P2 closure: the prior
            // shape included a redundant `currentTick - start >= decayTicks`
            // guard that left a future-dated start window dormant forever.
            // ComputeFlashIntensity already returns 0 when elapsed is past
            // the decay window OR when start is sentinel; retire whenever
            // the helper returns 0 AND the window is currently held.
            if (impactFlashStartTick >= 0L
                && currentTick >= impactFlashStartTick
                && flashIntensity <= 0f)
            {
                impactFlashStartTick = -1L;
            }
            if (screenToneStartTick >= 0L && currentTick >= screenToneEndTick)
            {
                screenToneStartTick = -1L;
                screenToneEndTick = -1L;
                screenToneBaseStrength = 0f;
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

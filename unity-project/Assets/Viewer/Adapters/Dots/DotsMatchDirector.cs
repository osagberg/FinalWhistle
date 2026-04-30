using System;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Core;
using UnityEngine;

namespace FinalWhistle.Viewer.Adapters.Dots
{
    /// <summary>
    /// Scene-singleton MonoBehaviour that owns the dots-adapter lifecycle
    /// per the Phase-3 dots-adapter blueprint §B Slice 2. Phase-3 Slice-2
    /// scope is intentionally narrow: instantiate <see cref="PitchView"/>,
    /// initialise <see cref="PitchQuad"/> + <see cref="DotPool"/>, then
    /// place the 23 dots at archetype formation positions. The live sim
    /// tick loop + <see cref="IShotPresentationAdapter"/> dispatch land in
    /// Slice 3.
    ///
    /// <para>
    /// <strong>Execution order -100</strong> ensures Awake fires before any
    /// downstream MB that might read the cached <see cref="PitchView"/>
    /// (Slice 3 viewer adapter; Slice 4 shot camera). The pool + quad
    /// children only need their transforms in place by the time
    /// <c>Initialize</c> is called.
    /// </para>
    ///
    /// <para>
    /// <strong>Loud-fail discipline (pr-review-toolkit Slice-2 P1):</strong>
    /// missing inspector references throw rather than <c>Debug.LogError +
    /// return</c>. The earlier early-return path produced a half-built
    /// scene where Slice 3+ tick logic would NRE against null adapter
    /// references on every frame; throwing makes the failure surface
    /// once + loud at scene-load time.
    /// </para>
    /// </summary>
    [DefaultExecutionOrder(-100)]
    public sealed class DotsMatchDirector : MonoBehaviour
    {
        [SerializeField] private DotPool dotPool;
        [SerializeField] private PitchQuad pitchQuad;

        [Tooltip("Archetype slug for the home side; matches a YAML file stem in MatchSim/Content/archetypes/.")]
        [SerializeField] private string homeArchetypeName = "direct-pressing";

        [Tooltip("Archetype slug for the away side; matches a YAML file stem in MatchSim/Content/archetypes/.")]
        [SerializeField] private string awayArchetypeName = "low-block-counter";

        private PitchView pitchView;

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

            pitchView = new PitchView();
            pitchQuad.Initialize(pitchView);
            dotPool.Initialize(pitchView);
        }

        private void Start()
        {
            BehaviorTreeArchetype home = BehaviorTreeArchetypes.Load(homeArchetypeName);
            BehaviorTreeArchetype away = BehaviorTreeArchetypes.Load(awayArchetypeName);
            dotPool.SetFormationPositions(home, away, Vector3Fixed.Zero);
        }
    }
}

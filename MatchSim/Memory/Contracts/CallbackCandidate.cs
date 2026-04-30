using System;
using FinalWhistle.MatchSim.Sim;

namespace FinalWhistle.MatchSim.Memory.Contracts;

/// <summary>
/// Reader-side surfacing record per ADR-0004 §"Key Interfaces" /
/// <c>CallbackCandidate</c>. Pairs the source <see cref="MemoryEvent"/>
/// with the reader-side surfacing-salience (callback-age + per-reader
/// modifiers applied per query, NEVER persisted) and the
/// <see cref="Template"/> ID the renderer picks up.
/// </summary>
public readonly struct CallbackCandidate
{
    public MemoryEvent Source { get; }

    /// <summary>
    /// Surfacing salience including reader-side modifiers (callback-age,
    /// player-attention, etc.). Recomputed per query — the underlying
    /// <see cref="MemoryEvent.Salience"/> is the immutable emission-time
    /// scalar; this is the surface-time scalar. Q32.32 in <c>[0, 1]</c>.
    /// </summary>
    public Fixed SurfacingSalience { get; }

    /// <summary>
    /// Content-pack-qualified callback-template ID per ADR-0006 ID format
    /// (<c>fwh.core:callback_template.&lt;slug&gt;</c>). Phase-3 has one
    /// template per <see cref="EventClass"/>; Phase-4+ adds template
    /// families with reader-side selection.
    /// </summary>
    public string Template { get; }

    public CallbackCandidate(MemoryEvent source, Fixed surfacingSalience, string template)
    {
        if (surfacingSalience < Fixed.Zero || surfacingSalience > Fixed.One)
        {
            throw new ArgumentOutOfRangeException(nameof(surfacingSalience), surfacingSalience,
                "SurfacingSalience must lie in [0, 1].");
        }
        if (string.IsNullOrEmpty(template))
        {
            throw new ArgumentException("Template must be non-empty.", nameof(template));
        }
        Source = source;
        SurfacingSalience = surfacingSalience;
        Template = template;
    }
}

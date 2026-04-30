using System;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Append-only entry in <see cref="MatchSimulationState.KeyEvents"/> per SPEC
/// 2026-04-28 PitchRules decisions-log entry. Records WHEN a significant
/// match event happened, WHO it was about, and WHERE it happened in pitch
/// coordinates. The golden-replay-corpus spec's <c>key_event_hashes</c>
/// field hashes the canonical encoding of this stream for replay
/// verification at Tier-A.
///
/// <para>
/// <strong>Phase-3 stub-active fields:</strong>
/// </para>
/// <list type="bullet">
///   <item><description><see cref="Tick"/> — when, in canonical sim ticks since match start.</description></item>
///   <item><description><see cref="Kind"/> — what (Goal / restart kind).</description></item>
///   <item><description><see cref="Side"/> — which team it concerns. For goals the scoring side; for restarts the side restarting play.</description></item>
///   <item><description><see cref="JerseyNumber"/> — player most-relevant to the event. Phase 3 does not track scorer/last-toucher (no possession tracking), so this is set to 0 for restarts and 0 for goals at Phase 3. Phase 4+ populates real player IDs once possession + last-touched land.</description></item>
///   <item><description><see cref="Position"/> — pitch coordinates of the event in canonical Q32.32 fixed-point.</description></item>
/// </list>
///
/// <para>
/// <strong>Canonical byte layout</strong> (35 bytes per event, locked v1):
/// </para>
/// <list type="number">
///   <item><description>Tick — 8 bytes LE.</description></item>
///   <item><description>Kind — 1 byte (<see cref="KeyEventKind"/>).</description></item>
///   <item><description>Side — 1 byte (<see cref="TeamSide"/>).</description></item>
///   <item><description>JerseyNumber — 1 byte.</description></item>
///   <item><description>Position — 24 bytes (3× <see cref="Vector3Fixed"/> components).</description></item>
/// </list>
/// </summary>
public readonly struct KeyEvent : IEquatable<KeyEvent>
{
    /// <summary>Canonical byte width per <see cref="KeyEvent"/> on disk. Locked v1.</summary>
    public const int EncodedByteCount = 35;

    /// <summary>
    /// Sentinel value for <see cref="JerseyNumber"/> meaning "scorer /
    /// last-toucher unknown" — used by Phase-3 emissions where MatchSim
    /// does not yet track possession or last-touched-by. Phase-4+ replaces
    /// this with real player attribution. Grep-able via this name; literal
    /// <c>0</c> at the call site is discouraged.
    /// </summary>
    public const byte JerseyUnspecified = 0;

    /// <summary>Tick at which the event occurred.</summary>
    public readonly Tick Tick;

    /// <summary>Discriminator for what kind of event this is.</summary>
    public readonly KeyEventKind Kind;

    /// <summary>Which side the event concerns (scoring side for goals; restarting side for out-of-play restarts).</summary>
    public readonly TeamSide Side;

    /// <summary>
    /// Most-relevant player jersey number, 0 if not applicable. Phase-3 leaves
    /// this 0 because possession + last-touched tracking is Phase 4+ scope.
    /// </summary>
    public readonly byte JerseyNumber;

    /// <summary>Pitch coordinates of the event (Q32.32 fixed-point).</summary>
    public readonly Vector3Fixed Position;

    public KeyEvent(Tick tick, KeyEventKind kind, TeamSide side, byte jerseyNumber, Vector3Fixed position)
    {
        // Defensive: forbid the sentinel + invalid TeamSide values so a
        // default-constructed KeyEvent can't sneak into the canonical stream.
        if (kind == KeyEventKind.None)
        {
            throw new ArgumentException(
                "KeyEventKind.None is a sentinel and must not be emitted.",
                nameof(kind));
        }
        if (side != TeamSide.Home && side != TeamSide.Away)
        {
            throw new ArgumentException(
                $"Invalid TeamSide value {(byte)side}; must be Home (1) or Away (2).",
                nameof(side));
        }
        // jerseyNumber 0 IS valid here (Phase 3 emits 0 when scorer/last-toucher
        // unknown). Real values are 1-99 like PlayerState; 0 is the
        // "unspecified" sentinel for restarts where Phase-3 doesn't track
        // who took the ball out.
        if (jerseyNumber > 99)
        {
            throw new ArgumentOutOfRangeException(
                nameof(jerseyNumber), jerseyNumber,
                "jerseyNumber must be 0 (unspecified) or 1-99 (valid jersey).");
        }

        Tick = tick;
        Kind = kind;
        Side = side;
        JerseyNumber = jerseyNumber;
        Position = position;
    }

    /// <summary>
    /// Write the canonical encoding of this event to the encoder. Locked
    /// v1 layout per the class summary.
    /// </summary>
    public void WriteCanonical(CanonicalEncoder encoder)
    {
        if (encoder is null)
        {
            throw new ArgumentNullException(nameof(encoder));
        }
        if (Kind == KeyEventKind.None)
        {
            // Defensive: a default-constructed KeyEvent (struct default) has
            // Kind=None. Per the constructor's rejection, this should be
            // unreachable, but encoder is the canonical-hash boundary so
            // we double-check before writing potentially-poison bytes.
            throw new InvalidOperationException(
                "Refusing to encode a KeyEvent with Kind=None (sentinel).");
        }

        encoder.WriteTick(Tick);
        encoder.WriteByte((byte)Kind);
        encoder.WriteByte((byte)Side);
        encoder.WriteByte(JerseyNumber);
        encoder.WriteVector3Fixed(Position);
    }

    public bool Equals(KeyEvent other)
        => Tick.Equals(other.Tick)
            && Kind == other.Kind
            && Side == other.Side
            && JerseyNumber == other.JerseyNumber
            && Position.Equals(other.Position);

    public override bool Equals(object? obj) => obj is KeyEvent other && Equals(other);

    /// <summary>
    /// Hash code for in-memory dictionary / set keying ONLY. <strong>Not
    /// cross-process / cross-platform stable</strong> because
    /// <c>HashCode.Combine</c> uses an xxHash variant whose seed
    /// is randomized per process per .NET runtime spec. If a future
    /// Phase-4+ feature wants to deterministically group KeyEvents
    /// (e.g. compose a per-tick replay-segment hash from key-event
    /// sub-hashes), do NOT use <see cref="GetHashCode"/> — fold the
    /// canonical bytes via <see cref="CanonicalEncoder"/> + SHA256 the
    /// way the rest of the determinism suite does. Codex round-4
    /// 2026-04-30 noted this risk; flagging here so the next reviewer
    /// catches an ambient-RNG-via-hash regression at type-definition
    /// time.
    /// </summary>
    public override int GetHashCode()
        => HashCode.Combine(Tick, (byte)Kind, (byte)Side, JerseyNumber, Position);

    public static bool operator ==(KeyEvent left, KeyEvent right) => left.Equals(right);
    public static bool operator !=(KeyEvent left, KeyEvent right) => !left.Equals(right);
}

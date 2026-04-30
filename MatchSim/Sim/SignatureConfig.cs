namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Spatial + cooldown thresholds for the Phase-3 signature triggers
/// in <see cref="SignatureRules"/>. Hoisted into a struct (rather than
/// scattered as private constants) so tests can inject tight-tolerance
/// configs that never fire OR wide configs that always fire, without
/// touching static state.
///
/// <para>
/// <strong>Production singleton</strong> at
/// <see cref="Phase3Defaults"/>. The threshold values are chosen so:
/// </para>
/// <list type="bullet">
///   <item><description>The smoke fixture (ball at centre, formation
///       positions, 60 ticks) cannot fire any signature — pinned
///       60-tick determinism hash stays green.</description></item>
///   <item><description>Realistic gameplay (a winger sprinting toward
///       the byline carrying the ball; a striker entering the box on a
///       wide cross; a CM receiving a moving ball in the middle third)
///       does fire under the right conditions.</description></item>
///   <item><description>Phase-4+ tuning swap-out is one-line: just
///       construct a different <see cref="SignatureConfig"/>.</description></item>
/// </list>
///
/// <para>
/// All distances in metres (<see cref="Fixed"/>). All ticks at 60 Hz
/// canonical rate.
/// </para>
/// </summary>
public readonly struct SignatureConfig
{
    public Fixed BylineProximityMetres { get; }
    public Fixed WideChannelZThreshold { get; }
    public Fixed MinLateralSpeed { get; }
    public Fixed PenaltyAreaDepthMetres { get; }
    public Fixed PenaltyAreaHalfWidthMetres { get; }
    public Fixed CrossDeliveryWideZThreshold { get; }
    public Fixed MinForwardRunSpeed { get; }
    public Fixed MinNearPostCurveSpeed { get; }
    public Fixed MiddleThirdHalfDepthMetres { get; }
    public Fixed MinBallSpeedForSwitch { get; }

    public int LowCutbackCooldownTicks { get; }
    public int BlindSideRunCooldownTicks { get; }
    public int DiagonalSwitchCooldownTicks { get; }

    public byte LowCutbackMaxFires { get; }
    public byte BlindSideRunMaxFires { get; }
    public byte DiagonalSwitchMaxFires { get; }

    public SignatureConfig(
        Fixed bylineProximityMetres,
        Fixed wideChannelZThreshold,
        Fixed minLateralSpeed,
        Fixed penaltyAreaDepthMetres,
        Fixed penaltyAreaHalfWidthMetres,
        Fixed crossDeliveryWideZThreshold,
        Fixed minForwardRunSpeed,
        Fixed minNearPostCurveSpeed,
        Fixed middleThirdHalfDepthMetres,
        Fixed minBallSpeedForSwitch,
        int lowCutbackCooldownTicks,
        int blindSideRunCooldownTicks,
        int diagonalSwitchCooldownTicks,
        byte lowCutbackMaxFires,
        byte blindSideRunMaxFires,
        byte diagonalSwitchMaxFires)
    {
        // Per pr-review-toolkit:type-design-analyzer 2026-04-30 round-2:
        // every threshold has a documented "must be positive" invariant.
        // Validate at construction so a test-author footgun (negative
        // threshold; zero cooldown → spam; zero max-fires → never fire)
        // surfaces immediately rather than as a confused trigger-detection
        // bug downstream.
        ThrowIfNegative(bylineProximityMetres, nameof(bylineProximityMetres));
        ThrowIfNegative(wideChannelZThreshold, nameof(wideChannelZThreshold));
        ThrowIfNegative(minLateralSpeed, nameof(minLateralSpeed));
        ThrowIfNegative(penaltyAreaDepthMetres, nameof(penaltyAreaDepthMetres));
        ThrowIfNegative(penaltyAreaHalfWidthMetres, nameof(penaltyAreaHalfWidthMetres));
        ThrowIfNegative(crossDeliveryWideZThreshold, nameof(crossDeliveryWideZThreshold));
        ThrowIfNegative(minForwardRunSpeed, nameof(minForwardRunSpeed));
        ThrowIfNegative(minNearPostCurveSpeed, nameof(minNearPostCurveSpeed));
        ThrowIfNegative(middleThirdHalfDepthMetres, nameof(middleThirdHalfDepthMetres));
        ThrowIfNegative(minBallSpeedForSwitch, nameof(minBallSpeedForSwitch));
        ThrowIfNonPositiveInt(lowCutbackCooldownTicks, nameof(lowCutbackCooldownTicks));
        ThrowIfNonPositiveInt(blindSideRunCooldownTicks, nameof(blindSideRunCooldownTicks));
        ThrowIfNonPositiveInt(diagonalSwitchCooldownTicks, nameof(diagonalSwitchCooldownTicks));
        ThrowIfZeroByte(lowCutbackMaxFires, nameof(lowCutbackMaxFires));
        ThrowIfZeroByte(blindSideRunMaxFires, nameof(blindSideRunMaxFires));
        ThrowIfZeroByte(diagonalSwitchMaxFires, nameof(diagonalSwitchMaxFires));

        BylineProximityMetres = bylineProximityMetres;
        WideChannelZThreshold = wideChannelZThreshold;
        MinLateralSpeed = minLateralSpeed;
        PenaltyAreaDepthMetres = penaltyAreaDepthMetres;
        PenaltyAreaHalfWidthMetres = penaltyAreaHalfWidthMetres;
        CrossDeliveryWideZThreshold = crossDeliveryWideZThreshold;
        MinForwardRunSpeed = minForwardRunSpeed;
        MinNearPostCurveSpeed = minNearPostCurveSpeed;
        MiddleThirdHalfDepthMetres = middleThirdHalfDepthMetres;
        MinBallSpeedForSwitch = minBallSpeedForSwitch;
        LowCutbackCooldownTicks = lowCutbackCooldownTicks;
        BlindSideRunCooldownTicks = blindSideRunCooldownTicks;
        DiagonalSwitchCooldownTicks = diagonalSwitchCooldownTicks;
        LowCutbackMaxFires = lowCutbackMaxFires;
        BlindSideRunMaxFires = blindSideRunMaxFires;
        DiagonalSwitchMaxFires = diagonalSwitchMaxFires;
    }

    /// <summary>
    /// Phase-3 production thresholds. Pre-computed once at static-init
    /// so the field access is allocation-free + the Q32.32 constants
    /// are computed ONCE rather than on every property access (per
    /// pr-review-toolkit:type-design-analyzer 2026-04-30 round-2).
    /// </summary>
    public static readonly SignatureConfig Phase3Defaults = new(
        bylineProximityMetres: Fixed.FromInt(3),         // within 3m of goal line
        wideChannelZThreshold: Fixed.FromInt(20),        // |Z| > 20m to be wide
        minLateralSpeed: Fixed.One,                      // 1 m/s lateral velocity
        penaltyAreaDepthMetres: Fixed.FromInt(16),       // ~real penalty-area depth
        penaltyAreaHalfWidthMetres: Fixed.FromInt(20),
        crossDeliveryWideZThreshold: Fixed.FromInt(15),  // ball wide for a cross
        minForwardRunSpeed: Fixed.One,                   // 1 m/s toward goal
        minNearPostCurveSpeed: Fixed.One,                // 1 m/s lateral curve
        middleThirdHalfDepthMetres: Fixed.FromInt(25),   // |X| < 25m for middle third
        minBallSpeedForSwitch: Fixed.One,                // 1 m/s ball on each axis
        lowCutbackCooldownTicks: 180,                    // 3 seconds at 60 Hz
        blindSideRunCooldownTicks: 240,                  // 4 seconds at 60 Hz
        diagonalSwitchCooldownTicks: 300,                // 5 seconds at 60 Hz
        lowCutbackMaxFires: 3,
        blindSideRunMaxFires: 2,
        diagonalSwitchMaxFires: 2);

    private static void ThrowIfNegative(Fixed value, string paramName)
    {
        if (value < Fixed.Zero)
        {
            throw new System.ArgumentOutOfRangeException(paramName, value,
                "SignatureConfig threshold must be non-negative.");
        }
    }

    private static void ThrowIfNonPositiveInt(int value, string paramName)
    {
        if (value <= 0)
        {
            throw new System.ArgumentOutOfRangeException(paramName, value,
                "SignatureConfig cooldown ticks must be strictly positive (zero would allow per-tick spam).");
        }
    }

    private static void ThrowIfZeroByte(byte value, string paramName)
    {
        if (value == 0)
        {
            throw new System.ArgumentOutOfRangeException(paramName, value,
                "SignatureConfig max-fires must be at least 1 (zero would prevent the signature from ever firing).");
        }
    }
}

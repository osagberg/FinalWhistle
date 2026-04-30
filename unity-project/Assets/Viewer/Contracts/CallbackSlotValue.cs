using System;

namespace FinalWhistle.Viewer.Contracts
{
    /// <summary>
    /// Deterministic slot value for callback-line rendering per ADR-0008
    /// §"ViewerEvent schema" / <c>CallbackSlotValue</c>. Pairs a slot name
    /// (e.g., <c>"player_name"</c>, <c>"minute"</c>) with EITHER an
    /// entity ID (preferred for player/club/signature slots; resolves
    /// locale at adapter-render time) OR a literal value (formatted with
    /// <c>InvariantCulture</c>; never with current-culture).
    ///
    /// <para>
    /// <strong>Exactly one of <see cref="EntityId"/> /
    /// <see cref="LiteralValue"/> is non-null</strong> at construction;
    /// constructor enforces this so a renderer can switch on which is set
    /// without ambiguity. Pre-rendered locale-specific text MUST flow
    /// through <see cref="EntityId"/> (resolved per-locale by the adapter)
    /// rather than <see cref="LiteralValue"/> (which is locale-invariant).
    /// </para>
    /// </summary>
    public readonly struct CallbackSlotValue : IEquatable<CallbackSlotValue>
    {
        /// <summary>
        /// Stable slot name. Sorted ordinal-ascending in the parent
        /// <see cref="MemoryHit.Slots"/> list per ADR-0008
        /// §"Determinism contract" ordering rules.
        /// </summary>
        public string SlotName { get; }

        /// <summary>
        /// Content-pack-qualified entity ID for player/club/signature
        /// slots. Resolves locale at adapter-render time.
        /// </summary>
        public string? EntityId { get; }

        /// <summary>
        /// Literal value (numeric / fixed-string). Formatted with
        /// <c>CultureInfo.InvariantCulture</c> — never with current-culture
        /// (locale drift would enter the trace).
        /// </summary>
        public string? LiteralValue { get; }

        public CallbackSlotValue(string slotName, string? entityId, string? literalValue)
        {
            if (string.IsNullOrEmpty(slotName))
            {
                throw new ArgumentException("SlotName must be non-empty.", nameof(slotName));
            }
            bool entitySet = entityId is not null;
            bool literalSet = literalValue is not null;
            if (entitySet == literalSet)
            {
                throw new ArgumentException(
                    "Exactly one of EntityId / LiteralValue must be non-null. " +
                    $"Got EntityId={(entitySet ? "set" : "null")} + " +
                    $"LiteralValue={(literalSet ? "set" : "null")}.",
                    entitySet ? nameof(entityId) : nameof(literalValue));
            }
            if (entitySet && entityId!.Length == 0)
            {
                throw new ArgumentException("EntityId must be null OR non-empty.", nameof(entityId));
            }
            // LiteralValue may be empty string deliberately (e.g., a slot
            // representing absence) — only reject null-vs-set ambiguity
            // above. SlotName is the unambiguous source of "this slot
            // exists."
            SlotName = slotName;
            EntityId = entityId;
            LiteralValue = literalValue;
        }

        /// <summary>Construct an entity-id slot.</summary>
        public static CallbackSlotValue ForEntity(string slotName, string entityId)
            => new(slotName, entityId, null);

        /// <summary>Construct a literal-value slot.</summary>
        public static CallbackSlotValue ForLiteral(string slotName, string literalValue)
            => new(slotName, null, literalValue);

        public bool Equals(CallbackSlotValue other) =>
            SlotName == other.SlotName
            && EntityId == other.EntityId
            && LiteralValue == other.LiteralValue;

        public override bool Equals(object? obj) => obj is CallbackSlotValue other && Equals(other);
        public override int GetHashCode() => HashCode.Combine(SlotName, EntityId, LiteralValue);
        public static bool operator ==(CallbackSlotValue left, CallbackSlotValue right) => left.Equals(right);
        public static bool operator !=(CallbackSlotValue left, CallbackSlotValue right) => !left.Equals(right);
        public override string ToString() =>
            EntityId is not null ? $"{SlotName}={EntityId}@entity" : $"{SlotName}={LiteralValue}@literal";
    }
}

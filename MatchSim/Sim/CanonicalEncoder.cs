using System;
using System.Buffers;
using System.Buffers.Binary;
using System.Security.Cryptography;
using System.Text;

namespace FinalWhistle.MatchSim.Sim;

/// <summary>
/// Deterministic canonical encoder for MatchSim state. Pinned by
/// <c>SerializationContract</c> tests in <c>MatchSim.Tests</c>: every primitive
/// has a literal expected byte sequence; the SHA256 of every well-known input
/// has a literal expected hash. Cross-platform Win/Mac/Linux byte-equality is
/// the contract per <c>design/specs/golden-replay-corpus.md</c> +
/// TECH_APPROACH §3.2.
///
/// <para>
/// <strong>Encoding rules (locked at v1):</strong>
/// </para>
/// <list type="bullet">
///   <item><description>Endianness: little-endian for all multi-byte values.
///         <see cref="BinaryPrimitives"/> is platform-independent;
///         <c>BitConverter</c> is not and is forbidden.</description></item>
///   <item><description>Primitives (<see cref="Fixed"/> / <see cref="Tick"/> /
///         <see cref="Seed"/>): 8 bytes little-endian over the underlying
///         64-bit value. No type tag; the schema is self-describing via
///         caller-driven ordering.</description></item>
///   <item><description>Strings: 4-byte little-endian length prefix +
///         UTF-8-encoded bytes. Length is byte count, not char count. Empty
///         string writes 4 zero bytes. Malformed UTF-16 surrogate sequences
///         throw instead of being replacement-encoded.</description></item>
///   <item><description>Counts (collection sizes): 4-byte little-endian
///         non-negative <see cref="int"/>. Negative values throw.</description></item>
///   <item><description>Booleans: 1 byte. <c>false</c> = <c>0x00</c>;
///         <c>true</c> = <c>0x01</c>. No other byte values are valid.</description></item>
/// </list>
///
/// <para>
/// <strong>Caller responsibilities</strong> per ADR-0008 §Determinism contract
/// ordering rules: collection elements MUST be sorted by ordinal comparison
/// (<c>StringComparer.Ordinal</c>) on a stable key before encoding. Map fields
/// in deterministic order. The encoder does not sort; it preserves the order
/// in which writes happen.
/// </para>
///
/// <para>
/// <strong>Hashing:</strong> <see cref="ComputeSha256Hex()"/> returns a
/// canonical-form string <c>"sha256:&lt;lowercase-hex&gt;"</c> matching
/// <c>golden-replay-corpus.md</c> hash format.
/// </para>
///
/// <para>
/// <strong>Determinism:</strong> the encoder allocates an internal growable
/// buffer (<c>System.Buffers.ArrayBufferWriter&lt;byte&gt;</c>); call <see cref="Reset"/> to
/// reuse the same instance for multiple encodings without re-allocating.
/// </para>
/// </summary>
public sealed class CanonicalEncoder
{
    private const int DefaultInitialCapacity = 256;

    private static readonly UTF8Encoding StrictUtf8 = new(encoderShouldEmitUTF8Identifier: false, throwOnInvalidBytes: true);

    private readonly ArrayBufferWriter<byte> _buffer;

    /// <summary>Construct with default initial capacity (256 bytes; grows as needed).</summary>
    public CanonicalEncoder() : this(DefaultInitialCapacity)
    {
    }

    /// <summary>Construct with caller-specified initial capacity. Useful for hot-path reuse with <see cref="Reset"/>.</summary>
    public CanonicalEncoder(int initialCapacity)
    {
        if (initialCapacity < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(initialCapacity), initialCapacity, "Initial capacity must be non-negative.");
        }
        _buffer = new ArrayBufferWriter<byte>(initialCapacity == 0 ? 1 : initialCapacity);
    }

    /// <summary>The encoded bytes written so far. Slice owned by the encoder; do not retain across later writes or <see cref="Reset"/>.</summary>
    public ReadOnlySpan<byte> WrittenSpan => _buffer.WrittenSpan;

    /// <summary>Number of bytes written so far.</summary>
    public int WrittenCount => _buffer.WrittenCount;

    #region Primitive writes

    /// <summary>Write a <see cref="Fixed"/> as 8 bytes little-endian over its raw <see cref="long"/> value.</summary>
    public void WriteFixed(Fixed value)
    {
        WriteInt64(value.RawValue);
    }

    /// <summary>Write a <see cref="Tick"/> as 8 bytes little-endian over its raw <see cref="long"/> value.</summary>
    public void WriteTick(Tick value)
    {
        WriteInt64(value.Value);
    }

    /// <summary>Write a <see cref="Seed"/> as 8 bytes little-endian over its raw <see cref="ulong"/> value.</summary>
    public void WriteSeed(Seed value)
    {
        WriteUInt64(value.Value);
    }

    /// <summary>
    /// Write a <see cref="Vector3Fixed"/> as 24 bytes: <see cref="Vector3Fixed.X"/>,
    /// <see cref="Vector3Fixed.Y"/>, <see cref="Vector3Fixed.Z"/> in that order
    /// — each 8 bytes little-endian per <see cref="WriteFixed"/>. Convenience
    /// helper; widely used by ball + player + future kinematic state.
    /// </summary>
    public void WriteVector3Fixed(Vector3Fixed value)
    {
        WriteFixed(value.X);
        WriteFixed(value.Y);
        WriteFixed(value.Z);
    }

    /// <summary>Write a 4-byte little-endian <see cref="int"/>.</summary>
    public void WriteInt32(int value)
    {
        Span<byte> dst = _buffer.GetSpan(sizeof(int));
        BinaryPrimitives.WriteInt32LittleEndian(dst, value);
        _buffer.Advance(sizeof(int));
    }

    /// <summary>Write a 4-byte little-endian <see cref="uint"/>.</summary>
    public void WriteUInt32(uint value)
    {
        Span<byte> dst = _buffer.GetSpan(sizeof(uint));
        BinaryPrimitives.WriteUInt32LittleEndian(dst, value);
        _buffer.Advance(sizeof(uint));
    }

    /// <summary>Write an 8-byte little-endian <see cref="long"/>.</summary>
    public void WriteInt64(long value)
    {
        Span<byte> dst = _buffer.GetSpan(sizeof(long));
        BinaryPrimitives.WriteInt64LittleEndian(dst, value);
        _buffer.Advance(sizeof(long));
    }

    /// <summary>Write an 8-byte little-endian <see cref="ulong"/>.</summary>
    public void WriteUInt64(ulong value)
    {
        Span<byte> dst = _buffer.GetSpan(sizeof(ulong));
        BinaryPrimitives.WriteUInt64LittleEndian(dst, value);
        _buffer.Advance(sizeof(ulong));
    }

    /// <summary>Write a single byte.</summary>
    public void WriteByte(byte value)
    {
        Span<byte> dst = _buffer.GetSpan(1);
        dst[0] = value;
        _buffer.Advance(1);
    }

    /// <summary>Write a boolean as 1 byte: <c>false</c> = <c>0x00</c>; <c>true</c> = <c>0x01</c>.</summary>
    public void WriteBool(bool value)
    {
        WriteByte(value ? (byte)0x01 : (byte)0x00);
    }

    /// <summary>
    /// Write a UTF-8 string with 4-byte little-endian length prefix (byte
    /// count, not char count). Empty string writes 4 zero bytes; null throws;
    /// malformed UTF-16 surrogate sequences throw.
    /// </summary>
    public void WriteString(string value)
    {
        if (value is null)
        {
            throw new ArgumentNullException(nameof(value));
        }

        // UTF-8 encode + length-prefix. Strict encoder rejects malformed
        // surrogate sequences instead of replacing them with U+FFFD.
        int byteCount = StrictUtf8.GetByteCount(value);
        WriteInt32(byteCount);
        if (byteCount == 0)
        {
            return;
        }

        Span<byte> dst = _buffer.GetSpan(byteCount);
        int written = StrictUtf8.GetBytes(value.AsSpan(), dst);
        if (written != byteCount)
        {
            // Defensive: GetBytes-into-span should match GetByteCount exactly.
            // If it doesn't, the underlying encoder is broken on this runtime.
            throw new InvalidOperationException($"UTF-8 encoding mismatch: expected {byteCount} bytes, encoder wrote {written}.");
        }
        _buffer.Advance(byteCount);
    }

    /// <summary>
    /// Write a non-negative collection-count as 4-byte little-endian
    /// <see cref="int"/>. Functionally identical to <see cref="WriteInt32"/>
    /// but documents intent. Negative counts throw.
    /// </summary>
    public void WriteCount(int count)
    {
        if (count < 0)
        {
            throw new ArgumentOutOfRangeException(nameof(count), count, "Collection count must be non-negative.");
        }
        WriteInt32(count);
    }

    #endregion

    #region Hashing + lifecycle

    /// <summary>
    /// Compute SHA256 of the bytes written so far and return the canonical
    /// form <c>"sha256:&lt;lowercase-hex&gt;"</c>. Computing the hash does NOT
    /// mutate the buffer; multiple calls return the same value until a write
    /// or <see cref="Reset"/>.
    /// </summary>
    public string ComputeSha256Hex()
    {
        return ComputeSha256Hex(WrittenSpan);
    }

    /// <summary>
    /// Compute SHA256 over an arbitrary span and return the canonical form
    /// <c>"sha256:&lt;lowercase-hex&gt;"</c>. Static helper for callers that
    /// have raw bytes already.
    /// </summary>
    public static string ComputeSha256Hex(ReadOnlySpan<byte> bytes)
    {
        Span<byte> hash = stackalloc byte[32];
        using (SHA256 sha = SHA256.Create())
        {
            // SHA256.TryComputeHash is netstandard2.1; allocation-free.
            if (!sha.TryComputeHash(bytes, hash, out int written) || written != 32)
            {
                throw new InvalidOperationException("SHA256 computation failed unexpectedly.");
            }
        }
        return "sha256:" + ToLowerHex(hash);
    }

    /// <summary>Reset the buffer to empty without releasing capacity. Reuse the encoder for the next encoding.</summary>
    public void Reset()
    {
        _buffer.Clear();
    }

    #endregion

    #region Internals

    private static string ToLowerHex(ReadOnlySpan<byte> bytes)
    {
        // 2 hex chars per byte. .NET 5+ has Convert.ToHexString; netstandard2.1
        // does not, so we hand-roll the lowercase form.
        const string HexChars = "0123456789abcdef";
        Span<char> chars = stackalloc char[bytes.Length * 2];
        for (int i = 0; i < bytes.Length; i++)
        {
            byte b = bytes[i];
            chars[i * 2] = HexChars[b >> 4];
            chars[i * 2 + 1] = HexChars[b & 0x0F];
        }
        return new string(chars);
    }

    #endregion
}

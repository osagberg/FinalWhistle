using System;
using System.Collections.Generic;
using System.Text;
using FinalWhistle.MatchSim.Sim;
using Xunit;

namespace FinalWhistle.MatchSim.Tests.Sim;

/// <summary>
/// SerializationContract — the executable spec for MatchSim canonical
/// serialization. Each primitive's byte encoding is pinned to a LITERAL
/// expected sequence; each well-known SHA256 hash is pinned to a literal
/// hex value. Failure of any test in this file means the on-disk byte
/// representation has drifted, which would silently invalidate every
/// pinned `golden-replay-corpus.md` fixture.
///
/// <para>
/// Cross-platform parity (Win/Mac/Linux) is the contract: BinaryPrimitives
/// gives platform-independent little-endian encoding; this file proves that
/// at the unit-test layer. The Tier-A CI matrix later asserts the SAME
/// hashes re-compute green on each OS.
/// </para>
///
/// <para>
/// Pinned SHA256 values were independently computed via openssl on
/// 2026-04-27 — they are the reference oracle. The encoder was
/// implementation-tested against these; if both ever drift in the same
/// direction the test still trips because the openssl-derived values are
/// fixed text in this file.
/// </para>
/// </summary>
public sealed class SerializationContract
{
    #region Pinned SHA256 reference values (independently computed via openssl 2026-04-27)

    private const string EmptyBufferSha256 = "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855";
    private const string EightZeroBytesSha256 = "sha256:af5570f5a1810b7af78caf4bc70a660f0df51e42baf91d4de5b2328de0e83dfc";
    private const string FixedOneSha256 = "sha256:01acecb507abfe1a354aa8064f4af5d3f1acd019e37db3c11c97523b71c76e9d";
    private const string FixedMaxValueSha256 = "sha256:6a69a6cc7473a16302890cd2a9e93e347281f6ea0e1bb784e589753bed0b3324";
    private const string TickOneSecondSha256 = "sha256:3fe8adee83a670dd3131f667e963775b2256f3f59d5a996984eb47f8375efd7c";
    private const string CorpusSmokeSeedSha256 = "sha256:743c764e0dad46651814faf26e7069fe15c2f224e33f43a4f6dbeb56e5e2b4a1";
    private const string TripleZeroSha256 = "sha256:9d908ecfb6b256def8b49a7c504e6c889c4b0e41fe6ce3e01863dd7b61a20aa0";
    private const string CafeStringSha256 = "sha256:1e5349bb37613b32941872cf4d7d69d8ee4cc0195c3a332c647ac51804064464";
    private const string EmptyStringSha256 = "sha256:df3f619804a92fdb4057192dc43dd748ea778adc52bc498ce80524c014b81119";

    #endregion

    #region Primitive byte-encoding rules (locked)

    [Fact]
    public void WriteFixed_Zero_Encodes8ZeroBytes()
    {
        CanonicalEncoder enc = new();
        enc.WriteFixed(Fixed.Zero);

        AssertBytes(enc, expected: new byte[] { 0, 0, 0, 0, 0, 0, 0, 0 });
    }

    [Fact]
    public void WriteFixed_One_EncodesLittleEndian1Shifted32()
    {
        // Fixed.One raw = 1L << 32 = 0x0000_0001_0000_0000.
        // Little-endian 8 bytes: 00 00 00 00 01 00 00 00.
        CanonicalEncoder enc = new();
        enc.WriteFixed(Fixed.One);

        AssertBytes(enc, expected: new byte[] { 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00 });
    }

    [Fact]
    public void WriteFixed_MaxValue_EncodesLongMaxValueLittleEndian()
    {
        // Fixed.MaxValue raw = long.MaxValue = 0x7FFF_FFFF_FFFF_FFFF.
        // Little-endian 8 bytes: ff ff ff ff ff ff ff 7f.
        CanonicalEncoder enc = new();
        enc.WriteFixed(Fixed.MaxValue);

        AssertBytes(enc, expected: new byte[] { 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0x7F });
    }

    [Fact]
    public void WriteFixed_MinValue_EncodesLongMinValueLittleEndian()
    {
        // Fixed.MinValue raw = long.MinValue = 0x8000_0000_0000_0000.
        // Little-endian: 00 00 00 00 00 00 00 80.
        CanonicalEncoder enc = new();
        enc.WriteFixed(Fixed.MinValue);

        AssertBytes(enc, expected: new byte[] { 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x80 });
    }

    [Fact]
    public void WriteTick_Zero_Encodes8ZeroBytes()
    {
        CanonicalEncoder enc = new();
        enc.WriteTick(Tick.Zero);

        AssertBytes(enc, expected: new byte[] { 0, 0, 0, 0, 0, 0, 0, 0 });
    }

    [Fact]
    public void WriteTick_FromSeconds1_Encodes60LittleEndian()
    {
        // Tick.FromSeconds(1) = 60 ticks. Little-endian: 3C 00 00 00 00 00 00 00.
        CanonicalEncoder enc = new();
        enc.WriteTick(Tick.FromSeconds(1));

        AssertBytes(enc, expected: new byte[] { 0x3C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00 });
    }

    [Fact]
    public void WriteSeed_Zero_Encodes8ZeroBytes()
    {
        CanonicalEncoder enc = new();
        enc.WriteSeed(Seed.Zero);

        AssertBytes(enc, expected: new byte[] { 0, 0, 0, 0, 0, 0, 0, 0 });
    }

    [Fact]
    public void WriteSeed_CorpusSmokeSeed_EncodesLittleEndian()
    {
        // Seed 0xDEADBEEFDEADBEEF — little-endian: EF BE AD DE EF BE AD DE.
        // Matches `golden-replay-corpus.md` Tier-A smoke seed canonical form.
        CanonicalEncoder enc = new();
        enc.WriteSeed(Seed.FromUInt64(0xDEADBEEFDEADBEEFUL));

        AssertBytes(enc, expected: new byte[] { 0xEF, 0xBE, 0xAD, 0xDE, 0xEF, 0xBE, 0xAD, 0xDE });
    }

    [Fact]
    public void WriteInt32_One_EncodesLittleEndian()
    {
        CanonicalEncoder enc = new();
        enc.WriteInt32(1);

        AssertBytes(enc, expected: new byte[] { 0x01, 0x00, 0x00, 0x00 });
    }

    [Fact]
    public void WriteInt32_MinusOne_EncodesTwosComplementLittleEndian()
    {
        CanonicalEncoder enc = new();
        enc.WriteInt32(-1);

        AssertBytes(enc, expected: new byte[] { 0xFF, 0xFF, 0xFF, 0xFF });
    }

    [Fact]
    public void WriteBool_FalseAndTrue_Encode0x00And0x01()
    {
        CanonicalEncoder enc = new();
        enc.WriteBool(false);
        enc.WriteBool(true);

        AssertBytes(enc, expected: new byte[] { 0x00, 0x01 });
    }

    [Fact]
    public void WriteByte_Encodes1Byte()
    {
        CanonicalEncoder enc = new();
        enc.WriteByte(0xAB);

        AssertBytes(enc, expected: new byte[] { 0xAB });
    }

    [Fact]
    public void WriteString_Empty_Encodes4ZeroBytesOnly()
    {
        // Empty string is length-prefix(0) + zero UTF-8 bytes = 4 bytes total.
        CanonicalEncoder enc = new();
        enc.WriteString(string.Empty);

        AssertBytes(enc, expected: new byte[] { 0x00, 0x00, 0x00, 0x00 });
    }

    [Fact]
    public void WriteString_Ascii_EncodesLengthPrefixThenUtf8()
    {
        // "a" = 1 UTF-8 byte (0x61). Length-prefix(1) + 0x61 = 5 bytes total.
        CanonicalEncoder enc = new();
        enc.WriteString("a");

        AssertBytes(enc, expected: new byte[] { 0x01, 0x00, 0x00, 0x00, 0x61 });
    }

    [Fact]
    public void WriteString_NonAscii_EncodesUtf8ByteCountNotCharCount()
    {
        // "café" = c-a-f-é. UTF-8 bytes: 63 61 66 c3 a9 (5 bytes; é is 2 bytes).
        // Length-prefix is 5 (BYTE count), not 4 (char count).
        CanonicalEncoder enc = new();
        enc.WriteString("café");

        AssertBytes(enc, expected: new byte[] { 0x05, 0x00, 0x00, 0x00, 0x63, 0x61, 0x66, 0xC3, 0xA9 });
    }

    [Fact]
    public void WriteCount_Zero_Encodes4ZeroBytes()
    {
        CanonicalEncoder enc = new();
        enc.WriteCount(0);

        AssertBytes(enc, expected: new byte[] { 0x00, 0x00, 0x00, 0x00 });
    }

    [Fact]
    public void WriteCount_PositiveValue_MatchesWriteInt32()
    {
        CanonicalEncoder a = new();
        a.WriteCount(42);

        CanonicalEncoder b = new();
        b.WriteInt32(42);

        Assert.Equal(b.WrittenSpan.ToArray(), a.WrittenSpan.ToArray());
    }

    #endregion

    #region Argument-validation contract

    [Fact]
    public void WriteString_Null_ThrowsArgumentNullException()
    {
        CanonicalEncoder enc = new();
        Assert.Throws<ArgumentNullException>(() => enc.WriteString(null!));
    }

    [Fact]
    public void WriteString_UnpairedSurrogate_ThrowsEncoderFallbackException()
    {
        // Canonical replay strings must be valid UTF-16. The default .NET
        // UTF8 encoder replaces malformed surrogate sequences with U+FFFD;
        // that would silently hash corrupted content instead of rejecting it.
        CanonicalEncoder enc = new();

        Assert.Throws<EncoderFallbackException>(() => enc.WriteString("\uD800"));
    }

    [Fact]
    public void WriteCount_Negative_ThrowsArgumentOutOfRangeException()
    {
        CanonicalEncoder enc = new();
        Assert.Throws<ArgumentOutOfRangeException>(() => enc.WriteCount(-1));
    }

    [Fact]
    public void Construct_NegativeInitialCapacity_ThrowsArgumentOutOfRangeException()
    {
        Assert.Throws<ArgumentOutOfRangeException>(() => new CanonicalEncoder(-1));
    }

    [Fact]
    public void Construct_ZeroInitialCapacity_DoesNotThrow()
    {
        // Edge case: ArrayBufferWriter requires positive initial capacity, but
        // our encoder treats 0 as "use minimal capacity" rather than throwing.
        CanonicalEncoder enc = new(0);
        enc.WriteByte(0xFF);
        AssertBytes(enc, expected: new byte[] { 0xFF });
    }

    #endregion

    #region SHA256 reference-hash contract (cross-platform parity bedrock)

    [Fact]
    public void ComputeSha256Hex_EmptyBuffer_MatchesPinnedReference()
    {
        // Well-known SHA256 of empty input — universal constant.
        CanonicalEncoder enc = new();
        Assert.Equal(EmptyBufferSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_FixedZero_MatchesPinnedReference()
    {
        CanonicalEncoder enc = new();
        enc.WriteFixed(Fixed.Zero);
        Assert.Equal(EightZeroBytesSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_TickZero_MatchesFixedZeroBecauseEncodingIs8ZeroBytes()
    {
        // Tick.Zero and Fixed.Zero both encode to 8 zero bytes; their hashes
        // must agree. This pins that the encoder is type-agnostic at the
        // byte level — only the underlying 64-bit value enters the hash.
        CanonicalEncoder enc = new();
        enc.WriteTick(Tick.Zero);
        Assert.Equal(EightZeroBytesSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_SeedZero_MatchesFixedZeroBecauseEncodingIs8ZeroBytes()
    {
        CanonicalEncoder enc = new();
        enc.WriteSeed(Seed.Zero);
        Assert.Equal(EightZeroBytesSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_FixedOne_MatchesPinnedReference()
    {
        CanonicalEncoder enc = new();
        enc.WriteFixed(Fixed.One);
        Assert.Equal(FixedOneSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_FixedMaxValue_MatchesPinnedReference()
    {
        CanonicalEncoder enc = new();
        enc.WriteFixed(Fixed.MaxValue);
        Assert.Equal(FixedMaxValueSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_TickFromSecondsOne_MatchesPinnedReference()
    {
        CanonicalEncoder enc = new();
        enc.WriteTick(Tick.FromSeconds(1));
        Assert.Equal(TickOneSecondSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_CorpusSmokeSeed_MatchesPinnedReference()
    {
        CanonicalEncoder enc = new();
        enc.WriteSeed(Seed.FromUInt64(0xDEADBEEFDEADBEEFUL));
        Assert.Equal(CorpusSmokeSeedSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_CanonicalTripleZero_MatchesPinnedReference()
    {
        // Composition test: writing Fixed.Zero + Tick.Zero + Seed.Zero in
        // sequence should produce 24 zero bytes; SHA256 should match the
        // independently-computed value.
        CanonicalEncoder enc = new();
        enc.WriteFixed(Fixed.Zero);
        enc.WriteTick(Tick.Zero);
        enc.WriteSeed(Seed.Zero);

        Assert.Equal(24, enc.WrittenCount);
        Assert.Equal(TripleZeroSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_CafeString_MatchesPinnedReference()
    {
        CanonicalEncoder enc = new();
        enc.WriteString("café");
        Assert.Equal(CafeStringSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_EmptyString_MatchesPinnedReference()
    {
        CanonicalEncoder enc = new();
        enc.WriteString(string.Empty);
        Assert.Equal(EmptyStringSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_StaticOverload_MatchesInstanceOverload()
    {
        CanonicalEncoder enc = new();
        enc.WriteFixed(Fixed.MaxValue);
        string instanceHash = enc.ComputeSha256Hex();
        string staticHash = CanonicalEncoder.ComputeSha256Hex(enc.WrittenSpan);

        Assert.Equal(instanceHash, staticHash);
    }

    [Fact]
    public void ComputeSha256Hex_AlwaysLowercaseHexWithSha256Prefix()
    {
        CanonicalEncoder enc = new();
        enc.WriteSeed(Seed.FromUInt64(0xFFFFFFFFFFFFFFFFUL));
        string hash = enc.ComputeSha256Hex();

        // Format: "sha256:" + 64 lowercase-hex chars = 71 total.
        Assert.StartsWith("sha256:", hash, StringComparison.Ordinal);
        Assert.Equal(7 + 64, hash.Length);
        // Body is all hex digits in lowercase.
        for (int i = 7; i < hash.Length; i++)
        {
            char c = hash[i];
            bool isLowerHex = (c >= '0' && c <= '9') || (c >= 'a' && c <= 'f');
            Assert.True(isLowerHex, $"Hash byte at position {i} ('{c}') is not lowercase-hex.");
        }
    }

    #endregion

    #region Encoder lifecycle + composition

    [Fact]
    public void WrittenCount_GrowsByExpectedSize()
    {
        CanonicalEncoder enc = new();
        Assert.Equal(0, enc.WrittenCount);

        enc.WriteByte(0xAA);
        Assert.Equal(1, enc.WrittenCount);

        enc.WriteInt32(0);
        Assert.Equal(5, enc.WrittenCount);

        enc.WriteFixed(Fixed.Zero);
        Assert.Equal(13, enc.WrittenCount);

        enc.WriteString("ab");                           // 4-byte length + 2 UTF-8 bytes
        Assert.Equal(13 + 4 + 2, enc.WrittenCount);
    }

    [Fact]
    public void Reset_ClearsBufferToEmpty()
    {
        CanonicalEncoder enc = new();
        enc.WriteFixed(Fixed.MaxValue);
        Assert.Equal(8, enc.WrittenCount);

        enc.Reset();
        Assert.Equal(0, enc.WrittenCount);
        Assert.Equal(EmptyBufferSha256, enc.ComputeSha256Hex());
    }

    [Fact]
    public void Reset_AllowsReuseWithIdenticalHashAsFreshInstance()
    {
        CanonicalEncoder reused = new();
        reused.WriteByte(0xFF);
        reused.WriteByte(0xFF);
        reused.Reset();
        reused.WriteFixed(Fixed.One);

        CanonicalEncoder fresh = new();
        fresh.WriteFixed(Fixed.One);

        Assert.Equal(fresh.ComputeSha256Hex(), reused.ComputeSha256Hex());
    }

    [Fact]
    public void ComputeSha256Hex_CalledTwice_ReturnsSameValueWithoutMutatingBuffer()
    {
        CanonicalEncoder enc = new();
        enc.WriteSeed(Seed.FromUInt64(0xDEADBEEFDEADBEEFUL));

        string first = enc.ComputeSha256Hex();
        int countAfterFirst = enc.WrittenCount;
        string second = enc.ComputeSha256Hex();

        Assert.Equal(first, second);
        Assert.Equal(countAfterFirst, enc.WrittenCount);
    }

    [Fact]
    public void Composition_OrderMatters_DifferentOrderProducesDifferentHash()
    {
        // ADR-0008 §Determinism contract: ordering rules are caller-driven.
        // The encoder MUST preserve write order; reversing should change
        // the hash. This protects against an accidental sort-on-write.
        CanonicalEncoder a = new();
        a.WriteFixed(Fixed.One);
        a.WriteFixed(Fixed.Zero);

        CanonicalEncoder b = new();
        b.WriteFixed(Fixed.Zero);
        b.WriteFixed(Fixed.One);

        Assert.NotEqual(a.ComputeSha256Hex(), b.ComputeSha256Hex());
    }

    [Fact]
    public void Composition_CountAndElements_HashMatchesConcatenation()
    {
        // A typical collection encoding: count + each element. Verify the
        // composed hash equals what we'd get from manually writing the same
        // bytes.
        ulong[] seeds = { 1UL, 2UL, 3UL };

        CanonicalEncoder structured = new();
        structured.WriteCount(seeds.Length);
        foreach (ulong s in seeds)
        {
            structured.WriteSeed(Seed.FromUInt64(s));
        }

        // Manually build the same byte sequence: 4-byte LE count + three
        // 8-byte LE seeds.
        byte[] expected = new byte[4 + (seeds.Length * 8)];
        // count = 3 → 03 00 00 00
        expected[0] = 0x03;
        // seeds: 0x0000000000000001 → 01 00 00 00 00 00 00 00 (LE)
        expected[4] = 0x01;
        expected[12] = 0x02;
        expected[20] = 0x03;

        Assert.Equal(expected, structured.WrittenSpan.ToArray());
    }

    #endregion

    #region Determinism — bedrock

    [Fact]
    public void Determinism_SameInput_SameHash100Times()
    {
        // 100 fresh encoders, same input, same expected hash. Catches any
        // hidden non-determinism (Random / DateTime / static-state).
        HashSet<string> distinctHashes = new();
        for (int i = 0; i < 100; i++)
        {
            CanonicalEncoder enc = new();
            enc.WriteFixed(Fixed.One);
            enc.WriteTick(Tick.FromSeconds(1));
            enc.WriteSeed(Seed.FromUInt64(0xDEADBEEFDEADBEEFUL));
            distinctHashes.Add(enc.ComputeSha256Hex());
        }

        Assert.Single(distinctHashes);
    }

    [Fact]
    public void Determinism_DifferentInputs_DifferentHashes()
    {
        // Sanity: 1000 distinct seeds produce 1000 distinct hashes (no
        // collision in the encoding-then-SHA256 pipeline).
        HashSet<string> hashes = new();
        for (ulong i = 0; i < 1000; i++)
        {
            CanonicalEncoder enc = new();
            enc.WriteSeed(Seed.FromUInt64(i));
            hashes.Add(enc.ComputeSha256Hex());
        }

        Assert.Equal(1000, hashes.Count);
    }

    #endregion

    #region Helpers

    private static void AssertBytes(CanonicalEncoder enc, byte[] expected)
    {
        Assert.Equal(expected, enc.WrittenSpan.ToArray());
        Assert.Equal(expected.Length, enc.WrittenCount);
    }

    #endregion
}

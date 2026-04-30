using System;
using System.IO;

namespace FinalWhistle.MatchSim.Content.Json;

/// <summary>
/// Minimal, strict, allocation-conservative JSON tokenizer for the
/// Phase-3 IdentityPacket schema per ADR-0006. Hand-rolled to remove
/// the <c>System.Text.Json</c> dependency from <c>MatchSim.csproj</c>
/// — STJ + transitive deps (System.Memory / System.Buffers /
/// System.Text.Encodings.Web / Microsoft.Bcl.AsyncInterfaces / etc.)
/// don't ship in Unity 6's Mono runtime, so a published MatchSim DLL
/// referencing STJ fails to load (Codex round-7 P1, 2026-04-30).
///
/// <para>
/// <strong>Phase-3 scope only.</strong> This reader supports the exact
/// JSON subset that the IdentityPacket fixtures use:
/// </para>
/// <list type="bullet">
///   <item><description>Objects: <c>{ "key": value, ... }</c> with strict
///       no-duplicate-keys + no-trailing-commas.</description></item>
///   <item><description>Arrays: <c>[ value, ... ]</c> with no-trailing-commas.</description></item>
///   <item><description>Strings: <c>"..."</c> with the two escapes the
///       fixtures actually use (<c>\"</c> and <c>\\</c>); other escapes
///       throw.</description></item>
///   <item><description>Numbers: signed <see cref="long"/> only — no
///       decimal point, no exponent. Q32.32 raw values fit. Floats are
///       banned from canonical state per ADR-0006 + ADR-0001.</description></item>
///   <item><description>Whitespace: ASCII space, tab, CR, LF.</description></item>
/// </list>
///
/// <para>
/// <strong>NOT supported</strong> (will throw): <c>true</c> / <c>false</c>
/// / <c>null</c> literals; <c>\uXXXX</c> unicode escapes; numbers with
/// decimals or exponents; comments. Phase-4+ may extend if a content-pack
/// schema bump introduces them.
/// </para>
///
/// <para>
/// <strong>Error policy.</strong> Every malformed-input path throws
/// <see cref="InvalidDataException"/> with a descriptive message + the
/// offending position. Callers (the schema-aware parser) treat any throw
/// as a fixture-authoring bug to surface in the validator-error list.
/// </para>
/// </summary>
internal sealed class JsonReader
{
    private readonly string _src;
    private int _pos;

    public JsonReader(string source)
    {
        _src = source ?? throw new ArgumentNullException(nameof(source));
        _pos = 0;
    }

    /// <summary>Current 0-based byte position in the source. Exposed for parse-error context only.</summary>
    public int Position => _pos;

    /// <summary>True iff <see cref="Position"/> has reached the end.</summary>
    public bool IsAtEnd => _pos >= _src.Length;

    /// <summary>
    /// Skip ASCII whitespace (space / tab / CR / LF). After this returns,
    /// either <see cref="IsAtEnd"/> is true or the next char is non-WS.
    /// </summary>
    public void SkipWhitespace()
    {
        while (_pos < _src.Length)
        {
            char c = _src[_pos];
            if (c == ' ' || c == '\t' || c == '\n' || c == '\r')
            {
                _pos++;
                continue;
            }
            break;
        }
    }

    /// <summary>Look at the next non-whitespace char without consuming.</summary>
    public char PeekNonWhitespace()
    {
        SkipWhitespace();
        if (_pos >= _src.Length)
        {
            throw Fail("unexpected end of input");
        }
        return _src[_pos];
    }

    /// <summary>Consume and assert the next non-whitespace char equals <paramref name="expected"/>.</summary>
    public void Expect(char expected)
    {
        SkipWhitespace();
        if (_pos >= _src.Length || _src[_pos] != expected)
        {
            char actual = _pos < _src.Length ? _src[_pos] : '\0';
            throw Fail($"expected '{expected}' but got '{actual}'");
        }
        _pos++;
    }

    /// <summary>
    /// Read a JSON string literal at the current position. Strict ruleset:
    /// only <c>\"</c> and <c>\\</c> escapes accepted; control chars
    /// (0x00-0x1F) must be encoded as escapes (which we don't support);
    /// any other escape throws.
    /// </summary>
    public string ReadString()
    {
        SkipWhitespace();
        if (_pos >= _src.Length || _src[_pos] != '"')
        {
            throw Fail("expected '\"' to begin string");
        }
        _pos++;  // consume opening quote

        // Fast path: scan until closing quote without escapes; allocate
        // only on the slow path (escapes present).
        int start = _pos;
        bool hasEscape = false;
        while (_pos < _src.Length)
        {
            char c = _src[_pos];
            if (c == '"')
            {
                if (!hasEscape)
                {
                    string fast = _src.Substring(start, _pos - start);
                    _pos++;
                    return fast;
                }
                break;
            }
            if (c == '\\')
            {
                hasEscape = true;
                _pos++;
                if (_pos >= _src.Length)
                {
                    throw Fail("unterminated escape sequence");
                }
                char esc = _src[_pos];
                if (esc != '"' && esc != '\\')
                {
                    throw Fail($"unsupported escape sequence \\{esc}; only \\\" and \\\\ are supported in Phase-3 IdentityPacket schema");
                }
                _pos++;
                continue;
            }
            if (c < 0x20)
            {
                throw Fail($"unescaped control char 0x{(int)c:X2} in string");
            }
            _pos++;
        }

        if (!hasEscape)
        {
            throw Fail("unterminated string (no closing '\"')");
        }

        // Slow path: replay with escape resolution.
        var sb = new System.Text.StringBuilder(_pos - start);
        int i = start;
        while (i < _src.Length)
        {
            char c = _src[i];
            if (c == '"')
            {
                _pos = i + 1;
                return sb.ToString();
            }
            if (c == '\\')
            {
                char esc = _src[i + 1];
                sb.Append(esc);  // \" → " ; \\ → \
                i += 2;
                continue;
            }
            sb.Append(c);
            i++;
        }
        throw Fail("unterminated string (no closing '\"' on slow path)");
    }

    /// <summary>
    /// Read a signed integer literal (signed <see cref="long"/>). No decimal
    /// point or exponent. Q32.32 raw values fit. Leading zeros are
    /// rejected to match canonical-JSON discipline (<c>015</c> is octal in
    /// some parsers; we reject to avoid ambiguity).
    /// </summary>
    public long ReadLong()
    {
        SkipWhitespace();
        int start = _pos;
        if (_pos < _src.Length && _src[_pos] == '-')
        {
            _pos++;
        }
        if (_pos >= _src.Length || _src[_pos] < '0' || _src[_pos] > '9')
        {
            throw Fail("expected digit to begin number");
        }
        // Reject "00", "01", "-0123" (leading zeros). "0" alone is fine; "-0" is fine.
        if (_src[_pos] == '0' && _pos + 1 < _src.Length && _src[_pos + 1] >= '0' && _src[_pos + 1] <= '9')
        {
            throw Fail("leading-zero numbers are rejected (canonical-JSON discipline)");
        }
        while (_pos < _src.Length && _src[_pos] >= '0' && _src[_pos] <= '9')
        {
            _pos++;
        }
        // Reject decimal-point / exponent — Phase-3 IdentityPacket schema is
        // long-only per ADR-0006 §canonical-JSON-rules ("no floats; Q32.32
        // integer representation").
        if (_pos < _src.Length && (_src[_pos] == '.' || _src[_pos] == 'e' || _src[_pos] == 'E'))
        {
            throw Fail("decimal/exponent in number rejected; Phase-3 schema is long-only");
        }

        string slice = _src.Substring(start, _pos - start);
        if (!long.TryParse(slice, System.Globalization.NumberStyles.Integer,
            System.Globalization.CultureInfo.InvariantCulture, out long value))
        {
            throw Fail($"number '{slice}' overflows long");
        }
        return value;
    }

    /// <summary>
    /// Construct a parse-error exception with a snippet of context around
    /// the current position so fixture-authoring bugs are easy to locate.
    /// </summary>
    public InvalidDataException Fail(string detail)
    {
        int contextStart = Math.Max(0, _pos - 20);
        int contextLen = Math.Min(40, _src.Length - contextStart);
        string context = _src.Substring(contextStart, contextLen)
            .Replace("\n", "\\n").Replace("\r", "\\r").Replace("\t", "\\t");
        return new InvalidDataException(
            $"JSON parse error at pos {_pos}: {detail}. Context: ...{context}...");
    }
}

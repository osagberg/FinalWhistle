// Polyfill for `init`-setter support on netstandard2.1.
//
// The `init`-only setter syntax was introduced in C# 9 (.NET 5+). It
// requires the marker type `System.Runtime.CompilerServices.IsExternalInit`,
// which ships in net5.0+ but is absent from netstandard2.1. Since
// MatchSim.csproj targets netstandard2.1 (Unity 6 Mono-runtime compat per
// CLAUDE.md tech-stack lock + 2026-04-28 decisions-log entry) AND
// LangVersion 14.0 (so we get the modern record syntax), we declare the
// marker locally.
//
// Internal scope: we don't expose the marker type to consumers of the
// MatchSim assembly. This file contains exactly one declaration; nothing
// else lives here.
//
// References:
// - https://learn.microsoft.com/en-us/dotnet/csharp/language-reference/proposals/csharp-9.0/init
// - https://github.com/dotnet/roslyn/issues/45284 (the canonical polyfill discussion)

namespace System.Runtime.CompilerServices;

internal static class IsExternalInit
{
}

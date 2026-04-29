---
paths:
  - "MatchSim.Tests/**/*.cs"
  - "unity-project/Assets/**/Tests/**/*.cs"
  - "unity-project/Assets/**/*Tests.cs"
  - "unity-project/Assets/**/*Test.cs"
---

<!-- Rules auto-read by Claude when editing files in this path scope. -->

# Tests — EditMode + PlayMode + Performance

Unity Test Framework. Tests are documentation that runs; keep them honest.

## MUST

- **MatchSim pure-C# tests** live under `MatchSim.Tests/<System>/` (xUnit; runs via `dotnet test FinalWhistle.slnx`; no Unity Test Framework). The cross-platform-determinism gate runs there per CI matrix in `fast-pr-ci.yml`.
- **Unity Test Framework tests** (when Phase-4+ Unity-side tests land): EditMode under `unity-project/Assets/<System>/Tests/EditMode/`. PlayMode under `unity-project/Assets/<System>/Tests/PlayMode/`. Performance under `unity-project/Assets/<System>/Tests/Performance/`. Never intermix.
- Test file name: `<Feature>Test.cs` (or `<Feature>Tests.cs`). Test class name matches file. Test method name: `<WhatIsBeingTested>_<Condition>_<Expected>`.
- EditMode for pure-logic tests — no MonoBehaviour, no scene, no coroutines. If the test needs `[UnityTest]` or `yield return`, it belongs in PlayMode.
- PlayMode tests use `[UnityTest]` + `yield return null` (or `UniTask.Yield()`), never `Thread.Sleep`, never `System.Threading.Tasks.Task.Delay`.
- Every bug fix gets a regression test. Add a row to [test-evidence.md](../../../design-templates/test-evidence.md) §Regression Watch; add the test file in the same PR/commit as the fix.
- Teardown cleanly: `[TearDown]` / `[OneTimeTearDown]` destroys test scene objects, releases Addressables handles, resets static state. Shared state across tests = flakiness.
- Performance tests use `Unity.PerformanceTesting` (`[Performance] [Test]` or `[Performance] [UnityTest]` with `Measure.Method` / `Measure.Frames`). Budgets sourced from `TECH_APPROACH.md` or the relevant ADR.

## SHOULD

- `Assert.AreEqual(expected, actual)` over `Assert.IsTrue(actual == expected)`. Better failure messages for free. Don't pad with a custom message unless the default is unclear.
- Use `[TestCase]` for parameterized tests — one test method covering 5 input ranges beats five near-duplicate methods.
- Keep tests <30 lines. If a test needs more setup than that, extract a test helper into the same folder (`<System>TestHelpers.cs`).
- Fixtures via `[SetUp]` / `[OneTimeSetUp]`. Prefer `[OneTimeSetUp]` for expensive fixtures (Addressables init, scene load) — run once per class, not per method.
- One logical assertion per test. Multiple `Assert` calls are OK if they're checking aspects of the same condition; split if they're testing different things.
- Name test classes after the SUT (`PlayerStatsTest`, not `Tests1`). Grep friendliness beats cleverness.

## AVOID

- `Thread.Sleep` in any test — use `yield return new WaitForSeconds(x)` (PlayMode) or `UniTask.Delay` with a test `CancellationToken` (EditMode async via UniTask). Sleeping blocks the main thread and makes CI slow + flaky.
- Tests that depend on test execution order. Use `[Order(N)]` only when the order is a genuine semantic dependency, not a convenience.
- External state — filesystem writes outside `Library/Temp/`, network calls, `PlayerPrefs` reads. Mock the boundary.
- Gigantic assertion messages padding every call: `Assert.AreEqual(5, result, "The player's health should be 5 after taking 5 damage from 10 max HP when ...")` — the test name already says that. Let it.
- Leaving `.meta` files without guids, or `asmdef` without a `PlayMode`/`EditMode` platform constraint. Tests compile into the wrong context otherwise.
- PlayMode tests that rely on visual rendering — use [state-dump](../../skills/state-dump/SKILL.md) for state assertions; reserve rendering checks for manual UX review.

## RATIONALE

Fast, isolated, deterministic tests get run. Slow or flaky tests get ignored and eventually deleted — the worst outcome. The EditMode/PlayMode split matters because EditMode runs in milliseconds and can be used in tight iteration; PlayMode spins up the full player loop and is slow by design. The naming convention is how `/audit` and CI tooling find tests without indexing every file. The teardown discipline prevents the "works alone, fails in suite" class of flake.

## References

- [CSharp/RULES.md](../CSharp/RULES.md) — UniTask usage, struct/class guidance
- [design-docs/RULES.md](../design-docs/RULES.md) — sibling: GDD formulas that these tests verify
- [design-templates/test-plan.md](../../../design-templates/test-plan.md) — test plan template
- [design-templates/test-evidence.md](../../../design-templates/test-evidence.md) — evidence template

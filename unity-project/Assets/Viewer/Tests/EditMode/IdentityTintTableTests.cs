using System;
using FinalWhistle.MatchSim.Content;
using FinalWhistle.MatchSim.Sim;
using FinalWhistle.Viewer.Adapters.Dots;
using NUnit.Framework;
using UnityEngine;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Slice-2 EditMode tests for <see cref="IdentityTintTable"/> +
    /// <see cref="ArchetypeRoleParser"/>. Pin the per-side keeper colour
    /// distinctness + the role-label → role-family helper invariants.
    /// Future Phase-4 IdentityPacket integration can retire
    /// <see cref="ArchetypeRoleParser"/>; these tests guard the Phase-3
    /// archetype-driven path until then.
    ///
    /// <para>
    /// <strong>SetUp/TearDown discipline (pr-review-toolkit
    /// feature-dev:code-reviewer Slice-2 P2):</strong> previous draft
    /// constructed the tint table inside each test + called
    /// <c>DestroyImmediate</c> at the bottom — assertion failures inside
    /// the test body would have leaked the SO and produced "works alone,
    /// fails in suite" flakes. Moved to <c>[SetUp]</c> + <c>[TearDown]</c>
    /// so the cleanup runs even when a body-assertion throws.
    /// </para>
    /// </summary>
    public sealed class IdentityTintTableTests
    {
        private IdentityTintTable table;

        [SetUp]
        public void SetUp()
        {
            table = ScriptableObject.CreateInstance<IdentityTintTable>();
        }

        [TearDown]
        public void TearDown()
        {
            if (table != null)
            {
                UnityEngine.Object.DestroyImmediate(table);
                table = null;
            }
        }

        // ----- Lookup invariants -----

        [Test]
        public void Lookup_GoalkeepersDifferBySide_LoudKeeperColours()
        {
            Color home = table.Lookup(RoleFamily.Goalkeeper, TeamSide.Home);
            Color away = table.Lookup(RoleFamily.Goalkeeper, TeamSide.Away);
            Assert.That(home, Is.Not.EqualTo(away),
                "Home + Away GK colours must differ for instant keeper-distinction at tactical-wide zoom.");
        }

        [Test]
        public void Lookup_OutfieldDiffersFromGoalkeeper_KeeperReadsAsLoneRole()
        {
            Color homeGK = table.Lookup(RoleFamily.Goalkeeper, TeamSide.Home);
            Color homeStriker = table.Lookup(RoleFamily.Striker, TeamSide.Home);
            Color awayGK = table.Lookup(RoleFamily.Goalkeeper, TeamSide.Away);
            Color awayStriker = table.Lookup(RoleFamily.Striker, TeamSide.Away);
            Assert.That(homeGK, Is.Not.EqualTo(homeStriker),
                "Home GK must differ from any home outfield colour (only-allowed-keeper polish-bar criterion).");
            Assert.That(awayGK, Is.Not.EqualTo(awayStriker),
                "Away GK must differ from any away outfield colour.");
        }

        [Test]
        public void Lookup_HomeAndAwayFamiliesAreDistinct_NoCrossSideCollision()
        {
            // Spot-check one role per family; the SO ships the same colour
            // for cross-band siblings (CB ≡ FB; CM ≡ DM; AM ≡ Winger) so
            // testing every pair would be noise.
            RoleFamily[] roles =
            {
                RoleFamily.Goalkeeper, RoleFamily.CentreBack, RoleFamily.DefensiveMidfielder,
                RoleFamily.AttackingMidfielder, RoleFamily.Striker,
            };
            foreach (RoleFamily role in roles)
            {
                Color home = table.Lookup(role, TeamSide.Home);
                Color away = table.Lookup(role, TeamSide.Away);
                Assert.That(home, Is.Not.EqualTo(away),
                    $"Home and Away {role} share a colour; cross-side collision breaks kit-discrimination polish-bar criterion.");
            }
        }

        // ----- ArchetypeRoleParser invariants (extracted from IdentityTintTable per type-design-analyzer Slice-2 P1) -----

        [TestCase("GK", RoleFamily.Goalkeeper)]
        [TestCase("CB", RoleFamily.CentreBack)]
        [TestCase("RCB", RoleFamily.CentreBack)]
        [TestCase("LCB", RoleFamily.CentreBack)]
        [TestCase("RB", RoleFamily.FullBack)]
        [TestCase("LB", RoleFamily.FullBack)]
        [TestCase("CDM", RoleFamily.DefensiveMidfielder)]
        [TestCase("DM", RoleFamily.DefensiveMidfielder)]
        [TestCase("CM", RoleFamily.CentralMidfielder)]
        [TestCase("RCM", RoleFamily.CentralMidfielder)]
        [TestCase("LCM", RoleFamily.CentralMidfielder)]
        [TestCase("CAM", RoleFamily.AttackingMidfielder)]
        [TestCase("AM", RoleFamily.AttackingMidfielder)]
        [TestCase("RM", RoleFamily.Winger)]
        [TestCase("LM", RoleFamily.Winger)]
        [TestCase("RW", RoleFamily.Winger)]
        [TestCase("LW", RoleFamily.Winger)]
        [TestCase("ST", RoleFamily.Striker)]
        [TestCase("RST", RoleFamily.Striker)]
        [TestCase("LST", RoleFamily.Striker)]
        [TestCase("CF", RoleFamily.Striker)]
        public void RoleFamilyForLabel_KnownLabels_ResolveToCorrectFamily(string label, RoleFamily expected)
        {
            Assert.That(ArchetypeRoleParser.RoleFamilyForLabel(label), Is.EqualTo(expected));
        }

        [Test]
        public void RoleFamilyForLabel_UnknownLabel_Throws()
        {
            Assert.Throws<ArgumentException>(
                () => ArchetypeRoleParser.RoleFamilyForLabel("XYZ"));
        }

        [Test]
        public void RoleFamilyForLabel_LowercaseLabel_ThrowsRatherThanCoercing()
        {
            // Per ArchetypeRoleParser doc-comment + pr-review-toolkit
            // silent-failure-hunter Slice-2 P3 (case-sensitive contract): a
            // lowercase YAML label is a content-pack lint failure that must
            // surface loudly, not get silently coerced via ToUpperInvariant.
            Assert.Throws<ArgumentException>(
                () => ArchetypeRoleParser.RoleFamilyForLabel("gk"));
            Assert.Throws<ArgumentException>(
                () => ArchetypeRoleParser.RoleFamilyForLabel("St"));
        }

        [Test]
        public void RoleFamilyForLabel_Empty_Throws()
        {
            Assert.Throws<ArgumentException>(
                () => ArchetypeRoleParser.RoleFamilyForLabel(""));
            Assert.Throws<ArgumentException>(
                () => ArchetypeRoleParser.RoleFamilyForLabel("   "));
        }
    }
}

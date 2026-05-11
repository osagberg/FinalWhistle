using FinalWhistle.Viewer.Adapters.Dots;
using NUnit.Framework;
using UnityEngine;

namespace FinalWhistle.Viewer.Tests.EditMode
{
    /// <summary>
    /// Slice-7 EditMode tests for <see cref="DotPool.IndexForFocalSubject"/>.
    /// Pin the strict-parser contract for the
    /// <c>"viewer.focal:&lt;side&gt;.&lt;jersey&gt;"</c> literal produced by
    /// <c>Viewer.EventBridge</c>. The parser is pure string operations
    /// + does NOT require an initialized pool, so these tests construct
    /// a bare <see cref="DotPool"/> via <c>AddComponent</c> without
    /// calling <c>Initialize</c>.
    ///
    /// <para>
    /// Dot-index contract: home jerseys 1-11 → dot indices 0-10; away
    /// jerseys 1-11 → dot indices 11-21. Pinned because the index
    /// mapping is the bridge between <c>ViewerEvent.FocalSubject</c>
    /// strings (jersey-based, human-readable) + the snapshot arrays the
    /// rest of the pool maintains (roster-slot-ordered, machine-indexed).
    /// </para>
    /// </summary>
    public sealed class DotPoolFocalSubjectTests
    {
        private GameObject host;
        private DotPool pool;

        [SetUp]
        public void SetUp()
        {
            host = new GameObject("DotPoolFocalSubjectTestHost");
            pool = host.AddComponent<DotPool>();
        }

        [TearDown]
        public void TearDown()
        {
            if (host != null)
            {
                Object.DestroyImmediate(host);
                host = null;
                pool = null;
            }
        }

        // ----- Valid forms (zero-padded + unpadded) -----

        [TestCase("viewer.focal:home.1", 0)]
        [TestCase("viewer.focal:home.01", 0)]
        [TestCase("viewer.focal:home.06", 5)]
        [TestCase("viewer.focal:home.6", 5)]
        [TestCase("viewer.focal:home.11", 10)]
        [TestCase("viewer.focal:away.1", 11)]
        [TestCase("viewer.focal:away.01", 11)]
        [TestCase("viewer.focal:away.9", 19)]
        [TestCase("viewer.focal:away.11", 21)]
        public void IndexForFocalSubject_Valid_ResolvesToCorrectDotIndex(string focal, int expectedIndex)
        {
            int actual = pool.IndexForFocalSubject(focal);
            Assert.AreEqual(expectedIndex, actual);
        }

        // ----- Null / empty -----

        [Test]
        public void IndexForFocalSubject_Null_ReturnsMinusOne()
        {
            Assert.AreEqual(-1, pool.IndexForFocalSubject(null));
        }

        [Test]
        public void IndexForFocalSubject_Empty_ReturnsMinusOne()
        {
            Assert.AreEqual(-1, pool.IndexForFocalSubject(string.Empty));
        }

        // ----- Malformed prefix / separator -----

        [TestCase("home.6")]                          // missing prefix
        [TestCase("Viewer.focal:home.6")]             // case-sensitive prefix
        [TestCase("viewer.focal:.6")]                 // empty side
        [TestCase("viewer.focal:home.")]              // empty jersey
        [TestCase("viewer.focal:home6")]              // missing dot separator
        [TestCase("viewer.focal:home/6")]             // wrong separator
        [TestCase("viewer.focal:")]                   // truncated
        [TestCase("viewer.focal:home")]               // no dot, no jersey
        public void IndexForFocalSubject_MalformedFormat_ReturnsMinusOne(string focal)
        {
            Assert.AreEqual(-1, pool.IndexForFocalSubject(focal));
        }

        // ----- Wrong side -----

        [TestCase("viewer.focal:HOME.6")]             // uppercase HOME rejected (lowercase only per spec)
        [TestCase("viewer.focal:Home.6")]             // mixed case rejected
        [TestCase("viewer.focal:neutral.6")]          // unknown side
        [TestCase("viewer.focal:h.6")]                // abbreviated rejected
        public void IndexForFocalSubject_WrongSide_ReturnsMinusOne(string focal)
        {
            Assert.AreEqual(-1, pool.IndexForFocalSubject(focal));
        }

        // ----- Jersey number out of range -----

        [TestCase("viewer.focal:home.0")]             // zero
        [TestCase("viewer.focal:home.12")]            // above max (Phase-3 caps at 11)
        [TestCase("viewer.focal:home.99")]            // far above
        [TestCase("viewer.focal:away.0")]
        [TestCase("viewer.focal:away.12")]
        public void IndexForFocalSubject_JerseyOutOfRange_ReturnsMinusOne(string focal)
        {
            Assert.AreEqual(-1, pool.IndexForFocalSubject(focal));
        }

        // ----- Jersey number not a clean integer -----

        [TestCase("viewer.focal:home.+6")]            // leading + rejected
        [TestCase("viewer.focal:home.-6")]            // negative rejected
        [TestCase("viewer.focal:home.6.5")]           // decimal rejected
        [TestCase("viewer.focal:home.6e0")]           // scientific rejected
        [TestCase("viewer.focal:home.0x6")]           // hex rejected
        [TestCase("viewer.focal:home. 6")]            // leading whitespace rejected
        [TestCase("viewer.focal:home.6 ")]            // trailing whitespace rejected
        [TestCase("viewer.focal:home.six")]           // non-numeric rejected
        public void IndexForFocalSubject_JerseyNotCleanInteger_ReturnsMinusOne(string focal)
        {
            Assert.AreEqual(-1, pool.IndexForFocalSubject(focal));
        }
    }
}

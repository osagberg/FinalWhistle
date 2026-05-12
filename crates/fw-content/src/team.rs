//! `TeamTemplate` — content-pack-authored club description.
//!
//! T0 stub: enough fields for the type to be referenced from `fw-content`'s
//! public surface. T2 fills in the full schema (formation graph, hierarchy,
//! signing budgets, scout regions).

use fw_core::ClubId;
use serde::{Deserialize, Serialize};

/// A club-template entry from a content pack.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TeamTemplate {
    /// Stable handle. Allocated by the content compiler at pack-bake time.
    pub id: ClubId,

    /// Content-pack-qualified textual ID (`fwh.core:club_00042`). The
    /// canonical form for cross-pack references.
    pub qualified_id: String,

    /// Display name. T2 will localize.
    pub display_name: String,
}

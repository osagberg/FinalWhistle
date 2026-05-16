//! `AppState` — Tauri-managed resource constructed once at app startup.
//!
//! T1-5 closes the T1-11 fix-pass deferral: instead of reloading
//! `ContentStore` on every IPC command (~10ms per call), the store is
//! loaded once at boot and injected via `tauri::Builder::manage(AppState)`.
//! Command handlers receive `tauri::State<'_, AppState>` and read the
//! pre-loaded store without touching the filesystem.
//!
//! The `Arc<BTreeMap<...>>` for signature_definitions is extracted at
//! construction time — Arc-clone per command avoids re-borrowing the whole
//! ContentStore across the async command boundary.

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use fw_content::{ContentLoadError, ContentStore, SignatureDefinition};

/// Application-level state managed by Tauri.
///
/// Held in a `tauri::State<'_, AppState>` behind a `Mutex` inside Tauri's
/// state container. Command handlers borrow it read-only; the state is never
/// mutated after construction.
///
/// Fields are `pub(crate)` — external consumers go through the
/// [`content`](Self::content) / [`signature_definitions`](Self::signature_definitions)
/// accessors so the "never mutated after construction" invariant is
/// type-enforced, not just doc'd. T1-5 type-design audit P2 (F1).
pub struct AppState {
    pub(crate) content: ContentStore,
    /// Arc-clone of `content.signature_definitions` for cheap per-command
    /// access without re-borrowing `content` across the async boundary.
    pub(crate) signature_definitions: Arc<BTreeMap<String, SignatureDefinition>>,
}

impl AppState {
    /// Construct `AppState` from a content root directory.
    ///
    /// Loads the full `ContentStore` from the given path, then Arc-clones
    /// `signature_definitions` for per-command perf-cached access. Called
    /// once at app startup.
    pub fn new(content_root: &Path) -> Result<Self, ContentLoadError> {
        let content = ContentStore::load_sources(content_root)?;
        let signature_definitions = Arc::new(content.signature_definitions.clone());
        Ok(AppState {
            content,
            signature_definitions,
        })
    }

    /// Read-only access to the loaded [`ContentStore`].
    pub fn content(&self) -> &ContentStore {
        &self.content
    }

    /// Read-only access to the perf-cached signature definitions table.
    ///
    /// This is an `Arc`-clone of `content.signature_definitions` captured at
    /// construction time; it stays consistent because `AppState` is
    /// immutable after construction. Phase-2 modding (hot-reload) will need
    /// to replace the whole `AppState` rather than mutate it in place.
    pub fn signature_definitions(&self) -> &Arc<BTreeMap<String, SignatureDefinition>> {
        &self.signature_definitions
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn workspace_content_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("content")
    }

    #[test]
    fn new_loads_content_store_successfully() {
        let state = AppState::new(&workspace_content_path())
            .expect("AppState::new should succeed with workspace content/");
        // sig_definitions Arc shares the same entries as the store itself.
        assert_eq!(
            state.signature_definitions().len(),
            state.content().signature_definitions.len(),
        );
    }

    #[test]
    fn arc_clone_does_not_double_the_signatures() {
        let state = AppState::new(&workspace_content_path()).expect("AppState::new");
        let cloned = Arc::clone(state.signature_definitions());
        assert_eq!(cloned.len(), state.signature_definitions().len());
    }
}

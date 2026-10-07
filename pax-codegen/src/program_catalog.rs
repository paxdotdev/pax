//! Build-time entry descriptions and shared component definitions.
//!
//! This is compiler infrastructure, not a runtime registration mechanism.
use color_eyre::eyre::{eyre, Result};
use pax_manifest::{PaxManifest, TypeId};
use std::path::PathBuf;

/// An authored runnable declaration. Paths retain their declaration scope so
/// frontends can perform Rust signature checks without moving private items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunnableSource {
    pub file: PathBuf,
    /// One-based line and zero-based character column of the first input token.
    pub line: usize,
    pub column: usize,
    pub module: String,
    pub root_path: String,
    pub initializer: Option<String>,
    pub engine_import_path: Option<String>,
}

/// One component that may be selected as an application root.
#[derive(Clone, Debug)]
pub struct RunnableEntry {
    pub root: TypeId,
    /// All declaration sites, including equivalent aliases and repetitions.
    pub declarations: Vec<RunnableSource>,
}

/// A package's runnable roots and the union of their reachable definitions.
///
/// Analysis visits each component once, even when roots share subtrees or one
/// root contains another. This value contains no component instances, property
/// graphs, executors, or initialized application state. Frontends still decide
/// which artifact owns platform exports and how the caller selects an entry.
pub struct ProgramCatalog {
    entries: Vec<RunnableEntry>,
    definitions: PaxManifest,
}

impl ProgramCatalog {
    pub(crate) fn new(entries: Vec<RunnableEntry>, definitions: PaxManifest) -> Self {
        Self {
            entries,
            definitions,
        }
    }

    /// Entries sorted by canonical component identity, independent of discovery
    /// order. Repeated declarations of a root occupy one entry.
    pub fn entries(&self) -> &[RunnableEntry] {
        &self.entries
    }

    /// Select a root without repeating source analysis or changing the catalog.
    ///
    /// Omitting `root` is valid only for a single-entry catalog. The resulting
    /// manifest owns its root role; other runnable components remain ordinary
    /// component definitions when used below this root. Initializers are never
    /// evaluated by this operation.
    pub fn manifest_for(&self, root: Option<&TypeId>) -> Result<PaxManifest> {
        let entry = match root {
            Some(root) => self
                .entries
                .iter()
                .find(|entry| &entry.root == root)
                .ok_or_else(|| eyre!("`{root}` is not a declared runnable root"))?,
            None if self.entries.len() == 1 => &self.entries[0],
            None => {
                return Err(eyre!(
                    "Select a runnable root explicitly; this package declares: {}",
                    self.entries
                        .iter()
                        .map(|entry| entry.root.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                ))
            }
        };
        let mut manifest = self.definitions.clone();
        manifest.main_component_type_id = entry.root.clone();
        for definition in manifest.components.values_mut() {
            definition.is_main_component = definition.type_id == entry.root;
        }
        Ok(manifest)
    }
}

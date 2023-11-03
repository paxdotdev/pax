//! # Reexports Module
//!
//! The `reexports` module provides structures and functions for generating the rexports.partial.rs file.
//! This file is used to re-export all of the dependencies of the user's template so that the generated
//! cartridge can use them.

use std::fs;

use itertools::Itertools;

use std::path::{Path, PathBuf};

use crate::manifest::PaxManifest;
mod namespace_trie_node;
pub use namespace_trie_node::NamespaceTrieNode;

#[cfg(test)]
mod tests;

const REEXPORTS_PARTIAL_FILE_NAME: &str = "reexports.partial.rs";

/// Returns a sorted and de-duped list of combined_reexports.
pub fn generate_reexports_partial_rs(pax_dir: &PathBuf, manifest: &PaxManifest) {
    let imports = manifest.import_paths.clone().into_iter().sorted().collect();

    let file_contents = &bundle_reexports_into_namespace_string(&imports);

    let path = pax_dir.join(Path::new(REEXPORTS_PARTIAL_FILE_NAME));
    fs::write(path, file_contents).unwrap();
}

fn bundle_reexports_into_namespace_string(sorted_reexports: &Vec<String>) -> String {
    let mut root = NamespaceTrieNode {
        node_string: None,
        children: Default::default(),
    };

    for s in sorted_reexports {
        root.insert(s);
    }

    root.serialize_to_reexports()
}

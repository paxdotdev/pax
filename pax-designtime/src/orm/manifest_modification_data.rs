use std::collections::HashSet;

use serde::{Deserialize, Serialize};

// Used to expose more granular updates to the designer
#[derive(Default, Serialize, Deserialize)]
pub struct ManifestModificationData {
    pub modified_properties: HashSet<String>,
    pub tree_modified: bool,
}

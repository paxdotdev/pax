use crate::orm::PaxManifestORM;
use crate::selection::PaxSelectionManager;
use crate::undo::PaxUndoManager;

pub mod orm;
pub mod messages;
pub mod selection;
pub mod undo;

mod serde_pax;

pub use serde_pax::de::{from_pax, Deserializer};
pub use serde_pax::error::{Error, Result};
pub use serde_pax::se::{to_pax, Serializer};

pub struct DesigntimeApi {
    orm: PaxManifestORM,
    selection: PaxSelectionManager,
    undo_stack: PaxUndoManager,
}
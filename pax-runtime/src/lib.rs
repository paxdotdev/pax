pub mod api;
pub mod cartridge;
pub mod component;
pub mod conditional;
pub mod constants;
/// Container-facing child ontology and geometry seams.
pub mod container;
#[cfg(feature = "designtime")]
pub mod designtime_support;
pub mod engine;
pub mod form_event;
pub mod layout;
pub mod properties;
pub mod rendering;
pub mod repeat;
pub mod router;
pub mod settings_motion;
pub mod slot;

pub use crate::cartridge::*;
pub use crate::component::*;
pub use crate::conditional::*;
pub use crate::container::*;
pub use crate::engine::*;
pub use crate::layout::*;
pub use crate::properties::*;
pub use crate::rendering::*;
pub use crate::repeat::*;
pub use crate::router::*;
pub use crate::settings_motion::*;
pub use crate::slot::*;

#[allow(unused)]
pub static DEBUG_TEXT_GREEN_BACKGROUND: bool = false;

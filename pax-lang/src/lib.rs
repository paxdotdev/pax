pub extern crate pax_macro;
pub use pax_macro::*;

pub use pax_runtime_api as api;

pub use pax_runtime_api::log;
pub use pax_runtime_api::Property;

#[cfg(feature = "parser")]
pub mod internal {
    pub use pax_compiler_api::manifest as message;
    pub use pax_compiler_api::PathQualifiable as PropertyManifestable;
}
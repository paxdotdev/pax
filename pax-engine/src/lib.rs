pub extern crate pax_macro;
pub use pax_macro::*;

pub use log;
pub use pax_lang;
pub use pax_manifest;
pub use pax_message;
pub use pax_runtime;
pub use pax_runtime::api;
pub use pax_runtime::api::math;
pub use pax_runtime::api::serde;
pub use pax_runtime::api::CoercionRules;
pub use pax_runtime::api::PaxValue;
pub use pax_runtime::api::Property;
pub use pax_runtime::api::ToPaxValue;
pub use pax_runtime::engine::node_interface::*;
pub use pax_runtime::layout;
pub use pax_runtime::rendering;
pub use pax_runtime::Slot;

#[cfg(feature = "designtime")]
pub use pax_designtime;

#[cfg(feature = "web")]
pub use {pax_chassis_web, wasm_bindgen, wasm_bindgen_futures};

#[cfg(any(feature = "macos", feature = "ios"))]
pub use pax_chassis_common;

pub use serde_json;

mod declarative_macros {
    #[macro_export]
    macro_rules! pax_struct {
        ($name:ident { $($field:ident : $value:expr),* $(,)? }) => {
            $name {
                $(
                    $field: Property::new($value),
                )*
            }
        };
    }
}

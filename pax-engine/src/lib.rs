pub extern crate pax_macro;
pub use pax_macro::*;

pub use pax_runtime::api;
pub use pax_runtime::engine::node_interface::*;
pub use pax_runtime::math;
pub use pax_runtime::rendering;

pub use pax_runtime::api::log;
pub use pax_runtime::api::serde;
pub use pax_runtime::api::Property;

mod declarative_macros {
    #[macro_export]
    macro_rules! pax_struct {
        ($name:ident { $($field:ident : $value:expr),* $(,)? }) => {
            $name {
                $(
                    $field: Box::new(PropertyLiteral::new($value)),
                )*
            }
        };
    }
}

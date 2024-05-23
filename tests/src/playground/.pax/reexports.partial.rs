pub mod pax_reexports {
    pub use bool;
    pub use crate::Example;
    pub mod pax_engine{
        pub mod api{
            pub use pax_engine::api::Color;
            pub use pax_engine::api::ColorChannel;
            pub use pax_engine::api::Fill;
            pub use pax_engine::api::Numeric;
            pub use pax_engine::api::Rotation;
            pub use pax_engine::api::Size;
            pub use pax_engine::api::Stroke;
            pub use pax_engine::api::Transform2D;
        }
    }
    pub mod pax_std{
        pub mod primitives{
            pub use pax_std::primitives::BlankComponent;
            pub use pax_std::primitives::Group;
            pub use pax_std::primitives::Rectangle;
            pub use pax_std::primitives::Text;
        }
        pub mod types{
            pub use pax_std::types::RectangleCornerRadii;
            pub mod text{
                pub use pax_std::types::text::Font;
                pub use pax_std::types::text::FontStyle;
                pub use pax_std::types::text::FontWeight;
                pub use pax_std::types::text::LocalFont;
                pub use pax_std::types::text::SystemFont;
                pub use pax_std::types::text::TextAlignHorizontal;
                pub use pax_std::types::text::TextAlignVertical;
                pub use pax_std::types::text::TextStyle;
                pub use pax_std::types::text::WebFont;
            }
        }
    }
    pub mod std{
        pub mod string{
            pub use std::string::String;
        }
    }
    pub use usize;

}
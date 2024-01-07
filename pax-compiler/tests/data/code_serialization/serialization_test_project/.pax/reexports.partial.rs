pub mod pax_reexports {
    pub use bool;
    pub use crate::Example;
    pub use f64;
    pub mod pax_lang{
        pub mod api{
            pub use pax_lang::api::Numeric;
            pub use pax_lang::api::Rotation;
            pub use pax_lang::api::Size;
            pub use pax_lang::api::SizePixels;
            pub use pax_lang::api::StringBox;
            pub use pax_lang::api::Transform2D;
        }
    }
    pub mod pax_std{
        pub mod primitives{
            pub use pax_std::primitives::Rectangle;
            pub use pax_std::primitives::Text;
        }
        pub mod types{
            pub use pax_std::types::Color;
            pub use pax_std::types::ColorVariant;
            pub use pax_std::types::Fill;
            pub use pax_std::types::GradientStop;
            pub use pax_std::types::LinearGradient;
            pub use pax_std::types::RadialGradient;
            pub use pax_std::types::RectangleCornerRadii;
            pub use pax_std::types::Stroke;
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
        pub mod vec{
            pub use std::vec::Vec;
        }
    }
    pub use usize;

}
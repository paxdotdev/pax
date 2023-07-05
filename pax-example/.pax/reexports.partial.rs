pub mod pax_reexports {
    pub use bool;
    pub use crate::Example;
    pub mod camera {
        pub use crate::camera::Camera;
        pub use crate::camera::TypeExample;
    }
    pub mod fireworks {
        pub use crate::fireworks::Fireworks;
    }
    pub mod grids {
        pub use crate::grids::Grids;
        pub use crate::grids::RectDef;
    }
    pub mod hello_rgb {
        pub use crate::hello_rgb::HelloRGB;
    }
    pub mod words {
        pub use crate::words::Words;
    }
    pub use f64;
    pub mod pax_lang{
        pub mod api{
            pub use pax_lang::api::Numeric;
            pub use pax_lang::api::Size;
            pub use pax_lang::api::SizePixels;
        }
    }
    pub mod pax_std{
        pub mod primitives{
            pub use pax_std::primitives::Ellipse;
            pub use pax_std::primitives::Frame;
            pub use pax_std::primitives::Group;
            pub use pax_std::primitives::Rectangle;
            pub use pax_std::primitives::Text;
        }
        pub mod stacker{
            pub use pax_std::stacker::Stacker;
        }
        pub mod types{
            pub use pax_std::types::Color;
            pub use pax_std::types::ColorVariant;
            pub use pax_std::types::StackerCell;
            pub use pax_std::types::StackerDirection;
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
        pub mod option{
            pub use std::option::Option;
        }
        pub mod string{
            pub use std::string::String;
        }
        pub mod vec{
            pub use std::vec::Vec;
        }
    }
    pub use usize;

}
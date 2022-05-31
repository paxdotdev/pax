#[macro_use]
extern crate lazy_static;

use pax::*;

pub mod types;
 mod stacker;
// pub mod stacker;

pub mod components {
    pub use super::stacker::*;
}


use pax::api::PropertyInstance;

pub mod primitives {
    use pax::pax_primitive;

    #[cfg(feature = "parser")]
    use pax_compiler_api;
    #[cfg(feature = "parser")]
    use pax_compiler_api::ManifestContext;

    #[pax_primitive("./pax-std-primitives", crate::FrameInstance)]
    pub struct Frame {}

    #[pax_primitive("./pax-std-primitives", crate::GroupInstance)]
    pub struct Group {}

    #[pax_primitive("./pax-std-primitives", crate::RectangleInstance)]
    pub struct Rectangle {
        pub stroke: crate::types::Stroke,
        pub fill: Box<dyn pax::api::PropertyInstance<crate::types::Color>>,
    }

    #[pax_primitive("./pax-std-primitives", crate::TextInstance)]
    pub struct Text {
        pub content: Box<dyn pax::api::PropertyInstance<String>>,
        pub font: crate::types::Font,
        pub fill: Box<dyn pax::api::PropertyInstance<crate::types::Color>>,
    }
}

#[macro_use]
extern crate lazy_static;


use pax::*;

pub mod types;
pub mod stacker;

pub mod components {
    pub use super::stacker::*;
}


pub mod primitives {
    use pax::pax_primitive;

    #[cfg(feature = "parser")]
    use pax_compiler;
    #[cfg(feature = "parser")]
    use pax_compiler::ParsingContext;
    #[cfg(feature = "parser")]
    use pax_compiler::reflection::PathQualifiable;

    #[pax_primitive("./pax-std-primitives",  crate::FrameInstance)]
    pub struct Frame {}

    #[pax_primitive("./pax-std-primitives",  crate::GroupInstance)]
    pub struct Group {}

    #[pax_primitive("./pax-std-primitives",  crate::RectangleInstance)]
    pub struct Rectangle {
        pub stroke: pax::Property<crate::types::Stroke>,
        pub fill: pax::Property<crate::types::Color>,
    }

    #[pax_primitive("./pax-std-primitives",  crate::TextInstance)]
    pub struct Text {
        pub content: pax::Property<String>,
        pub font: pax::Property<crate::types::Font>,
        pub fill: pax::Property<crate::types::Color>,
    }
}

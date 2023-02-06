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
    use pax_compiler::parsing::ParsingContext;
    #[cfg(feature = "parser")]
    use pax_compiler::reflection::PathQualifiable;

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::frame::FrameInstance)]
    pub struct Frame {}

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::group::GroupInstance)]
    pub struct Group {}

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::rectangle::RectangleInstance)]
    pub struct Rectangle {
        pub stroke: pax::Property<crate::types::Stroke>,
        pub fill: pax::Property<crate::types::Color>,
    }

    #[pax_primitive("./pax-std-primitives",  pax_std_primitives::text::TextInstance)]
    pub struct Text {
        pub text: pax::Property<String>,
        pub font: pax::Property<crate::types::Font>,
        pub fill: pax::Property<crate::types::Color>,
    }
}

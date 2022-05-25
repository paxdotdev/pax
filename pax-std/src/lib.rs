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
    use crate::types;
    use crate::types::{Color, Font};
    use pax::pax_primitive;

    #[pax_primitive("./pax-std-primitives", crate::FrameInstance)]
    pub struct Frame {}

    #[pax_primitive("./pax-std-primitives", crate::GroupInstance)]
    pub struct Group {}

    use std::rc::Rc;
    use pax::api::SizePixels;
    

    #[pax_primitive("./pax-std-primitives", crate::RectangleInstance)]
    pub struct Rectangle {
        pub stroke: types::Stroke,
        pub fill: Box<dyn pax::api::PropertyInstance<types::Color>>,
    }

    #[pax_primitive("./pax-std-primitives", crate::TextInstance)]
    pub struct Text {
        pub content: Box<dyn pax::api::PropertyInstance<String>>,
        pub font: types::Font,
        pub fill: Box<dyn pax::api::PropertyInstance<Color>>,
    }


}

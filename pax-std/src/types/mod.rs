use pax::*;

//register in coproduct;
//generate unwrapping code in cartridge-runtime (if let/match&panic)
//(will be wrapped up as a PropertyCoproduct)
#[pax_primitive_type("../pax-std-primitives", crate::StrokeInstance)]
pub struct Stroke {}

#[pax_primitive_type("../pax-std-primitives", crate::TransformInstance)]
#[derive(Default)]
pub struct Transform {
    operations: Option<Vec<Transform>>, ///when provided, take aggregate product of provided Transforms and prepend to
                                        /// any other self-contained Transforms.  This enables precise expression of transform order,
                                        /// as a (hierarchical & recursive) sequence of affine operations
    rotate: Option<f64>, ///over z axis
    translate: Option<[f64; 2]>,
    scale: Option<[f64; 2]>,
    origin: Option<[f64; 2]>,
    align: Option<[f64; 2]>,
}

#[pax_primitive_type("../pax-std-primitives", crate::ColorInstance)]
pub struct Color {
    h: f64,
    s: f64,
    l: f64,
    a: f64,
}

impl Color {
    pub fn hsla(h:f64, s:f64, l:f64, a:f64) -> Self {
        Self { h,s,l,a, }
    }
}


#[pax_primitive_type("../pax-std-primitives", crate::Size)]
pub use pax::api::Size;
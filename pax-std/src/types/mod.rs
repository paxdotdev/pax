use pax::*;

//register in coproduct;
//generate unwrapping code in cartridge-runtime (if let/match&panic)
//(will be wrapped up as a PropertyCoproduct)
#[pax_primitive_type("../pax-std-primitives", crate::StrokeInstance)]
pub struct Stroke {}



#[pax_primitive_type("../pax-std-primitives", crate::ColorInstance)]
pub struct Color{
    color_variant: ColorVariant,
}

pub enum ColorVariant {
    Hlca([f64; 4]),
    Rgba([f64; 4]),
}

impl Color {
    pub fn hlca(h:f64, l:f64, c:f64, a:f64) -> Self {
        Self {color_variant: ColorVariant::Hlca([h,l,c,a])}
    }
    pub fn rgba(r:f64, g:f64, b:f64, a:f64) -> Self {
        Self {color_variant: ColorVariant::Rgba([r,g,b,a])}
    }
}

#[pax_primitive_type("../pax-std-primitives", crate::Size)]
pub use pax::api::Size;
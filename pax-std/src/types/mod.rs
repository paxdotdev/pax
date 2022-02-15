use pax::*;
use pax::api::{Property, PropertyLiteral};

//register in coproduct;
//generate unwrapping code in cartridge-runtime (if let/match&panic)
//(will be wrapped up as a PropertyCoproduct)
// #[pax_type]
pub struct Stroke {
    pub color: Color,
    pub width: f64,
}

//this allows StrokeProperties to be set at runtime with an ergonomic Stroke object
impl Into<StrokeProperties> for Stroke {
    fn into(self) -> StrokeProperties {
        StrokeProperties {
            color: Box::new(PropertyLiteral {value: self.color}),
            width: Box::new(PropertyLiteral {value: self.width}),
        }
    }
}

#[pax_type]
pub struct StrokeProperties {
    pub color: Box<dyn Property<Color>>,
    pub width: Box<dyn Property<f64>>,
}


#[pax_primitive_type("../pax-std-primitives", crate::ColorInstance)]
pub struct Color{
    pub color_variant: ColorVariant,
}

impl Color {
    pub fn to_piet_color(&self) -> piet::Color {
        match self.color_variant {
            ColorVariant::Hlca(slice) => {
                piet::Color::hlca(slice[0], slice[1], slice[2], slice[3])
            },
            ColorVariant::Rgba(slice) => {
                piet::Color::rgba(slice[0], slice[1], slice[2], slice[3])
            }
        }
    }
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
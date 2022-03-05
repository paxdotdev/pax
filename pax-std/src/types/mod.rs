use pax::*;
use pax::api::{Property, PropertyLiteral};


#[pax_type]
#[derive(Default)]
pub struct Stroke {
    pub color: Box<dyn Property<Color>>,
    pub width: Box<dyn Property<f64>>,
}


#[pax_type]
pub struct Color{
    pub color_variant: ColorVariant,
}
impl Default for Color {
    fn default() -> Self {
        Self {
            color_variant: ColorVariant::Rgba([1.0, 0.0, 0.0, 1.0])
        }
    }
}
impl Color {
    pub fn hlca(h:f64, l:f64, c:f64, a:f64) -> Self {
        Self {color_variant: ColorVariant::Hlca([h,l,c,a])}
    }
    pub fn rgba(r:f64, g:f64, b:f64, a:f64) -> Self {
        Self {color_variant: ColorVariant::Rgba([r,g,b,a])}
    }
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
#[pax_type]
pub enum ColorVariant {
    Hlca([f64; 4]),
    Rgba([f64; 4]),
}

#[pax_type]
pub use pax::api::Size;
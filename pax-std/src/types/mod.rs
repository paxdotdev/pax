use pax::*;
use pax::api::{PropertyInstance, PropertyLiteral, Interpolatable};


#[pax_type]
#[derive(Default, Clone)]
pub struct Stroke {
    pub color: Box<dyn PropertyInstance<Color>>,
    pub width: Box<dyn PropertyInstance<f64>>,
}


#[derive(Clone)]
#[pax_type]
pub struct SpreadCellProperties {
    pub x_px: f64,
    pub y_px: f64,
    pub width_px: f64,
    pub height_px: f64,
}

/// Simple way to represent whether a spread should render
/// vertically or horizontally
#[pax_type]
#[derive(Clone)]
pub enum SpreadDirection {
    Vertical,
    Horizontal,
}

impl Default for SpreadDirection {
    fn default() -> Self {
        SpreadDirection::Horizontal
    }
}

impl Interpolatable for SpreadDirection {}


#[pax_type]
#[derive(Clone)]
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

impl Interpolatable for Color {
    //TODO: Colors can be meaningfully interpolated.
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
#[derive(Clone)]
pub enum ColorVariant {
    Hlca([f64; 4]),
    Rgba([f64; 4]),
}

#[pax_type]
pub use pax::api::Size;
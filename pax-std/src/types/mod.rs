use pax::*;
use pax::api::{PropertyInstance, PropertyLiteral, Interpolatable, SizePixels};

#[cfg(feature = "parser")]
use pax_compiler::reflection::PathQualifiable;

#[derive(Clone)]
#[pax_type]
pub struct Stroke {
    pub color: Box<dyn PropertyInstance<Color>>,
    pub width: Box<dyn PropertyInstance<SizePixels>>,
}
impl Default for Stroke {
    fn default() -> Self {
        Self {
            color: Box::new(PropertyLiteral::new(Default::default())),
            width: Box::new(PropertyLiteral::new(SizePixels(0.0))),
        }
    }
}

#[derive(Default, Clone)]
#[pax_type]
pub struct Text {
    pub content: Box<dyn PropertyInstance<String>>,
}

#[derive(Clone)]
#[pax_type]
pub struct StackerCellProperties {
    pub x_px: f64,
    pub y_px: f64,
    pub width_px: f64,
    pub height_px: f64,
}

#[derive(Clone)]
#[pax_type]
pub enum StackerDirection {
    Vertical,
    Horizontal,
}

impl Default for StackerDirection {
    fn default() -> Self {
        StackerDirection::Horizontal
    }
}
impl Interpolatable for StackerDirection {}


#[derive(Clone)]
#[pax_type]
pub struct Font {
    pub family: Box<dyn pax::api::PropertyInstance<String>>,
    pub variant: Box<dyn pax::api::PropertyInstance<String>>,
    pub size: Box<dyn pax::api::PropertyInstance<SizePixels>>,
}
impl Into<FontPatch> for &Font {
    fn into(self) -> FontPatch {
        FontPatch {
             family: Some(self.family.get().clone()),
             variant: Some(self.variant.get().clone()),
             size: Some(self.size.get().0),
        }
    }
}

impl PartialEq<FontPatch> for Font {
    fn eq(&self, other: &FontPatch) -> bool {
        matches!(&other.family, Some(family) if family.eq(self.family.get()))
            && matches!(&other.variant, Some(variant) if variant.eq(self.variant.get()))
            && matches!(&other.size, Some(size) if size.eq(self.size.get()))
    }
}
impl Default for Font {
    fn default() -> Self {
        Self {
            family: Box::new(PropertyLiteral::new("Courier New".to_string())),
            variant: Box::new(PropertyLiteral::new("Regular".to_string())),
            size: Box::new(PropertyLiteral::new(SizePixels(14.0))),
        }
    }
}
impl Interpolatable for Font {}

#[derive(Clone)]
#[pax_type]
pub struct Color{
    pub color_variant: ColorVariant,
}
impl Default for Color {
    fn default() -> Self {
        Self {
            color_variant: ColorVariant::Rgba([0.0, 0.0, 1.0, 1.0])
        }
    }
}
impl Into<ColorVariantMessage> for &Color {
    fn into(self) -> ColorVariantMessage {
        match self.color_variant {
            ColorVariant::Hlca(channels) => {
                ColorVariantMessage::Hlca(channels)
            },
            ColorVariant::Rgba(channels) => {
                ColorVariantMessage::Rgba(channels)
            }
        }
    }
}
impl PartialEq<ColorVariantMessage> for Color {
    fn eq(&self, other: &ColorVariantMessage) -> bool {
        match self.color_variant {
            ColorVariant::Hlca(channels_self) => {
                if matches!(other, ColorVariantMessage::Hlca(channels_other) if channels_other.eq(&channels_self)) {
                    return true;
                }
            },
            ColorVariant::Rgba(channels_self) => {
                if matches!(other, ColorVariantMessage::Rgba(channels_other) if channels_other.eq(&channels_self)) {
                    return true;
                }
            }
        }
        false
    }
}

impl Interpolatable for Color {
    //TODO: Colors can be meaningfully interpolated, thus
    //      we should include interpolation logic here
    //      (Note that piet::Color offers a `to_rgba` method, probably
    //      useful to establish a common color space)
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

#[derive(Clone)]
#[pax_type]
pub enum ColorVariant {
    Hlca([f64; 4]),
    Rgba([f64; 4]),
}

pub use pax::api::Size;

use pax_message::{ColorVariantMessage, FontPatch};
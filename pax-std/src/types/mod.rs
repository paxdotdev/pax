use kurbo::{Point};
use pax::*;
use pax::api::{PropertyInstance, PropertyLiteral, Interpolatable, SizePixels};
use pax::api::numeric::Numeric;
pub use pax::api::Size;
use pax_message::{ColorVariantMessage, FontPatch};
use crate::primitives::Path;

#[allow(unused_imports)]
#[cfg(feature = "parser")]
use pax_message::reflection::PathQualifiable;

#[pax_type]
pub struct Stroke {
    pub color: Box<dyn PropertyInstance<Color>>,
    pub width: Box<dyn PropertyInstance<SizePixels>>,
}

#[pax_type]
pub struct Text {
    pub content: Box<dyn PropertyInstance<String>>,
}

#[pax_type]
pub struct StackerCell {
    pub x_px: f64,
    pub y_px: f64,
    pub width_px: f64,
    pub height_px: f64,
}

#[pax_type]
pub enum StackerDirection {
    Vertical,
    #[default]
    Horizontal,
}

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
             size: Some(self.size.get().0.get_as_float()),
        }
    }
}

impl PartialEq<FontPatch> for Font {
    fn eq(&self, other: &FontPatch) -> bool {
        matches!(&other.family, Some(family) if family.eq(self.family.get()))
            && matches!(&other.variant, Some(variant) if variant.eq(self.variant.get()))
            && matches!(&other.size, Some(size) if size.eq(&self.size.get().0.get_as_float()))
    }
}
impl Default for Font {
    fn default() -> Self {
        Self {
            family: Box::new(PropertyLiteral::new("Courier New".to_string())),
            variant: Box::new(PropertyLiteral::new("Regular".to_string())),
            size: Box::new(PropertyLiteral::new(SizePixels(Numeric::from(14.0)))),
        }
    }
}

#[pax_type(Default)]
pub struct Color{
    pub color_variant: ColorVariant,
}
impl Color {
    pub fn hlca(h:Numeric, l:Numeric, c:Numeric, a:Numeric) -> Self {
        Self {color_variant: ColorVariant::Hlca([h.get_as_float(),l.get_as_float(),c.get_as_float(),a.get_as_float()])}
    }
    pub fn hlc(h:Numeric, l:Numeric, c:Numeric) -> Self {
        Self {color_variant: ColorVariant::Hlc([h.get_as_float(),l.get_as_float(),c.get_as_float()])}
    }
    pub fn rgba(r:Numeric, g:Numeric, b:Numeric, a:Numeric) -> Self {
        Self {color_variant: ColorVariant::Rgba([r.get_as_float(),g.get_as_float(),b.get_as_float(),a.get_as_float()])}
    }
    pub fn rgb(r:Numeric, g:Numeric, b:Numeric) -> Self {
        Self {color_variant: ColorVariant::Rgb([r.get_as_float(),g.get_as_float(),b.get_as_float()])}
    }
    pub fn to_piet_color(&self) -> piet::Color {
        match self.color_variant {
            ColorVariant::Hlca(slice) => {
                piet::Color::hlca(slice[0], slice[1], slice[2], slice[3])
            },
            ColorVariant::Hlc(slice) => {
                piet::Color::hlc(slice[0], slice[1], slice[2])
            },
            ColorVariant::Rgba(slice) => {
                piet::Color::rgba(slice[0], slice[1], slice[2], slice[3])
            },
            ColorVariant::Rgb(slice) => {
                piet::Color::rgb(slice[0], slice[1], slice[2])
            }
        }
    }
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
            },
            ColorVariant::Rgb(channels) => {
                ColorVariantMessage::Rgb(channels)
            },
            ColorVariant::Hlc(channels) => {
                ColorVariantMessage::Hlc(channels)
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
            ColorVariant::Hlc(channels_self) => {
                if matches!(other, ColorVariantMessage::Hlc(channels_other) if channels_other.eq(&channels_self)) {
                    return true;
                }
            },
            ColorVariant::Rgba(channels_self) => {
                if matches!(other, ColorVariantMessage::Rgba(channels_other) if channels_other.eq(&channels_self)) {
                    return true;
                }
            },
            ColorVariant::Rgb(channels_self) => {
                if matches!(other, ColorVariantMessage::Rgb(channels_other) if channels_other.eq(&channels_self)) {
                    return true;
                }
            }
        }
        false
    }
}


#[pax_type]
pub enum ColorVariant {
    Hlca([f64; 4]),
    Hlc([f64; 3]),
    Rgba([f64; 4]),
    Rgb([f64; 3]),
}

#[pax_type]
pub enum PathSegment {
    #[default]
    Empty,
    LineSegment(LineSegmentData),
    CurveSegment(CurveSegmentData),
}

#[pax_type]
pub struct LineSegmentData {
    pub start : Point,
    pub end : Point,
}

#[pax_type]
pub struct CurveSegmentData {
    pub start : Point,
    pub handle : Point,
    pub end : Point,
}


impl Path {
    pub fn start() -> Vec<PathSegment> {
        let start : Vec<PathSegment> = Vec::new();
        start
    }
    pub fn line_to(mut path: Vec<PathSegment>, start: (f64, f64), end: (f64, f64)) -> Vec<PathSegment> {
        let line_seg_data: LineSegmentData = LineSegmentData {
            start: Point::from(start),
            end: Point::from(end),
        };

        path.push(PathSegment::LineSegment(line_seg_data));
        path
    }

    pub fn curve_to(mut path: Vec<PathSegment>, start: (f64, f64), handle: (f64, f64), end: (f64, f64)) -> Vec<PathSegment> {
        let curve_seg_data: CurveSegmentData = CurveSegmentData {
            start:  Point::from(start),
            handle:  Point::from(handle),
            end:  Point::from(end),
        };

        path.push(PathSegment::CurveSegment(curve_seg_data));
        path
    }
}
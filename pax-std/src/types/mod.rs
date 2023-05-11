use kurbo::{Point};
use pax::*;
use pax::api::{PropertyInstance, PropertyLiteral, Interpolatable, SizePixels};
use pax::api::numeric::Numeric;

#[allow(unused_imports)]
#[cfg(feature = "parser")]
use pax_message::reflection::PathQualifiable;

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
            width: Box::new(PropertyLiteral::new(SizePixels(Numeric::from(0.0)))),
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
pub struct StackerCell {
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
            family: Box::new(PropertyLiteral::new("Arial".to_string())),
            variant: Box::new(PropertyLiteral::new("Italic".to_string())),
            size: Box::new(PropertyLiteral::new(SizePixels(Numeric::from(50.0)))),
        }
    }
}

impl Interpolatable for Font {}

impl Font {
    pub fn get(family: String, variant: String, size: Numeric) -> Self {
        Self {
            family: Box::new(PropertyLiteral::new(family)),
            variant: Box::new(PropertyLiteral::new(variant)),
            size: Box::new(PropertyLiteral::new(SizePixels(size))),
        }
    }
}

#[derive(Clone)]
#[pax_type]
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

impl Interpolatable for Color {
    //FUTURE: Colors can be meaningfully interpolated, thus
    //      we should include interpolation logic here
    //      (Note that piet::Color offers a `to_rgba` method, probably
    //      useful to establish a common color space)
}

#[derive(Clone)]
#[pax_type]
pub enum ColorVariant {
    Hlca([f64; 4]),
    Hlc([f64; 3]),
    Rgba([f64; 4]),
    Rgb([f64; 3]),
}

#[derive(Clone, Default)]
#[pax_type]
pub enum TextAlignHorizontal {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Clone, Default)]
#[pax_type]
pub enum TextAlignVertical {
    #[default]
    Top,
    Center,
    Bottom,
}

#[derive(Clone, Default)]
#[pax_type]
pub enum BoundingBox {
    #[default]
    Fixed,
    Auto
}

impl Into<TextAlignHorizontalMessage> for &TextAlignHorizontal {
    fn into(self) -> TextAlignHorizontalMessage {
        match self {
            TextAlignHorizontal::Center => {TextAlignHorizontalMessage::Center}
            TextAlignHorizontal::Left => {TextAlignHorizontalMessage::Left}
            TextAlignHorizontal::Right => {TextAlignHorizontalMessage::Right}
        }
    }
}

impl PartialEq<TextAlignHorizontalMessage> for TextAlignHorizontal {
    fn eq(&self, other: &TextAlignHorizontalMessage) -> bool {
        match (self, other) {
            (TextAlignHorizontal::Center, TextAlignHorizontalMessage::Center) => true,
            (TextAlignHorizontal::Left, TextAlignHorizontalMessage::Left) => true,
            (TextAlignHorizontal::Right, TextAlignHorizontalMessage::Right) => true,
            _ => false,
        }
    }
}

pub fn opt_alignment_to_message(opt_alignment: &Option<TextAlignHorizontal>) -> Option<TextAlignHorizontalMessage> {
    opt_alignment.as_ref().map(|alignment| {
        match alignment {
            TextAlignHorizontal::Center => TextAlignHorizontalMessage::Center,
            TextAlignHorizontal::Left => TextAlignHorizontalMessage::Left,
            TextAlignHorizontal::Right => TextAlignHorizontalMessage::Right,
        }
    })
}

pub fn opt_alignment_eq_opt_msg(opt_alignment: &Option<TextAlignHorizontal>, opt_alignment_msg: &Option<TextAlignHorizontalMessage>) -> bool {
    match (opt_alignment, opt_alignment_msg) {
        (Some(alignment), Some(alignment_msg)) => alignment.eq(alignment_msg),
        (None, None) => true,
        _ => false,
    }
}

impl Into<TextAlignVerticalMessage> for &TextAlignVertical {
    fn into(self) -> TextAlignVerticalMessage {
        match self {
            TextAlignVertical::Top => TextAlignVerticalMessage::Top,
            TextAlignVertical::Center => TextAlignVerticalMessage::Center,
            TextAlignVertical::Bottom => TextAlignVerticalMessage::Bottom,
        }
    }
}

impl PartialEq<TextAlignVerticalMessage> for TextAlignVertical {
    fn eq(&self, other: &TextAlignVerticalMessage) -> bool {
        match (self, other) {
            (TextAlignVertical::Top, TextAlignVerticalMessage::Top) => true,
            (TextAlignVertical::Center, TextAlignVerticalMessage::Center) => true,
            (TextAlignVertical::Bottom, TextAlignVerticalMessage::Bottom) => true,
            _ => false,
        }
    }
}


pub use pax::api::Size;
use pax_message::{ColorVariantMessage, FontPatch, TextAlignHorizontalMessage, TextAlignVerticalMessage};
use crate::primitives::Path;


#[derive(Clone)]
#[pax_type]
pub enum PathSegment {
    LineSegment(LineSegmentData),
    CurveSegment(CurveSegmentData),
}

impl Interpolatable for PathSegment {}

#[derive(Clone)]
#[pax_type]
pub struct LineSegmentData {
    pub start : Point,
    pub end : Point,
}


#[derive(Clone)]
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
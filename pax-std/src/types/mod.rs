pub mod text;

use crate::primitives::Path;
use kurbo::{Point, RoundedRectRadii};
use pax_lang::api::numeric::Numeric;
pub use pax_lang::api::Size;
use pax_lang::api::{PropertyLiteral, SizePixels};
use pax_lang::*;
use pax_message::ColorVariantMessage;
use piet::UnitPoint;

#[derive(Pax)]
#[custom(Default)]
pub struct Stroke {
    pub color: Property<Color>,
    pub width: Property<SizePixels>,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            color: Default::default(),
            width: Box::new(PropertyLiteral::new(SizePixels(0.0.into()))),
        }
    }
}

#[derive(Pax)]
#[custom(Imports)]
pub struct StackerCell {
    pub x_px: f64,
    pub y_px: f64,
    pub width_px: f64,
    pub height_px: f64,
}

#[derive(Pax)]
#[custom(Imports)]
pub enum StackerDirection {
    Vertical,
    #[default]
    Horizontal,
}

#[derive(Pax)]
#[custom(Imports)]
pub enum SidebarDirection {
    Left,
    #[default]
    Right,
}

#[derive(Pax)]
#[custom(Default, Imports)]
pub enum Fill {
    Solid(Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
}

#[derive(Pax)]
#[custom(Default, Imports)]
pub struct LinearGradient {
    pub start: (Size, Size),
    pub end: (Size, Size),
    pub stops: Vec<GradientStop>,
}

#[derive(Pax)]
#[custom(Default, Imports)]
pub struct RadialGradient {
    pub end: (Size, Size),
    pub start: (Size, Size),
    pub radius: f64,
    pub stops: Vec<GradientStop>,
}

#[derive(Pax)]
#[custom(Imports)]
pub struct GradientStop {
    position: Size,
    color: Color,
}

impl GradientStop {
    pub fn get(color: Color, position: Size) -> GradientStop {
        GradientStop { position, color }
    }
}

impl Default for Fill {
    fn default() -> Self {
        Self::Solid(Color::default())
    }
}

impl Fill {
    pub fn to_unit_point((x, y): (Size, Size), (width, height): (f64, f64)) -> UnitPoint {
        let normalized_x = match x {
            Size::Pixels(val) => val.get_as_float() / width,
            Size::Percent(val) => val.get_as_float() / 100.0,
            Size::Combined(pix, per) => (pix.get_as_float() / width) + (per.get_as_float() / 100.0),
        };

        let normalized_y = match y {
            Size::Pixels(val) => val.get_as_float() / height,
            Size::Percent(val) => val.get_as_float() / 100.0,
            Size::Combined(pix, per) => (pix.get_as_float() / width) + (per.get_as_float() / 100.0),
        };
        UnitPoint::new(normalized_x, normalized_y)
    }

    pub fn to_piet_gradient_stops(stops: Vec<GradientStop>) -> Vec<piet::GradientStop> {
        let mut ret = Vec::new();
        for gradient_stop in stops {
            match gradient_stop.position {
                Size::Pixels(_) => {
                    panic!("Gradient stops must be specified in percentages");
                }
                Size::Percent(p) => {
                    ret.push(piet::GradientStop {
                        pos: (p.get_as_float() / 100.0) as f32,
                        color: gradient_stop.color.to_piet_color(),
                    });
                }
                Size::Combined(_, _) => {
                    panic!("Gradient stops must be specified in percentages");
                }
            }
        }
        ret
    }

    #[allow(non_snake_case)]
    pub fn linearGradient(
        start: (Size, Size),
        end: (Size, Size),
        stops: Vec<GradientStop>,
    ) -> Fill {
        Fill::LinearGradient(LinearGradient { start, end, stops })
    }
}

#[derive(Pax)]
#[custom(Default, Imports)]
pub struct Color {
    pub color_variant: ColorVariant,
}
impl Color {
    pub fn hlca(h: Numeric, l: Numeric, c: Numeric, a: Numeric) -> Self {
        Self {
            color_variant: ColorVariant::Hlca([
                h.get_as_float(),
                l.get_as_float(),
                c.get_as_float(),
                a.get_as_float(),
            ]),
        }
    }
    pub fn hlc(h: Numeric, l: Numeric, c: Numeric) -> Self {
        Self {
            color_variant: ColorVariant::Hlc([
                h.get_as_float(),
                l.get_as_float(),
                c.get_as_float(),
            ]),
        }
    }
    pub fn rgba(r: Numeric, g: Numeric, b: Numeric, a: Numeric) -> Self {
        Self {
            color_variant: ColorVariant::Rgba([
                r.get_as_float(),
                g.get_as_float(),
                b.get_as_float(),
                a.get_as_float(),
            ]),
        }
    }
    pub fn rgb(r: Numeric, g: Numeric, b: Numeric) -> Self {
        Self {
            color_variant: ColorVariant::Rgb([
                r.get_as_float(),
                g.get_as_float(),
                b.get_as_float(),
            ]),
        }
    }
    pub fn to_piet_color(&self) -> piet::Color {
        match self.color_variant {
            ColorVariant::Hlca(slice) => piet::Color::hlca(slice[0], slice[1], slice[2], slice[3]),
            ColorVariant::Hlc(slice) => piet::Color::hlc(slice[0], slice[1], slice[2]),
            ColorVariant::Rgba(slice) => piet::Color::rgba(slice[0], slice[1], slice[2], slice[3]),
            ColorVariant::Rgb(slice) => piet::Color::rgb(slice[0], slice[1], slice[2]),
        }
    }
}

impl Default for Color {
    fn default() -> Self {
        Self {
            color_variant: ColorVariant::Rgba([0.0, 0.0, 1.0, 1.0]),
        }
    }
}
impl Into<ColorVariantMessage> for &Color {
    fn into(self) -> ColorVariantMessage {
        match self.color_variant {
            ColorVariant::Hlca(channels) => ColorVariantMessage::Hlca(channels),
            ColorVariant::Rgba(channels) => ColorVariantMessage::Rgba(channels),
            ColorVariant::Rgb(channels) => ColorVariantMessage::Rgb(channels),
            ColorVariant::Hlc(channels) => ColorVariantMessage::Hlc(channels),
        }
    }
}
impl PartialEq<ColorVariantMessage> for Color {
    fn eq(&self, other: &ColorVariantMessage) -> bool {
        match self.color_variant {
            ColorVariant::Hlca(channels_self) => {
                if matches!(other, ColorVariantMessage::Hlca(channels_other) if channels_other.eq(&channels_self))
                {
                    return true;
                }
            }
            ColorVariant::Hlc(channels_self) => {
                if matches!(other, ColorVariantMessage::Hlc(channels_other) if channels_other.eq(&channels_self))
                {
                    return true;
                }
            }
            ColorVariant::Rgba(channels_self) => {
                if matches!(other, ColorVariantMessage::Rgba(channels_other) if channels_other.eq(&channels_self))
                {
                    return true;
                }
            }
            ColorVariant::Rgb(channels_self) => {
                if matches!(other, ColorVariantMessage::Rgb(channels_other) if channels_other.eq(&channels_self))
                {
                    return true;
                }
            }
        }
        false
    }
}

#[derive(Pax, Debug)]
#[custom(Default, Imports)]
pub enum ColorVariant {
    Hlca([f64; 4]),
    Hlc([f64; 3]),
    Rgba([f64; 4]),
    Rgb([f64; 3]),
}

impl Default for ColorVariant {
    fn default() -> Self {
        Self::Rgb([0.0, 0.0, 1.0])
    }
}

#[derive(Pax)]
#[custom(Imports)]
pub enum PathSegment {
    #[default]
    Empty,
    LineSegment(LineSegmentData),
    CurveSegment(CurveSegmentData),
}

#[derive(Pax)]
#[custom(Imports)]
pub struct LineSegmentData {
    pub start: Point,
    pub end: Point,
}

#[derive(Pax)]
#[custom(Imports)]
pub struct CurveSegmentData {
    pub start: Point,
    pub handle: Point,
    pub end: Point,
}

impl Path {
    pub fn start() -> Vec<PathSegment> {
        let start: Vec<PathSegment> = Vec::new();
        start
    }
    pub fn line_to(
        mut path: Vec<PathSegment>,
        start: (f64, f64),
        end: (f64, f64),
    ) -> Vec<PathSegment> {
        let line_seg_data: LineSegmentData = LineSegmentData {
            start: Point::from(start),
            end: Point::from(end),
        };

        path.push(PathSegment::LineSegment(line_seg_data));
        path
    }

    pub fn curve_to(
        mut path: Vec<PathSegment>,
        start: (f64, f64),
        handle: (f64, f64),
        end: (f64, f64),
    ) -> Vec<PathSegment> {
        let curve_seg_data: CurveSegmentData = CurveSegmentData {
            start: Point::from(start),
            handle: Point::from(handle),
            end: Point::from(end),
        };

        path.push(PathSegment::CurveSegment(curve_seg_data));
        path
    }
}

#[derive(Pax)]
#[custom(Imports)]
pub struct RectangleCornerRadii {
    pub top_left: Property<f64>,
    pub top_right: Property<f64>,
    pub bottom_right: Property<f64>,
    pub bottom_left: Property<f64>,
}

impl Into<RoundedRectRadii> for &RectangleCornerRadii {
    fn into(self) -> RoundedRectRadii {
        RoundedRectRadii::new(
            self.top_left.get().clone(),
            self.top_right.get().clone(),
            self.bottom_right.get().clone(),
            self.bottom_left.get().clone(),
        )
    }
}

impl RectangleCornerRadii {
    pub fn radii(
        top_left: Numeric,
        top_right: Numeric,
        bottom_right: Numeric,
        bottom_left: Numeric,
    ) -> Self {
        RectangleCornerRadii {
            top_left: Box::new(PropertyLiteral::new(top_left.get_as_float())),
            top_right: Box::new(PropertyLiteral::new(top_right.get_as_float())),
            bottom_right: Box::new(PropertyLiteral::new(bottom_right.get_as_float())),
            bottom_left: Box::new(PropertyLiteral::new(bottom_left.get_as_float())),
        }
    }
}

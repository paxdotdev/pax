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

pub fn percent_to_float(percent: Size) -> f64 {
    match percent {
        Size::Pixels(_) => {
            panic!("Color percentages must be specified in percentages");
        }
        Size::Percent(p) => p.get_as_float() / 100.0,
        Size::Combined(_, _) => {
            panic!("Color percentages must be specified in percentages");
        }
    }
}

impl Color {
    pub fn hlca(h: Size, l: Size, c: Size, a: Size) -> Self {
        Self {
            color_variant: ColorVariant::Hlca([
                percent_to_float(h),
                percent_to_float(l),
                percent_to_float(c),
                percent_to_float(a),
            ]),
        }
    }
    pub fn hlc(h: Size, l: Size, c: Size) -> Self {
        Self {
            color_variant: ColorVariant::Hlc([
                percent_to_float(h),
                percent_to_float(l),
                percent_to_float(c),
            ]),
        }
    }
    pub fn rgba(r: Size, g: Size, b: Size, a: Size) -> Self {
        Self {
            color_variant: ColorVariant::Rgba([
                percent_to_float(r),
                percent_to_float(g),
                percent_to_float(b),
                percent_to_float(a),
            ]),
        }
    }
    pub fn rgb(r: Size, g: Size, b: Size) -> Self {
        Self {
            color_variant: ColorVariant::Rgb([
                percent_to_float(r),
                percent_to_float(g),
                percent_to_float(b),
            ]),
        }
    }

    pub fn shade(color: Color, shade: Numeric) -> Self {
        let shade = shade.get_as_float().clamp(0.0, 1000.0) / 1000.0;
        let (r, g, b, a) = color.to_piet_color().as_rgba();
        let r = ((r * 255.0) * shade) / 255.0;
        let g = ((g * 255.0) * shade) / 255.0;
        let b = ((b * 255.0) * shade) / 255.0;
        pax_runtime_api::log(format!("{r} {g} {b} {a}").as_str());
        Self {
            color_variant: ColorVariant::Rgba([r, g, b, a]),
        }
    }

    pub fn slate() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([100.0 / 255.0, 116.0 / 255.0, 139.0 / 255.0]),
        }
    }
    pub fn gray() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([107.0 / 255.0, 114.0 / 255.0, 128.0 / 255.0]),
        }
    }
    pub fn zinc() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([113.0 / 255.0, 113.0 / 255.0, 122.0 / 255.0]),
        }
    }
    pub fn neutral() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([115.0 / 255.0, 115.0 / 255.0, 115.0 / 255.0]),
        }
    }
    pub fn stone() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([120.0 / 255.0, 113.0 / 255.0, 108.0 / 255.0]),
        }
    }
    pub fn red() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([234.0 / 255.0, 68.0 / 255.0, 68.0 / 255.0]),
        }
    }
    pub fn orange() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([249.0 / 255.0, 115.0 / 255.0, 22.0 / 255.0]),
        }
    }
    pub fn amber() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([245.0 / 255.0, 158.0 / 255.0, 11.0 / 255.0]),
        }
    }
    pub fn yellow() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([234.0 / 255.0, 179.0 / 255.0, 8.0 / 255.0]),
        }
    }
    pub fn lime() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([132.0 / 255.0, 204.0 / 255.0, 22.0 / 255.0]),
        }
    }
    pub fn green() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([34.0 / 255.0, 197.0 / 255.0, 94.0 / 255.0]),
        }
    }
    pub fn emerald() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([16.0 / 255.0, 185.0 / 255.0, 129.0 / 255.0]),
        }
    }
    pub fn teal() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([20.0 / 255.0, 184.0 / 255.0, 166.0 / 255.0]),
        }
    }
    pub fn cyan() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([6.0 / 255.0, 182.0 / 255.0, 212.0 / 255.0]),
        }
    }
    pub fn sky() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([14.0 / 255.0, 165.0 / 255.0, 233.0 / 255.0]),
        }
    }
    pub fn blue() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([59.0 / 255.0, 130.0 / 255.0, 246.0 / 255.0]),
        }
    }
    pub fn indigo() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([99.0 / 255.0, 102.0 / 255.0, 241.0 / 255.0]),
        }
    }
    pub fn violet() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([132.0 / 255.0, 96.0 / 255.0, 246.0 / 255.0]),
        }
    }
    pub fn purple() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([168.0 / 255.0, 85.0 / 255.0, 247.0 / 255.0]),
        }
    }
    pub fn fuchsia() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([217.0 / 255.0, 70.0 / 255.0, 239.0 / 255.0]),
        }
    }
    pub fn pink() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([236.0 / 255.0, 72.0 / 255.0, 153.0 / 255.0]),
        }
    }
    pub fn rose() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([244.0 / 255.0, 63.0 / 255.0, 94.0 / 255.0]),
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
            *self.top_left.get(),
            *self.top_right.get(),
            *self.bottom_right.get(),
            *self.bottom_left.get(),
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

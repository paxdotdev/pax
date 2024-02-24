pub mod text;

use crate::primitives::Path;
pub use kurbo::RoundedRectRadii;
use pax_engine::api::Numeric;
use pax_engine::api::PropertyLiteral;
pub use pax_engine::api::Size;
use pax_engine::*;
use pax_message::ColorMessage;
use piet::UnitPoint;
use pax_runtime::api::Rotation;

#[pax]
#[custom(Default)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct Stroke {
    pub color: Property<Color>,
    pub width: Property<Size>,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            color: Default::default(),
            width: Box::new(PropertyLiteral::new(Size::Pixels(0.0.into()))),
        }
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[pax]
pub struct StackerCell {
    pub x_px: f64,
    pub y_px: f64,
    pub width_px: f64,
    pub height_px: f64,
}

#[pax]
pub enum StackerDirection {
    Vertical,
    #[default]
    Horizontal,
}

#[pax]
pub enum SidebarDirection {
    Left,
    #[default]
    Right,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[pax]
#[custom(Default)]
pub enum Fill {
    Solid(Color),
    LinearGradient(LinearGradient),
    RadialGradient(RadialGradient),
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[pax]
pub struct LinearGradient {
    pub start: (Size, Size),
    pub end: (Size, Size),
    pub stops: Vec<GradientStop>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[pax]
pub struct RadialGradient {
    pub end: (Size, Size),
    pub start: (Size, Size),
    pub radius: f64,
    pub stops: Vec<GradientStop>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[pax]
pub struct GradientStop {
    pub position: Size,
    pub color: Color,
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
            Size::Pixels(val) => val.to_float() / width,
            Size::Percent(val) => val.to_float() / 100.0,
            Size::Combined(pix, per) => (pix.to_float() / width) + (per.to_float() / 100.0),
        };

        let normalized_y = match y {
            Size::Pixels(val) => val.to_float() / height,
            Size::Percent(val) => val.to_float() / 100.0,
            Size::Combined(pix, per) => (pix.to_float() / width) + (per.to_float() / 100.0),
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
                        pos: (p.to_float() / 100.0) as f32,
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

/// Raw Percent type, which we use for serialization and dynamic traversal.  At the time
/// of authoring, this type is not used directly at runtime, but is intended for `into` coercion
/// into downstream types, e.g. ColorChannel, Rotation, and Size.  This allows us to be "dumb"
/// about how we parse `%`, and allow the context in which it is used to pull forward a specific
/// type through `into` inference.
pub struct Percent(Numeric);

impl Into<ColorChannel> for Percent {
    fn into(self) -> ColorChannel {
        ColorChannel::Percent(self.0)
    }
}

impl Into<Size> for Percent {
    fn into(self) -> Size {
        Size::Percent(self.0)
    }
}

impl Into<Rotation> for Percent {
    fn into(self) -> Rotation {
        Rotation::Percent(self.0)
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[pax]
#[custom(Default)]
pub enum ColorChannel {
    /// [0,255]
    Int(Numeric),
    /// [0.0, 100.0]
    Percent(Numeric),
}

impl Default for ColorChannel {
    fn default() -> Self {
        Self::Percent(50.0.into())
    }
}

impl ColorChannel {
    ///Normalizes this ColorChannel as a float [0.0, 1.0]
    pub fn to_float_0_1(&self) -> f64 {
        match self {
            Self::Percent(per) => {
                assert!(per.to_float() >= -0.000001 && per.to_float() <= 100.000001, "");
                (per.to_float() / 100.0).clamp(0_f64, 1_f64)
            },
            Self::Int(zero_to_255) => {
                assert!(zero_to_255.to_int() >= 0 && zero_to_255.to_int() <= 255, "Integer color channel values must be between 0 and 255");
                let f_zero : f64 = (*zero_to_255).into();
                f_zero / 255.0_f64.clamp(0_f64, 1_f64)
            }
        }
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[allow(non_camel_case_types)]
#[pax]
pub enum Color {

    /// Models a color in the RGB space, with an alpha channel of 100%
    rgb(ColorChannel, ColorChannel, ColorChannel),
    /// Models a color in the RGBA space
    rgba(ColorChannel, ColorChannel, ColorChannel, ColorChannel),

    /// Models a color in the HSL space.  Note the use of Rotation rather than a ColorChannel for hue.
    hsl(Rotation, ColorChannel, ColorChannel),
    hsla(Rotation, ColorChannel, ColorChannel, ColorChannel),

    #[default]
    red
    //TODO: with `red` as a prototype, add Tailwind-inspired pseudo-constants here
}
impl Color {

    //TODO: fill out Tailwind-style tint api
    //pub fn tint(tint_offset_amount) -> Self {...}

    pub fn to_piet_color(&self) -> piet::Color {
        let rgba = self.to_rgba();
        piet::Color::rgba(rgba[0], rgba[1], rgba[2], rgba[3])
    }

    pub fn to_rgba(&self) -> [f64; 4] {
        match self {
            Self::hsla(h,s,l,a) => {
                let rgb = hsl_to_rgb(h.to_float_0_1(),s.to_float_0_1(),l.to_float_0_1());
                [rgb[0], rgb[1], rgb[2], a.to_float_0_1()]
            },
            Self::hsl(h,s,l) => {
                let rgb = hsl_to_rgb(h.to_float_0_1(),s.to_float_0_1(),l.to_float_0_1());
                [rgb[0], rgb[1], rgb[2], 1.0]

            },
            Self::rgba(r,g,b,a) => [r.to_float_0_1(),g.to_float_0_1(),b.to_float_0_1(),a.to_float_0_1()],
            Self::rgb(r,g,b) => [r.to_float_0_1(),g.to_float_0_1(),b.to_float_0_1(),1.0],
            _ => {
                unimplemented!("Unsupported color variant lacks conversion logic to RGB")
            }
        }
    }
}

fn hsl_to_rgb(h: f64, s: f64, l: f64) -> [f64; 3] {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((h * 6.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;

    let (r, g, b) = if h < 1.0/6.0 {
        (c, x, 0.0)
    } else if h < 2.0/6.0 {
        (x, c, 0.0)
    } else if h < 3.0/6.0 {
        (0.0, c, x)
    } else if h < 4.0/6.0 {
        (0.0, x, c)
    } else if h < 5.0/6.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };

    [(r + m), (g + m), (b + m)]
}

impl Into<ColorMessage> for &Color {
    fn into(self) -> ColorMessage {
        let rgba = self.to_rgba();
        ColorMessage::Rgba(rgba)
    }
}
impl PartialEq<ColorMessage> for Color {
    fn eq(&self, other: &ColorMessage) -> bool {
        let self_rgba = self.to_rgba();

        match other {
            ColorMessage::Rgb(other_rgba) => {
                self_rgba[0] == other_rgba[0] &&
                    self_rgba[1] == other_rgba[1] &&
                    self_rgba[2] == other_rgba[2] &&
                    self_rgba[3] == 1.0
            },
            ColorMessage::Rgba(other_rgba) => {
                self_rgba[0] == other_rgba[0] &&
                    self_rgba[1] == other_rgba[1] &&
                    self_rgba[2] == other_rgba[2] &&
                    self_rgba[3] == other_rgba[3]
            },
        }
    }
}

#[pax]
#[cfg_attr(debug_assertions, derive(Debug))]
pub enum PathSegment {
    #[default]
    Empty,
    LineSegment(LineSegmentData),
    CurveSegment(CurveSegmentData),
}

impl PathSegment {
    pub fn line(line: LineSegmentData) -> Self {
        Self::LineSegment(line)
    }
}

#[pax]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct LineSegmentData {
    pub start: Point,
    pub end: Point,
}

impl LineSegmentData {
    pub fn new(p1: Point, p2: Point) -> Self {
        Self { start: p1, end: p2 }
    }
}

#[pax]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct CurveSegmentData {
    pub start: Point,
    pub handle: Point,
    pub end: Point,
}

#[pax]
#[derive(Copy)]
#[cfg_attr(debug_assertions, derive(Debug))]
pub struct Point {
    pub x: Size,
    pub y: Size,
}

impl Point {
    pub fn new(x: Size, y: Size) -> Self {
        Self { x, y }
    }

    pub fn to_kurbo_point(self, bounds: (f64, f64)) -> kurbo::Point {
        let x = self.x.evaluate(bounds, api::Axis::X);
        let y = self.y.evaluate(bounds, api::Axis::Y);
        kurbo::Point { x, y }
    }
}

impl Path {
    pub fn start() -> Vec<PathSegment> {
        let start: Vec<PathSegment> = Vec::new();
        start
    }
    pub fn line_to(mut path: Vec<PathSegment>, start: Point, end: Point) -> Vec<PathSegment> {
        let line_seg_data: LineSegmentData = LineSegmentData { start, end };

        path.push(PathSegment::LineSegment(line_seg_data));
        path
    }

    pub fn curve_to(
        mut path: Vec<PathSegment>,
        start: Point,
        handle: Point,
        end: Point,
    ) -> Vec<PathSegment> {
        let curve_seg_data: CurveSegmentData = CurveSegmentData { start, handle, end };

        path.push(PathSegment::CurveSegment(curve_seg_data));
        path
    }
}

#[pax]
#[cfg_attr(debug_assertions, derive(Debug))]
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
            top_left: Box::new(PropertyLiteral::new(top_left.to_float())),
            top_right: Box::new(PropertyLiteral::new(top_right.to_float())),
            bottom_right: Box::new(PropertyLiteral::new(bottom_right.to_float())),
            bottom_left: Box::new(PropertyLiteral::new(bottom_left.to_float())),
        }
    }
}

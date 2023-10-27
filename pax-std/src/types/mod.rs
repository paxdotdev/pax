pub mod text;

use crate::primitives::Path;
use kurbo::{Point, RoundedRectRadii};
use pax_lang::api::numeric::Numeric;
pub use pax_lang::api::Size;
use pax_lang::api::{PropertyLiteral, SizePixels};
use pax_lang::*;
use pax_message::ColorVariantMessage;
use piet::UnitPoint;

const RGB: f64 = 255.0;

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

impl From<Color> for Fill {
    fn from(color: Color) -> Self {
        Self::Solid(color)
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

    /// Converts an RGB color to its HSL representation.
    ///
    /// This function takes in RGB values (each between 0 and 255) and returns the corresponding HSL values(between 0.0 and 1.0)
    fn rgb_to_hsl(red: f64, green: f64, blue: f64) -> (f64, f64, f64) {
        let normalized_red = red / 255.0;
        let normalized_green = green / 255.0;
        let normalized_blue = blue / 255.0;

        let color_max = normalized_red.max(normalized_green.max(normalized_blue));
        let color_min = normalized_red.min(normalized_green.min(normalized_blue));

        let mut hue;
        let saturation;
        let lightness = (color_max + color_min) / 2.0;

        if color_max == color_min {
            hue = 0.0;
            saturation = 0.0;
        } else {
            let delta = color_max - color_min;
            saturation = if lightness > 0.5 {
                delta / (2.0 - color_max - color_min)
            } else {
                delta / (color_max + color_min)
            };

            hue = match color_max {
                _ if color_max == normalized_red => {
                    (normalized_green - normalized_blue) / delta
                        + (if normalized_green < normalized_blue {
                            6.0
                        } else {
                            0.0
                        })
                }
                _ if color_max == normalized_green => {
                    (normalized_blue - normalized_red) / delta + 2.0
                }
                _ => (normalized_red - normalized_green) / delta + 4.0,
            };

            hue /= 6.0;
        }

        (hue, saturation, lightness)
    }

    /// Converts an HSL(values between 0.0 and 1.0) color to its RGB representation.
    ///
    /// Returns a tuple of `(red, green, blue)` where each value is between 0.0 and 255.0.
    fn hsl_to_rgb(hue: f64, saturation: f64, lightness: f64) -> (f64, f64, f64) {
        if saturation == 0.0 {
            return (lightness * 255.0, lightness * 255.0, lightness * 255.0);
        }

        let temp_q = if lightness < 0.5 {
            lightness * (1.0 + saturation)
        } else {
            lightness + saturation - lightness * saturation
        };

        let temp_p = 2.0 * lightness - temp_q;

        let convert = |temp_hue: f64| -> f64 {
            let mut temp_hue = temp_hue;
            if temp_hue < 0.0 {
                temp_hue += 1.0;
            }
            if temp_hue > 1.0 {
                temp_hue -= 1.0;
            }
            if temp_hue < 1.0 / 6.0 {
                return temp_p + (temp_q - temp_p) * 6.0 * temp_hue;
            }
            if temp_hue < 1.0 / 2.0 {
                return temp_q;
            }
            if temp_hue < 2.0 / 3.0 {
                return temp_p + (temp_q - temp_p) * (2.0 / 3.0 - temp_hue) * 6.0;
            }
            return temp_p;
        };
        let red = (convert(hue + 1.0 / 3.0) * 255.0).min(255.0).max(0.0);
        let green = (convert(hue) * 255.0).min(255.0).max(0.0);
        let blue = (convert(hue - 1.0 / 3.0) * 255.0).min(255.0).max(0.0);

        (red, green, blue)
    }

    /// Shades a given color by a specified value.
    ///
    /// * `color` - A `Color` struct representing the color to be shaded.
    /// * `shade` - A `Numeric` struct representing the value by which to shade the color of.
    ///             Accepted values are between 0 and 1000, higher values result in a darker color.
    ///             Most neutral color will be achieved at 500.
    pub fn shade(color: Color, shade: Numeric) -> Self {
        let shade_multp = (-0.002 * shade.get_as_float()) + 2.0;
        let (r, g, b, _) = color.to_piet_color().as_rgba();
        let (h, s, l) = Self::rgb_to_hsl(r * 255.0, g * 255.0, b * 255.0);
        let (r, g, b) = Self::hsl_to_rgb(h, s, l * shade_multp);

        Self {
            color_variant: ColorVariant::Rgb([r / RGB, g / RGB, b / RGB]),
        }
    }

    pub fn slate() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([100.0 / RGB, 116.0 / RGB, 139.0 / RGB]),
        }
    }
    pub fn gray() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([107.0 / RGB, 114.0 / RGB, 128.0 / RGB]),
        }
    }
    pub fn zinc() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([113.0 / RGB, 113.0 / RGB, 122.0 / RGB]),
        }
    }
    pub fn neutral() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([115.0 / RGB, 115.0 / RGB, 115.0 / RGB]),
        }
    }
    pub fn stone() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([120.0 / RGB, 113.0 / RGB, 108.0 / RGB]),
        }
    }
    pub fn red() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([234.0 / RGB, 68.0 / RGB, 68.0 / RGB]),
        }
    }
    pub fn orange() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([249.0 / RGB, 115.0 / RGB, 22.0 / RGB]),
        }
    }
    pub fn amber() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([245.0 / RGB, 158.0 / RGB, 11.0 / RGB]),
        }
    }
    pub fn yellow() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([234.0 / RGB, 179.0 / RGB, 8.0 / RGB]),
        }
    }
    pub fn lime() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([132.0 / RGB, 204.0 / RGB, 22.0 / RGB]),
        }
    }
    pub fn green() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([34.0 / RGB, 197.0 / RGB, 94.0 / RGB]),
        }
    }
    pub fn emerald() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([16.0 / RGB, 185.0 / RGB, 129.0 / RGB]),
        }
    }
    pub fn teal() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([20.0 / RGB, 184.0 / RGB, 166.0 / RGB]),
        }
    }
    pub fn cyan() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([6.0 / RGB, 182.0 / RGB, 212.0 / RGB]),
        }
    }
    pub fn sky() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([14.0 / RGB, 165.0 / RGB, 233.0 / RGB]),
        }
    }
    pub fn blue() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([59.0 / RGB, 130.0 / RGB, 246.0 / RGB]),
        }
    }
    pub fn indigo() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([99.0 / RGB, 102.0 / RGB, 241.0 / RGB]),
        }
    }
    pub fn violet() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([132.0 / RGB, 96.0 / RGB, 246.0 / RGB]),
        }
    }
    pub fn purple() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([168.0 / RGB, 85.0 / RGB, 247.0 / RGB]),
        }
    }
    pub fn fuchsia() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([217.0 / RGB, 70.0 / RGB, 239.0 / RGB]),
        }
    }
    pub fn pink() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([236.0 / RGB, 72.0 / RGB, 153.0 / RGB]),
        }
    }
    pub fn rose() -> Self {
        Self {
            color_variant: ColorVariant::Rgb([244.0 / RGB, 63.0 / RGB, 94.0 / RGB]),
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

//! Vector drawing primitives: paths, fills, strokes, caps, and gradients.

use super::*;

pub mod stroke_utils;

/// Describes a single element of a vector path, such as a line, a point, or curve segment.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(crate = "crate::serde")]
pub enum PathElement {
    /// No-op path element.
    #[default]
    Empty,
    /// Moves the current point to the provided coordinate.
    Point(Size, Size),
    /// Draws a straight line to the following `Point`.
    Line,
    /// Draws a quadratic Bézier segment with one control point, ending at the following `Point`.
    Quadratic(Size, Size),
    /// Draws a cubic Bézier segment with two control points, ending at the following `Point`.
    Cubic(Size, Size, Size, Size),
    /// Closes the current contour.
    Close,
}

impl Interpolatable for PathElement {}
impl HelperFunctions for PathElement {}

/// Controls how an open stroke terminates at the exposed endpoints of a path.
///
/// `StrokeCap` affects primitives such as `Line` and open `Path` subpaths.
/// Closed geometry ignores cap style because it has no exposed endpoints.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(crate = "crate::serde")]
pub enum StrokeCap {
    /// Ends exactly at the path endpoint without extending past it.
    #[default]
    Butt,
    /// Adds a semicircular cap whose radius is half the stroke width.
    Round,
    /// Adds a square cap that extends half the stroke width past the endpoint.
    Square,
}

impl Interpolatable for StrokeCap {}

/// Describes the outline drawn around vector geometry.
///
/// Pax currently renders strokes centered on the underlying path. For open
/// geometry, `cap` controls how the stroke terminates at the start and end of
/// the path.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "crate::serde")]
pub struct Stroke {
    /// The stroke color, including alpha.
    pub color: Property<Color>,
    /// The stroke width.
    ///
    /// The type is [`Size`] for consistency with the wider property system, but
    /// current vector renderers interpret this value in pixels.
    pub width: Property<Size>,
    /// The cap style used for exposed endpoints on open paths.
    pub cap: Property<StrokeCap>,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            color: Default::default(),
            width: Property::new(Size::Pixels(Numeric::F64(0.0))),
            cap: Property::new(StrokeCap::default()),
        }
    }
}

impl PartialEq for Stroke {
    fn eq(&self, other: &Self) -> bool {
        self.color.get() == other.color.get()
            && self.width.get() == other.width.get()
            && self.cap.get() == other.cap.get()
    }
}

impl Interpolatable for Stroke {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self {
            color: Property::new(self.color.get().interpolate(&other.color.get(), t)),
            width: Property::new(self.width.get().interpolate(&other.width.get(), t)),
            cap: Property::new(if t < 1.0 {
                self.cap.get()
            } else {
                other.cap.get()
            }),
        }
    }
}

/// Describes where to open new windows or tabs when navigating to a URL from a `Link` node.
pub enum NavigationTarget {
    /// Navigate in the current window or tab.
    Current,
    /// Navigate in a new window or tab.
    New,
}

/// Describes how to fill vector geometry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "crate::serde")]
pub enum Fill {
    /// A single solid color.
    Solid(Color),
    /// A linear gradient.
    LinearGradient(LinearGradient),
    /// A radial gradient.
    RadialGradient(RadialGradient),
}

impl Hash for Fill {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Fill::Solid(color) => {
                state.write_u8(0);
                color.hash(state);
            }
            Fill::LinearGradient(linear) => {
                state.write_u8(1);
                linear.start.hash(state);
                linear.end.hash(state);
                linear.stops.hash(state);
            }
            Fill::RadialGradient(radial) => {
                state.write_u8(2);
                radial.start.hash(state);
                radial.end.hash(state);
                radial.radius.to_bits().hash(state);
                radial.stops.hash(state);
            }
        }
    }
}

impl Hash for Stroke {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.width.get().hash(state);
        self.color.get().hash(state);
        self.cap.get().hash(state);
    }
}

impl Interpolatable for Fill {
    fn interpolate(&self, _other: &Self, _t: f64) -> Self {
        // TODO interpolation
        self.clone()
    }
}

/// Describes a linear gradient fill with a start and end point, and a list of color stops.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
#[serde(crate = "crate::serde")]
pub struct LinearGradient {
    /// Gradient start point in the primitive's local coordinate space.
    pub start: (Size, Size),
    /// Gradient end point in the primitive's local coordinate space.
    pub end: (Size, Size),
    /// Ordered color stops along the gradient.
    pub stops: Vec<GradientStop>,
}

/// Describes a radial gradient fill with a start and end point, a radius, and a list of color stops.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "crate::serde")]
pub struct RadialGradient {
    /// Outer radius endpoint in the primitive's local coordinate space.
    pub end: (Size, Size),
    /// Gradient center point in the primitive's local coordinate space.
    pub start: (Size, Size),
    /// Radial gradient radius.
    pub radius: f64,
    /// Ordered color stops along the gradient.
    pub stops: Vec<GradientStop>,
}

/// A color stop for a gradient fill, defined by a position (% or px) and a color.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Hash)]
#[serde(crate = "crate::serde")]
pub struct GradientStop {
    /// Stop position, conventionally expressed as a percentage along the gradient.
    pub position: Size,
    /// Color at this stop.
    pub color: Color,
}

impl GradientStop {
    /// Constructs a gradient stop at `position`.
    pub fn get(color: Color, position: Size) -> GradientStop {
        GradientStop { position, color }
    }

    /// Returns a copy of this stop with alpha multiplied by `factor`.
    pub fn with_alpha_factor(&self, factor: f64) -> GradientStop {
        GradientStop {
            position: self.position.clone(),
            color: self.color.with_alpha_factor(factor),
        }
    }
}

impl Default for Fill {
    fn default() -> Self {
        Self::Solid(Color::default())
    }
}

impl Fill {
    fn gradient_stop_position_0_1(position: &Size) -> Option<f64> {
        match position {
            Size::Percent(p) => Some((p.to_float() / 100.0).clamp(0.0, 1.0)),
            Size::Pixels(_) | Size::Combined(_, _) => None,
        }
    }

    fn integrated_gradient_alpha_0_1(stops: &[GradientStop]) -> f64 {
        if stops.is_empty() {
            return 0.0;
        }

        let mut positioned_stops = stops
            .iter()
            .filter_map(|stop| {
                Self::gradient_stop_position_0_1(&stop.position)
                    .map(|position| (position, stop.color.alpha_0_1()))
            })
            .collect::<Vec<_>>();

        if positioned_stops.is_empty() {
            return stops
                .iter()
                .map(|stop| stop.color.alpha_0_1())
                .fold(0.0, f64::max);
        }

        positioned_stops.sort_by(|lhs, rhs| lhs.0.total_cmp(&rhs.0));

        if positioned_stops
            .first()
            .map(|stop| stop.0 > 0.0)
            .unwrap_or(false)
        {
            let alpha = positioned_stops[0].1;
            positioned_stops.insert(0, (0.0, alpha));
        }

        if positioned_stops
            .last()
            .map(|stop| stop.0 < 1.0)
            .unwrap_or(false)
        {
            let alpha = positioned_stops.last().map(|stop| stop.1).unwrap_or(0.0);
            positioned_stops.push((1.0, alpha));
        }

        let mut integrated_alpha = 0.0;
        for window in positioned_stops.windows(2) {
            let [(start_pos, start_alpha), (end_pos, end_alpha)] = window else {
                continue;
            };
            let span = (end_pos - start_pos).clamp(0.0, 1.0);
            if span <= f64::EPSILON {
                continue;
            }
            integrated_alpha += (start_alpha + end_alpha) * 0.5 * span;
        }

        integrated_alpha.clamp(0.0, 1.0)
    }

    // Converts a Pax point into Piet's unit-gradient coordinate system.
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

    // Converts Pax gradient stops into Piet's gradient-stop representation.
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
    /// Constructs a linear gradient fill.
    pub fn linearGradient(
        start: (Size, Size),
        end: (Size, Size),
        stops: Vec<GradientStop>,
    ) -> Fill {
        Fill::LinearGradient(LinearGradient { start, end, stops })
    }

    /// Returns a copy of this fill with alpha multiplied by `factor`.
    pub fn with_alpha_factor(&self, factor: f64) -> Fill {
        match self {
            Fill::Solid(color) => Fill::Solid(color.with_alpha_factor(factor)),
            Fill::LinearGradient(gradient) => Fill::LinearGradient(LinearGradient {
                start: gradient.start.clone(),
                end: gradient.end.clone(),
                stops: gradient
                    .stops
                    .iter()
                    .map(|stop| stop.with_alpha_factor(factor))
                    .collect(),
            }),
            Fill::RadialGradient(gradient) => Fill::RadialGradient(RadialGradient {
                start: gradient.start.clone(),
                end: gradient.end.clone(),
                radius: gradient.radius,
                stops: gradient
                    .stops
                    .iter()
                    .map(|stop| stop.with_alpha_factor(factor))
                    .collect(),
            }),
        }
    }

    /// Returns the maximum alpha used by this fill.
    pub fn max_alpha_0_1(&self) -> f64 {
        match self {
            Fill::Solid(color) => color.alpha_0_1(),
            Fill::LinearGradient(gradient) => gradient
                .stops
                .iter()
                .map(|stop| stop.color.alpha_0_1())
                .fold(0.0, f64::max),
            Fill::RadialGradient(gradient) => gradient
                .stops
                .iter()
                .map(|stop| stop.color.alpha_0_1())
                .fold(0.0, f64::max),
        }
    }

    /// Estimates the alpha coverage contributed by this fill.
    pub fn coverage_alpha_0_1(&self) -> f64 {
        match self {
            Fill::Solid(color) => color.alpha_0_1(),
            Fill::LinearGradient(gradient) => Self::integrated_gradient_alpha_0_1(&gradient.stops),
            Fill::RadialGradient(gradient) => Self::integrated_gradient_alpha_0_1(&gradient.stops),
        }
    }
}

#[cfg(test)]
mod fill_coverage_tests {
    use super::{Color, ColorChannel, Fill, GradientStop, LinearGradient, Numeric, Size};

    fn rgba_alpha(alpha: u8) -> Color {
        Color::rgba(
            ColorChannel::Integer(0),
            ColorChannel::Integer(0),
            ColorChannel::Integer(0),
            ColorChannel::Integer(alpha),
        )
    }

    #[test]
    fn coverage_alpha_matches_solid_alpha() {
        let fill = Fill::Solid(rgba_alpha(128));
        assert!((fill.coverage_alpha_0_1() - (128.0 / 255.0)).abs() < 1e-6);
    }

    #[test]
    fn coverage_alpha_integrates_gradient_stops() {
        let fill = Fill::LinearGradient(LinearGradient {
            start: (
                Size::Percent(Numeric::F64(0.0)),
                Size::Percent(Numeric::F64(50.0)),
            ),
            end: (
                Size::Percent(Numeric::F64(100.0)),
                Size::Percent(Numeric::F64(50.0)),
            ),
            stops: vec![
                GradientStop::get(rgba_alpha(0), Size::Percent(Numeric::F64(0.0))),
                GradientStop::get(rgba_alpha(255), Size::Percent(Numeric::F64(50.0))),
                GradientStop::get(rgba_alpha(0), Size::Percent(Numeric::F64(100.0))),
            ],
        });
        assert!((fill.coverage_alpha_0_1() - 0.5).abs() < 1e-6);
    }
}

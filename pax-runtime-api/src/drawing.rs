//! Vector drawing primitives: paths, fills, strokes, caps, and gradients.

use super::*;

pub mod path_smoothing;
pub mod path_trim;
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

/// Controls optional curve smoothing for path geometry before tessellation.
///
/// `PathSmoothing` is intended for authored or imported paths whose source data
/// approximates curves with many short line segments, such as single-stroke SVG
/// fonts. Existing paths keep their exact geometry by default.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(crate = "crate::serde")]
pub enum PathSmoothing {
    /// Preserve the authored path exactly.
    #[default]
    None,
    /// Lightly smooth polyline runs while preserving sharp corners.
    Light,
    /// More aggressively smooth polyline runs for pen-like paths.
    Strong,
}

impl Interpolatable for PathSmoothing {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        if t < 1.0 {
            *self
        } else {
            *other
        }
    }
}

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

/// Controls how stroke segments are joined at path vertices.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(crate = "crate::serde")]
pub enum StrokeJoin {
    /// Extends outer edges to a point, subject to the renderer's miter limit.
    #[default]
    Miter,
    /// Rounds the outside of each join.
    Round,
    /// Cuts joins off with a straight edge.
    Bevel,
}

impl Interpolatable for StrokeJoin {}

/// Describes the outline drawn around vector geometry.
///
/// Pax currently renders strokes centered on the underlying path. For open
/// geometry, `cap` controls how the stroke terminates at the start and end of
/// the path, while `join` controls how adjacent segments meet.
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
    /// The join style used where adjacent stroke segments meet.
    pub join: Property<StrokeJoin>,
}

impl Default for Stroke {
    fn default() -> Self {
        Self {
            color: Default::default(),
            width: Property::new(Size::Pixels(Numeric::F64(0.0))),
            cap: Property::new(StrokeCap::default()),
            join: Property::new(StrokeJoin::default()),
        }
    }
}

impl PartialEq for Stroke {
    fn eq(&self, other: &Self) -> bool {
        self.color.get() == other.color.get()
            && self.width.get() == other.width.get()
            && self.cap.get() == other.cap.get()
            && self.join.get() == other.join.get()
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
            join: Property::new(if t < 1.0 {
                self.join.get()
            } else {
                other.join.get()
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
        self.join.get().hash(state);
    }
}

/// Logical scene depth for lighting calculations.
///
/// `Depth` is expressed in logical pixels. Unlike [`Size`], it is not anchored
/// to a viewport or parent box, so percent and combined units are intentionally
/// rejected during value coercion.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Default, PartialEq, PartialOrd)]
#[serde(crate = "crate::serde")]
pub struct Depth(pub Numeric);

impl Depth {
    /// Returns the depth as a floating-point logical pixel value.
    pub fn to_float(&self) -> f64 {
        self.0.to_float()
    }
}

impl From<Numeric> for Depth {
    fn from(value: Numeric) -> Self {
        Self(value)
    }
}

impl From<f64> for Depth {
    fn from(value: f64) -> Self {
        Self(Numeric::F64(value))
    }
}

impl Hash for Depth {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl Interpolatable for Depth {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self(self.0.interpolate(&other.0, t))
    }
}

impl HelperFunctions for Depth {}

/// A three-dimensional vector in logical scene space.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "crate::serde")]
pub struct Vector3 {
    /// X component.
    pub x: f64,
    /// Y component.
    pub y: f64,
    /// Z component.
    pub z: f64,
}

impl Vector3 {
    /// Constructs a 3D vector.
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self { x, y, z }
    }
}

impl Default for Vector3 {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        }
    }
}

impl Hash for Vector3 {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.x.to_bits().hash(state);
        self.y.to_bits().hash(state);
        self.z.to_bits().hash(state);
    }
}

impl Interpolatable for Vector3 {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self {
            x: self.x.interpolate(&other.x, t),
            y: self.y.interpolate(&other.y, t),
            z: self.z.interpolate(&other.z, t),
        }
    }
}

impl HelperFunctions for Vector3 {
    fn register_all_functions() {
        register_function(
            "Vector3".to_string(),
            "new".to_string(),
            std::sync::Arc::new(|args| {
                if args.len() != 3 {
                    return Err("Expected 3 arguments for function Vector3::new".to_string());
                }
                let mut itr = args.into_iter();
                let x = f64::try_coerce(itr.next().unwrap())?;
                let y = f64::try_coerce(itr.next().unwrap())?;
                let z = f64::try_coerce(itr.next().unwrap())?;
                Ok(Vector3::new(x, y, z).to_pax_value())
            }),
        );
    }
}

/// Shape of a light contribution in logical scene space.
#[derive(Debug, Copy, Clone, Serialize, Deserialize, Default, PartialEq, Eq, Hash)]
#[serde(crate = "crate::serde")]
pub enum LightShape {
    /// A positional light with radius-based attenuation.
    #[default]
    Point,
    /// A light with direction but no position or attenuation.
    Directional,
}

impl Interpolatable for LightShape {}
impl HelperFunctions for LightShape {}

/// Tunable response parameters for a light-reactive vector material.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(crate = "crate::serde")]
pub struct MaterialParams {
    /// Ambient contribution multiplier.
    pub ambient: Property<f64>,
    /// Diffuse contribution multiplier.
    pub diffuse: Property<f64>,
    /// Specular contribution multiplier.
    pub specular: Property<f64>,
    /// Surface roughness in the `[0.0, 1.0]` range.
    pub roughness: Property<f64>,
    /// Metallic response in the `[0.0, 1.0]` range.
    pub metallic: Property<f64>,
    /// Additive emissive color.
    pub emissive: Property<Color>,
    /// Additive emissive intensity.
    pub emissive_intensity: Property<f64>,
}

impl Default for MaterialParams {
    fn default() -> Self {
        Self {
            ambient: Property::new(1.0),
            diffuse: Property::new(0.82),
            specular: Property::new(0.08),
            roughness: Property::new(0.78),
            metallic: Property::new(0.0),
            emissive: Property::new(Color::BLACK),
            emissive_intensity: Property::new(0.0),
        }
    }
}

impl PartialEq for MaterialParams {
    fn eq(&self, other: &Self) -> bool {
        self.ambient.get() == other.ambient.get()
            && self.diffuse.get() == other.diffuse.get()
            && self.specular.get() == other.specular.get()
            && self.roughness.get() == other.roughness.get()
            && self.metallic.get() == other.metallic.get()
            && self.emissive.get() == other.emissive.get()
            && self.emissive_intensity.get() == other.emissive_intensity.get()
    }
}

impl Hash for MaterialParams {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.ambient.get().to_bits().hash(state);
        self.diffuse.get().to_bits().hash(state);
        self.specular.get().to_bits().hash(state);
        self.roughness.get().to_bits().hash(state);
        self.metallic.get().to_bits().hash(state);
        self.emissive.get().hash(state);
        self.emissive_intensity.get().to_bits().hash(state);
    }
}

impl Interpolatable for MaterialParams {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self {
            ambient: Property::new(self.ambient.get().interpolate(&other.ambient.get(), t)),
            diffuse: Property::new(self.diffuse.get().interpolate(&other.diffuse.get(), t)),
            specular: Property::new(self.specular.get().interpolate(&other.specular.get(), t)),
            roughness: Property::new(self.roughness.get().interpolate(&other.roughness.get(), t)),
            metallic: Property::new(self.metallic.get().interpolate(&other.metallic.get(), t)),
            emissive: Property::new(self.emissive.get().interpolate(&other.emissive.get(), t)),
            emissive_intensity: Property::new(
                self.emissive_intensity
                    .get()
                    .interpolate(&other.emissive_intensity.get(), t),
            ),
        }
    }
}

impl HelperFunctions for MaterialParams {}

/// Light-reactive surface response for vector primitives.
///
/// This is intentionally named `Material`; `texture` is reserved for future
/// bitmap-backed texture maps and pattern data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "crate::serde")]
pub enum Material {
    /// Responds to scene lighting using parameterized material coefficients.
    Lit(MaterialParams),
    /// Ignores scene lighting and preserves legacy unlit rendering behavior.
    Unlit,
}

impl Default for Material {
    fn default() -> Self {
        Self::matte()
    }
}

impl Material {
    /// A soft, low-specular material suitable as the default lit response.
    pub fn matte() -> Self {
        Self::Lit(MaterialParams::default())
    }

    /// A higher-specular material with lower roughness.
    pub fn glossy(specular: f64) -> Self {
        let mut params = MaterialParams::default();
        params.specular = Property::new(specular.clamp(0.0, 1.0));
        params.roughness = Property::new(0.28);
        Self::Lit(params)
    }

    /// A metallic material response.
    pub fn metallic(metallic: f64) -> Self {
        let mut params = MaterialParams::default();
        params.metallic = Property::new(metallic.clamp(0.0, 1.0));
        params.diffuse = Property::new(0.42);
        params.specular = Property::new(0.55);
        params.roughness = Property::new(0.36);
        Self::Lit(params)
    }

    /// An emissive material that adds color independent of lights.
    pub fn emissive(color: Color, intensity: f64) -> Self {
        let mut params = MaterialParams::default();
        params.emissive = Property::new(color);
        params.emissive_intensity = Property::new(intensity.max(0.0));
        Self::Lit(params)
    }

    /// Creates a lit material from explicit coefficients.
    pub fn custom(params: MaterialParams) -> Self {
        Self::Lit(params)
    }

    /// A material that ignores authored lights.
    pub fn unlit() -> Self {
        Self::Unlit
    }
}

impl Hash for Material {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Material::Lit(params) => {
                state.write_u8(0);
                params.hash(state);
            }
            Material::Unlit => {
                state.write_u8(1);
            }
        }
    }
}

impl Interpolatable for Material {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        match (self, other) {
            (Self::Lit(lhs), Self::Lit(rhs)) => Self::Lit(lhs.interpolate(rhs, t)),
            _ if t < 1.0 => self.clone(),
            _ => other.clone(),
        }
    }
}

impl HelperFunctions for Material {
    fn register_all_functions() {
        register_function(
            "Material".to_string(),
            "matte".to_string(),
            std::sync::Arc::new(|args| {
                if !args.is_empty() {
                    return Err("Expected 0 arguments for function Material::matte".to_string());
                }
                Ok(Material::matte().to_pax_value())
            }),
        );
        register_function(
            "Material".to_string(),
            "glossy".to_string(),
            std::sync::Arc::new(|args| {
                if args.len() != 1 {
                    return Err("Expected 1 argument for function Material::glossy".to_string());
                }
                let specular = f64::try_coerce(args.into_iter().next().unwrap())?;
                Ok(Material::glossy(specular).to_pax_value())
            }),
        );
        register_function(
            "Material".to_string(),
            "metallic".to_string(),
            std::sync::Arc::new(|args| {
                if args.len() != 1 {
                    return Err("Expected 1 argument for function Material::metallic".to_string());
                }
                let metallic = f64::try_coerce(args.into_iter().next().unwrap())?;
                Ok(Material::metallic(metallic).to_pax_value())
            }),
        );
        register_function(
            "Material".to_string(),
            "emissive".to_string(),
            std::sync::Arc::new(|args| {
                if args.len() != 2 {
                    return Err("Expected 2 arguments for function Material::emissive".to_string());
                }
                let mut itr = args.into_iter();
                let color = Color::try_coerce(itr.next().unwrap())?;
                let intensity = f64::try_coerce(itr.next().unwrap())?;
                Ok(Material::emissive(color, intensity).to_pax_value())
            }),
        );
        register_function(
            "Material".to_string(),
            "custom".to_string(),
            std::sync::Arc::new(|args| {
                if args.len() != 1 {
                    return Err("Expected 1 argument for function Material::custom".to_string());
                }
                let params = MaterialParams::try_coerce(args.into_iter().next().unwrap())?;
                Ok(Material::custom(params).to_pax_value())
            }),
        );
        register_function(
            "Material".to_string(),
            "unlit".to_string(),
            std::sync::Arc::new(|args| {
                if !args.is_empty() {
                    return Err("Expected 0 arguments for function Material::unlit".to_string());
                }
                Ok(Material::unlit().to_pax_value())
            }),
        );
    }
}

/// Resolved ambient light for one logical canvas layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "crate::serde")]
pub struct SceneAmbientLight {
    /// Ambient color.
    pub color: Color,
    /// Ambient intensity.
    pub intensity: f64,
}

impl Default for SceneAmbientLight {
    fn default() -> Self {
        Self {
            color: Color::WHITE,
            intensity: 1.0,
        }
    }
}

impl Interpolatable for SceneAmbientLight {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self {
            color: self.color.interpolate(&other.color, t),
            intensity: self.intensity.interpolate(&other.intensity, t),
        }
    }
}

impl HelperFunctions for SceneAmbientLight {}

/// Resolved light contribution for one logical canvas layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "crate::serde")]
pub struct SceneLight {
    /// Positional or directional light shape.
    pub shape: LightShape,
    /// Position in logical canvas pixels for point lights.
    pub position: Vector3,
    /// Direction in scene space for directional lights.
    pub direction: Vector3,
    /// Light color.
    pub color: Color,
    /// Light intensity.
    pub intensity: f64,
    /// Point light radius in logical pixels.
    pub radius: f64,
    /// Whether this light contributes.
    pub enabled: bool,
}

impl Default for SceneLight {
    fn default() -> Self {
        Self {
            shape: LightShape::Point,
            position: Vector3::default(),
            direction: Vector3::new(0.0, 0.0, -1.0),
            color: Color::WHITE,
            intensity: 1.0,
            radius: 240.0,
            enabled: true,
        }
    }
}

impl Interpolatable for SceneLight {
    fn interpolate(&self, other: &Self, t: f64) -> Self {
        Self {
            shape: if t < 1.0 { self.shape } else { other.shape },
            position: self.position.interpolate(&other.position, t),
            direction: self.direction.interpolate(&other.direction, t),
            color: self.color.interpolate(&other.color, t),
            intensity: self.intensity.interpolate(&other.intensity, t),
            radius: self.radius.interpolate(&other.radius, t),
            enabled: if t < 1.0 { self.enabled } else { other.enabled },
        }
    }
}

impl HelperFunctions for SceneLight {}

/// Resolved lighting state for one logical canvas layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(crate = "crate::serde")]
pub struct SceneLighting {
    /// Whether authored lights or ambient overrides are present.
    pub active: bool,
    /// Whether `ambient` came from an enabled authored `AmbientLight`.
    ///
    /// The default ambient is only applied to primitives with at least one
    /// eligible direct light. An authored ambient remains layer-wide.
    pub ambient_is_authored: bool,
    /// Singleton ambient contribution.
    pub ambient: SceneAmbientLight,
    /// Positional and directional light contributions.
    pub lights: Vec<SceneLight>,
}

impl Default for SceneLighting {
    fn default() -> Self {
        Self {
            active: false,
            ambient_is_authored: false,
            ambient: SceneAmbientLight::default(),
            lights: vec![],
        }
    }
}

impl SceneLighting {
    /// Maximum number of simultaneously enabled lights in one target canvas layer.
    pub const MAX_LIGHTS: usize = 8;

    /// Ambient intensity used when point/directional lights exist but no explicit ambient override exists.
    pub const DEFAULT_AMBIENT_INTENSITY: f64 = 0.35;

    /// Returns lighting that preserves unlit legacy rendering.
    pub fn identity() -> Self {
        Self::default()
    }

    /// Creates active lighting with Pax's default ambient term.
    pub fn with_default_ambient(lights: Vec<SceneLight>) -> Self {
        Self {
            active: !lights.is_empty(),
            ambient_is_authored: false,
            ambient: SceneAmbientLight {
                color: Color::WHITE,
                intensity: Self::DEFAULT_AMBIENT_INTENSITY,
            },
            lights,
        }
    }
}

impl Interpolatable for SceneLighting {}
impl HelperFunctions for SceneLighting {}

impl Interpolatable for Fill {
    fn interpolate(&self, _other: &Self, _t: f64) -> Self {
        // TODO interpolation
        self.clone()
    }
}

/// Describes a linear gradient fill with a start and end point, and a list of color stops.
///
/// Pax templates canonically author each point as `[x, y]`: magic index `0`
/// is the horizontal coordinate and index `1` is the vertical coordinate.
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
///
/// Pax templates canonically author each point as `[x, y]`: magic index `0`
/// is the horizontal coordinate and index `1` is the vertical coordinate.
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
    ///
    /// Pax templates should normally use `@gradient`. When this helper is
    /// needed explicitly, pass `start` and `end` as `[x, y]` lists.
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

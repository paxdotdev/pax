use pax_engine::*;
use pax_runtime::api::{
    Color, ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType as T, ShaderPass,
    ShaderProgram, ShaderValue as V,
};

/// Experimental CMY contour glints from a caller-positioned spherical light.
///
/// By default, follows the alpha silhouette of GPU-rendered children. For internal
/// contours in two-color artwork, set `surface_color` and `contrast_color` to its
/// distinct opaque fills. The original silhouette and alpha remain unchanged.
/// This is a stylized contour treatment, not refraction of the scene behind it.
///
/// Enable the `shaders` feature. WGPU captures vectors and renderer-backed images;
/// native descendants are unsupported. CPU renders the children without the effect.
/// The caller owns light position, choreography, and visibility. No Radiance required.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::shaders::rim_refraction::RimRefractionInstance")]
pub struct RimRefraction {
    /// None (default) follows source alpha. Some(color) selects that fill in two-color artwork.
    pub surface_color: Property<Option<Color>>,
    /// Unselected fill when surface_color is Some. Defaults to white; must differ from the surface.
    pub contrast_color: Property<Color>,
    /// Neutral highlight color, white by default. Alpha scales the entire rim's strength.
    pub light_color: Property<Color>,
    /// Light X in container-local logical pixels. Defaults to 820.
    pub light_x: Property<f64>,
    /// Light Y in container-local logical pixels. Defaults to 185.
    pub light_y: Property<f64>,
    /// Positive distance behind the surface, 120–900 local pixels. Defaults to 340.
    pub light_depth: Property<f64>,
    /// Effective sphere radius, 1–160 local pixels. Defaults to 60; larger values broaden glints.
    pub light_radius: Property<f64>,
    /// Light intensity, 0–6. Defaults to 5; zero bypasses the effect.
    pub brightness: Property<f64>,
    /// Tone-mapped contour tint strength, 0–1. Defaults to 0.795; zero disables the rim.
    pub intensity: Property<f64>,
    /// Inward contour footprint, 0–8 local pixels. Defaults to 3.4; zero disables the rim.
    pub rim_width: Property<f64>,
    /// Angular CMY separation, 0–1. Defaults to 0.98; one spreads lobes by ±40 degrees.
    /// Zero overlaps all three into a neutral highlight at any saturation.
    pub dispersion: Property<f64>,
    /// Chromatic strength, 0–1. Defaults to 0.77; zero is neutral, one fully chromatic.
    /// Tint is blended over the surface, so white fills do not clip the colors away.
    pub saturation: Property<f64>,
}

impl Default for RimRefraction {
    fn default() -> Self {
        Self {
            surface_color: Property::new(None),
            contrast_color: Property::new(Color::WHITE),
            light_color: Property::new(Color::WHITE),
            light_x: Property::new(820.),
            light_y: Property::new(185.),
            light_depth: Property::new(340.),
            light_radius: Property::new(60.),
            brightness: Property::new(5.),
            intensity: Property::new(0.795),
            rim_width: Property::new(3.4),
            dispersion: Property::new(0.98),
            saturation: Property::new(0.77),
        }
    }
}

static PROGRAM: ShaderProgram = ShaderProgram {
    label: "Rim refraction",
    passes: &[ShaderPass {
        label: "rim-refraction.wgsl",
        source: include_str!("rim_refraction.wgsl"),
        parameters: &[
            ShaderParameter {
                name: "surface_color",
                ty: T::Vec4,
            },
            ShaderParameter {
                name: "contrast_color",
                ty: T::Vec4,
            },
            ShaderParameter {
                name: "select_color",
                ty: T::Uint,
            },
            ShaderParameter {
                name: "light_color",
                ty: T::Vec4,
            },
            ShaderParameter {
                name: "light",
                ty: T::Vec4,
            },
            ShaderParameter {
                name: "light_radius",
                ty: T::Float,
            },
            ShaderParameter {
                name: "brightness",
                ty: T::Float,
            },
            ShaderParameter {
                name: "rim",
                ty: T::Vec4,
            },
        ],
        inputs: [ShaderInput::Subtree; 2],
        downsample: 1,
    }],
};

impl RimRefraction {
    fn recipe(&self) -> ShaderEffect {
        let rgba = |c: Color| V::Vec4(c.to_rgba_0_1().map(|v| v as f32));
        let surface = self.surface_color.get();
        ShaderEffect {
            lighting: None,
            program: &PROGRAM,
            parameters: vec![vec![
                rgba(surface.clone().unwrap_or(Color::BLACK)),
                rgba(self.contrast_color.get()),
                V::Uint(surface.is_some() as u32),
                rgba(self.light_color.get()),
                V::Vec4([
                    self.light_x.get() as f32,
                    self.light_y.get() as f32,
                    self.light_depth.get().clamp(120., 900.) as f32,
                    0.,
                ]),
                V::Float(self.light_radius.get().clamp(1., 160.) as f32),
                V::Float(self.brightness.get().clamp(0., 6.) as f32),
                V::Vec4([
                    self.intensity.get().clamp(0., 1.) as f32,
                    self.rim_width.get().clamp(0., 8.) as f32,
                    self.dispersion.get().clamp(0., 1.) as f32,
                    self.saturation.get().clamp(0., 1.) as f32,
                ]),
            ]],
            // Reserve the full contour footprint so width animation reuses captures.
            outset: 9.,
        }
    }
}

effect_instance!(RimRefractionInstance, RimRefraction);

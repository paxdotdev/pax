use pax_engine::*;
use pax_runtime::api::{
    Color, ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType as T, ShaderPass,
    ShaderProgram, ShaderValue as V,
};

/// Experimental CMY contour glints from a caller-positioned sphere or scoped scene lights.
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
    /// Neutral highlight color, white by default. Alpha sets the scene-lit rim's
    /// final blend opacity. In sphere mode it scales light strength before tone mapping.
    pub light_color: Property<Color>,
    /// Use the container's scoped direct lights and camera. Defaults to false (caller-positioned sphere).
    pub scene_lighting: Property<bool>,
    /// Scene mode starts revealing the rim at this direct specular response, reaching full activation
    /// 0.08 above it. Defaults to 0.30. Ambient light does not enter the threshold.
    pub light_threshold: Property<f64>,
    /// Scene mode: full falloff span along the contour for each finite emitter, as a
    /// fraction of the container width. Zero (default) leaves its response unfocused.
    /// Point and strip contributions add before thresholding; directional lights are unchanged.
    pub light_span: Property<f64>,
    /// CMY cycles per visible, focused point or strip glint (0.25–8, default 1).
    /// One cycle shows one cyan, magenta, and yellow band without changing the lit footprint.
    pub hue_cycles: Property<f64>,
    /// Focused-glint hue turns per surface width of rightward or downward travel (−8–8).
    /// Defaults to 0 (fixed band order). Negative values reverse the rotation.
    pub hue_travel: Property<f64>,
    /// Light X in container-local logical pixels. Defaults to 820.
    pub light_x: Property<f64>,
    /// Light Y in container-local logical pixels. Defaults to 185.
    pub light_y: Property<f64>,
    /// Positive distance behind the surface, 120–900 local pixels. Defaults to 340.
    pub light_depth: Property<f64>,
    /// Effective sphere radius, 1–160 local pixels. Defaults to 60; larger values broaden glints.
    pub light_radius: Property<f64>,
    /// Caller-positioned sphere intensity, 0–6. Defaults to 5; zero bypasses the effect.
    pub brightness: Property<f64>,
    /// Tone-mapped contour tint strength, 0–1. Defaults to 0.795; zero disables the rim.
    pub intensity: Property<f64>,
    /// Inward contour footprint, 0–8 local pixels. Defaults to 3.4; zero disables the rim.
    pub rim_width: Property<f64>,
    /// Angular CMY separation, 0–1. Defaults to 0.98; one spreads the sphere-mode lobes by ±40 degrees.
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
            scene_lighting: Property::new(false),
            light_threshold: Property::new(0.30),
            light_span: Property::new(0.0),
            hue_cycles: Property::new(1.0),
            hue_travel: Property::new(0.0),
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
                name: "scene_mode",
                ty: T::Uint,
            },
            ShaderParameter {
                name: "threshold",
                ty: T::Float,
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
            ShaderParameter {
                name: "hue_cycles",
                ty: T::Float,
            },
            ShaderParameter {
                name: "hue_travel",
                ty: T::Float,
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
        let scene = self.scene_lighting.get();
        let width = self.rim_width.get().clamp(0., 8.);
        ShaderEffect {
            lighting: scene.then(Default::default),
            program: &PROGRAM,
            parameters: vec![vec![
                rgba(surface.clone().unwrap_or(Color::BLACK)),
                rgba(self.contrast_color.get()),
                V::Uint(surface.is_some() as u32),
                rgba(self.light_color.get()),
                V::Uint(scene as u32),
                V::Float(self.light_threshold.get().max(0.) as f32),
                V::Vec4([
                    self.light_x.get() as f32,
                    self.light_y.get() as f32,
                    self.light_depth.get().clamp(120., 900.) as f32,
                    self.light_span.get().clamp(0., 1.) as f32,
                ]),
                V::Float(self.light_radius.get().clamp(1., 160.) as f32),
                V::Float(self.brightness.get().clamp(0., 6.) as f32),
                V::Vec4([
                    self.intensity.get().clamp(0., 1.) as f32,
                    width as f32,
                    self.dispersion.get().clamp(0., 1.) as f32,
                    self.saturation.get().clamp(0., 1.) as f32,
                ]),
                V::Float(self.hue_cycles.get().clamp(0.25, 8.) as f32),
                V::Float(self.hue_travel.get().clamp(-8., 8.) as f32),
            ]],
            // Light motion has fixed support. Keep the sphere mode's original width-animation envelope.
            outset: if scene { width.ceil() + 1. } else { 9. },
        }
    }
}

effect_instance!(RimRefractionInstance, RimRefraction);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn focused_hue_controls_change_only_their_own_color_uniforms() {
        let rim = RimRefraction::default();
        rim.scene_lighting.set(true);
        rim.light_span.set(0.22);
        let cycles = PROGRAM.passes[0]
            .parameters
            .iter()
            .position(|p| p.name == "hue_cycles")
            .unwrap();
        let base = rim.recipe();
        assert_eq!(base.parameters[0][cycles], V::Float(1.));
        for (input, expected) in [(0., 0.25), (0.5, 0.5), (2., 2.), (10., 8.)] {
            rim.hue_cycles.set(input);
            let recipe = rim.recipe();
            let mut expected_params = base.parameters.clone();
            expected_params[0][cycles] = V::Float(expected);
            assert_eq!(recipe.parameters, expected_params);
            assert_eq!(recipe.outset, base.outset);
            assert!(recipe.encode_parameters().is_ok());
        }
        rim.hue_cycles.set(1.);
        let travel = PROGRAM.passes[0]
            .parameters
            .iter()
            .position(|p| p.name == "hue_travel")
            .unwrap();
        assert_eq!(base.parameters[0][travel], V::Float(0.));
        for (input, expected) in [(-9., -8.), (-2., -2.), (2., 2.), (9., 8.)] {
            rim.hue_travel.set(input);
            let recipe = rim.recipe();
            let mut expected_params = base.parameters.clone();
            expected_params[0][travel] = V::Float(expected);
            assert_eq!(recipe.parameters, expected_params);
            assert_eq!(recipe.outset, base.outset);
        }
    }

    #[test]
    fn scene_lighting_is_opt_in_and_light_changes_keep_the_sampling_margin() {
        let rim = RimRefraction::default();
        assert!(rim.recipe().lighting.is_none());
        assert_eq!(rim.recipe().outset, 9.);

        rim.scene_lighting.set(true);
        rim.rim_width.set(2.);
        let recipe = rim.recipe();
        assert!(recipe.lighting.is_some());
        assert_eq!(recipe.outset, 3.);
        assert!(recipe.encode_parameters().is_ok());

        rim.light_threshold.set(0.65);
        rim.light_x.set(-200.);
        assert_eq!(rim.recipe().outset, recipe.outset);
        rim.rim_width.set(3.4);
        assert_eq!(rim.recipe().outset, 5.);

        rim.scene_lighting.set(false);
        assert!(rim.recipe().lighting.is_none());
        assert_eq!(rim.recipe().outset, 9.);
    }
}

use pax_engine::*;
use pax_runtime::api::{
    Color, ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType, ShaderPass,
    ShaderProgram, ShaderValue,
};

/// Experimental point-light projection through a two-color rendered stencil.
///
/// Foreground coverage emits the full light field; background-colored coverage
/// can transmit it and catch a polished sheen. The full-resolution silhouette
/// and current reveal masks remain crisp. Intended for a light aperture with
/// dark, obsidian-like lettering; this is a stylized 2.5D scattering effect,
/// not a physically calibrated glass material or arbitrary background refraction.
///
/// Enable the `shaders` feature. WGPU captures vector/image children; native
/// descendants are unsupported. CPU renders the children without the effect.
/// The caller owns light position, choreography, and visibility.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::shaders::radiant_aperture::RadiantApertureInstance")]
pub struct RadiantAperture {
    /// Emitting source color, white by default. Must match the actual solid fill.
    pub foreground: Property<Color>,
    /// Glass source color, black by default. Must be distinct from foreground.
    pub background: Property<Color>,
    /// Light X in container-local logical pixels. Defaults to 820.
    pub light_x: Property<f64>,
    /// Light Y in container-local logical pixels. Defaults to 185.
    pub light_y: Property<f64>,
    /// Positive distance behind the stencil plane, 120–900 local pixels. Defaults to 340.
    pub light_depth: Property<f64>,
    /// Light intensity, 0–6. Defaults to 5; zero bypasses rays and glass shading.
    pub brightness: Property<f64>,
    /// Maximum projected distance, 0–360 local pixels. Defaults to 320.
    pub reach: Property<f64>,
    /// Distance attenuation exponent, 0.5–4. Defaults to 1.
    pub falloff: Property<f64>,
    /// Fraction transmitted by background-colored coverage, 0–0.5. Defaults to 0.16.
    /// Affects the projected rays and the light visible inside the source; retains source alpha.
    pub transmission: Property<f64>,
    /// Moving sheen and narrow contour highlight strength, 0–1. Defaults to 0.55.
    /// Set both this and transmission to zero for solid source coverage.
    pub polish: Property<f64>,
}

impl Default for RadiantAperture {
    fn default() -> Self {
        Self {
            foreground: Property::new(Color::WHITE),
            background: Property::new(Color::BLACK),
            light_x: Property::new(820.),
            light_y: Property::new(185.),
            light_depth: Property::new(340.),
            brightness: Property::new(5.),
            reach: Property::new(320.),
            falloff: Property::new(1.),
            transmission: Property::new(0.16),
            polish: Property::new(0.55),
        }
    }
}

const PARAMETERS: &[ShaderParameter] = &[
    ShaderParameter {
        name: "foreground",
        ty: ShaderParameterType::Vec4,
    },
    ShaderParameter {
        name: "background",
        ty: ShaderParameterType::Vec4,
    },
    ShaderParameter {
        name: "light",
        ty: ShaderParameterType::Vec4,
    },
    ShaderParameter {
        name: "brightness",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "reach",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "falloff",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "transmission",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "polish",
        ty: ShaderParameterType::Float,
    },
];

static PROGRAM: ShaderProgram = ShaderProgram {
    label: "Radiant aperture",
    passes: &[
        ShaderPass {
            label: "radiance-projection.wgsl",
            source: concat!(
                include_str!("radiance_common.wgsl"),
                include_str!("radiance_projection.wgsl")
            ),
            parameters: PARAMETERS,
            inputs: [ShaderInput::Subtree; 2],
            downsample: 2,
        },
        ShaderPass {
            label: "radiance-composite.wgsl",
            source: concat!(
                include_str!("radiance_common.wgsl"),
                include_str!("radiance_composite.wgsl")
            ),
            parameters: PARAMETERS,
            inputs: [ShaderInput::Pass(0), ShaderInput::Subtree],
            downsample: 1,
        },
    ],
};

impl RadiantAperture {
    fn recipe(&self) -> ShaderEffect {
        let rgba = |c: Color| ShaderValue::Vec4(c.to_rgba_0_1().map(|v| v as f32));
        let values = vec![
            rgba(self.foreground.get()),
            rgba(self.background.get()),
            ShaderValue::Vec4([
                self.light_x.get() as f32,
                self.light_y.get() as f32,
                self.light_depth.get().clamp(120., 900.) as f32,
                600.,
            ]),
            ShaderValue::Float(self.brightness.get().clamp(0., 6.) as f32),
            ShaderValue::Float(self.reach.get().clamp(0., 360.) as f32),
            ShaderValue::Float(self.falloff.get().clamp(0.5, 4.) as f32),
            ShaderValue::Float(self.transmission.get().clamp(0., 0.5) as f32),
            ShaderValue::Float(self.polish.get().clamp(0., 1.) as f32),
        ];
        ShaderEffect {
            program: &PROGRAM,
            parameters: vec![values.clone(), values],
            // Reserve the tuning envelope so light motion never reallocates capture bounds.
            outset: 376.,
        }
    }
}

effect_instance!(RadiantApertureInstance, RadiantAperture);

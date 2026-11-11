use pax_engine::*;
use pax_runtime::api::Color;
use pax_runtime::api::{
    ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType, ShaderPass, ShaderProgram,
    ShaderValue,
};

/// Experimental, colorable corona around a two-color GPU-rendered stencil.
///
/// Supply the distinct, opaque foreground/background colors actually used by the
/// source. Foreground coverage emits independently of its luminance, including
/// internal boundaries. Transparent regions, source opacity and masks are supported;
/// arbitrary gradients and multicolor sources are not. The core keeps its geometry.
///
/// Enable the `shaders` Cargo feature. WGPU captures vectors and renderer-backed
/// images; native descendants are unsupported. Vello CPU renders the children
/// without the effect. Motion is caller-owned: a fixed phase freezes the field.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::shaders::corona::CoronaInstance")]
pub struct Corona {
    /// Emitting source color. Defaults to white; must match the source's actual fill.
    pub foreground: Property<Color>,
    /// Non-emitting source color. Defaults to black; must differ from foreground.
    pub background: Property<Color>,
    /// Optional emission tint. None follows foreground, including dark-on-light palettes.
    /// Tint changes only the emitted light; its alpha scales emission coverage.
    pub tint: Property<Option<Color>>,
    /// Periodic motion position in cycles, initially zero. The caller owns time and visibility.
    pub phase: Property<f64>,
    /// Emission-layer opacity, clamped to 0–1. Defaults to 1; zero leaves surface shading.
    pub emission: Property<f64>,
    /// Maximum projection distance in container-local logical pixels, 0–120. Defaults to 20.
    pub reach: Property<f64>,
    /// Variation between short and long flares, 0–1. Defaults to 0.8.
    pub amplitude: Property<f64>,
    /// Distance attenuation exponent, 0.5–5. Defaults to 2.7.
    pub falloff: Property<f64>,
    /// Surface tonal depth, 0–0.35. Defaults to 0.04; zero retains source colors.
    pub surface: Property<f64>,
}

impl Default for Corona {
    fn default() -> Self {
        Self {
            foreground: Property::new(Color::WHITE),
            background: Property::new(Color::BLACK),
            tint: Property::new(None),
            phase: Property::new(0.),
            emission: Property::new(1.),
            reach: Property::new(20.),
            amplitude: Property::new(0.8),
            falloff: Property::new(2.7),
            surface: Property::new(0.04),
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
        name: "tint",
        ty: ShaderParameterType::Vec4,
    },
    ShaderParameter {
        name: "phase",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "emission",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "reach",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "amplitude",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "falloff",
        ty: ShaderParameterType::Float,
    },
    ShaderParameter {
        name: "surface",
        ty: ShaderParameterType::Float,
    },
];

static PROGRAM: ShaderProgram = ShaderProgram {
    label: "Corona",
    passes: &[
        ShaderPass {
            label: "corona-beams.wgsl",
            source: concat!(
                include_str!("corona_common.wgsl"),
                include_str!("corona_beams.wgsl")
            ),
            parameters: PARAMETERS,
            inputs: [ShaderInput::Subtree; 2],
            downsample: 2,
        },
        ShaderPass {
            label: "corona-composite.wgsl",
            source: concat!(
                include_str!("corona_common.wgsl"),
                include_str!("corona_composite.wgsl")
            ),
            parameters: PARAMETERS,
            inputs: [ShaderInput::Pass(0), ShaderInput::Subtree],
            downsample: 1,
        },
    ],
};

impl Corona {
    fn recipe(&self) -> ShaderEffect {
        let rgba = |c: Color| ShaderValue::Vec4(c.to_rgba_0_1().map(|v| v as f32));
        let values = vec![
            rgba(self.foreground.get()),
            rgba(self.background.get()),
            rgba(self.tint.get().unwrap_or_else(|| self.foreground.get())),
            ShaderValue::Float(self.phase.get().rem_euclid(1.) as f32),
            ShaderValue::Float(self.emission.get().clamp(0., 1.) as f32),
            ShaderValue::Float(self.reach.get().clamp(0., 120.) as f32),
            ShaderValue::Float(self.amplitude.get().clamp(0., 1.) as f32),
            ShaderValue::Float(self.falloff.get().clamp(0.5, 5.) as f32),
            ShaderValue::Float(self.surface.get().clamp(0., 0.35) as f32),
        ];
        ShaderEffect {
            lighting: None,
            program: &PROGRAM,
            parameters: vec![values.clone(), values],
            // Reserve the whole tuning envelope plus reconstruction support so
            // changing reach cannot reallocate the capture on every slider event.
            outset: 144.,
        }
    }
}

effect_instance!(CoronaInstance, Corona);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_recipe_retains_the_accepted_corona_tuning() {
        let corona = Corona::default();
        let recipe = corona.recipe();
        recipe.encode_parameters().unwrap();
        assert_eq!(corona.emission.get(), 1.);
        assert_eq!(corona.reach.get(), 20.);
        assert_eq!(corona.amplitude.get(), 0.8);
        assert_eq!(corona.falloff.get(), 2.7);
        assert_eq!(corona.surface.get(), 0.04);
        assert_eq!(recipe.parameters[0][0], recipe.parameters[0][2]);
    }

    #[test]
    fn tint_is_independent_of_source_palette_and_follows_it_when_unset() {
        let corona = Corona::default();
        corona.foreground.set(Color::BLACK);
        corona.background.set(Color::WHITE);
        let untinted = corona.recipe();
        assert_eq!(untinted.parameters[0][0], untinted.parameters[0][2]);

        corona.tint.set(Some(Color::rgba(
            0.into(),
            128.into(),
            255.into(),
            128.into(),
        )));
        let tinted = corona.recipe();
        tinted.encode_parameters().unwrap();
        for (before, after) in untinted.parameters.iter().zip(&tinted.parameters) {
            for (index, (a, b)) in before.iter().zip(after).enumerate() {
                if index == 2 {
                    assert_ne!(a, b);
                    let ShaderValue::Vec4(rgba) = b else {
                        panic!("tint must be RGBA")
                    };
                    assert!((rgba[3] - 128. / 255.).abs() < 0.0001);
                } else {
                    assert_eq!(a, b, "tint must not change source extraction or motion");
                }
            }
        }
        corona.tint.set(None);
        assert_eq!(corona.recipe(), untinted);
    }
}

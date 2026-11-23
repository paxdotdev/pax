use pax_engine::*;
use pax_runtime::api::{
    Color, ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType, ShaderPass,
    ShaderProgram, ShaderValue,
};

/// Shallow cloud glass clipped to the alpha of the projected face.
/// Keep text, icons, and hit targets outside the effect.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::shaders::cloud_glass::CloudGlassInstance")]
pub struct CloudGlass {
    /// Amount of gas (0–8). Higher values grow the wisps and populate more pockets.
    pub density: Property<f64>,
    /// Scattered-light color of the gas, independent of the clear shell.
    pub gas_color: Property<Color>,
    /// Opacity of occupied gas (0–1). The wisp shapes and shell lighting stay fixed.
    pub gas_opacity: Property<f64>,
    /// Characteristic radius of a billow in local logical pixels.
    pub billow_scale: Property<f64>,
    /// Add the scene-lit CMY button glint across the 2 px shell bevel. Defaults to false.
    pub button_rim: Property<bool>,
    /// Integrated drift time, in seconds at normal speed. Hold to freeze.
    pub phase: Property<f64>,
}

impl Default for CloudGlass {
    fn default() -> Self {
        Self {
            density: Property::new(2.9),
            gas_color: Property::new(Color::WHITE),
            gas_opacity: Property::new(0.95),
            billow_scale: Property::new(23.),
            phase: Property::new(0.),
            button_rim: Property::new(false),
        }
    }
}

static CLOUD: ShaderProgram = ShaderProgram {
    label: "Cloud glass",
    passes: &[ShaderPass {
        label: "cloud.wgsl",
        source: concat!(include_str!("scene_rim.wgsl"), include_str!("cloud.wgsl")),
        inputs: [ShaderInput::Subtree; 2],
        downsample: 1,
        parameters: &[
            ShaderParameter {
                name: "gas",
                ty: ShaderParameterType::Vec4,
            },
            ShaderParameter {
                name: "tint",
                ty: ShaderParameterType::Vec4,
            },
        ],
    }],
};

impl CloudGlass {
    fn recipe(&self) -> ShaderEffect {
        let mut tint = self.gas_color.get().to_rgba_0_1().map(|v| v as f32);
        tint[3] *= self.gas_opacity.get().clamp(0., 1.) as f32;
        ShaderEffect {
            program: &CLOUD,
            lighting: Some(Default::default()),
            parameters: vec![vec![
                ShaderValue::Vec4([
                    self.density.get().clamp(0., 8.) as f32,
                    self.billow_scale.get().clamp(4., 128.) as f32,
                    self.phase.get() as f32,
                    u8::from(self.button_rim.get()) as f32,
                ]),
                ShaderValue::Vec4(tint),
            ]],
            outset: 3.,
        }
    }
}

effect_instance!(CloudGlassInstance, CloudGlass);

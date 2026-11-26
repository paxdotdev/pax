use pax_engine::*;
use pax_runtime::api::{
    ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType, ShaderPass, ShaderProgram,
    ShaderValue,
};

/// Orthographic water caustics over the projected artwork's paint and alpha.
/// Phase is integrated wave time. Hold it still to freeze the water while lights move.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::shaders::pool_floor_surface::PoolFloorSurfaceInstance")]
pub struct PoolFloorSurface {
    /// Wave time in seconds at normal speed.
    pub phase: Property<f64>,
    /// Spatial frequency, clamped to 0.25–3. Defaults to 1.
    pub density: Property<f64>,
    /// Wave strength from 0 to 1. Defaults to 0.65.
    pub amplitude: Property<f64>,
    /// Render at full physical resolution instead of half on each axis.
    pub full_resolution: Property<bool>,
    /// Use a pass-through at matching resolution for profiling.
    pub identity: Property<bool>,
    /// Add the scene-lit two-pixel button contour in the same pass.
    pub button_rim: Property<bool>,
}
impl Default for PoolFloorSurface {
    fn default() -> Self {
        Self {
            phase: Property::new(2.9),
            density: Property::new(1.0),
            amplitude: Property::new(0.65),
            full_resolution: Property::new(false),
            identity: Property::new(false),
            button_rim: Property::new(false),
        }
    }
}

const WATER: ShaderPass = ShaderPass {
    label: "Pool floor / caustics",
    source: concat!(
        include_str!("pool_floor.wgsl"),
        "
fn effect(p: vec2<f32>) -> vec4<f32> {
    let source = sample_input0(p);
    let color = pool_color(p, source.rgb / max(source.a, 0.0001));
    return vec4<f32>(color * source.a, source.a);
}"
    ),
    parameters: &[ShaderParameter {
        name: "wave",
        ty: ShaderParameterType::Vec4,
    }],
    inputs: [ShaderInput::Subtree; 2],
    downsample: 2,
};

const IDENTITY: ShaderPass = ShaderPass {
    label: "Pool floor / identity",
    source: "fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p); }",
    ..WATER
};

const HALF: ShaderProgram = ShaderProgram {
    label: "Pool floor",
    passes: &[WATER],
};
const FULL: ShaderProgram = ShaderProgram {
    label: "Pool floor / full",
    passes: &[ShaderPass {
        downsample: 1,
        ..WATER
    }],
};
const IDENTITY_HALF: ShaderProgram = ShaderProgram {
    label: "Pool floor / identity",
    passes: &[IDENTITY],
};
const IDENTITY_FULL: ShaderProgram = ShaderProgram {
    label: "Pool floor / identity full",
    passes: &[ShaderPass {
        downsample: 1,
        ..IDENTITY
    }],
};

const BUTTON_PASS: ShaderPass = ShaderPass {
    source: concat!(
        include_str!("scene_rim.wgsl"),
        include_str!("pool_floor.wgsl"),
        include_str!("pool_button.wgsl")
    ),
    ..WATER
};
const BUTTON_HALF: ShaderProgram = ShaderProgram {
    label: "Pool floor / button",
    passes: &[BUTTON_PASS],
};
const BUTTON_FULL: ShaderProgram = ShaderProgram {
    label: "Pool floor / full button",
    passes: &[ShaderPass {
        downsample: 1,
        ..BUTTON_PASS
    }],
};

impl PoolFloorSurface {
    fn recipe(&self) -> ShaderEffect {
        let density = self.density.get();
        let density = if density.is_nan() {
            1.
        } else {
            density.clamp(0.25, 3.)
        };
        let program = match (
            self.identity.get(),
            self.button_rim.get(),
            self.full_resolution.get(),
        ) {
            (false, false, false) => &HALF,
            (false, true, false) => &BUTTON_HALF,
            (false, false, true) => &FULL,
            (false, true, true) => &BUTTON_FULL,
            (true, _, false) => &IDENTITY_HALF,
            (true, _, true) => &IDENTITY_FULL,
        };
        // The identity comparison shares the cadence, lighting dependency and resolution.
        ShaderEffect {
            program,
            lighting: Some(Default::default()),
            outset: 0.,
            parameters: vec![vec![ShaderValue::Vec4([
                self.phase.get() as f32,
                density as f32,
                self.amplitude.get().clamp(0., 1.) as f32,
                program.passes[0].downsample as f32,
            ])]],
        }
    }
}
effect_instance!(PoolFloorSurfaceInstance, PoolFloorSurface);

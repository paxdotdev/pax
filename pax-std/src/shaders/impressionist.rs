use pax_engine::*;
use pax_runtime::api::{
    ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType as T, ShaderPass,
    ShaderProgram, ShaderValue as V,
};

/// Experimental painterly filter with deterministic marks anchored to the container.
///
/// Requires WGPU and captures vectors and renderer-backed images. Native descendants
/// are unsupported. Source colors update independently of the marks' breathing motion.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::shaders::impressionist::ImpressionistInstance")]
pub struct Impressionist {
    /// Breathing phase, initially zero. Animate this property to move the marks.
    pub phase: Property<f64>,
    /// Deterministic mark seed. Defaults to zero.
    pub seed: Property<u32>,
    /// Mark size in local logical pixels, clamped to 4–32. Defaults to zero (effective size 4).
    pub size: Property<f64>,
}

static PAINT: ShaderProgram = ShaderProgram {
    label: "Surface anchored paint",
    passes: &[
        ShaderPass {
            label: "orientation.wgsl",
            source: include_str!("orientation.wgsl"),
            parameters: &[ShaderParameter {
                name: "size",
                ty: T::Float,
            }],
            inputs: [ShaderInput::Subtree; 2],
            downsample: 2,
        },
        ShaderPass {
            label: "strokes.wgsl",
            source: include_str!("strokes.wgsl"),
            parameters: &[
                ShaderParameter {
                    name: "phase",
                    ty: T::Float,
                },
                ShaderParameter {
                    name: "seed",
                    ty: T::Uint,
                },
                ShaderParameter {
                    name: "size",
                    ty: T::Float,
                },
            ],
            inputs: [ShaderInput::Subtree, ShaderInput::Pass(0)],
            downsample: 1,
        },
    ],
};

impl Impressionist {
    fn recipe(&self) -> ShaderEffect {
        let size = self.size.get().clamp(4., 32.) as f32;
        ShaderEffect {
            program: &PAINT,
            parameters: vec![
                vec![V::Float(size)],
                vec![
                    V::Float(self.phase.get() as f32),
                    V::Uint(self.seed.get()),
                    V::Float(size),
                ],
            ],
            outset: 64.,
        }
    }
}

effect_instance!(ImpressionistInstance, Impressionist);

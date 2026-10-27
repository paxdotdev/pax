use pax_engine::*;
use pax_runtime::api::{
    ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType as T, ShaderPass,
    ShaderProgram, ShaderValue as V,
};

/// Experimental Gaussian blur of GPU-rendered descendants.
///
/// WGPU captures vectors and renderer-backed images. Native descendants are
/// unsupported in GPU captures; native overlap masks retain original coverage.
/// Vello CPU renders the children without blur. Use `@gpu` / `@cpu` for an alternate appearance.
#[pax]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::shaders::gaussian_blur::GaussianBlurInstance")]
pub struct GaussianBlur {
    /// Standard deviation in local logical pixels, clamped to 0–48. Defaults to zero (identity).
    pub radius: Property<f64>,
    /// Force full-resolution separable convolution. Defaults to false, using adaptive resolution.
    pub reference: Property<bool>,
}

const BLUR_PARAMETERS: &[ShaderParameter] = &[
    ShaderParameter {
        name: "sigma",
        ty: T::Float,
    },
    ShaderParameter {
        name: "axis",
        ty: T::Vec2,
    },
    ShaderParameter {
        name: "spacing",
        ty: T::Float,
    },
    ShaderParameter {
        name: "enabled",
        ty: T::Uint,
    },
];
static BLUR: ShaderProgram = ShaderProgram {
    label: "Gaussian blur",
    passes: &[
        ShaderPass {
            label: "gaussian-x.wgsl",
            source: include_str!("gaussian.wgsl"),
            parameters: BLUR_PARAMETERS,
            inputs: [ShaderInput::Subtree; 2],
            downsample: 1,
        },
        ShaderPass {
            label: "gaussian-y.wgsl",
            source: include_str!("gaussian.wgsl"),
            parameters: BLUR_PARAMETERS,
            inputs: [ShaderInput::Pass(0); 2],
            downsample: 1,
        },
        ShaderPass {
            label: "gaussian-coarse-x.wgsl",
            source: include_str!("gaussian.wgsl"),
            parameters: BLUR_PARAMETERS,
            inputs: [ShaderInput::Subtree; 2],
            downsample: 4,
        },
        ShaderPass {
            label: "gaussian-coarse-y.wgsl",
            source: include_str!("gaussian.wgsl"),
            parameters: BLUR_PARAMETERS,
            inputs: [ShaderInput::Pass(2); 2],
            downsample: 4,
        },
        ShaderPass {
            label: "blur-mix.wgsl",
            source: include_str!("blur_mix.wgsl"),
            parameters: &[ShaderParameter {
                name: "approximation",
                ty: T::Float,
            }],
            inputs: [ShaderInput::Pass(1), ShaderInput::Pass(3)],
            downsample: 1,
        },
    ],
};

impl GaussianBlur {
    fn recipe(&self) -> ShaderEffect {
        let sigma = self.radius.get().clamp(0., 48.) as f32;
        let t = if self.reference.get() {
            0.
        } else {
            ((sigma - 6.) / 4.).clamp(0., 1.)
        };
        let blend = t * t * (3. - 2. * t);
        let pass = |axis, spacing, enabled| {
            vec![
                V::Float(sigma),
                V::Vec2(axis),
                V::Float(spacing),
                V::Uint(enabled),
            ]
        };
        ShaderEffect {
            program: &BLUR,
            parameters: vec![
                pass([1., 0.], 1., (blend < 1.) as u32),
                pass([0., 1.], 1., (blend < 1.) as u32),
                pass([1., 0.], 4., (blend > 0.) as u32),
                pass([0., 1.], 1., (blend > 0.) as u32),
                vec![V::Float(blend)],
            ],
            outset: 145.,
        }
    }
}

effect_instance!(GaussianBlurInstance, GaussianBlur);

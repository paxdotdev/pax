use pax_engine::*;
use pax_runtime::api::{
    ShaderEffect, ShaderInput, ShaderParameter, ShaderParameterType as T, ShaderPass,
    ShaderProgram, ShaderValue as V,
};

/// Experimental, surface-anchored specular glints with soft atmospheric flares.
///
/// A seeded field samples the captured children's alpha to place tiny reflectors.
/// Reflector positions stay fixed. Scoped scene `LightSource`s drive the glints;
/// move the light to sweep reflections across the artwork. Compose a child vector
/// with `Material::frosted(...)` for grain and a broad sheen from those same lights.
/// Optional facet motion changes each core's projected shape and reflection.
/// Long flare rays share a fixed orientation in the effect's local viewing frame;
/// soft haze surrounds them. Light/facet motion changes intensity, not ray direction.
/// Flares can extend beyond the source silhouette without replacing the artwork.
/// This is a stylized sequin treatment, not a physical material or particle simulation.
///
/// Enable the `shaders` feature. WGPU captures vectors and renderer-backed images;
/// native Text and controls cannot be captured. Use vector outlines for lettering.
/// CPU renders the children unchanged. The caller owns motion and visibility.
#[pax]
#[custom(Default)]
#[engine_import_path("pax_engine")]
#[primitive("pax_std::shaders::flashy::FlashyInstance")]
pub struct Flashy {
    /// Optional facet-motion position in turns, initially zero. One turn loops seamlessly.
    /// Leave fixed for a stationary surface and animate the light position instead.
    pub phase: Property<f64>,
    /// Deterministic spatial distribution and reflector orientations. Defaults to 17.
    pub seed: Property<u32>,
    /// Reflector spacing in local logical pixels, clamped to 8–48. Defaults to 18.
    /// Larger values create fewer, larger glints. Changing spacing redistributes them.
    pub spacing: Property<f64>,
    /// Glint strength, clamped to 0–4. Defaults to 1.4; zero preserves the captured material.
    /// Scene light intensity controls the shared illumination of materials and glints.
    pub brightness: Property<f64>,
    /// Atmospheric flare strength and spread, clamped to 0–5. Defaults to 0.65.
    /// Zero retains pinpoint cores. Up to 1 increases strength and spread;
    /// above 1 scales ray length and haze radius, reaching five times the size at 5.
    /// Maximum support is 4.75 times spacing; larger flares cost more GPU work.
    pub flare: Property<f64>,
}

impl Default for Flashy {
    fn default() -> Self {
        Self {
            phase: Property::new(0.),
            seed: Property::new(17),
            spacing: Property::new(18.),
            brightness: Property::new(1.4),
            flare: Property::new(0.65),
        }
    }
}

static PROGRAM: ShaderProgram = ShaderProgram {
    label: "Flashy",
    passes: &[ShaderPass {
        label: "flashy.wgsl",
        source: include_str!("flashy.wgsl"),
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
                name: "spacing",
                ty: T::Float,
            },
            ShaderParameter {
                name: "brightness",
                ty: T::Float,
            },
            ShaderParameter {
                name: "flare",
                ty: T::Float,
            },
        ],
        inputs: [ShaderInput::Subtree; 2],
        downsample: 1,
    }],
};

impl Flashy {
    fn recipe(&self) -> ShaderEffect {
        ShaderEffect {
            lighting: Some(Default::default()),
            program: &PROGRAM,
            parameters: vec![vec![
                // Reduce before f32 conversion so long-running clocks retain phase precision.
                V::Float(self.phase.get().rem_euclid(1.) as f32),
                V::Uint(self.seed.get()),
                V::Float(self.spacing.get().clamp(8., 48.) as f32),
                V::Float(self.brightness.get().clamp(0., 4.) as f32),
                V::Float(self.flare.get().clamp(0., 5.) as f32),
            ]],
            // 4.75 * max spacing plus source filtering. Stable through all tuning and motion.
            outset: 240.,
        }
    }
}

effect_instance!(FlashyInstance, Flashy);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wgsl_validates_with_actual_parameter_schema() {
        let pass = &PROGRAM.passes[0];
        let mut source = String::from("struct Parameters {\n");
        for parameter in pass.parameters {
            let ty = match parameter.ty {
                T::Float => "f32",
                T::Uint => "u32",
                T::Vec2 => "vec2<f32>",
                T::Vec4 => "vec4<f32>",
            };
            source.push_str(&format!("@align(16) @size(16) {}: {ty},\n", parameter.name));
        }
        source.push_str("};\n@group(0) @binding(1) var<uniform> parameters: Parameters;\n");
        // Exercise the public fragment contract without depending on unpublished
        // files from pax-gpu. Live fixture validation covers the renderer wrapper.
        source.push_str(
            "fn sample_input0(p: vec2<f32>) -> vec4<f32> { return vec4<f32>(p, 0.0, 1.0); }\n\
             fn scene_specular(p: vec2<f32>, n: vec3<f32>, power: f32) -> vec3<f32> { return vec3<f32>(1.0); }\n\
             fn input0_pixel_size() -> vec2<f32> { return vec2<f32>(1.0); }\n\
             @fragment fn main(@builtin(position) p: vec4<f32>) -> @location(0) vec4<f32> { return effect(p.xy); }\n",
        );
        source.push_str(pass.source);
        let module = naga::front::wgsl::parse_str(&source)
            .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
        naga::valid::Validator::new(
            naga::valid::ValidationFlags::all(),
            naga::valid::Capabilities::empty(),
        )
        .validate(&module)
        .unwrap_or_else(|e| panic!("{}", e.emit_to_string(&source)));
    }

    #[test]
    fn phase_wraps_without_reseeding_or_changing_capture_bounds() {
        let effect = Flashy::default();
        effect.phase.set(0.25);
        let first = effect.recipe();
        for phase in [-0.75, 1.25, 1_000_000.25] {
            effect.phase.set(phase);
            assert_eq!(first, effect.recipe());
        }
        first.encode_parameters().unwrap();
    }

    #[test]
    fn tuning_is_bounded_and_keeps_one_capture_envelope() {
        let effect = Flashy::default();
        for (spacing, brightness, flare, expected) in [
            (-100., -1., -1., [8., 0., 0.]),
            (18., 1.4, 1., [18., 1.4, 1.]),
            (18., 1.4, 2., [18., 1.4, 2.]),
            (18., 1.4, 4., [18., 1.4, 4.]),
            (48., 4., 5., [48., 4., 5.]),
            (1000., 20., 20., [48., 4., 5.]),
        ] {
            effect.spacing.set(spacing);
            effect.brightness.set(brightness);
            effect.flare.set(flare);
            effect.seed.set(u32::MAX);
            let recipe = effect.recipe();
            assert_eq!(recipe.outset, 240.);
            assert_eq!(&recipe.parameters[0][2..5], &expected.map(V::Float));
            recipe.encode_parameters().unwrap();
        }
        effect.phase.set(f64::NAN);
        assert!(effect.recipe().encode_parameters().is_err());
    }

    #[test]
    fn requests_scene_lighting_without_private_light_parameters() {
        let recipe = Flashy::default().recipe();
        assert!(recipe.lighting.is_some());
        assert_eq!(recipe.parameters[0].len(), 5);
        recipe.encode_parameters().unwrap();
    }
}

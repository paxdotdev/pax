//! Bounded fragment-pass execution. Sources, pipelines and outputs have separate lifetimes so
//! changing a uniform never recompiles a pipeline or records the source scene again.
use super::*;
use pax_runtime_api::{ShaderEffect, ShaderInput, ShaderParameterType, ShaderPass};
use wgpu::util::DeviceExt;

const MAX_EFFECT_BYTES: u64 = 256 * 1024 * 1024;

pub(super) struct Effects {
    layout: BindGroupLayout,
    composite_layout: BindGroupLayout,
    composite: RenderPipeline,
    sampler: wgpu::Sampler,
    pipelines: HashMap<ShaderPass, RenderPipeline>,
    outputs: HashMap<u32, Output>,
}

struct PassOutput {
    texture: Texture,
    view: TextureView,
    parameters: wgpu::Buffer,
    binding: BindGroup,
}

struct Output {
    recipe: &'static pax_runtime_api::ShaderProgram,
    source: Texture,
    domain: [[f32; 4]; 5],
    domain_buffer: wgpu::Buffer,
    rect_buffer: wgpu::Buffer,
    passes: Vec<PassOutput>,
    composite: BindGroup,
    signature: Option<u64>,
    bytes: u64,
}

fn uniform(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}
fn texture(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: true },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}
fn sampler(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
        count: None,
    }
}

pub(super) fn shader_source(pass: &ShaderPass) -> Result<String, String> {
    let mut source = String::from("struct Parameters {\n");
    if pass.parameters.is_empty() {
        source.push_str("@align(16) @size(16) unused: f32,\n");
    }
    for parameter in pass.parameters {
        let ty = match parameter.ty {
            ShaderParameterType::Float => "f32",
            ShaderParameterType::Uint => "u32",
            ShaderParameterType::Vec2 => "vec2<f32>",
            ShaderParameterType::Vec4 => "vec4<f32>",
        };
        source.push_str(&format!(
            "@align(16) @size(16) {}: {},\n",
            parameter.name, ty
        ));
    }
    source.push_str("};\n@group(0) @binding(1) var<uniform> parameters: Parameters;\n");
    source.push_str(include_str!("effect.wgsl"));
    source.push('\n');
    source.push_str(pass.source);
    let module = naga::front::wgsl::parse_str(&source)
        .map_err(|e| format!("{}: {}", pass.label, e.emit_to_string(&source)))?;
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .map_err(|e| format!("{}: {}", pass.label, e.emit_to_string(&source)))?;
    // The wrapper owns exactly five resources. Naga permits unused declarations to alias
    // their bindings, so checking only the binding range would admit extra resources.
    let resources: Vec<_> = module
        .global_variables
        .iter()
        .filter_map(|(_, variable)| variable.binding.as_ref())
        .collect();
    if module.entry_points.len() != 2
        || resources.len() != 5
        || resources
            .iter()
            .any(|binding| binding.group != 0 || binding.binding > 4)
    {
        return Err(format!(
            "{}: effect code cannot declare resources or entry points",
            pass.label
        ));
    }
    Ok(source)
}

fn pipeline(
    device: &wgpu::Device,
    source: &str,
    label: &str,
    layouts: &[&BindGroupLayout],
    format: TextureFormat,
    samples: u32,
    composite: bool,
) -> RenderPipeline {
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some(label),
        source: wgpu::ShaderSource::Wgsl(source.into()),
    });
    let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some(label),
        bind_group_layouts: layouts,
        immediate_size: 0,
    });
    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some(label),
        layout: Some(&layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: Some("vs_main"),
            compilation_options: Default::default(),
            buffers: &[],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: Some("fs_main"),
            compilation_options: Default::default(),
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: composite.then_some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        primitive: Default::default(),
        depth_stencil: composite.then_some(wgpu::DepthStencilState {
            format: TextureFormat::Stencil8,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::Always,
            stencil: wgpu::StencilState {
                front: wgpu::StencilFaceState {
                    compare: wgpu::CompareFunction::Equal,
                    fail_op: wgpu::StencilOperation::Keep,
                    depth_fail_op: wgpu::StencilOperation::Keep,
                    pass_op: wgpu::StencilOperation::Keep,
                },
                back: wgpu::StencilFaceState::IGNORE,
                read_mask: !0,
                write_mask: 0,
            },
            bias: Default::default(),
        }),
        multisample: wgpu::MultisampleState {
            count: samples,
            ..Default::default()
        },
        multiview_mask: None,
        cache: None,
    })
}

impl Effects {
    fn new(backend: &RenderBackend<'_>) -> Self {
        let device = &backend.device;
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Effect pass"),
            entries: &[uniform(0), uniform(1), texture(2), texture(3), sampler(4)],
        });
        let composite_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Effect composition"),
            entries: &[uniform(0), uniform(1), texture(2), sampler(3)],
        });
        let alpha = alpha_mask::sampling_layout(device);
        let composite = pipeline(
            device,
            include_str!("effect_composite.wgsl"),
            "Premultiplied effect composition",
            &[&composite_layout, &alpha],
            backend.surface_config.format,
            backend.sample_count,
            true,
        );
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Effect linear sampler"),
            min_filter: wgpu::FilterMode::Linear,
            mag_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        Self {
            layout,
            composite_layout,
            composite,
            sampler,
            pipelines: HashMap::new(),
            outputs: HashMap::new(),
        }
    }

    pub(super) fn draw(&self, id: u32, pass: &mut wgpu::RenderPass<'_>) {
        if let Some(output) = self.outputs.get(&id) {
            pass.set_pipeline(&self.composite);
            pass.set_bind_group(0, &output.composite, &[]);
            pass.draw(0..6, 0..1);
        }
    }

    pub(super) fn retain(&mut self, active: &HashSet<u32>) {
        self.outputs.retain(|id, _| active.contains(id));
        self.pipelines.retain(|recipe, _| {
            self.outputs
                .values()
                .any(|o| o.recipe.passes.contains(recipe))
        });
    }
}

impl RenderBackend<'_> {
    pub(crate) fn render_effect(
        &mut self,
        id: u32,
        effect: &ShaderEffect,
        transform: Transform2D,
        signature: u64,
        stats: &mut crate::ResourceChurnStats,
    ) {
        let parameters = effect
            .encode_parameters()
            .unwrap_or_else(|error| panic!("shader effect {}: {error}", effect.program.label));
        let (source, rect) = self
            .retained_surfaces
            .as_ref()
            .unwrap()
            .effect_source(capture::CaptureKey::EffectSource(id), &self.globals);
        let inverse = transform
            .inverse()
            .expect("shader effect requires an invertible transform");
        let rows = |t: Transform2D| [[t.m11, t.m21, t.m31, 0.0], [t.m12, t.m22, t.m32, 0.0]];
        let a = rows(inverse);
        let b = rows(transform);
        let domain = [rect, a[0], a[1], b[0], b[1]];
        if self.effects.is_none() {
            self.effects = Some(Effects::new(self));
        }
        let effects = self.effects.as_mut().unwrap();
        for recipe in effect.program.passes {
            if !effects.pipelines.contains_key(recipe) {
                let source = shader_source(recipe).unwrap_or_else(|error| {
                    panic!("shader effect {}: {error}", effect.program.label)
                });
                effects.pipelines.insert(
                    recipe.clone(),
                    pipeline(
                        &self.device,
                        &source,
                        recipe.label,
                        &[&effects.layout],
                        self.surface_config.format,
                        1,
                        false,
                    ),
                );
                stats.effect_pipeline_compilations += 1;
            }
        }
        if effects
            .outputs
            .get(&id)
            .is_none_or(|o| o.recipe != effect.program || o.source != source)
        {
            let required: u64 = effect
                .program
                .passes
                .iter()
                .map(|p| {
                    4 * u64::from(source.width().div_ceil(p.downsample))
                        * u64::from(source.height().div_ceil(p.downsample))
                })
                .sum();
            let used: u64 = effects
                .outputs
                .iter()
                .filter(|(key, _)| **key != id)
                .map(|(_, o)| o.bytes)
                .sum();
            assert!(
                used + required <= MAX_EFFECT_BYTES,
                "shader effect {} exceeds the 256 MiB per-tile output budget",
                effect.program.label
            );
            let domain_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Effect coordinates"),
                    contents: bytemuck::cast_slice(&domain),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });
            let source_view = source.create_view(&Default::default());
            let mut passes: Vec<PassOutput> = Vec::new();
            for (recipe, values) in effect.program.passes.iter().zip(&parameters) {
                let texture = self.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(recipe.label),
                    size: wgpu::Extent3d {
                        width: source.width().div_ceil(recipe.downsample),
                        height: source.height().div_ceil(recipe.downsample),
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: self.surface_config.format,
                    usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                    view_formats: &[],
                });
                let view = texture.create_view(&Default::default());
                let parameters =
                    self.device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Effect parameters"),
                            contents: values,
                            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                        });
                let input = |input| match input {
                    ShaderInput::Subtree => &source_view,
                    ShaderInput::Pass(n) => &passes[n].view,
                };
                let binding = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some(recipe.label),
                    layout: &effects.layout,
                    entries: &[
                        wgpu::BindGroupEntry {
                            binding: 0,
                            resource: domain_buffer.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 1,
                            resource: parameters.as_entire_binding(),
                        },
                        wgpu::BindGroupEntry {
                            binding: 2,
                            resource: wgpu::BindingResource::TextureView(input(recipe.inputs[0])),
                        },
                        wgpu::BindGroupEntry {
                            binding: 3,
                            resource: wgpu::BindingResource::TextureView(input(recipe.inputs[1])),
                        },
                        wgpu::BindGroupEntry {
                            binding: 4,
                            resource: wgpu::BindingResource::Sampler(&effects.sampler),
                        },
                    ],
                });
                passes.push(PassOutput {
                    texture,
                    view,
                    parameters,
                    binding,
                });
            }
            let rect_buffer = self
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Effect output rectangle"),
                    contents: bytemuck::cast_slice(&rect),
                    usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
                });
            let composite = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Effect output"),
                layout: &effects.composite_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: self.globals_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: rect_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(&passes.last().unwrap().view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::Sampler(&effects.sampler),
                    },
                ],
            });
            stats.effect_texture_allocations += passes.len() as u64;
            stats.effect_texture_bytes += required;
            effects.outputs.insert(
                id,
                Output {
                    recipe: effect.program,
                    source,
                    domain,
                    domain_buffer,
                    rect_buffer,
                    passes,
                    composite,
                    signature: None,
                    bytes: required,
                },
            );
        }
        let output = effects.outputs.get_mut(&id).unwrap();
        if output.domain != domain {
            // Coordinate changes reuse pass textures and bindings. Only a new
            // source texture or recipe changes their resource dependencies.
            self.queue
                .write_buffer(&output.domain_buffer, 0, bytemuck::cast_slice(&domain));
            self.queue
                .write_buffer(&output.rect_buffer, 0, bytemuck::cast_slice(&rect));
            output.domain = domain;
            output.signature = None;
        }
        if output.signature == Some(signature) {
            stats.effect_cache_hits += 1;
            return;
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some(effect.program.label),
            });
        for ((pass, recipe), values) in output
            .passes
            .iter()
            .zip(effect.program.passes)
            .zip(&parameters)
        {
            self.queue.write_buffer(&pass.parameters, 0, values);
            let mut render = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some(recipe.label),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &pass.view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
                multiview_mask: None,
            });
            render.set_pipeline(&effects.pipelines[recipe]);
            render.set_bind_group(0, &pass.binding, &[]);
            render.draw(0..3, 0..1);
            stats.effect_passes += 1;
            stats.effect_pixels +=
                u64::from(pass.texture.width()) * u64::from(pass.texture.height());
        }
        output.signature = Some(signature);
        self.enqueue_command_buffer(encoder.finish());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_runtime_api::ShaderParameter;
    #[test]
    fn rejects_additional_resources_on_engine_bindings() {
        for binding in 0..=4 {
            let source = format!(
                "@group(0) @binding({binding}) var extra: texture_2d<f32>;\n\
                 fn effect(p: vec2<f32>) -> vec4<f32> {{ return sample_input0(p); }}"
            );
            let pass = ShaderPass {
                label: "extra-resource.wgsl",
                source: Box::leak(source.into_boxed_str()),
                parameters: &[],
                inputs: [ShaderInput::Subtree; 2],
                downsample: 1,
            };
            let error = shader_source(&pass).expect_err("extra resources must be rejected");
            assert!(error.contains("extra-resource.wgsl: effect code cannot declare resources"));
        }
    }

    #[test]
    fn generated_uniform_layout_and_diagnostics() {
        let pass=ShaderPass {label:"user-color.wgsl",source:"fn effect(p: vec2<f32>) -> vec4<f32> { return sample_input0(p) * parameters.color; }",parameters:&[ShaderParameter {name:"color",ty:ShaderParameterType::Vec4}],inputs:[ShaderInput::Subtree;2],downsample:1};
        shader_source(&pass).unwrap();
        let mut invalid = pass.clone();
        invalid.source = "fn effect(p: vec2<f32>) -> vec4<f32> { return parameters.typo; }";
        let error = shader_source(&invalid).unwrap_err();
        assert!(error.contains("user-color.wgsl") && error.contains("typo"));
        invalid = pass;
        invalid.source="@compute @workgroup_size(1) fn extra() {} fn effect(p: vec2<f32>) -> vec4<f32> { return vec4<f32>(0.0); }";
        assert!(shader_source(&invalid)
            .unwrap_err()
            .contains("entry points"));
    }
}

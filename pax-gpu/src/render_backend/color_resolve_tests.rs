//! Pixel/order checks for resolve-on-consumption versus eager per-pass resolves.
use super::*;

fn make_target(device: &Device, uses: &AttachmentUses, samples: u32) -> CaptureTarget {
    let make = |samples| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Resolve regression target"),
            size: wgpu::Extent3d {
                width: 64,
                height: 64,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: samples,
            dimension: wgpu::TextureDimension::D2,
            format: TextureFormat::Rgba8Unorm,
            usage: TextureUsages::RENDER_ATTACHMENT
                | if samples == 1 {
                    TextureUsages::COPY_SRC
                } else {
                    TextureUsages::empty()
                },
            view_formats: &[],
        })
    };
    let texture = make(1);
    let view = texture.create_view(&Default::default());
    let multisampled_target = (samples > 1).then(|| {
        let texture = make(samples);
        let view = texture.create_view(&Default::default());
        Attachment::new(texture, view, uses)
    });
    CaptureTarget {
        texture,
        view,
        multisampled_target,
        initialized: false,
        color_resolve: ColorResolve::default(),
    }
}

fn readback(
    device: &Device,
    encoder: &mut wgpu::CommandEncoder,
    texture: &Texture,
) -> wgpu::Buffer {
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: 256 * 64,
        usage: BufferUsages::COPY_DST | BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(256),
                rows_per_image: None,
            },
        },
        texture.size(),
    );
    buffer
}

fn pixels(device: &Device, buffer: wgpu::Buffer) -> Vec<u8> {
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, |result| result.unwrap());
    device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
    let pixels = buffer.slice(..).get_mapped_range().to_vec();
    buffer.unmap();
    pixels
}

#[test]
#[ignore = "requires a native GPU adapter"]
fn color_resolve_consumers_match_eager_pixels_and_preserve_samples() {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let (device, queue) = pollster::block_on(adapter.request_device(&Default::default())).unwrap();
    let scope = device.push_error_scope(wgpu::ErrorFilter::Validation);
    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: None,
        source: wgpu::ShaderSource::Wgsl(r#"
            struct Out { @builtin(position) p: vec4f, @location(0) @interpolate(flat) color: vec4f }
            @vertex fn vs(@builtin(vertex_index) i: u32, @builtin(instance_index) n: u32) -> Out {
                let p = array(vec2f(-0.9, -0.7), vec2f(0.6, -0.8), vec2f(-0.4, 0.9));
                let c = array(vec4f(0.5,0,0,0.5), vec4f(0,0.5,0,0.5), vec4f(0,0,0.5,0.5));
                var o: Out; o.p = vec4f(p[i] + vec2f(f32(n)*0.12, 0), 0, 1); o.color = c[n]; return o;
            }
            @fragment fn fs(i: Out) -> @location(0) vec4f { return i.color; }
        "#.into()),
    });
    for samples in [1, 4] {
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: None,
            layout: None,
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: Default::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: samples,
                ..Default::default()
            },
            multiview_mask: None,
            cache: None,
        });
        let mut results = Vec::new();
        for eager in [true, false] {
            let uses = AttachmentUses::default();
            let target = make_target(&device, &uses, samples);
            let mut encoder = device.create_command_encoder(&Default::default());
            let mut buffers = Vec::new();
            let mut commands = Vec::new();
            for n in 0..3 {
                {
                    let view = target.color_write_view();
                    let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view,
                            depth_slice: None,
                            resolve_target: (eager && samples > 1).then_some(&target.view),
                            ops: wgpu::Operations {
                                load: if n == 0 {
                                    wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT)
                                } else {
                                    wgpu::LoadOp::Load
                                },
                                store: wgpu::StoreOp::Store,
                            },
                        })],
                        ..Default::default()
                    });
                    pass.set_pipeline(&pipeline);
                    pass.draw(0..3, n..n + 1);
                }
                if n == 0 {
                    // An encoder/yield boundary does not consume the color.
                    assert_eq!(target.color_resolve.encoded_resolves.get(), 0);
                    commands.push(encoder.finish());
                    encoder = device.create_command_encoder(&Default::default());
                } else {
                    if !eager {
                        target.resolve_color(&mut encoder);
                        // A repeated read needs no second resolve.
                        target.resolve_color(&mut encoder);
                        assert_eq!(
                            target.color_resolve.encoded_resolves.get(),
                            if samples > 1 { n as usize } else { 0 }
                        );
                    }
                    buffers.push(readback(&device, &mut encoder, &target.texture));
                }
            }
            if !eager && samples > 1 {
                // New destination, retained MSAA samples, no new draw.
                let fresh = make_target(&device, &uses, 1);
                let fresh_state = ColorResolve::default();
                fresh_state.encode(
                    &mut encoder,
                    &fresh.view,
                    target.multisampled_target.as_ref(),
                );
                assert_eq!(fresh_state.encoded_resolves.get(), 1);
                buffers.push(readback(&device, &mut encoder, &fresh.texture));
            }
            commands.push(encoder.finish());
            let batch = uses.take(commands);
            drop(target); // Encoded resolves retain attachments through submission.
            batch.submit(&queue);
            let mut output: Vec<_> = buffers.into_iter().map(|b| pixels(&device, b)).collect();
            if output.len() == 3 {
                assert_eq!(output.pop().unwrap(), output[1], "fresh destination");
            }
            assert_ne!(
                output[0], output[1],
                "write/read/write/read must not reuse stale pixels"
            );
            if samples > 1 {
                assert!(
                    output[0].chunks_exact(4).any(|p| p[3] > 0 && p[3] < 128),
                    "antialiasing preserved"
                );
            }
            results.push(output);
        }
        assert_eq!(results[0], results[1], "eager/deferred, {samples} samples");
    }
    assert!(pollster::block_on(scope.pop()).is_none());
}

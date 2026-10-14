//! Run explicitly on a machine with a WGPU adapter (hardware or software):
//! `cargo test -p pax-gpu --test offscreen -- --ignored --nocapture`
//! Set `WGPU_BACKEND=metal`, `vulkan`, or `dx12` to qualify a specific backend.
use pax_gpu::render_backend::{GpuContext, RenderBackend, RenderConfig};
use pax_gpu::{point, Color, Fill, Path, WgpuRenderer};
use std::time::Duration;

fn rect(x: f32, y: f32, w: f32, h: f32) -> Path {
    let mut path = Path::builder();
    path.begin(point(x, y));
    path.line_to(point(x + w, y));
    path.line_to(point(x + w, y + h));
    path.line_to(point(x, y + h));
    path.end(true);
    path.build()
}

fn pixels(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
    let row = (texture.width() * 4).next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("offscreen test readback"),
        size: u64::from(row * texture.height()),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(texture.height()),
            },
        },
        texture.size(),
    );
    let submission = queue.submit([encoder.finish()]);
    let (send, receive) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |result| {
            send.send(result).unwrap();
        });
    device
        .poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })
        .unwrap();
    receive
        .recv_timeout(Duration::from_secs(10))
        .unwrap()
        .unwrap();
    let mapped = buffer.slice(..).get_mapped_range();
    let result = mapped
        .chunks_exact(row as usize)
        .flat_map(|row| row[..texture.width() as usize * 4].iter().copied())
        .collect();
    drop(mapped);
    buffer.unmap();
    result
}

#[test]
#[ignore = "requires a WGPU adapter; qualify Metal, Vulkan, and DX12 separately"]
fn offscreen_preserves_alpha_clips_clean_frames_and_resize_lifetimes() {
    let descriptor = wgpu::InstanceDescriptor::from_env_or_default();
    eprintln!("offscreen instance backends: {:?}", descriptor.backends);
    let instance = wgpu::Instance::new(&descriptor);
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    eprintln!("offscreen adapter: {:?}", adapter.get_info());
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .unwrap();
    let (limited_device, limited_queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            required_limits: wgpu::Limits {
                max_storage_buffers_per_shader_stage: 0,
                ..GpuContext::required_limits()
            },
            ..Default::default()
        }))
        .unwrap();
    let failure = GpuContext::from_wgpu(
        instance.clone(),
        adapter.clone(),
        limited_device,
        limited_queue,
    );
    assert!(failure
        .err()
        .unwrap()
        .to_string()
        .contains("max_storage_buffers_per_shader_stage"));
    let context = GpuContext::from_wgpu(instance, adapter, device.clone(), queue.clone()).unwrap();
    for unsupported in [
        wgpu::TextureFormat::Depth32Float,
        wgpu::TextureFormat::Rgba8UnormSrgb,
        wgpu::TextureFormat::Bgra8UnormSrgb,
    ] {
        assert!(RenderBackend::to_texture(
            context.clone(),
            unsupported,
            RenderConfig::new(true, 32, 32, [1.0, 1.0])
        )
        .is_err());
    }
    for (format, red_channel) in [
        (wgpu::TextureFormat::Rgba8Unorm, 0),
        (wgpu::TextureFormat::Bgra8Unorm, 2),
    ] {
        let backend = RenderBackend::to_texture(
            context.clone(),
            format,
            RenderConfig::new(true, 32, 32, [1.0, 1.0]),
        )
        .unwrap();
        let mut renderer = WgpuRenderer::new(backend);
        renderer.begin_node(1, 0, 0);
        renderer.save();
        renderer.clip(rect(8.0, 8.0, 16.0, 16.0));
        renderer.fill_path(
            rect(0.0, 0.0, 32.0, 32.0),
            Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 0.5)),
        );
        renderer.restore();
        renderer.end_node(1);
        renderer.flush();
        let first = renderer.output_texture().unwrap();
        assert_eq!(first.sample_count(), 1, "copy exposes resolved output");
        assert!(first.usage().contains(wgpu::TextureUsages::COPY_SRC));
        let initial = pixels(&device, &queue, &first);
        assert_eq!(&initial[(2 * 32 + 2) * 4..][..4], &[0, 0, 0, 0]);
        let middle = &initial[(16 * 32 + 16) * 4..][..4];
        assert!(
            middle[red_channel].abs_diff(128) <= 1 && middle[3].abs_diff(128) <= 1,
            "premultiplied {format:?} output: {middle:?}"
        );
        assert_eq!(middle[1], 0);
        assert_eq!(middle[2 - red_channel], 0);
        renderer.flush();
        assert_eq!(
            pixels(&device, &queue, &renderer.output_texture().unwrap()),
            initial,
            "clean frame must preserve retained pixels"
        );
        renderer.resize(64.0, 48.0);
        renderer.flush();
        let second = renderer.output_texture().unwrap();
        assert_eq!((second.width(), second.height()), (64, 48));
        let resized = pixels(&device, &queue, &second);
        assert_eq!(
            &resized[(16 * 64 + 16) * 4..][..4],
            middle,
            "resize replays clean retained geometry into new backing"
        );
        assert_eq!(&resized[(40 * 64 + 60) * 4..][..4], &[0, 0, 0, 0]);
        drop(renderer);
        assert_eq!(
            pixels(&device, &queue, &first),
            initial,
            "old output remains usable after resize and renderer drop"
        );
    }
}

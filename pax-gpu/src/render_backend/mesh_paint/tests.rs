use super::*;

#[test]
fn mesh_shader_parses_and_validates() {
    let module = naga::front::wgsl::parse_str(include_str!("../mesh_paint.wgsl")).unwrap();
    naga::valid::Validator::new(
        naga::valid::ValidationFlags::all(),
        naga::valid::Capabilities::empty(),
    )
    .validate(&module)
    .unwrap();
    assert_eq!(std::mem::size_of::<Patch>(), 512);
    assert_eq!(std::mem::size_of::<Domain>(), 32);
}

#[test]
fn mesh_page_capacity_and_resolution_are_bounded() {
    let limits = wgpu::Limits::downlevel_webgl2_defaults();
    let input = MeshInput {
        rows: vec![],
        projected_axes: [[390.0, 0.0], [0.0, 256.0]],
        domain: [0.0, 0.0, 1.0, 1.0],
    };
    assert_eq!(
        page_extent(&[input.clone()], [1.0, 1.0], &limits, None).unwrap(),
        [416, 256, 1]
    );
    assert_eq!(
        page_extent(&[input.clone()], [2.0, 2.0], &limits, None).unwrap(),
        [800, 512, 1]
    );
    assert_eq!(
        page_extent(&[input.clone()], [1.0, 1.0], &limits, Some([448, 288])).unwrap(),
        [448, 288, 1]
    );
    assert!(page_extent(
        &vec![input.clone(); limits.max_texture_array_layers as usize + 1],
        [1.0, 1.0],
        &limits,
        None
    )
    .is_err());
    assert!(page_extent(&vec![input.clone(); 64], [2.0, 2.0], &limits, None).is_err());
    let mut rectangular = input.clone();
    rectangular.rows = vec![vec![Control([0.0; 8]); 3]; 3];
    let mut ragged = rectangular.clone();
    ragged.rows[1].pop();
    ragged.rows[2].push(Control([0.0; 8]));
    assert!(
        PageKey::new(&[rectangular], [32, 32, 1]) != PageKey::new(&[ragged], [32, 32, 1]),
        "row partitioning is part of cache identity"
    );
    let mut too_large = input;
    too_large.projected_axes[0][0] = f32::INFINITY;
    assert!(page_extent(&[too_large], [1.0, 1.0], &limits, None).is_err());
}

#[cfg(target_os = "macos")]
mod hardware {
    use super::*;

    fn fixture() -> Vec<Vec<Control>> {
        let colors = [
            [0.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 0.0, 1.0],
            [1.0, 1.0, 0.0],
            [0.0, 1.0, 1.0],
        ];
        (0..3)
            .map(|y| {
                (0..3)
                    .map(|x| {
                        let alpha = if x == 1 && y == 1 { 0.25 } else { 0.75 };
                        let c = colors[y * 3 + x];
                        let dx = if x == 1 && y == 1 { 0.13 } else { 0.0 };
                        let dy = if x == 1 && y == 1 { -0.09 } else { 0.0 };
                        Control([
                            10.0 + x as f32 * 0.5 + dx,
                            20.0 + y as f32 * 0.5 + dy,
                            c[0] * alpha,
                            c[1] * alpha,
                            c[2] * alpha,
                            alpha,
                            0.0,
                            0.0,
                        ])
                    })
                    .collect()
            })
            .collect()
    }

    fn device() -> (wgpu::Device, wgpu::Queue) {
        pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
                backends: wgpu::Backends::METAL,
                ..Default::default()
            });
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions::default())
                .await
                .unwrap();
            eprintln!("mesh prototype adapter: {:?}", adapter.get_info());
            adapter
                .request_device(&wgpu::DeviceDescriptor {
                    // Use the same conservative requested limits as the web renderer.
                    required_limits: wgpu::Limits {
                        max_storage_buffers_per_shader_stage: 1,
                        max_storage_buffer_binding_size: 128 * 1024 * 1024,
                        ..wgpu::Limits::downlevel_webgl2_defaults()
                    },
                    ..Default::default()
                })
                .await
                .unwrap()
        })
    }

    #[test]
    #[ignore = "requires a macOS Metal device"]
    fn mesh_pages_share_live_fields_and_copy_on_write() {
        let (device, queue) = device();
        let rasterizer = MeshRasterizer::new(&device);
        let layout = sampling_layout(&device);
        let input = MeshInput {
            rows: fixture(),
            projected_axes: [[128.0, 0.0], [0.0, 128.0]],
            domain: [10.0, 20.0, 1.0, 1.0],
        };
        let mut a = None;
        let mut b = None;
        let prepare = |page: &mut Option<Rc<PaintPage>>, inputs: &[MeshInput]| {
            let mut stats = RasterStats::default();
            let mut encoder = device.create_command_encoder(&Default::default());
            rasterizer
                .prepare_page(
                    &device,
                    &queue,
                    &mut encoder,
                    &layout,
                    page,
                    inputs,
                    [1.0, 1.0],
                    &mut stats,
                )
                .unwrap();
            queue.submit([encoder.finish()]);
            stats
        };
        assert_eq!(prepare(&mut a, &[input.clone()]).texture_allocations, 1);
        assert_eq!(prepare(&mut b, &[input.clone()]).texture_allocations, 0);
        assert!(Rc::ptr_eq(a.as_ref().unwrap(), b.as_ref().unwrap()));
        let original = read(&device, &queue, &a.as_ref().unwrap().texture);
        let mut changed = input.clone();
        changed.rows[1][1].0[2] = 0.2;
        assert_eq!(prepare(&mut b, &[changed.clone()]).texture_allocations, 1);
        assert!(!Rc::ptr_eq(a.as_ref().unwrap(), b.as_ref().unwrap()));
        assert_eq!(
            read(&device, &queue, &a.as_ref().unwrap().texture),
            original
        );
        changed.rows[1][1].0[2] = 0.1;
        assert_eq!(prepare(&mut b, &[changed]).texture_allocations, 0);
        assert_eq!(prepare(&mut a, &[input]).raster_passes, 0);
        // Identical animated consumers must not re-share and allocate on every frame.
        for frame in 0..4 {
            let mut together = MeshInput {
                rows: fixture(),
                projected_axes: [[128.0, 0.0], [0.0, 128.0]],
                domain: [10.0, 20.0, 1.0, 1.0],
            };
            together.rows[1][1].0[2] = 0.1 + frame as f32 * 0.02;
            assert_eq!(prepare(&mut a, &[together.clone()]).texture_allocations, 0);
            assert_eq!(prepare(&mut b, &[together]).texture_allocations, 0);
        }
        let weak = Rc::downgrade(a.as_ref().unwrap());
        prepare(&mut a, &[]);
        assert!(weak.upgrade().is_none(), "last live consumer releases page");
    }

    fn read(device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) -> Vec<u8> {
        let width = texture.width();
        let height = texture.height();
        let stride = (width * 4).div_ceil(256) * 256;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Mesh prototype readback"),
            size: u64::from(stride * height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        queue.submit([encoder.finish()]);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer.map_async(wgpu::MapMode::Read, .., move |result| {
            sender.send(result).unwrap()
        });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        receiver.recv().unwrap().unwrap();
        let mapped = buffer.get_mapped_range(..);
        mapped
            .chunks_exact(stride as usize)
            .flat_map(|row| row[..(width * 4) as usize].iter().copied())
            .collect()
    }

    fn raster(
        rasterizer: &MeshRasterizer,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: &mut Option<MeshTexture>,
        rows: &[Vec<Control>],
        resolution: [u32; 2],
        stats: &mut RasterStats,
    ) {
        let mut encoder = device.create_command_encoder(&Default::default());
        rasterizer
            .rasterize(
                device,
                queue,
                &mut encoder,
                slot,
                rows,
                [10.0, 20.0, 1.0, 1.0],
                resolution,
                stats,
            )
            .unwrap();
        queue.submit([encoder.finish()]);
    }

    /// Independent high-density CPU triangle raster, including row-major replacement at folds.
    fn reference(rows: &[Vec<Control>], size: usize) -> Vec<u8> {
        let patches = mesh::patches(rows).unwrap();
        let n = 256;
        let mut image = vec![0; size * size * 4];
        for patch in patches {
            let vertices: Vec<_> = (0..=n)
                .flat_map(|y| (0..=n).map(move |x| (x, y)))
                .map(|(x, y)| {
                    let mut value = patch.evaluate(x as f64 / n as f64, y as f64 / n as f64);
                    value[0] = (value[0] - 10.0) * size as f64;
                    value[1] = (value[1] - 20.0) * size as f64;
                    value
                })
                .collect();
            for y in 0..n {
                for x in 0..n {
                    let a = vertices[y * (n + 1) + x];
                    let b = vertices[y * (n + 1) + x + 1];
                    let c = vertices[(y + 1) * (n + 1) + x + 1];
                    let d = vertices[(y + 1) * (n + 1) + x];
                    triangle(&mut image, size, [a, b, c]);
                    triangle(&mut image, size, [a, c, d]);
                }
            }
        }
        image
    }

    fn triangle(image: &mut [u8], size: usize, v: [[f64; 8]; 3]) {
        let edge = |a: [f64; 8], b: [f64; 8], x: f64, y: f64| {
            (b[0] - a[0]) * (y - a[1]) - (b[1] - a[1]) * (x - a[0])
        };
        let area = edge(v[0], v[1], v[2][0], v[2][1]);
        if area.abs() < 1e-12 {
            return;
        }
        let min_x = v
            .iter()
            .map(|p| p[0])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .max(0.0) as usize;
        let max_x = v
            .iter()
            .map(|p| p[0])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .max(0.0)
            .min(size as f64) as usize;
        let min_y = v
            .iter()
            .map(|p| p[1])
            .fold(f64::INFINITY, f64::min)
            .floor()
            .max(0.0) as usize;
        let max_y = v
            .iter()
            .map(|p| p[1])
            .fold(f64::NEG_INFINITY, f64::max)
            .ceil()
            .max(0.0)
            .min(size as f64) as usize;
        for y in min_y..max_y {
            for x in min_x..max_x {
                let px = x as f64 + 0.5;
                let py = y as f64 + 0.5;
                let weights = [
                    edge(v[1], v[2], px, py) / area,
                    edge(v[2], v[0], px, py) / area,
                    edge(v[0], v[1], px, py) / area,
                ];
                if weights.iter().any(|w| *w < -1e-9) {
                    continue;
                }
                let value = |c| (0..3).map(|i| weights[i] * v[i][c]).sum::<f64>();
                let alpha = value(5).clamp(0.0, 1.0);
                for c in 0..4 {
                    image[(y * size + x) * 4 + c] =
                        (value(c + 2).clamp(0.0, alpha) * 255.0).round() as u8;
                }
            }
        }
    }

    #[test]
    #[ignore = "requires a macOS Metal device; retained paint workload measurement"]
    fn mesh_raster_workload() {
        let (device, queue) = device();
        let rasterizer = MeshRasterizer::new(&device);
        let layout = sampling_layout(&device);
        for size in [3, 8] {
            for consumers in [1, 16] {
                for width in [390, 920] {
                    for dpr in [1, 2] {
                        for mode in ["paused", "color", "position"] {
                            let mut slots: Vec<_> = (0..consumers).map(|_| None).collect();
                            let mut stats = RasterStats::default();
                            let mut cpu = Vec::new();
                            let mut complete = Vec::new();
                            for frame in 0..12 {
                                let start = std::time::Instant::now();
                                let mut encoder =
                                    device.create_command_encoder(&Default::default());
                                for (consumer, slot) in slots.iter_mut().enumerate() {
                                    let mut rows: Vec<Vec<Control>> = (0..size)
                                        .map(|y| {
                                            (0..size)
                                                .map(|x| {
                                                    let c = (x + y) % 3;
                                                    Control([
                                                        x as f32 / (size - 1) as f32,
                                                        y as f32 / (size - 1) as f32,
                                                        if c == 0 { 0.0 } else { 0.75 },
                                                        if c == 1 { 0.0 } else { 0.75 },
                                                        if c == 2 { 0.0 } else { 0.75 },
                                                        0.75,
                                                        0.0,
                                                        0.0,
                                                    ])
                                                })
                                                .collect()
                                        })
                                        .collect();
                                    // Distinct consumers measure independent fields, not the sharing fast path.
                                    rows[1][1].0[2] = 0.2 + consumer as f32 * 0.01;
                                    if mode == "color" {
                                        rows[1][1].0[2] += frame as f32 * 0.005;
                                    }
                                    if mode == "position" {
                                        rows[1][1].0[0] += frame as f32 * 0.001;
                                    }
                                    let input = MeshInput {
                                        rows,
                                        projected_axes: [[width as f32, 0.0], [0.0, 256.0]],
                                        domain: [0.0, 0.0, 1.0, 1.0],
                                    };
                                    rasterizer
                                        .prepare_page(
                                            &device,
                                            &queue,
                                            &mut encoder,
                                            &layout,
                                            slot,
                                            &[input],
                                            [dpr as f32; 2],
                                            &mut stats,
                                        )
                                        .unwrap();
                                }
                                cpu.push(start.elapsed().as_secs_f64() * 1000.0);
                                queue.submit([encoder.finish()]);
                                device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
                                complete.push(start.elapsed().as_secs_f64() * 1000.0);
                            }
                            let cold = complete.remove(0);
                            cpu.remove(0);
                            cpu.sort_by(f64::total_cmp);
                            complete.sort_by(f64::total_cmp);
                            let bytes: u64 = slots
                                .iter()
                                .map(|s| {
                                    let p = s.as_ref().unwrap();
                                    u64::from(p.texture.width()) * u64::from(p.texture.height()) * 4
                                })
                                .sum();
                            eprintln!("mesh workload grid={size}, consumers={consumers}, width={width}, dpr={dpr}, mode={mode}: cold={cold:.3}ms, CPU={:.3}ms, completion={:.3}ms, tail={:.3}ms, bytes={bytes}, stats={stats:?}", cpu[5], complete[5], complete[10]);
                            assert_eq!(stats.texture_allocations, consumers as u64);
                            assert_eq!(
                                stats.raster_passes,
                                consumers as u64 * if mode == "paused" { 1 } else { 12 }
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    #[ignore = "requires a macOS Metal device"]
    fn mesh_raster_pixels_and_retention() {
        let (device, queue) = device();
        let rasterizer = MeshRasterizer::new(&device);
        let mut rows = fixture();
        let mut slot = None;
        let mut stats = RasterStats::default();
        raster(
            &rasterizer,
            &device,
            &queue,
            &mut slot,
            &rows,
            [256; 2],
            &mut stats,
        );
        let pixels = read(&device, &queue, &slot.as_ref().unwrap().texture);
        let expected = reference(&rows, 256);
        let differences: Vec<_> = pixels
            .iter()
            .zip(&expected)
            .map(|(a, b)| a.abs_diff(*b))
            .collect();
        let max = *differences.iter().max().unwrap();
        let mean = differences.iter().map(|v| *v as f64).sum::<f64>() / differences.len() as f64;
        eprintln!(
            "3x3 curved translucent mesh: subdivision={}, max byte error={max}, mean={mean:.5}",
            slot.as_ref().unwrap().domain.subdivision
        );
        assert!(
            max <= 2,
            "high-resolution CPU comparison: max={max}, mean={mean}"
        );
        assert!(
            pixels.chunks_exact(4).all(|p| p[3] > 0),
            "no cracks in the covered domain"
        );
        if let Ok(dir) = std::env::var("PAX_MESH_PROTOTYPE_OUTPUT") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                std::path::Path::new(&dir).join("mesh-gpu-256.rgba"),
                &pixels,
            )
            .unwrap();
            std::fs::write(
                std::path::Path::new(&dir).join("mesh-cpu-256.rgba"),
                &expected,
            )
            .unwrap();
        }
        for _ in 0..4 {
            raster(
                &rasterizer,
                &device,
                &queue,
                &mut slot,
                &rows,
                [256; 2],
                &mut stats,
            );
        }
        assert_eq!(stats.raster_passes, 1);
        assert_eq!(stats.cache_hits, 4);
        let n = slot.as_ref().unwrap().domain.subdivision;
        for frame in 0..16 {
            rows[1][1].0[2] = frame as f32 / 64.0;
            raster(
                &rasterizer,
                &device,
                &queue,
                &mut slot,
                &rows,
                [256; 2],
                &mut stats,
            );
            assert_eq!(slot.as_ref().unwrap().domain.subdivision, n);
        }
        assert_eq!(stats.texture_allocations, 1);
        raster(
            &rasterizer,
            &device,
            &queue,
            &mut slot,
            &rows,
            [128; 2],
            &mut stats,
        );
        assert_eq!(stats.texture_allocations, 2);
        eprintln!("retention: {stats:?}");

        // Transparent colors carry no hidden hue, and moving boundaries leave transparent pixels.
        for row in &mut rows {
            for point in row {
                point.0[0] = 10.2 + (point.0[0] - 10.0) * 0.6;
                point.0[1] = 20.2 + (point.0[1] - 20.0) * 0.6;
                point.0[2..6].copy_from_slice(&[0.0, 0.0, 0.0, 0.0]);
            }
        }
        raster(
            &rasterizer,
            &device,
            &queue,
            &mut slot,
            &rows,
            [128; 2],
            &mut stats,
        );
        assert!(read(&device, &queue, &slot.as_ref().unwrap().texture)
            .iter()
            .all(|v| *v == 0));

        // Two fully overlapping patches; the later blue patch replaces translucent red.
        let fold = vec![
            vec![
                Control([10., 20., 0.5, 0., 0., 0.5, 0., 0.]),
                Control([11., 20., 0.5, 0., 0., 0.5, 0., 0.]),
                Control([10., 20., 0., 0., 0.5, 0.5, 0., 0.]),
            ],
            vec![
                Control([10., 21., 0.5, 0., 0., 0.5, 0., 0.]),
                Control([11., 21., 0.5, 0., 0., 0.5, 0., 0.]),
                Control([10., 21., 0., 0., 0.5, 0.5, 0., 0.]),
            ],
        ];
        raster(
            &rasterizer,
            &device,
            &queue,
            &mut slot,
            &fold,
            [128; 2],
            &mut stats,
        );
        let folded = read(&device, &queue, &slot.as_ref().unwrap().texture);
        assert!(
            folded
                .chunks_exact(4)
                .all(|p| (p[3] as i16 - 128).abs() <= 1),
            "overlaps must not accumulate alpha"
        );
        let left = &folded[(64 * 128 + 2) * 4..][..4];
        assert!(
            left[2] > left[0],
            "later patch replaces earlier patch: {left:?}"
        );
    }
}

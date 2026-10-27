use super::*;
use crate::engine::layer_surface::LayerSurfaceSize;
use api::RenderContext;

#[derive(Default)]
struct Frame {
    rgba: Vec<u8>,
    width: u32,
    uploads: Vec<(u32, u32, (u32, u32))>,
}
struct Surface(Rc<RefCell<Frame>>);
impl CpuSurface for Surface {
    fn configure(&mut self, size: (u32, u32)) {
        let mut frame = self.0.borrow_mut();
        if frame.width != size.0 || frame.rgba.len() != (size.0 * size.1 * 4) as usize {
            frame.width = size.0;
            frame.rgba = vec![0; (size.0 * size.1 * 4) as usize];
        }
    }
    fn present(&mut self, bytes: &[u8], width: u32, height: u32, origin: (u32, u32)) {
        let mut frame = self.0.borrow_mut();
        frame.uploads.push((width, height, origin));
        for y in 0..height {
            let from = (y * width * 4) as usize;
            let to = (((y + origin.1) * frame.width + origin.0) * 4) as usize;
            frame.rgba[to..to + (width * 4) as usize]
                .copy_from_slice(&bytes[from..from + (width * 4) as usize]);
        }
    }
}
fn renderer(
    origin: (f32, f32),
    size: (u32, u32),
    dpr: f32,
) -> (CpuRenderer<Surface>, Rc<RefCell<Frame>>) {
    let frame = Rc::new(RefCell::new(Frame::default()));
    let output = frame.clone();
    let mut renderer = CpuRenderer::new(move |_| {
        let entry = LayerSurfaceEntry {
            key: "tile".into(),
            host_signature: "host".into(),
            origin_x: origin.0,
            origin_y: origin.1,
            replay_priority: 0,
            surface: LayerSurfaceSize {
                logical_width: size.0 as f32 / dpr,
                logical_height: size.1 as f32 / dpr,
                surface_width: size.0,
                surface_height: size.1,
                dpr: [dpr; 2],
            },
        };
        let target = CpuLayerTarget::new(
            vec![CpuLayerRenderer::new(
                entry.key.clone(),
                entry.host_signature.clone(),
                Surface(output.clone()),
                &entry,
            )],
            true,
        );
        (
            target,
            Box::new(move || LayerSurfaceLayout {
                surfaces: vec![entry.clone()],
                active: true,
            }),
        )
    });
    renderer.resize_layers_to(1, Rc::new(RefCell::new(vec![true])));
    (renderer, frame)
}
fn fill(r: &mut CpuRenderer<Surface>, rect: Rect, color: api::Color, opacity: f64) {
    r.fill_with_opacity(0, rect.to_path(0.1), &Paint::Solid(color), opacity);
}
fn flush(r: &mut CpuRenderer<Surface>) {
    r.flush(0, Rc::new(RefCell::new(vec![true])));
}
fn pixel(frame: &Rc<RefCell<Frame>>, x: usize, y: usize) -> [u8; 4] {
    let f = frame.borrow();
    f.rgba[(y * f.width as usize + x) * 4..][..4]
        .try_into()
        .unwrap()
}
fn near(value: u8, expected: u8) {
    assert!(value.abs_diff(expected) <= 2, "{value} != {expected}");
}
fn scope(r: &mut CpuRenderer<Surface>, id: u32, opacity: f32) {
    r.set_node_opacity_scopes(
        0,
        id,
        &[OpacityScope {
            node_id: id,
            opacity,
        }],
    );
}

#[test]
fn group_opacity_composes_overlaps_once_and_preserves_siblings() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    scope(&mut r, 1, 0.5);
    fill(
        &mut r,
        Rect::new(0., 0., 40., 40.),
        api::Color::from_rgba_0_1([1., 0., 0., 1.]),
        1.,
    );
    fill(
        &mut r,
        Rect::new(20., 20., 60., 60.),
        api::Color::from_rgba_0_1([0., 0., 1., 1.]),
        1.,
    );
    r.set_node_opacity_scopes(0, 2, &[]);
    fill(&mut r, Rect::new(60., 0., 64., 10.), api::Color::WHITE, 1.);
    flush(&mut r);
    near(pixel(&f, 30, 30)[3], 128);
    near(pixel(&f, 5, 5)[3], 128);
    assert_eq!(pixel(&f, 62, 5), [255; 4]);
}

#[test]
fn pixel_conversion_preserves_alpha_and_matches_integer_unpremultiplication() {
    // Exercise both the SIMD body and the scalar tail.
    let mut scratch = RasterScratch::take(&mut Vec::new(), 257, 255);
    for (i, pixel) in scratch.pixels.data_mut().iter_mut().enumerate() {
        let alpha = (i / 256) as u8;
        let value = (i % 256) as u8;
        *pixel = vello_cpu::color::PremulRgba8 {
            r: value.min(alpha),
            g: value.min(alpha),
            b: value.min(alpha),
            a: alpha,
        };
    }
    for (i, pixel) in scratch.straight_rgba().chunks_exact(4).enumerate() {
        let alpha = (i / 256) as u32;
        let value = (i as u32 % 256).min(alpha);
        let expected = if alpha == 0 {
            0
        } else {
            ((value * 255 + alpha / 2) / alpha) as u8
        };
        assert_eq!(pixel[3], alpha as u8);
        assert!(pixel[..3]
            .iter()
            .all(|channel| channel.abs_diff(expected) <= 1));
    }
    let capacity = scratch.pixels.capacity();
    let mut smaller = RasterScratch::take(&mut vec![scratch], 32, 32);
    smaller.rasterize();
    assert_eq!(smaller.pixels.capacity(), capacity);
    assert!(smaller.pixels.data_as_u8_slice().iter().all(|v| *v == 0));
}

#[test]
fn unfeathered_mask_preserves_alpha_across_cropped_tiles_and_densities() {
    let draw_source = |r: &mut CpuRenderer<Surface>| {
        for alpha in 0..256 {
            let x = 3. + f64::from(alpha);
            fill(
                r,
                Rect::new(x, 5., x + 1., 13.),
                api::Color::WHITE,
                f64::from(alpha) / 255.,
            );
        }
    };
    for (origin, dpr) in [((0., 0.), 1.), ((9.25, 8.5), 1.25), ((-5.5, -3.25), 2.)] {
        let (mut masked, mask_frame) = renderer(origin, (300, 64), dpr);
        let (mut direct, direct_frame) = renderer(origin, (300, 64), dpr);
        // Every alpha byte occurs in the source; the other tiles crop it and
        // rasterize at fractional positions, exercising antialiased coverage.
        masked.begin_alpha_source(0, Affine::IDENTITY, 0, 0.);
        draw_source(&mut masked);
        masked.end_alpha_source(0);
        fill(
            &mut masked,
            Rect::new(-100., -100., 500., 500.),
            api::Color::WHITE,
            1.,
        );
        flush(&mut masked);

        draw_source(&mut direct);
        flush(&mut direct);
        if origin == (0., 0.) {
            for alpha in 0..256 {
                assert_eq!(pixel(&direct_frame, 3 + alpha, 8)[3], alpha as u8);
            }
        }
        for (actual, expected) in mask_frame
            .borrow()
            .rgba
            .chunks_exact(4)
            .zip(direct_frame.borrow().rgba.chunks_exact(4))
        {
            assert_eq!(actual[3], expected[3], "origin={origin:?}, dpr={dpr}");
        }
    }
}

#[test]
fn partial_replay_preserves_full_frame_mask_capture_limits() {
    for (width, feather, nested) in [(4100, 0., false), (4096, 1., false), (4096, 1., true)] {
        let (mut r, frame) = renderer((0., 0.), (width, 16), 1.);
        for opacity in [1., 0.5] {
            r.clear(0);
            if nested {
                r.begin_alpha_source(0, Affine::IDENTITY, 0, 0.);
            }
            r.begin_alpha_source(0, Affine::IDENTITY, 0, feather);
            fill(
                &mut r,
                Rect::new(0., 0., width as f64, 16.),
                api::Color::WHITE,
                1.,
            );
            r.end_alpha_source(0);
            if nested {
                fill(
                    &mut r,
                    Rect::new(0., 0., width as f64, 16.),
                    api::Color::WHITE,
                    1.,
                );
                r.end_alpha_source(0);
            }
            fill(
                &mut r,
                Rect::new(100., 2., 110., 14.),
                api::Color::WHITE,
                opacity,
            );
            flush(&mut r);
            assert!(frame.borrow().rgba.iter().all(|value| *value == 0));
        }
        assert!(frame.borrow().uploads[1].0 < 32);
    }
}

#[test]
fn warm_window_changes_reuse_matching_canvas_history_but_not_replaced_hosts() {
    let entry = |key: &str, host: &str, x| LayerSurfaceEntry {
        key: key.into(),
        host_signature: host.into(),
        origin_x: x,
        origin_y: 0.,
        replay_priority: 0,
        surface: LayerSurfaceSize {
            logical_width: 64.,
            logical_height: 64.,
            surface_width: 64,
            surface_height: 64,
            dpr: [1.; 2],
        },
    };
    let first = entry("a", "host-a", 0.);
    let second = entry("b", "host-b", 64.);
    let layout = Rc::new(RefCell::new(LayerSurfaceLayout {
        surfaces: vec![first.clone()],
        active: true,
    }));
    let outputs = Rc::new(RefCell::new(HashMap::<String, Rc<RefCell<Frame>>>::new()));
    let current = layout.clone();
    let frames = outputs.clone();
    let mut r = CpuRenderer::new(move |_| {
        let renderers = current
            .borrow()
            .surfaces
            .iter()
            .map(|entry| {
                let frame = frames
                    .borrow_mut()
                    .entry(entry.host_signature.clone())
                    .or_default()
                    .clone();
                CpuLayerRenderer::new(
                    entry.key.clone(),
                    entry.host_signature.clone(),
                    Surface(frame),
                    entry,
                )
            })
            .collect();
        let provider = current.clone();
        (
            CpuLayerTarget::new(renderers, true),
            Box::new(move || provider.borrow().clone()) as Box<dyn Fn() -> LayerSurfaceLayout>,
        )
    });
    r.resize_layers_to(1, Rc::new(RefCell::new(vec![true])));
    let paint = |r: &mut CpuRenderer<Surface>| {
        r.clear(0);
        fill(r, Rect::new(0., 0., 128., 64.), api::Color::WHITE, 1.);
        flush(r);
    };
    paint(&mut r);
    layout.borrow_mut().surfaces = vec![second.clone(), first.clone()];
    r.refresh_layers(&[0]);
    paint(&mut r);
    assert_eq!(outputs.borrow()["host-a"].borrow().uploads.len(), 1);
    assert_eq!(outputs.borrow()["host-b"].borrow().uploads.len(), 1);
    layout.borrow_mut().surfaces = vec![first, entry("b", "replacement-b", 64.)];
    r.refresh_layers(&[0]);
    paint(&mut r);
    assert_eq!(outputs.borrow()["host-a"].borrow().uploads.len(), 1);
    assert_eq!(outputs.borrow()["replacement-b"].borrow().uploads.len(), 1);
    assert_eq!(pixel(&outputs.borrow()["replacement-b"], 32, 32), [255; 4]);
}

#[test]
fn mask_applies_to_each_draw_and_restores_for_following_content() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    r.save(0);
    r.begin_alpha_source(0, Affine::IDENTITY, 0, 0.);
    fill(&mut r, Rect::new(0., 0., 64., 64.), api::Color::WHITE, 0.5);
    r.end_alpha_source(0);
    fill(
        &mut r,
        Rect::new(0., 0., 40., 40.),
        api::Color::from_rgba_0_1([1., 0., 0., 1.]),
        1.,
    );
    fill(
        &mut r,
        Rect::new(20., 20., 60., 60.),
        api::Color::from_rgba_0_1([1., 0., 0., 1.]),
        1.,
    );
    r.restore(0);
    fill(&mut r, Rect::new(60., 0., 64., 10.), api::Color::WHITE, 1.);
    flush(&mut r);
    near(pixel(&f, 30, 30)[3], 192);
    near(pixel(&f, 5, 5)[3], 128);
    assert_eq!(pixel(&f, 62, 5)[3], 255);
}

#[test]
fn source_group_opacity_is_independent_of_consumer_scope_and_clip() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    scope(&mut r, 1, 0.5);
    r.clip(0, Rect::new(0., 0., 50., 50.).to_path(0.1));
    r.begin_alpha_source(0, Affine::IDENTITY, 1, 0.);
    r.set_node_opacity_scopes(
        0,
        3,
        &[
            OpacityScope {
                node_id: 1,
                opacity: 0.5,
            },
            OpacityScope {
                node_id: 2,
                opacity: 0.5,
            },
        ],
    );
    fill(&mut r, Rect::new(0., 0., 40., 40.), api::Color::WHITE, 1.);
    fill(&mut r, Rect::new(20., 20., 60., 60.), api::Color::WHITE, 1.);
    r.end_alpha_source(0);
    scope(&mut r, 1, 0.5);
    fill(
        &mut r,
        Rect::new(0., 0., 64., 64.),
        api::Color::from_rgba_0_1([1., 0., 0., 1.]),
        1.,
    );
    flush(&mut r);
    near(pixel(&f, 30, 30)[3], 64);
    near(pixel(&f, 5, 5)[3], 64);
    assert_eq!(pixel(&f, 55, 55)[3], 0);
}

#[test]
fn nested_masks_multiply_coverage_without_moving_the_source() {
    let (mut r, f) = renderer((10., 20.), (64, 64), 2.);
    r.begin_alpha_source(0, Affine::translate((10., 20.)), 0, 0.);
    r.begin_alpha_source(0, Affine::IDENTITY, 0, 0.);
    fill(&mut r, Rect::new(0., 0., 20., 20.), api::Color::WHITE, 0.5);
    r.end_alpha_source(0);
    fill(&mut r, Rect::new(0., 0., 30., 30.), api::Color::WHITE, 0.5);
    r.end_alpha_source(0);
    fill(
        &mut r,
        Rect::new(10., 20., 42., 52.),
        api::Color::from_rgba_0_1([1., 0., 0., 1.]),
        1.,
    );
    flush(&mut r);
    near(pixel(&f, 10, 10)[3], 64);
    assert_eq!(pixel(&f, 50, 50)[3], 0);
}

#[test]
fn clips_capture_their_transform_and_saves_restore_it() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    r.save(0);
    r.transform(0, Affine::translate((10., 20.)));
    r.clip(0, Rect::new(0., 0., 10., 10.).to_path(0.1));
    r.transform(0, Affine::translate((-10., -20.)));
    fill(
        &mut r,
        Rect::new(0., 0., 64., 64.),
        api::Color::from_rgba_0_1([1., 0., 0., 1.]),
        1.,
    );
    r.restore(0);
    fill(
        &mut r,
        Rect::new(40., 0., 50., 10.),
        api::Color::from_rgba_0_1([0., 0., 1., 1.]),
        1.,
    );
    flush(&mut r);
    assert_eq!(pixel(&f, 15, 25), [255, 0, 0, 255]);
    assert_eq!(pixel(&f, 5, 5)[3], 0);
    assert_eq!(pixel(&f, 45, 5), [0, 0, 255, 255]);
}

#[test]
fn image_opacity_does_not_mutate_cached_pixels() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    r.load_image("test", &[255, 0, 0, 128], 1, 1);
    scope(&mut r, 1, 0.5);
    r.draw_image_with_opacity(0, "test", Rect::new(0., 0., 10., 10.), 0.5);
    r.set_node_opacity_scopes(0, 2, &[]);
    r.draw_image_with_opacity(0, "test", Rect::new(20., 0., 30., 10.), 1.);
    flush(&mut r);
    near(pixel(&f, 5, 5)[3], 32);
    near(pixel(&f, 25, 5)[3], 128);
    near(pixel(&f, 5, 5)[0], 255);
}

#[test]
fn paint_mixture_applies_coverage_and_opacity_once() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    let paint = Paint::Blend(vec![
        (
            Paint::Solid(api::Color::from_rgba_0_1([1., 0., 0., 1.])),
            0.5,
        ),
        (
            Paint::Solid(api::Color::from_rgba_0_1([0., 0., 1., 1.])),
            0.5,
        ),
    ]);
    scope(&mut r, 1, 0.5);
    r.fill_with_opacity(0, Rect::new(0., 0., 40., 40.).to_path(0.1), &paint, 1.);
    flush(&mut r);
    let p = pixel(&f, 20, 20);
    near(p[0], 128);
    near(p[2], 128);
    near(p[3], 128);
}

#[test]
fn clearing_and_reusing_raster_storage_does_not_retain_old_pixels() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    fill(
        &mut r,
        Rect::new(0., 0., 64., 64.),
        api::Color::from_rgba_0_1([1., 0., 0., 1.]),
        1.,
    );
    flush(&mut r);
    r.clear(0);
    flush(&mut r);
    assert!(f.borrow().rgba.iter().all(|v| *v == 0));
    assert_eq!(r.rasters.len(), 1);
}

#[test]
fn feather_halo_matches_across_tile_boundaries() {
    fn draw(origin: f32, width: u32) -> Rc<RefCell<Frame>> {
        let (mut r, f) = renderer((origin, 0.), (width, 32), 1.);
        r.begin_alpha_source(0, Affine::IDENTITY, 0, 3.);
        fill(&mut r, Rect::new(28., 5., 36., 27.), api::Color::WHITE, 1.);
        r.end_alpha_source(0);
        fill(
            &mut r,
            Rect::new(0., 0., 64., 32.),
            api::Color::from_rgba_0_1([1., 0., 0., 1.]),
            1.,
        );
        flush(&mut r);
        f
    }
    let full = draw(0., 64);
    let left = draw(0., 32);
    let right = draw(32., 32);
    for y in 0..32 {
        for x in 0..64 {
            let tile = if x < 32 { &left } else { &right };
            near(pixel(&full, x, y)[3], pixel(tile, x % 32, y)[3]);
        }
    }
    assert!(pixel(&full, 25, 16)[3] > 0);
    assert!(pixel(&full, 32, 16)[3] > 200);
}

#[test]
fn empty_or_oversized_mask_hides_content_without_a_panic() {
    for feather in [0.0, 100_000.0, f64::INFINITY] {
        let (mut r, f) = renderer((0., 0.), (32, 32), 1.);
        r.begin_alpha_source(0, Affine::IDENTITY, 0, feather);
        if feather > 0.0 {
            fill(&mut r, Rect::new(0., 0., 32., 32.), api::Color::WHITE, 1.);
        }
        r.end_alpha_source(0);
        fill(&mut r, Rect::new(0., 0., 32., 32.), api::Color::WHITE, 1.);
        flush(&mut r);
        assert!(f.borrow().rgba.iter().all(|v| *v == 0));
    }
}

#[test]
fn image_cache_is_available_before_any_surface_exists() {
    let mut r = CpuRenderer::<Surface>::new(|_| panic!("no surfaces requested"));
    r.load_image("early", &[255, 255, 255, 128], 1, 1);
    assert!(r.image_loaded("early"));
    assert_eq!(r.get_image_size("early"), Some((1, 1)));
}

#[test]
fn source_clips_are_captured_in_source_coordinates() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    r.begin_alpha_source(0, Affine::translate((20., 10.)), 0, 0.);
    r.clip(0, Rect::new(0., 0., 10., 10.).to_path(0.1));
    fill(&mut r, Rect::new(0., 0., 64., 64.), api::Color::WHITE, 1.);
    r.end_alpha_source(0);
    fill(&mut r, Rect::new(0., 0., 64., 64.), api::Color::WHITE, 1.);
    flush(&mut r);
    assert_eq!(pixel(&f, 25, 15)[3], 255);
    assert_eq!(pixel(&f, 5, 5)[3], 0);
    assert_eq!(pixel(&f, 35, 15)[3], 0);
}

#[test]
fn trimmed_stroke_keeps_full_path_gradient_coordinates() {
    let (mut r, f) = renderer((0., 0.), (100, 32), 1.);
    let paint = Paint::LinearGradient(api::LinearGradient {
        start: (api::Size::Percent(0.into()), api::Size::Percent(0.into())),
        end: (api::Size::Percent(100.into()), api::Size::Percent(0.into())),
        stops: vec![
            api::GradientStop::get(api::Color::BLACK, api::Size::Percent(0.into())),
            api::GradientStop::get(api::Color::WHITE, api::Size::Percent(100.into())),
        ],
    });
    let stroke = ResolvedStroke {
        paint,
        width: api::Size::Pixels(8.into()),
        cap: api::StrokeCap::Butt,
        ..api::Stroke::default().snapshot()
    };
    let mut path = BezPath::new();
    path.move_to((10., 16.));
    path.line_to((90., 16.));
    r.stroke_with_draw_range_and_material_and_opacity(
        0,
        path,
        &stroke,
        &stroke.material,
        1.,
        0.25,
        0.75,
    );
    flush(&mut r);
    assert_eq!(pixel(&f, 20, 16)[3], 0);
    assert_eq!(pixel(&f, 80, 16)[3], 0);
    assert_eq!(pixel(&f, 50, 16)[3], 255);
    near(pixel(&f, 50, 16)[0], 129);
    assert!(pixel(&f, 32, 16)[0] > 60 && pixel(&f, 32, 16)[0] < 85);
}

#[test]
fn small_alpha_sources_do_not_allocate_a_tile_sized_capture() {
    let (mut r, f) = renderer((0., 0.), (1024, 1024), 1.);
    r.begin_alpha_source(0, Affine::IDENTITY, 0, 0.);
    fill(
        &mut r,
        Rect::new(400., 400., 420., 420.),
        api::Color::WHITE,
        1.,
    );
    r.end_alpha_source(0);
    fill(
        &mut r,
        Rect::new(0., 0., 1024., 1024.),
        api::Color::WHITE,
        1.,
    );
    flush(&mut r);
    assert_eq!(pixel(&f, 410, 410)[3], 255);
    assert_eq!(pixel(&f, 390, 410)[3], 0);
    assert!(r
        .rasters
        .iter()
        .any(|scratch| scratch.pixels.width() < 64 && scratch.pixels.height() < 64));
}

#[test]
fn clean_flush_preserves_pixels_and_incremental_draws_keep_prior_content() {
    let (mut r, f) = renderer((0., 0.), (64, 32), 1.);
    fill(&mut r, Rect::new(0., 0., 20., 20.), api::Color::WHITE, 1.);
    flush(&mut r);
    flush(&mut r);
    assert_eq!(pixel(&f, 10, 10), [255; 4]);
    fill(&mut r, Rect::new(32., 0., 52., 20.), api::Color::WHITE, 0.5);
    flush(&mut r);
    assert_eq!(pixel(&f, 10, 10), [255; 4]);
    near(pixel(&f, 42, 10)[3], 128);
    r.clear(0);
    flush(&mut r);
    assert!(f.borrow().rgba.iter().all(|v| *v == 0));
}

#[test]
fn replay_only_uploads_damage_and_erases_removed_content() {
    let (mut r, f) = renderer((0., 0.), (256, 256), 1.);
    let draw = |r: &mut CpuRenderer<Surface>, x| {
        fill(r, Rect::new(0., 0., 256., 256.), api::Color::BLACK, 1.);
        fill(r, Rect::new(x, 30., x + 10., 40.), api::Color::WHITE, 1.);
    };
    draw(&mut r, 20.);
    flush(&mut r);
    for _ in 0..2 {
        r.clear(0);
        draw(&mut r, 20.);
        flush(&mut r);
    }
    assert_eq!(f.borrow().uploads.len(), 1);
    r.clear(0);
    // An abandoned replay must not replace the last presented command list.
    draw(&mut r, 150.);
    r.clear(0);
    draw(&mut r, 40.);
    flush(&mut r);
    assert_eq!(f.borrow().uploads[1], (32, 12, (19, 29)));
    assert_eq!(pixel(&f, 25, 35), [0, 0, 0, 255]);
    assert_eq!(pixel(&f, 45, 35), [255; 4]);
    assert_eq!(pixel(&f, 200, 200), [0, 0, 0, 255]);
    r.clear(0);
    fill(&mut r, Rect::new(0., 0., 256., 256.), api::Color::BLACK, 1.);
    flush(&mut r);
    assert_eq!(f.borrow().uploads[2], (12, 12, (39, 29)));
    assert_eq!(pixel(&f, 45, 35), [0, 0, 0, 255]);
    r.clear(0);
    flush(&mut r);
    assert!(f.borrow().rgba.iter().all(|v| *v == 0));
}

#[test]
fn partial_replay_matches_full_compositing_with_masks_clips_and_opacity() {
    fn draw(r: &mut CpuRenderer<Surface>, phase: usize) {
        fill(r, Rect::new(0., 0., 128., 128.), api::Color::BLACK, 1.);
        r.save(0);
        r.transform(0, Affine::translate((32.25, 23.5)) * Affine::rotate(0.13));
        r.clip(0, Rect::new(0., 0., 45., 40.).to_path(0.1));
        scope(r, 8, if phase == 3 { 0.3 } else { 0.7 });
        r.begin_alpha_source(0, Affine::IDENTITY, 0, if phase == 2 { 2. } else { 1. });
        fill(
            r,
            Rect::new(28., 20., 60. + phase as f64, 65.),
            api::Color::WHITE,
            0.6,
        );
        r.end_alpha_source(0);
        fill(r, Rect::new(2., 4., 25., 28.), api::Color::WHITE, 0.8);
        fill(r, Rect::new(15., 12., 40., 38.), api::Color::WHITE, 0.8);
        r.restore(0);
        scope(r, 9, 1.);
        let stroke = ResolvedStroke {
            width: api::Size::Pixels(3.into()),
            paint: Paint::Solid(api::Color::WHITE),
            join: api::StrokeJoin::Miter,
            cap: api::StrokeCap::Square,
            ..api::Stroke::default().snapshot()
        };
        let mut path = BezPath::new();
        path.move_to((62., 60.));
        path.line_to((72., 38. + phase as f64));
        path.line_to((73., 61.));
        r.stroke_with_opacity(0, path, &stroke, 0.8);
    }
    let (mut retained, frame) = renderer((2.25, 1.5), (160, 160), 1.25);
    for phase in [0, 0, 1, 2, 3, 0] {
        retained.clear(0);
        draw(&mut retained, phase);
        flush(&mut retained);
        let (mut fresh, expected) = renderer((2.25, 1.5), (160, 160), 1.25);
        draw(&mut fresh, phase);
        flush(&mut fresh);
        for (actual, expected) in frame.borrow().rgba.iter().zip(&expected.borrow().rgba) {
            near(*actual, *expected);
        }
    }
    assert_eq!(frame.borrow().uploads.len(), 5);
    assert!(frame.borrow().uploads[1..]
        .iter()
        .all(|(w, h, _)| *w < 160 && *h < 160));
}

#[test]
fn replay_notices_reloaded_images_and_layout_changes() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    let rect = Rect::new(20., 20., 40., 40.);
    r.load_image("image", &[255, 255, 255, 255], 1, 1);
    r.draw_image(0, "image", rect);
    flush(&mut r);
    r.clear(0);
    r.draw_image(0, "image", rect);
    flush(&mut r);
    assert_eq!(f.borrow().uploads.len(), 1);
    r.load_image("image", &[255, 0, 0, 255], 1, 1);
    r.clear(0);
    r.draw_image(0, "image", rect);
    flush(&mut r);
    assert_eq!(pixel(&f, 30, 30), [255, 0, 0, 255]);
    let mut entry = (r.layers[0].1)().surfaces.remove(0);
    for (origin, dpr) in [(10., 1.), (10., 1.5)] {
        entry.origin_x = origin;
        entry.surface.dpr = [dpr; 2];
        r.layers[0].0.renderers[0].update_layout(&entry);
        r.clear(0);
        r.draw_image(0, "image", rect);
        flush(&mut r);
        assert_eq!(f.borrow().uploads.last(), Some(&(64, 64, (0, 0))));
        assert_eq!(pixel(&f, if dpr == 1. { 35 } else { 50 }, 35), [0; 4]);
        assert_eq!(pixel(&f, 25, 35), [255, 0, 0, 255]);
    }
}

#[test]
fn damage_includes_unchanged_draws_when_opacity_groups_join_or_split() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    for middle_scope in [2, 1, 2] {
        r.clear(0);
        scope(&mut r, 1, 0.5);
        fill(&mut r, Rect::new(10., 10., 40., 40.), api::Color::WHITE, 1.);
        scope(&mut r, middle_scope, 0.5);
        fill(&mut r, Rect::new(50., 50., 60., 60.), api::Color::WHITE, 1.);
        scope(&mut r, 1, 0.5);
        fill(&mut r, Rect::new(20., 20., 45., 45.), api::Color::WHITE, 1.);
        flush(&mut r);
        near(
            pixel(&f, 30, 30)[3],
            if middle_scope == 1 { 128 } else { 192 },
        );
    }
}

#[test]
fn mesh_fallback_averages_premultiplied_colors_and_preserves_opacity() {
    let (mut r, f) = renderer((0., 0.), (32, 32), 1.);
    let paint = Paint::MeshGradient(api::MeshGradient {
        rows: (0..2)
            .map(|y| {
                (0..2)
                    .map(|x| api::MeshPoint {
                        position: (
                            api::Size::Percent((x * 100).into()),
                            api::Size::Percent((y * 100).into()),
                        ),
                        color: api::Color::from_rgba_0_1(if x == 0 {
                            [1., 0., 0., 1.]
                        } else {
                            [0., 0., 1., 0.]
                        }),
                    })
                    .collect()
            })
            .collect(),
    });
    r.fill_with_opacity(0, Rect::new(0., 0., 32., 32.).to_path(0.1), &paint, 0.5);
    flush(&mut r);
    let p = pixel(&f, 16, 16);
    near(p[0], 255);
    near(p[2], 0);
    near(p[3], 64);
}

#[test]
fn radial_gradient_alpha_uses_local_pixel_radius_in_source_capture() {
    let (mut r, f) = renderer((0., 0.), (64, 64), 1.);
    let point = (api::Size::Percent(50.into()), api::Size::Percent(50.into()));
    let paint = Paint::RadialGradient(api::RadialGradient {
        start: point,
        end: point,
        radius: 20.,
        stops: vec![
            api::GradientStop::get(api::Color::WHITE, api::Size::Pixels(0.into())),
            api::GradientStop::get(api::Color::TRANSPARENT, api::Size::Pixels(20.into())),
        ],
    });
    r.begin_alpha_source(0, Affine::IDENTITY, 0, 0.);
    r.fill_with_opacity(0, Rect::new(0., 0., 64., 64.).to_path(0.1), &paint, 1.);
    r.end_alpha_source(0);
    fill(&mut r, Rect::new(0., 0., 64., 64.), api::Color::WHITE, 1.);
    flush(&mut r);
    assert!(pixel(&f, 32, 32)[3] > 240);
    assert!(pixel(&f, 42, 32)[3] > 110 && pixel(&f, 42, 32)[3] < 130);
    assert_eq!(pixel(&f, 55, 32)[3], 0);
}

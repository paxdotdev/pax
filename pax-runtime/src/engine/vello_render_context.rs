//! Portable software drawing and compositing, with chassis-owned presentation.

use kurbo::{Affine, BezPath, Rect, Shape};
use pax_runtime_api::{OpacityScope, Paint, ResolvedStroke};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::Arc,
};
use vello_cpu::{
    color::{AlphaColor, Srgb},
    kurbo as vk,
    peniko::{
        BlendMode, ColorStop, ColorStops, Compose, Gradient, ImageSampler, InterpolationAlphaSpace,
        LinearGradientPosition, RadialGradientPosition,
    },
    Image, ImageSource, Mask, PaintType, Pixmap, RenderContext as VelloContext, Resources,
};

use super::layer_surface::{
    replay_batches_by_directional_priority, surface_intersects_coverage_bounds, LayerSurfaceEntry,
    LayerSurfaceLayout, LayoutChangeKind, ReplayPriorityEntry, SurfaceReplayCoordinator,
};
#[cfg(debug_assertions)]
use super::layer_surface::{visible_surface_escape, VisibleSurfaceEscape};
use crate::api;

#[cfg(test)]
#[path = "vello_compositing_tests.rs"]
mod compositing_tests;

#[cfg(debug_assertions)]
fn log_visible_surface_escape(layer: usize, escape: VisibleSurfaceEscape) {
    log::warn!("[pax-tile-window-escape] backend=cpu layer={} visible_surfaces={} escaped_visible_surfaces={} max_gap_x={:.1} max_gap_y={:.1} previous_bounds={:?} visible_bounds={:?}", layer, escape.visible_surfaces, escape.escaped_visible_surfaces, escape.max_gap_x, escape.max_gap_y, escape.previous_bounds, escape.visible_bounds);
}

/// A presentation target for Vello CPU. Rasterization and intermediate surfaces
/// belong to the runtime; the chassis only installs the completed pixels.
pub trait CpuSurface {
    /// Configure backing dimensions when a surface assignment changes.
    fn configure(&mut self, size: (u32, u32));
    /// Replace the rectangle at `origin` (in backing pixels) with tightly packed,
    /// straight-alpha RGBA8 pixels, preserving pixels outside it. The slice is
    /// borrowed only for this call; targets must not retain a pointer into it.
    fn present(&mut self, rgba: &[u8], width: u32, height: u32, origin: (u32, u32));
}

type LayerDef<S> = (CpuLayerTarget<S>, Box<dyn Fn() -> LayerSurfaceLayout>);

#[derive(Clone, Default)]
struct DrawState {
    transform: Affine,
    // Freeze clip coordinates when installed, independently of later transforms.
    clips: Vec<Rc<BezPath>>,
    masks: Vec<Rc<AlphaSource>>,
}

struct AlphaSource {
    draws: Vec<CpuDraw>,
    mapping: Affine,
    feather: f64,
}

struct SourceCapture {
    state: DrawState,
    saves: Vec<DrawState>,
    scopes: Vec<OpacityScope>,
    draws: Vec<CpuDraw>,
    mapping: Affine,
    feather: f64,
}

enum CpuPaint {
    Fill(BezPath, Paint, f64, Rect),
    Stroke(BezPath, ResolvedStroke, f64, Rect),
    Image(Arc<Pixmap>, Rect, f64),
}

struct CpuDraw {
    state: DrawState,
    scopes: Vec<OpacityScope>,
    paint: CpuPaint,
    coverage: Option<Rect>,
}

impl CpuDraw {
    fn new(state: DrawState, scopes: Vec<OpacityScope>, paint: CpuPaint) -> Self {
        let coverage = match &paint {
            CpuPaint::Fill(path, _, opacity, _) if *opacity > 0.0 => Some(path.bounding_box()),
            // Keep the full stroke envelope stable across animated draw ranges.
            // The miter limit is four half-widths.
            CpuPaint::Stroke(_, stroke, opacity, bounds) if *opacity > 0.0 => {
                Some(bounds.inflate(stroke.width_pixels() * 2.0, stroke.width_pixels() * 2.0))
            }
            CpuPaint::Image(_, rect, opacity) if *opacity > 0.0 => Some(*rect),
            _ => None,
        };
        Self {
            state,
            scopes,
            paint,
            coverage,
        }
    }
}

// Compare values, not hashes: a collision must never leave stale pixels. Captured
// masks form a shared DAG, so compare each source pair only once per replay.
#[derive(Default)]
struct ReplayComparison {
    equal_sources: HashSet<(usize, usize)>,
}

impl ReplayComparison {
    fn draw_eq(&mut self, a: &CpuDraw, b: &CpuDraw) -> bool {
        a.scopes == b.scopes
            && a.state.transform == b.state.transform
            && a.state.clips == b.state.clips
            && self.paint_eq(&a.paint, &b.paint)
            && a.state.masks.len() == b.state.masks.len()
            && a.state.masks.iter().zip(&b.state.masks).all(|(a, b)| {
                let key = (Rc::as_ptr(a) as usize, Rc::as_ptr(b) as usize);
                if Rc::ptr_eq(a, b) || self.equal_sources.contains(&key) {
                    return true;
                }
                let equal = a.mapping == b.mapping
                    && a.feather == b.feather
                    && a.draws.len() == b.draws.len()
                    && a.draws
                        .iter()
                        .zip(&b.draws)
                        .all(|(a, b)| self.draw_eq(a, b));
                if equal {
                    self.equal_sources.insert(key);
                }
                equal
            })
    }

    fn paint_eq(&self, a: &CpuPaint, b: &CpuPaint) -> bool {
        match (a, b) {
            (CpuPaint::Fill(ap, ac, ao, ab), CpuPaint::Fill(bp, bc, bo, bb)) => {
                ap == bp && ac == bc && ao == bo && ab == bb
            }
            (CpuPaint::Stroke(ap, ac, ao, ab), CpuPaint::Stroke(bp, bc, bo, bb)) => {
                ap == bp && ac == bc && ao == bo && ab == bb
            }
            (CpuPaint::Image(a, ar, ao), CpuPaint::Image(b, br, bo)) => {
                Arc::ptr_eq(a, b) && ar == br && ao == bo
            }
            _ => false,
        }
    }
}

// Shared across sequential tiles, not one full framebuffer per warm canvas.
// Nested mask captures temporarily borrow another entry; siblings reuse it.
struct RasterScratch {
    context: VelloContext,
    resources: Resources,
    pixels: Pixmap,
    straight_rgba: Vec<u8>,
}

impl RasterScratch {
    fn take(pool: &mut Vec<Self>, width: u16, height: u16) -> Self {
        let mut scratch = pool.pop().unwrap_or_else(|| Self {
            context: VelloContext::new(width, height),
            resources: Resources::new(),
            pixels: Pixmap::new(width, height),
            straight_rgba: Vec::new(),
        });
        if scratch.pixels.width() != width || scratch.pixels.height() != height {
            scratch.context.reset_and_resize(width, height);
            scratch.pixels.resize(width, height);
        } else {
            scratch.context.reset();
        }
        scratch
    }

    fn rasterize(&mut self) {
        self.context.flush();
        self.context.render(&mut self.pixels, &mut self.resources);
    }

    fn straight_rgba(&mut self) -> &[u8] {
        self.straight_rgba.clear();
        self.straight_rgba
            .extend_from_slice(self.pixels.data_as_u8_slice());
        // Use Vello's SIMD conversion while keeping its pixmap premultiplied.
        vello_common::pixmap::unpremultiply_rgba8(&mut self.straight_rgba);
        &self.straight_rgba
    }
}

fn to_vello_path(path: &BezPath) -> vk::BezPath {
    path.elements()
        .iter()
        .map(|el| match *el {
            kurbo::PathEl::MoveTo(p) => vk::PathEl::MoveTo((p.x, p.y).into()),
            kurbo::PathEl::LineTo(p) => vk::PathEl::LineTo((p.x, p.y).into()),
            kurbo::PathEl::QuadTo(a, b) => vk::PathEl::QuadTo((a.x, a.y).into(), (b.x, b.y).into()),
            kurbo::PathEl::CurveTo(a, b, c) => {
                vk::PathEl::CurveTo((a.x, a.y).into(), (b.x, b.y).into(), (c.x, c.y).into())
            }
            kurbo::PathEl::ClosePath => vk::PathEl::ClosePath,
        })
        .collect()
}

fn vello_color(color: &api::Color) -> AlphaColor<Srgb> {
    AlphaColor::new(color.to_rgba_0_1().map(|v| v as f32))
}

fn paint_brush(paint: &Paint, bounds: Rect, opacity: f64) -> PaintType {
    let opacity = opacity.clamp(0.0, 1.0);
    let transparent = || PaintType::from(vello_color(&api::Color::TRANSPARENT));
    let Some(bounds) = api::drawing::paint_bounds(bounds) else {
        return transparent();
    };
    let stops = |stops: &[api::GradientStop], length: f64| -> ColorStops {
        ColorStops(
            stops
                .iter()
                .map(|s| {
                    ColorStop::from((
                        (s.position.evaluate((length, 0.0), api::Axis::X) / length) as f32,
                        vello_color(&s.color.with_alpha_factor(opacity)),
                    ))
                })
                .collect(),
        )
    };
    match paint {
        Paint::Solid(color) => vello_color(&color.with_alpha_factor(opacity)).into(),
        Paint::LinearGradient(g) => {
            let point = |p: (api::Size, api::Size)| {
                vk::Point::new(
                    bounds.x0
                        + p.0
                            .evaluate((bounds.width(), bounds.height()), api::Axis::X),
                    bounds.y0
                        + p.1
                            .evaluate((bounds.width(), bounds.height()), api::Axis::Y),
                )
            };
            let start = point(g.start);
            let end = point(g.end);
            let length = start.distance(end);
            if !length.is_finite() || length <= f64::EPSILON || g.stops.is_empty() {
                return transparent();
            }
            Gradient {
                kind: LinearGradientPosition { start, end }.into(),
                stops: stops(&g.stops, length),
                interpolation_alpha_space: InterpolationAlphaSpace::Premultiplied,
                ..Default::default()
            }
            .into()
        }
        Paint::RadialGradient(g) => {
            let Some(geometry) = g.resolve_geometry(bounds) else {
                return transparent();
            };
            if g.stops.is_empty() {
                return transparent();
            }
            Gradient {
                kind: RadialGradientPosition {
                    start_center: (geometry.focal_point.x, geometry.focal_point.y).into(),
                    start_radius: 0.0,
                    end_center: (geometry.center.x, geometry.center.y).into(),
                    end_radius: geometry.radius as f32,
                }
                .into(),
                stops: stops(&g.stops, geometry.radius),
                interpolation_alpha_space: InterpolationAlphaSpace::Premultiplied,
                ..Default::default()
            }
            .into()
        }
        Paint::MeshGradient(mesh) => {
            static WARNING: std::sync::Once = std::sync::Once::new();
            WARNING.call_once(|| log::warn!("CPU rendering uses a flat average for mesh paints; use $gpu / $cpu to author an alternate paint"));
            if mesh.validate().is_err() {
                return transparent();
            }
            let mut average = [0.0f64; 4];
            let mut count = 0;
            for point in mesh.rows.iter().flatten() {
                let [r, g, b, a] = point.color.to_rgba_0_1();
                for (sum, value) in average.iter_mut().zip([r * a, g * a, b * a, a]) {
                    *sum += value;
                }
                count += 1;
            }
            if average[3] <= 0.0 {
                return transparent();
            }
            AlphaColor::<Srgb>::new([
                (average[0] / average[3]) as f32,
                (average[1] / average[3]) as f32,
                (average[2] / average[3]) as f32,
                (average[3] / count as f64 * opacity) as f32,
            ])
            .into()
        }
        Paint::Blend(_) => unreachable!("paint mixtures are composited before shape coverage"),
    }
}

fn accumulate_paint(
    context: &mut VelloContext,
    paint: &Paint,
    bounds: Rect,
    coverage: &vk::Rect,
    weight: f64,
) {
    if let Paint::Blend(terms) = paint {
        for (term, amount) in terms {
            accumulate_paint(context, term, bounds, coverage, weight * amount);
        }
    } else if weight > 0.0 {
        context.set_paint(paint_brush(paint, bounds, weight));
        context.fill_rect(coverage);
    }
}

struct PaintPass<'a> {
    base: Affine,
    dpr: [f64; 2],
    width: u16,
    height: u16,
    // The uncropped framebuffer in this pass's coordinates. Allocation limits
    // must not change when only a smaller damaged rectangle is repainted.
    capture_viewport: Rect,
    masks: HashMap<Vec<usize>, Mask>,
    pool: &'a mut Vec<RasterScratch>,
}

impl PaintPass<'_> {
    fn mask(&mut self, sources: &[Rc<AlphaSource>]) -> Option<Mask> {
        if sources.is_empty() {
            return None;
        }
        let key: Vec<_> = sources.iter().map(|s| Rc::as_ptr(s) as usize).collect();
        if let Some(mask) = self.masks.get(&key) {
            return Some(mask.clone());
        }
        let mut masks = Vec::with_capacity(sources.len());
        for source in sources {
            let single_key = vec![Rc::as_ptr(source) as usize];
            let mask = if let Some(mask) = self.masks.get(&single_key) {
                mask.clone()
            } else {
                let mask = render_alpha_source(
                    source,
                    self.base,
                    self.dpr,
                    self.width,
                    self.height,
                    self.capture_viewport,
                    self.pool,
                );
                self.masks.insert(single_key, mask.clone());
                mask
            };
            masks.push(mask);
        }
        let mask = if masks.len() == 1 {
            masks.pop().unwrap()
        } else {
            let mut data = Vec::with_capacity(usize::from(self.width) * usize::from(self.height));
            for y in 0..self.height {
                for x in 0..self.width {
                    data.push(
                        masks
                            .iter()
                            .fold(255u32, |a, m| (a * u32::from(m.sample(x, y)) + 127) / 255)
                            as u8,
                    );
                }
            }
            Mask::from_parts(data, self.width, self.height)
        };
        self.masks.insert(key, mask.clone());
        Some(mask)
    }

    fn paint_draws(&mut self, context: &mut VelloContext, draws: &[CpuDraw], depth: usize) {
        let mut index = 0;
        while index < draws.len() {
            if let Some(scope) = draws[index].scopes.get(depth) {
                let end = index
                    + draws[index..]
                        .iter()
                        .take_while(|draw| {
                            draw.scopes
                                .get(depth)
                                .is_some_and(|next| next.node_id == scope.node_id)
                        })
                        .count();
                let opacity = scope.opacity.clamp(0.0, 1.0);
                if opacity > 0.0 {
                    if opacity < 1.0 {
                        context.reset_mask();
                        context.push_opacity_layer(opacity);
                    }
                    self.paint_draws(context, &draws[index..end], depth + 1);
                    if opacity < 1.0 {
                        context.pop_layer();
                    }
                }
                index = end;
                continue;
            }
            let draw = &draws[index];
            let visible = draw_bounds(draw, self.base).is_some_and(|bounds| {
                let bounds = bounds.inflate(1.0, 1.0).intersect(Rect::new(
                    0.0,
                    0.0,
                    self.width as f64,
                    self.height as f64,
                ));
                bounds.width() > 0.0 && bounds.height() > 0.0
            });
            if !visible {
                index += 1;
                continue;
            }
            context.reset_mask();
            context.reset_paint_transform();
            context.set_blend_mode(BlendMode::default());
            context.set_transform(vk::Affine::new(self.base.as_coeffs()));
            for clip in &draw.state.clips {
                context.push_clip_path(&to_vello_path(clip));
            }
            context.set_transform(vk::Affine::new(
                (self.base * draw.state.transform).as_coeffs(),
            ));
            let mask = self.mask(&draw.state.masks);
            match &draw.paint {
                CpuPaint::Fill(path, paint, opacity, bounds) => {
                    let path = to_vello_path(path);
                    if matches!(paint, Paint::Blend(_)) {
                        // Accumulate complete paints, then apply coverage, mask and opacity once.
                        context.push_layer(Some(&path), None, Some(*opacity as f32), mask, None);
                        context.set_blend_mode(Compose::Plus.into());
                        let b = vk::Shape::bounding_box(&path).inflate(1.0, 1.0);
                        accumulate_paint(context, paint, *bounds, &b, 1.0);
                        context.set_blend_mode(BlendMode::default());
                        context.pop_layer();
                    } else {
                        if let Some(mask) = mask {
                            context.set_mask(mask);
                        }
                        context.set_paint(paint_brush(paint, *bounds, *opacity));
                        context.fill_path(&path);
                    }
                }
                CpuPaint::Stroke(path, stroke, opacity, bounds) => {
                    if let Some(mask) = mask {
                        context.set_mask(mask);
                    }
                    context.set_paint(paint_brush(&stroke.paint, *bounds, *opacity));
                    let cap = match stroke.cap {
                        api::StrokeCap::Butt => vk::Cap::Butt,
                        api::StrokeCap::Round => vk::Cap::Round,
                        api::StrokeCap::Square => vk::Cap::Square,
                    };
                    let join = match stroke.join {
                        api::StrokeJoin::Miter => vk::Join::Miter,
                        api::StrokeJoin::Round => vk::Join::Round,
                        api::StrokeJoin::Bevel => vk::Join::Bevel,
                    };
                    context.set_stroke(
                        vk::Stroke::new(stroke.width_pixels())
                            .with_caps(cap)
                            .with_join(join)
                            .with_miter_limit(4.0),
                    );
                    context.stroke_path(&to_vello_path(path));
                }
                CpuPaint::Image(image, rect, opacity) => {
                    if let Some(mask) = mask {
                        context.set_mask(mask);
                    }
                    context.set_paint(Image {
                        image: ImageSource::Pixmap(image.clone()),
                        sampler: ImageSampler {
                            alpha: *opacity as f32,
                            ..Default::default()
                        },
                    });
                    context.set_paint_transform(
                        vk::Affine::translate((rect.x0, rect.y0))
                            * vk::Affine::scale_non_uniform(
                                rect.width() / f64::from(image.width()),
                                rect.height() / f64::from(image.height()),
                            ),
                    );
                    context.fill_rect(&vk::Rect::new(rect.x0, rect.y0, rect.x1, rect.y1));
                }
            }
            context.reset_mask();
            context.reset_paint_transform();
            for _ in &draw.state.clips {
                context.pop_clip();
            }
            index += 1;
        }
    }
}

fn warn_capture_limit() {
    static WARNING: std::sync::Once = std::sync::Once::new();
    WARNING.call_once(|| log::warn!("CPU alpha capture exceeds its 4096-pixel dimension limit or has nonfinite feather; masked content is hidden"));
}

fn draw_bounds(draw: &CpuDraw, transform: Affine) -> Option<Rect> {
    if draw.scopes.iter().any(|scope| scope.opacity <= 0.0) {
        return None;
    }
    let bounds = draw.coverage?;
    let mut bounds = (transform * draw.state.transform).transform_rect_bbox(bounds);
    for clip in &draw.state.clips {
        bounds = bounds.intersect(transform.transform_rect_bbox(clip.bounding_box()));
    }
    (bounds.width() > 0.0 && bounds.height() > 0.0).then_some(bounds)
}

fn alpha_source_bounds(source: &AlphaSource, transform: Affine) -> Option<Rect> {
    source
        .draws
        .iter()
        .filter_map(|draw| draw_bounds(draw, transform))
        .reduce(|a, b| a.union(b))
}

fn replay_damage(previous: &[CpuDraw], current: &[CpuDraw], base: Affine) -> Option<Rect> {
    let mut comparison = ReplayComparison::default();
    let mut damage: Option<Rect> = None;
    let mut previous_groups = Vec::new();
    let mut current_groups = Vec::new();
    for index in 0..previous.len().max(current.len()) {
        let old = previous.get(index);
        let new = current.get(index);
        update_scope_starts(previous, index, &mut previous_groups);
        update_scope_starts(current, index, &mut current_groups);
        if previous_groups == current_groups
            && old.zip(new).is_some_and(|(a, b)| comparison.draw_eq(a, b))
        {
            continue;
        }
        for bounds in old
            .into_iter()
            .chain(new)
            .filter_map(|draw| draw_bounds(draw, base))
        {
            damage = Some(damage.map_or(bounds, |damage| damage.union(bounds)));
        }
    }
    damage
}

fn update_scope_starts(draws: &[CpuDraw], index: usize, starts: &mut Vec<(usize, f32)>) {
    let Some(draw) = draws.get(index) else {
        starts.clear();
        return;
    };
    let common = index.checked_sub(1).map_or(0, |previous| {
        draws[previous]
            .scopes
            .iter()
            .zip(&draw.scopes)
            .take_while(|(a, b)| a.node_id == b.node_id)
            .count()
    });
    // Identical draws can still change composition when a neighboring command
    // joins or splits an opacity group. Track contiguous scope runs as well.
    starts.truncate(common);
    starts.extend(
        draw.scopes[common..]
            .iter()
            .map(|scope| (index, scope.opacity)),
    );
}

fn render_alpha_source(
    source: &AlphaSource,
    base: Affine,
    dpr: [f64; 2],
    width: u16,
    height: u16,
    capture_viewport: Rect,
    pool: &mut Vec<RasterScratch>,
) -> Mask {
    let empty = || {
        Mask::from_parts(
            vec![0; usize::from(width) * usize::from(height)],
            width,
            height,
        )
    };
    let sigma = dpr.map(|d| source.feather.max(0.0) * d);
    let halo = sigma.map(|s| (3.0 * s).ceil());
    if !source.feather.is_finite() || halo.iter().any(|pad| !pad.is_finite() || *pad > 2048.0) {
        warn_capture_limit();
        return empty();
    }
    let Some(bounds) = alpha_source_bounds(source, base * source.mapping) else {
        return empty();
    };
    let full_domain = bounds
        .inflate(halo[0] + 1.0, halo[1] + 1.0)
        .intersect(capture_viewport.inflate(halo[0], halo[1]))
        .expand();
    if ![
        full_domain.x0,
        full_domain.y0,
        full_domain.x1,
        full_domain.y1,
    ]
    .into_iter()
    .all(f64::is_finite)
        || full_domain.width() > 4096.0
        || full_domain.height() > 4096.0
    {
        warn_capture_limit();
        return empty();
    }
    // Rasterize only the source's conservative envelope, including AA and the
    // complete feather halo. Clip to the tile's halo, not its visible boundary.
    let domain = bounds
        .inflate(halo[0] + 1.0, halo[1] + 1.0)
        .intersect(Rect::new(
            -halo[0],
            -halo[1],
            f64::from(width) + halo[0],
            f64::from(height) + halo[1],
        ))
        .expand();
    if domain.width() <= 0.0 || domain.height() <= 0.0 {
        return empty();
    }
    let expanded = [domain.width() as u16, domain.height() as u16];
    let mut scratch = RasterScratch::take(pool, expanded[0], expanded[1]);
    PaintPass {
        base: Affine::translate((-domain.x0, -domain.y0)) * base * source.mapping,
        dpr,
        width: expanded[0],
        height: expanded[1],
        capture_viewport: full_domain + kurbo::Vec2::new(-domain.x0, -domain.y0),
        masks: HashMap::new(),
        pool,
    }
    .paint_draws(&mut scratch.context, &source.draws, 0);
    scratch.rasterize();
    if !sigma.iter().any(|s| *s > 0.0) {
        // Unfeathered coverage is already exact RGBA8 alpha. Only the blur
        // needs floating-point intermediates; copy the visible bytes directly.
        let mask = crop_mask_alpha(domain, expanded[0], width, height, |i| {
            scratch.pixels.data()[i].a
        });
        pool.push(scratch);
        return mask;
    }
    let mut alpha: Vec<f32> = scratch
        .pixels
        .data()
        .iter()
        .map(|p| f32::from(p.a))
        .collect();
    // Feather is an axis-separable Gaussian in logical surface pixels. Include
    // the full 3-sigma halo before filtering so adjacent tiles share edge coverage.
    {
        let w = usize::from(expanded[0]);
        let h = usize::from(expanded[1]);
        let mut temp = vec![0.0; alpha.len()];
        for axis in 0..2 {
            if sigma[axis] <= 0.0 {
                continue;
            }
            let radius = halo[axis] as isize;
            let mut kernel: Vec<f32> = (-radius..=radius)
                .map(|x| (-0.5 * (x as f64 / sigma[axis]).powi(2)).exp() as f32)
                .collect();
            let sum: f32 = kernel.iter().sum();
            for k in &mut kernel {
                *k /= sum;
            }
            for y in 0..h {
                for x in 0..w {
                    let mut sum = 0.0;
                    for (i, weight) in kernel.iter().enumerate() {
                        let offset = i as isize - radius;
                        let (sx, sy) = if axis == 0 {
                            (x as isize + offset, y as isize)
                        } else {
                            (x as isize, y as isize + offset)
                        };
                        if sx >= 0 && sy >= 0 && sx < w as isize && sy < h as isize {
                            sum += alpha[sy as usize * w + sx as usize] * weight;
                        }
                    }
                    temp[y * w + x] = sum;
                }
            }
            std::mem::swap(&mut alpha, &mut temp);
        }
    }
    let mask = crop_mask_alpha(domain, expanded[0], width, height, |i| {
        alpha[i].round().clamp(0.0, 255.0) as u8
    });
    pool.push(scratch);
    mask
}

fn crop_mask_alpha(
    domain: Rect,
    source_width: u16,
    width: u16,
    height: u16,
    alpha: impl Fn(usize) -> u8,
) -> Mask {
    let mut cropped = vec![0; usize::from(width) * usize::from(height)];
    let visible = domain.intersect(Rect::new(0.0, 0.0, f64::from(width), f64::from(height)));
    for y in visible.y0 as usize..visible.y1 as usize {
        for x in visible.x0 as usize..visible.x1 as usize {
            let source_index = (y as f64 - domain.y0) as usize * usize::from(source_width)
                + (x as f64 - domain.x0) as usize;
            cropped[y * usize::from(width) + x] = alpha(source_index);
        }
    }
    Mask::from_parts(cropped, width, height)
}

/// One software-backed canvas assignment in the shared surface scheduler.
pub struct CpuLayerRenderer<S: CpuSurface> {
    key: String,
    host_signature: String,
    surface: S,
    state: DrawState,
    saves: Vec<DrawState>,
    scopes: Vec<OpacityScope>,
    draws: Vec<CpuDraw>,
    previous_draws: Option<Vec<CpuDraw>>,
    presented_layout_valid: bool,
    needs_present: bool,
    captures: Vec<SourceCapture>,
    origin_x: f32,
    origin_y: f32,
    logical_width: f32,
    logical_height: f32,
    surface_width: u32,
    surface_height: u32,
    dpr: [f32; 2],
}

impl<S: CpuSurface> CpuLayerRenderer<S> {
    /// Attach a chassis presentation target to its committed surface plan entry.
    pub fn new(
        key: String,
        host_signature: String,
        mut surface: S,
        entry: &LayerSurfaceEntry,
    ) -> Self {
        surface.configure((entry.surface.surface_width, entry.surface.surface_height));
        Self {
            key,
            host_signature,
            surface,
            state: DrawState::default(),
            saves: Vec::new(),
            scopes: Vec::new(),
            draws: Vec::new(),
            previous_draws: None,
            presented_layout_valid: false,
            needs_present: true,
            captures: Vec::new(),
            origin_x: entry.origin_x,
            origin_y: entry.origin_y,
            logical_width: entry.surface.logical_width,
            logical_height: entry.surface.logical_height,
            surface_width: entry.surface.surface_width,
            surface_height: entry.surface.surface_height,
            dpr: entry.surface.dpr,
        }
    }

    fn clear(&mut self) {
        if !self.needs_present && self.presented_layout_valid && self.captures.is_empty() {
            self.previous_draws = Some(std::mem::take(&mut self.draws));
        }
        self.needs_present = true;
        self.draws.clear();
        self.state = DrawState::default();
        self.saves.clear();
        self.scopes.clear();
        self.captures.clear();
    }

    fn record(&mut self, paint: CpuPaint) {
        self.needs_present = true;
        self.draws
            .push(CpuDraw::new(self.state.clone(), self.scopes.clone(), paint));
    }

    fn flush(&mut self, pool: &mut Vec<RasterScratch>) {
        if !self.needs_present {
            return;
        }
        let base = Affine::scale_non_uniform(self.dpr[0] as f64, self.dpr[1] as f64)
            * Affine::translate((-self.origin_x as f64, -self.origin_y as f64));
        let full = Rect::new(
            0.0,
            0.0,
            self.surface_width.max(1) as f64,
            self.surface_height.max(1) as f64,
        );
        let damage = self.previous_draws.take().map_or(Some(full), |previous| {
            replay_damage(&previous, &self.draws, base)
        });
        self.needs_present = false;
        self.presented_layout_valid = true;
        let Some(damage) = damage else { return };
        // Include antialiasing coverage on both the old and new geometry. Every
        // command is composited again within this region, preserving backdrop,
        // group opacity, clip and mask semantics without touching other pixels.
        let domain = damage.inflate(1.0, 1.0).expand().intersect(full);
        if domain.width() <= 0.0 || domain.height() <= 0.0 {
            return;
        }
        let width = u16::try_from(domain.width() as u32).expect("CPU surface too wide");
        let height = u16::try_from(domain.height() as u32).expect("CPU surface too tall");
        let mut scratch = RasterScratch::take(pool, width, height);
        PaintPass {
            base: Affine::translate((-domain.x0, -domain.y0)) * base,
            dpr: self.dpr.map(f64::from),
            width,
            height,
            capture_viewport: full + kurbo::Vec2::new(-domain.x0, -domain.y0),
            masks: HashMap::new(),
            pool,
        }
        .paint_draws(&mut scratch.context, &self.draws, 0);
        scratch.rasterize();
        self.surface.present(
            scratch.straight_rgba(),
            u32::from(width),
            u32::from(height),
            (domain.x0 as u32, domain.y0 as u32),
        );
        pool.push(scratch);
        // Keep the commands until clear: presentation can be requested on an
        // unchanged canvas or after an incremental append. Pixels remain owned
        // by the chassis, while reusable software buffers serve other tiles.
    }

    fn begin_alpha_source(&mut self, mapping: Affine, feather: f64) {
        self.captures.push(SourceCapture {
            state: std::mem::take(&mut self.state),
            saves: std::mem::take(&mut self.saves),
            scopes: std::mem::take(&mut self.scopes),
            draws: std::mem::take(&mut self.draws),
            mapping,
            feather,
        });
    }

    fn end_alpha_source(&mut self) {
        let capture = self.captures.pop().expect("balanced alpha-source capture");
        let source = Rc::new(AlphaSource {
            draws: std::mem::replace(&mut self.draws, capture.draws),
            mapping: capture.mapping,
            feather: capture.feather,
        });
        self.state = capture.state;
        self.saves = capture.saves;
        self.scopes = capture.scopes;
        self.state.masks.push(source);
    }
    fn intersects_coverage_bounds(&self, bounds: &kurbo::Rect) -> bool {
        surface_intersects_coverage_bounds(
            bounds,
            self.origin_x as f64,
            self.origin_y as f64,
            self.logical_width as f64,
            self.logical_height as f64,
        )
    }

    fn coverage_bounds(&self) -> kurbo::Rect {
        kurbo::Rect::new(
            self.origin_x as f64,
            self.origin_y as f64,
            self.origin_x as f64 + self.logical_width as f64,
            self.origin_y as f64 + self.logical_height as f64,
        )
    }

    fn update_layout(&mut self, surface: &LayerSurfaceEntry) -> LayoutChangeKind {
        let origin_changed = (self.origin_x - surface.origin_x).abs() > f32::EPSILON
            || (self.origin_y - surface.origin_y).abs() > f32::EPSILON;
        let size_changed = self.logical_width != surface.surface.logical_width
            || self.logical_height != surface.surface.logical_height
            || self.surface_width != surface.surface.surface_width
            || self.surface_height != surface.surface.surface_height
            || self.dpr != surface.surface.dpr;

        self.origin_x = surface.origin_x;
        self.origin_y = surface.origin_y;
        self.logical_width = surface.surface.logical_width;
        self.logical_height = surface.surface.logical_height;
        self.surface_width = surface.surface.surface_width;
        self.surface_height = surface.surface.surface_height;
        self.dpr = surface.surface.dpr;

        if size_changed || origin_changed {
            self.previous_draws = None;
            self.presented_layout_valid = false;
            self.surface
                .configure((self.surface_width, self.surface_height));
        }

        if size_changed {
            self.clear();
        }

        if size_changed {
            LayoutChangeKind::Resized
        } else if origin_changed {
            LayoutChangeKind::OriginOnly
        } else {
            LayoutChangeKind::Unchanged
        }
    }
}

/// Current CPU surface set for one logical layer.
pub struct CpuLayerTarget<S: CpuSurface> {
    renderers: Vec<CpuLayerRenderer<S>>,
    active: bool,
}

impl<S: CpuSurface> CpuLayerTarget<S> {
    pub fn new(renderers: Vec<CpuLayerRenderer<S>>, active: bool) -> Self {
        Self { renderers, active }
    }

    fn activate(&mut self) -> bool {
        let changed = !self.active;
        self.active = true;
        changed
    }

    fn deactivate(&mut self) {
        self.active = false;
    }
}

/// `RenderContext` implementation backed by Vello CPU.
pub struct CpuRenderer<S: CpuSurface> {
    layers: Vec<LayerDef<S>>,
    image_map: HashMap<String, Arc<Pixmap>>,
    rasters: Vec<RasterScratch>,
    alpha_source_scopes: Vec<(usize, usize)>,
    layer_factory: Box<dyn Fn(usize) -> LayerDef<S>>,
    ready_layers: Vec<usize>,
    replay_layers: Vec<usize>,
    surface_replay: SurfaceReplayCoordinator,
    active_render_scopes: Vec<Vec<Vec<usize>>>,
    clean_skipped_canvas_nodes: HashSet<(usize, u32)>,
}

impl<S: CpuSurface> CpuRenderer<S> {
    /// Create a Vello CPU renderer with a chassis-provided logical layer factory.
    pub fn new(layer_factory: impl Fn(usize) -> LayerDef<S> + 'static) -> Self {
        Self {
            layer_factory: Box::new(layer_factory),
            layers: Vec::new(),
            image_map: HashMap::new(),
            rasters: Vec::new(),
            alpha_source_scopes: Vec::new(),
            ready_layers: Vec::new(),
            replay_layers: Vec::new(),
            surface_replay: SurfaceReplayCoordinator::default(),
            active_render_scopes: Vec::new(),
            clean_skipped_canvas_nodes: HashSet::new(),
        }
    }

    fn with_layer_renderer(&mut self, layer: usize, mut f: impl FnMut(&mut CpuLayerRenderer<S>)) {
        let scoped_indices = self
            .active_render_scopes
            .get(layer)
            .and_then(|scopes| scopes.last())
            .cloned();

        let Some((target, _)) = self.layers.get_mut(layer) else {
            return;
        };
        if !target.active {
            return;
        }

        if let Some(indices) = scoped_indices {
            for index in indices {
                if let Some(renderer) = target.renderers.get_mut(index) {
                    f(renderer);
                }
            }
        } else {
            for renderer in &mut target.renderers {
                f(renderer);
            }
        }
    }

    fn push_render_scope(&mut self, layer: usize, renderer_indices: Vec<usize>) {
        if self.active_render_scopes.len() <= layer {
            self.active_render_scopes.resize_with(layer + 1, Vec::new);
        }
        self.active_render_scopes[layer].push(renderer_indices);
    }

    fn pop_render_scope(&mut self, layer: usize) {
        if let Some(scopes) = self.active_render_scopes.get_mut(layer) {
            scopes.pop();
        }
    }

    fn take_replay_layer_ids(&mut self) -> Vec<usize> {
        let mut replay = std::mem::take(&mut self.replay_layers);
        replay.sort_unstable();
        replay.dedup();
        replay
    }

    fn targeted_or_all_indices(&self, layer: usize, renderer_count: usize) -> Vec<usize> {
        self.surface_replay
            .targeted_or_all_indices(layer, renderer_count)
    }

    fn clear_targeted_replay_scope(&mut self, layer: usize) {
        self.surface_replay.clear_targeted_replay_scope(layer);
    }

    fn set_targeted_replay_batches(
        &mut self,
        layer: usize,
        batches: Vec<Vec<usize>>,
        bounds_by_surface: HashMap<usize, Vec<kurbo::Rect>>,
    ) {
        self.surface_replay
            .set_targeted_replay_batches(layer, batches, bounds_by_surface);
    }

    fn advance_targeted_replay_queue(&mut self, layer: usize) -> bool {
        self.surface_replay.advance_targeted_replay_queue(layer)
    }

    fn targeted_replay_surface_bounds(
        &self,
        layer: usize,
        target: &CpuLayerTarget<S>,
    ) -> Option<Vec<kurbo::Rect>> {
        self.surface_replay
            .targeted_replay_surface_bounds(layer, |index| {
                target
                    .renderers
                    .get(index)
                    .map(CpuLayerRenderer::coverage_bounds)
            })
    }

    fn replay_region_bounds(&self, layer: usize) -> Option<Vec<kurbo::Rect>> {
        let Some((target, _)) = self.layers.get(layer) else {
            return None;
        };
        self.targeted_replay_surface_bounds(layer, target)
    }

    fn refresh_layer_layouts<I>(&mut self, layer_indices: I)
    where
        I: IntoIterator<Item = usize>,
    {
        for layer_index in layer_indices {
            let Some((target, layout_provider)) = self.layers.get_mut(layer_index) else {
                continue;
            };
            let layout = (layout_provider)();
            let layout_matches = layer_layout_matches_target(target, &layout);

            if !layout.active {
                target.deactivate();
                self.clear_targeted_replay_scope(layer_index);
                continue;
            }

            if !layout_matches {
                let mut retained: HashMap<_, _> = std::mem::take(&mut target.renderers)
                    .into_iter()
                    .map(|renderer| {
                        (
                            (renderer.key.clone(), renderer.host_signature.clone()),
                            renderer,
                        )
                    })
                    .collect();
                self.clear_targeted_replay_scope(layer_index);
                let (mut new_target, new_provider) = (self.layer_factory)(layer_index);
                for renderer in &mut new_target.renderers {
                    if let Some(mut previous) =
                        retained.remove(&(renderer.key.clone(), renderer.host_signature.clone()))
                    {
                        if let Some(entry) = layout.surfaces.iter().find(|entry| {
                            entry.key == renderer.key
                                && entry.host_signature == renderer.host_signature
                        }) {
                            // A warm-window change must not discard matching canvas pixels
                            // or their replay history. Host identity includes the surface lease.
                            previous.update_layout(entry);
                            *renderer = previous;
                        }
                    }
                }
                let has_surfaces = !new_target.renderers.is_empty();
                self.layers[layer_index] = (new_target, new_provider);
                if has_surfaces {
                    self.ready_layers.push(layer_index);
                    self.replay_layers.push(layer_index);
                }
                continue;
            }

            let activated_replay = target.activate() && !target.renderers.is_empty();

            let mut targeted_replay_entries = Vec::new();
            let mut targeted_replay_bounds = HashMap::new();
            let mut needs_full_layer_replay = false;
            #[cfg(debug_assertions)]
            {
                let previous_surface_bounds: Vec<_> = target
                    .renderers
                    .iter()
                    .map(CpuLayerRenderer::coverage_bounds)
                    .collect();
                if let Some(escape) =
                    visible_surface_escape(&previous_surface_bounds, &layout.surfaces)
                {
                    log_visible_surface_escape(layer_index, escape);
                }
            }
            for (index, (surface, renderer)) in layout
                .surfaces
                .iter()
                .zip(target.renderers.iter_mut())
                .enumerate()
            {
                let previous_bounds = renderer.coverage_bounds();
                let layout_change = renderer.update_layout(surface);
                let current_bounds = renderer.coverage_bounds();
                match layout_change {
                    LayoutChangeKind::Unchanged => {}
                    LayoutChangeKind::OriginOnly => {
                        targeted_replay_entries.push(ReplayPriorityEntry::new(
                            index,
                            surface.replay_priority,
                            previous_bounds,
                            current_bounds,
                        ));
                        targeted_replay_bounds.insert(index, vec![previous_bounds, current_bounds]);
                    }
                    LayoutChangeKind::Resized => {
                        needs_full_layer_replay = true;
                    }
                }
            }
            if activated_replay || needs_full_layer_replay {
                self.clear_targeted_replay_scope(layer_index);
                self.replay_layers.push(layer_index);
            } else if !targeted_replay_entries.is_empty() {
                self.set_targeted_replay_batches(
                    layer_index,
                    replay_batches_by_directional_priority(&targeted_replay_entries),
                    targeted_replay_bounds,
                );
                self.replay_layers.push(layer_index);
            }
        }
    }
}

fn layer_layout_matches_target<S: CpuSurface>(
    target: &CpuLayerTarget<S>,
    layout: &LayerSurfaceLayout,
) -> bool {
    layout.surfaces.len() == target.renderers.len()
        && layout
            .surfaces
            .iter()
            .zip(target.renderers.iter())
            .all(|(surface, renderer)| {
                surface.key == renderer.key && surface.host_signature == renderer.host_signature
            })
}

#[cfg(test)]
mod tests {
    use super::replay_batches_by_directional_priority;
    use crate::engine::layer_surface::ReplayPriorityEntry;
    use kurbo::Rect;

    #[test]
    fn cpu_replay_batches_coalesce_visible_surfaces_first() {
        let batches = replay_batches_by_directional_priority(&[
            replay_entry(0, 0, 0.0, 100.0),
            replay_entry(1, 0, 100.0, 200.0),
            replay_entry(2, 1, 200.0, 300.0),
            replay_entry(3, 1, -100.0, 0.0),
        ]);

        assert_eq!(batches[0], vec![0, 1]);
        assert_eq!(batches[1], vec![2]);
        assert_eq!(batches[2], vec![3]);
    }

    #[test]
    fn cpu_replay_batches_keep_warm_only_directional_batches() {
        let batches = replay_batches_by_directional_priority(&[
            replay_entry(0, 1, 0.0, 100.0),
            replay_entry(1, 1, 100.0, 200.0),
            replay_entry(2, 1, -100.0, 0.0),
        ]);

        assert_eq!(batches, vec![vec![1], vec![0], vec![2]]);
    }

    fn replay_entry(
        index: usize,
        priority: i32,
        previous_y: f64,
        current_y: f64,
    ) -> ReplayPriorityEntry {
        ReplayPriorityEntry::new(
            index,
            priority,
            Rect::new(0.0, previous_y, 100.0, previous_y + 100.0),
            Rect::new(0.0, current_y, 100.0, current_y + 100.0),
        )
    }
}

impl<S: CpuSurface> api::RenderContext for CpuRenderer<S> {
    fn fill_with_opacity(&mut self, layer: usize, path: BezPath, fill: &Paint, opacity: f64) {
        let bounds = path.bounding_box();
        self.with_layer_renderer(layer, |renderer| {
            renderer.record(CpuPaint::Fill(
                path.clone(),
                fill.clone(),
                opacity.clamp(0.0, 1.0),
                bounds,
            ))
        });
    }

    fn stroke_with_opacity(
        &mut self,
        layer: usize,
        path: BezPath,
        stroke: &ResolvedStroke,
        opacity: f64,
    ) {
        self.stroke_with_draw_range_and_material_and_opacity(
            layer,
            path,
            stroke,
            &stroke.material,
            opacity,
            0.0,
            1.0,
        );
    }

    fn stroke_with_draw_range_and_material_and_opacity(
        &mut self,
        layer: usize,
        path: BezPath,
        stroke: &ResolvedStroke,
        _material: &api::ResolvedMaterial,
        opacity: f64,
        draw_start: f64,
        draw_end: f64,
    ) {
        if draw_start >= draw_end || stroke.width_pixels() <= f64::EPSILON {
            return;
        }
        let bounds = path.bounding_box();
        let path = api::drawing::path_trim::trim_bez_path(&path, draw_start, draw_end);
        if matches!(stroke.paint, Paint::Blend(_))
            || (bounds.width() == 0.0 && bounds.height() == 0.0)
        {
            if let Some(outline) = api::drawing::stroke_utils::stroked_outline_path(&path, stroke) {
                self.with_layer_renderer(layer, |r| {
                    r.record(CpuPaint::Fill(
                        outline.clone(),
                        stroke.paint.clone(),
                        opacity.clamp(0.0, 1.0),
                        bounds,
                    ))
                });
            }
        } else {
            self.with_layer_renderer(layer, |r| {
                r.record(CpuPaint::Stroke(
                    path.clone(),
                    stroke.clone(),
                    opacity.clamp(0.0, 1.0),
                    bounds,
                ))
            });
        }
    }

    fn save(&mut self, layer: usize) {
        self.with_layer_renderer(layer, |r| r.saves.push(r.state.clone()));
    }
    fn restore(&mut self, layer: usize) {
        self.with_layer_renderer(layer, |r| {
            if let Some(state) = r.saves.pop() {
                r.state = state;
            }
        });
    }
    fn transform(&mut self, layer: usize, affine: Affine) {
        self.with_layer_renderer(layer, |r| r.state.transform *= affine);
    }
    fn clip(&mut self, layer: usize, path: BezPath) {
        self.with_layer_renderer(layer, |r| {
            r.state.clips.push(Rc::new(r.state.transform * &path))
        });
    }

    fn supports_alpha_source_capture(&self) -> bool {
        true
    }
    fn alpha_source_layer(&self) -> Option<usize> {
        self.alpha_source_scopes.last().map(|s| s.0)
    }
    fn begin_alpha_source(
        &mut self,
        layer: usize,
        mapping: Affine,
        inherited_scopes: usize,
        feather: f64,
    ) {
        self.with_layer_renderer(layer, |r| r.begin_alpha_source(mapping, feather));
        self.alpha_source_scopes.push((layer, inherited_scopes));
    }
    fn end_alpha_source(&mut self, layer: usize) {
        self.with_layer_renderer(layer, |r| r.end_alpha_source());
        assert_eq!(self.alpha_source_scopes.pop().map(|s| s.0), Some(layer));
    }
    fn clip_alpha(&mut self, layer: usize, paints: &[api::AlphaMaskPaint], feather: f64) {
        self.with_layer_renderer(layer, |r| {
            let draws = paints
                .iter()
                .map(|paint| {
                    CpuDraw::new(
                        DrawState {
                            transform: paint.transform,
                            ..Default::default()
                        },
                        paint
                            .composition
                            .map(|(node_id, opacity)| OpacityScope { node_id, opacity })
                            .into_iter()
                            .collect(),
                        CpuPaint::Fill(
                            paint.path.clone(),
                            paint.fill.clone(),
                            paint.opacity,
                            paint.paint_bounds,
                        ),
                    )
                })
                .collect();
            r.state.masks.push(Rc::new(AlphaSource {
                draws,
                mapping: r.state.transform,
                feather,
            }));
        });
    }

    fn load_image(&mut self, path: &str, bytes: &[u8], width: usize, height: usize) {
        let (Ok(w), Ok(h)) = (u16::try_from(width), u16::try_from(height)) else {
            log::error!("CPU image exceeds 65535 pixels: {path}");
            return;
        };
        if w == 0
            || h == 0
            || width.checked_mul(height).and_then(|n| n.checked_mul(4)) != Some(bytes.len())
        {
            log::error!("invalid RGBA image: {path}");
            return;
        }
        let mut image = Pixmap::new(w, h);
        for (out, p) in image.data_mut().iter_mut().zip(bytes.chunks_exact(4)) {
            *out = AlphaColor::<Srgb>::from_rgba8(p[0], p[1], p[2], p[3])
                .premultiply()
                .to_rgba8();
        }
        self.image_map.insert(path.to_owned(), Arc::new(image));
    }
    fn get_image_size(&mut self, path: &str) -> Option<(usize, usize)> {
        self.image_map
            .get(path)
            .map(|i| (usize::from(i.width()), usize::from(i.height())))
    }
    fn image_loaded(&self, path: &str) -> bool {
        self.image_map.contains_key(path)
    }
    fn draw_image_with_opacity(&mut self, layer: usize, path: &str, rect: Rect, opacity: f64) {
        let Some(image) = self.image_map.get(path).cloned() else {
            return;
        };
        self.with_layer_renderer(layer, |r| {
            r.record(CpuPaint::Image(
                image.clone(),
                rect,
                opacity.clamp(0.0, 1.0),
            ))
        });
    }
    fn layers(&self) -> usize {
        self.layers.len()
    }

    fn resize_layers_to(&mut self, layer_count: usize, dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        let current_len = self.layers.len();
        match layer_count.cmp(&current_len) {
            std::cmp::Ordering::Less => {
                self.layers.truncate(layer_count);
                self.active_render_scopes.truncate(layer_count);
                self.surface_replay.truncate_layers(layer_count);
            }
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Greater => {
                for i in current_len..layer_count {
                    let (target, provider) = (self.layer_factory)(i);
                    let has_surfaces = target.active && !target.renderers.is_empty();
                    self.layers.push((target, provider));
                    if let Some(dirty_bit) = dirty_canvases.borrow_mut().get_mut(i) {
                        *dirty_bit = true;
                    }
                    if has_surfaces {
                        self.ready_layers.push(i);
                    }
                }
            }
        }
    }

    fn clear(&mut self, layer: usize) {
        let scoped_indices = self
            .active_render_scopes
            .get(layer)
            .and_then(|scopes| scopes.last())
            .cloned()
            .or_else(|| {
                self.layers
                    .get(layer)
                    .map(|(target, _)| self.targeted_or_all_indices(layer, target.renderers.len()))
            });
        let Some((target, _)) = self.layers.get_mut(layer) else {
            return;
        };
        if !target.active {
            return;
        }
        if let Some(indices) = scoped_indices {
            for index in indices {
                if let Some(renderer) = target.renderers.get_mut(index) {
                    renderer.clear();
                }
            }
        } else {
            for renderer in &mut target.renderers {
                renderer.clear();
            }
        }
    }

    fn supports_subtree_opacity(&self) -> bool {
        true
    }

    fn set_node_opacity_scopes(&mut self, layer: usize, _node_id: u32, scopes: &[OpacityScope]) {
        let inherited = self
            .alpha_source_scopes
            .last()
            .map_or(0, |s| s.1)
            .min(scopes.len());
        self.with_layer_renderer(layer, |renderer| {
            renderer.scopes = scopes[inherited..].to_vec()
        });
    }

    fn flush(&mut self, layer: usize, _dirty_canvases: Rc<RefCell<Vec<bool>>>) {
        let mut pool = std::mem::take(&mut self.rasters);
        self.with_layer_renderer(layer, |renderer| renderer.flush(&mut pool));
        self.rasters = pool;
        if self.advance_targeted_replay_queue(layer) {
            self.replay_layers.push(layer);
        }
    }

    fn resize(&mut self, _width: usize, _height: usize) {
        let layer_count = self.layers.len();
        self.refresh_layer_layouts(0..layer_count);
    }

    fn refresh_layers(&mut self, layers: &[usize]) {
        self.refresh_layer_layouts(layers.iter().copied());
    }

    fn clear_targeted_replay(&mut self, layer: usize) {
        self.clear_targeted_replay_scope(layer);
    }

    fn take_ready_canvas_layers(&mut self) -> Vec<usize> {
        let mut ready = std::mem::take(&mut self.ready_layers);
        ready.sort_unstable();
        ready.dedup();
        ready
    }

    fn take_replay_canvas_layer_updates(
        &mut self,
    ) -> Vec<pax_runtime_api::ReplayCanvasLayerUpdate> {
        let replay_layers = self.take_replay_layer_ids();
        replay_layers
            .into_iter()
            .map(|layer| pax_runtime_api::ReplayCanvasLayerUpdate {
                layer,
                regions: self.replay_region_bounds(layer),
            })
            .collect()
    }

    fn retains_canvas_nodes(&self) -> bool {
        false
    }

    fn begin_node(&mut self, layer: usize, _node_id: u32, _z_index: i32, _light_mask: u32) -> bool {
        let renderer_count = {
            let Some((target, _)) = self.layers.get(layer) else {
                return false;
            };
            if !target.active {
                return false;
            }
            target.renderers.len()
        };
        let selected = self.targeted_or_all_indices(layer, renderer_count);
        let began = !selected.is_empty();
        if began {
            self.push_render_scope(layer, selected);
        }
        began
    }

    fn begin_node_with_bounds(
        &mut self,
        layer: usize,
        node_id: u32,
        _z_index: i32,
        coverage_bounds: kurbo::Rect,
        _light_mask: u32,
    ) -> bool {
        if self.alpha_source_layer().is_some() {
            return self.begin_node(layer, node_id, _z_index, _light_mask);
        }
        let renderer_count = {
            let Some((target, _)) = self.layers.get(layer) else {
                return false;
            };
            if !target.active {
                return false;
            }
            target.renderers.len()
        };
        let candidate_indices = self.targeted_or_all_indices(layer, renderer_count);
        let Some((target, _)) = self.layers.get(layer) else {
            return false;
        };
        let selected: Vec<_> = target
            .renderers
            .iter()
            .enumerate()
            .filter(|(index, _)| candidate_indices.contains(index))
            .filter_map(|(index, renderer)| {
                renderer
                    .intersects_coverage_bounds(&coverage_bounds)
                    .then_some(index)
            })
            .collect();
        let began = !selected.is_empty();
        if began {
            self.push_render_scope(layer, selected);
        } else {
            self.clean_skipped_canvas_nodes.insert((layer, node_id));
        }
        began
    }

    fn take_clean_skipped_node(&mut self, layer: usize, node_id: u32) -> bool {
        self.clean_skipped_canvas_nodes.remove(&(layer, node_id))
    }

    fn end_node(&mut self, layer: usize, _node_id: u32) -> bool {
        self.pop_render_scope(layer);
        true
    }

    fn remove_node(&mut self, _layer: usize, _node_id: u32) -> bool {
        true
    }
}

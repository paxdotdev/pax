use std::pin::Pin;

use crate::browser_surface_policy::BrowserSurfacePolicy;
use pax_runtime::api::RenderContext;
use pax_runtime::engine::layer_surface::{LayerSurfaceEntry, LayerSurfaceLayout, LayerSurfaceSize};
use wasm_bindgen::JsCast;
use wasm_bindgen::{closure::Closure, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Document, HtmlCanvasElement, Window};

struct SurfaceMetrics {
    logical_width: f32,
    logical_height: f32,
    surface_width: u32,
    surface_height: u32,
    dpr: [f32; 2],
}

struct LayerCanvasTarget {
    key: String,
    host_signature: String,
    canvas: HtmlCanvasElement,
    origin_x: f32,
    origin_y: f32,
    replay_priority: i32,
    active: bool,
    surface: SurfaceMetrics,
}

const TILED_SCROLLER_DESIRED_DPR: f64 = 1.0;

fn compute_surface_metrics(
    logical_width: f64,
    logical_height: f64,
    desired_dpr: f64,
    max_surface_dimension: u32,
    minimum_dpr: f64,
) -> SurfaceMetrics {
    let logical_width = logical_width.max(0.0);
    let logical_height = logical_height.max(0.0);
    let minimum_dpr = minimum_dpr.clamp(0.1, 1.0);
    let desired_dpr = desired_dpr.max(minimum_dpr);
    let max_surface_dimension = max_surface_dimension.max(1) as f64;

    let width_limit = if logical_width > 0.0 {
        max_surface_dimension / logical_width
    } else {
        desired_dpr
    };
    let height_limit = if logical_height > 0.0 {
        max_surface_dimension / logical_height
    } else {
        desired_dpr
    };
    let requested_dpr_x = desired_dpr.min(width_limit).max(minimum_dpr);
    let requested_dpr_y = desired_dpr.min(height_limit).max(minimum_dpr);
    let surface_width =
        ((logical_width * requested_dpr_x).round() as u32).clamp(1, max_surface_dimension as u32);
    let surface_height =
        ((logical_height * requested_dpr_y).round() as u32).clamp(1, max_surface_dimension as u32);
    let dpr_x = if logical_width > 0.0 {
        surface_width as f64 / logical_width
    } else {
        desired_dpr
    };
    let dpr_y = if logical_height > 0.0 {
        surface_height as f64 / logical_height
    } else {
        desired_dpr
    };

    if dpr_x + 0.01 < desired_dpr || dpr_y + 0.01 < desired_dpr {
        log::debug!(
            "render backend: clamped backing scale from {:.2} to {:.2}x{:.2} to fit max surface dimension {}",
            desired_dpr,
            dpr_x,
            dpr_y,
            max_surface_dimension as u32
        );
    }

    SurfaceMetrics {
        logical_width: logical_width as f32,
        logical_height: logical_height as f32,
        surface_width,
        surface_height,
        dpr: [dpr_x as f32, dpr_y as f32],
    }
}

pub(crate) fn get_render_context(
    window: Window,
    surface_policy: BrowserSurfacePolicy,
) -> Box<dyn RenderContext> {
    if cfg!(feature = "piet") || surface_policy.use_piet_fallback() {
        log::info!("render backend: using Piet/CPU browser renderer");
        Box::new(get_piet_render_context(window, surface_policy))
    } else {
        log::info!("render backend: using WebGPU browser renderer");
        Box::new(get_gpu_render_context(window, surface_policy))
    }
}

fn get_piet_render_context(
    window: Window,
    surface_policy: BrowserSurfacePolicy,
) -> impl RenderContext {
    use pax_runtime::piet_render_context::{PietLayerRenderer, PietLayerTarget, PietRenderer};
    use piet_web::WebRenderContext;

    PietRenderer::new(move |layer| {
        let document = window.document().unwrap();
        let targets = query_layer_canvas_targets(
            &document,
            layer,
            window.device_pixel_ratio(),
            surface_policy.effective_max_surface_dimension(layer, u32::MAX),
            surface_policy.minimum_dpr(layer),
            surface_policy.defer_transient_root_host_surfaces(layer),
        );
        let layout = layer_surface_layout_from_targets(targets.iter());
        let renderers = targets
            .into_iter()
            .map(|target| {
                let context: web_sys::CanvasRenderingContext2d = target
                    .canvas
                    .get_context("2d")
                    .unwrap()
                    .unwrap()
                    .dyn_into::<web_sys::CanvasRenderingContext2d>()
                    .unwrap();
                configure_piet_canvas_context(
                    &context,
                    &target.canvas,
                    target.origin_x,
                    target.origin_y,
                    target.surface.surface_width,
                    target.surface.surface_height,
                    target.surface.dpr,
                    true,
                );
                let entry = layer_surface_entry_from_target(&target);
                let clear_fn = Box::new({
                    let context = context.clone();
                    let canvas = target.canvas.clone();
                    move || {
                        clear_piet_canvas_context(&context, &canvas);
                    }
                });
                let configure_fn = Box::new({
                    let context = context.clone();
                    let canvas = target.canvas.clone();
                    move |origin_x, origin_y, surface_width, surface_height, dpr, resize_surface| {
                        configure_piet_canvas_context(
                            &context,
                            &canvas,
                            origin_x,
                            origin_y,
                            surface_width,
                            surface_height,
                            dpr,
                            resize_surface,
                        );
                    }
                });
                // Piet's generic API has no image opacity parameter. Apply it on
                // the existing canvas, preserving the source image and canvas state.
                let draw_image_fn = Box::new({
                    let context = context.clone();
                    move |renderer: &mut WebRenderContext,
                          image: &piet_web::WebImage,
                          rect,
                          opacity| {
                        let previous = context.global_alpha();
                        context.set_global_alpha(opacity);
                        piet::RenderContext::draw_image(
                            renderer,
                            image,
                            rect,
                            piet::InterpolationMode::Bilinear,
                        );
                        context.set_global_alpha(previous);
                    }
                });
                let draw_blend_fn = paint_blend_callback(&context, &target.canvas, &window);
                PietLayerRenderer::new(
                    target.key.clone(),
                    target.host_signature.clone(),
                    WebRenderContext::new(context, window.clone()),
                    clear_fn,
                    configure_fn,
                    draw_image_fn,
                    draw_blend_fn,
                    &entry,
                )
            })
            .collect();
        let layout_provider: Box<dyn Fn() -> LayerSurfaceLayout> = Box::new({
            let document = document.clone();
            let window = window.clone();
            move || {
                build_layer_surface_layout(
                    &document,
                    layer,
                    window.device_pixel_ratio(),
                    surface_policy.effective_max_surface_dimension(layer, u32::MAX),
                    surface_policy.minimum_dpr(layer),
                    surface_policy.defer_transient_root_host_surfaces(layer),
                )
            }
        });

        (
            PietLayerTarget::new(renderers, layout.active),
            layout_provider,
        )
    })
}

// One scratch canvas per surface, allocated only while a paint is transitioning.
// Add premultiplied endpoint colors on transparent black, then clip and composite
// the resulting paint once. Native elements and group boundaries are not involved.
fn paint_blend_callback(
    target: &web_sys::CanvasRenderingContext2d,
    canvas: &HtmlCanvasElement,
    window: &Window,
) -> Box<
    dyn FnMut(
        &mut piet_web::WebRenderContext,
        &piet::kurbo::BezPath,
        &[(pax_runtime::api::Fill, f64)],
        f64,
    ),
> {
    use pax_runtime::piet_render_context::fill_to_piet_brush;
    use piet::{
        kurbo::{Affine, Rect, Shape},
        IntoBrush, RenderContext,
    };
    let target = target.clone();
    let canvas = canvas.clone();
    let window = window.clone();
    let mut scratch: Option<(HtmlCanvasElement, web_sys::CanvasRenderingContext2d)> = None;
    Box::new(move |renderer, path, terms, opacity| {
        let rect = path.bounding_box();
        let matrix = target.get_transform().expect("canvas transform");
        let transform = Affine::new([
            matrix.a(),
            matrix.b(),
            matrix.c(),
            matrix.d(),
            matrix.e(),
            matrix.f(),
        ]);
        if transform.determinant().abs() < f64::EPSILON {
            return;
        }
        let bounds = transform
            .transform_rect_bbox(rect)
            .inflate(1.0, 1.0)
            .intersect(Rect::new(
                0.0,
                0.0,
                canvas.width() as f64,
                canvas.height() as f64,
            ))
            .expand();
        if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
            return;
        }
        let (scratch_canvas, context) = scratch.get_or_insert_with(|| {
            let canvas: HtmlCanvasElement = window
                .document()
                .unwrap()
                .create_element("canvas")
                .unwrap()
                .dyn_into()
                .unwrap();
            let context = canvas
                .get_context("2d")
                .unwrap()
                .unwrap()
                .dyn_into()
                .unwrap();
            (canvas, context)
        });
        let width = bounds.width() as u32;
        let height = bounds.height() as u32;
        // Grow geometrically within the surface's existing allocation limit.
        if scratch_canvas.width() < width || scratch_canvas.width() > canvas.width() {
            scratch_canvas.set_width(width.next_power_of_two().min(canvas.width()));
        }
        if scratch_canvas.height() < height || scratch_canvas.height() > canvas.height() {
            scratch_canvas.set_height(height.next_power_of_two().min(canvas.height()));
        }
        context.reset_transform().unwrap();
        context.clear_rect(
            0.0,
            0.0,
            scratch_canvas.width() as f64,
            scratch_canvas.height() as f64,
        );
        context
            .set_transform(
                matrix.a(),
                matrix.b(),
                matrix.c(),
                matrix.d(),
                matrix.e() - bounds.x0,
                matrix.f() - bounds.y0,
            )
            .unwrap();
        context.set_global_composite_operation("lighter").unwrap();
        let mut painter = piet_web::WebRenderContext::new(context.clone(), window.clone());
        // Resolve brushes against the original bounds, but extend their painted
        // area past the path edge: antialiasing belongs to the final clip only.
        let inverse = transform.inverse().as_coeffs();
        let inset =
            2.0 * (inverse[0].abs() + inverse[1].abs() + inverse[2].abs() + inverse[3].abs());
        fn accumulate(
            painter: &mut piet_web::WebRenderContext,
            context: &web_sys::CanvasRenderingContext2d,
            rect: Rect,
            inset: f64,
            terms: &[(pax_runtime::api::Fill, f64)],
            weight: f64,
        ) {
            for (fill, inner_weight) in terms {
                let weight = weight * inner_weight;
                if let pax_runtime::api::Fill::Blend(terms) = fill {
                    accumulate(painter, context, rect, inset, terms, weight);
                } else if let Some(brush) = fill_to_piet_brush(fill, rect) {
                    context.set_global_alpha(weight.clamp(0.0, 1.0));
                    let brush = brush.make_brush(painter, || rect).into_owned();
                    painter.fill(rect.inflate(inset, inset), &brush);
                }
            }
        }
        accumulate(&mut painter, context, rect, inset, terms, 1.0);
        renderer.save().unwrap();
        renderer.clip(path.clone());
        target.reset_transform().unwrap();
        target.set_global_alpha(target.global_alpha() * opacity.clamp(0.0, 1.0));
        target
            .draw_image_with_html_canvas_element_and_sw_and_sh_and_dx_and_dy_and_dw_and_dh(
                scratch_canvas,
                0.0,
                0.0,
                width as f64,
                height as f64,
                bounds.x0,
                bounds.y0,
                width as f64,
                height as f64,
            )
            .unwrap();
        renderer.restore().unwrap();
    })
}

fn get_gpu_render_context(
    window: Window,
    surface_policy: BrowserSurfacePolicy,
) -> impl RenderContext {
    use pax_gpu::{
        render_backend::{RenderBackend, RenderConfig, SharedGpuContext},
        Transform2D, WgpuRenderer,
    };
    use pax_runtime::pax_gpu_render_context::{LayerRenderer, LayerTarget, PaxGpuRenderer};
    PaxGpuRenderer::new(move |layer| {
        let window = window.clone();
        let surface_policy = surface_policy;
        Box::pin(async move {
            let document = window.document().unwrap();
            let initial_targets = wait_for_layer_canvas_targets(
                &window,
                &document,
                layer,
                window.device_pixel_ratio(),
                surface_policy.effective_max_surface_dimension(layer, u32::MAX),
                surface_policy.minimum_dpr(layer),
                surface_policy.defer_transient_root_host_surfaces(layer),
            )
            .await;
            let mut renderers = Vec::with_capacity(initial_targets.len());
            let mut backend_limit = u32::MAX;
            let mut shared_context: Option<SharedGpuContext> = None;
            for target in &initial_targets {
                target.canvas.set_width(target.surface.surface_width);
                target.canvas.set_height(target.surface.surface_height);

                let (backend, context) = match RenderBackend::to_canvas_with_context(
                    target.canvas.clone(),
                    RenderConfig::new(
                        false,
                        target.surface.surface_width,
                        target.surface.surface_height,
                        target.surface.dpr,
                    )
                    .with_browser_premultiplied_alpha(true),
                    shared_context.clone(),
                )
                .await
                {
                    Ok(backend) => backend,
                    Err(err) => {
                        log::warn!(
                            "failed to create browser render backend for layer {} tile {}: {}",
                            layer,
                            target.key,
                            err
                        );
                        return None;
                    }
                };
                shared_context.get_or_insert(context);

                let mut renderer = WgpuRenderer::new(backend);
                renderer.set_surface_transform(Transform2D::from_array([
                    1.0,
                    0.0,
                    0.0,
                    1.0,
                    -target.origin_x,
                    -target.origin_y,
                ]));
                backend_limit = backend_limit.min(renderer.max_surface_dimension());
                renderers.push(LayerRenderer::new(
                    target.key.clone(),
                    target.host_signature.clone(),
                    renderer,
                    target.origin_x,
                    target.origin_y,
                    target.surface.logical_width,
                    target.surface.logical_height,
                    target.surface.surface_width,
                    target.surface.surface_height,
                    target.surface.dpr,
                ));
            }

            let effective_max_surface_dimension =
                surface_policy.effective_max_surface_dimension(layer, backend_limit);
            let resize_provider = Box::pin({
                let document = document.clone();
                let window = window.clone();
                move || {
                    build_layer_surface_layout(
                        &document,
                        layer,
                        window.device_pixel_ratio(),
                        effective_max_surface_dimension,
                        surface_policy.minimum_dpr(layer),
                        surface_policy.defer_transient_root_host_surfaces(layer),
                    )
                }
            }) as Pin<Box<dyn Fn() -> LayerSurfaceLayout>>;
            let layout = resize_provider();
            // Browser-owned scroller layers can intentionally shed every canvas target while they
            // are well outside the viewport. Keep an empty target alive here so later layout syncs
            // can rehydrate the renderer instead of treating "no surfaces right now" as fatal.
            let mut target = LayerTarget::new(renderers, layout.active);
            for (surface, renderer) in layout
                .surfaces
                .iter()
                .zip(target.renderers_mut().iter_mut())
            {
                renderer
                    .renderer_mut()
                    .set_surface_transform(Transform2D::from_array([
                        1.0,
                        0.0,
                        0.0,
                        1.0,
                        -surface.origin_x,
                        -surface.origin_y,
                    ]));
                renderer.renderer_mut().resize_surface(
                    surface.surface.surface_width as f32,
                    surface.surface.surface_height as f32,
                );
                renderer.renderer_mut().set_viewport(
                    surface.surface.logical_width,
                    surface.surface.logical_height,
                    surface.surface.dpr,
                );
                renderer.sync_layout_metadata(surface);
            }
            Some((target, resize_provider))
        })
    })
}

async fn wait_for_canvas_layout_settle(window: &Window, document: &Document, layer: usize) {
    let mut last_signature = String::new();
    let mut stable_frames = 0;
    for _ in 0..12 {
        let canvases = query_layer_canvases(document, layer);
        let parent_role = canvases
            .first()
            .and_then(|canvas| canvas.parent_element())
            .and_then(|node| node.get_attribute("data-role"));
        // Non-root canvas layers are mounted into browser-owned scroller hosts. Give the DOM a
        // frame to settle there before sizing the backing surface, otherwise we can initialize
        // against the bootstrap size and immediately force a second surface configure.
        let on_scroller_host = parent_role.as_deref() == Some("scroller-canvas-host");
        let on_root_host = parent_role.is_none();
        if !on_scroller_host {
            if layer == 0 || !on_root_host {
                return;
            }
        }
        let signature = canvases
            .iter()
            .map(canvas_layout_signature)
            .collect::<Vec<_>>()
            .join("|");
        let has_size = !canvases.is_empty()
            && canvases.iter().all(|canvas| {
                planned_canvas_logical_width(canvas) > 0.0
                    && planned_canvas_logical_height(canvas) > 0.0
            });
        if has_size && signature == last_signature {
            stable_frames += 1;
        } else {
            stable_frames = 0;
            last_signature = signature;
        }
        if has_size && stable_frames >= 1 {
            // Non-root layers often appear on the root host for a frame before an explicit
            // scroller-island claim rebinds them into their browser-owned canvas host. Waiting for
            // one stable frame avoids bootstrapping browser render surfaces against that transient
            // root placement, which is especially costly on iOS WebKit where the total canvas
            // budget is small.
            return;
        }
        wait_for_animation_frame(window).await;
    }
}

async fn wait_for_layer_canvas_targets(
    window: &Window,
    document: &Document,
    layer: usize,
    desired_dpr: f64,
    max_surface_dimension: u32,
    minimum_dpr: f64,
    defer_transient_root_host_surfaces: bool,
) -> Vec<LayerCanvasTarget> {
    if layer != 0 && query_layer_canvases(document, layer).is_empty() {
        return Vec::new();
    }
    for _ in 0..24 {
        wait_for_canvas_layout_settle(window, document, layer).await;
        let targets = query_layer_canvas_targets(
            document,
            layer,
            desired_dpr,
            max_surface_dimension,
            minimum_dpr,
            defer_transient_root_host_surfaces,
        );
        if !targets.is_empty() {
            return targets;
        }
        wait_for_animation_frame(window).await;
    }
    Vec::new()
}

fn query_layer_canvas_targets(
    document: &Document,
    layer: usize,
    desired_dpr: f64,
    max_surface_dimension: u32,
    minimum_dpr: f64,
    defer_transient_root_host_surfaces: bool,
) -> Vec<LayerCanvasTarget> {
    // The web chassis discovers however many physical canvases currently represent this logical
    // layer. Root layers usually expose one canvas; browser-owned nested scroller hosts can expose
    // an active tile set instead. The Rust render path mirrors that set without changing node code.
    query_layer_canvases(document, layer)
        .into_iter()
        .filter(|canvas| {
            if !defer_transient_root_host_surfaces {
                return true;
            }
            let Some(parent) = canvas.parent_element() else {
                return false;
            };
            let parent_role = parent.get_attribute("data-role");
            if parent_role.is_some() {
                return true;
            }
            false
        })
        .map(|canvas| {
            let key = canvas
                .get_attribute("data-tile-key")
                .unwrap_or_else(|| canvas.id());
            let active = canvas_render_state(&canvas) != "parked";
            let role = canvas
                .parent_element()
                .and_then(|parent| parent.get_attribute("data-role"));
            let host_signature = canvas_host_signature(&canvas);
            let origin_x = canvas
                .get_attribute("data-tile-origin-x")
                .and_then(|value: String| value.parse::<f32>().ok())
                .unwrap_or(0.0);
            let origin_y = canvas
                .get_attribute("data-tile-origin-y")
                .and_then(|value: String| value.parse::<f32>().ok())
                .unwrap_or(0.0);
            let replay_priority = canvas
                .get_attribute("data-replay-priority")
                .and_then(|value: String| value.parse::<i32>().ok())
                .unwrap_or(0);
            let surface_desired_dpr =
                if role.as_deref() == Some("scroller-canvas-host") && key != "single" {
                    // Large browser-owned scrollers are the current perf hotspot. Once a scroller is
                    // tiled, favor fewer and cheaper pixels over matching the window DPR exactly.
                    TILED_SCROLLER_DESIRED_DPR
                } else {
                    desired_dpr
                };
            let surface = compute_surface_metrics(
                planned_canvas_logical_width(&canvas),
                planned_canvas_logical_height(&canvas),
                surface_desired_dpr,
                max_surface_dimension,
                minimum_dpr,
            );
            LayerCanvasTarget {
                key,
                host_signature,
                canvas,
                origin_x,
                origin_y,
                replay_priority,
                active,
                surface,
            }
        })
        .collect()
}

fn build_layer_surface_layout(
    document: &Document,
    layer: usize,
    desired_dpr: f64,
    max_surface_dimension: u32,
    minimum_dpr: f64,
    defer_transient_root_host_surfaces: bool,
) -> LayerSurfaceLayout {
    let targets = query_layer_canvas_targets(
        document,
        layer,
        desired_dpr,
        max_surface_dimension,
        minimum_dpr,
        defer_transient_root_host_surfaces,
    );
    layer_surface_layout_from_targets(targets.iter())
}

fn layer_surface_layout_from_targets<'a>(
    targets: impl IntoIterator<Item = &'a LayerCanvasTarget>,
) -> LayerSurfaceLayout {
    let mut active = false;
    let surfaces = targets
        .into_iter()
        .map(|target| {
            active |= target.active;
            layer_surface_entry_from_target(target)
        })
        .collect();
    LayerSurfaceLayout { surfaces, active }
}

fn layer_surface_entry_from_target(target: &LayerCanvasTarget) -> LayerSurfaceEntry {
    LayerSurfaceEntry {
        key: target.key.clone(),
        host_signature: target.host_signature.clone(),
        origin_x: target.origin_x,
        origin_y: target.origin_y,
        replay_priority: target.replay_priority,
        surface: LayerSurfaceSize {
            logical_width: target.surface.logical_width,
            logical_height: target.surface.logical_height,
            surface_width: target.surface.surface_width,
            surface_height: target.surface.surface_height,
            dpr: target.surface.dpr,
        },
    }
}

fn configure_piet_canvas_context(
    context: &web_sys::CanvasRenderingContext2d,
    canvas: &HtmlCanvasElement,
    origin_x: f32,
    origin_y: f32,
    surface_width: u32,
    surface_height: u32,
    dpr: [f32; 2],
    resize_surface: bool,
) {
    if resize_surface || canvas.width() != surface_width || canvas.height() != surface_height {
        canvas.set_width(surface_width);
        canvas.set_height(surface_height);
    }
    let _ = context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
    let _ = context.scale(dpr[0] as f64, dpr[1] as f64);
    let _ = context.translate(-(origin_x as f64), -(origin_y as f64));
}

fn clear_piet_canvas_context(
    context: &web_sys::CanvasRenderingContext2d,
    canvas: &HtmlCanvasElement,
) {
    let _ = context.save();
    let _ = context.set_transform(1.0, 0.0, 0.0, 1.0, 0.0, 0.0);
    context.clear_rect(0.0, 0.0, canvas.width() as f64, canvas.height() as f64);
    let _ = context.restore();
}

fn query_layer_canvases(document: &Document, layer: usize) -> Vec<HtmlCanvasElement> {
    let mut canvases = Vec::new();
    let layer_marker = layer.to_string();
    let node_list = document.get_elements_by_tag_name("canvas");
    for index in 0..node_list.length() {
        let Some(node) = node_list.item(index) else {
            continue;
        };
        let Ok(canvas) = node.dyn_into::<HtmlCanvasElement>() else {
            continue;
        };
        let layer_id = canvas.get_attribute("data-layer-id");
        if layer_id.as_deref() != Some(layer_marker.as_str()) {
            continue;
        }
        canvases.push(canvas);
    }
    if canvases.is_empty() {
        if let Some(canvas) = document
            .get_element_by_id(layer.to_string().as_str())
            .and_then(|node| node.dyn_into::<HtmlCanvasElement>().ok())
        {
            canvases.push(canvas);
        }
    }
    canvases.sort_by(|left, right| {
        // The engine plans ring-stable physical tile slots; DOM origin can change when a slot is
        // reused for a new content tile. Keep layout-provider order aligned with renderer identity.
        parse_tile_key(left)
            .cmp(&parse_tile_key(right))
            .then(left.id().cmp(&right.id()))
    });
    canvases
}

fn parse_tile_key(canvas: &HtmlCanvasElement) -> (i32, i32) {
    canvas
        .get_attribute("data-tile-key")
        .and_then(|key| {
            let (column, row) = key.split_once(':')?;
            Some((column.parse().ok()?, row.parse().ok()?))
        })
        .unwrap_or((0, 0))
}

fn canvas_layout_signature(canvas: &HtmlCanvasElement) -> String {
    let host_signature = attached_canvas_host_signature(canvas);
    format!(
        "{}x{}@{}",
        planned_canvas_logical_width(canvas),
        planned_canvas_logical_height(canvas),
        host_signature,
    )
}

fn canvas_render_state(canvas: &HtmlCanvasElement) -> String {
    canvas
        .parent_element()
        .and_then(|node| node.get_attribute("data-render-state"))
        .unwrap_or_else(|| "active".to_string())
}

fn canvas_host_signature(canvas: &HtmlCanvasElement) -> String {
    if let Some(signature) = canvas.get_attribute("data-host-signature") {
        if !signature.is_empty() {
            return signature;
        }
    }
    let parent = canvas.parent_element();
    let parent_role = parent
        .as_ref()
        .and_then(|node| node.get_attribute("data-role"));
    let scroller_id = parent
        .as_ref()
        .and_then(|node| node.get_attribute("data-scroller-id"));
    if let (Some(role), Some(scroller_id)) = (parent_role, scroller_id) {
        return format!("{role}:{scroller_id}");
    }
    parent
        .as_ref()
        .and_then(|node| node.get_attribute("data-role"))
        .or_else(|| {
            parent
                .as_ref()
                .and_then(|node| node.get_attribute("pax_id"))
        })
        .or_else(|| parent.as_ref().and_then(|node| node.get_attribute("class")))
        .unwrap_or_default()
}

fn planned_canvas_logical_width(canvas: &HtmlCanvasElement) -> f64 {
    canvas
        .get_attribute("data-logical-width")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(canvas.client_width() as f64)
}

fn planned_canvas_logical_height(canvas: &HtmlCanvasElement) -> f64 {
    canvas
        .get_attribute("data-logical-height")
        .and_then(|value| value.parse::<f64>().ok())
        .unwrap_or(canvas.client_height() as f64)
}

fn attached_canvas_host_signature(canvas: &HtmlCanvasElement) -> String {
    let parent = canvas.parent_element();
    let parent_role = parent
        .as_ref()
        .and_then(|node| node.get_attribute("data-role"));
    let scroller_id = parent
        .as_ref()
        .and_then(|node| node.get_attribute("data-scroller-id"));
    if let (Some(role), Some(scroller_id)) = (parent_role, scroller_id) {
        return format!("{role}:{scroller_id}");
    }
    parent
        .as_ref()
        .and_then(|node| node.get_attribute("data-role"))
        .or_else(|| {
            parent
                .as_ref()
                .and_then(|node| node.get_attribute("pax_id"))
        })
        .or_else(|| parent.as_ref().and_then(|node| node.get_attribute("class")))
        .unwrap_or_else(|| canvas_host_signature(canvas))
}

async fn wait_for_animation_frame(window: &Window) {
    let mut maybe_window = Some(window.clone());
    let promise = js_sys::Promise::new(&mut move |resolve, _reject| {
        let Some(window) = maybe_window.take() else {
            let _ = resolve.call0(&JsValue::UNDEFINED);
            return;
        };
        let resolve_for_callback = resolve.clone();
        let callback = Closure::once(move || {
            let _ = resolve_for_callback.call0(&JsValue::UNDEFINED);
        });
        if window
            .request_animation_frame(callback.as_ref().unchecked_ref())
            .is_err()
        {
            let _ = resolve.call0(&JsValue::UNDEFINED);
        }
        callback.forget();
    });
    let _ = JsFuture::from(promise).await;
}

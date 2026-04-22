use std::pin::Pin;

use crate::browser_surface_policy::BrowserSurfacePolicy;
use pax_runtime::api::RenderContext;
#[cfg(not(feature = "piet"))]
use pax_runtime::pax_gpu_render_context::{
    LayerSurfaceEntry, LayerSurfaceLayout, LayerSurfaceSize,
};
use wasm_bindgen::JsCast;
use wasm_bindgen::{closure::Closure, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{Document, HtmlCanvasElement, Window};
#[cfg(not(feature = "piet"))]

struct SurfaceMetrics {
    logical_width: f32,
    logical_height: f32,
    surface_width: u32,
    surface_height: u32,
    dpr: [f32; 2],
}

#[cfg(not(feature = "piet"))]
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

#[cfg(not(feature = "piet"))]
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

#[cfg(feature = "piet")]
pub(crate) fn get_render_context(
    window: Window,
    _surface_policy: BrowserSurfacePolicy,
) -> impl RenderContext {
    use pax_runtime::piet_render_context::PietRenderer;
    use piet_web::WebRenderContext;
    PietRenderer::new(move |layer| {
        let dpr = window.device_pixel_ratio();
        let document = window.document().unwrap();
        let canvas = document
            .get_element_by_id(layer.to_string().as_str())
            .unwrap()
            .dyn_into::<HtmlCanvasElement>()
            .unwrap();
        let context: web_sys::CanvasRenderingContext2d = canvas
            .get_context("2d")
            .unwrap()
            .unwrap()
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .unwrap();

        let width = canvas.offset_width() as f64 * dpr;
        let height = canvas.offset_height() as f64 * dpr;

        canvas.set_width(width as u32);
        canvas.set_height(height as u32);
        let _ = context.scale(dpr, dpr);

        (
            WebRenderContext::new(context.clone(), window.clone()),
            // clear fn
            Box::new({
                let context = context.clone();
                let canvas = canvas.clone();
                move || {
                    let w = canvas.width();
                    let h = canvas.height();
                    context.clear_rect(0.0, 0.0, w as f64, h as f64);
                }
            }),
            // resize fn
            Box::new({
                let window = window.clone();
                move || {
                    let dpr = window.device_pixel_ratio();
                    canvas.set_width((canvas.client_width() as f64 * dpr) as u32);
                    canvas.set_height((canvas.client_height() as f64 * dpr) as u32);
                    let _ = context.scale(dpr, dpr);
                }
            }),
        )
    })
}

#[cfg(not(feature = "piet"))]
pub(crate) fn get_render_context(
    window: Window,
    surface_policy: BrowserSurfacePolicy,
) -> impl RenderContext {
    use pax_gpu::{
        render_backend::{RenderBackend, RenderConfig},
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
            if surface_policy.force_gl() {
                #[cfg(feature = "webgl")]
                log::info!(
                    "render backend: forcing GL canvas path for iOS WebKit browser surfaces"
                );
                #[cfg(not(feature = "webgl"))]
                log::warn!(
                    "render backend: iOS WebKit requested GL canvas path, but this build was compiled without WebGL fallback"
                );
            }
            let force_gl = surface_policy.force_gl() && cfg!(feature = "webgl");

            let mut renderers = Vec::with_capacity(initial_targets.len());
            let mut backend_limit = u32::MAX;
            for target in &initial_targets {
                target.canvas.set_width(target.surface.surface_width);
                target.canvas.set_height(target.surface.surface_height);

                let backend = match if force_gl {
                    RenderBackend::to_canvas_gl(
                        target.canvas.clone(),
                        RenderConfig::new(
                            false,
                            target.surface.surface_width,
                            target.surface.surface_height,
                            target.surface.dpr,
                        ),
                    )
                    .await
                } else {
                    RenderBackend::to_canvas(
                        target.canvas.clone(),
                        RenderConfig::new(
                            false,
                            target.surface.surface_width,
                            target.surface.surface_height,
                            target.surface.dpr,
                        ),
                    )
                    .await
                } {
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
                renderer.renderer_mut().resize_surface(
                    surface.surface.surface_width as f32,
                    surface.surface.surface_height as f32,
                );
                renderer.renderer_mut().set_viewport(
                    surface.surface.logical_width,
                    surface.surface.logical_height,
                    surface.surface.dpr,
                );
            }
            Some((target, resize_provider))
        })
    })
}

#[cfg(not(feature = "piet"))]
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
            && canvases
                .iter()
                .all(|canvas| canvas.offset_width() > 0 && canvas.offset_height() > 0);
        if has_size && signature == last_signature {
            stable_frames += 1;
        } else {
            stable_frames = 0;
            last_signature = signature;
        }
        if has_size && stable_frames >= 1 {
            // Non-root layers often appear on the root host for a frame before an explicit
            // scroller-island claim rebinds them into their browser-owned canvas host. Waiting for
            // one stable frame avoids bootstrapping GL contexts against that transient root
            // placement, which is especially costly on iOS WebKit where the total context budget is
            // small.
            return;
        }
        wait_for_animation_frame(window).await;
    }
}

#[cfg(not(feature = "piet"))]
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

#[cfg(not(feature = "piet"))]
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
                canvas.client_width() as f64,
                canvas.client_height() as f64,
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

#[cfg(not(feature = "piet"))]
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
    let active = targets.iter().any(|target| target.active);
    LayerSurfaceLayout {
        surfaces: targets
            .into_iter()
            .map(|target| LayerSurfaceEntry {
                key: target.key,
                host_signature: target.host_signature,
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
            })
            .collect(),
        active,
    }
}

#[cfg(not(feature = "piet"))]
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

#[cfg(not(feature = "piet"))]
fn parse_tile_key(canvas: &HtmlCanvasElement) -> (i32, i32) {
    canvas
        .get_attribute("data-tile-key")
        .and_then(|key| {
            let (column, row) = key.split_once(':')?;
            Some((column.parse().ok()?, row.parse().ok()?))
        })
        .unwrap_or((0, 0))
}

#[cfg(not(feature = "piet"))]
fn canvas_layout_signature(canvas: &HtmlCanvasElement) -> String {
    let host_signature = canvas_host_signature(canvas);
    format!(
        "{}x{}@{}",
        canvas.client_width(),
        canvas.client_height(),
        host_signature,
    )
}

#[cfg(not(feature = "piet"))]
fn canvas_render_state(canvas: &HtmlCanvasElement) -> String {
    canvas
        .parent_element()
        .and_then(|node| node.get_attribute("data-render-state"))
        .unwrap_or_else(|| "active".to_string())
}

#[cfg(not(feature = "piet"))]
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

#[cfg(not(feature = "piet"))]
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

use std::pin::Pin;

use crate::browser_surface_policy::{BrowserSurfacePolicy, WebSurfacePolicy};
use crate::installed_surfaces::{InstalledSurface, InstalledSurfaces};
use pax_runtime::api::RenderContext;
use pax_runtime::engine::layer_surface::{LayerSurfaceEntry, LayerSurfaceLayout, LayerSurfaceSize};
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use web_sys::{Document, HtmlCanvasElement, Window};

#[cfg(test)]
use crate::surface_metrics::projected_surface_metrics;
use crate::surface_metrics::{installed_surface_metrics, SurfaceMetrics};

fn trace_surface(canvas: &HtmlCanvasElement, event: &str) {
    #[cfg(target_arch = "wasm32")]
    pax_gpu::render_backend::record_browser_surface_event(canvas, event);
    #[cfg(not(target_arch = "wasm32"))]
    let _ = (canvas, event);
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

fn report_density_fallback(
    canvas: &HtmlCanvasElement,
    surface: &SurfaceMetrics,
    max_surface_dimension: u32,
) {
    const ATTRIBUTE: &str = "data-pax-resolution-fallback";
    // Ignore normal subpixel-to-integer rounding. A real allocation clamp is
    // inspectable on the canvas and logged once per changed fallback, not frame.
    if surface.density_clamped {
        let detail = format!(
            "actual={:.3}x{:.3};limit={max_surface_dimension}",
            surface.dpr[0], surface.dpr[1],
        );
        if canvas.get_attribute(ATTRIBUTE).as_deref() != Some(detail.as_str()) {
            log::warn!(
                "render backend: canvas {} reduced resolution to fit a hard surface limit ({})",
                canvas.id(),
                detail,
            );
            let _ = canvas.set_attribute(ATTRIBUTE, &detail);
        }
    } else if canvas.has_attribute(ATTRIBUTE) {
        let _ = canvas.remove_attribute(ATTRIBUTE);
    }
}

/// Renderer selected and initialized before application state or templates mount.
/// Prepare outside an `ApplicationInstance` enter guard: WebGPU initialization awaits JavaScript.
pub struct PreparedWebRenderer {
    pub(crate) policy: WebSurfacePolicy,
    pub(crate) installed: InstalledSurfaces,
    pub(crate) renderer: Box<dyn RenderContext>,
}

/// Select WebGPU or Vello CPU for this session. Automatic selection falls back
/// when adapter/device initialization fails. `?pax_renderer=cpu|gpu|auto` can
/// override selection for testing; an explicitly requested GPU must initialize.
pub async fn prepare_renderer() -> Result<PreparedWebRenderer, JsValue> {
    use pax_gpu::render_backend::{RenderBackend, RenderConfig};
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("missing browser window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("missing browser document"))?;
    let params = web_sys::UrlSearchParams::new_with_str(&window.location().search()?)?;
    let requested = params.get("pax_renderer").unwrap_or_else(|| "auto".into());
    let mut browser = BrowserSurfacePolicy::detect(&window);
    match requested.as_str() {
        "auto" => {
            if cfg!(feature = "cpu") {
                browser.select_cpu(true);
            }
        }
        "cpu" => browser.select_cpu(true),
        "gpu" => browser.select_cpu(false),
        _ => return Err(JsValue::from_str("pax_renderer must be auto, cpu, or gpu")),
    }
    let probe = document
        .create_element("canvas")?
        .dyn_into::<HtmlCanvasElement>()?;
    let mut shared = None;
    let mut limit = 2048;
    if !browser.use_cpu_fallback() {
        match RenderBackend::to_canvas_with_context(
            probe.clone(),
            RenderConfig::new(false, 1, 1, [1.0, 1.0]).with_browser_premultiplied_alpha(true),
            None,
        )
        .await
        {
            Ok((backend, context)) => {
                limit = backend.max_surface_dimension();
                shared = Some(context);
            }
            Err(error) if requested == "gpu" => {
                return Err(JsValue::from_str(&format!(
                    "WebGPU initialization failed: {error}"
                )))
            }
            Err(error) => {
                log::warn!("WebGPU initialization failed; selecting Vello CPU: {error}");
                browser.select_cpu(true);
            }
        }
    }
    let policy = WebSurfacePolicy::new(browser);
    policy.observe_backend_limit(limit);
    let installed = std::rc::Rc::new(std::cell::RefCell::new(Default::default()));
    let renderer: Box<dyn RenderContext> = if browser.use_cpu_fallback() {
        // A failed GPU probe may already own its canvas context. Verify CPU
        // presentation on a fresh canvas, never on that consumed target.
        let canvas = document
            .create_element("canvas")?
            .dyn_into::<HtmlCanvasElement>()?;
        canvas
            .get_context("2d")?
            .ok_or_else(|| JsValue::from_str("Canvas2D pixel presentation unavailable"))?;
        log::info!("render backend: using Vello CPU browser renderer");
        Box::new(get_cpu_render_context(
            window,
            policy.clone(),
            installed.clone(),
        ))
    } else {
        log::info!("render backend: using WebGPU browser renderer");
        Box::new(get_gpu_render_context(
            window,
            policy.clone(),
            installed.clone(),
            shared,
        ))
    };
    Ok(PreparedWebRenderer {
        policy,
        installed,
        renderer,
    })
}

struct WebCpuSurface {
    canvas: HtmlCanvasElement,
    context: web_sys::CanvasRenderingContext2d,
}

impl WebCpuSurface {
    fn new(canvas: HtmlCanvasElement) -> Self {
        let context = canvas
            .get_context("2d")
            .expect("create CPU presentation context")
            .expect("Canvas2D pixel presentation unavailable")
            .dyn_into::<web_sys::CanvasRenderingContext2d>()
            .expect("Canvas2D context");
        Self { canvas, context }
    }
}

impl pax_runtime::vello_render_context::CpuSurface for WebCpuSurface {
    fn configure(&mut self, size: (u32, u32)) {
        if self.canvas.width() != size.0 {
            self.canvas.set_width(size.0);
        }
        if self.canvas.height() != size.1 {
            self.canvas.set_height(size.1);
        }
    }

    fn present(&mut self, rgba: &[u8], width: u32, height: u32, origin: (u32, u32)) {
        let image = web_sys::ImageData::new_with_u8_clamped_array_and_sh(
            wasm_bindgen::Clamped(rgba),
            width,
            height,
        )
        .expect("CPU framebuffer dimensions");
        self.context
            .put_image_data(&image, f64::from(origin.0), f64::from(origin.1))
            .expect("present CPU framebuffer");
        trace_surface(&self.canvas, "first-submit");
    }
}

fn get_cpu_render_context(
    window: Window,
    policy: WebSurfacePolicy,
    installed: InstalledSurfaces,
) -> impl RenderContext {
    use pax_runtime::vello_render_context::{CpuLayerRenderer, CpuLayerTarget, CpuRenderer};

    CpuRenderer::new(move |layer| {
        let document = window.document().unwrap();
        let policy = policy.clone();
        let targets = query_layer_canvas_targets(
            &installed,
            &document,
            layer,
            policy.max_surface_dimension(layer),
            policy.browser.defer_transient_root_host_surfaces(layer),
        );
        let layout = layer_surface_layout_from_targets(targets.iter());
        let renderers = targets
            .into_iter()
            .map(|target| {
                let entry = layer_surface_entry_from_target(&target);
                let surface = WebCpuSurface::new(target.canvas.clone());
                CpuLayerRenderer::new(
                    target.key.clone(),
                    target.host_signature.clone(),
                    surface,
                    &entry,
                )
            })
            .collect();
        let layout_provider: Box<dyn Fn() -> LayerSurfaceLayout> = Box::new({
            let document = document.clone();
            let installed = installed.clone();
            move || {
                build_layer_surface_layout(
                    &installed,
                    &document,
                    layer,
                    policy.max_surface_dimension(layer),
                    policy.browser.defer_transient_root_host_surfaces(layer),
                )
            }
        });

        (
            CpuLayerTarget::new(renderers, layout.active),
            layout_provider,
        )
    })
}

fn get_gpu_render_context(
    window: Window,
    policy: WebSurfacePolicy,
    installed: InstalledSurfaces,
    shared: Option<pax_gpu::render_backend::SharedGpuContext>,
) -> impl RenderContext {
    let surface_policy = policy.browser;
    use pax_gpu::{
        render_backend::{RenderBackend, RenderConfig},
        Transform2D, WgpuRenderer,
    };
    use pax_runtime::pax_gpu_render_context::{LayerRenderer, LayerTarget, PaxGpuRenderer};
    // Every browser canvas in this chassis can use the same WebGPU device.
    // Keep it across cold-row eviction; pixel textures still have weak cache ownership.
    // The async mutex prevents concurrent first rows from each creating a device.
    let browser_context = std::rc::Rc::new(futures_util::lock::Mutex::new(shared));
    let priority_document = window.document().unwrap();
    let priority_installed = installed.clone();
    PaxGpuRenderer::new(move |layer, request| {
        let window = window.clone();
        let surface_policy = surface_policy;
        let policy = policy.clone();
        let installed = installed.clone();
        let browser_context = browser_context.clone();
        Box::pin(async move {
            let document = window.document().unwrap();
            // Waiting for a shared context is not permission to retain stale
            // urgency. Select from the latest committed plan after the await.
            let mut browser_context = browser_context.lock().await;
            let current_priority = installed
                .borrow()
                .initialization_priority(None, |s| unowned_installed_canvas(&document, s));
            let initial_targets = query_layer_canvas_targets(
                &installed,
                &document,
                layer,
                surface_policy
                    .effective_max_surface_dimension(layer, request.max_surface_dimension),
                surface_policy.defer_transient_root_host_surfaces(layer),
            );
            let mut renderers = Vec::with_capacity(initial_targets.len());
            let mut backend_limit = request.max_surface_dimension;
            // Only committed targets enter this factory. Revalidate assignment
            // identity after this await and after asynchronous GPU creation.
            let mut shared_context = browser_context.clone().or(request.shared_context.clone());
            for target in initial_targets
                .iter()
                .filter(|target| request.needs_surface(&target.key, &target.host_signature))
                .filter(|target| {
                    target.active
                        && target.replay_priority <= current_priority
                        && current_priority != i32::MAX
                })
                .take(1)
            {
                // Earlier surfaces can await device creation. A later canvas in this
                // snapshot may have been reassigned while we were suspended.
                if canvas_host_signature(&target.canvas) != target.host_signature {
                    trace_surface(&target.canvas, "stale-initialization-rejected");
                    continue;
                }
                if !request.needs_surface(&target.key, &target.host_signature) {
                    continue;
                }
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
                        trace_surface(&target.canvas, "initialization-failed");
                        log::warn!(
                            "failed to create browser render backend for layer {} tile {}: {}",
                            layer,
                            target.key,
                            err
                        );
                        return None;
                    }
                };
                trace_surface(&target.canvas, "initialization-complete");
                browser_context.get_or_insert_with(|| context.clone());
                shared_context.get_or_insert(context);

                if canvas_host_signature(&target.canvas) != target.host_signature {
                    trace_surface(&target.canvas, "stale-initialization-rejected");
                    // The lease stays with `backend` until it is dropped here;
                    // never publish a late result into a retired assignment.
                    continue;
                }

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
                policy.observe_backend_limit(backend_limit);
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
            drop(browser_context);

            let effective_max_surface_dimension =
                surface_policy.effective_max_surface_dimension(layer, backend_limit);
            let resize_provider = Box::pin({
                let document = document.clone();
                let installed = installed.clone();
                move || {
                    build_layer_surface_layout(
                        &installed,
                        &document,
                        layer,
                        effective_max_surface_dimension,
                        surface_policy.defer_transient_root_host_surfaces(layer),
                    )
                }
            }) as Pin<Box<dyn Fn() -> LayerSurfaceLayout>>;
            let layout = resize_provider();
            // Browser-owned scroller layers can intentionally shed every canvas target while they
            // are well outside the viewport. Keep an empty target alive here so later layout syncs
            // can rehydrate the renderer instead of treating "no surfaces right now" as fatal.
            let mut target =
                LayerTarget::new(renderers, layout.active).with_incremental_initialization();
            for renderer in target.renderers_mut() {
                let Some(surface) = layout
                    .surfaces
                    .iter()
                    .find(|surface| renderer.matches_surface(surface))
                else {
                    continue;
                };
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
    .with_surface_content_preparation()
    .with_layer_initialization_priority(move |layer| {
        priority_installed
            .borrow()
            .initialization_priority(Some(layer), |s| {
                unowned_installed_canvas(&priority_document, s)
            })
    })
}

fn unowned_installed_canvas(document: &Document, surface: &InstalledSurface) -> bool {
    installed_canvas(document, surface).is_some_and(|canvas| {
        canvas
            .get_attribute("data-pax-surface-owners")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0)
            == 0
    })
}

fn installed_canvas(document: &Document, surface: &InstalledSurface) -> Option<HtmlCanvasElement> {
    let canvas: HtmlCanvasElement = document.get_element_by_id(&surface.id)?.dyn_into().ok()?;
    if canvas.get_attribute("data-surface-generation").as_deref()
        != Some(surface.generation.as_str())
        || canvas_host_owner_signature(&canvas) != surface.host_signature
        || canvas.parent_element().is_none()
    {
        return None;
    }
    Some(canvas)
}

fn query_layer_canvas_targets(
    installed: &InstalledSurfaces,
    document: &Document,
    layer: usize,
    max_surface_dimension: u32,
    defer_transient_root_host_surfaces: bool,
) -> Vec<LayerCanvasTarget> {
    let registry = installed.borrow();
    let mut targets: Vec<_> = registry
        .surfaces
        .iter()
        .filter(|s| s.layer_id == layer)
        .filter_map(|s| {
            let canvas = installed_canvas(document, s)?;
            if defer_transient_root_host_surfaces
                && canvas
                    .parent_element()?
                    .get_attribute("data-role")
                    .is_none()
            {
                return None;
            }
            // A backend limit may have changed since planning. Wait for the
            // policy-revision replan rather than install a different size.
            if s.backing_width > max_surface_dimension || s.backing_height > max_surface_dimension {
                return None;
            }
            let surface = installed_surface_metrics(
                s.width,
                s.height,
                [s.backing_width, s.backing_height],
                s.density_clamped,
            );
            report_density_fallback(&canvas, &surface, max_surface_dimension);
            Some(LayerCanvasTarget {
                key: s.key.clone(),
                host_signature: canvas_host_signature(&canvas),
                canvas,
                origin_x: s.left as f32,
                origin_y: s.top as f32,
                replay_priority: s.replay_priority,
                active: s.active,
                surface,
            })
        })
        .collect();
    targets.sort_by_key(|target| target.replay_priority);
    targets
}

fn build_layer_surface_layout(
    installed: &InstalledSurfaces,
    document: &Document,
    layer: usize,
    max_surface_dimension: u32,
    defer_transient_root_host_surfaces: bool,
) -> LayerSurfaceLayout {
    let targets = query_layer_canvas_targets(
        installed,
        document,
        layer,
        max_surface_dimension,
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

thread_local! {
    static CANVAS_GENERATIONS: (js_sys::WeakMap, std::cell::Cell<u32>) =
        (js_sys::WeakMap::new(), std::cell::Cell::new(0));
}

fn canvas_host_signature(canvas: &HtmlCanvasElement) -> String {
    let generation = CANVAS_GENERATIONS.with(|(generations, next)| {
        let object: &js_sys::Object = canvas.unchecked_ref();
        if let Some(generation) = generations.get(object).as_f64() {
            return generation;
        }
        let generation = next
            .get()
            .checked_add(1)
            .expect("canvas generation exhausted");
        next.set(generation);
        generations.set(object, &JsValue::from_f64(generation as f64));
        generation as f64
    });
    format!(
        "{}|surface:{}|lease:{}",
        canvas_host_owner_signature(canvas),
        generation,
        canvas
            .get_attribute("data-surface-generation")
            .unwrap_or_default(),
    )
}

fn canvas_host_owner_signature(canvas: &HtmlCanvasElement) -> String {
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

#[cfg(test)]
mod density_tests {
    use super::*;

    #[test]
    fn carousel_tiles_match_screen_density() {
        for dpr in [1.0, 1.25, 1.5, 2.0, 3.0] {
            let surface = projected_surface_metrics(1248.0, 534.0, [dpr; 2], 4096);
            assert_eq!(surface.surface_width, (1248.0 * dpr).ceil() as u32);
            assert_eq!(surface.surface_height, (534.0 * dpr).ceil() as u32);
            assert!((surface.dpr[0] as f64 - dpr).abs() < 0.002);
            assert!((surface.dpr[1] as f64 - dpr).abs() < 0.002);
        }
    }

    #[test]
    fn fractional_logical_bounds_only_round_to_physical_pixels() {
        let surface = projected_surface_metrics(73.8181, 534.25, [2.0; 2], 4096);
        assert_eq!((surface.surface_width, surface.surface_height), (148, 1069));
        assert_eq!(surface.logical_width, 73.8181_f32);
        assert_eq!(surface.logical_height, 534.25);
        assert!(!surface.density_clamped);
        let tiny = projected_surface_metrics(0.6, 0.6, [2.0; 2], 4096);
        assert!(!tiny.density_clamped);
    }

    #[test]
    fn hard_limits_remain_a_bounded_last_resort() {
        let surface = projected_surface_metrics(3000.0, 600.0, [3.0; 2], 2048);
        assert_eq!(
            (surface.surface_width, surface.surface_height),
            (2048, 1800)
        );
        assert!(surface.dpr[0] < 1.0);
        assert_eq!(surface.dpr[1], 3.0);
        assert!(surface.density_clamped);
        // Smaller tiles restore density, rather than retaining a degraded mode.
        let tile = projected_surface_metrics(682.0, 600.0, [3.0; 2], 2048);
        assert_eq!((tile.surface_width, tile.surface_height), (2046, 1800));
        assert_eq!(tile.dpr, [3.0, 3.0]);
        assert!(!tile.density_clamped);
    }
}

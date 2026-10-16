use super::{DesktopGpuBackend, DesktopGpuFactory, DesktopSurfaceFrame, DesktopSurfaceInfo};
use crate::native_interrupt::NativeInterruptDispatcher;
use pax_gpu::render_backend::{RenderBackend, RenderConfig};
use pax_gpu::{Transform2D, WgpuRenderer, NATIVE_VECTOR_RESOURCE_CACHE_BYTES};
use pax_message::{NativeInterrupt, NativeMessage};
use pax_runtime::api::{RenderContext, StopReason};
use pax_runtime::engine::layer_surface::{LayerSurfaceEntry, LayerSurfaceLayout, LayerSurfaceSize};
use pax_runtime::engine::layer_tiling::{scroller_canvas_plan_with_policy, ScrollerTilingPolicy};
use pax_runtime::pax_gpu_render_context::{LayerRenderer, LayerTarget, PaxGpuRenderer};
use pax_runtime::PaxEngine;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::ffi::c_void;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::rc::Rc;
use std::thread::{self, ThreadId};

const JSON_LIMIT: usize = 16 * 1024 * 1024;
const MAX_SURFACES: usize = 512;

/// Shared native surface planning limits. GPU exporters independently enforce
/// their byte budgets across live and retiring generations.
pub fn native_scroller_tiling_policy() -> ScrollerTilingPolicy {
    ScrollerTilingPolicy {
        max_tile_backing_width: Some(4096.0),
        max_tile_backing_height: Some(4096.0),
        max_tile_backing_area: Some(4096.0 * 4096.0),
        max_surfaces_per_layer: Some(12),
        ..Default::default()
    }
}

/// Opaque UI-thread session. Native code must never inspect its Rust layout.
pub struct DesktopEngineContainer {
    owner: ThreadId,
    session: RefCell<Session>,
}

struct Session {
    engine: PaxEngine,
    interrupts: NativeInterruptDispatcher,
    factory: DesktopGpuFactory,
    gpu: Option<Box<dyn DesktopGpuBackend>>,
    renderer: Option<PaxGpuRenderer>,
    surfaces: Rc<RefCell<Vec<DesktopSurfaceInfo>>>,
    renderer_error: Rc<RefCell<Option<String>>>,
    published: HashMap<(usize, String, String), u64>,
    closing: bool,
    fatal_error: Option<String>,
    #[cfg(feature = "designtime")]
    traverser: Option<Box<dyn pax_runtime::cartridge::DefinitionToInstanceTraverser>>,
    #[cfg(feature = "designtime")]
    manager: Option<Rc<RefCell<pax_designtime::DesigntimeManager>>>,
}

impl DesktopEngineContainer {
    /// Retains a prepared engine and creates its negotiated GPU lazily, on the
    /// same UI thread. Application activation remains an explicit host step.
    pub fn new(engine: PaxEngine, factory: DesktopGpuFactory) -> Self {
        Self {
            owner: thread::current().id(),
            session: RefCell::new(Session {
                engine,
                factory,
                interrupts: Default::default(),
                gpu: None,
                renderer: None,
                surfaces: Default::default(),
                renderer_error: Default::default(),
                published: Default::default(),
                closing: false,
                fatal_error: None,
                #[cfg(feature = "designtime")]
                traverser: None,
                #[cfg(feature = "designtime")]
                manager: None,
            }),
        }
    }

    /// Retains the application's own traverser and reload manager in Debug.
    #[cfg(feature = "designtime")]
    pub fn new_designtime(
        engine: PaxEngine,
        traverser: Box<dyn pax_runtime::cartridge::DefinitionToInstanceTraverser>,
        manager: Rc<RefCell<pax_designtime::DesigntimeManager>>,
        factory: DesktopGpuFactory,
    ) -> Self {
        let container = Self::new(engine, factory);
        {
            let mut session = container.session.borrow_mut();
            session.traverser = Some(traverser);
            session.manager = Some(manager);
        }
        container
    }
}

fn surface_layout(surfaces: &[DesktopSurfaceInfo], layer: usize) -> LayerSurfaceLayout {
    let surfaces: Vec<_> = surfaces
        .iter()
        .filter(|s| s.layer == layer)
        .map(|s| LayerSurfaceEntry {
            key: s.key.clone(),
            host_signature: s.host_signature.clone(),
            origin_x: s.origin_x,
            origin_y: s.origin_y,
            replay_priority: s.replay_priority,
            surface: LayerSurfaceSize {
                logical_width: s.logical_width,
                logical_height: s.logical_height,
                surface_width: s.width,
                surface_height: s.height,
                dpr: s.dpr,
            },
        })
        .collect();
    LayerSurfaceLayout {
        active: !surfaces.is_empty(),
        surfaces,
    }
}

fn validate_surfaces(surfaces: &[DesktopSurfaceInfo]) -> Result<(), String> {
    if surfaces.len() > MAX_SURFACES {
        return Err("desktop surface count exceeds 512".into());
    }
    let mut keys = HashSet::new();
    for s in surfaces {
        if s.layer >= MAX_SURFACES
            || s.key.is_empty()
            || s.key.len() > 1024
            || s.host_signature.is_empty()
            || s.host_signature.len() > 4096
            || !keys.insert((s.layer, &s.key))
        {
            return Err("invalid or duplicate desktop surface identity".into());
        }
        if s.width == 0
            || s.height == 0
            || s.width > 16384
            || s.height > 16384
            || !s.origin_x.is_finite()
            || !s.origin_y.is_finite()
            || !s.logical_width.is_finite()
            || !s.logical_height.is_finite()
            || s.logical_width <= 0.0
            || s.logical_height <= 0.0
            || s.dpr
                .iter()
                .any(|v| !v.is_finite() || *v <= 0.0 || *v > 16.0)
        {
            return Err("invalid desktop surface geometry".into());
        }
    }
    Ok(())
}

fn validate_generations(
    previous: &[DesktopSurfaceInfo],
    next: &[DesktopSurfaceInfo],
) -> Result<(), String> {
    for surface in next {
        if previous.iter().any(|old| {
            old.layer == surface.layer
                && old.key == surface.key
                && old.host_signature == surface.host_signature
                && (old.width != surface.width
                    || old.height != surface.height
                    || old.dpr != surface.dpr)
        }) {
            return Err("resized desktop backing requires a new hostSignature".into());
        }
    }
    Ok(())
}

impl Session {
    fn fail(&mut self, error: String) -> String {
        self.fatal_error.get_or_insert(error).clone()
    }

    fn live(&self) -> Result<(), String> {
        if self.closing {
            return Err("desktop session is closing".into());
        }
        if let Some(error) = &self.fatal_error {
            return Err(error.clone());
        }
        Ok(())
    }

    fn initialize_gpu(&mut self) -> Result<(), String> {
        self.live()?;
        if self.gpu.is_some() {
            return Ok(());
        }
        let gpu = (self.factory)().map_err(|error| self.fail(error))?;
        let context = gpu.context();
        let format = gpu.format();
        if !matches!(
            format,
            wgpu::TextureFormat::Rgba8Unorm | wgpu::TextureFormat::Bgra8Unorm
        ) {
            return Err(
                self.fail("desktop compositor requires premultiplied RGBA8/BGRA8 unorm".into())
            );
        }
        let registry = Rc::clone(&self.surfaces);
        let failure = Rc::clone(&self.renderer_error);
        self.renderer = Some(PaxGpuRenderer::new(move |layer, request| {
            let registry = registry.clone();
            let context = context.clone();
            let failure = failure.clone();
            Box::pin(async move {
                let layout = surface_layout(&registry.borrow(), layer);
                let mut renderers = Vec::new();
                for surface in &layout.surfaces {
                    if !request.needs_surface(&surface.key, &surface.host_signature) {
                        continue;
                    }
                    let size = &surface.surface;
                    let backend = match RenderBackend::to_texture(
                        context.clone(),
                        format,
                        RenderConfig::new(false, size.surface_width, size.surface_height, size.dpr),
                    ) {
                        Ok(backend) => backend,
                        Err(error) => {
                            *failure.borrow_mut() = Some(error.to_string());
                            return None;
                        }
                    };
                    let mut renderer = WgpuRenderer::new(backend);
                    renderer
                        .set_vector_resource_cache_max_bytes(NATIVE_VECTOR_RESOURCE_CACHE_BYTES);
                    renderer.set_surface_transform(Transform2D::from_array([
                        1.0,
                        0.0,
                        0.0,
                        1.0,
                        -surface.origin_x,
                        -surface.origin_y,
                    ]));
                    renderer.set_viewport(size.logical_width, size.logical_height, size.dpr);
                    renderers.push(LayerRenderer::new(
                        surface.key.clone(),
                        surface.host_signature.clone(),
                        renderer,
                        surface.origin_x,
                        surface.origin_y,
                        size.logical_width,
                        size.logical_height,
                        size.surface_width,
                        size.surface_height,
                        size.dpr,
                    ));
                }
                let provider: Pin<Box<dyn Fn() -> LayerSurfaceLayout>> =
                    Box::pin(move || surface_layout(&registry.borrow(), layer));
                Some((LayerTarget::new(renderers, layout.active), provider))
            })
        }));
        self.gpu = Some(gpu);
        Ok(())
    }

    fn tick(&mut self, width: f32, height: f32, dpr: f32) -> Result<Value, String> {
        self.live()?;
        if !width.is_finite()
            || !height.is_finite()
            || width < 0.0
            || height < 0.0
            || !dpr.is_finite()
            || dpr <= 0.0
        {
            return Err("invalid viewport".into());
        }
        #[cfg(feature = "designtime")]
        if let (Some(manager), Some(traverser)) = (&self.manager, &self.traverser) {
            manager
                .borrow_mut()
                .handle_recv(self.engine.runtime_context.get_screenshot_map())
                .map_err(|e| format!("designtime receive failed: {e:?}"))?;
            pax_runtime::designtime_support::apply_designtime_userland_reload(
                &mut self.engine,
                traverser.as_ref(),
                manager,
            );
        }
        self.engine.set_viewport_size((width as f64, height as f64));
        let messages = self.engine.tick();
        for message in &messages {
            if let NativeMessage::ScrollerDelete(id) = message {
                self.interrupts.forget_scroller(*id);
            }
        }
        Ok(
            json!({"ok":true,"messages":messages,"layerCount":self.engine.runtime_context.layer_count.get()}),
        )
    }

    fn plan(&self, layer: usize, dpr: f32) -> Result<Value, String> {
        self.live()?;
        if !dpr.is_finite() || dpr <= 0.0 || layer >= MAX_SURFACES {
            return Err("invalid layer/scale".into());
        }
        let ctx = &self.engine.runtime_context;
        let owner = ctx.get_layer_scroller_owner(layer).map(|id| id.to_u32());
        let mut policy = self.engine.scroller_tiling_policy;
        policy.min_untiled_render_dpr = policy.min_untiled_render_dpr.max(dpr as f64);
        let plan = if let Some(id) = owner {
            ctx.get_scroller_surface_state(id).map(|s| {
                let mut vw = s.viewport_width;
                let mut vh = s.viewport_height;
                let mut x = if s.presentation_scroll_x.is_finite() {
                    s.presentation_scroll_x
                } else {
                    s.scroll_x
                };
                let mut y = if s.presentation_scroll_y.is_finite() {
                    s.presentation_scroll_y
                } else {
                    s.scroll_y
                };
                if ctx.get_root_scroller_id() == Some(id) {
                    if let Some(v) = ctx.get_visual_viewport_state() {
                        vw = v.width;
                        vh = v.height;
                        x = v.page_scroll_x;
                        y = v.page_scroll_y;
                    }
                }
                scroller_canvas_plan_with_policy(
                    layer,
                    format!("scroller:{id}"),
                    s.content_width,
                    s.content_height,
                    vw,
                    vh,
                    x,
                    y,
                    dpr as f64,
                    policy,
                )
            })
        } else {
            // Root layers can be interleaved with native peers, not only layer zero.
            let viewport = ctx.globals().viewport.get();
            let (w, h) = viewport.bounds;
            Some(scroller_canvas_plan_with_policy(
                layer,
                "root".into(),
                w,
                h,
                w,
                h,
                0.0,
                0.0,
                dpr as f64,
                policy,
            ))
        };
        Ok(json!({"ok":true,"plan":plan,"owner":owner}))
    }

    fn render(&mut self) -> Result<Value, String> {
        self.live()?;
        self.render_frame().map_err(|error| self.fail(error))
    }

    fn render_frame(&mut self) -> Result<Value, String> {
        self.initialize_gpu()?;
        let gpu = self.gpu.as_mut().unwrap();
        gpu.poll()?;
        let surfaces = self.surfaces.borrow().clone();
        if !gpu.can_publish(&surfaces)? {
            return Ok(json!({"ok":true,"blocked":true,"frame":null}));
        }
        let renderer = self.renderer.as_mut().unwrap();
        for layer in renderer.take_ready_canvas_layers() {
            self.engine.runtime_context.set_canvas_dirty(layer);
            self.engine
                .runtime_context
                .mark_canvas_nodes_on_layer_dirty(layer);
        }
        for update in renderer.take_replay_canvas_layer_updates() {
            self.engine.runtime_context.request_canvas_replay(update);
        }
        self.engine.render(renderer as &mut dyn RenderContext);
        if let Some(error) = self.renderer_error.borrow_mut().take() {
            return Err(error);
        }
        let mut frames = Vec::new();
        let mut revisions = Vec::new();
        let mut ready = HashSet::new();
        renderer.visit_ready_surfaces(|layer, key, generation, revision, renderer| {
            if revision == 0 {
                return;
            }
            let identity = (layer, key.to_string(), generation.to_string());
            if let Some(info) = surfaces
                .iter()
                .find(|s| s.layer == layer && s.key == key && s.host_signature == generation)
            {
                if let Some(texture) = renderer.output_texture() {
                    ready.insert(identity.clone());
                    if self.published.get(&identity) == Some(&revision) {
                        return;
                    }
                    frames.push(DesktopSurfaceFrame {
                        info: info.clone(),
                        texture,
                    });
                    revisions.push((identity, revision));
                }
            }
        });
        // A new layer/generation may still be waiting for its renderer or first
        // replay. Hosts commit native patches together with this frame, so a
        // partial set cannot be reported as a successful native-only update.
        if ready.len() != surfaces.len() {
            return Ok(json!({"ok":true,"blocked":true,"frame":null}));
        }
        self.published.retain(|identity, _| ready.contains(identity));
        if frames.is_empty() {
            return Ok(json!({"ok":true,"blocked":false,"frame":null}));
        }
        let frame = gpu.publish(frames)?;
        self.published.extend(revisions);
        Ok(json!({"ok":true,"blocked":false,"frame":frame}))
    }

    fn begin_close(&mut self) {
        if self.closing {
            return;
        }
        self.closing = true;
        #[cfg(feature = "designtime")]
        if let Some(manager) = &self.manager {
            manager.borrow_mut().shutdown();
        }
        let reason = if self.engine.runtime_context.application.context().phase()
            == pax_runtime::api::AppPhase::Preparing
        {
            StopReason::PreparationDiscarded
        } else {
            StopReason::HostClosed
        };
        self.engine.shutdown(reason);
    }

    fn poll(&mut self) -> Result<Value, String> {
        let (drained, diagnostics, error) = if let Some(gpu) = &mut self.gpu {
            let mut error = gpu.poll().err();
            // A failed rendering device must still get a chance to prove its
            // resources retired. Never mistake an error for fence completion.
            let drained = if self.closing {
                match gpu.shutdown_poll() {
                    Ok(drained) => drained,
                    Err(failure) => {
                        error.get_or_insert(failure);
                        false
                    }
                }
            } else {
                false
            };
            (drained, gpu.diagnostics(), error)
        } else {
            (self.closing, Value::Null, None)
        };
        if let Some(error) = error {
            self.fail(error);
        }
        Ok(
            json!({"ok":self.fatal_error.is_none(),"error":self.fatal_error,
            "closing":self.closing,"drained":drained,"gpu":diagnostics}),
        )
    }
}

/// Owned byte buffer with an ordinary C layout; free with `pax_desktop_free_buffer`.
#[repr(C)]
pub struct DesktopBuffer {
    pub data: *mut u8,
    pub len: usize,
}

fn buffer(value: Value) -> DesktopBuffer {
    let bytes = serde_json::to_vec(&value)
        .expect("JSON value serializes")
        .into_boxed_slice();
    let len = bytes.len();
    DesktopBuffer {
        data: Box::into_raw(bytes).cast::<u8>(),
        len,
    }
}

fn call(
    container: *mut DesktopEngineContainer,
    f: impl FnOnce(&mut Session) -> Result<Value, String>,
) -> DesktopBuffer {
    let result = catch_unwind(AssertUnwindSafe(|| {
        let container = unsafe { container.as_ref() }.ok_or("null desktop session")?;
        if container.owner != thread::current().id() {
            return Err("desktop session called off its UI thread".into());
        }
        let mut session = container
            .session
            .try_borrow_mut()
            .map_err(|_| "reentrant desktop session call")?;
        match catch_unwind(AssertUnwindSafe(|| f(&mut session))) {
            Ok(result) => result,
            Err(_) => {
                session.fatal_error =
                    Some("desktop runtime operation panicked; close the session".into());
                Err(session.fatal_error.clone().unwrap())
            }
        }
    }));
    buffer(match result {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => json!({"ok":false,"error":error}),
        Err(_) => json!({"ok":false,"error":"desktop ABI operation panicked"}),
    })
}

unsafe fn bytes<'a>(data: *const u8, len: usize, limit: usize) -> Result<&'a [u8], String> {
    if len > limit || (len > 0 && data.is_null()) {
        return Err("invalid or oversized input buffer".into());
    }
    Ok(if len == 0 {
        &[]
    } else {
        unsafe { std::slice::from_raw_parts(data, len) }
    })
}

/// Releases exactly one buffer returned by this cartridge. Null/empty is harmless.
#[no_mangle]
pub unsafe extern "C" fn pax_desktop_free_buffer(value: DesktopBuffer) {
    if !value.data.is_null() {
        drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(value.data, value.len)) });
    }
}

/// Negotiates the platform device before the first native frame.
#[no_mangle]
pub extern "C" fn pax_desktop_initialize_gpu(c: *mut DesktopEngineContainer) -> DesktopBuffer {
    call(c, |s| {
        s.initialize_gpu()?;
        Ok(json!({"ok":true,"gpu":s.gpu.as_ref().unwrap().diagnostics()}))
    })
}
#[no_mangle]
pub extern "C" fn pax_desktop_tick(
    c: *mut DesktopEngineContainer,
    w: f32,
    h: f32,
    dpr: f32,
) -> DesktopBuffer {
    call(c, |s| s.tick(w, h, dpr))
}
#[no_mangle]
pub extern "C" fn pax_desktop_plan(
    c: *mut DesktopEngineContainer,
    layer: u32,
    dpr: f32,
) -> DesktopBuffer {
    call(c, |s| s.plan(layer as usize, dpr))
}
#[no_mangle]
pub unsafe extern "C" fn pax_desktop_set_surfaces(
    c: *mut DesktopEngineContainer,
    data: *const u8,
    len: usize,
) -> DesktopBuffer {
    call(c, |s| {
        s.live()?;
        let surfaces: Vec<DesktopSurfaceInfo> =
            serde_json::from_slice(unsafe { bytes(data, len, JSON_LIMIT)? })
                .map_err(|e| e.to_string())?;
        validate_surfaces(&surfaces)?;
        validate_generations(&s.surfaces.borrow(), &surfaces)?;
        if *s.surfaces.borrow() != surfaces {
            s.published.retain(|(layer, key, generation), _| {
                surfaces
                    .iter()
                    .any(|v| v.layer == *layer && v.key == *key && v.host_signature == *generation)
            });
            *s.surfaces.borrow_mut() = surfaces;
            if let Some(renderer) = &mut s.renderer {
                let layers: Vec<_> = (0..renderer.layers()).collect();
                renderer.refresh_layers(&layers);
            }
        }
        Ok(json!({"ok":true}))
    })
}
#[no_mangle]
pub extern "C" fn pax_desktop_render(c: *mut DesktopEngineContainer) -> DesktopBuffer {
    call(c, |s| s.render())
}
#[no_mangle]
pub unsafe extern "C" fn pax_desktop_interrupt(
    c: *mut DesktopEngineContainer,
    data: *const u8,
    len: usize,
) -> DesktopBuffer {
    call(c, |s| {
        s.live()?;
        let event: NativeInterrupt =
            serde_json::from_slice(unsafe { bytes(data, len, JSON_LIMIT)? })
                .map_err(|e| e.to_string())?;
        let prevented = s
            .interrupts
            .dispatch(&s.engine, &event)
            .ok_or("interrupt requires a dedicated desktop host operation")?;
        Ok(json!({"ok":true,"preventDefault":prevented}))
    })
}
#[no_mangle]
pub unsafe extern "C" fn pax_desktop_load_image(
    c: *mut DesktopEngineContainer,
    path: *const u8,
    path_len: usize,
    rgba: *const u8,
    rgba_len: usize,
    w: u32,
    h: u32,
) -> DesktopBuffer {
    call(c, |s| {
        s.initialize_gpu()?;
        let path = std::str::from_utf8(unsafe { bytes(path, path_len, 16384)? })
            .map_err(|e| e.to_string())?;
        let expected = (w as usize)
            .checked_mul(h as usize)
            .and_then(|n| n.checked_mul(4))
            .ok_or("image size overflow")?;
        if expected != rgba_len || w == 0 || h == 0 {
            return Err("invalid RGBA image dimensions".into());
        }
        s.renderer.as_mut().unwrap().load_image(
            path,
            unsafe { bytes(rgba, rgba_len, 256 * 1024 * 1024)? },
            w as usize,
            h as usize,
        );
        s.engine.runtime_context.set_all_canvases_dirty();
        s.engine.runtime_context.mark_all_canvas_nodes_dirty();
        Ok(json!({"ok":true}))
    })
}
#[no_mangle]
pub unsafe extern "C" fn pax_desktop_release(
    c: *mut DesktopEngineContainer,
    token: u64,
    data: *const u8,
    len: usize,
) -> DesktopBuffer {
    call(c, |s| {
        let completion: Value = serde_json::from_slice(unsafe { bytes(data, len, JSON_LIMIT)? })
            .map_err(|e| e.to_string())?;
        s.gpu
            .as_mut()
            .ok_or("GPU was not initialized")?
            .release(token, completion)?;
        Ok(json!({"ok":true}))
    })
}
#[no_mangle]
pub extern "C" fn pax_desktop_poll(c: *mut DesktopEngineContainer) -> DesktopBuffer {
    call(c, |s| s.poll())
}

fn discard(c: *mut DesktopEngineContainer, f: impl FnOnce(&mut Session) -> Result<Value, String>) {
    let result = call(c, f);
    unsafe { pax_desktop_free_buffer(result) };
}
#[no_mangle]
pub extern "C" fn pax_desktop_activate(c: *mut DesktopEngineContainer) {
    discard(c, |s| {
        s.live()?;
        s.engine.activate_application();
        Ok(json!({"ok":true}))
    })
}
#[no_mangle]
pub extern "C" fn pax_desktop_begin_close(c: *mut DesktopEngineContainer) {
    discard(c, |s| {
        s.begin_close();
        Ok(json!({"ok":true}))
    })
}
#[no_mangle]
pub unsafe extern "C" fn pax_desktop_finish_close(c: *mut DesktopEngineContainer) -> bool {
    let mut drained = false;
    discard(c, |s| {
        if s.closing {
            drained = s.poll()?["drained"].as_bool().unwrap_or(false);
        }
        Ok(json!({"ok":true}))
    });
    if drained {
        // Ownership is consumed even if application/backend destruction panics.
        // Returning false would invite the host to retry with a freed pointer.
        let _ = catch_unwind(AssertUnwindSafe(|| drop(unsafe { Box::from_raw(c) })));
    }
    drained
}
#[no_mangle]
pub extern "C" fn pax_desktop_set_property_waker(
    c: *mut DesktopEngineContainer,
    token: u64,
    wake: extern "C" fn(u64),
) {
    discard(c, |s| {
        s.live()?;
        s.engine
            .runtime_context
            .application
            .set_waker(move || wake(token));
        Ok(json!({"ok":true}))
    })
}
#[no_mangle]
pub extern "C" fn pax_desktop_has_pending_properties(c: *mut DesktopEngineContainer) -> bool {
    let mut pending = false;
    discard(c, |s| {
        s.live()?;
        pending = s.engine.runtime_context.application.has_pending_work();
        Ok(json!({"ok":true}))
    });
    pending
}
#[no_mangle]
pub extern "C" fn pax_desktop_set_shutdown_waker(token: u64, wake: extern "C" fn(u64)) {
    pax_runtime::api::application_shutdown::set_shutdown_waker(move || wake(token));
}
#[no_mangle]
pub extern "C" fn pax_desktop_pump_shutdowns() -> u64 {
    catch_unwind(pax_runtime::api::application_shutdown::pump_shutdowns)
        .map(|n| n as u64)
        .unwrap_or(1)
}
#[no_mangle]
pub unsafe extern "C" fn pax_desktop_request(
    c: *mut DesktopEngineContainer,
    data: *const u8,
    len: usize,
    context: *mut c_void,
    reply: extern "C" fn(*mut c_void, u32, *const u8, usize),
) {
    let once = Rc::new(RefCell::new(Some(reply)));
    let respond = move |result: Result<Vec<u8>, String>| {
        let callback = once.borrow_mut().take();
        let Some(reply) = callback else { return };
        let (status, bytes) = match result {
            Ok(b) => (0, b),
            Err(e) => (1, e.into_bytes()),
        };
        reply(context, status, bytes.as_ptr(), bytes.len());
    };
    let mut accepted = false;
    let result = call(c, |s| {
        s.live()?;
        let data = unsafe { bytes(data, len, pax_runtime::api::HOST_MESSAGE_LIMIT)? }.to_vec();
        s.engine
            .runtime_context
            .application
            .context()
            .request(data, respond.clone());
        accepted = true;
        Ok(json!({"ok":true}))
    });
    if !accepted {
        respond(Err(String::from_utf8_lossy(unsafe {
            std::slice::from_raw_parts(result.data, result.len)
        })
        .into_owned()));
    }
    unsafe { pax_desktop_free_buffer(result) };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    fn container() -> *mut DesktopEngineContainer {
        use pax_runtime::api::{Platform, OS};
        #[cfg(feature = "designtime")]
        let engine = PaxEngine::new_empty_with_designtime(
            (100.0, 50.0),
            Rc::new(RefCell::new(
                pax_designtime::DesigntimeManager::new_offline(pax_manifest::PaxManifest {
                    components: Default::default(),
                    main_component_type_id: Default::default(),
                    type_table: Default::default(),
                    assets_dirs: Vec::new(),
                    engine_import_path: "pax_engine".into(),
                }),
            )),
            Platform::Native,
            OS::Linux,
            Box::new(|| 0),
            Default::default(),
        );
        #[cfg(not(feature = "designtime"))]
        let engine = PaxEngine::new_empty(
            (100.0, 50.0),
            Platform::Native,
            OS::Linux,
            Box::new(|| 0),
            Default::default(),
        );
        Box::into_raw(Box::new(DesktopEngineContainer::new(engine, || {
            Err("test adapter unavailable".into())
        })))
    }

    fn response(buffer: DesktopBuffer) -> Value {
        let value =
            serde_json::from_slice(unsafe { std::slice::from_raw_parts(buffer.data, buffer.len) })
                .unwrap();
        unsafe { pax_desktop_free_buffer(buffer) };
        value
    }

    struct FailedBackend {
        drained: Rc<Cell<bool>>,
        dropped: Rc<Cell<bool>>,
        panic_on_drop: bool,
    }
    impl Drop for FailedBackend {
        fn drop(&mut self) {
            self.dropped.set(true);
            assert!(!self.panic_on_drop, "backend destructor panicked");
        }
    }
    impl DesktopGpuBackend for FailedBackend {
        fn context(&self) -> pax_gpu::render_backend::SharedGpuContext {
            panic!("not needed")
        }
        fn format(&self) -> wgpu::TextureFormat {
            wgpu::TextureFormat::Bgra8Unorm
        }
        fn poll(&mut self) -> Result<(), String> {
            Err("device lost".into())
        }
        fn can_publish(&mut self, _: &[DesktopSurfaceInfo]) -> Result<bool, String> {
            panic!("not needed")
        }
        fn publish(&mut self, _: Vec<DesktopSurfaceFrame>) -> Result<Value, String> {
            panic!("not needed")
        }
        fn release(&mut self, _: u64, _: Value) -> Result<(), String> {
            Ok(())
        }
        fn shutdown_poll(&mut self) -> Result<bool, String> {
            Ok(self.drained.get())
        }
        fn diagnostics(&self) -> Value {
            Value::Null
        }
    }

    #[test]
    fn terminal_gpu_error_preserves_exports_until_drain_is_proven() {
        let c = container();
        let drained = Rc::new(Cell::new(false));
        let dropped = Rc::new(Cell::new(false));
        unsafe { &*c }.session.borrow_mut().gpu = Some(Box::new(FailedBackend {
            drained: drained.clone(),
            dropped: dropped.clone(),
            panic_on_drop: false,
        }));
        assert_eq!(response(pax_desktop_poll(c))["error"], "device lost");
        assert_eq!(response(pax_desktop_tick(c, 100.0, 50.0, 1.0))["ok"], false);
        pax_desktop_begin_close(c);
        assert!(!unsafe { pax_desktop_finish_close(c) });
        assert!(!dropped.get());
        let completion = b"{}";
        assert_eq!(
            response(unsafe { pax_desktop_release(c, 1, completion.as_ptr(), completion.len()) })
                ["ok"],
            true
        );
        drained.set(true);
        assert!(unsafe { pax_desktop_finish_close(c) });
        assert!(dropped.get());
    }

    #[test]
    fn final_destructor_panic_still_reports_consumed_ownership() {
        let c = container();
        let dropped = Rc::new(Cell::new(false));
        unsafe { &*c }.session.borrow_mut().gpu = Some(Box::new(FailedBackend {
            drained: Rc::new(Cell::new(true)),
            dropped: dropped.clone(),
            panic_on_drop: true,
        }));
        pax_desktop_begin_close(c);
        assert!(unsafe { pax_desktop_finish_close(c) });
        assert!(dropped.get());
    }

    #[test]
    fn runtime_panic_latches_failure_and_still_allows_close() {
        let c = container();
        assert_eq!(
            response(call(c, |_| panic!("application hook panicked")))["ok"],
            false
        );
        assert_eq!(response(pax_desktop_tick(c, 100.0, 50.0, 1.0))["ok"], false);
        pax_desktop_begin_close(c);
        assert!(unsafe { pax_desktop_finish_close(c) });
    }
    fn surface() -> DesktopSurfaceInfo {
        DesktopSurfaceInfo {
            layer: 0,
            key: "tile".into(),
            host_signature: "root:1".into(),
            origin_x: 0.0,
            origin_y: 0.0,
            logical_width: 100.0,
            logical_height: 50.0,
            width: 200,
            height: 100,
            dpr: [2.0, 2.0],
            replay_priority: 0,
        }
    }
    #[test]
    fn surface_registry_rejects_aliases_and_nonfinite_geometry() {
        let s = surface();
        assert!(validate_surfaces(&[s.clone()]).is_ok());
        assert!(validate_surfaces(&[s.clone(), s.clone()]).is_err());
        let mut bad = s.clone();
        bad.origin_x = f32::NAN;
        assert!(validate_surfaces(&[bad]).is_err());
        let mut bad = s;
        bad.dpr[0] = 0.0;
        assert!(validate_surfaces(&[bad]).is_err());
    }
    #[test]
    fn surface_layout_preserves_generation_and_pixel_scale() {
        let layout = surface_layout(&[surface()], 0);
        assert!(layout.active);
        assert_eq!(layout.surfaces[0].host_signature, "root:1");
        assert_eq!(layout.surfaces[0].surface.surface_width, 200);
        assert_eq!(layout.surfaces[0].surface.dpr, [2.0, 2.0]);
        assert!(!surface_layout(&[surface()], 1).active);
    }
    #[test]
    fn resize_requires_a_new_backing_generation_but_origin_changes_do_not() {
        let old = surface();
        let mut next = old.clone();
        next.origin_y += 50.0;
        assert!(validate_generations(&[old.clone()], &[next.clone()]).is_ok());
        next.width *= 2;
        assert!(validate_generations(&[old.clone()], &[next.clone()]).is_err());
        next.host_signature = "root:2".into();
        assert!(validate_generations(&[old], &[next]).is_ok());
    }
    #[test]
    fn abi_rejects_null_and_oversized_inputs_without_reading() {
        assert!(unsafe { bytes(std::ptr::null(), 1, 10) }.is_err());
        assert!(unsafe { bytes(std::ptr::null(), 0, 10) }.is_ok());
        assert!(unsafe { bytes(std::ptr::null(), 11, 10) }.is_err());
        let result = pax_desktop_render(std::ptr::null_mut());
        let json: Value =
            serde_json::from_slice(unsafe { std::slice::from_raw_parts(result.data, result.len) })
                .unwrap();
        assert_eq!(json["ok"], false);
        unsafe { pax_desktop_free_buffer(result) };
    }
}

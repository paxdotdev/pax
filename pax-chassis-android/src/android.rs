use pax_chassis_common::desktop::*;
use pax_gpu::render_backend::{GpuContext, SharedGpuContext};
use pax_runtime::PaxEngine;
use raw_window_handle::{
    AndroidDisplayHandle, AndroidNdkWindowHandle, RawDisplayHandle, RawWindowHandle,
};
use serde_json::{json, Value};
use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_void,
    panic::{catch_unwind, AssertUnwindSafe},
    ptr::NonNull,
    rc::Rc,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread::{self, ThreadId},
};

#[link(name = "android")]
extern "C" {
    fn ANativeWindow_acquire(window: *mut c_void);
    fn ANativeWindow_release(window: *mut c_void);
}

struct Window {
    // Rust drops the WGPU surface before relinquishing the native-window lease.
    surface: Option<wgpu::Surface<'static>>,
    pointer: NonNull<c_void>,
    width: u32,
    height: u32,
    attached: bool,
}
impl Drop for Window {
    fn drop(&mut self) {
        self.surface.take();
        unsafe { ANativeWindow_release(self.pointer.as_ptr()) };
    }
}
type Registry = Rc<RefCell<HashMap<u64, Window>>>;

/// Opaque UI-thread Android owner, containing the shared native engine session.
/// The host keeps this alive across configuration recreation and detaches every
/// old surface before reattaching views. It never transfers Rust values over JNI.
pub struct AndroidEngineContainer {
    engine: *mut DesktopEngineContainer,
    windows: Registry,
    owner: ThreadId,
}
impl AndroidEngineContainer {
    /// Prepares a session without creating a device or activating application work.
    pub fn new(mut engine: PaxEngine) -> Self {
        engine.scroller_tiling_policy = crate::tiling_policy();
        let windows: Registry = Default::default();
        let registry = windows.clone();
        let engine = DesktopEngineContainer::with_gpu_factory(engine, move || {
            Backend::new(registry).map(|b| Box::new(b) as Box<dyn DesktopGpuBackend>)
        });
        Self {
            engine: Box::into_raw(Box::new(engine)),
            windows,
            owner: thread::current().id(),
        }
    }

    /// Retains Debug manifest/reload ownership in the same cartridge as the root.
    #[cfg(feature = "designtime")]
    pub fn new_designtime(
        engine: PaxEngine,
        traverser: Box<dyn pax_runtime::cartridge::DefinitionToInstanceTraverser>,
        manager: Rc<RefCell<pax_designtime::DesigntimeManager>>,
    ) -> Self {
        let result = Self::new(engine);
        unsafe { &*result.engine }.retain_designtime(traverser, manager);
        result
    }
}

unsafe fn session<'a>(pointer: *mut AndroidEngineContainer) -> Option<&'a AndroidEngineContainer> {
    pointer
        .as_ref()
        .filter(|s| s.owner == thread::current().id())
}

/// Borrows the common engine token until Android close consumes its owner.
/// # Safety
/// `pointer` must be a live Android session, used only on its owning UI thread.
#[no_mangle]
pub unsafe extern "C" fn pax_android_engine(
    pointer: *mut AndroidEngineContainer,
) -> *mut DesktopEngineContainer {
    session(pointer).map_or(std::ptr::null_mut(), |s| s.engine)
}

/// Projects Markdown into UTF-16 text spans for Android's native text system.
/// The result uses the common buffer allocator and must be freed with
/// `pax_desktop_free_buffer`; it never contains borrowed cartridge memory.
/// # Safety
/// `data` must reference `len` readable bytes for this call.
#[no_mangle]
pub unsafe extern "C" fn pax_android_text(data: *const u8, len: usize) -> DesktopBuffer {
    let result = catch_unwind(AssertUnwindSafe(|| {
        if data.is_null() || len > crate::rich_text::MAX_SOURCE_BYTES {
            return Err("Invalid Android rich-text source (maximum 1 MiB)".to_owned());
        }
        let source = std::str::from_utf8(std::slice::from_raw_parts(data, len))
            .map_err(|error| error.to_string())?;
        crate::rich_text::parse(source)
    }));
    let value = match result {
        Ok(Ok(document)) => json!({"ok": true, "document": document}),
        Ok(Err(error)) => json!({"ok": false, "error": error}),
        Err(_) => json!({"ok": false, "error": "Android rich-text parser panicked"}),
    };
    let bytes = serde_json::to_vec(&value)
        .expect("text JSON serializes")
        .into_boxed_slice();
    DesktopBuffer {
        len: bytes.len(),
        data: Box::into_raw(bytes).cast::<u8>(),
    }
}

/// Acquires a native-window lease for one unique backing generation.
/// # Safety
/// `window` must be a live ANativeWindow. The caller retains its own reference.
/// All arguments and the session must belong to the calling UI thread.
#[no_mangle]
pub unsafe extern "C" fn pax_android_attach_surface(
    pointer: *mut AndroidEngineContainer,
    generation: u64,
    window: *mut c_void,
    width: u32,
    height: u32,
) -> bool {
    catch_unwind(AssertUnwindSafe(|| {
        let Some(s) = session(pointer) else {
            return false;
        };
        let Some(pointer) = NonNull::new(window) else {
            return false;
        };
        if generation == 0 || width == 0 || height == 0 || width > 4096 || height > 4096 {
            return false;
        }
        let mut windows = s.windows.borrow_mut();
        if windows.contains_key(&generation) || windows.len() >= 64 {
            return false;
        }
        ANativeWindow_acquire(pointer.as_ptr());
        windows.insert(
            generation,
            Window {
                surface: None,
                pointer,
                width,
                height,
                attached: true,
            },
        );
        true
    }))
    .unwrap_or(false)
}

/// Stops new presentation to a generation. Keep its SurfaceTexture alive until
/// `pax_android_surface_retired` is true; GPU completion is polled asynchronously.
/// # Safety
/// `pointer` must be a live session on its UI thread.
#[no_mangle]
pub unsafe extern "C" fn pax_android_detach_surface(
    pointer: *mut AndroidEngineContainer,
    generation: u64,
) {
    if let Some(s) = session(pointer) {
        let mut windows = s.windows.borrow_mut();
        if let Some(window) = windows.get_mut(&generation) {
            window.attached = false;
            // A generation with no swapchain cannot have submitted GPU work.
            // In particular, this retires views destroyed before GPU startup.
            if window.surface.is_none() {
                windows.remove(&generation);
            }
        }
    }
}

/// Whether the host may release a detached generation's SurfaceTexture.
/// # Safety
/// `pointer` must be a live session on its UI thread.
#[no_mangle]
pub unsafe extern "C" fn pax_android_surface_retired(
    pointer: *mut AndroidEngineContainer,
    generation: u64,
) -> bool {
    session(pointer).is_some_and(|s| !s.windows.borrow().contains_key(&generation))
}

/// Consumes the Android session only after the common engine/GPU close has drained.
/// # Safety
/// Begin close first. Call only on the owning UI thread and stop using both tokens
/// when this returns true; a false result retains them for a later poll.
#[no_mangle]
pub unsafe extern "C" fn pax_android_finish_close(pointer: *mut AndroidEngineContainer) -> bool {
    let Some(s) = session(pointer) else {
        return false;
    };
    if pax_desktop_finish_close(s.engine) {
        drop(Box::from_raw(pointer));
        true
    } else {
        false
    }
}

struct Backend {
    instance: wgpu::Instance,
    adapter: wgpu::Adapter,
    device: wgpu::Device,
    queue: wgpu::Queue,
    context: SharedGpuContext,
    windows: Registry,
    complete: Arc<AtomicBool>,
    acquired: HashMap<u64, wgpu::SurfaceTexture>,
    frames: u64,
    closing: bool,
    close_complete: Option<Arc<AtomicBool>>,
}
impl Backend {
    fn new(windows: Registry) -> Result<Self, String> {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            // Android emulator drivers can reject SPIR-V's newer WGSL source
            // metadata. Omit shader debug symbols while retaining validation.
            flags: wgpu::InstanceFlags::default().difference(wgpu::InstanceFlags::DEBUG),
            ..Default::default()
        });
        let mut registry = windows.borrow_mut();
        let mut first = registry.values_mut().find(|w| w.attached);
        if let Some(window) = first.as_mut() {
            window.surface = Some(Self::surface(&instance, window.pointer)?);
        }
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::LowPower,
            compatible_surface: first.as_ref().and_then(|w| w.surface.as_ref()),
            force_fallback_adapter: false,
        }))
        .map_err(|e| e.to_string())?;
        let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
            label: Some("Pax Android"),
            required_limits: adapter.limits(),
            ..Default::default()
        }))
        .map_err(|e| e.to_string())?;
        let context = GpuContext::from_wgpu(
            instance.clone(),
            adapter.clone(),
            device.clone(),
            queue.clone(),
        )
        .map_err(|e| e.to_string())?;
        // Defer swapchain configuration until the first complete surface registry.
        if let Some(window) = first {
            window.surface.take();
        }
        drop(registry);
        Ok(Self {
            instance,
            adapter,
            device,
            queue,
            context,
            windows,
            complete: Arc::new(AtomicBool::new(true)),
            acquired: HashMap::new(),
            frames: 0,
            closing: false,
            close_complete: None,
        })
    }

    fn surface(
        instance: &wgpu::Instance,
        pointer: NonNull<c_void>,
    ) -> Result<wgpu::Surface<'static>, String> {
        // The Registry holds ANativeWindow until this surface and its GPU work retire.
        unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::RawHandle {
                raw_display_handle: RawDisplayHandle::Android(AndroidDisplayHandle::new()),
                raw_window_handle: RawWindowHandle::AndroidNdk(AndroidNdkWindowHandle::new(
                    pointer,
                )),
            })
        }
        .map_err(|e| e.to_string())
    }

    fn configure(&self, window: &mut Window) -> Result<(), String> {
        let surface = Self::surface(&self.instance, window.pointer)?;
        let caps = surface.get_capabilities(&self.adapter);
        if !caps.formats.contains(&wgpu::TextureFormat::Rgba8Unorm)
            || !caps.usages.contains(wgpu::TextureUsages::COPY_DST)
        {
            return Err("Android surface lacks RGBA8 copy presentation".into());
        }
        let alpha_mode = [
            wgpu::CompositeAlphaMode::PreMultiplied,
            wgpu::CompositeAlphaMode::Inherit,
        ]
        .into_iter()
        .find(|m| caps.alpha_modes.contains(m))
        .ok_or("Android surface lacks premultiplied alpha")?;
        surface.configure(
            &self.device,
            &wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: wgpu::TextureFormat::Rgba8Unorm,
                width: window.width,
                height: window.height,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode,
                view_formats: vec![],
            },
        );
        window.surface = Some(surface);
        Ok(())
    }
}
impl DesktopGpuBackend for Backend {
    fn context(&self) -> SharedGpuContext {
        self.context.clone()
    }
    fn format(&self) -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8Unorm
    }
    fn poll(&mut self) -> Result<(), String> {
        self.acquired.clear();
        self.device
            .poll(wgpu::PollType::Poll)
            .map_err(|e| e.to_string())?;
        if self.complete.load(Ordering::Acquire) {
            self.windows.borrow_mut().retain(|_, w| w.attached);
        }
        Ok(())
    }
    fn can_publish(&mut self, surfaces: &[DesktopSurfaceInfo]) -> Result<bool, String> {
        if self.closing || !self.complete.load(Ordering::Acquire) {
            return Ok(false);
        }
        self.acquired.clear();
        let mut windows = self.windows.borrow_mut();
        for info in surfaces {
            let generation = info
                .host_signature
                .parse::<u64>()
                .map_err(|_| "invalid Android surface generation")?;
            let Some(window) = windows.get_mut(&generation).filter(|w| w.attached) else {
                self.acquired.clear();
                return Ok(false);
            };
            if window.width != info.width || window.height != info.height {
                return Err("Android backing size differs from render registry".into());
            }
            if window.surface.is_none() {
                self.configure(window)?;
            }
        }
        Ok(true)
    }
    fn prepare_publish(&mut self, surfaces: &[DesktopSurfaceFrame]) -> Result<bool, String> {
        self.acquired.clear();
        let mut windows = self.windows.borrow_mut();
        for frame in surfaces {
            let generation = frame
                .info
                .host_signature
                .parse::<u64>()
                .map_err(|_| "invalid Android surface generation")?;
            let window = windows
                .get_mut(&generation)
                .ok_or("missing Android surface")?;
            match window.surface.as_ref().unwrap().get_current_texture() {
                Ok(frame) => {
                    self.acquired.insert(generation, frame);
                }
                Err(wgpu::SurfaceError::Timeout) => {
                    self.acquired.clear();
                    return Ok(false);
                }
                Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                    window.surface.take();
                    self.acquired.clear();
                    return Ok(false);
                }
                Err(error) => return Err(error.to_string()),
            }
        }
        Ok(true)
    }
    fn publish(&mut self, surfaces: Vec<DesktopSurfaceFrame>) -> Result<Value, String> {
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let mut presented = Vec::new();
        let mut generations = Vec::new();
        for frame in surfaces {
            let generation = frame
                .info
                .host_signature
                .parse::<u64>()
                .map_err(|_| "invalid generation")?;
            let output = self
                .acquired
                .remove(&generation)
                .ok_or("Android surface was not acquired")?;
            encoder.copy_texture_to_texture(
                frame.texture.as_image_copy(),
                output.texture.as_image_copy(),
                wgpu::Extent3d {
                    width: frame.info.width,
                    height: frame.info.height,
                    depth_or_array_layers: 1,
                },
            );
            presented.push(output);
            generations.push(generation);
        }
        self.acquired.clear();
        self.complete.store(false, Ordering::Release);
        self.queue.submit([encoder.finish()]);
        let complete = self.complete.clone();
        self.queue
            .on_submitted_work_done(move || complete.store(true, Ordering::Release));
        for frame in presented {
            frame.present();
        }
        self.frames += 1;
        // Submission does not mean TextureView has received the buffer. The
        // UI host waits for these consumers before committing native geometry.
        Ok(json!({"presented":true,"frames":self.frames,"generations":generations}))
    }
    fn release(&mut self, _: u64, _: Value) -> Result<(), String> {
        Err("Android presents to leased windows; there are no exported frame tokens".into())
    }
    fn shutdown_poll(&mut self) -> Result<bool, String> {
        if !self.closing {
            self.closing = true;
            // A render may have submitted work before swapchain acquisition
            // returned backpressure. Close must fence that work too, not only
            // the last successful presentation copy.
            let complete = Arc::new(AtomicBool::new(false));
            self.close_complete = Some(complete.clone());
            self.queue
                .on_submitted_work_done(move || complete.store(true, Ordering::Release));
        }
        self.acquired.clear();
        self.poll()?;
        let complete = self
            .close_complete
            .as_ref()
            .unwrap()
            .load(Ordering::Acquire);
        if complete {
            self.windows.borrow_mut().clear();
        }
        Ok(complete)
    }
    fn diagnostics(&self) -> Value {
        let windows = self.windows.borrow();
        let pixels: u64 = windows
            .values()
            .map(|w| u64::from(w.width) * u64::from(w.height))
            .sum();
        json!({"backend":"android-vulkan", "adapter":self.adapter.get_info().name,
            "frames":self.frames, "windows":windows.len(), "backingPixels":pixels,
            "nominalColorBytes":pixels * 4, "pending":!self.complete.load(Ordering::Acquire)})
    }
}

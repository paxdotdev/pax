//! Bounded Pax DX12 producer for the native composition probe.
//! All calls occur on the WinUI thread. C++ must finish the prior D3D11On12
//! copy before rendering again and must release its wrappers before dropping us.

use anyhow::{bail, Context, Result};
use pax_gpu::render_backend::{GpuContext, RenderBackend, RenderConfig, SharedGpuContext};
use pax_gpu::{point, Color, Fill, Path, WgpuRenderer};
use std::{
    cell::RefCell,
    ffi::{c_char, c_void, CString},
    fs::OpenOptions,
    io::Write,
    ptr,
    sync::{Arc, Mutex},
    time::Duration,
};
use windows_core::Interface;

type Dx12 = wgpu::hal::api::Dx12;

thread_local! {
    static LAST_ERROR: RefCell<CString> = RefCell::new(CString::default());
}

#[repr(C)]
pub struct ExportInfo {
    device: *mut c_void,
    queue: *mut c_void,
    scene: *mut c_void,
    mask: *mut c_void,
}

struct Image {
    renderer: WgpuRenderer<'static>,
    export: wgpu::Texture,
    width: u32,
    height: u32,
}

struct ProbeContext {
    images: [Image; 2],
    device: wgpu::Device,
    queue: wgpu::Queue,
    clear_width: u32,
    frames: u64,
    errors: Arc<Mutex<Vec<String>>>,
}

fn report(message: &str) {
    if let Ok(mut stream) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("composition-probe.log")
    {
        let _ = writeln!(stream, "pax_wgpu {message}");
    }
}

fn rect(x: f32, y: f32, width: f32, height: f32) -> Path {
    let mut path = Path::builder();
    path.begin(point(x, y));
    path.line_to(point(x + width, y));
    path.line_to(point(x + width, y + height));
    path.line_to(point(x, y + height));
    path.end(true);
    path.build()
}

fn draw(renderer: &mut WgpuRenderer<'static>, id: u32, path: Path, color: Color) {
    renderer.begin_node(id, id as i32, 0);
    renderer.fill_path(path, Fill::Solid(color));
    renderer.end_node(id);
}

fn image(
    context: SharedGpuContext,
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> Result<Image> {
    let backend = RenderBackend::to_texture(
        context,
        wgpu::TextureFormat::Bgra8Unorm,
        RenderConfig::new(false, width, height, [1.0, 1.0]),
    )?;
    let export = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Windows native composition export"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Bgra8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    Ok(Image {
        renderer: WgpuRenderer::new(backend),
        export,
        width,
        height,
    })
}

impl ProbeContext {
    fn new(
        allow_software: bool,
        scene_width: u32,
        scene_height: u32,
        editor_width: u32,
        editor_height: u32,
        clear_width: u32,
    ) -> Result<Self> {
        if [scene_width, scene_height, editor_width, editor_height].contains(&0)
            || clear_width >= editor_width
        {
            bail!("invalid export dimensions");
        }
        report("creating_instance backend=DX12");
        // HAL telemetry reports rejected adapters, which public enumeration omits.
        let hal = unsafe {
            <wgpu::hal::dx12::Instance as wgpu::hal::Instance>::init(
                &wgpu::hal::InstanceDescriptor {
                    name: "Pax Windows native composition probe",
                    flags: wgpu::InstanceFlags::default(),
                    memory_budget_thresholds: Default::default(),
                    backend_options: Default::default(),
                    telemetry: Some(wgpu::hal::Telemetry {
                        d3d12_expose_adapter: |desc, version, result| {
                            use wgpu::hal::D3D12ExposeAdapterResult as Outcome;
                            let name = String::from_utf16_lossy(&desc.Description);
                            let reason = match result {
                                Outcome::Success(_, _) => "exposed".to_string(),
                                Outcome::ResourceBindingTier2Requirement => {
                                    "rejected: ResourceBindingTier < 2".to_string()
                                }
                                Outcome::ShaderModel6Requirement => {
                                    "rejected: shader model 6 required by compiler".to_string()
                                }
                                Outcome::CreateDeviceError(error) => match error {
                                    wgpu::hal::dx12::CreateDeviceError::D3D12CreateDevice(hr) => {
                                        format!("rejected: D3D12CreateDevice {hr:?}")
                                    }
                                    wgpu::hal::dx12::CreateDeviceError::GetProcAddress => {
                                        "rejected: missing D3D12CreateDevice".to_string()
                                    }
                                    wgpu::hal::dx12::CreateDeviceError::RetDeviceIsNull => {
                                        "rejected: null device".to_string()
                                    }
                                },
                            };
                            report(&format!(
                                "DX12 telemetry adapter={} driver={version:?} {reason}",
                                name.trim_end_matches('\0')
                            ));
                        },
                    }),
                },
            )
        }?;
        let instance = unsafe { wgpu::Instance::from_hal::<Dx12>(hal) };
        report("enumerating_adapters");
        let mut adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::DX12));
        for adapter in &adapters {
            report(&format!("enumerated_adapter={:?}", adapter.get_info()));
        }
        let hardware = adapters
            .iter()
            .position(|adapter| adapter.get_info().device_type != wgpu::DeviceType::Cpu);
        let index = match hardware {
            Some(index) => index,
            None if allow_software && !adapters.is_empty() => 0,
            None => bail!("No WGPU-compatible DX12 hardware adapter; see telemetry exclusions"),
        };
        let adapter = adapters.swap_remove(index);
        let info = adapter.get_info();
        report(&format!("adapter={info:?} allow_software={allow_software}"));
        if info.device_type == wgpu::DeviceType::Cpu && !allow_software {
            bail!("WGPU selected software adapter without --allow-warp");
        }
        report("requesting_device");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
                label: Some("Pax native WinUI composition probe"),
                required_limits: adapter.limits(),
                ..Default::default()
            }))?;
        let errors = Arc::new(Mutex::new(Vec::new()));
        let handler_errors = Arc::clone(&errors);
        device.on_uncaptured_error(Arc::new(move |error| {
            let message = error.to_string();
            report(&format!("GPU_ERROR: {message}"));
            handler_errors
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .push(message);
        }));
        report("creating_pax_context");
        let pax = GpuContext::from_wgpu(instance, adapter, device.clone(), queue.clone())?;
        report("creating_pax_renderers_and_export_textures");
        let images = [
            image(pax.clone(), &device, scene_width, scene_height)?,
            image(pax, &device, editor_width, editor_height)?,
        ];
        let mut result = Self {
            images,
            device,
            queue,
            clear_width,
            frames: 0,
            errors,
        };
        // Establish COPY_SOURCE before the native wrappers are created.
        report("rendering_initial_pax_frame");
        result.render(0.0)?;
        result.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        })?;
        result.check_errors()?;
        report("initial_pax_frame_complete");
        Ok(result)
    }

    fn check_errors(&self) -> Result<()> {
        let errors = self.errors.lock().unwrap_or_else(|e| e.into_inner());
        if !errors.is_empty() {
            bail!("{}", errors.join("; "));
        }
        Ok(())
    }

    fn render(&mut self, phase: f32) -> Result<()> {
        self.check_errors()?;
        let scene = &mut self.images[0];
        let center = scene.width as f32 / 2.0 + phase.sin() * 160.0;
        // Real Pax vector tessellation, retained nodes, transparency, and clip.
        scene.renderer.begin_node(1, 1, 0);
        scene.renderer.save();
        scene.renderer.clip(rect(
            6.0,
            6.0,
            scene.width as f32 - 12.0,
            scene.height as f32 - 12.0,
        ));
        scene.renderer.fill_path(
            rect(center - 64.0, 16.0, 128.0, 128.0),
            Fill::Solid(Color::rgba(0.15, 0.85, 0.7, 0.72)),
        );
        scene.renderer.restore();
        scene.renderer.end_node(1);
        draw(
            &mut scene.renderer,
            2,
            rect(2.0, 2.0, scene.width as f32 - 4.0, 2.0),
            Color::rgba(1.0, 0.55, 0.2, 1.0),
        );
        scene.renderer.flush();

        let mask = &mut self.images[1];
        draw(
            &mut mask.renderer,
            1,
            rect(
                self.clear_width as f32,
                0.0,
                (mask.width - self.clear_width) as f32,
                mask.height as f32,
            ),
            Color::rgba(1.0, 1.0, 1.0, 0.35),
        );
        draw(
            &mut mask.renderer,
            2,
            rect(
                230.0 + phase.sin() * 100.0 - 64.0,
                0.0,
                128.0,
                mask.height as f32,
            ),
            Color::rgba(1.0, 1.0, 1.0, 1.0),
        );
        mask.renderer.flush();

        let mut encoder = self.device.create_command_encoder(&Default::default());
        for image in &self.images {
            let source = image
                .renderer
                .output_texture()
                .context("missing Pax output texture")?;
            encoder.copy_texture_to_texture(
                source.as_image_copy(),
                image.export.as_image_copy(),
                wgpu::Extent3d {
                    width: image.width,
                    height: image.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        // WGPU's tracker knows the boundary state. D3D11On12 acquires/releases
        // the exports in COPY_SOURCE and never changes that externally visible state.
        encoder.transition_resources(
            std::iter::empty(),
            self.images.iter().map(|image| wgpu::TextureTransition {
                texture: &image.export,
                selector: None,
                state: wgpu::TextureUses::COPY_SRC,
            }),
        );
        self.queue.submit([encoder.finish()]);
        self.device.poll(wgpu::PollType::Poll)?;
        self.check_errors()?;
        self.frames += 1;
        if self.frames == 1 || self.frames % 60 == 0 {
            report(&format!(
                "frame={} source=pax-gpu exports=2 state=COPY_SOURCE no_readback=true",
                self.frames
            ));
        }
        Ok(())
    }

    fn info(&self) -> Result<ExportInfo> {
        // HAL guards carry ranked locks; never hold texture and device guards
        // simultaneously. The owners keep the borrowed COM pointers alive.
        let (device, queue) = {
            let hal = unsafe { self.device.as_hal::<Dx12>() }.context("device is not DX12")?;
            (hal.raw_device().as_raw(), hal.raw_queue().as_raw())
        };
        let scene = {
            let hal =
                unsafe { self.images[0].export.as_hal::<Dx12>() }.context("scene is not DX12")?;
            unsafe { hal.raw_resource() }.as_raw()
        };
        let mask = {
            let hal =
                unsafe { self.images[1].export.as_hal::<Dx12>() }.context("mask is not DX12")?;
            unsafe { hal.raw_resource() }.as_raw()
        };
        Ok(ExportInfo {
            device,
            queue,
            scene,
            mask,
        })
    }
}

fn ffi<T>(operation: impl FnOnce() -> Result<T>) -> Option<T> {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation));
    let message = match result {
        Ok(Ok(value)) => return Some(value),
        Ok(Err(error)) => format!("{error:#}"),
        Err(payload) => {
            let detail = payload
                .downcast_ref::<String>()
                .map(String::as_str)
                .or_else(|| payload.downcast_ref::<&str>().copied())
                .unwrap_or("non-string panic");
            format!("panic contained at Windows probe FFI: {detail}")
        }
    };
    report(&format!("ERROR: {message}"));
    LAST_ERROR.with(|slot| *slot.borrow_mut() = CString::new(message.replace('\0', " ")).unwrap());
    None
}

#[no_mangle]
pub extern "C" fn pax_probe_last_error() -> *const c_char {
    LAST_ERROR.with(|slot| slot.borrow().as_ptr())
}

#[no_mangle]
/// # Safety
/// `info` points to writable ExportInfo storage. Returned COM pointers are
/// borrowed from the context and remain valid until context destruction.
pub unsafe extern "C" fn pax_probe_gpu_new(
    allow_software: bool,
    scene_width: u32,
    scene_height: u32,
    editor_width: u32,
    editor_height: u32,
    clear_width: u32,
    info: *mut ExportInfo,
) -> *mut c_void {
    ffi(|| {
        if info.is_null() {
            bail!("null output info");
        }
        let context = Box::new(ProbeContext::new(
            allow_software,
            scene_width,
            scene_height,
            editor_width,
            editor_height,
            clear_width,
        )?);
        ptr::write(info, context.info()?);
        Ok(Box::into_raw(context).cast())
    })
    .unwrap_or(ptr::null_mut())
}

#[no_mangle]
/// # Safety
/// `context` is a live result of new on this thread. The native copy fence has
/// completed, and all wrappers have been released back to COPY_SOURCE.
pub unsafe extern "C" fn pax_probe_gpu_render(context: *mut c_void, phase: f32) -> bool {
    ffi(|| {
        if context.is_null() {
            bail!("null render context");
        }
        (&mut *context.cast::<ProbeContext>()).render(phase)
    })
    .is_some()
}

#[no_mangle]
/// # Safety
/// `context` is null or an unreleased result of new on this thread. All native
/// GPU copies have completed and native wrappers have been destroyed.
pub unsafe extern "C" fn pax_probe_gpu_drop(context: *mut c_void) {
    if !context.is_null() {
        drop(Box::from_raw(context.cast::<ProbeContext>()));
        report("context dropped after native copy fence");
    }
}

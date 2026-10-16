//! Isolated zero-readback export probe, not the production renderer.
//! Images are immutable after publication. The opt-in synchronization diagnostics
//! exercise explicit producer fences; they do not implement a streaming pool.
//! Test scripts bound the process, not every underlying GPU wait.

use anyhow::{anyhow, bail, Context, Result};
use ash::vk::{self, Handle};
use pax_gpu::render_backend::{GpuContext, RenderBackend, RenderConfig, SharedGpuContext};
use pax_gpu::{point, Color, Fill, Path, WgpuRenderer};
use std::{
    ffi::c_void,
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
    ptr,
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

static LIVE_ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

type Vulkan = wgpu::hal::api::Vulkan;

pub struct ContextState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    handle_type: vk::ExternalMemoryHandleTypeFlags,
    async_dmabuf: bool,
    delayed_test_available: AtomicBool,
}

struct ProbeContext {
    gpu: Arc<ContextState>,
    // Pax's retained scene lives on GTK's main thread. Only the Send + Sync
    // allocation owner crosses into WGPU's resource retirement callback.
    pax: SharedGpuContext,
}

struct Allocation {
    context: Arc<ContextState>,
    raw: ash::Device,
    image: vk::Image,
    memory: vk::DeviceMemory,
}

impl Drop for Allocation {
    fn drop(&mut self) {
        // The WGPU texture's drop callback retains this allocation until WGPU
        // retires the resource; the GDK owner retains a second reference.
        unsafe {
            if self.image != vk::Image::null() {
                self.raw.destroy_image(self.image, None);
            }
            if self.memory != vk::DeviceMemory::null() {
                self.raw.free_memory(self.memory, None);
            }
        }
        LIVE_ALLOCATIONS.fetch_sub(1, Ordering::Relaxed);
    }
}

pub struct Tile {
    allocation: Arc<Allocation>,
    texture: Option<wgpu::Texture>,
    pending: Option<PendingTransfer>,
}

struct PendingTransfer {
    allocation: Arc<Allocation>,
    pool: vk::CommandPool,
    fence: vk::Fence,
    semaphore: vk::Semaphore,
    submitted: bool,
    test_event: vk::Event,
    test_worker: Option<std::thread::JoinHandle<()>>,
}

impl Drop for PendingTransfer {
    fn drop(&mut self) {
        if let Some(worker) = self.test_worker.take() {
            let _ = worker.join();
        }
        let raw = &self.allocation.raw;
        unsafe {
            if self.submitted
                && raw
                    .wait_for_fences(&[self.fence], true, 10_000_000_000)
                    .is_err()
            {
                // A hung producer must not turn a diagnostic timeout into use-after-free.
                // Retain the device and allocation; the allocation-baseline test fails.
                eprintln!(
                    "PAX_WGPU_EXPORT_ERROR: producer retirement failed; retaining GPU resources"
                );
                std::mem::forget(Arc::clone(&self.allocation));
                return;
            }
            if self.semaphore != vk::Semaphore::null() {
                raw.destroy_semaphore(self.semaphore, None);
            }
            if self.test_event != vk::Event::null() {
                raw.destroy_event(self.test_event, None);
            }
            if self.fence != vk::Fence::null() {
                raw.destroy_fence(self.fence, None);
            }
            raw.destroy_command_pool(self.pool, None);
        }
    }
}

impl Drop for Tile {
    fn drop(&mut self) {
        self.pending.take();
        self.texture.take();
        let _ = self.allocation.context.device.poll(wgpu::PollType::Wait {
            submission_index: None,
            timeout: Some(Duration::from_secs(10)),
        });
    }
}

#[repr(C)]
pub struct ExportInfo {
    fd: i32,
    sync_fd: i32,
    width: u32,
    height: u32,
    stride: u32,
    offset: u32,
    allocation_size: u64,
    fourcc: u32,
    modifier: u64,
    device_uuid: [u8; vk::UUID_SIZE],
    driver_uuid: [u8; vk::UUID_SIZE],
}

fn parse_device_uuid(value: &str) -> Result<[u8; vk::UUID_SIZE]> {
    if value.len() != vk::UUID_SIZE * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("PAX_PROBE_GPU_DEVICE_UUID must contain exactly 32 hexadecimal digits");
    }
    let mut uuid = [0; vk::UUID_SIZE];
    for (index, byte) in uuid.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16)?;
    }
    Ok(uuid)
}

fn physical_gpu(device_type: wgpu::DeviceType) -> bool {
    matches!(
        device_type,
        wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
    )
}

fn context_new(dma_buf: bool) -> Result<ProbeContext> {
    // Capability inventory and composition must exercise the same adapter on
    // hybrid systems. Never silently fall back after an explicit UUID request.
    let requested_uuid = match std::env::var("PAX_PROBE_GPU_DEVICE_UUID") {
        Ok(value) => Some(parse_device_uuid(&value)?),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(error).context("Invalid PAX_PROBE_GPU_DEVICE_UUID"),
    };
    let allow_software = std::env::var("PAX_PROBE_ALLOW_SOFTWARE").as_deref() == Ok("1");
    let async_dmabuf = dma_buf && std::env::var("PAX_PROBE_DMABUF_SYNC").as_deref() == Ok("1");
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::VULKAN,
        ..Default::default()
    });
    let mut adapters = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::VULKAN));
    adapters.sort_by_key(|adapter| match adapter.get_info().device_type {
        wgpu::DeviceType::DiscreteGpu => 0,
        wgpu::DeviceType::IntegratedGpu => 1,
        wgpu::DeviceType::VirtualGpu => 2,
        wgpu::DeviceType::Other => 3,
        wgpu::DeviceType::Cpu => 4,
    });
    let adapter = adapters.into_iter().find(|adapter| {
        let info = adapter.get_info();
        let Some(hal) = (unsafe { adapter.as_hal::<Vulkan>() }) else { return false; };
        let mut ids = vk::PhysicalDeviceIDProperties::default();
        let mut properties = vk::PhysicalDeviceProperties2::default().push_next(&mut ids);
        unsafe {
            hal.shared_instance().raw_instance().get_physical_device_properties2(
                hal.raw_physical_device(), &mut properties,
            );
        }
        let uuid = ids.device_uuid.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        let identity_matches = requested_uuid.is_none_or(|expected| expected == ids.device_uuid);
        let policy_matches = physical_gpu(info.device_type) || allow_software;
        eprintln!("wgpu_candidate={info:?} device_uuid={uuid} identity_matches={identity_matches} policy_matches={policy_matches}");
        identity_matches && policy_matches
    }).context("No eligible physical Vulkan adapter. PAX_PROBE_ALLOW_SOFTWARE=1 permits diagnostic CPU/virtual/unknown adapters only; an explicit PAX_PROBE_GPU_DEVICE_UUID is never substituted.")?;
    eprintln!("wgpu_adapter={:?}", adapter.get_info());
    let hal = unsafe { adapter.as_hal::<Vulkan>() }.context("adapter is not Vulkan")?;
    let caps = hal.physical_device_capabilities();
    let mut extensions = vec![ash::khr::external_memory_fd::NAME];
    if dma_buf {
        extensions.push(ash::ext::external_memory_dma_buf::NAME);
    }
    if async_dmabuf {
        extensions.push(ash::khr::external_semaphore_fd::NAME);
        extensions.push(ash::ext::queue_family_foreign::NAME);
        let info = vk::PhysicalDeviceExternalSemaphoreInfo::default()
            .handle_type(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD);
        let mut properties = vk::ExternalSemaphoreProperties::default();
        unsafe {
            hal.shared_instance()
                .raw_instance()
                .get_physical_device_external_semaphore_properties(
                    hal.raw_physical_device(),
                    &info,
                    &mut properties,
                );
        }
        if !properties
            .external_semaphore_features
            .contains(vk::ExternalSemaphoreFeatureFlags::EXPORTABLE)
        {
            bail!("SYNC_FD semaphore export is unavailable");
        }
    }
    for extension in &extensions {
        if !caps.supports_extension(extension) {
            bail!("Vulkan adapter does not support required extension {extension:?}");
        }
    }
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("PAX Linux export probe"),
        required_limits: adapter.limits(),
        ..Default::default()
    };
    let open = unsafe {
        hal.open_with_callback(
            wgpu::Features::empty(),
            &wgpu::MemoryHints::Performance,
            Some(Box::new(move |args| {
                for extension in extensions {
                    if !args.extensions.contains(&extension) {
                        args.extensions.push(extension);
                    }
                }
            })),
        )?
    };
    drop(hal);
    let (device, queue) = unsafe { adapter.create_device_from_hal::<Vulkan>(open, &descriptor) }?;
    let pax = GpuContext::from_wgpu(instance, adapter, device.clone(), queue.clone())?;
    Ok(ProbeContext {
        pax,
        gpu: Arc::new(ContextState {
            device,
            queue,
            async_dmabuf,
            delayed_test_available: AtomicBool::new(
                async_dmabuf && std::env::var("PAX_PROBE_SYNC_DELAY_TEST").as_deref() == Ok("1"),
            ),
            handle_type: if dma_buf {
                vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT
            } else {
                vk::ExternalMemoryHandleTypeFlags::OPAQUE_FD
            },
        }),
    })
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

fn render_pax_pattern(
    probe: &ProbeContext,
    width: u32,
    height: u32,
    red: f64,
    green: f64,
    blue: f64,
) -> Result<wgpu::Texture> {
    let backend = RenderBackend::to_texture(
        probe.pax.clone(),
        wgpu::TextureFormat::Rgba8Unorm,
        RenderConfig::new(false, width, height, [1.0, 1.0]),
    )?;
    let mut renderer = WgpuRenderer::new(backend);
    renderer.begin_node(1, 0, 0);
    renderer.fill_path(
        rect(0.0, 0.0, width as f32, height as f32),
        Fill::Solid(Color::rgba(red as f32, green as f32, blue as f32, 1.0)),
    );
    renderer.end_node(1);
    renderer.begin_node(2, 1, 0);
    renderer.save();
    renderer.clip(rect(
        (width / 4) as f32,
        (height / 4) as f32,
        (width * 3 / 4 - width / 4) as f32,
        (height * 3 / 4 - height / 4) as f32,
    ));
    renderer.fill_path(
        rect(0.0, 0.0, width as f32, height as f32),
        Fill::Solid(Color::rgba(1.0, 1.0, 1.0, 1.0)),
    );
    renderer.restore();
    renderer.end_node(2);
    renderer.flush();
    renderer
        .output_texture()
        .context("Pax offscreen renderer has no output")
}

fn export_tile(
    probe: &ProbeContext,
    width: u32,
    height: u32,
    red: f64,
    green: f64,
    blue: f64,
) -> Result<(Tile, ExportInfo)> {
    if width == 0 || height == 0 {
        bail!("zero-sized export");
    }
    let context = &probe.gpu;
    let hal = unsafe { context.device.as_hal::<Vulkan>() }.context("device is not Vulkan")?;
    let raw = hal.raw_device();
    let instance = hal.shared_instance().raw_instance();
    let physical = hal.raw_physical_device();
    let mut ids = vk::PhysicalDeviceIDProperties::default();
    let mut device_properties = vk::PhysicalDeviceProperties2::default().push_next(&mut ids);
    unsafe { instance.get_physical_device_properties2(physical, &mut device_properties) };
    let linear = context.handle_type == vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT;
    let tiling = if linear {
        vk::ImageTiling::LINEAR
    } else {
        vk::ImageTiling::OPTIMAL
    };
    let usage = vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED;
    let mut external_query =
        vk::PhysicalDeviceExternalImageFormatInfo::default().handle_type(context.handle_type);
    let query = vk::PhysicalDeviceImageFormatInfo2::default()
        .format(vk::Format::R8G8B8A8_UNORM)
        .ty(vk::ImageType::TYPE_2D)
        .tiling(tiling)
        .usage(usage)
        .push_next(&mut external_query);
    let mut external_properties = vk::ExternalImageFormatProperties::default();
    let mut properties = vk::ImageFormatProperties2::default().push_next(&mut external_properties);
    unsafe {
        instance.get_physical_device_image_format_properties2(physical, &query, &mut properties)
    }
    .context("RGBA8 image cannot use requested external memory and tiling")?;
    if !external_properties
        .external_memory_properties
        .external_memory_features
        .contains(vk::ExternalMemoryFeatureFlags::EXPORTABLE)
    {
        bail!("RGBA8 image is not exportable with requested memory handle");
    }
    let mut external_create =
        vk::ExternalMemoryImageCreateInfo::default().handle_types(context.handle_type);
    let create = vk::ImageCreateInfo::default()
        .image_type(vk::ImageType::TYPE_2D)
        .format(vk::Format::R8G8B8A8_UNORM)
        .extent(vk::Extent3D {
            width,
            height,
            depth: 1,
        })
        .mip_levels(1)
        .array_layers(1)
        .samples(vk::SampleCountFlags::TYPE_1)
        .tiling(tiling)
        .usage(usage)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .push_next(&mut external_create);
    let image = unsafe { raw.create_image(&create, None) }?;
    let mut allocation = Allocation {
        context: Arc::clone(context),
        raw: raw.clone(),
        image,
        memory: vk::DeviceMemory::null(),
    };
    LIVE_ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    let requirements = unsafe { raw.get_image_memory_requirements(image) };
    let memory_properties = unsafe { instance.get_physical_device_memory_properties(physical) };
    let memory_type = (0..memory_properties.memory_type_count)
        .find(|i| requirements.memory_type_bits & (1 << i) != 0)
        .context("no compatible image memory type")?;
    let mut export = vk::ExportMemoryAllocateInfo::default().handle_types(context.handle_type);
    let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
    let allocate = vk::MemoryAllocateInfo::default()
        .allocation_size(requirements.size)
        .memory_type_index(memory_type)
        .push_next(&mut export)
        .push_next(&mut dedicated);
    allocation.memory = unsafe { raw.allocate_memory(&allocate, None) }?;
    unsafe { raw.bind_image_memory(image, allocation.memory, 0) }?;
    let layout = if linear {
        unsafe {
            raw.get_image_subresource_layout(
                image,
                vk::ImageSubresource {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    mip_level: 0,
                    array_layer: 0,
                },
            )
        }
    } else {
        vk::SubresourceLayout::default()
    };
    let allocation = Arc::new(allocation);
    let size = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture_descriptor = wgpu::TextureDescriptor {
        label: Some("exported native tile"),
        size,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    };
    let owner = Arc::clone(&allocation);
    let hal_texture = unsafe {
        hal.texture_from_raw(
            image,
            &wgpu::hal::TextureDescriptor {
                label: texture_descriptor.label,
                size,
                mip_level_count: 1,
                sample_count: 1,
                dimension: texture_descriptor.dimension,
                format: texture_descriptor.format,
                usage: wgpu::TextureUses::COPY_DST | wgpu::TextureUses::RESOURCE,
                memory_flags: wgpu::hal::MemoryFlags::empty(),
                view_formats: vec![],
            },
            Some(Box::new(move || drop(owner))),
            wgpu::hal::vulkan::TextureMemory::External,
        )
    };
    drop(hal);
    // This immutable export is separate from Pax's mutable retained backing.
    // Queue ordering places the full-image copy after Pax's flush submission.
    let texture = unsafe {
        context
            .device
            .create_texture_from_hal::<Vulkan>(hal_texture, &texture_descriptor)
    };
    let source = render_pax_pattern(probe, width, height, red, green, blue)?;
    let mut encoder = context.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_texture(source.as_image_copy(), texture.as_image_copy(), size);
    let submission = context.queue.submit([encoder.finish()]);
    if !context.async_dmabuf {
        context.device.poll(wgpu::PollType::Wait {
            submission_index: Some(submission),
            timeout: Some(Duration::from_secs(10)),
        })?;
    }

    // This single-threaded probe submits both operations to the same queue.
    // The barrier follows the WGPU copy; WGPU never accesses this image again.
    let hal = unsafe { context.device.as_hal::<Vulkan>() }.context("device disappeared")?;
    let raw = hal.raw_device();
    let pool = unsafe {
        raw.create_command_pool(
            &vk::CommandPoolCreateInfo::default().queue_family_index(hal.queue_family_index()),
            None,
        )
    }?;
    let mut pending = PendingTransfer {
        allocation: Arc::clone(&allocation),
        pool,
        fence: vk::Fence::null(),
        semaphore: vk::Semaphore::null(),
        submitted: false,
        test_event: vk::Event::null(),
        test_worker: None,
    };
    pending.fence = unsafe { raw.create_fence(&vk::FenceCreateInfo::default(), None) }?;
    if context.async_dmabuf {
        let mut export = vk::ExportSemaphoreCreateInfo::default()
            .handle_types(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD);
        pending.semaphore = unsafe {
            raw.create_semaphore(
                &vk::SemaphoreCreateInfo::default().push_next(&mut export),
                None,
            )
        }?;
    }
    // One dedicated test image holds the producer in flight so a fast GPU
    // cannot turn the synchronization test into an already-completed handoff.
    if width == 81
        && height == 49
        && context
            .delayed_test_available
            .swap(false, Ordering::Relaxed)
    {
        pending.test_event = unsafe { raw.create_event(&vk::EventCreateInfo::default(), None) }?;
    }
    let transfer_result = (|| -> Result<()> {
        let command = unsafe {
            raw.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )
        }?[0];
        unsafe {
            raw.begin_command_buffer(command, &vk::CommandBufferBeginInfo::default())?;
            if pending.test_event != vk::Event::null() {
                raw.cmd_wait_events(
                    command,
                    &[pending.test_event],
                    vk::PipelineStageFlags::HOST,
                    vk::PipelineStageFlags::TRANSFER,
                    &[],
                    &[],
                    &[],
                );
            }
            let barrier = vk::ImageMemoryBarrier::default()
                .image(image)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE)
                .dst_access_mask(vk::AccessFlags::empty())
                .src_queue_family_index(hal.queue_family_index())
                .dst_queue_family_index(if context.async_dmabuf {
                    vk::QUEUE_FAMILY_FOREIGN_EXT
                } else {
                    vk::QUEUE_FAMILY_EXTERNAL
                })
                .subresource_range(vk::ImageSubresourceRange {
                    aspect_mask: vk::ImageAspectFlags::COLOR,
                    base_mip_level: 0,
                    level_count: 1,
                    base_array_layer: 0,
                    layer_count: 1,
                });
            raw.cmd_pipeline_barrier(
                command,
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[barrier],
            );
            raw.end_command_buffer(command)?;
            let commands = [command];
            let signals = if context.async_dmabuf {
                vec![pending.semaphore]
            } else {
                vec![]
            };
            raw.queue_submit(
                hal.raw_queue(),
                &[vk::SubmitInfo::default()
                    .command_buffers(&commands)
                    .signal_semaphores(&signals)],
                pending.fence,
            )?;
            pending.submitted = true;
            if pending.test_event != vk::Event::null() {
                let event = pending.test_event;
                let owner = Arc::clone(&allocation);
                match std::thread::Builder::new()
                    .name("pax-sync-test-gate".into())
                    .spawn(move || {
                        std::thread::sleep(Duration::from_secs(2));
                        if let Err(error) = owner.raw.set_event(event) {
                            eprintln!("PAX_WGPU_EXPORT_ERROR: test gate signal: {error:?}");
                        }
                    }) {
                    Ok(worker) => pending.test_worker = Some(worker),
                    Err(error) => {
                        raw.set_event(event)?;
                        return Err(error.into());
                    }
                }
            }
            if !context.async_dmabuf {
                raw.queue_wait_idle(hal.raw_queue())?;
            }
        }
        Ok(())
    })();
    transfer_result?;
    let memory_fd =
        ash::khr::external_memory_fd::Device::new(hal.shared_instance().raw_instance(), raw);
    let stride = layout.row_pitch.try_into()?;
    let offset = layout.offset.try_into()?;
    let fd = unsafe {
        OwnedFd::from_raw_fd(
            memory_fd.get_memory_fd(
                &vk::MemoryGetFdInfoKHR::default()
                    .memory(allocation.memory)
                    .handle_type(context.handle_type),
            )?,
        )
    };
    let sync_fd = if context.async_dmabuf {
        let api =
            ash::khr::external_semaphore_fd::Device::new(hal.shared_instance().raw_instance(), raw);
        let fd = unsafe {
            api.get_semaphore_fd(
                &vk::SemaphoreGetFdInfoKHR::default()
                    .semaphore(pending.semaphore)
                    .handle_type(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD),
            )
        }?;
        // -1 is Vulkan's already-signaled sync_file sentinel, not an owned FD.
        if fd >= 0 {
            Some(unsafe { OwnedFd::from_raw_fd(fd) })
        } else {
            None
        }
    } else {
        None
    };
    let producer_pending = unsafe { !raw.get_fence_status(pending.fence)? };
    eprintln!("producer_handoff async={} ownership={} sync_fd={} cpu_wait_before_publish={} producer_pending={}",
        context.async_dmabuf, if context.async_dmabuf { "FOREIGN_EXT" } else { "EXTERNAL" },
        sync_fd.as_ref().map_or(-1, AsRawFd::as_raw_fd), !context.async_dmabuf, producer_pending);
    eprintln!(
        "wgpu_tile_export image={} fd={} {}x{} bytes={} dma_buf={} source=pax-gpu copies=1 no_readback=true",
        image.as_raw(),
        fd.as_raw_fd(),
        width,
        height,
        requirements.size,
        linear
    );
    Ok((
        Tile {
            allocation,
            texture: Some(texture),
            pending: Some(pending),
        },
        ExportInfo {
            fd: fd.into_raw_fd(),
            sync_fd: sync_fd.map_or(-1, IntoRawFd::into_raw_fd),
            width,
            height,
            stride,
            offset,
            allocation_size: requirements.size,
            fourcc: u32::from_le_bytes(*b"AB24"),
            modifier: 0,
            device_uuid: ids.device_uuid,
            driver_uuid: ids.driver_uuid,
        },
    ))
}

fn ffi_result<T>(operation: impl FnOnce() -> Result<T>) -> Option<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(Ok(value)) => Some(value),
        Ok(Err(error)) => {
            eprintln!("PAX_WGPU_EXPORT_ERROR: {error:#}");
            None
        }
        Err(_) => {
            eprintln!("PAX_WGPU_EXPORT_ERROR: panic contained at probe FFI");
            None
        }
    }
}

#[no_mangle]
pub extern "C" fn pax_probe_gpu_abi_version() -> u32 {
    2
}

#[no_mangle]
pub extern "C" fn pax_probe_gpu_new(dma_buf: bool) -> *mut c_void {
    ffi_result(|| context_new(dma_buf))
        .map(|context| Box::into_raw(Box::new(context)).cast())
        .unwrap_or(ptr::null_mut())
}

#[no_mangle]
/// # Safety
/// `context` must be null or an unreleased result of `pax_probe_gpu_new`.
/// Creation, use, and destruction of the context occur on the GTK main thread.
pub unsafe extern "C" fn pax_probe_gpu_drop(context: *mut c_void) {
    if !context.is_null() {
        drop(Box::from_raw(context.cast::<ProbeContext>()));
    }
}

#[no_mangle]
/// # Safety
/// `context` must be a live context created by this library on the current
/// thread; `info` must point to writable storage for this revision's ExportInfo.
pub unsafe extern "C" fn pax_probe_gpu_tile(
    context: *const c_void,
    width: u32,
    height: u32,
    red: f64,
    green: f64,
    blue: f64,
    info: *mut ExportInfo,
) -> *mut c_void {
    ffi_result(|| {
        if context.is_null() || info.is_null() {
            return Err(anyhow!("null export argument"));
        }
        let (tile, output) = export_tile(
            &*context.cast::<ProbeContext>(),
            width,
            height,
            red,
            green,
            blue,
        )?;
        ptr::write(info, output);
        Ok(Box::into_raw(Box::new(tile)).cast())
    })
    .unwrap_or(ptr::null_mut())
}

#[no_mangle]
/// # Safety
/// `tile` must be null or an unreleased tile from this library. All external
/// consumers must have finished using its memory before releasing the tile.
pub unsafe extern "C" fn pax_probe_gpu_tile_drop(tile: *mut c_void) {
    if !tile.is_null() {
        drop(Box::from_raw(tile.cast::<Tile>()));
    }
}

#[no_mangle]
pub extern "C" fn pax_probe_gpu_live_allocations() -> usize {
    LIVE_ALLOCATIONS.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uuid_pin_is_exact_and_rejects_malformed_input() {
        let expected = [
            0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45, 0x67, 0x89, 0xab,
            0xcd, 0xef,
        ];
        assert_eq!(
            parse_device_uuid("0123456789abcdef0123456789ABCDEF").unwrap(),
            expected
        );
        for invalid in [
            "",
            "0123456789abcdef",
            "g123456789abcdef0123456789abcdef",
            "é23456789abcdef0123456789abcdef0",
        ] {
            assert!(parse_device_uuid(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn diagnostic_adapter_types_cannot_qualify_physical_hardware() {
        for device_type in [
            wgpu::DeviceType::Cpu,
            wgpu::DeviceType::VirtualGpu,
            wgpu::DeviceType::Other,
        ] {
            assert!(!physical_gpu(device_type));
        }
        assert!(physical_gpu(wgpu::DeviceType::IntegratedGpu));
        assert!(physical_gpu(wgpu::DeviceType::DiscreteGpu));
    }
}

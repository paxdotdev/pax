use anyhow::{bail, Context, Result};
use ash::vk;
use pax_chassis_common::desktop::{DesktopGpuBackend, DesktopSurfaceFrame, DesktopSurfaceInfo};
use pax_gpu::render_backend::{GpuContext, SharedGpuContext};
use serde_json::{json, Value};
use std::{
    collections::{HashMap, HashSet},
    os::fd::{AsRawFd, FromRawFd, IntoRawFd, OwnedFd},
    sync::{
        atomic::{AtomicU64, AtomicUsize, Ordering},
        Arc,
    },
};

type Vulkan = wgpu::hal::api::Vulkan;
const MAX_SLOTS: usize = 48;
const MAX_BYTES: u64 = 256 * 1024 * 1024;
const SLOTS_PER_SURFACE: usize = 3;
struct ContextState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    live_bytes: AtomicU64,
    live_images: AtomicUsize,
}
struct LinuxContext {
    gpu: Arc<ContextState>,
    pax: SharedGpuContext,
}
struct Allocation {
    context: Arc<ContextState>,
    raw: ash::Device,
    image: vk::Image,
    memory: vk::DeviceMemory,
    bytes: u64,
}
impl Drop for Allocation {
    fn drop(&mut self) {
        unsafe {
            self.raw.destroy_image(self.image, None);
            if self.memory != vk::DeviceMemory::null() {
                self.raw.free_memory(self.memory, None);
                self.context
                    .live_bytes
                    .fetch_sub(self.bytes, Ordering::Relaxed);
                self.context.live_images.fetch_sub(1, Ordering::Relaxed);
            }
        }
    }
}
// A submission is destroyed only after a nonblocking fence query succeeds.
struct Submission {
    raw: ash::Device,
    pool: vk::CommandPool,
    fence: vk::Fence,
    signal: vk::Semaphore,
    wait: vk::Semaphore,
    completion_queued: bool,
}
impl Submission {
    fn ready(&self) -> Result<bool> {
        Ok(unsafe { self.raw.get_fence_status(self.fence) }?)
    }
}
impl Drop for Submission {
    fn drop(&mut self) {
        unsafe {
            if self.signal != vk::Semaphore::null() {
                self.raw.destroy_semaphore(self.signal, None);
            }
            if self.wait != vk::Semaphore::null() {
                self.raw.destroy_semaphore(self.wait, None);
            }
            self.raw.destroy_fence(self.fence, None);
            self.raw.destroy_command_pool(self.pool, None);
        }
    }
}
struct Slot {
    allocation: Arc<Allocation>,
    texture: wgpu::Texture,
    fd: OwnedFd,
    width: u32,
    height: u32,
    stride: u32,
    offset: u32,
    bytes: u64,
    surface: (usize, String),
    token: Option<u64>,
    pending: Option<Submission>,
    returned: bool,
    consumer: Option<OwnedFd>,
    external: bool,
}
impl Slot {
    fn idle(&self) -> bool {
        self.token.is_none() && self.pending.is_none()
    }
}
struct Backend {
    context: LinuxContext,
    slots: Vec<Slot>,
    requirements: HashMap<(u32, u32), u64>,
    reserved: Vec<usize>,
    next_token: u64,
    closing: bool,
    published: u64,
    reused: u64,
    blocked: u64,
}
/// Negotiate a physical Vulkan adapter with DMA-BUF and binary SYNC_FD support.
/// No software or CPU-readback fallback is installed by this native backend.
pub fn create_backend() -> std::result::Result<Box<dyn DesktopGpuBackend>, String> {
    Backend::new()
        .map(|backend| Box::new(backend) as Box<dyn DesktopGpuBackend>)
        .map_err(|error| format!("{error:#}"))
}

impl Backend {
    fn new() -> Result<Self> {
        let context = context_new()?;
        Ok(Self {
            context,
            slots: Vec::new(),
            requirements: HashMap::new(),
            reserved: Vec::new(),
            next_token: 1,
            closing: false,
            published: 0,
            reused: 0,
            blocked: 0,
        })
    }
}

fn parse_device_uuid(value: &str) -> Result<[u8; vk::UUID_SIZE]> {
    if value.len() != vk::UUID_SIZE * 2 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        bail!("PAX_GPU_DEVICE_UUID must contain exactly 32 hexadecimal digits");
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

fn context_new() -> Result<LinuxContext> {
    // Capability inventory and composition must exercise the same adapter on
    // hybrid systems. Never silently fall back after an explicit UUID request.
    let requested_uuid = match std::env::var("PAX_GPU_DEVICE_UUID") {
        Ok(value) => Some(parse_device_uuid(&value)?),
        Err(std::env::VarError::NotPresent) => None,
        Err(error) => return Err(error).context("Invalid PAX_GPU_DEVICE_UUID"),
    };
    let dma_buf = true;
    let async_dmabuf = true;
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
        let policy_matches = physical_gpu(info.device_type);
        eprintln!("wgpu_candidate={info:?} device_uuid={uuid} identity_matches={identity_matches} policy_matches={policy_matches}");
        identity_matches && policy_matches
    }).context("No eligible physical Vulkan adapter. An explicit PAX_GPU_DEVICE_UUID is never substituted.")?;
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
        if !properties.external_semaphore_features.contains(
            vk::ExternalSemaphoreFeatureFlags::EXPORTABLE
                | vk::ExternalSemaphoreFeatureFlags::IMPORTABLE,
        ) {
            bail!("SYNC_FD semaphore import/export is unavailable");
        }
    }
    for extension in &extensions {
        if !caps.supports_extension(extension) {
            bail!("Vulkan adapter does not support required extension {extension:?}");
        }
    }
    let descriptor = wgpu::DeviceDescriptor {
        label: Some("Pax Linux desktop"),
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
    Ok(LinuxContext {
        pax,
        gpu: Arc::new(ContextState {
            device,
            queue,
            live_bytes: AtomicU64::new(0),
            live_images: AtomicUsize::new(0),
        }),
    })
}

impl Backend {
    fn submit(&mut self, index: usize, frame: DesktopSurfaceFrame) -> Result<Value> {
        let token = self.next_token;
        self.next_token = self
            .next_token
            .checked_add(1)
            .context("frame token exhausted")?;
        let slot = &mut self.slots[index];
        let gpu = &self.context.gpu;
        let hal = unsafe { gpu.device.as_hal::<Vulkan>() }.context("Vulkan device unavailable")?;
        let raw = hal.raw_device();
        let external_semaphores =
            ash::khr::external_semaphore_fd::Device::new(hal.shared_instance().raw_instance(), raw);
        let pool = unsafe {
            raw.create_command_pool(
                &vk::CommandPoolCreateInfo::default().queue_family_index(hal.queue_family_index()),
                None,
            )
        }?;
        let fence = match unsafe { raw.create_fence(&vk::FenceCreateInfo::default(), None) } {
            Ok(fence) => fence,
            Err(error) => {
                unsafe {
                    raw.destroy_command_pool(pool, None);
                }
                return Err(error.into());
            }
        };
        let mut submission = Submission {
            raw: raw.clone(),
            pool,
            fence,
            signal: vk::Semaphore::null(),
            wait: vk::Semaphore::null(),
            completion_queued: false,
        };
        let mut export = vk::ExportSemaphoreCreateInfo::default()
            .handle_types(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD);
        submission.signal = unsafe {
            raw.create_semaphore(
                &vk::SemaphoreCreateInfo::default().push_next(&mut export),
                None,
            )
        }?;
        if let Some(consumer) = slot.consumer.take() {
            submission.wait =
                unsafe { raw.create_semaphore(&vk::SemaphoreCreateInfo::default(), None) }?;
            // SYNC_FD import transfers descriptor ownership only on success.
            unsafe {
                external_semaphores.import_semaphore_fd(
                    &vk::ImportSemaphoreFdInfoKHR::default()
                        .semaphore(submission.wait)
                        .flags(vk::SemaphoreImportFlags::TEMPORARY)
                        .handle_type(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD)
                        .fd(consumer.as_raw_fd()),
                )
            }?;
            let _ = consumer.into_raw_fd();
        }
        let commands = unsafe {
            raw.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(2),
            )
        }?;
        let range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);
        let family = hal.queue_family_index();
        let queue = hal.raw_queue();
        // From this point a partial error is terminal and the slot retains every
        // object: a failed submission must never free a command pool still in use.
        slot.token = Some(token);
        slot.returned = false;
        slot.pending = Some(submission);
        let pending = slot.pending.as_ref().unwrap();
        if slot.external {
            unsafe {
                raw.begin_command_buffer(commands[0], &vk::CommandBufferBeginInfo::default())?;
                let acquire = vk::ImageMemoryBarrier::default()
                    .image(slot.allocation.image)
                    .subresource_range(range)
                    .old_layout(vk::ImageLayout::GENERAL)
                    .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                    .src_queue_family_index(vk::QUEUE_FAMILY_FOREIGN_EXT)
                    .dst_queue_family_index(family)
                    .dst_access_mask(vk::AccessFlags::TRANSFER_WRITE);
                raw.cmd_pipeline_barrier(
                    commands[0],
                    vk::PipelineStageFlags::TOP_OF_PIPE,
                    vk::PipelineStageFlags::TRANSFER,
                    vk::DependencyFlags::empty(),
                    &[],
                    &[],
                    &[acquire],
                );
                raw.end_command_buffer(commands[0])?;
                let waits = if pending.wait == vk::Semaphore::null() {
                    Vec::new()
                } else {
                    vec![pending.wait]
                };
                let stages = vec![vk::PipelineStageFlags::TRANSFER; waits.len()];
                raw.queue_submit(
                    queue,
                    &[vk::SubmitInfo::default()
                        .command_buffers(&commands[..1])
                        .wait_semaphores(&waits)
                        .wait_dst_stage_mask(&stages)],
                    vk::Fence::null(),
                )?;
            }
            self.reused += 1;
        }
        drop(hal);
        // WGPU tracks the retained output's layout. The external destination is
        // returned to COPY_DST above, matching its last tracked WGPU use.
        let mut encoder = gpu.device.create_command_encoder(&Default::default());
        encoder.copy_texture_to_texture(
            frame.texture.as_image_copy(),
            slot.texture.as_image_copy(),
            wgpu::Extent3d {
                width: slot.width,
                height: slot.height,
                depth_or_array_layers: 1,
            },
        );
        gpu.queue.submit([encoder.finish()]);
        let hal = unsafe { gpu.device.as_hal::<Vulkan>() }.context("Vulkan device unavailable")?;
        let raw = hal.raw_device();
        unsafe {
            raw.begin_command_buffer(commands[1], &vk::CommandBufferBeginInfo::default())?;
            let release = vk::ImageMemoryBarrier::default()
                .image(slot.allocation.image)
                .subresource_range(range)
                .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
                .new_layout(vk::ImageLayout::GENERAL)
                .src_queue_family_index(family)
                .dst_queue_family_index(vk::QUEUE_FAMILY_FOREIGN_EXT)
                .src_access_mask(vk::AccessFlags::TRANSFER_WRITE);
            raw.cmd_pipeline_barrier(
                commands[1],
                vk::PipelineStageFlags::TRANSFER,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                vk::DependencyFlags::empty(),
                &[],
                &[],
                &[release],
            );
            raw.end_command_buffer(commands[1])?;
            raw.queue_submit(
                queue,
                &[vk::SubmitInfo::default()
                    .command_buffers(&commands[1..])
                    .signal_semaphores(&[pending.signal])],
                pending.fence,
            )?;
        }
        slot.pending.as_mut().unwrap().completion_queued = true;
        slot.external = true;
        let signal = slot.pending.as_ref().unwrap().signal;
        let sync_fd = unsafe {
            external_semaphores.get_semaphore_fd(
                &vk::SemaphoreGetFdInfoKHR::default()
                    .semaphore(signal)
                    .handle_type(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD),
            )
        }?;
        let sync_fd = exported_sync_fd(sync_fd)?;
        let image_fd = slot.fd.try_clone()?;
        self.published += 1;
        let producer_complete = sync_fd.is_none();
        // Both FDs transfer to the in-process host; it closes or imports each once.
        Ok(
            json!({"token":token,"surface":frame.info,"fd":image_fd.into_raw_fd(),"syncFd":sync_fd.map(IntoRawFd::into_raw_fd).unwrap_or(-1),"producerComplete":producer_complete,
        "stride":slot.stride,"offset":slot.offset,"fourcc":0x34324241_u32,"modifier":0_u64}),
        )
    }

    fn allocate(&self, info: &DesktopSurfaceInfo) -> Result<Slot> {
        let width = info.width;
        let height = info.height;
        let context = &self.context.gpu;
        let hal = unsafe { context.device.as_hal::<Vulkan>() }.context("device is not Vulkan")?;
        let raw = hal.raw_device();
        let instance = hal.shared_instance().raw_instance();
        let physical = hal.raw_physical_device();
        let mut ids = vk::PhysicalDeviceIDProperties::default();
        let mut device_properties = vk::PhysicalDeviceProperties2::default().push_next(&mut ids);
        unsafe { instance.get_physical_device_properties2(physical, &mut device_properties) };
        let linear = true;
        let tiling = if linear {
            vk::ImageTiling::LINEAR
        } else {
            vk::ImageTiling::OPTIMAL
        };
        let usage = vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED;
        let mut external_query = vk::PhysicalDeviceExternalImageFormatInfo::default()
            .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
        let query = vk::PhysicalDeviceImageFormatInfo2::default()
            .format(vk::Format::R8G8B8A8_UNORM)
            .ty(vk::ImageType::TYPE_2D)
            .tiling(tiling)
            .usage(usage)
            .push_next(&mut external_query);
        let mut external_properties = vk::ExternalImageFormatProperties::default();
        let mut properties =
            vk::ImageFormatProperties2::default().push_next(&mut external_properties);
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
        let mut external_create = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
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
            bytes: 0,
        };
        let requirements = unsafe { raw.get_image_memory_requirements(image) };
        let memory_properties = unsafe { instance.get_physical_device_memory_properties(physical) };
        let memory_type = (0..memory_properties.memory_type_count)
            .find(|i| requirements.memory_type_bits & (1 << i) != 0)
            .context("no compatible image memory type")?;
        let mut export = vk::ExportMemoryAllocateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
        let mut dedicated = vk::MemoryDedicatedAllocateInfo::default().image(image);
        let allocate = vk::MemoryAllocateInfo::default()
            .allocation_size(requirements.size)
            .memory_type_index(memory_type)
            .push_next(&mut export)
            .push_next(&mut dedicated);
        if context.live_bytes.load(Ordering::Relaxed) + requirements.size > MAX_BYTES {
            bail!("allocation quota changed after reservation");
        }
        allocation.memory = unsafe { raw.allocate_memory(&allocate, None) }?;
        allocation.bytes = requirements.size;
        context
            .live_bytes
            .fetch_add(requirements.size, Ordering::Relaxed);
        context.live_images.fetch_add(1, Ordering::Relaxed);
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
        let texture = unsafe {
            context
                .device
                .create_texture_from_hal::<Vulkan>(hal_texture, &texture_descriptor)
        };
        let hal =
            unsafe { context.device.as_hal::<Vulkan>() }.context("Vulkan device unavailable")?;
        let export = ash::khr::external_memory_fd::Device::new(
            hal.shared_instance().raw_instance(),
            hal.raw_device(),
        );
        let fd = unsafe {
            export.get_memory_fd(
                &vk::MemoryGetFdInfoKHR::default()
                    .memory(allocation.memory)
                    .handle_type(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT),
            )
        }?;
        Ok(Slot {
            allocation,
            texture,
            fd: unsafe { OwnedFd::from_raw_fd(fd) },
            width,
            height,
            stride: u32::try_from(layout.row_pitch)?,
            offset: u32::try_from(layout.offset)?,
            bytes: requirements.size,
            surface: (info.layer, info.key.clone()),
            token: None,
            pending: None,
            returned: false,
            consumer: None,
            external: false,
        })
    }
}

#[repr(C)]
#[derive(Default)]
struct SyncFileInfo {
    name: [u8; 32],
    status: i32,
    flags: u32,
    num_fences: u32,
    pad: u32,
    fences: u64,
}

fn exported_sync_fd(fd: i32) -> Result<Option<OwnedFd>> {
    // Vulkan permits -1 for an already-signaled SYNC_FD payload. It is
    // completion evidence, not an owned descriptor or an EGL import request.
    match fd {
        -1 => Ok(None),
        0.. => Ok(Some(unsafe { OwnedFd::from_raw_fd(fd) })),
        _ => bail!("Vulkan returned an invalid exported SYNC_FD"),
    }
}

fn sync_file_status(fd: i32) -> Result<i32> {
    // Linux UAPI linux/sync_file.h: _IOWR('>', 4, struct sync_file_info).
    const SYNC_IOC_FILE_INFO: libc::c_ulong = 0xc0383e04;
    let mut info = SyncFileInfo::default();
    if unsafe { libc::ioctl(fd, SYNC_IOC_FILE_INFO, &mut info) } < 0 {
        return Err(std::io::Error::last_os_error())
            .context("consumer descriptor is not a SYNC_FD");
    }
    if info.status < 0 {
        bail!(
            "consumer completion fence failed with status {}",
            info.status
        );
    }
    Ok(info.status)
}

fn consumer_ready(fd: &OwnedFd) -> Result<bool> {
    let mut descriptor = libc::pollfd {
        fd: fd.as_raw_fd(),
        events: libc::POLLIN,
        revents: 0,
    };
    let result = unsafe { libc::poll(&mut descriptor, 1, 0) };
    if result < 0 {
        let error = std::io::Error::last_os_error();
        if error.kind() == std::io::ErrorKind::Interrupted {
            return Ok(false);
        }
        return Err(error.into());
    }
    if descriptor.revents & (libc::POLLERR | libc::POLLNVAL) != 0 {
        bail!("consumer completion fence failed");
    }
    Ok(result > 0
        && descriptor.revents & libc::POLLIN != 0
        && sync_file_status(fd.as_raw_fd())? > 0)
}
impl Backend {
    fn discard_descriptors(&mut self, frames: &[Value]) {
        for frame in frames {
            for field in ["fd", "syncFd"] {
                let fd = frame[field].as_i64().unwrap() as i32;
                if fd >= 0 {
                    unsafe {
                        libc::close(fd);
                    }
                }
            }
            if let Some(slot) = self
                .slots
                .iter_mut()
                .find(|slot| slot.token == frame["token"].as_u64())
            {
                slot.returned = true;
            }
        }
    }
    fn complete_unhanded(&mut self, index: usize) {
        let slot = &mut self.slots[index];
        slot.returned = true;
        if let Some(pending) = slot.pending.as_mut() {
            if !pending.completion_queued {
                // No descriptor escaped. A final empty submission fences any
                // partial acquire/copy, permitting safe asynchronous teardown.
                // A failed recovery leaves the slot pinned on device failure.
                if let Some(hal) = unsafe { self.context.gpu.device.as_hal::<Vulkan>() } {
                    pending.completion_queued = unsafe {
                        hal.raw_device().queue_submit(
                            hal.raw_queue(),
                            &[vk::SubmitInfo::default()],
                            pending.fence,
                        )
                    }
                    .is_ok();
                }
            }
        }
    }
    fn required_bytes(&mut self, width: u32, height: u32) -> Result<u64> {
        if let Some(bytes) = self.requirements.get(&(width, height)) {
            return Ok(*bytes);
        }
        let hal = unsafe { self.context.gpu.device.as_hal::<Vulkan>() }
            .context("Vulkan device unavailable")?;
        let raw = hal.raw_device();
        let mut external = vk::ExternalMemoryImageCreateInfo::default()
            .handle_types(vk::ExternalMemoryHandleTypeFlags::DMA_BUF_EXT);
        let descriptor = vk::ImageCreateInfo::default()
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
            .tiling(vk::ImageTiling::LINEAR)
            .usage(vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::SAMPLED)
            .sharing_mode(vk::SharingMode::EXCLUSIVE)
            .initial_layout(vk::ImageLayout::UNDEFINED)
            .push_next(&mut external);
        let image = unsafe { raw.create_image(&descriptor, None) }?;
        let bytes = unsafe { raw.get_image_memory_requirements(image) }.size;
        unsafe {
            raw.destroy_image(image, None);
        }
        if self.requirements.len() > 512 {
            self.requirements.clear();
        }
        self.requirements.insert((width, height), bytes);
        Ok(bytes)
    }
    fn poll_ready(&mut self) -> Result<()> {
        let _ = self.context.gpu.device.poll(wgpu::PollType::Poll)?;
        for slot in &mut self.slots {
            if slot
                .pending
                .as_ref()
                .is_some_and(|p| p.ready().unwrap_or(false))
            {
                slot.pending.take();
            }
            // Propagate device loss; do not silently turn it into backpressure.
            if let Some(pending) = &slot.pending {
                pending.ready()?;
            }
            if slot.returned
                && slot.pending.is_none()
                && slot
                    .consumer
                    .as_ref()
                    .map(consumer_ready)
                    .transpose()?
                    .unwrap_or(true)
            {
                slot.token = None;
            }
        }
        Ok(())
    }
    fn capacity(&mut self, surfaces: &[DesktopSurfaceInfo]) -> Result<bool> {
        self.reserved.clear();
        if self.closing {
            bail!("Linux renderer is closing");
        }
        self.poll_ready()?;
        if surfaces.len() > MAX_SLOTS {
            bail!("frame exceeds Linux export surface limit {MAX_SLOTS}");
        }
        let mut identities = HashSet::new();
        let mut minimum_bytes = 0_u64;
        for info in surfaces {
            if info.width == 0
                || info.height == 0
                || !identities.insert((info.layer, info.key.clone()))
            {
                bail!("invalid or duplicate Linux frame surface");
            }
            let bytes = self.required_bytes(info.width, info.height)?;
            minimum_bytes = minimum_bytes
                .checked_add(bytes)
                .context("frame byte size overflow")?;
        }
        if minimum_bytes > MAX_BYTES {
            bail!("frame exceeds Linux export byte budget {MAX_BYTES}");
        }
        self.slots.retain(|slot| {
            !slot.idle()
                || surfaces.iter().any(|info| {
                    slot.surface == (info.layer, info.key.clone())
                        && slot.width == info.width
                        && slot.height == info.height
                })
        });
        self.context.gpu.device.poll(wgpu::PollType::Poll)?;
        for info in surfaces {
            let identity = (info.layer, info.key.clone());
            if let Some(index) = self.slots.iter().enumerate().find_map(|(index, slot)| {
                (slot.idle()
                    && slot.surface == identity
                    && slot.width == info.width
                    && slot.height == info.height
                    && !self.reserved.contains(&index))
                .then_some(index)
            }) {
                self.reserved.push(index);
                continue;
            }
            let generation_slots = self
                .slots
                .iter()
                .filter(|slot| {
                    slot.surface == identity
                        && slot.width == info.width
                        && slot.height == info.height
                })
                .count();
            let current_bytes = self.context.gpu.live_bytes.load(Ordering::Relaxed);
            let estimate = self.required_bytes(info.width, info.height)?;
            if generation_slots >= SLOTS_PER_SURFACE
                || self.context.gpu.live_images.load(Ordering::Relaxed) >= MAX_SLOTS
                || current_bytes + estimate > MAX_BYTES
            {
                self.reserved.clear();
                self.blocked += 1;
                return Ok(false);
            }
            let slot = self.allocate(info)?;
            if slot.bytes > MAX_BYTES {
                bail!("one DMA-BUF allocation exceeds Linux export byte budget");
            }
            if current_bytes + slot.bytes > MAX_BYTES {
                self.reserved.clear();
                self.blocked += 1;
                return Ok(false);
            }
            self.reserved.push(self.slots.len());
            self.slots.push(slot);
        }
        Ok(true)
    }
}
impl DesktopGpuBackend for Backend {
    fn context(&self) -> SharedGpuContext {
        self.context.pax.clone()
    }
    fn format(&self) -> wgpu::TextureFormat {
        wgpu::TextureFormat::Rgba8Unorm
    }
    fn poll(&mut self) -> std::result::Result<(), String> {
        self.poll_ready().map_err(|e| format!("{e:#}"))
    }
    fn can_publish(
        &mut self,
        surfaces: &[DesktopSurfaceInfo],
    ) -> std::result::Result<bool, String> {
        self.capacity(surfaces).map_err(|e| format!("{e:#}"))
    }
    fn publish(
        &mut self,
        surfaces: Vec<DesktopSurfaceFrame>,
    ) -> std::result::Result<Value, String> {
        if self.closing {
            return Err("Linux renderer is closing".into());
        }
        let reserved = std::mem::take(&mut self.reserved);
        let mut frames = Vec::with_capacity(surfaces.len());
        let mut used = HashSet::new();
        for frame in surfaces {
            let index = reserved.iter().copied().find(|index| {
                let slot = &self.slots[*index];
                slot.surface == (frame.info.layer, frame.info.key.clone())
                    && slot.width == frame.info.width
                    && slot.height == frame.info.height
                    && !used.contains(index)
            });
            let Some(index) = index else {
                self.discard_descriptors(&frames);
                self.closing = true;
                return Err("Linux publication has no reserved surface slot".into());
            };
            used.insert(index);
            match self.submit(index, frame) {
                Ok(frame) => frames.push(frame),
                Err(error) => {
                    self.discard_descriptors(&frames);
                    self.complete_unhanded(index);
                    self.closing = true;
                    return Err(format!("Linux GPU publication failed: {error:#}"));
                }
            }
        }
        Ok(json!({"version":1,"transport":"dmabuf-egl","frames":frames}))
    }
    fn release(&mut self, token: u64, completion: Value) -> std::result::Result<(), String> {
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.token == Some(token) && !slot.returned)
            .ok_or_else(|| "unknown, stale or duplicate Linux frame token".to_string())?;
        let consumed = completion["consumed"]
            .as_bool()
            .ok_or("missing consumed flag")?;
        if consumed {
            let fd = completion["syncFd"]
                .as_i64()
                .and_then(|value| i32::try_from(value).ok())
                .filter(|fd| *fd >= 0)
                .ok_or("missing consumer SYNC_FD")?;
            // Validate the descriptor before accepting ownership. The host closes
            // it itself only if this call rejects the release.
            sync_file_status(fd).map_err(|error| format!("{error:#}"))?;
            slot.consumer = Some(unsafe { OwnedFd::from_raw_fd(fd) });
        }
        slot.returned = true;
        Ok(())
    }
    fn shutdown_poll(&mut self) -> std::result::Result<bool, String> {
        self.closing = true;
        self.poll_ready().map_err(|e| format!("{e:#}"))?;
        self.slots.retain(|slot| !slot.idle());
        self.context
            .gpu
            .device
            .poll(wgpu::PollType::Poll)
            .map_err(|e| e.to_string())?;
        Ok(self.slots.is_empty() && self.context.gpu.live_images.load(Ordering::Relaxed) == 0)
    }
    fn diagnostics(&self) -> Value {
        json!({"transport":"dmabuf-egl","slots":self.slots.len(),"maxSlots":MAX_SLOTS,
            "bytes":self.context.gpu.live_bytes.load(Ordering::Relaxed),"maxBytes":MAX_BYTES,
            "liveAllocations":self.context.gpu.live_images.load(Ordering::Relaxed),
            "pending":self.slots.iter().filter(|slot| slot.token.is_some()).count(),
            "published":self.published,"reused":self.reused,"backpressure":self.blocked,
            "closing":self.closing,"cpuPresentationReadback":false})
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn surface(width: u32) -> DesktopSurfaceInfo {
        DesktopSurfaceInfo {
            layer: 0,
            key: "lifecycle".into(),
            host_signature: format!("generation-{width}"),
            origin_x: 0.0,
            origin_y: 0.0,
            logical_width: width as f32,
            logical_height: 32.0,
            width,
            height: 32,
            dpr: [1.0; 2],
            replay_priority: 0,
        }
    }

    fn publish_without_consumer(backend: &mut Backend, info: DesktopSurfaceInfo) -> u64 {
        let texture = backend
            .context
            .gpu
            .device
            .create_texture(&wgpu::TextureDescriptor {
                label: Some("Linux lifecycle test source"),
                size: wgpu::Extent3d {
                    width: info.width,
                    height: info.height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: backend.format(),
                usage: wgpu::TextureUsages::COPY_SRC | wgpu::TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            });
        // The test never imports this image into another API. Closing its
        // descriptor copies is therefore consistent with consumed:false.
        let result = backend
            .publish(vec![DesktopSurfaceFrame { info, texture }])
            .unwrap();
        let frame = &result["frames"][0];
        for field in ["fd", "syncFd"] {
            let fd = frame[field].as_i64().unwrap() as i32;
            if fd >= 0 {
                drop(unsafe { OwnedFd::from_raw_fd(fd) });
            }
        }
        frame["token"].as_u64().unwrap()
    }

    #[test]
    #[ignore = "requires physical Vulkan DMA-BUF/SYNC_FD hardware"]
    fn hardware_leases_resize_budget_and_close() {
        let mut backend = Backend::new().expect("physical Vulkan backend");
        let mut tokens = Vec::new();
        for _ in 0..SLOTS_PER_SURFACE {
            let info = surface(32);
            assert!(backend.can_publish(std::slice::from_ref(&info)).unwrap());
            tokens.push(publish_without_consumer(&mut backend, info));
        }
        assert!(!backend.can_publish(&[surface(32)]).unwrap());
        assert_eq!(backend.diagnostics()["pending"], SLOTS_PER_SURFACE);

        // Resize generations share the same global quota with leases that the
        // consumer still owns. They cannot reset the budget or evict old data.
        for width in 33..33 + (MAX_SLOTS - SLOTS_PER_SURFACE) as u32 {
            let info = surface(width);
            assert!(backend.can_publish(std::slice::from_ref(&info)).unwrap());
            tokens.push(publish_without_consumer(&mut backend, info));
        }
        assert_eq!(backend.diagnostics()["liveAllocations"], MAX_SLOTS);
        assert!(!backend.can_publish(&[surface(100)]).unwrap());
        assert!(backend.diagnostics()["bytes"].as_u64().unwrap() <= MAX_BYTES);
        let mut oversized = surface(8192);
        oversized.height = 8193;
        assert!(backend
            .can_publish(&[oversized])
            .unwrap_err()
            .contains("byte budget"));
        assert!(backend
            .release(u64::MAX, json!({"consumed":false}))
            .is_err());
        assert!(!backend.shutdown_poll().unwrap());
        assert!(backend.can_publish(&[surface(32)]).is_err());
        for token in tokens {
            backend.release(token, json!({"consumed":false})).unwrap();
            assert!(backend.release(token, json!({"consumed":false})).is_err());
        }
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !backend.shutdown_poll().unwrap() {
            assert!(std::time::Instant::now() < deadline, "close did not drain");
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert_eq!(backend.diagnostics()["liveAllocations"], 0);
        assert_eq!(backend.diagnostics()["bytes"], 0);
    }

    #[test]
    fn ordinary_file_is_not_consumer_completion_evidence() {
        let file = std::fs::File::open("/dev/null").unwrap();
        assert!(sync_file_status(file.as_raw_fd()).is_err());
        // Rejection does not take descriptor ownership.
        assert!(unsafe { libc::fcntl(file.as_raw_fd(), libc::F_GETFD) } >= 0);
    }

    #[test]
    fn already_signaled_export_is_not_an_owned_negative_descriptor() {
        assert!(exported_sync_fd(-1).unwrap().is_none());
        assert!(exported_sync_fd(-2).is_err());
    }

    #[test]
    #[ignore = "requires physical Vulkan DMA-BUF/SYNC_FD hardware"]
    fn hardware_pending_consumer_blocks_reuse_and_close() {
        let mut backend = Backend::new().expect("physical Vulkan backend");
        let info = surface(32);
        assert!(backend.can_publish(std::slice::from_ref(&info)).unwrap());
        let token = publish_without_consumer(&mut backend, info);
        let device = Arc::clone(&backend.context.gpu);
        let hal = unsafe { device.device.as_hal::<Vulkan>() }.unwrap();
        let raw = hal.raw_device().clone();
        let event = unsafe { raw.create_event(&vk::EventCreateInfo::default(), None) }.unwrap();
        let pool = unsafe {
            raw.create_command_pool(
                &vk::CommandPoolCreateInfo::default().queue_family_index(hal.queue_family_index()),
                None,
            )
        }
        .unwrap();
        let command = unsafe {
            raw.allocate_command_buffers(
                &vk::CommandBufferAllocateInfo::default()
                    .command_pool(pool)
                    .level(vk::CommandBufferLevel::PRIMARY)
                    .command_buffer_count(1),
            )
        }
        .unwrap()[0];
        let mut export = vk::ExportSemaphoreCreateInfo::default()
            .handle_types(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD);
        let semaphore = unsafe {
            raw.create_semaphore(
                &vk::SemaphoreCreateInfo::default().push_next(&mut export),
                None,
            )
        }
        .unwrap();
        let fence = unsafe { raw.create_fence(&vk::FenceCreateInfo::default(), None) }.unwrap();
        unsafe {
            raw.begin_command_buffer(command, &vk::CommandBufferBeginInfo::default())
                .unwrap();
            // Probe-only consumer delay. This is an actual pending GPU job,
            // not a sleeping host thread or a manufactured ordinary file FD.
            raw.cmd_wait_events(
                command,
                &[event],
                vk::PipelineStageFlags::HOST,
                vk::PipelineStageFlags::BOTTOM_OF_PIPE,
                &[],
                &[],
                &[],
            );
            raw.end_command_buffer(command).unwrap();
            raw.queue_submit(
                hal.raw_queue(),
                &[vk::SubmitInfo::default()
                    .command_buffers(&[command])
                    .signal_semaphores(&[semaphore])],
                fence,
            )
            .unwrap();
        }
        let exports = ash::khr::external_semaphore_fd::Device::new(
            hal.shared_instance().raw_instance(),
            &raw,
        );
        let fd = exported_sync_fd(unsafe {
            exports
                .get_semaphore_fd(
                    &vk::SemaphoreGetFdInfoKHR::default()
                        .semaphore(semaphore)
                        .handle_type(vk::ExternalSemaphoreHandleTypeFlags::SYNC_FD),
                )
                .unwrap()
        })
        .unwrap()
        .expect("host event must still hold the consumer fence pending");
        drop(hal);
        assert_eq!(sync_file_status(fd.as_raw_fd()).unwrap(), 0);
        backend
            .release(token, json!({"consumed":true,"syncFd":fd.as_raw_fd()}))
            .unwrap();
        let _ = fd.into_raw_fd();
        for _ in 0..8 {
            backend.poll().unwrap();
            assert_eq!(backend.diagnostics()["pending"], 1);
            assert!(!backend.shutdown_poll().unwrap());
        }
        unsafe { raw.set_event(event) }.unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while !backend.shutdown_poll().unwrap() {
            assert!(
                std::time::Instant::now() < deadline,
                "consumer release did not drain"
            );
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        assert!(unsafe { raw.get_fence_status(fence) }.unwrap());
        unsafe {
            raw.destroy_fence(fence, None);
            raw.destroy_semaphore(semaphore, None);
            raw.destroy_command_pool(pool, None);
            raw.destroy_event(event, None);
        }
        assert_eq!(backend.diagnostics()["liveAllocations"], 0);
    }

    #[test]
    fn device_uuid_is_exact_and_does_not_accept_partial_selection() {
        assert_eq!(
            parse_device_uuid("00112233445566778899aabbccddeeff").unwrap()[15],
            255
        );
        assert!(parse_device_uuid("00112233").is_err());
        assert!(parse_device_uuid("00112233445566778899aabbccddeeff00").is_err());
        assert!(!physical_gpu(wgpu::DeviceType::Cpu));
        assert!(!physical_gpu(wgpu::DeviceType::VirtualGpu));
    }
}
impl Drop for Backend {
    fn drop(&mut self) {
        // Normal shutdown drains all slots first. Device loss, failed submits,
        // or a violated host close contract must pin memory rather than free
        // resources which an external queue may still access.
        for slot in self.slots.drain(..) {
            if !slot.idle() {
                std::mem::forget(slot);
            }
        }
    }
}

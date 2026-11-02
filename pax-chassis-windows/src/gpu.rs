use pax_chassis_common::desktop::{DesktopGpuBackend, DesktopSurfaceFrame, DesktopSurfaceInfo};
use pax_gpu::render_backend::{GpuContext, SharedGpuContext};
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::c_void;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use windows::Win32::Graphics::{Direct3D12::*, Dxgi::Common::*};
use windows_core::Interface;

type Dx12 = wgpu::hal::api::Dx12;
const TRANSPORT: &str = "dx12-composition-v1";
const FRAME_CAPACITY: usize = 3;
const IMAGE_CAPACITY: usize = 64;
const DEFAULT_EXPORT_BUDGET: u64 = 256 * 1024 * 1024;
const MAX_JSON_INTEGER: u64 = (1 << 53) - 1;
const ALLOCATOR_SAMPLE_INTERVAL: Duration = Duration::from_secs(1);
const ALLOCATOR_LABEL_LIMIT: usize = 16;
const ALLOCATOR_LABEL_CHARS: usize = 160;

#[derive(Default)]
struct AllocatorDiagnostics {
    sample: Option<(Instant, Value)>,
}

impl AllocatorDiagnostics {
    fn snapshot(
        &mut self,
        now: Instant,
        report: impl FnOnce() -> Option<wgpu::AllocatorReport>,
    ) -> Value {
        // Reports lock and walk the device allocator. Frequent shutdown polls
        // must reuse the sample rather than turning diagnostics into frame work.
        if self.sample.as_ref().map_or(true, |(at, _)| {
            now.saturating_duration_since(*at) >= ALLOCATOR_SAMPLE_INTERVAL
        }) {
            let started = Instant::now();
            let sample_unix_ms = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .ok()
                .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok());
            let mut value = allocator_summary(report());
            value["sampleUnixMs"] = json!(sample_unix_ms);
            value["sampleDurationMs"] = json!(started.elapsed().as_secs_f64() * 1000.0);
            self.sample = Some((now, value));
        }
        let (at, sample) = self.sample.as_ref().unwrap();
        let mut value = sample.clone();
        value["sampleAgeMs"] = json!(now.saturating_duration_since(*at).as_secs_f64() * 1000.0);
        value
    }
}

fn allocator_summary(report: Option<wgpu::AllocatorReport>) -> Value {
    let mut result = json!({"scope":"wgpu-device-suballocations",
        "includesProcessPrivateBytes":false,"includesNativeCompositionAllocations":false,
        "supported":report.is_some()});
    let Some(report) = report else {
        return result;
    };
    let mut labels = BTreeMap::<&str, (u64, u64)>::new();
    for allocation in &report.allocations {
        let (count, bytes) = labels.entry(&allocation.name).or_default();
        *count += 1;
        *bytes += allocation.size;
    }
    let mut labels: Vec<_> = labels.into_iter().collect();
    labels.sort_by(|(a, (_, a_bytes)), (b, (_, b_bytes))| {
        b_bytes.cmp(a_bytes).then_with(|| a.cmp(b))
    });
    let omitted = labels.iter().skip(ALLOCATOR_LABEL_LIMIT);
    result["omittedLabelCount"] = json!(labels.len().saturating_sub(ALLOCATOR_LABEL_LIMIT));
    result["omittedLabelBytes"] = json!(omitted.clone().map(|(_, (_, bytes))| bytes).sum::<u64>());
    result["omittedAllocationCount"] = json!(omitted.map(|(_, (count, _))| count).sum::<u64>());
    result["labels"] = json!(labels
        .into_iter()
        .take(ALLOCATOR_LABEL_LIMIT)
        .map(|(label, (count, bytes))| json!({
            "label":label.chars().take(ALLOCATOR_LABEL_CHARS).collect::<String>(),
            "labelTruncated":label.chars().count() > ALLOCATOR_LABEL_CHARS,
            "allocationCount":count,"allocatedBytes":bytes}))
        .collect::<Vec<_>>());
    result["liveAllocatedBytes"] = json!(report.total_allocated_bytes);
    result["reservedBytes"] = json!(report.total_reserved_bytes);
    // Reserved slack includes padding/fragmentation; it is not a leak count.
    result["unusedReservedBytes"] = json!(report
        .total_reserved_bytes
        .saturating_sub(report.total_allocated_bytes));
    result["allocationCount"] = json!(report.allocations.len());
    result["blockCount"] = json!(report.blocks.len());
    result["emptyBlockBytes"] = json!(report
        .blocks
        .iter()
        .filter(|block| block.allocations.is_empty())
        .map(|block| block.size)
        .sum::<u64>());
    result["blockReport"] = allocator_block_summary(&report);
    result
}

fn allocator_block_summary(report: &wgpu::AllocatorReport) -> Value {
    const BLOCK_LIMIT: usize = 16;
    const LABEL_LIMIT: usize = 8;
    let mut assigned = vec![false; report.allocations.len()];
    let mut ranges_valid = true;
    let mut ranges_disjoint = true;
    let mut layouts_valid = true;
    let mut block_live_total = Some(0u64);
    let mut omitted_live = Some(0u64);
    let mut omitted_reserved = Some(0u64);
    let mut blocks = Vec::with_capacity(report.blocks.len().min(BLOCK_LIMIT));
    for (index, block) in report.blocks.iter().enumerate() {
        let allocations = report.allocations.get(block.allocations.clone());
        let mut live = None;
        let mut offsets_valid = true;
        let mut offsets_disjoint = true;
        let mut overlaps_earlier_range = false;
        let mut labels = BTreeMap::<&str, (u64, u64)>::new();
        if let Some(allocations) = allocations {
            live = allocations
                .iter()
                .try_fold(0u64, |sum, a| sum.checked_add(a.size));
            let mut spans = Vec::with_capacity(allocations.len());
            for (relative, allocation) in allocations.iter().enumerate() {
                let assigned = &mut assigned[block.allocations.start + relative];
                overlaps_earlier_range |= *assigned;
                *assigned = true;
                match allocation.offset.checked_add(allocation.size) {
                    Some(end) if end <= block.size => {
                        // Empty allocations do not overlap an occupied byte interval.
                        if allocation.size != 0 {
                            spans.push((allocation.offset, end));
                        }
                    }
                    _ => offsets_valid = false,
                }
                if index < BLOCK_LIMIT && live.is_some() {
                    let (count, bytes) = labels.entry(&allocation.name).or_default();
                    *count += 1;
                    *bytes += allocation.size;
                }
            }
            spans.sort_unstable();
            let mut previous_end = 0;
            for (start, end) in spans {
                offsets_disjoint &= start >= previous_end;
                previous_end = previous_end.max(end);
            }
        } else {
            ranges_valid = false;
        }
        ranges_disjoint &= !overlaps_earlier_range;
        let fits = live.is_some_and(|bytes| bytes <= block.size);
        layouts_valid &= allocations.is_some() && offsets_valid && offsets_disjoint && fits;
        block_live_total =
            block_live_total.and_then(|total| live.and_then(|v| total.checked_add(v)));
        if index >= BLOCK_LIMIT {
            omitted_live = omitted_live.and_then(|total| live.and_then(|v| total.checked_add(v)));
            omitted_reserved = omitted_reserved.and_then(|total| total.checked_add(block.size));
            continue;
        }
        let mut labels: Vec<_> = labels.into_iter().collect();
        labels.sort_by(|(a, (_, a_bytes)), (b, (_, b_bytes))| {
            b_bytes.cmp(a_bytes).then_with(|| a.cmp(b))
        });
        let omitted = labels.iter().skip(LABEL_LIMIT);
        let labels_valid = allocations.is_some() && live.is_some();
        blocks.push(json!({
            "snapshotIndex":index,"reservedBytes":block.size,
            "liveAllocatedBytes":live,
            "unusedReservedBytes":live.and_then(|bytes| block.size.checked_sub(bytes)),
            "allocationCount":allocations.map(|a| a.len()),
            "allocationRangeValid":allocations.is_some(),
            "rangeOverlapsEarlierBlock":overlaps_earlier_range,
            "allocationOffsetsValid":allocations.is_some() && offsets_valid,
            "allocationOffsetsDisjoint":allocations.is_some() && offsets_disjoint,
            "liveFitsReserved":fits,
            "labelsValid":labels_valid,
            "omittedLabelCount":labels_valid.then_some(labels.len().saturating_sub(LABEL_LIMIT)),
            "omittedLabelBytes":labels_valid.then(|| omitted.clone().map(|(_, (_, bytes))| bytes).sum::<u64>()),
            "omittedAllocationCount":labels_valid.then(|| omitted.map(|(_, (count, _))| count).sum::<u64>()),
            "labels":labels.into_iter().take(LABEL_LIMIT).map(|(label, (count, bytes))| json!({
                "label":label.chars().take(ALLOCATOR_LABEL_CHARS).collect::<String>(),
                "labelTruncated":label.chars().count() > ALLOCATOR_LABEL_CHARS,
                "allocationCount":count,"allocatedBytes":bytes
            })).collect::<Vec<_>>()
        }));
    }
    let reserved = report
        .blocks
        .iter()
        .try_fold(0u64, |sum, block| sum.checked_add(block.size));
    let allocated = report
        .allocations
        .iter()
        .try_fold(0u64, |sum, a| sum.checked_add(a.size));
    let all_assigned = assigned.iter().all(|assigned| *assigned);
    let allocated_matches = allocated == Some(report.total_allocated_bytes);
    let reserved_matches = reserved == Some(report.total_reserved_bytes);
    let block_live_matches = block_live_total == Some(report.total_allocated_bytes);
    json!({
        "scope":"Reported wgpu block occupants only; indices are local to this snapshot, not stable block identities, heap types or VMMap addresses",
        "reportedBlockCount":report.blocks.len(),"displayedBlockCount":blocks.len(),
        "omittedBlockCount":report.blocks.len().saturating_sub(BLOCK_LIMIT),
        "omittedBlockReservedBytes":omitted_reserved,"omittedBlockLiveAllocatedBytes":omitted_live,
        "consistent":ranges_valid && ranges_disjoint && layouts_valid && all_assigned
            && allocated_matches && reserved_matches && block_live_matches,
        "consistency":{
            "allocationRangesValid":ranges_valid,"allocationRangesDisjoint":ranges_disjoint,
            "allocationLayoutsValid":layouts_valid,"allAllocationsAssigned":all_assigned,
            "allocationByteSumValid":allocated.is_some(),
            "blockLiveByteSumValid":block_live_total.is_some(),
            "blockReservedByteSumValid":reserved.is_some(),
            "allocationBytesMatchReportedTotal":allocated_matches,
            "blockLiveBytesMatchReportedTotal":block_live_matches,
            "blockReservedBytesMatchReportedTotal":reserved_matches
        },
        "blocks":blocks
    })
}

struct Completion {
    fence: ID3D12Fence,
    value: u64,
}
struct Image {
    texture: wgpu::Texture,
    info: DesktopSurfaceInfo,
    token: u64,
    completion: Option<Completion>,
    allocation_bytes: u64,
}
#[derive(Default)]
struct Batch {
    images: Vec<Image>,
    producer_done: Option<Arc<AtomicBool>>,
}
impl Batch {
    fn available(&self) -> bool {
        self.producer_done.is_none()
    }
}

struct WindowsGpu {
    context: SharedGpuContext,
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter: wgpu::AdapterInfo,
    batches: [Batch; FRAME_CAPACITY],
    next_token: u64,
    published: u64,
    retired: u64,
    backpressure: u64,
    closing: bool,
    error: Arc<Mutex<Option<String>>>,
    byte_budget: u64,
    allocator_diagnostics: Option<RefCell<AllocatorDiagnostics>>,
}

/// Creates an explicitly DX12 hardware backend. Software adapters are diagnostic
/// probe options, not an implicit fallback for the native application host.
pub fn create_backend() -> Result<Box<dyn DesktopGpuBackend>, String> {
    let byte_budget = match std::env::var("PAX_WINDOWS_EXPORT_BUDGET_BYTES") {
        Ok(value) => value
            .parse::<u64>()
            .ok()
            .filter(|bytes| *bytes >= 4)
            .ok_or("PAX_WINDOWS_EXPORT_BUDGET_BYTES must be an integer of at least 4")?,
        Err(std::env::VarError::NotPresent) => DEFAULT_EXPORT_BUDGET,
        Err(error) => return Err(error.to_string()),
    };
    Ok(Box::new(create_backend_with_budget(byte_budget)?))
}

fn create_backend_with_budget(byte_budget: u64) -> Result<WindowsGpu, String> {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
        backends: wgpu::Backends::DX12,
        ..Default::default()
    });
    let adapter = pollster::block_on(instance.enumerate_adapters(wgpu::Backends::DX12))
        .into_iter()
        .find(|adapter| {
            matches!(
                adapter.get_info().device_type,
                wgpu::DeviceType::IntegratedGpu | wgpu::DeviceType::DiscreteGpu
            )
        })
        .ok_or_else(|| {
            "No WGPU-compatible DX12 hardware adapter; software fallback is disabled".to_string()
        })?;
    let adapter_info = adapter.get_info();
    let (device, queue) = pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        label: Some("Pax Windows desktop"),
        required_limits: adapter.limits(),
        ..Default::default()
    }))
    .map_err(|error| error.to_string())?;
    let error = Arc::new(Mutex::new(None));
    let errors = error.clone();
    device.on_uncaptured_error(Arc::new(move |error| {
        let mut slot = errors.lock().unwrap_or_else(|poison| poison.into_inner());
        if slot.is_none() {
            *slot = Some(error.to_string());
        }
    }));
    let context = GpuContext::from_wgpu(instance, adapter, device.clone(), queue.clone())
        .map_err(|error| error.to_string())?;
    Ok(WindowsGpu {
        context,
        device,
        queue,
        adapter: adapter_info,
        batches: std::array::from_fn(|_| Batch::default()),
        next_token: 1,
        published: 0,
        retired: 0,
        backpressure: 0,
        closing: false,
        error,
        byte_budget,
        allocator_diagnostics: (std::env::var("PAX_GPU_MEMORY_DIAGNOSTICS").as_deref() == Ok("1"))
            .then(|| RefCell::new(AllocatorDiagnostics::default())),
    })
}

fn pointer(value: &Value, field: &str) -> Result<usize, String> {
    let value = value
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing string handle {field}"))?;
    let address = value
        .parse::<usize>()
        .map_err(|_| format!("Invalid handle {field}"))?;
    if address == 0 {
        return Err(format!("Null handle {field}"));
    }
    Ok(address)
}

impl WindowsGpu {
    fn allocation_bytes(device: &wgpu::Device, width: u32, height: u32) -> Result<u64, String> {
        let desc = D3D12_RESOURCE_DESC {
            Dimension: D3D12_RESOURCE_DIMENSION_TEXTURE2D,
            Alignment: 0,
            Width: u64::from(width),
            Height: height,
            DepthOrArraySize: 1,
            MipLevels: 1,
            Format: DXGI_FORMAT_B8G8R8A8_UNORM,
            SampleDesc: DXGI_SAMPLE_DESC {
                Count: 1,
                Quality: 0,
            },
            Layout: D3D12_TEXTURE_LAYOUT_UNKNOWN,
            Flags: D3D12_RESOURCE_FLAG_NONE,
        };
        let hal = unsafe { device.as_hal::<Dx12>() }.ok_or("Device is not DX12")?;
        let bytes = unsafe { hal.raw_device().GetResourceAllocationInfo(0, &[desc]) }.SizeInBytes;
        if bytes == 0 || bytes == u64::MAX {
            return Err("DX12 cannot allocate this export shape".into());
        }
        Ok(bytes)
    }
    fn dimensions(&self, surfaces: &[DesktopSurfaceInfo]) -> Result<u64, String> {
        if surfaces.len() > IMAGE_CAPACITY {
            return Err("Windows publication exceeds 64 images".into());
        }
        let limit = self.device.limits().max_texture_dimension_2d;
        let mut bytes = 0u64;
        for surface in surfaces {
            if surface.width == 0
                || surface.height == 0
                || surface.width > limit
                || surface.height > limit
            {
                return Err("Invalid Windows export dimensions".into());
            }
            bytes = bytes
                .checked_add(Self::allocation_bytes(
                    &self.device,
                    surface.width,
                    surface.height,
                )?)
                .ok_or("Windows export size overflow")?;
        }
        if bytes > self.byte_budget {
            return Err(format!(
                "Windows frame needs {bytes} export bytes but its budget is {}",
                self.byte_budget
            ));
        }
        Ok(bytes)
    }

    fn handles(&self) -> Result<(String, String), String> {
        // Never overlap HAL guards: WGPU assigns different lock ranks to its
        // device and texture guards. Rust owns the borrowed COM handles.
        let hal = unsafe { self.device.as_hal::<Dx12>() }.ok_or("Device is not DX12")?;
        Ok((
            (hal.raw_device().as_raw() as usize).to_string(),
            (hal.raw_queue().as_raw() as usize).to_string(),
        ))
    }

    fn check_error(&self) -> Result<(), String> {
        match &*self
            .error
            .lock()
            .unwrap_or_else(|poison| poison.into_inner())
        {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
}

impl DesktopGpuBackend for WindowsGpu {
    fn context(&self) -> SharedGpuContext {
        self.context.clone()
    }
    fn format(&self) -> wgpu::TextureFormat {
        wgpu::TextureFormat::Bgra8Unorm
    }

    fn poll(&mut self) -> Result<(), String> {
        self.device
            .poll(wgpu::PollType::Poll)
            .map_err(|error| error.to_string())?;
        self.check_error()?;
        for batch in &mut self.batches {
            let Some(producer) = &batch.producer_done else {
                continue;
            };
            if !producer.load(Ordering::Acquire) {
                continue;
            }
            let mut ready = true;
            for image in &batch.images {
                let Some(completion) = &image.completion else {
                    ready = false;
                    continue;
                };
                let completed = unsafe { completion.fence.GetCompletedValue() };
                if completed == u64::MAX {
                    return Err("DX12 consumer device was removed".into());
                }
                ready &= completed >= completion.value;
            }
            if ready {
                self.retired += batch.images.len() as u64;
                for image in &mut batch.images {
                    image.token = 0;
                    image.completion = None;
                }
                batch.producer_done = None;
            }
        }
        Ok(())
    }

    fn can_publish(&mut self, surfaces: &[DesktopSurfaceInfo]) -> Result<bool, String> {
        self.poll()?;
        let required = self.dimensions(surfaces)?;
        let pending_bytes: u64 = self
            .batches
            .iter()
            .filter(|batch| !batch.available())
            .flat_map(|batch| &batch.images)
            .map(|image| image.allocation_bytes)
            .sum();
        let available = !self.closing
            && self.batches.iter().any(Batch::available)
            && required <= self.byte_budget - pending_bytes;
        if !available {
            self.backpressure += 1;
        }
        Ok(available)
    }

    fn publish(&mut self, surfaces: Vec<DesktopSurfaceFrame>) -> Result<Value, String> {
        let infos: Vec<_> = surfaces.iter().map(|frame| frame.info.clone()).collect();
        if !self.can_publish(&infos)? {
            return Err("Windows export pool is saturated or closing".into());
        }
        if surfaces.is_empty() {
            return Ok(json!({"transport": TRANSPORT, "images": []}));
        }
        let count = surfaces.len() as u64;
        if self
            .next_token
            .checked_add(count)
            .is_none_or(|end| end > MAX_JSON_INTEGER)
        {
            return Err("Windows publication token space exhausted".into());
        }
        let (device_handle, queue_handle) = self.handles()?;
        let required = self.dimensions(&infos)?;
        let allocated: u64 = self
            .batches
            .iter()
            .flat_map(|batch| &batch.images)
            .map(|image| image.allocation_bytes)
            .sum();
        let selected_bytes: u64 = self
            .batches
            .iter()
            .find(|batch| batch.available())
            .unwrap()
            .images
            .iter()
            .map(|image| image.allocation_bytes)
            .sum();
        if allocated - selected_bytes + required > self.byte_budget {
            // Only completed, unowned caches are evictable. Live generations
            // retain their allocation budget until both sides have finished.
            for batch in self.batches.iter_mut().filter(|batch| batch.available()) {
                batch.images.clear();
            }
        }
        let batch = self
            .batches
            .iter_mut()
            .find(|batch| batch.available())
            .unwrap();
        // A free batch has no consumer or GPU ownership. Drop incompatible
        // allocations before replacing them, keeping resize peaks bounded.
        let compatible = batch.images.len() == surfaces.len()
            && batch.images.iter().zip(&surfaces).all(|(image, frame)| {
                image.info.width == frame.info.width && image.info.height == frame.info.height
            });
        if !compatible {
            batch.images.clear();
            for frame in &surfaces {
                batch.images.push(Image {
                    texture: self.device.create_texture(&wgpu::TextureDescriptor {
                        label: Some("Pax reusable Windows composition export"),
                        size: wgpu::Extent3d {
                            width: frame.info.width,
                            height: frame.info.height,
                            depth_or_array_layers: 1,
                        },
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format: wgpu::TextureFormat::Bgra8Unorm,
                        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::COPY_SRC,
                        view_formats: &[],
                    }),
                    info: frame.info.clone(),
                    token: 0,
                    completion: None,
                    allocation_bytes: Self::allocation_bytes(
                        &self.device,
                        frame.info.width,
                        frame.info.height,
                    )?,
                });
            }
        }
        let mut images = Vec::with_capacity(surfaces.len());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Pax Windows export batch"),
            });
        for (image, frame) in batch.images.iter_mut().zip(surfaces) {
            image.info = frame.info;
            image.token = self.next_token;
            self.next_token += 1;
            let resource = {
                let hal = unsafe { image.texture.as_hal::<Dx12>() }
                    .ok_or("Export texture is not DX12")?;
                (unsafe { hal.raw_resource() }.as_raw() as usize).to_string()
            };
            images.push(json!({"token": image.token, "resource": resource, "surface": image.info}));
            encoder.copy_texture_to_texture(
                frame.texture.as_image_copy(),
                image.texture.as_image_copy(),
                wgpu::Extent3d {
                    width: image.info.width,
                    height: image.info.height,
                    depth_or_array_layers: 1,
                },
            );
        }
        encoder.transition_resources(
            std::iter::empty(),
            batch.images.iter().map(|image| wgpu::TextureTransition {
                texture: &image.texture,
                selector: None,
                state: wgpu::TextureUses::COPY_SRC,
            }),
        );
        let producer_done = Arc::new(AtomicBool::new(false));
        // Ownership becomes pending before submission. A validation error or
        // device failure cannot accidentally make submitted allocations reusable.
        batch.producer_done = Some(producer_done.clone());
        self.queue.submit([encoder.finish()]);
        self.queue
            .on_submitted_work_done(move || producer_done.store(true, Ordering::Release));
        self.published += count;
        Ok(
            json!({"transport": TRANSPORT, "device": device_handle, "queue": queue_handle, "images": images}),
        )
    }

    fn release(&mut self, token: u64, completion: Value) -> Result<(), String> {
        if token == 0 {
            return Err("Invalid Windows publication token".into());
        }
        let image = self
            .batches
            .iter_mut()
            .filter(|batch| !batch.available())
            .flat_map(|batch| batch.images.iter_mut())
            .find(|image| image.token == token)
            .ok_or("Unknown or stale Windows publication token")?;
        if image.completion.is_some() {
            return Err("Windows publication token already released".into());
        }
        if completion.get("transport").and_then(Value::as_str) != Some(TRANSPORT) {
            return Err("Incompatible Windows completion transport".into());
        }
        let value = completion
            .get("value")
            .and_then(Value::as_u64)
            .filter(|value| *value > 0 && *value <= MAX_JSON_INTEGER)
            .ok_or("Invalid DX12 completion value")?;
        let raw = pointer(&completion, "fence")? as *mut c_void;
        // The host calls release while it owns a live ID3D12Fence. Retain a COM
        // reference before returning; this process-local ABI is not untrusted IPC.
        let fence = unsafe { ID3D12Fence::from_raw_borrowed(&raw) }
            .ok_or("Null completion fence")?
            .clone();
        let mut fence_device: Option<ID3D12Device> = None;
        unsafe { fence.GetDevice(&mut fence_device) }.map_err(|error| error.to_string())?;
        let hal = unsafe { self.device.as_hal::<Dx12>() }.ok_or("Device is not DX12")?;
        if fence_device.as_ref() != Some(hal.raw_device()) {
            return Err("Consumer completion fence belongs to another device".into());
        }
        image.completion = Some(Completion { fence, value });
        Ok(())
    }

    fn shutdown_poll(&mut self) -> Result<bool, String> {
        self.closing = true;
        self.poll()?;
        let drained = self.batches.iter().all(Batch::available);
        if drained {
            for batch in &mut self.batches {
                batch.images.clear();
            }
        }
        Ok(drained)
    }

    fn diagnostics(&self) -> Value {
        let handles = self.handles().ok();
        let mut result = json!({"transport": TRANSPORT, "adapter": self.adapter.name,
            "deviceType": format!("{:?}", self.adapter.device_type),
            "device": handles.as_ref().map(|handles| &handles.0),
            "queue": handles.as_ref().map(|handles| &handles.1),
            "capacity": FRAME_CAPACITY, "imageCapacity": IMAGE_CAPACITY,
            "exportByteBudget": self.byte_budget,
            "pendingFrames": self.batches.iter().filter(|batch| !batch.available()).count(),
            "allocatedImages": self.batches.iter().map(|batch| batch.images.len()).sum::<usize>(),
            "allocatedBytes": self.batches.iter().flat_map(|batch| &batch.images)
                .map(|image| image.allocation_bytes).sum::<u64>(),
            "pendingBytes": self.batches.iter().filter(|batch| !batch.available())
                .flat_map(|batch| &batch.images).map(|image| image.allocation_bytes).sum::<u64>(),
            "published": self.published, "retired": self.retired, "backpressure": self.backpressure,
            "closing": self.closing, "cpuPresentationReadback": false});
        if let Some(diagnostics) = &self.allocator_diagnostics {
            result["allocatorReport"] = diagnostics
                .borrow_mut()
                .snapshot(Instant::now(), || self.device.generate_allocator_report());
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pax_gpu::render_backend::{RenderBackend, RenderConfig};
    use pax_gpu::{point, Color, Fill, Path, WgpuRenderer};
    use std::time::{Duration, Instant};

    #[test]
    fn allocator_diagnostics_preserve_totals_with_bounded_labels() {
        let allocations: Vec<_> = (0..20)
            .flat_map(|i| {
                (0..2).map(move |_| wgpu::wgt::AllocationReport {
                    name: format!("{i:02}\"{}", "é".repeat(170)),
                    offset: 0,
                    size: i + 1,
                })
            })
            .collect();
        let result = allocator_summary(Some(wgpu::AllocatorReport {
            total_allocated_bytes: allocations.iter().map(|allocation| allocation.size).sum(),
            total_reserved_bytes: 3072,
            blocks: vec![
                wgpu::wgt::MemoryBlockReport {
                    size: 1024,
                    allocations: 0..40,
                },
                wgpu::wgt::MemoryBlockReport {
                    size: 2048,
                    allocations: 40..40,
                },
            ],
            allocations,
        }));
        assert_eq!(result["liveAllocatedBytes"], 420);
        assert_eq!(result["reservedBytes"], 3072);
        assert_eq!(result["unusedReservedBytes"], 2652);
        assert_eq!(result["emptyBlockBytes"], 2048);
        assert_eq!(result["allocationCount"], 40);
        let labels = result["labels"].as_array().unwrap();
        assert_eq!(labels.len(), 16);
        assert_eq!(labels[0]["allocatedBytes"], 40);
        assert!(labels.iter().all(|label| label["allocationCount"] == 2
            && label["labelTruncated"] == true
            && label["label"].as_str().unwrap().chars().count() == ALLOCATOR_LABEL_CHARS));
        assert_eq!(result["omittedLabelCount"], 4);
        assert_eq!(result["omittedAllocationCount"], 8);
        assert_eq!(result["omittedLabelBytes"], 20);
        assert_eq!(
            labels
                .iter()
                .map(|label| label["allocatedBytes"].as_u64().unwrap())
                .sum::<u64>()
                + result["omittedLabelBytes"].as_u64().unwrap(),
            420
        );
    }

    #[test]
    fn allocator_diagnostics_cache_unsupported_and_refresh_after_interval() {
        let mut diagnostics = AllocatorDiagnostics::default();
        let now = Instant::now();
        let first = diagnostics.snapshot(now, || None);
        assert_eq!(first["supported"], false);
        assert!(first.get("liveAllocatedBytes").is_none());
        let cached = diagnostics.snapshot(now + Duration::from_millis(500), || {
            panic!("cached report must not walk allocator")
        });
        assert_eq!(cached["sampleUnixMs"], first["sampleUnixMs"]);
        assert_eq!(cached["supported"], false);
        assert_eq!(cached["sampleAgeMs"], 500.0);
        let refreshed = diagnostics.snapshot(now + ALLOCATOR_SAMPLE_INTERVAL, || {
            Some(wgpu::AllocatorReport {
                allocations: Vec::new(),
                blocks: Vec::new(),
                total_allocated_bytes: 0,
                total_reserved_bytes: 0,
            })
        });
        assert_eq!(refreshed["supported"], true);
        assert_eq!(refreshed["liveAllocatedBytes"], 0);
        assert_eq!(refreshed["blockCount"], 0);
    }

    #[test]
    fn allocator_diagnostics_blocks_reconcile_bounded_labels_and_omissions() {
        let mut allocations = Vec::new();
        let mut blocks = Vec::new();
        for block in 0..18 {
            let start = allocations.len();
            for label in 0..10 {
                allocations.push(wgpu::wgt::AllocationReport {
                    name: format!("block-{block}-label-{label}"),
                    offset: label * 64,
                    size: 64,
                });
            }
            blocks.push(wgpu::wgt::MemoryBlockReport {
                size: 1024,
                allocations: start..allocations.len(),
            });
        }
        let value = allocator_block_summary(&wgpu::AllocatorReport {
            allocations,
            blocks,
            total_allocated_bytes: 18 * 640,
            total_reserved_bytes: 18 * 1024,
        });
        assert_eq!(value["consistent"], true);
        assert_eq!(value["reportedBlockCount"], 18);
        assert_eq!(value["displayedBlockCount"], 16);
        assert_eq!(value["omittedBlockCount"], 2);
        assert_eq!(value["omittedBlockReservedBytes"], 2048);
        assert_eq!(value["omittedBlockLiveAllocatedBytes"], 1280);
        for (index, block) in value["blocks"].as_array().unwrap().iter().enumerate() {
            assert_eq!(block["snapshotIndex"], index);
            assert_eq!(block["liveAllocatedBytes"], 640);
            assert_eq!(block["unusedReservedBytes"], 384);
            assert_eq!(block["labels"].as_array().unwrap().len(), 8);
            assert_eq!(block["omittedLabelCount"], 2);
            assert_eq!(block["omittedLabelBytes"], 128);
            assert_eq!(block["omittedAllocationCount"], 2);
            assert_eq!(
                block["labels"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v["allocatedBytes"].as_u64().unwrap())
                    .sum::<u64>()
                    + block["omittedLabelBytes"].as_u64().unwrap(),
                640
            );
        }
    }

    #[test]
    fn allocator_diagnostics_blocks_reject_malformed_ranges_and_layouts() {
        let allocation = |offset, size| wgpu::wgt::AllocationReport {
            name: "test".into(),
            offset,
            size,
        };
        let block = |size, allocations| wgpu::wgt::MemoryBlockReport { size, allocations };
        let value = allocator_block_summary(&wgpu::AllocatorReport {
            allocations: vec![allocation(0, 8), allocation(4, 8), allocation(u64::MAX, 1)],
            blocks: vec![
                block(16, 0..2),
                block(16, 1..3),
                block(16, 4..3),
                block(16, 3..5),
            ],
            total_allocated_bytes: 17,
            total_reserved_bytes: 64,
        });
        assert_eq!(value["consistent"], false);
        assert_eq!(value["consistency"]["allocationRangesValid"], false);
        assert_eq!(value["consistency"]["allocationRangesDisjoint"], false);
        assert_eq!(value["consistency"]["allocationLayoutsValid"], false);
        assert_eq!(value["blocks"][0]["allocationOffsetsDisjoint"], false);
        assert_eq!(value["blocks"][1]["rangeOverlapsEarlierBlock"], true);
        assert_eq!(value["blocks"][1]["allocationOffsetsValid"], false);
        assert_eq!(value["blocks"][2]["allocationRangeValid"], false);
        assert!(value["blocks"][2]["liveAllocatedBytes"].is_null());
        assert!(value["blocks"][3]["unusedReservedBytes"].is_null());
    }

    #[test]
    fn allocator_diagnostics_blocks_preserve_unicode_labels_and_dedicated_layout() {
        let name = format!("quoted\"{}", "é".repeat(170));
        let value = allocator_block_summary(&wgpu::AllocatorReport {
            allocations: vec![wgpu::wgt::AllocationReport {
                name: name.clone(),
                offset: 0,
                size: 1024,
            }],
            blocks: vec![wgpu::wgt::MemoryBlockReport {
                size: 1024,
                allocations: 0..1,
            }],
            total_allocated_bytes: 1024,
            total_reserved_bytes: 1024,
        });
        assert_eq!(value["consistent"], true);
        let block = &value["blocks"][0];
        assert_eq!(block["unusedReservedBytes"], 0);
        assert_eq!(block["labels"][0]["labelTruncated"], true);
        assert_eq!(
            block["labels"][0]["label"],
            name.chars().take(160).collect::<String>()
        );
        assert_eq!(
            serde_json::from_str::<Value>(&value.to_string()).unwrap(),
            value
        );
    }

    #[test]
    fn allocator_diagnostics_blocks_mark_overflow_unknown_instead_of_zero() {
        let value = allocator_block_summary(&wgpu::AllocatorReport {
            allocations: vec![
                wgpu::wgt::AllocationReport {
                    name: "huge".into(),
                    offset: 0,
                    size: u64::MAX,
                },
                wgpu::wgt::AllocationReport {
                    name: "extra".into(),
                    offset: 0,
                    size: 1,
                },
            ],
            blocks: vec![
                wgpu::wgt::MemoryBlockReport {
                    size: u64::MAX,
                    allocations: 0..2,
                },
                wgpu::wgt::MemoryBlockReport {
                    size: 1,
                    allocations: 2..2,
                },
            ],
            total_allocated_bytes: 0,
            total_reserved_bytes: 0,
        });
        assert_eq!(value["consistent"], false);
        for field in [
            "allocationByteSumValid",
            "blockLiveByteSumValid",
            "blockReservedByteSumValid",
            "allocationBytesMatchReportedTotal",
            "blockLiveBytesMatchReportedTotal",
            "blockReservedBytesMatchReportedTotal",
        ] {
            assert_eq!(value["consistency"][field], false, "{field}");
        }
        let block = &value["blocks"][0];
        assert!(block["liveAllocatedBytes"].is_null());
        assert!(block["unusedReservedBytes"].is_null());
        assert_eq!(block["labelsValid"], false);
        assert!(block["omittedLabelBytes"].is_null());
        assert!(block["omittedAllocationCount"].is_null());
        assert_eq!(value["blocks"][1]["liveAllocatedBytes"], 0);
        assert_eq!(value["blocks"][1]["unusedReservedBytes"], 1);
    }

    #[test]
    fn allocator_diagnostics_blocks_report_empty_and_unassigned_allocations() {
        let mut report = wgpu::AllocatorReport {
            allocations: Vec::new(),
            blocks: vec![wgpu::wgt::MemoryBlockReport {
                size: 1024,
                allocations: 0..0,
            }],
            total_allocated_bytes: 0,
            total_reserved_bytes: 1024,
        };
        let value = allocator_block_summary(&report);
        assert_eq!(value["consistent"], true);
        assert_eq!(value["blocks"][0]["liveAllocatedBytes"], 0);
        assert_eq!(value["blocks"][0]["unusedReservedBytes"], 1024);
        report.allocations.push(wgpu::wgt::AllocationReport {
            name: "unassigned".into(),
            offset: 0,
            size: 16,
        });
        report.total_allocated_bytes = 16;
        report.total_reserved_bytes = 2048;
        let value = allocator_block_summary(&report);
        assert_eq!(value["consistent"], false);
        assert_eq!(value["consistency"]["allAllocationsAssigned"], false);
        assert_eq!(
            value["consistency"]["blockLiveBytesMatchReportedTotal"],
            false
        );
        assert_eq!(
            value["consistency"]["blockReservedBytesMatchReportedTotal"],
            false
        );
    }

    fn surface(width: u32, height: u32, generation: &str) -> DesktopSurfaceInfo {
        DesktopSurfaceInfo {
            layer: 0,
            key: "single".into(),
            host_signature: generation.into(),
            origin_x: 0.0,
            origin_y: 0.0,
            logical_width: width as f32,
            logical_height: height as f32,
            width,
            height,
            dpr: [1.0, 1.0],
            replay_priority: 0,
        }
    }
    fn render(backend: &dyn DesktopGpuBackend, info: &DesktopSurfaceInfo) -> DesktopSurfaceFrame {
        let target = RenderBackend::to_texture(
            backend.context(),
            backend.format(),
            RenderConfig::new(false, info.width, info.height, [1.0, 1.0]),
        )
        .unwrap();
        let mut renderer = WgpuRenderer::new(target);
        let mut path = Path::builder();
        path.begin(point(0.0, 0.0));
        path.line_to(point(info.width as f32, 0.0));
        path.line_to(point(info.width as f32, info.height as f32));
        path.line_to(point(0.0, info.height as f32));
        path.end(true);
        renderer.begin_node(1, 1, 0);
        renderer.fill_path(path.build(), Fill::Solid(Color::rgba(0.2, 0.7, 0.4, 1.0)));
        renderer.end_node(1);
        renderer.flush();
        DesktopSurfaceFrame {
            info: info.clone(),
            texture: renderer.output_texture().unwrap().clone(),
        }
    }
    fn pump(backend: &mut dyn DesktopGpuBackend, ready: impl Fn(&Value) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            backend.poll().unwrap();
            if ready(&backend.diagnostics()) {
                break;
            }
            assert!(
                Instant::now() < deadline,
                "GPU retirement deadline: {}",
                backend.diagnostics()
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    #[test]
    fn held_exports_backpressure_resize_and_close_without_early_reuse() {
        if std::env::var("PAX_WINDOWS_GPU_TESTS").as_deref() != Ok("1") {
            eprintln!("Not qualified: set PAX_WINDOWS_GPU_TESTS=1 on a DX12 hardware machine");
            return;
        }
        // Deliberately small allocations; the controlled CPU-signalled fence
        // stands for a delayed native consumer, not a production queue fence.
        let mut backend = create_backend().expect("DX12 hardware backend");
        println!(
            "windows_export_test stage=initialized diagnostics={}",
            backend.diagnostics()
        );
        let raw = pointer(&backend.diagnostics(), "device").unwrap() as *mut c_void;
        let device = unsafe { ID3D12Device::from_raw_borrowed(&raw) }
            .unwrap()
            .clone();
        let fence: ID3D12Fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }.unwrap();
        let completion = |value| {
            json!({"transport": TRANSPORT,
            "fence": (fence.as_raw() as usize).to_string(), "value": value})
        };
        let info = surface(81, 49, "first");
        let mut tokens = Vec::new();
        for _ in 0..FRAME_CAPACITY {
            assert!(backend.can_publish(std::slice::from_ref(&info)).unwrap());
            let frame = render(backend.as_ref(), &info);
            let publication = backend.publish(vec![frame]).unwrap();
            tokens.push(publication["images"][0]["token"].as_u64().unwrap());
        }
        assert!(!backend.can_publish(std::slice::from_ref(&info)).unwrap());
        let resized = surface(113, 67, "resized");
        assert!(!backend.can_publish(std::slice::from_ref(&resized)).unwrap());
        assert!(backend.release(u64::MAX, completion(1)).is_err());
        backend.release(tokens[0], completion(1)).unwrap();
        assert!(backend.release(tokens[0], completion(1)).is_err());
        for _ in 0..8 {
            backend.poll().unwrap();
        }
        assert_eq!(backend.diagnostics()["pendingFrames"], FRAME_CAPACITY);
        println!(
            "windows_export_test stage=held diagnostics={}",
            backend.diagnostics()
        );
        unsafe { fence.Signal(1) }.unwrap();
        pump(backend.as_mut(), |state| {
            state["pendingFrames"] == FRAME_CAPACITY - 1
        });
        assert!(backend.release(tokens[0], completion(1)).is_err());
        let frame = render(backend.as_ref(), &resized);
        let publication = backend.publish(vec![frame]).unwrap();
        let replacement = publication["images"][0]["token"].as_u64().unwrap();
        assert!(replacement > *tokens.last().unwrap());
        assert!(!backend.shutdown_poll().unwrap());
        assert!(!backend.can_publish(std::slice::from_ref(&info)).unwrap());
        for token in [tokens[1], tokens[2], replacement] {
            backend.release(token, completion(2)).unwrap();
        }
        assert!(!backend.shutdown_poll().unwrap());
        unsafe { fence.Signal(2) }.unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !backend.shutdown_poll().unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(backend.diagnostics()["allocatedImages"], 0);
        assert_eq!(backend.diagnostics()["retired"], 4);
        assert_eq!(backend.diagnostics()["published"], 4);
        println!(
            "windows_export_test stage=closed diagnostics={}",
            backend.diagnostics()
        );
    }

    #[test]
    fn export_byte_budget_includes_held_resize_generations() {
        if std::env::var("PAX_WINDOWS_GPU_TESTS").as_deref() != Ok("1") {
            eprintln!("Not qualified: set PAX_WINDOWS_GPU_TESTS=1 on a DX12 hardware machine");
            return;
        }
        let mut backend =
            create_backend_with_budget(DEFAULT_EXPORT_BUDGET).expect("DX12 hardware backend");
        let first = surface(81, 49, "small");
        let resized = surface(1024, 257, "larger");
        let first_bytes =
            WindowsGpu::allocation_bytes(&backend.device, first.width, first.height).unwrap();
        let resized_bytes =
            WindowsGpu::allocation_bytes(&backend.device, resized.width, resized.height).unwrap();
        assert!(
            resized_bytes > first_bytes * 2,
            "Test requires distinct allocation sizes"
        );
        backend.byte_budget = resized_bytes;
        let raw = pointer(&backend.diagnostics(), "device").unwrap() as *mut c_void;
        let device = unsafe { ID3D12Device::from_raw_borrowed(&raw) }
            .unwrap()
            .clone();
        let fence: ID3D12Fence = unsafe { device.CreateFence(0, D3D12_FENCE_FLAG_NONE) }.unwrap();
        let completion = |value| {
            json!({"transport": TRANSPORT,
            "fence": (fence.as_raw() as usize).to_string(), "value": value})
        };
        let first_frame = render(&backend, &first);
        let publication = backend.publish(vec![first_frame]).unwrap();
        let first_token = publication["images"][0]["token"].as_u64().unwrap();
        assert_eq!(backend.diagnostics()["pendingFrames"], 1);
        // Two frame slots remain free: only the global byte budget can block
        // this resize. Changing the generation cannot discard consumer-owned data.
        for generation in ["larger-a", "larger-b", "larger-c"] {
            let mut candidate = resized.clone();
            candidate.host_signature = generation.into();
            assert!(!backend.can_publish(&[candidate]).unwrap());
            assert_eq!(backend.diagnostics()["allocatedBytes"], first_bytes);
        }
        let oversized = surface(2048, 2048, "cannot-fit");
        let error = backend.can_publish(&[oversized]).unwrap_err();
        assert!(
            error.contains("budget"),
            "Unexpected oversize rejection: {error}"
        );
        backend.release(first_token, completion(1)).unwrap();
        assert!(!backend.can_publish(std::slice::from_ref(&resized)).unwrap());
        println!(
            "windows_export_budget_test stage=held diagnostics={}",
            backend.diagnostics()
        );
        unsafe { fence.Signal(1) }.unwrap();
        pump(&mut backend, |state| state["pendingFrames"] == 0);
        assert!(backend.can_publish(std::slice::from_ref(&resized)).unwrap());
        let resized_frame = render(&backend, &resized);
        let publication = backend.publish(vec![resized_frame]).unwrap();
        let resized_token = publication["images"][0]["token"].as_u64().unwrap();
        assert_eq!(backend.diagnostics()["allocatedBytes"], resized_bytes);
        assert!(!backend.can_publish(std::slice::from_ref(&first)).unwrap());
        assert!(!backend.shutdown_poll().unwrap());
        backend.release(resized_token, completion(2)).unwrap();
        assert!(!backend.shutdown_poll().unwrap());
        unsafe { fence.Signal(2) }.unwrap();
        let deadline = Instant::now() + Duration::from_secs(10);
        while !backend.shutdown_poll().unwrap() {
            assert!(Instant::now() < deadline);
            std::thread::sleep(Duration::from_millis(2));
        }
        assert_eq!(backend.diagnostics()["allocatedBytes"], 0);
        assert_eq!(backend.diagnostics()["published"], 2);
        assert_eq!(backend.diagnostics()["retired"], 2);
        println!(
            "windows_export_budget_test stage=closed diagnostics={}",
            backend.diagnostics()
        );
    }
}

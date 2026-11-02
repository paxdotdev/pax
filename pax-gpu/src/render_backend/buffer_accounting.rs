use wgpu::{Backend, BufferUsages};

const DX12_BUFFER_ALIGNMENT: u64 = 64 * 1024;

// This is a retention-budget charge, not heap reservation, residency, or process memory.
// Other backends retain their existing payload accounting until separately qualified.
pub(super) fn retained_buffer_cache_bytes(
    backend: Backend,
    upload_bytes: u64,
    buffers: impl IntoIterator<Item = (u64, BufferUsages)>,
) -> u64 {
    if backend != Backend::Dx12 {
        return upload_bytes;
    }
    buffers.into_iter().fold(0, |total, (size, usage)| {
        total.saturating_add(dx12_buffer_allocation_bytes(size, usage))
    })
}

fn dx12_buffer_allocation_bytes(size: u64, usage: BufferUsages) -> u64 {
    // Match wgpu-core 28.0.1 device/resource.rs and wgpu-hal 28.0.1
    // dx12/device.rs: Buffer::size() excludes the core's extra vertex byte and
    // HAL uniform padding. The utility's upload padding is already in size().
    let core_size = if size == 0 {
        wgpu::COPY_BUFFER_ALIGNMENT
    } else if usage.contains(BufferUsages::VERTEX) {
        size.saturating_add(1)
    } else {
        size
    };
    let mut width = align_up(core_size, wgpu::COPY_BUFFER_ALIGNMENT);
    if usage.contains(BufferUsages::UNIFORM) {
        width = align_up(width, 256);
    }
    // Pinned HAL dx12/conv.rs uses ordinary ROW_MAJOR buffers with Alignment=0
    // and no USE_TIGHT_ALIGNMENT flag. D3D12 specifies round_up(Width, 64 KiB)
    // for these allocation requirements, including unordered-access buffers:
    // https://learn.microsoft.com/en-us/windows/win32/api/d3d12/nf-d3d12-id3d12device-getresourceallocationinfo(uint_uint_constd3d12_resource_desc)
    // Re-audit the padding and flags when upgrading wgpu; reserved allocator
    // blocks, staging buffers, and other resources are not charged here.
    align_up(width, DX12_BUFFER_ALIGNMENT)
}

fn align_up(size: u64, alignment: u64) -> u64 {
    size.checked_add(alignment - 1)
        .map(|size| size & !(alignment - 1))
        .unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_and_small_buffers_still_have_an_allocation() {
        for usage in [
            BufferUsages::VERTEX,
            BufferUsages::INDEX,
            BufferUsages::UNIFORM,
            BufferUsages::STORAGE,
        ] {
            for size in [0, 1, 4, 255, 256, 257] {
                assert_eq!(dx12_buffer_allocation_bytes(size, usage), 65_536);
            }
        }
    }

    #[test]
    fn vertex_padding_crosses_the_allocation_boundary() {
        for (size, expected) in [
            (65_532, 65_536),
            (65_535, 65_536),
            (65_536, 131_072),
            (65_537, 131_072),
            (131_072, 196_608),
        ] {
            assert_eq!(
                dx12_buffer_allocation_bytes(size, BufferUsages::VERTEX),
                expected
            );
        }
    }

    #[test]
    fn uniform_and_other_roles_round_at_the_actual_boundary() {
        for usage in [
            BufferUsages::INDEX,
            BufferUsages::UNIFORM,
            BufferUsages::STORAGE,
        ] {
            for (size, expected) in [(65_535, 65_536), (65_536, 65_536), (65_537, 131_072)] {
                assert_eq!(dx12_buffer_allocation_bytes(size, usage), expected);
            }
        }
        assert_eq!(
            dx12_buffer_allocation_bytes(
                65_536,
                BufferUsages::VERTEX | BufferUsages::UNIFORM | BufferUsages::COPY_DST,
            ),
            131_072
        );
    }

    #[test]
    fn six_small_allocations_charge_separately_from_upload_bytes() {
        let usages = [
            BufferUsages::VERTEX,
            BufferUsages::INDEX,
            BufferUsages::UNIFORM,
            BufferUsages::UNIFORM,
            BufferUsages::STORAGE,
            BufferUsages::UNIFORM,
        ];
        let upload_bytes = 24;
        assert_eq!(
            retained_buffer_cache_bytes(
                Backend::Dx12,
                upload_bytes,
                usages.map(|usage| (4, usage | BufferUsages::COPY_DST)),
            ),
            393_216
        );
    }

    #[test]
    fn every_other_backend_preserves_payload_charge_exactly() {
        for backend in [
            Backend::Noop,
            Backend::Vulkan,
            Backend::Metal,
            Backend::Gl,
            Backend::BrowserWebGpu,
        ] {
            for upload_bytes in [0, 3, 65_536, u64::MAX] {
                assert_eq!(
                    retained_buffer_cache_bytes(
                        backend,
                        upload_bytes,
                        std::iter::from_fn(|| {
                            panic!("non-DX12 accounting must not inspect buffer descriptors")
                        }),
                    ),
                    upload_bytes
                );
            }
        }
    }

    #[test]
    fn size_and_sum_overflow_cannot_turn_into_a_small_charge() {
        for usage in [
            BufferUsages::VERTEX,
            BufferUsages::UNIFORM,
            BufferUsages::INDEX,
        ] {
            assert_eq!(dx12_buffer_allocation_bytes(u64::MAX, usage), u64::MAX);
            assert_eq!(dx12_buffer_allocation_bytes(u64::MAX - 1, usage), u64::MAX);
        }
        assert_eq!(
            retained_buffer_cache_bytes(Backend::Dx12, 0, [(u64::MAX / 2, BufferUsages::INDEX); 2],),
            u64::MAX
        );
    }
}

#[cfg(all(test, target_os = "windows"))]
mod hardware_tests {
    use super::*;
    use std::time::Duration;
    use wgpu::util::DeviceExt;

    // Run explicitly with PAX_WINDOWS_GPU_TESTS=1 and this ignored test's exact
    // filter on DX12 hardware. Unsupported allocator reports fail qualification.
    #[test]
    #[ignore = "requires DX12 hardware and PAX_WINDOWS_GPU_TESTS=1"]
    fn dx12_retained_buffer_charge_matches_allocator_report() {
        assert_eq!(std::env::var("PAX_WINDOWS_GPU_TESTS").as_deref(), Ok("1"));
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor {
            backends: wgpu::Backends::DX12,
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let info = adapter.get_info();
        assert_eq!(info.backend, Backend::Dx12);
        assert!(matches!(
            info.device_type,
            wgpu::DeviceType::DiscreteGpu | wgpu::DeviceType::IntegratedGpu
        ));
        println!("retained_buffer_allocation_adapter={info:?}");
        let (device, queue) =
            pollster::block_on(adapter.request_device(&Default::default())).unwrap();
        let usages = [
            BufferUsages::VERTEX,
            BufferUsages::INDEX,
            BufferUsages::UNIFORM,
            BufferUsages::UNIFORM,
            BufferUsages::STORAGE,
            BufferUsages::UNIFORM,
        ];
        let labels: Vec<_> = (0..6)
            .map(|index| format!("pax-retained-budget-contract-{index}"))
            .collect();
        for sizes in [[0; 6], [65_536, 0, 257, 65_536, 65_537, 4]] {
            let buffers: Vec<_> = sizes
                .into_iter()
                .enumerate()
                .map(|(index, size)| {
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some(&labels[index]),
                        contents: &vec![0; size],
                        usage: usages[index] | BufferUsages::COPY_DST,
                    })
                })
                .collect();
            let expected = retained_buffer_cache_bytes(
                info.backend,
                sizes.iter().sum::<usize>() as u64,
                buffers.iter().map(|buffer| (buffer.size(), buffer.usage())),
            );
            let submission = queue.submit([]);
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: Some(submission),
                    timeout: Some(Duration::from_secs(10)),
                })
                .unwrap();
            let report = device
                .generate_allocator_report()
                .expect("DX12 allocation report");
            let mut observed = 0;
            for (index, label) in labels.iter().enumerate() {
                let rows: Vec<_> = report
                    .allocations
                    .iter()
                    .filter(|row| &row.name == label)
                    .collect();
                assert_eq!(rows.len(), 1, "one live allocation for {label}");
                let buffer = &buffers[index];
                assert_eq!(
                    rows[0].size,
                    dx12_buffer_allocation_bytes(buffer.size(), buffer.usage()),
                    "allocation for {label}, descriptor bytes {}",
                    buffer.size(),
                );
                observed += rows[0].size;
            }
            assert_eq!(observed, expected);
            println!("retained_buffer_allocation sizes={sizes:?} expected={expected} observed={observed}");
            drop(buffers);
            device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: Some(Duration::from_secs(10)),
                })
                .unwrap();
            let retired = device
                .generate_allocator_report()
                .expect("DX12 retirement report");
            assert!(retired
                .allocations
                .iter()
                .all(|row| !labels.contains(&row.name)));
        }
    }
}

//! Ordered buffer uploads. Destination versions belong to the command stream,
//! even when several versions are encoded before one queue submission.

pub(crate) struct OrderedUploads {
    transport: Transport,
}

enum Transport {
    #[cfg(not(target_arch = "wasm32"))]
    Mapped(wgpu::util::StagingBelt),
    #[cfg(any(target_arch = "wasm32", test))]
    Queued(QueueUploads),
}

impl OrderedUploads {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, chunk_size: u64) -> Self {
        #[cfg(target_arch = "wasm32")]
        return Self::queued(device, queue, chunk_size);
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = queue;
            Self {
                transport: Transport::Mapped(wgpu::util::StagingBelt::new(device, chunk_size)),
            }
        }
    }

    #[cfg(any(target_arch = "wasm32", test))]
    pub fn queued(device: wgpu::Device, queue: wgpu::Queue, chunk_size: u64) -> Self {
        Self {
            transport: Transport::Queued(QueueUploads {
                device,
                queue,
                chunk_size,
                active: Vec::new(),
                free: Vec::new(),
                first_dirty: 0,
            }),
        }
    }

    pub fn write(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        target: &wgpu::Buffer,
        bytes: &[u8],
    ) {
        if bytes.is_empty() {
            return;
        }
        assert_eq!(bytes.len() as u64 % wgpu::COPY_BUFFER_ALIGNMENT, 0);
        match &mut self.transport {
            #[cfg(not(target_arch = "wasm32"))]
            Transport::Mapped(belt) => {
                let size = wgpu::BufferSize::new(bytes.len() as u64).unwrap();
                belt.write_buffer(encoder, target, 0, size)
                    .copy_from_slice(bytes);
            }
            #[cfg(any(target_arch = "wasm32", test))]
            Transport::Queued(uploads) => uploads.write(encoder, target, bytes),
        }
    }

    /// Complete CPU uploads before finishing the command buffer that uses them.
    /// This does not release source ranges: other command buffers may still be pending.
    pub fn finish(&mut self) {
        match &mut self.transport {
            #[cfg(not(target_arch = "wasm32"))]
            Transport::Mapped(belt) => belt.finish(),
            #[cfg(any(target_arch = "wasm32", test))]
            Transport::Queued(uploads) => uploads.finish(),
        }
    }

    /// Called only after all encoded users of these source ranges were submitted.
    pub fn after_submit(&mut self) {
        match &mut self.transport {
            #[cfg(not(target_arch = "wasm32"))]
            Transport::Mapped(belt) => belt.recall(),
            #[cfg(any(target_arch = "wasm32", test))]
            Transport::Queued(uploads) => uploads.after_submit(),
        }
    }
}

#[cfg(any(target_arch = "wasm32", test))]
struct UploadPage {
    buffer: wgpu::Buffer,
    capacity: u64,
    bytes: Vec<u8>,
    uploaded: usize,
}

#[cfg(any(target_arch = "wasm32", test))]
struct QueueUploads {
    device: wgpu::Device,
    queue: wgpu::Queue,
    chunk_size: u64,
    active: Vec<UploadPage>,
    free: Vec<UploadPage>,
    first_dirty: usize,
}

#[cfg(any(target_arch = "wasm32", test))]
impl QueueUploads {
    fn write(&mut self, encoder: &mut wgpu::CommandEncoder, target: &wgpu::Buffer, bytes: &[u8]) {
        let size = bytes.len() as u64;
        if self
            .active
            .last()
            .is_none_or(|page| size > page.capacity - page.bytes.len() as u64)
        {
            let page = if let Some(index) = self.free.iter().position(|page| page.capacity >= size)
            {
                self.free.swap_remove(index)
            } else {
                let capacity = self.chunk_size.max(size);
                UploadPage {
                    buffer: self.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Ordered upload source"),
                        size: capacity,
                        usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    }),
                    capacity,
                    bytes: Vec::new(),
                    uploaded: 0,
                }
            };
            self.active.push(page);
        }
        self.first_dirty = self.first_dirty.min(self.active.len() - 1);
        let page = self.active.last_mut().unwrap();
        let offset = page.bytes.len() as u64;
        page.bytes.extend_from_slice(bytes);
        // Queue writes populate distinct source ranges, never the destination.
        // Copies remain interleaved with draws, preserving successive versions.
        encoder.copy_buffer_to_buffer(&page.buffer, offset, target, 0, size);
    }

    fn finish(&mut self) {
        // Safari rejects repeated copies encoded from a still-mapped source.
        // Unmapped COPY_DST sources avoid that path without a fresh chunk per write.
        for page in &mut self.active[self.first_dirty..] {
            if page.uploaded < page.bytes.len() {
                self.queue.write_buffer(
                    &page.buffer,
                    page.uploaded as u64,
                    &page.bytes[page.uploaded..],
                );
                page.uploaded = page.bytes.len();
            }
        }
        self.first_dirty = self.active.len();
    }

    fn after_submit(&mut self) {
        debug_assert!(self
            .active
            .iter()
            .all(|page| page.uploaded == page.bytes.len()));
        // Later queue writes execute after the preceding submission's copies;
        // CPU reuse needs no mapAsync round trip or GPU-completion wait.
        // Retain only the last submission's pages, not an all-time high-water cache.
        self.free.clear();
        for mut page in self.active.drain(..) {
            page.bytes.clear();
            page.uploaded = 0;
            self.free.push(page);
        }
        self.first_dirty = 0;
    }
}

#[cfg(all(test, target_os = "macos"))]
mod tests {
    use super::*;

    fn device() -> (wgpu::Device, wgpu::Queue) {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        pollster::block_on(adapter.request_device(&Default::default())).unwrap()
    }

    fn read(device: &wgpu::Device, buffer: &wgpu::Buffer) -> Vec<u8> {
        let (send, recv) = std::sync::mpsc::channel();
        buffer
            .slice(..)
            .map_async(wgpu::MapMode::Read, move |result| {
                send.send(result).unwrap()
            });
        device.poll(wgpu::PollType::wait_indefinitely()).unwrap();
        recv.recv().unwrap().unwrap();
        let result = buffer.slice(..).get_mapped_range().to_vec();
        buffer.unmap();
        result
    }

    #[test]
    #[ignore = "requires a native GPU adapter"]
    fn ordered_uploads_preserve_versions_across_encoders_and_reuse_after_submit() {
        let (device, queue) = device();
        let target = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 64,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        for queued in [false, true] {
            let mut uploads = if queued {
                OrderedUploads::queued(device.clone(), queue.clone(), 256)
            } else {
                OrderedUploads::new(device.clone(), queue.clone(), 256)
            };
            let mut results = Vec::new();
            // Do not wait for the GPU between submissions: queue ordering must
            // protect old copies when the next batch reuses an upload page.
            for submission in 0..8u8 {
                let result = device.create_buffer(&wgpu::BufferDescriptor {
                    label: None,
                    size: 256,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                let mut commands = Vec::new();
                for encoder_index in 0..2u8 {
                    let mut encoder = device.create_command_encoder(&Default::default());
                    for version in 0..2u8 {
                        let index = encoder_index * 2 + version;
                        uploads.write(&mut encoder, &target, &[submission * 4 + index; 64]);
                        encoder.copy_buffer_to_buffer(
                            &target,
                            0,
                            &result,
                            u64::from(index) * 64,
                            64,
                        );
                    }
                    uploads.finish();
                    commands.push(encoder.finish());
                }
                if let Transport::Queued(state) = &uploads.transport {
                    assert_eq!(state.active.len(), 1, "small writes share a single page");
                    assert_eq!(state.active[0].bytes.len(), 256);
                    assert_eq!(
                        state.active[0].buffer.usage(),
                        wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST
                    );
                }
                queue.submit(commands);
                uploads.after_submit();
                results.push(result);
            }
            for (submission, result) in results.iter().enumerate() {
                let bytes = read(&device, result);
                for (version, actual) in bytes.chunks_exact(64).enumerate() {
                    assert!(actual
                        .iter()
                        .all(|b| *b == (submission * 4 + version) as u8));
                }
            }
        }
    }

    #[test]
    #[ignore = "requires a native GPU adapter"]
    fn queued_upload_pages_retire_unused_peaks_and_abandon_unsubmitted_copies() {
        let (device, queue) = device();
        let target = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1024,
            usage: wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut uploads = OrderedUploads::queued(device.clone(), queue.clone(), 256);
        let mut encoder = device.create_command_encoder(&Default::default());
        uploads.write(&mut encoder, &target, &[3; 1024]);
        uploads.finish();
        // Resize/reset may discard pending copies. No stale source ranges may
        // leak into a replacement uploader's submission.
        drop(encoder);
        drop(uploads);
        uploads = OrderedUploads::queued(device.clone(), queue.clone(), 256);
        for writes in [8, 1, 1] {
            let mut encoder = device.create_command_encoder(&Default::default());
            uploads.write(&mut encoder, &target, &[]);
            for _ in 0..writes {
                uploads.write(&mut encoder, &target, &[7; 256]);
            }
            uploads.finish();
            queue.submit([encoder.finish()]);
            uploads.after_submit();
            let Transport::Queued(state) = &uploads.transport else {
                unreachable!()
            };
            assert_eq!(state.free.len(), writes);
            assert!(state.active.is_empty());
        }
        let result = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: 1024,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&target, 0, &result, 0, 1024);
        queue.submit([encoder.finish()]);
        let bytes = read(&device, &result);
        assert_eq!(&bytes[..256], &[7; 256]);
        assert_eq!(&bytes[256..], &[0; 768]);
    }
}

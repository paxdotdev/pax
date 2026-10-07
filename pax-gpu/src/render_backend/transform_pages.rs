use super::data::{GpuPrimitive, GpuTransform, GpuVertex};
use lyon::tessellation::VertexBuffers;
use std::ops::Range;
use wgpu::util::DeviceExt;

// Keep this in sync with geometry.wgsl and stencil.wgsl. Each binding is 15 KiB,
// independent of scene size, and requires no storage-buffer features.
pub(super) const TRANSFORMS_PER_PAGE: usize = 480;

pub(super) fn layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("Transform page layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: wgpu::BufferSize::new(
                    (TRANSFORMS_PER_PAGE * std::mem::size_of::<GpuTransform>()) as u64,
                ),
            },
            count: None,
        }],
    })
}

pub(super) fn bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    buffer: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("Transform page"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: buffer.as_entire_binding(),
        }],
    })
}

struct Page {
    buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    uploaded: Vec<GpuTransform>,
}

pub(super) struct TransformPages {
    layout: wgpu::BindGroupLayout,
    pages: Vec<Page>,
}

impl TransformPages {
    pub(super) fn new(layout: wgpu::BindGroupLayout) -> Self {
        Self {
            layout,
            pages: Vec::new(),
        }
    }

    pub(super) fn from_buffer(
        device: &wgpu::Device,
        layout: wgpu::BindGroupLayout,
        buffer: &wgpu::Buffer,
    ) -> Self {
        let group = bind_group(device, &layout, buffer);
        Self {
            layout,
            pages: vec![Page {
                buffer: buffer.clone(),
                bind_group: group,
                uploaded: Vec::new(),
            }],
        }
    }

    pub(super) fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        transforms: &[GpuTransform],
    ) {
        // Pages grow independently: adding a page never invalidates earlier bindings or
        // retained vertex/index buffers. Free scene slots are reused by the CPU arena.
        for (index, chunk) in transforms.chunks(TRANSFORMS_PER_PAGE).enumerate() {
            if index == self.pages.len() {
                let mut uploaded = vec![GpuTransform::default(); TRANSFORMS_PER_PAGE];
                uploaded[..chunk.len()].copy_from_slice(chunk);
                let buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Scene transform page"),
                    contents: bytemuck::cast_slice(&uploaded),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });
                let bind_group = bind_group(device, &self.layout, &buffer);
                self.pages.push(Page {
                    buffer,
                    bind_group,
                    uploaded,
                });
            } else {
                let page = &mut self.pages[index];
                let bytes: &[u8] = bytemuck::cast_slice(chunk);
                if page.uploaded.len() < chunk.len()
                    || bytes != bytemuck::cast_slice::<_, u8>(&page.uploaded[..chunk.len()])
                {
                    queue.write_buffer(&page.buffer, 0, bytes);
                    page.uploaded
                        .resize(TRANSFORMS_PER_PAGE, GpuTransform::default());
                    page.uploaded[..chunk.len()].copy_from_slice(chunk);
                }
            }
        }
    }

    pub(super) fn bind_group(&self, page: usize) -> &wgpu::BindGroup {
        &self.pages[page].bind_group
    }
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct TransformRun {
    pub page: usize,
    pub indices: Range<u32>,
}

pub(super) fn transform_runs(
    geometry: &VertexBuffers<GpuVertex, u16>,
    primitives: &[GpuPrimitive],
) -> Vec<TransformRun> {
    let mut runs: Vec<TransformRun> = Vec::new();
    // Tessellation assigns every vertex of a triangle to the same primitive. Only
    // split index ranges; copying or reordering geometry would break retained reuse
    // or painter order. Identity triangles can use the preceding page.
    for (triangle, indices) in geometry.indices.chunks_exact(3).enumerate() {
        let primitive = &primitives[geometry.vertices[indices[0] as usize].prim_id as usize];
        let page = if primitive.transform_id == 0 {
            runs.last().map_or(0, |run| run.page)
        } else {
            primitive.transform_id as usize / TRANSFORMS_PER_PAGE
        };
        let start = triangle as u32 * 3;
        if let Some(run) = runs.last_mut().filter(|run| run.page == page) {
            run.indices.end = start + 3;
        } else {
            runs.push(TransformRun {
                page,
                indices: start..start + 3,
            });
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_runs_preserve_painter_order_and_identity_across_boundaries() {
        let ids = [479, 480, 0, 959, 960, 479];
        let mut geometry = VertexBuffers::new();
        let mut primitives = Vec::new();
        for (prim_id, transform_id) in ids.into_iter().enumerate() {
            geometry.vertices.push(GpuVertex {
                prim_id: prim_id as u32,
                ..GpuVertex::default()
            });
            geometry.indices.extend([prim_id as u16; 3]);
            primitives.push(GpuPrimitive {
                transform_id,
                ..GpuPrimitive::default()
            });
        }
        assert_eq!(
            transform_runs(&geometry, &primitives),
            vec![
                TransformRun {
                    page: 0,
                    indices: 0..3
                },
                TransformRun {
                    page: 1,
                    indices: 3..12
                },
                TransformRun {
                    page: 2,
                    indices: 12..15
                },
                TransformRun {
                    page: 0,
                    indices: 15..18
                },
            ]
        );
        assert!(TRANSFORMS_PER_PAGE * std::mem::size_of::<GpuTransform>() <= 16 * 1024);
    }
}

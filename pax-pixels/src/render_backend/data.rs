use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct GpuGlobals {
    pub resolution: [f32; 2],
    pub dpr: u32,
    pub _pad2: i32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuPrimitive {
    pub fill_id: u16,
    pub fill_type_flag: u16,
    pub z_index: i32,
    pub clipping_id: u32,
    pub transform_id: u32, //not used atm
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuColor {
    pub color: [f32; 4],
}

// OBS: if you change this, you need to change the padding in GpuColoring to
// match the alignment requirements (> 16 bytes + power of two).
const MAX_GRADIENT_STOPS: usize = 8;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuGradient {
    pub colors: [[f32; 4]; MAX_GRADIENT_STOPS],
    pub stops: [f32; MAX_GRADIENT_STOPS],
    pub position: [f32; 2],
    pub main_axis: [f32; 2],
    pub off_axis: [f32; 2],
    pub stop_count: u32,
    pub type_id: u32,
    pub _padding: [u32; 16],
}

#[repr(C)]
#[derive(Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuTransform {
    pub transform: [[f32; 2]; 3],
    pub _pad: u32,
    pub _pad2: u32,
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuVertex {
    pub position: [f32; 2],
    pub normal: [f32; 2],
    pub prim_id: u32,
}

impl GpuVertex {
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBS: [wgpu::VertexAttribute; 3] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Uint32];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }
}

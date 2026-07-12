use bytemuck::{Pod, Zeroable};

use crate::Transform2D;

pub(crate) const MAX_SCENE_LIGHTS: usize = 8;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct GpuGlobals {
    pub resolution: [f32; 2],
    pub dpr: [f32; 2],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuPrimitive {
    pub fill_id: u16,
    pub fill_type_flag: u16,
    pub z_index: i32,
    pub material_id: u32,
    pub transform_id: u32,
    pub draw_range: [f32; 4],
    pub fill_reveal: [f32; 4],
    pub reveal_bounds: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuColor {
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuMaterial {
    pub coefficients: [f32; 4],
    pub emissive: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuSceneLight {
    pub position: [f32; 4],
    pub direction: [f32; 4],
    pub color: [f32; 4],
    pub params: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuSceneLighting {
    pub ambient: [f32; 4],
    pub meta: [u32; 4],
    pub lights: [GpuSceneLight; MAX_SCENE_LIGHTS],
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
#[derive(Debug, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuTransform {
    pub transform: [[f32; 2]; 3],
    pub opacity: f32,
    pub _pad: f32,
}

impl Default for GpuTransform {
    fn default() -> Self {
        Self {
            transform: Transform2D::identity().to_arrays(),
            opacity: 1.0,
            _pad: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuVertex {
    pub position: [f32; 2],
    pub normal: [f32; 2],
    pub prim_id: u32,
    pub path_progress: f32,
}

impl GpuVertex {
    pub(crate) fn desc() -> wgpu::VertexBufferLayout<'static> {
        const ATTRIBS: [wgpu::VertexAttribute; 4] =
            wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Uint32, 3 => Float32];
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<GpuVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &ATTRIBS,
        }
    }
}

use bytemuck::{Pod, Zeroable};

use crate::Transform2D;

pub(crate) const MAX_SCENE_LIGHTS: usize = 8;

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub struct GpuGlobals {
    pub resolution: [f32; 2],
    pub dpr: [f32; 2],
    pub origin: [f32; 2],
    pub _padding: [f32; 2],
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
    pub light_mask: u32,
    pub paint_count: u32,
    pub _padding: [u32; 2],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuColor {
    pub color: [f32; 4],
}

#[repr(C)]
#[derive(Debug, Default, Copy, Clone, Pod, Zeroable)]
pub(crate) struct GpuMaterial {
    pub surface: [f32; 4],
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
    pub surface_to_world_x: [f32; 4],
    pub surface_to_world_y: [f32; 4],
    pub view: [f32; 4],
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
    pub weight: f32,
    pub _padding_0: u32,
    pub focal_point: [f32; 2],
    pub _padding: [u32; 12],
}

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, Pod, Zeroable)]
pub(crate) struct GpuTransform {
    pub transform: [[f32; 2]; 3],
    pub opacity: f32,
    /// 0 = affine; 1 = homogeneous; 2 = homogeneous with camera depth clipping.
    pub projection_mode: f32,
    #[cfg(feature = "3d")]
    pub projection: [[f32; 4]; 3],
    #[cfg(feature = "3d")]
    pub camera_clip: [[f32; 4]; 2],
    #[cfg(feature = "3d")]
    pub lighting_x: [f32; 4],
    #[cfg(feature = "3d")]
    pub lighting_y: [f32; 4],
    #[cfg(feature = "3d")]
    pub lighting_origin: [f32; 4],
    // w distinguishes an explicit world frame from immediate affine draws.
    #[cfg(feature = "3d")]
    pub lighting_normal: [f32; 4],
}

impl Default for GpuTransform {
    fn default() -> Self {
        Self {
            transform: Transform2D::identity().to_arrays(),
            opacity: 1.0,
            projection_mode: 0.0,
            #[cfg(feature = "3d")]
            projection: [[0.; 4]; 3],
            #[cfg(feature = "3d")]
            camera_clip: [[0.; 4]; 2],
            #[cfg(feature = "3d")]
            lighting_x: [1., 0., 0., 0.],
            #[cfg(feature = "3d")]
            lighting_y: [0., 1., 0., 0.],
            #[cfg(feature = "3d")]
            lighting_origin: [0.; 4],
            #[cfg(feature = "3d")]
            lighting_normal: [0., 0., 1., 0.],
        }
    }
}

impl GpuTransform {
    #[cfg(feature = "3d")]
    pub(crate) fn projected(plane: pax_runtime_api::math::ProjectedPlane, opacity: f32) -> Self {
        let m = plane.matrix().coeffs();
        Self {
            projection_mode: if plane.clips_depth() { 2. } else { 1. },
            projection: [
                [m[0], m[1], m[2], m[3]].map(|v| v as f32),
                [m[4], m[5], m[6], m[7]].map(|v| v as f32),
                [m[12], m[13], m[14], m[15]].map(|v| v as f32),
            ],
            camera_clip: plane
                .camera_clip_rows()
                .map(|r| [r[0] as f32, r[1] as f32, r[2] as f32, 0.]),
            opacity,
            ..Self::default()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpu_primitive_matches_wgsl_storage_stride() {
        assert_eq!(std::mem::size_of::<GpuPrimitive>(), 48);
        assert_eq!(std::mem::align_of::<GpuPrimitive>(), 4);
        assert_eq!(std::mem::size_of::<GpuGradient>(), 256);
    }
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

impl GpuTransform {
    #[cfg(feature = "3d")]
    pub(crate) fn with_lighting(
        mut self,
        surface: Option<pax_runtime_api::LightingSurface>,
    ) -> Self {
        if let Some(s) = surface {
            self.lighting_x = [s.tangent_x[0], s.tangent_x[1], s.tangent_x[2], 0.];
            self.lighting_y = [s.tangent_y[0], s.tangent_y[1], s.tangent_y[2], 0.];
            self.lighting_origin = [s.origin[0], s.origin[1], s.origin[2], 0.];
            self.lighting_normal = [s.normal[0], s.normal[1], s.normal[2], 1.];
        }
        self
    }
}

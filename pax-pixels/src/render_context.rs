use crate::render_backend::stencil;
use crate::render_backend::CpuBuffers;
use crate::render_backend::MAX_BATCH_COLORS;
use crate::render_backend::MAX_BATCH_GRADIENTS;
use crate::render_backend::MAX_BATCH_PRIMITIVES;
use crate::render_backend::MAX_BATCH_TRANSFORMS;
use crate::render_backend::RetainedDraw;
use crate::render_backend::RetainedImageDraw;
use crate::render_backend::RetainedVectorResource;
use crate::Box2D;
use crate::Image;
use crate::Point2D;
use crate::Transform2D;
use crate::Vector2D;
use lyon::lyon_tessellation::BuffersBuilder;
use lyon::lyon_tessellation::FillOptions;
use lyon::lyon_tessellation::FillTessellator;
use lyon::lyon_tessellation::FillVertex;
use lyon::lyon_tessellation::VertexBuffers;
use lyon::path::PathEvent;
use lyon::path::Path;
use lyon::tessellation::StrokeOptions;
use lyon::tessellation::StrokeTessellator;
use lyon::tessellation::StrokeVertex;

use crate::render_backend::data::GpuColor;
use crate::render_backend::data::GpuGradient;
use crate::render_backend::data::GpuPrimitive;
use crate::render_backend::data::GpuTransform;
use crate::render_backend::data::GpuVertex;
use crate::render_backend::CachedTextureResource;
use crate::render_backend::RenderBackend;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};

const DEFAULT_TESSELLATION_TOLERANCE: f32 = 0.1;

pub struct WgpuRenderer<'w> {
    render_backend: RenderBackend<'w>,
    scene: HashMap<u32, RetainedNode>,
    sorted_nodes: Vec<(i32, u32)>,
    order_dirty: bool,
    scene_dirty: bool,
    cached_images: HashMap<String, CachedImageEntry>,
    transform_stack: Vec<Transform2D>,
    clip_stack: Vec<ClipGeometry>,
    saves: Vec<SceneStateSave>,
    current_node: Option<PendingNode>,
    tolerance: f32,
}

struct CachedImageEntry {
    texture: CachedTextureResource,
    version: u64,
}

impl<'w> WgpuRenderer<'w> {
    pub fn new(render_backend: RenderBackend<'w>) -> Self {
        Self {
            render_backend,
            tolerance: DEFAULT_TESSELLATION_TOLERANCE, // TODO expose as option
            scene: HashMap::new(),
            sorted_nodes: Vec::new(),
            order_dirty: false,
            scene_dirty: false,
            cached_images: HashMap::new(),
            transform_stack: vec![Transform2D::identity()],
            clip_stack: Vec::new(),
            saves: vec![],
            current_node: None,
        }
    }

    pub fn current_transform(&self) -> Transform2D {
        self.transform_stack
            .last()
            .copied()
            .unwrap_or_else(Transform2D::identity)
    }

    pub fn stroke_path(&mut self, path: Path, stroke_fill: Fill, stroke_width: f32) {
        let current_transform = self.current_transform();
        let geometry_signature = hash_vector_path(&path, PendingVectorOpKind::Stroke(stroke_width));
        let Some(PendingNode {
            kind: PendingNodeKind::Vector(buffers),
            ..
        }) = self.ensure_vector_node()
        else {
            return;
        };
        buffers.ops.push(PendingVectorOp {
            path,
            fill: stroke_fill,
            transform: current_transform,
            kind: PendingVectorOpKind::Stroke(stroke_width),
            geometry_signature,
        });
    }

    pub fn fill_path(&mut self, path: Path, fill: Fill) {
        let current_transform = self.current_transform();
        let geometry_signature = hash_vector_path(&path, PendingVectorOpKind::Fill);
        let Some(PendingNode {
            kind: PendingNodeKind::Vector(buffers),
            ..
        }) = self.ensure_vector_node()
        else {
            return;
        };
        buffers.ops.push(PendingVectorOp {
            path,
            fill,
            transform: current_transform,
            kind: PendingVectorOpKind::Fill,
            geometry_signature,
        });
    }

    pub fn clear(&mut self) {
        self.scene_dirty = true;
        self.render_backend.clear();
    }

    pub fn draw_image(&mut self, image_key: &str, image_version: u64, image: &Image, rect: Box2D) {
        let needs_upload = self
            .cached_images
            .get(image_key)
            .map(|entry| entry.version != image_version)
            .unwrap_or(true);
        if needs_upload {
            self.cached_images.insert(
                image_key.to_owned(),
                CachedImageEntry {
                    texture: self.render_backend.create_cached_texture(
                        &image.rgba,
                        image.pixel_width,
                        image.pixel_height,
                    ),
                    version: image_version,
                },
            );
        }
        let transform = self.current_transform();
        let clip_stack = self.clip_stack.clone();
        let draw = self
            .render_backend
            .create_image_draw(image_key.to_owned(), transform, rect);
        let Some(current_node) = self.current_node.as_mut() else {
            return;
        };
        current_node.kind = PendingNodeKind::Image(PendingImageNode {
            draw,
            clip_stack,
        });
    }

    pub fn flush(&mut self) {
        if !self.scene_dirty {
            self.transform_stack.truncate(1);
            self.clip_stack.clear();
            self.saves.clear();
            self.current_node = None;
            return;
        }
        if self.order_dirty {
            self.sorted_nodes = self
                .scene
                .iter()
                .map(|(node_id, node)| (node.z_index(), *node_id))
                .collect();
            self.sorted_nodes.sort_unstable();
            self.order_dirty = false;
        }
        if self.should_use_vector_scene_batch() {
            self.flush_vector_scene_batch();
            self.scene_dirty = false;
            self.transform_stack.truncate(1);
            self.clip_stack.clear();
            self.saves.clear();
            self.current_node = None;
            return;
        }
        self.ensure_vector_resources_for_immediate_scene();
        let mut current_clip_stack: Vec<u64> = Vec::new();
        let mut current_batch: Vec<RetainedDraw<'_>> = Vec::new();
        let mut current_batch_clip_stack: Option<Vec<ClipGeometry>> = None;
        for (_, node_id) in &self.sorted_nodes {
            let Some(node) = self.scene.get(node_id) else {
                continue;
            };
            if let Some(batch_clip_stack) = current_batch_clip_stack.as_ref() {
                if !clip_stacks_match(batch_clip_stack, node.clip_stack()) {
                    sync_clip_stack(
                        &mut self.render_backend,
                        &mut current_clip_stack,
                        batch_clip_stack,
                    );
                    let stencil_index = self.render_backend.get_clip_depth();
                    self.render_backend
                        .draw_retained_batch(stencil_index, &current_batch);
                    current_batch.clear();
                    current_batch_clip_stack = Some(node.clip_stack().to_vec());
                }
            } else {
                current_batch_clip_stack = Some(node.clip_stack().to_vec());
            }
            match node {
                RetainedNode::Vector(node) => {
                    let Some(resource) = node.resource.as_ref() else {
                        continue;
                    };
                    current_batch.push(RetainedDraw::Vector(resource));
                }
                RetainedNode::Image(node) => {
                    let Some(texture) = self.cached_images.get(&node.draw.resource.image_key) else {
                        continue;
                    };
                    current_batch.push(RetainedDraw::Image {
                        texture: &texture.texture,
                        draw: &node.draw,
                    });
                }
            }
        }
        if let Some(batch_clip_stack) = current_batch_clip_stack.as_ref() {
            sync_clip_stack(
                &mut self.render_backend,
                &mut current_clip_stack,
                batch_clip_stack,
            );
            let stencil_index = self.render_backend.get_clip_depth();
            self.render_backend
                .draw_retained_batch(stencil_index, &current_batch);
        }
        self.cached_images.retain(|image_key, _| {
            self.scene.values().any(|node| {
                matches!(
                    node,
                    RetainedNode::Image(image_node)
                        if image_node.draw.resource.image_key == *image_key
                )
            })
        });
        self.render_backend.present();
        self.scene_dirty = false;
        self.transform_stack.truncate(1);
        self.clip_stack.clear();
        self.saves.clear();
        self.current_node = None;
    }

    pub fn save(&mut self) {
        self.saves.push(SceneStateSave {
            transform_depth: self.transform_stack.len(),
            clip_depth: self.clip_stack.len(),
        });
    }

    pub fn restore(&mut self) {
        if let Some(save) = self.saves.pop() {
            self.transform_stack.truncate(save.transform_depth);
            self.clip_stack.truncate(save.clip_depth);
        }
    }

    pub fn transform(&mut self, transform: Transform2D) {
        self.transform_stack
            .push(transform.then(&self.current_transform()));
    }

    pub fn clip(&mut self, path: Path) {
        let path = path.transformed(&self.current_transform());
        let options = FillOptions::tolerance(self.tolerance);
        let mut geometry = VertexBuffers::new();
        let mut geometry_builder =
            BuffersBuilder::new(&mut geometry, |vertex: FillVertex| stencil::Vertex {
                position: vertex.position().to_array(),
            });
        match FillTessellator::new().tessellate_path(&path, &options, &mut geometry_builder) {
            Ok(_) => {}
            Err(e) => log::warn!("{:?}", e),
        };
        self.clip_stack.push(ClipGeometry {
            signature: hash_clip_geometry(&geometry),
            geometry,
        });
    }

    pub fn resize(&mut self, width: f32, height: f32) {
        self.scene_dirty = true;
        self.render_backend.resize(width as u32, height as u32);
    }

    pub fn resize_surface(&mut self, width: f32, height: f32) {
        self.scene_dirty = true;
        self.render_backend
            .resize_surface(width.round() as u32, height.round() as u32);
    }

    pub fn set_viewport(&mut self, width: f32, height: f32, dpr: f32) {
        self.scene_dirty = true;
        self.render_backend.set_viewport(width, height, dpr.max(1.0));
    }

    pub fn max_surface_dimension(&self) -> u32 {
        self.render_backend.max_surface_dimension()
    }

    pub fn size(&self) -> (f32, f32) {
        let res = &self.render_backend.globals.resolution;
        (res[0], res[1])
    }

    pub fn begin_node(&mut self, node_id: u32, z_index: i32) -> bool {
        self.current_node = Some(PendingNode {
            id: node_id,
            z_index,
            kind: PendingNodeKind::Empty,
        });
        true
    }

    pub fn end_node(&mut self, node_id: u32) -> bool {
        let Some(pending_node) = self.current_node.take() else {
            return true;
        };
        if pending_node.id != node_id {
            return false;
        }

        let updated = match pending_node.kind {
            PendingNodeKind::Empty => None,
            PendingNodeKind::Vector(buffers) => {
                let geometry_signatures = buffers
                    .ops
                    .iter()
                    .map(|op| op.geometry_signature)
                    .collect::<Vec<_>>();
                let style_signature = hash_vector_styles(&buffers.ops);
                if let Some(RetainedNode::Vector(existing)) = self.scene.get_mut(&node_id) {
                    let prev_z = existing.z_index;
                    let reuse_geometry = existing.geometry_signatures == geometry_signatures;
                    let style_changed = existing.style_signature != style_signature;
                    if !reuse_geometry || style_changed {
                        rebuild_vector_buffers(
                            self.tolerance,
                            &buffers.ops,
                            &mut existing.buffers,
                            reuse_geometry,
                        );
                    }
                    existing.clip_stack = buffers.clip_stack;
                    existing.z_index = pending_node.z_index;
                    existing.gpu_resource_dirty |= !reuse_geometry || style_changed;
                    existing.geometry_signatures = geometry_signatures;
                    existing.style_signature = style_signature;
                    if prev_z != pending_node.z_index {
                        self.order_dirty = true;
                    }
                    self.scene_dirty = true;
                    return true;
                }

                Some(RetainedNode::Vector(RetainedVectorNode {
                    resource: None,
                    buffers: {
                        let mut buffers_out = new_cpu_buffers();
                        rebuild_vector_buffers(
                            self.tolerance,
                            &buffers.ops,
                            &mut buffers_out,
                            false,
                        );
                        buffers_out
                    },
                    clip_stack: buffers.clip_stack,
                    z_index: pending_node.z_index,
                    gpu_resource_dirty: true,
                    geometry_signatures,
                    style_signature,
                }))
            }
            PendingNodeKind::Image(image_node) => Some(RetainedNode::Image(RetainedImageNode {
                draw: image_node.draw,
                clip_stack: image_node.clip_stack,
                z_index: pending_node.z_index,
            })),
        };

        match updated {
            Some(node) => {
                let prev_z = self.scene.get(&node_id).map(|node| node.z_index());
                self.scene.insert(node_id, node);
                if prev_z != Some(pending_node.z_index) {
                    self.order_dirty = true;
                }
                self.scene_dirty = true;
            }
            None => {
                if self.scene.remove(&node_id).is_some() {
                    self.order_dirty = true;
                    self.scene_dirty = true;
                }
            }
        }
        true
    }

    pub fn remove_node(&mut self, node_id: u32) -> bool {
        if self.scene.remove(&node_id).is_some() {
            self.order_dirty = true;
            self.scene_dirty = true;
        }
        true
    }

    fn ensure_vector_node(&mut self) -> Option<&mut PendingNode> {
        let current_node = self.current_node.as_mut()?;
        if matches!(current_node.kind, PendingNodeKind::Empty) {
            current_node.kind = PendingNodeKind::Vector(PendingVectorNode {
                ops: Vec::new(),
                clip_stack: self.clip_stack.clone(),
            });
        }
        Some(current_node)
    }

    fn should_use_vector_scene_batch(&self) -> bool {
        self.scene.len() >= 64
            && self
                .scene
                .values()
                .all(|node| matches!(node, RetainedNode::Vector(_)))
    }

    fn ensure_vector_resources_for_immediate_scene(&mut self) {
        let node_ids: Vec<u32> = self.sorted_nodes.iter().map(|(_, node_id)| *node_id).collect();
        for node_id in node_ids {
            let Some(RetainedNode::Vector(node)) = self.scene.get_mut(&node_id) else {
                continue;
            };
            if !node.gpu_resource_dirty {
                continue;
            }

            match node.resource.as_mut() {
                Some(resource) => {
                    if !self
                        .render_backend
                        .update_vector_resource(resource, &mut node.buffers)
                    {
                        node.resource = Some(
                            self.render_backend.create_vector_resource(&mut node.buffers),
                        );
                    }
                }
                None => {
                    node.resource = Some(self.render_backend.create_vector_resource(&mut node.buffers));
                }
            }
            node.gpu_resource_dirty = false;
        }
    }

    fn flush_vector_scene_batch(&mut self) {
        let mut current_clip_stack: Vec<u64> = Vec::new();
        let mut current_batch_clip_stack: Option<Vec<ClipGeometry>> = None;
        let mut current_buffers = new_cpu_buffers();
        let mut has_geometry = false;

        for (_, node_id) in &self.sorted_nodes {
            let Some(RetainedNode::Vector(node)) = self.scene.get(node_id) else {
                continue;
            };

            if let Some(batch_clip_stack) = current_batch_clip_stack.as_ref() {
                if !clip_stacks_match(batch_clip_stack, &node.clip_stack) {
                    if has_geometry {
                        sync_clip_stack(
                            &mut self.render_backend,
                            &mut current_clip_stack,
                            batch_clip_stack,
                        );
                        self.render_backend.render_primitives(&mut current_buffers);
                        current_buffers = new_cpu_buffers();
                    }
                    current_batch_clip_stack = Some(node.clip_stack.clone());
                }
            } else {
                current_batch_clip_stack = Some(node.clip_stack.clone());
            }

            if has_geometry && would_exceed_batch_capacity(&current_buffers, &node.buffers) {
                if let Some(batch_clip_stack) = current_batch_clip_stack.as_ref() {
                    sync_clip_stack(
                        &mut self.render_backend,
                        &mut current_clip_stack,
                        batch_clip_stack,
                    );
                }
                self.render_backend.render_primitives(&mut current_buffers);
                current_buffers = new_cpu_buffers();
                current_batch_clip_stack = Some(node.clip_stack.clone());
            }

            append_cpu_buffers(&mut current_buffers, &node.buffers);
            has_geometry = true;
        }

        if has_geometry {
            if let Some(batch_clip_stack) = current_batch_clip_stack.as_ref() {
                sync_clip_stack(
                    &mut self.render_backend,
                    &mut current_clip_stack,
                    batch_clip_stack,
                );
            }
            self.render_backend.render_primitives(&mut current_buffers);
        }

        self.render_backend.present();
    }
}

struct SceneStateSave {
    transform_depth: usize,
    clip_depth: usize,
}

#[derive(Clone)]
struct ClipGeometry {
    signature: u64,
    geometry: VertexBuffers<stencil::Vertex, u16>,
}

struct PendingNode {
    id: u32,
    z_index: i32,
    kind: PendingNodeKind,
}

enum PendingNodeKind {
    Empty,
    Vector(PendingVectorNode),
    Image(PendingImageNode),
}

struct PendingVectorNode {
    ops: Vec<PendingVectorOp>,
    clip_stack: Vec<ClipGeometry>,
}

struct PendingVectorOp {
    path: Path,
    fill: Fill,
    transform: Transform2D,
    kind: PendingVectorOpKind,
    geometry_signature: u64,
}

#[derive(Clone, Copy)]
enum PendingVectorOpKind {
    Fill,
    Stroke(f32),
}

struct PendingImageNode {
    draw: RetainedImageDraw,
    clip_stack: Vec<ClipGeometry>,
}

enum RetainedNode {
    Vector(RetainedVectorNode),
    Image(RetainedImageNode),
}

impl RetainedNode {
    fn z_index(&self) -> i32 {
        match self {
            RetainedNode::Vector(node) => node.z_index,
            RetainedNode::Image(node) => node.z_index,
        }
    }

    fn clip_stack(&self) -> &[ClipGeometry] {
        match self {
            RetainedNode::Vector(node) => &node.clip_stack,
            RetainedNode::Image(node) => &node.clip_stack,
        }
    }
}

struct RetainedVectorNode {
    resource: Option<RetainedVectorResource>,
    buffers: CpuBuffers,
    clip_stack: Vec<ClipGeometry>,
    z_index: i32,
    gpu_resource_dirty: bool,
    geometry_signatures: Vec<u64>,
    style_signature: u64,
}

struct RetainedImageNode {
    draw: RetainedImageDraw,
    clip_stack: Vec<ClipGeometry>,
    z_index: i32,
}

fn new_cpu_buffers() -> CpuBuffers {
    CpuBuffers {
        geometry: VertexBuffers::new(),
        primitives: Vec::new(),
        colors: Vec::new(),
        gradients: Vec::new(),
        transforms: vec![GpuTransform::default()],
    }
}

fn rebuild_vector_buffers(
    tolerance: f32,
    ops: &[PendingVectorOp],
    buffers: &mut CpuBuffers,
    reuse_geometry: bool,
) {
    if !reuse_geometry {
        buffers.geometry.vertices.clear();
        buffers.geometry.indices.clear();
    }
    buffers.primitives.clear();
    buffers.colors.clear();
    buffers.gradients.clear();
    buffers.transforms.truncate(1);

    for op in ops {
        let transform_id = push_transform(buffers, op.transform);
        let prim_id = push_primitive_def(buffers, op.fill.clone(), transform_id);

        if reuse_geometry {
            continue;
        }

        match op.kind {
            PendingVectorOpKind::Fill => {
                let options = FillOptions::tolerance(tolerance);
                let mut geometry_builder =
                    BuffersBuilder::new(&mut buffers.geometry, |vertex: FillVertex| GpuVertex {
                        position: vertex.position().to_array(),
                        normal: [0.0; 2],
                        prim_id,
                    });
                if let Err(err) = FillTessellator::new()
                    .tessellate_path(&op.path, &options, &mut geometry_builder)
                {
                    log::warn!("{:?}", err);
                }
            }
            PendingVectorOpKind::Stroke(stroke_width) => {
                let options = StrokeOptions::tolerance(tolerance).with_line_width(stroke_width);
                let mut geometry_builder =
                    BuffersBuilder::new(&mut buffers.geometry, |vertex: StrokeVertex| GpuVertex {
                        position: vertex.position().to_array(),
                        normal: [0.0; 2],
                        prim_id,
                    });
                if let Err(err) = StrokeTessellator::new()
                    .tessellate_path(&op.path, &options, &mut geometry_builder)
                {
                    log::warn!("{:?}", err);
                }
            }
        }
    }
}

fn append_cpu_buffers(dst: &mut CpuBuffers, src: &CpuBuffers) {
    let vertex_offset = dst.geometry.vertices.len() as u16;
    let primitive_offset = dst.primitives.len() as u32;
    let src_uses_only_identity_transform =
        src.transforms.len() == 1 && src.transforms[0].transform == GpuTransform::default().transform;
    let transform_offset = if src_uses_only_identity_transform {
        0
    } else {
        dst.transforms.len() as u32 - 1
    };
    let color_offset = dst.colors.len() as u16;
    let gradient_offset = dst.gradients.len() as u16;

    dst.geometry.vertices.extend(src.geometry.vertices.iter().map(|vertex| {
        let mut vertex = *vertex;
        vertex.prim_id += primitive_offset;
        vertex
    }));
    dst.geometry.indices.extend(
        src.geometry
            .indices
            .iter()
            .map(|index| index.saturating_add(vertex_offset)),
    );
    dst.primitives.extend(src.primitives.iter().map(|primitive| {
        let mut primitive = *primitive;
        if primitive.transform_id != 0 {
            primitive.transform_id += transform_offset;
        }
        if primitive.fill_type_flag == 0 {
            primitive.fill_id = primitive.fill_id.saturating_add(color_offset);
        } else {
            primitive.fill_id = primitive.fill_id.saturating_add(gradient_offset);
        }
        primitive
    }));
    if !src_uses_only_identity_transform {
        dst.transforms.extend(src.transforms.iter().skip(1).copied());
    }
    dst.colors.extend(src.colors.iter().copied());
    dst.gradients.extend(src.gradients.iter().copied());
}

fn push_transform(buffers: &mut CpuBuffers, transform: Transform2D) -> u32 {
    if transform == Transform2D::identity() {
        0
    } else {
        let transform_arrays = transform.to_arrays();
        if let Some(transform_id) = buffers
            .transforms
            .iter()
            .position(|existing| existing.transform == transform_arrays)
        {
            return transform_id as u32;
        }

        let transform_id = buffers.transforms.len() as u32;
        buffers.transforms.push(GpuTransform {
            transform: transform_arrays,
            ..GpuTransform::default()
        });
        transform_id
    }
}

fn would_exceed_batch_capacity(dst: &CpuBuffers, src: &CpuBuffers) -> bool {
    let extra_transforms = src.transforms.len().saturating_sub(1);

    dst.geometry.vertices.len() + src.geometry.vertices.len() > u16::MAX as usize
        || dst.primitives.len() + src.primitives.len() > MAX_BATCH_PRIMITIVES
        || dst.colors.len() + src.colors.len() > MAX_BATCH_COLORS
        || dst.gradients.len() + src.gradients.len() > MAX_BATCH_GRADIENTS
        || dst.transforms.len() + extra_transforms > MAX_BATCH_TRANSFORMS
}

fn hash_vector_styles(ops: &[PendingVectorOp]) -> u64 {
    let mut hasher = DefaultHasher::new();
    ops.len().hash(&mut hasher);
    for op in ops {
        hash_transform_bits(&op.transform, &mut hasher);
        hash_fill_bits(&op.fill, &mut hasher);
    }
    hasher.finish()
}

fn hash_transform_bits<H: Hasher>(transform: &Transform2D, state: &mut H) {
    for row in transform.to_arrays() {
        for value in row {
            value.to_bits().hash(state);
        }
    }
}

fn hash_fill_bits<H: Hasher>(fill: &Fill, state: &mut H) {
    match fill {
        Fill::Solid(color) => {
            0u8.hash(state);
            hash_color_bits(color, state);
        }
        Fill::Gradient {
            gradient_type,
            pos,
            main_axis,
            off_axis,
            stops,
        } => {
            1u8.hash(state);
            match gradient_type {
                GradientType::Linear => 0u8.hash(state),
                GradientType::Radial => 1u8.hash(state),
            }
            pos.x.to_bits().hash(state);
            pos.y.to_bits().hash(state);
            main_axis.x.to_bits().hash(state);
            main_axis.y.to_bits().hash(state);
            off_axis.x.to_bits().hash(state);
            off_axis.y.to_bits().hash(state);
            stops.len().hash(state);
            for stop in stops {
                hash_color_bits(&stop.color, state);
                stop.stop.to_bits().hash(state);
            }
        }
    }
}

fn hash_color_bits<H: Hasher>(color: &Color, state: &mut H) {
    for value in color.rgba {
        value.to_bits().hash(state);
    }
}

fn push_primitive_def(buffers: &mut CpuBuffers, fill: Fill, transform_id: u32) -> u32 {
    let fill_id;
    let fill_type_flag;
    match fill {
        Fill::Solid(color) => {
            fill_id = buffers.colors.len() as u16;
            fill_type_flag = 0;
            buffers.colors.push(GpuColor { color: color.rgba });
        }
        Fill::Gradient {
            gradient_type,
            pos,
            main_axis,
            off_axis,
            stops,
        } => {
            fill_id = buffers.gradients.len() as u16;
            fill_type_flag = 1;
            if stops.len() > 8 {
                log::warn!("can't draw graidents with more than 8 stops. truncating.");
            }
            let len = stops.len().min(8);
            let mut colors_buff = [[0.0; 4]; 8];
            let mut stops_buff = [0.0; 8];
            for i in 0..len {
                colors_buff[i] = stops[i].color.rgba;
                stops_buff[i] = stops[i].stop;
            }
            buffers.gradients.push(GpuGradient {
                type_id: match gradient_type {
                    GradientType::Linear => 0,
                    GradientType::Radial => 1,
                },
                position: pos.to_array(),
                main_axis: main_axis.to_array(),
                off_axis: off_axis.to_array(),
                stop_count: len as u32,
                colors: colors_buff,
                stops: stops_buff,
                _padding: [0; 16],
            });
        }
    }
    let primitive = GpuPrimitive {
        fill_id,
        fill_type_flag,
        clipping_id: 0,
        transform_id,
        z_index: 0,
    };
    let prim_id = buffers.primitives.len() as u32;
    buffers.primitives.push(primitive);
    prim_id
}

fn hash_vector_path(path: &Path, kind: PendingVectorOpKind) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    match kind {
        PendingVectorOpKind::Fill => 0u8.hash(&mut hasher),
        PendingVectorOpKind::Stroke(width) => {
            1u8.hash(&mut hasher);
            width.to_bits().hash(&mut hasher);
        }
    }
    for event in path.iter() {
        match event {
            PathEvent::Begin { at } => {
                0u8.hash(&mut hasher);
                at.x.to_bits().hash(&mut hasher);
                at.y.to_bits().hash(&mut hasher);
            }
            PathEvent::Line { from, to } => {
                1u8.hash(&mut hasher);
                from.x.to_bits().hash(&mut hasher);
                from.y.to_bits().hash(&mut hasher);
                to.x.to_bits().hash(&mut hasher);
                to.y.to_bits().hash(&mut hasher);
            }
            PathEvent::Quadratic { from, ctrl, to } => {
                2u8.hash(&mut hasher);
                from.x.to_bits().hash(&mut hasher);
                from.y.to_bits().hash(&mut hasher);
                ctrl.x.to_bits().hash(&mut hasher);
                ctrl.y.to_bits().hash(&mut hasher);
                to.x.to_bits().hash(&mut hasher);
                to.y.to_bits().hash(&mut hasher);
            }
            PathEvent::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => {
                3u8.hash(&mut hasher);
                from.x.to_bits().hash(&mut hasher);
                from.y.to_bits().hash(&mut hasher);
                ctrl1.x.to_bits().hash(&mut hasher);
                ctrl1.y.to_bits().hash(&mut hasher);
                ctrl2.x.to_bits().hash(&mut hasher);
                ctrl2.y.to_bits().hash(&mut hasher);
                to.x.to_bits().hash(&mut hasher);
                to.y.to_bits().hash(&mut hasher);
            }
            PathEvent::End { last, first, close } => {
                4u8.hash(&mut hasher);
                last.x.to_bits().hash(&mut hasher);
                last.y.to_bits().hash(&mut hasher);
                first.x.to_bits().hash(&mut hasher);
                first.y.to_bits().hash(&mut hasher);
                close.hash(&mut hasher);
            }
        }
    }
    hasher.finish()
}

fn hash_clip_geometry(geometry: &VertexBuffers<stencil::Vertex, u16>) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    geometry.indices.hash(&mut hasher);
    for vertex in &geometry.vertices {
        vertex.position[0].to_bits().hash(&mut hasher);
        vertex.position[1].to_bits().hash(&mut hasher);
    }
    hasher.finish()
}

fn sync_clip_stack<'w>(
    render_backend: &mut RenderBackend<'w>,
    current_clip_stack: &mut Vec<u64>,
    desired_clip_stack: &[ClipGeometry],
) {
    let desired_signatures: Vec<u64> = desired_clip_stack.iter().map(|clip| clip.signature).collect();
    let shared_prefix = current_clip_stack
        .iter()
        .zip(desired_signatures.iter())
        .take_while(|(left, right)| left == right)
        .count();
    render_backend.reset_stencil_depth_to(shared_prefix as u32);
    current_clip_stack.truncate(shared_prefix);
    for clip in desired_clip_stack.iter().skip(shared_prefix) {
        render_backend.push_stencil_geometry(clip.geometry.clone());
        current_clip_stack.push(clip.signature);
    }
}

fn clip_stacks_match(left: &[ClipGeometry], right: &[ClipGeometry]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right.iter())
            .all(|(left, right)| left.signature == right.signature)
}

#[derive(Debug, Clone, Copy)]
pub struct GradientStop {
    pub color: Color,
    pub stop: f32,
}

#[derive(Debug, Clone, Copy)]
pub enum GradientType {
    Linear,
    Radial,
}

#[derive(Debug, Clone, Copy)]
pub struct Color {
    rgba: [f32; 4],
}

impl Color {
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { rgba: [r, g, b, a] }
    }

    //credit: ChatGPT
    pub fn hsva(h: f32, s: f32, v: f32, a: f32) -> Self {
        let i = (h * 6.0).floor() as i32;
        let f = h * 6.0 - i as f32;
        let p = v * (1.0 - s);
        let q = v * (1.0 - f * s);
        let t = v * (1.0 - (1.0 - f) * s);

        let (r, g, b) = match i % 6 {
            0 => (v, t, p),
            1 => (q, v, p),
            2 => (p, v, t),
            3 => (p, q, v),
            4 => (t, p, v),
            _ => (v, p, q),
        };
        Self { rgba: [r, g, b, a] }
    }

    //Credit piet library: https://docs.rs/piet/latest/src/piet/color.rs.html#130-173
    pub fn hlca(h: f32, l: f32, c: f32, a: f32) -> Self {
        // The reverse transformation from Lab to XYZ, see
        // https://en.wikipedia.org/wiki/CIELAB_color_space
        fn f_inv(t: f32) -> f32 {
            let d = 6. / 29.;
            if t > d {
                t.powi(3)
            } else {
                3. * d * d * (t - 4. / 29.)
            }
        }
        let th = h * (std::f32::consts::PI / 180.);
        let a_2 = c * th.cos();
        let b = c * th.sin();
        let ll = (l + 16.) * (1. / 116.);
        // Produce raw XYZ values
        let x = f_inv(ll + a_2 * (1. / 500.));
        let y = f_inv(ll);
        let z = f_inv(ll - b * (1. / 200.));
        // This matrix is the concatenation of three sources.
        // First, the white point is taken to be ICC standard D50, so
        // the diagonal matrix of [0.9642, 1, 0.8249]. Note that there
        // is some controversy around this value. However, it matches
        // the other matrices, thus minimizing chroma error.
        //
        // Second, an adaption matrix from D50 to D65. This is the
        // inverse of the recommended D50 to D65 adaptation matrix
        // from the W3C sRGB spec:
        // https://www.w3.org/Graphics/Color/srgb
        //
        // Finally, the conversion from XYZ to linear sRGB values,
        // also taken from the W3C sRGB spec.
        let r_lin = 3.02172918 * x - 1.61692294 * y - 0.40480625 * z;
        let g_lin = -0.94339358 * x + 1.91584267 * y + 0.02755094 * z;
        let b_lin = 0.06945666 * x - 0.22903204 * y + 1.15957526 * z;
        fn gamma(u: f32) -> f32 {
            if u <= 0.0031308 {
                12.92 * u
            } else {
                1.055 * u.powf(1. / 2.4) - 0.055
            }
        }
        Self {
            rgba: [
                gamma(r_lin).clamp(0.0, 1.0),
                gamma(g_lin).clamp(0.0, 1.0),
                gamma(b_lin).clamp(0.0, 1.0),
                a,
            ],
        }
    }
}

#[derive(Debug, Clone)]
pub enum Fill {
    Solid(Color),
    Gradient {
        gradient_type: GradientType,
        pos: Point2D,
        main_axis: Vector2D,
        off_axis: Vector2D,
        stops: Vec<GradientStop>,
    },
}

pub struct Stroke {
    pub color: Color,
    pub weight: f32,
}

use crate::render_backend::stencil;
use crate::render_backend::CpuBuffers;
use crate::render_backend::RetainedImageDraw;
use crate::render_backend::RetainedVectorResource;
use crate::render_backend::SharedRetainedVectorResource;
use crate::render_backend::MAX_BATCH_COLORS;
use crate::render_backend::MAX_BATCH_GRADIENTS;
use crate::render_backend::MAX_BATCH_MATERIALS;
use crate::render_backend::MAX_BATCH_PRIMITIVES;
use crate::render_backend::MAX_BATCH_TRANSFORMS;
use crate::render_backend::MAX_SCENE_CLIPS;
use crate::render_backend::MAX_SCENE_TRANSFORMS;
use crate::render_backend::{RetainedBatchRun, RetainedDraw};
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
use lyon::path::Path;
use lyon::path::PathEvent;
use lyon::tessellation::StrokeOptions;
use lyon::tessellation::StrokeTessellator;
use lyon::tessellation::StrokeVertex;

use crate::point;
use crate::render_backend::data::GpuColor;
use crate::render_backend::data::GpuGradient;
use crate::render_backend::data::GpuMaterial;
use crate::render_backend::data::GpuPrimitive;
use crate::render_backend::data::GpuSceneLight;
use crate::render_backend::data::GpuSceneLighting;
use crate::render_backend::data::GpuTransform;
use crate::render_backend::data::GpuVertex;
use crate::render_backend::data::MAX_SCENE_LIGHTS;
use crate::render_backend::CachedTextureResource;
use crate::render_backend::CapturedFrame;
use crate::render_backend::PrimitiveBatch;
use crate::render_backend::PrimitiveBatchSegment;
use crate::render_backend::RenderBackend;
use crate::render_backend::ScissorRect;
use crate::render_backend::VectorResourceDirty;
use std::cell::RefCell;
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::rc::Rc;

const DEFAULT_TESSELLATION_TOLERANCE: f32 = 0.1;
const MAX_VECTOR_GEOMETRY_CACHE_BYTES: usize = 16 * 1024 * 1024;
const MAX_VECTOR_RESOURCE_CACHE_BYTES: usize = 128 * 1024 * 1024;
pub const NATIVE_VECTOR_RESOURCE_CACHE_BYTES: usize = 256 * 1024 * 1024;

type SharedVectorGeometryCache = Rc<RefCell<VectorGeometryCache>>;
type SharedVectorResourceCache = Rc<RefCell<VectorResourceCache>>;

/// Resource churn counters for renderer profiling.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ResourceChurnStats {
    pub flushes: u64,
    pub retained_scene_resets: u64,
    pub vector_batch_flushes: u64,
    pub vector_buffer_rebuilds: u64,
    pub vector_geometry_rebuilds: u64,
    pub vector_geometry_cache_hits: u64,
    pub vector_geometry_cache_misses: u64,
    pub vector_geometry_cache_evictions: u64,
    pub vector_geometry_cache_bytes: u64,
    pub tessellated_vertices: u64,
    pub tessellated_indices: u64,
    pub cached_vertices_reused: u64,
    pub cached_indices_reused: u64,
    pub vector_resource_creates: u64,
    pub vector_resource_updates: u64,
    pub vector_resource_recreates: u64,
    pub vector_resource_create_bytes: u64,
    pub vector_resource_update_bytes: u64,
    pub vector_resource_cache_hits: u64,
    pub vector_resource_cache_misses: u64,
    pub vector_resource_cache_evictions: u64,
    pub vector_resource_cache_bytes: u64,
    pub texture_creates: u64,
    pub texture_upload_bytes: u64,
    pub retained_nodes_considered: u64,
    pub retained_nodes_visible: u64,
    pub retained_draw_batches: u64,
    pub retained_draws: u64,
    pub retained_vector_draws: u64,
    pub retained_image_draws: u64,
}

impl ResourceChurnStats {
    pub fn merge(&mut self, other: Self) {
        self.flushes += other.flushes;
        self.retained_scene_resets += other.retained_scene_resets;
        self.vector_batch_flushes += other.vector_batch_flushes;
        self.vector_buffer_rebuilds += other.vector_buffer_rebuilds;
        self.vector_geometry_rebuilds += other.vector_geometry_rebuilds;
        self.vector_geometry_cache_hits += other.vector_geometry_cache_hits;
        self.vector_geometry_cache_misses += other.vector_geometry_cache_misses;
        self.vector_geometry_cache_evictions += other.vector_geometry_cache_evictions;
        self.vector_geometry_cache_bytes = self
            .vector_geometry_cache_bytes
            .max(other.vector_geometry_cache_bytes);
        self.tessellated_vertices += other.tessellated_vertices;
        self.tessellated_indices += other.tessellated_indices;
        self.cached_vertices_reused += other.cached_vertices_reused;
        self.cached_indices_reused += other.cached_indices_reused;
        self.vector_resource_creates += other.vector_resource_creates;
        self.vector_resource_updates += other.vector_resource_updates;
        self.vector_resource_recreates += other.vector_resource_recreates;
        self.vector_resource_create_bytes += other.vector_resource_create_bytes;
        self.vector_resource_update_bytes += other.vector_resource_update_bytes;
        self.vector_resource_cache_hits += other.vector_resource_cache_hits;
        self.vector_resource_cache_misses += other.vector_resource_cache_misses;
        self.vector_resource_cache_evictions += other.vector_resource_cache_evictions;
        self.vector_resource_cache_bytes = self
            .vector_resource_cache_bytes
            .max(other.vector_resource_cache_bytes);
        self.texture_creates += other.texture_creates;
        self.texture_upload_bytes += other.texture_upload_bytes;
        self.retained_nodes_considered += other.retained_nodes_considered;
        self.retained_nodes_visible += other.retained_nodes_visible;
        self.retained_draw_batches += other.retained_draw_batches;
        self.retained_draws += other.retained_draws;
        self.retained_vector_draws += other.retained_vector_draws;
        self.retained_image_draws += other.retained_image_draws;
    }

    pub fn has_resource_churn(&self) -> bool {
        let mut stats = *self;
        stats.flushes = 0;
        stats.retained_nodes_considered = 0;
        stats.retained_nodes_visible = 0;
        stats.retained_draw_batches = 0;
        stats.retained_draws = 0;
        stats.retained_vector_draws = 0;
        stats.retained_image_draws = 0;
        stats != Self::default()
    }
}

/// Retained scene renderer that records Pax vector/image commands and flushes them through wgpu.
pub struct WgpuRenderer<'w> {
    render_backend: RenderBackend<'w>,
    scene: HashMap<u32, RetainedNode>,
    transform_arena: TransformArena,
    clip_arena: ClipArena,
    clip_owner_keys: HashMap<u32, Vec<ClipArenaKey>>,
    sorted_nodes: Vec<(i32, u32)>,
    order_dirty: bool,
    scene_dirty: bool,
    cached_images: HashMap<String, CachedImageEntry>,
    transform_stack: Vec<Transform2D>,
    clip_stack: Vec<ClipReference>,
    saves: Vec<SceneStateSave>,
    current_node: Option<PendingNode>,
    tolerance: f32,
    vector_geometry_cache: SharedVectorGeometryCache,
    vector_resource_cache: SharedVectorResourceCache,
    resource_churn_stats: ResourceChurnStats,
}

struct CachedImageEntry {
    texture: CachedTextureResource,
    version: u64,
}

#[derive(Clone, Copy, Debug, Hash, PartialEq, Eq)]
struct VectorGeometryCacheKey {
    geometry_signature: u64,
    tolerance_bits: u32,
}

struct CachedVectorGeometry {
    vertices: Vec<GpuVertex>,
    indices: Vec<u16>,
    bytes: usize,
    last_used: u64,
}

#[derive(Default)]
struct VectorGeometryCacheInsertStats {
    evicted_entries: u64,
    current_bytes: usize,
}

struct VectorGeometryCache {
    entries: HashMap<VectorGeometryCacheKey, CachedVectorGeometry>,
    current_bytes: usize,
    max_bytes: usize,
    clock: u64,
}

impl Default for VectorGeometryCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            current_bytes: 0,
            max_bytes: MAX_VECTOR_GEOMETRY_CACHE_BYTES,
            clock: 0,
        }
    }
}

impl VectorGeometryCache {
    fn get(&mut self, key: &VectorGeometryCacheKey) -> Option<&CachedVectorGeometry> {
        self.clock = self.clock.wrapping_add(1);
        let entry = self.entries.get_mut(key)?;
        entry.last_used = self.clock;
        Some(entry)
    }

    fn insert(
        &mut self,
        key: VectorGeometryCacheKey,
        vertices: Vec<GpuVertex>,
        indices: Vec<u16>,
    ) -> VectorGeometryCacheInsertStats {
        let bytes = vector_geometry_bytes(vertices.len(), indices.len());
        if bytes > self.max_bytes {
            return VectorGeometryCacheInsertStats {
                current_bytes: self.current_bytes,
                ..Default::default()
            };
        }

        if let Some(previous) = self.entries.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(previous.bytes);
        }

        self.clock = self.clock.wrapping_add(1);
        self.current_bytes += bytes;
        self.entries.insert(
            key,
            CachedVectorGeometry {
                vertices,
                indices,
                bytes,
                last_used: self.clock,
            },
        );

        let mut stats = VectorGeometryCacheInsertStats {
            current_bytes: self.current_bytes,
            ..Default::default()
        };
        while self.current_bytes > self.max_bytes {
            let Some(oldest_key) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| *key)
            else {
                break;
            };
            if let Some(removed) = self.entries.remove(&oldest_key) {
                self.current_bytes = self.current_bytes.saturating_sub(removed.bytes);
                stats.evicted_entries += 1;
            }
        }
        stats.current_bytes = self.current_bytes;
        stats
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct VectorResourceKey {
    tolerance_bits: u32,
    geometry_signatures: Vec<u64>,
    fill_signature: u64,
    transform_layout_signature: u64,
}

struct CachedVectorResource {
    resource: Rc<SharedRetainedVectorResource>,
    bytes: usize,
    last_used: u64,
}

#[derive(Default)]
struct VectorResourceCacheStats {
    evicted_entries: u64,
    current_bytes: usize,
}

struct VectorResourceCache {
    entries: HashMap<VectorResourceKey, CachedVectorResource>,
    current_bytes: usize,
    max_bytes: usize,
    clock: u64,
}

impl Default for VectorResourceCache {
    fn default() -> Self {
        Self {
            entries: HashMap::new(),
            current_bytes: 0,
            max_bytes: MAX_VECTOR_RESOURCE_CACHE_BYTES,
            clock: 0,
        }
    }
}

impl VectorResourceCache {
    fn set_max_bytes(&mut self, max_bytes: usize) -> VectorResourceCacheStats {
        self.max_bytes = max_bytes.max(1);
        self.evict_if_needed()
    }

    fn get(&mut self, key: &VectorResourceKey) -> Option<Rc<SharedRetainedVectorResource>> {
        self.clock = self.clock.wrapping_add(1);
        let entry = self.entries.get_mut(key)?;
        entry.last_used = self.clock;
        Some(Rc::clone(&entry.resource))
    }

    fn current_bytes(&self) -> usize {
        self.current_bytes
    }

    fn insert(
        &mut self,
        key: VectorResourceKey,
        resource: Rc<SharedRetainedVectorResource>,
        bytes: u64,
    ) -> VectorResourceCacheStats {
        let bytes = bytes as usize;
        if bytes > self.max_bytes {
            return VectorResourceCacheStats {
                current_bytes: self.current_bytes,
                ..Default::default()
            };
        }

        if let Some(previous) = self.entries.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(previous.bytes);
        }

        self.clock = self.clock.wrapping_add(1);
        self.current_bytes += bytes;
        self.entries.insert(
            key,
            CachedVectorResource {
                resource,
                bytes,
                last_used: self.clock,
            },
        );
        self.evict_if_needed()
    }

    fn remove_if_same(
        &mut self,
        key: &VectorResourceKey,
        resource: &Rc<SharedRetainedVectorResource>,
    ) -> Option<CachedVectorResource> {
        let entry = self.entries.get(key)?;
        if !Rc::ptr_eq(&entry.resource, resource) {
            return None;
        }
        let removed = self.entries.remove(key)?;
        self.current_bytes = self.current_bytes.saturating_sub(removed.bytes);
        Some(removed)
    }

    fn evict_if_needed(&mut self) -> VectorResourceCacheStats {
        let mut stats = VectorResourceCacheStats {
            current_bytes: self.current_bytes,
            ..Default::default()
        };
        while self.current_bytes > self.max_bytes {
            let Some(oldest_key) = self
                .entries
                .iter()
                .filter(|(_, entry)| Rc::strong_count(&entry.resource) == 1)
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            if let Some(removed) = self.entries.remove(&oldest_key) {
                self.current_bytes = self.current_bytes.saturating_sub(removed.bytes);
                stats.evicted_entries += 1;
            }
        }
        stats.current_bytes = self.current_bytes;
        stats
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TransformArenaKey {
    node_id: u32,
    op_index: u32,
}

struct TransformArena {
    slots: Vec<GpuTransform>,
    entries: HashMap<TransformArenaKey, u32>,
    free_slots: Vec<u32>,
    next_slot: u32,
    dirty: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct ClipArenaKey {
    node_id: u32,
    clip_index: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum ClipReference {
    Stencil { clip_id: u32 },
    Scissor(ScissorRect),
}

struct ClipArenaEntry {
    geometry_signature: u64,
    geometry: VertexBuffers<stencil::Vertex, u16>,
}

struct ClipArena {
    slots: Vec<GpuTransform>,
    key_to_id: HashMap<ClipArenaKey, u32>,
    entries: HashMap<u32, ClipArenaEntry>,
    free_ids: Vec<u32>,
    next_id: u32,
    dirty: bool,
}

impl ClipArena {
    fn new() -> Self {
        Self {
            slots: vec![GpuTransform::default(); MAX_SCENE_CLIPS],
            key_to_id: HashMap::new(),
            entries: HashMap::new(),
            free_ids: Vec::new(),
            next_id: 0,
            dirty: true,
        }
    }

    fn sync_clip(
        &mut self,
        node_id: u32,
        clip_index: u32,
        geometry_signature: u64,
        geometry: VertexBuffers<stencil::Vertex, u16>,
        transform: Transform2D,
    ) -> Option<(ClipArenaKey, u32)> {
        let key = ClipArenaKey {
            node_id,
            clip_index,
        };
        let clip_id = if let Some(clip_id) = self.key_to_id.get(&key).copied() {
            clip_id
        } else {
            let Some(clip_id) = self.free_ids.pop().or_else(|| {
                ((self.next_id as usize) < MAX_SCENE_CLIPS).then(|| {
                    let clip_id = self.next_id;
                    self.next_id += 1;
                    clip_id
                })
            }) else {
                log::error!(
                    "clip arena capacity exceeded (capacity {})",
                    MAX_SCENE_CLIPS
                );
                return None;
            };
            self.key_to_id.insert(key, clip_id);
            self.dirty = true;
            clip_id
        };

        let value = GpuTransform {
            transform: transform.to_arrays(),
            ..GpuTransform::default()
        };
        if self.slots[clip_id as usize].transform != value.transform {
            self.slots[clip_id as usize] = value;
            self.dirty = true;
        }

        match self.entries.get_mut(&clip_id) {
            Some(entry) => {
                if entry.geometry_signature != geometry_signature {
                    entry.geometry_signature = geometry_signature;
                    entry.geometry = geometry;
                }
            }
            None => {
                self.entries.insert(
                    clip_id,
                    ClipArenaEntry {
                        geometry_signature,
                        geometry,
                    },
                );
                self.dirty = true;
            }
        }

        Some((key, clip_id))
    }

    fn release_keys(&mut self, keys: &[ClipArenaKey]) {
        for key in keys {
            let Some(clip_id) = self.key_to_id.remove(key) else {
                continue;
            };
            self.entries.remove(&clip_id);
            self.slots[clip_id as usize] = GpuTransform::default();
            self.free_ids.push(clip_id);
            self.dirty = true;
        }
    }

    fn get(&self, clip_id: u32) -> Option<&ClipArenaEntry> {
        self.entries.get(&clip_id)
    }

    fn flush_to_gpu<'w>(&mut self, render_backend: &RenderBackend<'w>) {
        if !self.dirty {
            return;
        }
        render_backend.update_scene_clip_transforms(&self.slots);
        self.dirty = false;
    }
}

impl TransformArena {
    fn new() -> Self {
        Self {
            slots: vec![GpuTransform::default(); MAX_SCENE_TRANSFORMS],
            entries: HashMap::new(),
            free_slots: Vec::new(),
            next_slot: 1,
            dirty: true,
        }
    }

    fn sync_node(
        &mut self,
        node_id: u32,
        local_transforms: &[GpuTransform],
    ) -> (Vec<TransformArenaKey>, Vec<u32>) {
        let mut keys = Vec::with_capacity(local_transforms.len().saturating_sub(1));
        let mut ids = Vec::with_capacity(local_transforms.len());
        ids.push(0);
        for (transform_index, transform) in local_transforms.iter().enumerate().skip(1) {
            let key = TransformArenaKey {
                node_id,
                op_index: (transform_index - 1) as u32,
            };
            let slot = self.entries.get(&key).copied().unwrap_or_else(|| {
                let slot = self
                    .free_slots
                    .pop()
                    .or_else(|| {
                        ((self.next_slot as usize) < MAX_SCENE_TRANSFORMS).then(|| {
                            let slot = self.next_slot;
                            self.next_slot += 1;
                            slot
                        })
                    })
                    .unwrap_or_else(|| {
                        log::error!(
                            "transform arena capacity exceeded (capacity {})",
                            MAX_SCENE_TRANSFORMS
                        );
                        0
                    });
                self.entries.insert(key, slot);
                self.dirty = true;
                slot
            });
            if self.slots[slot as usize].transform != transform.transform
                || (self.slots[slot as usize].opacity - transform.opacity).abs() > f32::EPSILON
            {
                self.slots[slot as usize] = *transform;
                self.dirty = true;
            }
            keys.push(key);
            ids.push(slot);
        }
        (keys, ids)
    }

    fn release_keys(&mut self, keys: &[TransformArenaKey]) {
        for key in keys {
            if let Some(slot) = self.entries.remove(key) {
                if slot != 0 {
                    self.free_slots.push(slot);
                }
            }
        }
    }

    fn flush_to_gpu<'w>(&mut self, render_backend: &RenderBackend<'w>) {
        if !self.dirty {
            return;
        }
        render_backend.update_scene_transforms(&self.slots);
        self.dirty = false;
    }
}

impl<'w> WgpuRenderer<'w> {
    /// Create a retained renderer around a low-level `RenderBackend`.
    pub fn new(render_backend: RenderBackend<'w>) -> Self {
        Self {
            render_backend,
            tolerance: DEFAULT_TESSELLATION_TOLERANCE, // TODO expose as option
            scene: HashMap::new(),
            transform_arena: TransformArena::new(),
            clip_arena: ClipArena::new(),
            clip_owner_keys: HashMap::new(),
            sorted_nodes: Vec::new(),
            order_dirty: false,
            scene_dirty: false,
            cached_images: HashMap::new(),
            transform_stack: vec![Transform2D::identity()],
            clip_stack: Vec::new(),
            saves: vec![],
            current_node: None,
            vector_geometry_cache: Rc::new(RefCell::new(VectorGeometryCache::default())),
            vector_resource_cache: Rc::new(RefCell::new(VectorResourceCache::default())),
            resource_churn_stats: ResourceChurnStats::default(),
        }
    }

    /// Share vector resource caches with another renderer for the same logical layer.
    pub fn share_vector_caches_from(&mut self, other: &Self) {
        self.vector_geometry_cache = Rc::clone(&other.vector_geometry_cache);
        self.vector_resource_cache = Rc::clone(&other.vector_resource_cache);
    }

    pub fn set_vector_resource_cache_max_bytes(&mut self, max_bytes: usize) {
        let stats = self
            .vector_resource_cache
            .borrow_mut()
            .set_max_bytes(max_bytes);
        self.merge_vector_resource_cache_stats(stats);
    }

    pub fn shared_gpu_context_id(&self) -> usize {
        self.render_backend.shared_context_id()
    }

    /// Return and reset accumulated resource churn counters.
    pub fn take_resource_churn_stats(&mut self) -> ResourceChurnStats {
        std::mem::take(&mut self.resource_churn_stats)
    }

    /// Current transform at the top of the render-state stack.
    pub fn current_transform(&self) -> Transform2D {
        self.transform_stack
            .last()
            .copied()
            .unwrap_or_else(Transform2D::identity)
    }

    /// Set the base transform for the physical surface tile being rendered.
    pub fn set_surface_transform(&mut self, transform: Transform2D) {
        // A logical Pax layer may fan out to multiple physical browser canvases. Keep the tile's
        // content-space translation as the base transform so retained node transforms remain
        // tile-agnostic and flush/reset preserves the per-surface coordinate space.
        if let Some(base_transform) = self.transform_stack.first_mut() {
            *base_transform = transform;
        } else {
            self.transform_stack.push(transform);
        }
        self.scene_dirty = true;
        self.clip_stack.clear();
        self.saves.clear();
    }

    /// Drop retained scene state for a surface that has been rebound to a new tile origin.
    pub fn reset_retained_scene(&mut self) {
        self.resource_churn_stats.retained_scene_resets += 1;
        // Reusing a physical browser surface for a different tile origin is only safe if the
        // retained scene is truly origin-agnostic. The current retained vector path still caches
        // per-node transforms/resources against the previous slot assignment, so drop that state
        // and let the engine replay dirty canvas nodes into the reassigned surface.
        self.scene.clear();
        self.transform_arena = TransformArena::new();
        self.clip_arena = ClipArena::new();
        self.clip_owner_keys.clear();
        self.sorted_nodes.clear();
        self.order_dirty = false;
        self.scene_dirty = true;
        self.clip_stack.clear();
        self.saves.clear();
        self.current_node = None;
    }

    /// Queue a stroked vector path into the current retained node.
    pub fn stroke_path(&mut self, path: Path, stroke: Stroke) {
        self.stroke_path_with_opacity(path, stroke, 1.0);
    }

    /// Queue a stroked vector path with an extra opacity multiplier.
    pub fn stroke_path_with_opacity(&mut self, path: Path, stroke: Stroke, opacity: f32) {
        self.stroke_path_with_material_and_opacity(path, stroke, Material::default(), opacity);
    }

    /// Queue a stroked vector path with material response and an extra opacity multiplier.
    pub fn stroke_path_with_material_and_opacity(
        &mut self,
        path: Path,
        stroke: Stroke,
        material: Material,
        opacity: f32,
    ) {
        let current_transform = self.current_transform();
        let geometry_signature = hash_vector_path(
            &path,
            PendingVectorOpKind::Stroke(stroke.weight, stroke.cap, stroke.join),
        );
        let Some(PendingNode {
            kind: PendingNodeKind::Vector(buffers),
            ..
        }) = self.ensure_vector_node()
        else {
            return;
        };
        buffers.ops.push(PendingVectorOp {
            path,
            fill: stroke.fill,
            material,
            transform: current_transform,
            opacity,
            kind: PendingVectorOpKind::Stroke(stroke.weight, stroke.cap, stroke.join),
            geometry_signature,
        });
    }

    /// Queue a filled vector path into the current retained node.
    pub fn fill_path(&mut self, path: Path, fill: Fill) {
        self.fill_path_with_opacity(path, fill, 1.0);
    }

    /// Queue a filled vector path with an extra opacity multiplier.
    pub fn fill_path_with_opacity(&mut self, path: Path, fill: Fill, opacity: f32) {
        self.fill_path_with_material_and_opacity(path, fill, Material::default(), opacity);
    }

    /// Queue a filled vector path with material response and an extra opacity multiplier.
    pub fn fill_path_with_material_and_opacity(
        &mut self,
        path: Path,
        fill: Fill,
        material: Material,
        opacity: f32,
    ) {
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
            material,
            transform: current_transform,
            opacity,
            kind: PendingVectorOpKind::Fill,
            geometry_signature,
        });
    }

    pub fn set_scene_lighting(&mut self, lighting: SceneLighting) {
        self.render_backend
            .set_scene_lighting(to_gpu_scene_lighting(&lighting));
        self.scene_dirty = true;
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
            self.resource_churn_stats.texture_creates += 1;
            self.resource_churn_stats.texture_upload_bytes += image.rgba.len() as u64;
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
        let bounds = transform_box(rect, &transform);
        let draw = self
            .render_backend
            .create_image_draw(image_key.to_owned(), transform, rect);
        let Some(current_node) = self.current_node.as_mut() else {
            return;
        };
        current_node.kind = PendingNodeKind::Image(PendingImageNode {
            draw,
            clip_stack,
            bounds,
        });
    }

    pub fn flush(&mut self) {
        self.flush_internal(false);
    }

    pub fn flush_deferred(&mut self) {
        self.flush_internal(true);
    }

    fn flush_internal(&mut self, defer_submit: bool) {
        self.resource_churn_stats.flushes += 1;
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
            self.resource_churn_stats.vector_batch_flushes += 1;
            self.flush_vector_scene_batch(defer_submit);
            self.scene_dirty = false;
            self.transform_stack.truncate(1);
            self.clip_stack.clear();
            self.saves.clear();
            self.current_node = None;
            return;
        }
        self.render_backend.ensure_frame_cleared();
        self.ensure_vector_resources_for_immediate_scene();
        self.transform_arena.flush_to_gpu(&self.render_backend);
        self.clip_arena.flush_to_gpu(&self.render_backend);
        self.resource_churn_stats.retained_nodes_considered += self.sorted_nodes.len() as u64;
        // Each retained renderer represents one physical surface tile. Cull retained nodes against
        // that tile-local viewport before batching so newly revealed tiles do not replay the full
        // layer scene.
        let viewport_bounds = self.viewport_bounds();
        let mut current_clip_stack: Vec<u32> = Vec::new();
        let mut current_batch: Vec<RetainedDraw<'_>> = Vec::new();
        let mut retained_runs: Vec<RetainedBatchRun<'_>> = Vec::new();
        let mut current_batch_clip_stack: Option<Vec<ClipReference>> = None;
        for (_, node_id) in &self.sorted_nodes {
            let Some(node) = self.scene.get(node_id) else {
                continue;
            };
            if !node.intersects_bounds(&viewport_bounds) {
                continue;
            }
            self.resource_churn_stats.retained_nodes_visible += 1;
            if let Some(batch_clip_stack) = current_batch_clip_stack.as_ref() {
                if !clip_stacks_match(batch_clip_stack, node.clip_stack()) {
                    sync_clip_stack(
                        &mut self.render_backend,
                        &mut current_clip_stack,
                        batch_clip_stack,
                        &self.clip_arena,
                    );
                    let stencil_index = self.render_backend.get_clip_depth();
                    let scissor = scissor_for_clip_stack(batch_clip_stack);
                    record_retained_batch_stats(&mut self.resource_churn_stats, &current_batch);
                    if !current_batch.is_empty() {
                        retained_runs.push(RetainedBatchRun {
                            stencil_index,
                            scissor,
                            draws: std::mem::take(&mut current_batch),
                        });
                    }
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
                    if !resource.is_drawable() {
                        continue;
                    }
                    current_batch.push(RetainedDraw::Vector(resource));
                }
                RetainedNode::Image(node) => {
                    let Some(texture) = self.cached_images.get(&node.draw.resource.image_key)
                    else {
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
                &self.clip_arena,
            );
            let stencil_index = self.render_backend.get_clip_depth();
            let scissor = scissor_for_clip_stack(batch_clip_stack);
            record_retained_batch_stats(&mut self.resource_churn_stats, &current_batch);
            if !current_batch.is_empty() {
                retained_runs.push(RetainedBatchRun {
                    stencil_index,
                    scissor,
                    draws: std::mem::take(&mut current_batch),
                });
            }
        }
        self.render_backend.draw_retained_batch_runs(&retained_runs);
        self.cached_images.retain(|image_key, _| {
            self.scene.values().any(|node| {
                matches!(
                    node,
                    RetainedNode::Image(image_node)
                        if image_node.draw.resource.image_key == *image_key
                )
            })
        });
        let (active_clip_signatures, active_clip_ids) =
            collect_active_clip_resources(&self.scene, &self.clip_arena);
        self.render_backend
            .retain_stencil_resources(&active_clip_signatures, &active_clip_ids);
        self.finish_backend_frame(defer_submit);
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
        let transform = self.current_transform();
        let Some(current_node) = self.current_node.as_mut() else {
            log::warn!("clip called without an active node");
            return;
        };
        if let Some(scissor) = axis_aligned_rect_scissor(&path, &transform) {
            self.clip_stack.push(ClipReference::Scissor(scissor));
            return;
        }
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
        let geometry_signature = hash_clip_geometry(&geometry);
        let clip_index = current_node.owned_clip_keys.len() as u32;
        let Some((clip_key, clip_id)) = self.clip_arena.sync_clip(
            current_node.id,
            clip_index,
            geometry_signature,
            geometry,
            transform,
        ) else {
            return;
        };
        current_node.owned_clip_keys.push(clip_key);
        self.clip_stack.push(ClipReference::Stencil { clip_id });
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

    pub fn set_viewport(&mut self, width: f32, height: f32, dpr: [f32; 2]) {
        self.scene_dirty = true;
        self.render_backend.set_viewport(width, height, dpr);
    }

    pub fn max_surface_dimension(&self) -> u32 {
        self.render_backend.max_surface_dimension()
    }

    pub fn scene_len(&self) -> usize {
        self.scene.len()
    }

    pub fn visible_node_count(&self) -> usize {
        // Resource uploads are per physical surface. Skip nodes outside this tile's viewport so a
        // seam-crossing tile does not eagerly allocate buffers for the entire logical layer.
        let viewport_bounds = self.viewport_bounds();
        self.scene
            .values()
            .filter(|node| node.intersects_bounds(&viewport_bounds))
            .count()
    }

    pub fn size(&self) -> (f32, f32) {
        let res = &self.render_backend.globals.resolution;
        (res[0], res[1])
    }

    pub fn request_screenshot_capture(&mut self, request_id: u32) {
        self.scene_dirty = true;
        self.render_backend.request_screenshot_capture(request_id);
    }

    pub fn take_screenshot_capture(&mut self, request_id: u32) -> Option<CapturedFrame> {
        self.render_backend.take_screenshot_capture(request_id)
    }

    pub fn begin_node(&mut self, node_id: u32, z_index: i32) -> bool {
        self.current_node = Some(PendingNode {
            id: node_id,
            z_index,
            owned_clip_keys: Vec::new(),
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

        let pending_z_index = pending_node.z_index;
        let pending_owned_clip_keys = pending_node.owned_clip_keys.clone();
        let pending_clip_count = pending_owned_clip_keys.len();
        let previous_owned_clip_keys = self
            .clip_owner_keys
            .get(&node_id)
            .cloned()
            .unwrap_or_default();

        let previous = self.scene.remove(&node_id);
        let prev_z = previous.as_ref().map(|node| node.z_index());

        let updated = match pending_node.kind {
            PendingNodeKind::Empty => {
                if let Some(RetainedNode::Vector(existing)) = previous.as_ref() {
                    self.transform_arena.release_keys(&existing.transform_keys);
                }
                None
            }
            PendingNodeKind::Vector(buffers) => {
                let geometry_signatures = buffers
                    .ops
                    .iter()
                    .map(|op| op.geometry_signature)
                    .collect::<Vec<_>>();
                let fill_signature = hash_vector_fills(&buffers.ops);
                let transform_signature = hash_vector_transforms(&buffers.ops);
                let local_transforms = build_local_transform_table(&buffers.ops);
                let (transform_keys, transform_ids) =
                    self.transform_arena.sync_node(node_id, &local_transforms);
                let transform_layout_signature = hash_transform_ids(&transform_ids);
                let resource_key = VectorResourceKey {
                    tolerance_bits: self.tolerance.to_bits(),
                    geometry_signatures: geometry_signatures.clone(),
                    fill_signature,
                    transform_layout_signature,
                };

                match previous {
                    Some(RetainedNode::Vector(mut existing)) => {
                        if existing.transform_keys.len() > transform_keys.len() {
                            self.transform_arena
                                .release_keys(&existing.transform_keys[transform_keys.len()..]);
                        }
                        let reuse_geometry = existing.geometry_signatures == geometry_signatures;
                        let geometry_changed = !reuse_geometry;
                        let fill_changed = existing.fill_signature != fill_signature;
                        let transform_changed = existing.transform_signature != transform_signature;
                        let transform_layout_changed =
                            existing.transform_layout_signature != transform_layout_signature;

                        if geometry_changed || fill_changed || transform_changed {
                            rebuild_vector_buffers(
                                self.tolerance,
                                &buffers.ops,
                                &mut existing.buffers,
                                reuse_geometry,
                                !fill_changed,
                                &self.vector_geometry_cache,
                                &mut self.resource_churn_stats,
                            );
                        }

                        if geometry_changed || fill_changed || transform_layout_changed {
                            existing.retained_primitives = build_retained_primitives(
                                &existing.buffers.primitives,
                                &transform_ids,
                            );
                        }

                        existing.transform_keys = transform_keys;
                        existing.clip_stack = buffers.clip_stack;
                        existing.z_index = pending_z_index;
                        existing.resource_dirty.geometry |= geometry_changed;
                        existing.resource_dirty.primitives |=
                            geometry_changed || fill_changed || transform_layout_changed;
                        existing.resource_dirty.transforms = false;
                        existing.resource_dirty.fill |= fill_changed;
                        existing.geometry_signatures = geometry_signatures;
                        existing.fill_signature = fill_signature;
                        existing.transform_signature = transform_signature;
                        existing.transform_layout_signature = transform_layout_signature;
                        existing.resource_key = resource_key;
                        existing.bounds = compute_vector_node_bounds(&buffers.ops);
                        Some(RetainedNode::Vector(existing))
                    }
                    Some(RetainedNode::Image(_existing)) => {
                        let mut buffers_out = new_cpu_buffers();
                        rebuild_vector_buffers(
                            self.tolerance,
                            &buffers.ops,
                            &mut buffers_out,
                            false,
                            false,
                            &self.vector_geometry_cache,
                            &mut self.resource_churn_stats,
                        );
                        let retained_primitives =
                            build_retained_primitives(&buffers_out.primitives, &transform_ids);
                        Some(RetainedNode::Vector(RetainedVectorNode {
                            resource: None,
                            buffers: buffers_out,
                            retained_primitives,
                            transform_keys,
                            clip_stack: buffers.clip_stack,
                            z_index: pending_z_index,
                            bounds: compute_vector_node_bounds(&buffers.ops),
                            resource_dirty: VectorResourceDirty {
                                geometry: true,
                                primitives: true,
                                transforms: false,
                                fill: true,
                            },
                            geometry_signatures,
                            fill_signature,
                            transform_signature,
                            transform_layout_signature,
                            resource_key,
                            active_resource_key: None,
                        }))
                    }
                    None => {
                        let mut buffers_out = new_cpu_buffers();
                        rebuild_vector_buffers(
                            self.tolerance,
                            &buffers.ops,
                            &mut buffers_out,
                            false,
                            false,
                            &self.vector_geometry_cache,
                            &mut self.resource_churn_stats,
                        );
                        let retained_primitives =
                            build_retained_primitives(&buffers_out.primitives, &transform_ids);
                        Some(RetainedNode::Vector(RetainedVectorNode {
                            resource: None,
                            buffers: buffers_out,
                            retained_primitives,
                            transform_keys,
                            clip_stack: buffers.clip_stack,
                            z_index: pending_z_index,
                            bounds: compute_vector_node_bounds(&buffers.ops),
                            resource_dirty: VectorResourceDirty {
                                geometry: true,
                                primitives: true,
                                transforms: false,
                                fill: true,
                            },
                            geometry_signatures,
                            fill_signature,
                            transform_signature,
                            transform_layout_signature,
                            resource_key,
                            active_resource_key: None,
                        }))
                    }
                }
            }
            PendingNodeKind::Image(image_node) => {
                if let Some(RetainedNode::Vector(existing)) = previous.as_ref() {
                    self.transform_arena.release_keys(&existing.transform_keys);
                }
                Some(RetainedNode::Image(RetainedImageNode {
                    draw: image_node.draw,
                    clip_stack: image_node.clip_stack,
                    z_index: pending_z_index,
                    bounds: image_node.bounds,
                }))
            }
        };

        if previous_owned_clip_keys.len() > pending_clip_count {
            self.clip_arena
                .release_keys(&previous_owned_clip_keys[pending_clip_count..]);
        }
        if pending_owned_clip_keys.is_empty() {
            self.clip_owner_keys.remove(&node_id);
        } else {
            self.clip_owner_keys
                .insert(node_id, pending_owned_clip_keys.clone());
        }

        match updated {
            Some(node) => {
                self.scene.insert(node_id, node);
                if prev_z != Some(pending_z_index) {
                    self.order_dirty = true;
                }
                self.scene_dirty = true;
            }
            None => {
                if prev_z.is_some() {
                    self.order_dirty = true;
                    self.scene_dirty = true;
                }
            }
        }
        true
    }

    pub fn remove_node(&mut self, node_id: u32) -> bool {
        let mut removed = false;
        if let Some(node) = self.scene.remove(&node_id) {
            if let RetainedNode::Vector(node) = &node {
                self.transform_arena.release_keys(&node.transform_keys);
            }
            removed = true;
        }
        if let Some(owned_clip_keys) = self.clip_owner_keys.remove(&node_id) {
            self.clip_arena.release_keys(&owned_clip_keys);
            removed = true;
        }
        if removed {
            self.order_dirty = true;
            self.scene_dirty = true;
        }
        removed
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
        // Vector-scene batching has the same per-tile duplication pressure as the immediate path,
        // so apply the same viewport cull before appending geometry.
        let viewport_bounds = self.viewport_bounds();
        let node_ids: Vec<u32> = self
            .sorted_nodes
            .iter()
            .map(|(_, node_id)| *node_id)
            .collect();
        for node_id in node_ids {
            let Some(retained_node) = self.scene.remove(&node_id) else {
                continue;
            };
            let RetainedNode::Vector(mut node) = retained_node else {
                self.scene.insert(node_id, retained_node);
                continue;
            };
            if !boxes_intersect(&node.bounds, &viewport_bounds) {
                self.scene.insert(node_id, RetainedNode::Vector(node));
                continue;
            }
            if !node.resource_dirty.any() && node.resource.is_some() {
                self.scene.insert(node_id, RetainedNode::Vector(node));
                continue;
            }

            self.ensure_vector_resource_for_node(&mut node);
            self.scene.insert(node_id, RetainedNode::Vector(node));
        }
    }

    fn ensure_vector_resource_for_node(&mut self, node: &mut RetainedVectorNode) {
        let resource_key = node.resource_key.clone();
        if node.resource.is_some() && node.active_resource_key.as_ref() == Some(&resource_key) {
            node.resource_dirty = VectorResourceDirty::default();
            return;
        }

        if let (Some(old_key), Some(resource)) =
            (node.active_resource_key.clone(), node.resource.as_mut())
        {
            let removed_cache_entry = if Rc::strong_count(resource.shared()) == 2 {
                self.vector_resource_cache
                    .borrow_mut()
                    .remove_if_same(&old_key, resource.shared())
            } else {
                None
            };
            if let Some(removed_cache_entry) = removed_cache_entry {
                self.resource_churn_stats.vector_resource_updates += 1;
                if let Some(upload_bytes) = self.render_backend.update_vector_resource(
                    resource,
                    &mut node.buffers,
                    &node.retained_primitives,
                    node.resource_dirty,
                ) {
                    self.resource_churn_stats.vector_resource_update_bytes += upload_bytes;
                    let cache_stats = self.vector_resource_cache.borrow_mut().insert(
                        resource_key.clone(),
                        Rc::clone(resource.shared()),
                        removed_cache_entry.bytes as u64,
                    );
                    self.merge_vector_resource_cache_stats(cache_stats);
                    node.active_resource_key = Some(resource_key);
                    node.resource_dirty = VectorResourceDirty::default();
                    return;
                }
            }
            self.resource_churn_stats.vector_resource_recreates += 1;
        }

        let (cached_resource, cache_bytes) = {
            let mut cache = self.vector_resource_cache.borrow_mut();
            let cached_resource = cache.get(&resource_key);
            (cached_resource, cache.current_bytes())
        };
        if let Some(shared_resource) = cached_resource {
            self.resource_churn_stats.vector_resource_cache_hits += 1;
            self.resource_churn_stats.vector_resource_cache_bytes = self
                .resource_churn_stats
                .vector_resource_cache_bytes
                .max(cache_bytes as u64);
            node.resource = Some(
                self.render_backend
                    .create_vector_resource_from_shared(shared_resource),
            );
            node.active_resource_key = Some(resource_key);
            node.resource_dirty = VectorResourceDirty::default();
            return;
        }

        self.resource_churn_stats.vector_resource_cache_misses += 1;
        self.resource_churn_stats.vector_resource_creates += 1;
        let (shared_resource, upload_bytes) = self
            .render_backend
            .create_shared_vector_resource(&mut node.buffers, &node.retained_primitives);
        self.resource_churn_stats.vector_resource_create_bytes += upload_bytes;
        let cache_stats = self.vector_resource_cache.borrow_mut().insert(
            resource_key.clone(),
            Rc::clone(&shared_resource),
            upload_bytes,
        );
        self.merge_vector_resource_cache_stats(cache_stats);
        node.resource = Some(
            self.render_backend
                .create_vector_resource_from_shared(shared_resource),
        );
        node.active_resource_key = Some(resource_key);
        node.resource_dirty = VectorResourceDirty::default();
    }

    fn merge_vector_resource_cache_stats(&mut self, stats: VectorResourceCacheStats) {
        self.resource_churn_stats.vector_resource_cache_evictions += stats.evicted_entries;
        self.resource_churn_stats.vector_resource_cache_bytes = self
            .resource_churn_stats
            .vector_resource_cache_bytes
            .max(stats.current_bytes as u64);
    }

    fn flush_vector_scene_batch(&mut self, defer_submit: bool) {
        self.transform_arena.flush_to_gpu(&self.render_backend);
        self.clip_arena.flush_to_gpu(&self.render_backend);
        let viewport_bounds = self.viewport_bounds();
        let mut current_clip_stack: Vec<u32> = Vec::new();
        let mut current_batch_clip_stack: Option<Vec<ClipReference>> = None;
        let mut current_buffers = new_cpu_buffers();
        let mut current_segment_start: Option<usize> = None;
        let mut has_geometry = false;
        let mut segments = Vec::new();
        let mut batches = Vec::new();
        for (_, node_id) in &self.sorted_nodes {
            let Some(RetainedNode::Vector(node)) = self.scene.get(node_id) else {
                continue;
            };
            if !boxes_intersect(&node.bounds, &viewport_bounds) {
                continue;
            }

            if let Some(batch_clip_stack) = current_batch_clip_stack.as_ref() {
                if !clip_stacks_match(batch_clip_stack, &node.clip_stack) {
                    if has_geometry {
                        push_primitive_segment(
                            &mut segments,
                            &mut current_clip_stack,
                            batch_clip_stack,
                            &self.clip_arena,
                            current_segment_start.take().unwrap_or(0),
                            current_buffers.geometry.indices.len(),
                        );
                        has_geometry = false;
                    }
                    current_batch_clip_stack = Some(node.clip_stack.clone());
                }
            } else {
                current_batch_clip_stack = Some(node.clip_stack.clone());
            }

            if has_geometry && would_exceed_batch_capacity(&current_buffers, &node.buffers) {
                if let Some(batch_clip_stack) = current_batch_clip_stack.as_ref() {
                    push_primitive_segment(
                        &mut segments,
                        &mut current_clip_stack,
                        batch_clip_stack,
                        &self.clip_arena,
                        current_segment_start.take().unwrap_or(0),
                        current_buffers.geometry.indices.len(),
                    );
                    push_primitive_batch(
                        &mut batches,
                        std::mem::replace(&mut current_buffers, new_cpu_buffers()),
                        std::mem::take(&mut segments),
                    );
                }
                current_batch_clip_stack = Some(node.clip_stack.clone());
            }

            if current_segment_start.is_none() {
                current_segment_start = Some(current_buffers.geometry.indices.len());
            }
            append_cpu_buffers(&mut current_buffers, &node.buffers);
            has_geometry = true;
        }

        if has_geometry {
            if let Some(batch_clip_stack) = current_batch_clip_stack.as_ref() {
                push_primitive_segment(
                    &mut segments,
                    &mut current_clip_stack,
                    batch_clip_stack,
                    &self.clip_arena,
                    current_segment_start.take().unwrap_or(0),
                    current_buffers.geometry.indices.len(),
                );
            }
        }
        push_primitive_batch(&mut batches, current_buffers, segments);
        if batches.is_empty() {
            self.render_backend.ensure_frame_cleared();
        } else {
            self.render_backend.render_primitive_batches(&mut batches);
        }

        let (active_clip_signatures, active_clip_ids) =
            collect_active_clip_resources(&self.scene, &self.clip_arena);
        self.render_backend
            .retain_stencil_resources(&active_clip_signatures, &active_clip_ids);
        self.finish_backend_frame(defer_submit);
    }

    fn finish_backend_frame(&mut self, defer_submit: bool) {
        if defer_submit {
            self.render_backend.finish_frame();
        } else {
            self.render_backend.present();
        }
    }

    pub fn take_pending_command_buffers(&mut self) -> Vec<wgpu::CommandBuffer> {
        self.render_backend.take_pending_command_buffers()
    }

    pub fn submit_command_buffers(&self, command_buffers: Vec<wgpu::CommandBuffer>) {
        self.render_backend.submit_command_buffers(command_buffers);
    }

    pub fn complete_submitted_work(&mut self) {
        self.render_backend.complete_submitted_work();
    }

    pub fn has_pending_submission_cleanup(&self) -> bool {
        self.render_backend.has_pending_submission_cleanup()
    }

    pub fn present_deferred_frame(&mut self) {
        self.render_backend.present_queued_frame();
    }

    fn viewport_bounds(&self) -> Box2D {
        let (width, height) = self.size();
        Box2D {
            min: point(0.0, 0.0),
            max: point(width.max(0.0), height.max(0.0)),
        }
    }
}

struct SceneStateSave {
    transform_depth: usize,
    clip_depth: usize,
}

struct PendingNode {
    id: u32,
    z_index: i32,
    owned_clip_keys: Vec<ClipArenaKey>,
    kind: PendingNodeKind,
}

enum PendingNodeKind {
    Empty,
    Vector(PendingVectorNode),
    Image(PendingImageNode),
}

struct PendingVectorNode {
    ops: Vec<PendingVectorOp>,
    clip_stack: Vec<ClipReference>,
}

struct PendingVectorOp {
    path: Path,
    fill: Fill,
    material: Material,
    transform: Transform2D,
    opacity: f32,
    kind: PendingVectorOpKind,
    geometry_signature: u64,
}

#[derive(Clone, Copy)]
enum PendingVectorOpKind {
    Fill,
    Stroke(f32, StrokeCap, StrokeJoin),
}

struct PendingImageNode {
    draw: RetainedImageDraw,
    clip_stack: Vec<ClipReference>,
    bounds: Box2D,
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

    fn clip_stack(&self) -> &[ClipReference] {
        match self {
            RetainedNode::Vector(node) => &node.clip_stack,
            RetainedNode::Image(node) => &node.clip_stack,
        }
    }

    fn intersects_bounds(&self, viewport_bounds: &Box2D) -> bool {
        match self {
            RetainedNode::Vector(node) => boxes_intersect(&node.bounds, viewport_bounds),
            RetainedNode::Image(node) => boxes_intersect(&node.bounds, viewport_bounds),
        }
    }
}

struct RetainedVectorNode {
    resource: Option<RetainedVectorResource>,
    buffers: CpuBuffers,
    retained_primitives: Vec<GpuPrimitive>,
    transform_keys: Vec<TransformArenaKey>,
    clip_stack: Vec<ClipReference>,
    z_index: i32,
    bounds: Box2D,
    resource_dirty: VectorResourceDirty,
    geometry_signatures: Vec<u64>,
    fill_signature: u64,
    transform_signature: u64,
    transform_layout_signature: u64,
    resource_key: VectorResourceKey,
    active_resource_key: Option<VectorResourceKey>,
}

struct RetainedImageNode {
    draw: RetainedImageDraw,
    clip_stack: Vec<ClipReference>,
    z_index: i32,
    bounds: Box2D,
}

fn new_cpu_buffers() -> CpuBuffers {
    CpuBuffers {
        geometry: VertexBuffers::new(),
        primitives: Vec::new(),
        colors: Vec::new(),
        gradients: Vec::new(),
        materials: Vec::new(),
        transforms: vec![GpuTransform::default()],
    }
}

fn build_retained_primitives(
    local_primitives: &[GpuPrimitive],
    transform_ids: &[u32],
) -> Vec<GpuPrimitive> {
    local_primitives
        .iter()
        .map(|primitive| GpuPrimitive {
            transform_id: transform_ids
                .get(primitive.transform_id as usize)
                .copied()
                .unwrap_or(0),
            ..*primitive
        })
        .collect()
}

fn record_retained_batch_stats(stats: &mut ResourceChurnStats, draws: &[RetainedDraw<'_>]) {
    if draws.is_empty() {
        return;
    }
    stats.retained_draw_batches += 1;
    stats.retained_draws += draws.len() as u64;
    for draw in draws {
        match draw {
            RetainedDraw::Vector(resource) if resource.is_drawable() => {
                stats.retained_vector_draws += 1
            }
            RetainedDraw::Vector(_) => {}
            RetainedDraw::Image { .. } => stats.retained_image_draws += 1,
        }
    }
}

fn build_local_transform_table(ops: &[PendingVectorOp]) -> Vec<GpuTransform> {
    let mut transforms = vec![GpuTransform::default()];
    for op in ops {
        let transform = GpuTransform {
            transform: op.transform.to_arrays(),
            opacity: op.opacity,
            ..GpuTransform::default()
        };
        if transform.transform == GpuTransform::default().transform
            && (transform.opacity - GpuTransform::default().opacity).abs() <= f32::EPSILON
        {
            continue;
        }
        if transforms.iter().any(|existing| {
            existing.transform == transform.transform
                && (existing.opacity - transform.opacity).abs() <= f32::EPSILON
        }) {
            continue;
        }
        transforms.push(transform);
    }
    transforms
}

fn rebuild_vector_buffers(
    tolerance: f32,
    ops: &[PendingVectorOp],
    buffers: &mut CpuBuffers,
    reuse_geometry: bool,
    reuse_fill: bool,
    geometry_cache: &SharedVectorGeometryCache,
    stats: &mut ResourceChurnStats,
) {
    stats.vector_buffer_rebuilds += 1;
    if !reuse_geometry {
        stats.vector_geometry_rebuilds += 1;
        buffers.geometry.vertices.clear();
        buffers.geometry.indices.clear();
    }
    buffers.primitives.clear();
    if !reuse_fill {
        buffers.colors.clear();
        buffers.gradients.clear();
        buffers.materials.clear();
    }
    buffers.transforms.truncate(1);

    let mut next_color_id: u16 = 0;
    let mut next_gradient_id: u16 = 0;
    let mut next_material_id: u32 = 0;

    for op in ops {
        let transform_id = push_transform_with_opacity(buffers, op.transform, op.opacity);
        let prim_id = if reuse_fill {
            push_primitive_with_existing_fill(
                buffers,
                &op.fill,
                transform_id,
                &mut next_color_id,
                &mut next_gradient_id,
                &mut next_material_id,
            )
        } else {
            push_primitive_def(buffers, op.fill.clone(), op.material, transform_id)
        };

        if reuse_geometry {
            continue;
        }

        let cache_key = VectorGeometryCacheKey {
            geometry_signature: op.geometry_signature,
            tolerance_bits: tolerance.to_bits(),
        };
        let mut cache_hit = false;
        {
            let mut cache = geometry_cache.borrow_mut();
            if let Some(geometry) = cache.get(&cache_key) {
                stats.vector_geometry_cache_hits += 1;
                stats.cached_vertices_reused += geometry.vertices.len() as u64;
                stats.cached_indices_reused += geometry.indices.len() as u64;
                append_cached_geometry(
                    buffers,
                    &geometry.vertices,
                    &geometry.indices,
                    prim_id,
                    op.geometry_signature,
                );
                cache_hit = true;
            }
        }
        if cache_hit {
            continue;
        }

        stats.vector_geometry_cache_misses += 1;
        let geometry = tessellate_vector_geometry(tolerance, op);
        stats.tessellated_vertices += geometry.vertices.len() as u64;
        stats.tessellated_indices += geometry.indices.len() as u64;
        append_cached_geometry(
            buffers,
            &geometry.vertices,
            &geometry.indices,
            prim_id,
            op.geometry_signature,
        );
        let insert_stats =
            geometry_cache
                .borrow_mut()
                .insert(cache_key, geometry.vertices, geometry.indices);
        stats.vector_geometry_cache_evictions += insert_stats.evicted_entries;
        stats.vector_geometry_cache_bytes = insert_stats.current_bytes as u64;
    }
}

fn tessellate_vector_geometry(
    tolerance: f32,
    op: &PendingVectorOp,
) -> VertexBuffers<GpuVertex, u16> {
    let mut geometry = VertexBuffers::new();
    match op.kind {
        PendingVectorOpKind::Fill => {
            let options = FillOptions::tolerance(tolerance);
            let mut geometry_builder =
                BuffersBuilder::new(&mut geometry, |vertex: FillVertex| GpuVertex {
                    position: vertex.position().to_array(),
                    normal: [0.0; 2],
                    prim_id: 0,
                });
            if let Err(err) =
                FillTessellator::new().tessellate_path(&op.path, &options, &mut geometry_builder)
            {
                log::warn!("{:?}", err);
            }
        }
        PendingVectorOpKind::Stroke(stroke_width, stroke_cap, stroke_join) => {
            let options = StrokeOptions::tolerance(tolerance)
                .with_line_width(stroke_width)
                .with_line_cap(match stroke_cap {
                    StrokeCap::Butt => lyon::tessellation::LineCap::Butt,
                    StrokeCap::Round => lyon::tessellation::LineCap::Round,
                    StrokeCap::Square => lyon::tessellation::LineCap::Square,
                })
                .with_line_join(match stroke_join {
                    StrokeJoin::Miter => lyon::tessellation::LineJoin::Miter,
                    StrokeJoin::Round => lyon::tessellation::LineJoin::Round,
                    StrokeJoin::Bevel => lyon::tessellation::LineJoin::Bevel,
                });
            let mut geometry_builder =
                BuffersBuilder::new(&mut geometry, |vertex: StrokeVertex| GpuVertex {
                    position: vertex.position().to_array(),
                    normal: [0.0; 2],
                    prim_id: 0,
                });
            if let Err(err) =
                StrokeTessellator::new().tessellate_path(&op.path, &options, &mut geometry_builder)
            {
                log::warn!("{:?}", err);
            }
        }
    }
    geometry
}

fn append_cached_geometry(
    buffers: &mut CpuBuffers,
    vertices: &[GpuVertex],
    indices: &[u16],
    prim_id: u32,
    geometry_signature: u64,
) {
    let vertex_offset = buffers.geometry.vertices.len();
    let Some(vertex_offset) = u16::try_from(vertex_offset).ok() else {
        log::warn!(
            "vector geometry cache append exceeded u16 vertex offset for signature {}",
            geometry_signature
        );
        return;
    };
    if indices
        .iter()
        .any(|index| (*index as usize) + vertex_offset as usize > u16::MAX as usize)
    {
        log::warn!(
            "vector geometry cache append exceeded u16 index capacity for signature {}",
            geometry_signature
        );
        return;
    }

    buffers
        .geometry
        .vertices
        .extend(vertices.iter().map(|vertex| {
            let mut vertex = *vertex;
            vertex.prim_id = prim_id;
            vertex
        }));
    buffers
        .geometry
        .indices
        .extend(indices.iter().map(|index| *index + vertex_offset));
}

fn vector_geometry_bytes(vertices: usize, indices: usize) -> usize {
    vertices * std::mem::size_of::<GpuVertex>() + indices * std::mem::size_of::<u16>()
}

fn append_cpu_buffers(dst: &mut CpuBuffers, src: &CpuBuffers) {
    let vertex_offset = dst.geometry.vertices.len() as u16;
    let primitive_offset = dst.primitives.len() as u32;
    let src_uses_only_identity_transform = src.transforms.len() == 1
        && src.transforms[0].transform == GpuTransform::default().transform
        && (src.transforms[0].opacity - GpuTransform::default().opacity).abs() <= f32::EPSILON;
    let transform_offset = if src_uses_only_identity_transform {
        0
    } else {
        dst.transforms.len() as u32 - 1
    };
    let color_offset = dst.colors.len() as u16;
    let gradient_offset = dst.gradients.len() as u16;
    let material_offset = dst.materials.len() as u32;

    dst.geometry
        .vertices
        .extend(src.geometry.vertices.iter().map(|vertex| {
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
    dst.primitives
        .extend(src.primitives.iter().map(|primitive| {
            let mut primitive = *primitive;
            if primitive.transform_id != 0 {
                primitive.transform_id += transform_offset;
            }
            if primitive.fill_type_flag == 0 {
                primitive.fill_id = primitive.fill_id.saturating_add(color_offset);
            } else {
                primitive.fill_id = primitive.fill_id.saturating_add(gradient_offset);
            }
            primitive.material_id = primitive.material_id.saturating_add(material_offset);
            primitive
        }));
    if !src_uses_only_identity_transform {
        dst.transforms
            .extend(src.transforms.iter().skip(1).copied());
    }
    dst.colors.extend(src.colors.iter().copied());
    dst.gradients.extend(src.gradients.iter().copied());
    dst.materials.extend(src.materials.iter().copied());
}

fn push_transform_with_opacity(
    buffers: &mut CpuBuffers,
    transform: Transform2D,
    opacity: f32,
) -> u32 {
    if transform == Transform2D::identity() && (opacity - 1.0).abs() <= f32::EPSILON {
        0
    } else {
        let transform_arrays = transform.to_arrays();
        if let Some(transform_id) = buffers.transforms.iter().position(|existing| {
            existing.transform == transform_arrays
                && (existing.opacity - opacity).abs() <= f32::EPSILON
        }) {
            return transform_id as u32;
        }

        let transform_id = buffers.transforms.len() as u32;
        buffers.transforms.push(GpuTransform {
            transform: transform_arrays,
            opacity,
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
        || dst.materials.len() + src.materials.len() > MAX_BATCH_MATERIALS
        || dst.transforms.len() + extra_transforms > MAX_BATCH_TRANSFORMS
}

fn hash_vector_fills(ops: &[PendingVectorOp]) -> u64 {
    let mut hasher = DefaultHasher::new();
    ops.len().hash(&mut hasher);
    for op in ops {
        hash_fill_bits(&op.fill, &mut hasher);
        hash_material_bits(&op.material, &mut hasher);
    }
    hasher.finish()
}

fn hash_vector_transforms(ops: &[PendingVectorOp]) -> u64 {
    let mut hasher = DefaultHasher::new();
    ops.len().hash(&mut hasher);
    for op in ops {
        hash_transform_bits(&op.transform, op.opacity, &mut hasher);
    }
    hasher.finish()
}

fn hash_transform_ids(transform_ids: &[u32]) -> u64 {
    let mut hasher = DefaultHasher::new();
    transform_ids.len().hash(&mut hasher);
    for transform_id in transform_ids {
        transform_id.hash(&mut hasher);
    }
    hasher.finish()
}

fn compute_vector_node_bounds(ops: &[PendingVectorOp]) -> Box2D {
    let mut bounds: Option<Box2D> = None;
    for op in ops {
        let Some(path_bounds) = path_control_bounds(&op.path) else {
            continue;
        };
        let mut op_bounds = transform_box(path_bounds, &op.transform);
        if let PendingVectorOpKind::Stroke(width, _, _) = op.kind {
            op_bounds = expand_box(op_bounds, width * 0.5);
        }
        bounds = Some(match bounds {
            Some(existing) => union_boxes(existing, op_bounds),
            None => op_bounds,
        });
    }
    bounds.unwrap_or_else(empty_box)
}

fn path_control_bounds(path: &Path) -> Option<Box2D> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    let mut visit = |point: Point2D| {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    };

    for event in path.iter() {
        match event {
            PathEvent::Begin { at } => visit(at),
            PathEvent::Line { from, to } => {
                visit(from);
                visit(to);
            }
            PathEvent::Quadratic { from, ctrl, to } => {
                visit(from);
                visit(ctrl);
                visit(to);
            }
            PathEvent::Cubic {
                from,
                ctrl1,
                ctrl2,
                to,
            } => {
                visit(from);
                visit(ctrl1);
                visit(ctrl2);
                visit(to);
            }
            PathEvent::End { first, last, .. } => {
                visit(first);
                visit(last);
            }
        }
    }

    if min_x.is_finite() && min_y.is_finite() && max_x.is_finite() && max_y.is_finite() {
        Some(Box2D {
            min: point(min_x, min_y),
            max: point(max_x, max_y),
        })
    } else {
        None
    }
}

fn transform_box(bounds: Box2D, transform: &Transform2D) -> Box2D {
    let corners = [
        bounds.min,
        point(bounds.max.x, bounds.min.y),
        point(bounds.min.x, bounds.max.y),
        bounds.max,
    ];
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for corner in corners {
        let transformed = transform.transform_point(corner);
        min_x = min_x.min(transformed.x);
        min_y = min_y.min(transformed.y);
        max_x = max_x.max(transformed.x);
        max_y = max_y.max(transformed.y);
    }
    Box2D {
        min: point(min_x, min_y),
        max: point(max_x, max_y),
    }
}

fn expand_box(bounds: Box2D, inset: f32) -> Box2D {
    Box2D {
        min: point(bounds.min.x - inset, bounds.min.y - inset),
        max: point(bounds.max.x + inset, bounds.max.y + inset),
    }
}

fn union_boxes(left: Box2D, right: Box2D) -> Box2D {
    Box2D {
        min: point(left.min.x.min(right.min.x), left.min.y.min(right.min.y)),
        max: point(left.max.x.max(right.max.x), left.max.y.max(right.max.y)),
    }
}

fn empty_box() -> Box2D {
    Box2D {
        min: point(0.0, 0.0),
        max: point(0.0, 0.0),
    }
}

fn boxes_intersect(left: &Box2D, right: &Box2D) -> bool {
    left.min.x <= right.max.x
        && left.max.x >= right.min.x
        && left.min.y <= right.max.y
        && left.max.y >= right.min.y
}

fn axis_aligned_rect_scissor(path: &Path, transform: &Transform2D) -> Option<ScissorRect> {
    const EPSILON: f32 = 0.001;
    let mut points = Vec::new();
    for event in path.iter() {
        match event {
            PathEvent::Begin { at } => {
                push_unique_point(&mut points, transform.transform_point(at))
            }
            PathEvent::Line { to, .. } => {
                push_unique_point(&mut points, transform.transform_point(to));
            }
            PathEvent::End { close, .. } => {
                if !close {
                    return None;
                }
            }
            PathEvent::Quadratic { .. } | PathEvent::Cubic { .. } => return None,
        }
    }

    if points.len() > 1 && points_close(points[0], *points.last().unwrap(), EPSILON) {
        points.pop();
    }
    if points.len() != 4 {
        return None;
    }

    let mut xs = Vec::new();
    let mut ys = Vec::new();
    for point in &points {
        push_unique_scalar(&mut xs, point.x, EPSILON);
        push_unique_scalar(&mut ys, point.y, EPSILON);
    }
    if xs.len() != 2 || ys.len() != 2 {
        return None;
    }
    xs.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    ys.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let rect = ScissorRect {
        min_x: xs[0],
        min_y: ys[0],
        max_x: xs[1],
        max_y: ys[1],
    };
    if rect.max_x - rect.min_x <= EPSILON || rect.max_y - rect.min_y <= EPSILON {
        return None;
    }
    let has_all_corners = [
        (rect.min_x, rect.min_y),
        (rect.max_x, rect.min_y),
        (rect.max_x, rect.max_y),
        (rect.min_x, rect.max_y),
    ]
    .iter()
    .all(|(x, y)| {
        points
            .iter()
            .any(|point| (point.x - *x).abs() <= EPSILON && (point.y - *y).abs() <= EPSILON)
    });
    has_all_corners.then_some(rect)
}

fn push_unique_point(points: &mut Vec<Point2D>, point: Point2D) {
    const EPSILON: f32 = 0.001;
    if points
        .last()
        .is_some_and(|previous| points_close(*previous, point, EPSILON))
    {
        return;
    }
    points.push(point);
}

fn points_close(left: Point2D, right: Point2D, epsilon: f32) -> bool {
    (left.x - right.x).abs() <= epsilon && (left.y - right.y).abs() <= epsilon
}

fn push_unique_scalar(values: &mut Vec<f32>, value: f32, epsilon: f32) {
    if !values
        .iter()
        .any(|existing| (*existing - value).abs() <= epsilon)
    {
        values.push(value);
    }
}

fn hash_transform_bits<H: Hasher>(transform: &Transform2D, opacity: f32, state: &mut H) {
    for row in transform.to_arrays() {
        for value in row {
            value.to_bits().hash(state);
        }
    }
    opacity.to_bits().hash(state);
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

fn hash_material_bits<H: Hasher>(material: &Material, state: &mut H) {
    material.unlit.hash(state);
    for value in material.coefficients {
        value.to_bits().hash(state);
    }
    for value in material.emissive {
        value.to_bits().hash(state);
    }
}

fn hash_color_bits<H: Hasher>(color: &Color, state: &mut H) {
    for value in color.rgba {
        value.to_bits().hash(state);
    }
}

fn to_gpu_material(material: Material) -> GpuMaterial {
    if material.unlit {
        return GpuMaterial {
            coefficients: [-1.0, 0.0, 0.0, 0.0],
            emissive: [0.0; 4],
        };
    }

    GpuMaterial {
        coefficients: material.coefficients,
        emissive: material.emissive,
    }
}

fn to_gpu_scene_lighting(lighting: &SceneLighting) -> GpuSceneLighting {
    let mut out = GpuSceneLighting {
        ambient: [
            lighting.ambient_color.rgba[0],
            lighting.ambient_color.rgba[1],
            lighting.ambient_color.rgba[2],
            lighting.ambient_intensity.max(0.0),
        ],
        meta: [lighting.active as u32, 0, 0, 0],
        ..GpuSceneLighting::default()
    };

    let count = lighting.lights.len().min(MAX_SCENE_LIGHTS);
    out.meta[1] = count as u32;
    for (index, light) in lighting.lights.iter().take(count).enumerate() {
        out.lights[index] = GpuSceneLight {
            position: [light.position[0], light.position[1], light.position[2], 0.0],
            direction: [
                light.direction[0],
                light.direction[1],
                light.direction[2],
                0.0,
            ],
            color: [
                light.color.rgba[0],
                light.color.rgba[1],
                light.color.rgba[2],
                light.intensity.max(0.0),
            ],
            params: [
                match light.shape {
                    LightShape::Point => 0.0,
                    LightShape::Directional => 1.0,
                },
                light.radius.max(0.0),
                0.0,
                0.0,
            ],
        };
    }

    out
}

fn push_primitive_def(
    buffers: &mut CpuBuffers,
    fill: Fill,
    material: Material,
    transform_id: u32,
) -> u32 {
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
    let material_id = buffers.materials.len() as u32;
    buffers.materials.push(to_gpu_material(material));
    let primitive = GpuPrimitive {
        fill_id,
        fill_type_flag,
        material_id,
        transform_id,
        z_index: 0,
    };
    let prim_id = buffers.primitives.len() as u32;
    buffers.primitives.push(primitive);
    prim_id
}

fn push_primitive_with_existing_fill(
    buffers: &mut CpuBuffers,
    fill: &Fill,
    transform_id: u32,
    next_color_id: &mut u16,
    next_gradient_id: &mut u16,
    next_material_id: &mut u32,
) -> u32 {
    let (fill_id, fill_type_flag) = match fill {
        Fill::Solid(_) => {
            let fill_id = *next_color_id;
            *next_color_id = next_color_id.saturating_add(1);
            (fill_id, 0)
        }
        Fill::Gradient { .. } => {
            let fill_id = *next_gradient_id;
            *next_gradient_id = next_gradient_id.saturating_add(1);
            (fill_id, 1)
        }
    };
    let material_id = *next_material_id;
    *next_material_id = next_material_id.saturating_add(1);
    let prim_id = buffers.primitives.len() as u32;
    buffers.primitives.push(GpuPrimitive {
        fill_id,
        fill_type_flag,
        material_id,
        transform_id,
        z_index: 0,
    });
    prim_id
}

fn hash_vector_path(path: &Path, kind: PendingVectorOpKind) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    match kind {
        PendingVectorOpKind::Fill => 0u8.hash(&mut hasher),
        PendingVectorOpKind::Stroke(width, cap, join) => {
            1u8.hash(&mut hasher);
            width.to_bits().hash(&mut hasher);
            cap.hash(&mut hasher);
            join.hash(&mut hasher);
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

fn clip_stack_sync<'a>(
    current_clip_stack: &[u32],
    desired_clip_stack: &[ClipReference],
    clip_arena: &'a ClipArena,
) -> (usize, Vec<stencil::ClipDraw<'a>>) {
    let desired_stencil_clip_ids: Vec<u32> = desired_clip_stack
        .iter()
        .filter_map(|clip| match clip {
            ClipReference::Stencil { clip_id } => Some(*clip_id),
            ClipReference::Scissor(_) => None,
        })
        .collect();
    let shared_prefix = current_clip_stack
        .iter()
        .zip(desired_stencil_clip_ids.iter())
        .take_while(|(left, right)| left == right)
        .count();
    let mut clip_draws = Vec::new();
    for clip_id in desired_stencil_clip_ids.iter().skip(shared_prefix) {
        let Some(entry) = clip_arena.get(*clip_id) else {
            log::error!("missing clip arena entry for clip {}", clip_id);
            continue;
        };
        clip_draws.push(stencil::ClipDraw {
            clip_id: *clip_id,
            geometry_signature: entry.geometry_signature,
            geometry: &entry.geometry,
        });
    }
    (shared_prefix, clip_draws)
}

fn update_current_clip_stack(
    current_clip_stack: &mut Vec<u32>,
    shared_prefix: usize,
    clip_draws: &[stencil::ClipDraw<'_>],
) {
    current_clip_stack.truncate(shared_prefix);
    current_clip_stack.extend(clip_draws.iter().map(|clip| clip.clip_id));
}

fn push_primitive_segment<'a>(
    segments: &mut Vec<PrimitiveBatchSegment<'a>>,
    current_clip_stack: &mut Vec<u32>,
    desired_clip_stack: &[ClipReference],
    clip_arena: &'a ClipArena,
    index_start: usize,
    index_end: usize,
) {
    if index_end <= index_start {
        return;
    }
    let (shared_prefix, clip_draws) =
        clip_stack_sync(current_clip_stack, desired_clip_stack, clip_arena);
    update_current_clip_stack(current_clip_stack, shared_prefix, &clip_draws);
    segments.push(PrimitiveBatchSegment {
        stencil_depth: shared_prefix as u32,
        clips: clip_draws,
        scissor: scissor_for_clip_stack(desired_clip_stack),
        index_start: index_start as u32,
        index_count: (index_end - index_start) as u32,
    });
}

fn push_primitive_batch<'a>(
    batches: &mut Vec<PrimitiveBatch<'a>>,
    buffers: CpuBuffers,
    segments: Vec<PrimitiveBatchSegment<'a>>,
) {
    if segments.is_empty() {
        return;
    }
    batches.push(PrimitiveBatch { buffers, segments });
}

fn sync_clip_stack<'w>(
    render_backend: &mut RenderBackend<'w>,
    current_clip_stack: &mut Vec<u32>,
    desired_clip_stack: &[ClipReference],
    clip_arena: &ClipArena,
) {
    let (shared_prefix, clip_draws) =
        clip_stack_sync(current_clip_stack, desired_clip_stack, clip_arena);
    render_backend.sync_stencil_stack(shared_prefix as u32, &clip_draws);
    update_current_clip_stack(current_clip_stack, shared_prefix, &clip_draws);
}

fn scissor_for_clip_stack(clip_stack: &[ClipReference]) -> Option<ScissorRect> {
    let mut scissor = None;
    for clip in clip_stack {
        let ClipReference::Scissor(rect) = clip else {
            continue;
        };
        scissor = Some(match scissor {
            Some(existing) => intersect_scissor_rects(existing, *rect),
            None => *rect,
        });
    }
    scissor
}

fn intersect_scissor_rects(left: ScissorRect, right: ScissorRect) -> ScissorRect {
    ScissorRect {
        min_x: left.min_x.max(right.min_x),
        min_y: left.min_y.max(right.min_y),
        max_x: left.max_x.min(right.max_x),
        max_y: left.max_y.min(right.max_y),
    }
}

fn collect_active_clip_resources(
    scene: &HashMap<u32, RetainedNode>,
    clip_arena: &ClipArena,
) -> (HashSet<u64>, HashSet<u32>) {
    let mut signatures = HashSet::new();
    let mut clip_ids = HashSet::new();
    for node in scene.values() {
        for clip in node.clip_stack() {
            let ClipReference::Stencil { clip_id } = clip else {
                continue;
            };
            clip_ids.insert(*clip_id);
            if let Some(entry) = clip_arena.get(*clip_id) {
                signatures.insert(entry.geometry_signature);
            }
        }
    }
    (signatures, clip_ids)
}

fn clip_stacks_match(left: &[ClipReference], right: &[ClipReference]) -> bool {
    left == right
}

#[derive(Debug, Clone, Copy)]
/// One color stop in a GPU gradient fill.
pub struct GradientStop {
    pub color: Color,
    pub stop: f32,
}

#[derive(Debug, Clone, Copy)]
/// Shape of a GPU gradient fill.
pub enum GradientType {
    Linear,
    Radial,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// Linear RGBA color used by the low-level renderer.
pub struct Color {
    rgba: [f32; 4],
}

impl Color {
    /// Construct a color from linear RGBA channels in the range 0.0-1.0.
    pub fn rgba(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { rgba: [r, g, b, a] }
    }

    /// Construct a color from HSV plus alpha, with hue normalized to 0.0-1.0.
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

    /// Construct a color from HLC/Lab-style components plus alpha.
    // Credit piet library: https://docs.rs/piet/latest/src/piet/color.rs.html#130-173
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
/// Fill style for a tessellated vector path.
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

#[derive(Debug, Clone, Copy, PartialEq)]
/// Light-reactive material coefficients for a tessellated vector path.
pub struct Material {
    pub coefficients: [f32; 4],
    pub emissive: [f32; 4],
    pub unlit: bool,
}

impl Material {
    pub fn matte() -> Self {
        Self::default()
    }

    pub fn custom(
        ambient: f32,
        diffuse: f32,
        specular: f32,
        roughness: f32,
        emissive: Color,
        emissive_intensity: f32,
    ) -> Self {
        Self {
            coefficients: [
                ambient.clamp(0.0, 8.0),
                diffuse.clamp(0.0, 8.0),
                specular.clamp(0.0, 8.0),
                roughness.clamp(0.0, 1.0),
            ],
            emissive: [
                emissive.rgba[0],
                emissive.rgba[1],
                emissive.rgba[2],
                emissive_intensity.max(0.0),
            ],
            unlit: false,
        }
    }

    pub fn unlit() -> Self {
        Self {
            coefficients: [0.0; 4],
            emissive: [0.0; 4],
            unlit: true,
        }
    }
}

impl Default for Material {
    fn default() -> Self {
        Self {
            coefficients: [1.0, 0.82, 0.08, 0.78],
            emissive: [0.0; 4],
            unlit: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Shape of a scene light.
pub enum LightShape {
    Point,
    Directional,
}

#[derive(Debug, Clone, Copy, PartialEq)]
/// A resolved scene light for the low-level renderer.
pub struct SceneLight {
    pub shape: LightShape,
    pub position: [f32; 3],
    pub direction: [f32; 3],
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
}

#[derive(Debug, Clone, PartialEq)]
/// Resolved lighting state for one retained vector scene.
pub struct SceneLighting {
    pub active: bool,
    pub ambient_color: Color,
    pub ambient_intensity: f32,
    pub lights: Vec<SceneLight>,
}

impl Default for SceneLighting {
    fn default() -> Self {
        Self {
            active: false,
            ambient_color: Color::rgba(1.0, 1.0, 1.0, 1.0),
            ambient_intensity: 1.0,
            lights: vec![],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Stroke end-cap style.
pub enum StrokeCap {
    Butt,
    Round,
    Square,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Stroke join style.
pub enum StrokeJoin {
    Miter,
    Round,
    Bevel,
}

#[derive(Debug, Clone)]
/// Stroke style for a tessellated vector path.
pub struct Stroke {
    pub fill: Fill,
    pub weight: f32,
    pub cap: StrokeCap,
    pub join: StrokeJoin,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_path(width: f32, height: f32) -> Path {
        let mut builder = Path::builder();
        builder.begin(point(0.0, 0.0));
        builder.line_to(point(width, 0.0));
        builder.line_to(point(width, height));
        builder.line_to(point(0.0, height));
        builder.end(true);
        builder.build()
    }

    #[test]
    fn axis_aligned_rect_clip_becomes_scissor() {
        let transform = Transform2D::from_array([2.0, 0.0, 0.0, 3.0, 5.0, 7.0]);
        let scissor = axis_aligned_rect_scissor(&rect_path(10.0, 5.0), &transform).unwrap();
        assert_eq!(scissor.min_x, 5.0);
        assert_eq!(scissor.min_y, 7.0);
        assert_eq!(scissor.max_x, 25.0);
        assert_eq!(scissor.max_y, 22.0);
    }

    #[test]
    fn sheared_rect_clip_stays_on_stencil_path() {
        let transform = Transform2D::from_array([1.0, 0.5, 0.0, 1.0, 0.0, 0.0]);
        assert!(axis_aligned_rect_scissor(&rect_path(10.0, 5.0), &transform).is_none());
    }

    #[test]
    fn vector_geometry_cache_reuses_tessellation_and_remaps_primitive_ids() {
        let path = rect_path(10.0, 5.0);
        let op = PendingVectorOp {
            path: path.clone(),
            fill: Fill::Solid(Color::rgba(1.0, 0.0, 0.0, 1.0)),
            material: Material::default(),
            transform: Transform2D::identity(),
            opacity: 1.0,
            kind: PendingVectorOpKind::Fill,
            geometry_signature: hash_vector_path(&path, PendingVectorOpKind::Fill),
        };
        let ops = vec![
            PendingVectorOp {
                path,
                fill: Fill::Solid(Color::rgba(0.0, 1.0, 0.0, 1.0)),
                material: Material::default(),
                transform: Transform2D::identity(),
                opacity: 1.0,
                kind: PendingVectorOpKind::Fill,
                geometry_signature: op.geometry_signature,
            },
            op,
        ];
        let cache = Rc::new(RefCell::new(VectorGeometryCache::default()));
        let mut stats = ResourceChurnStats::default();
        let mut buffers = new_cpu_buffers();

        rebuild_vector_buffers(
            DEFAULT_TESSELLATION_TOLERANCE,
            &ops,
            &mut buffers,
            false,
            false,
            &cache,
            &mut stats,
        );

        assert_eq!(stats.vector_geometry_cache_misses, 1);
        assert_eq!(stats.vector_geometry_cache_hits, 1);
        assert!(buffers
            .geometry
            .vertices
            .iter()
            .any(|vertex| vertex.prim_id == 0));
        assert!(buffers
            .geometry
            .vertices
            .iter()
            .any(|vertex| vertex.prim_id == 1));

        let mut second_stats = ResourceChurnStats::default();
        let mut second_buffers = new_cpu_buffers();
        rebuild_vector_buffers(
            DEFAULT_TESSELLATION_TOLERANCE,
            &ops,
            &mut second_buffers,
            false,
            false,
            &cache,
            &mut second_stats,
        );

        assert_eq!(second_stats.vector_geometry_cache_misses, 0);
        assert_eq!(second_stats.vector_geometry_cache_hits, 2);
        assert_eq!(second_stats.tessellated_vertices, 0);
        assert_eq!(
            buffers.geometry.vertices.len(),
            second_buffers.geometry.vertices.len()
        );
        for (left, right) in buffers
            .geometry
            .vertices
            .iter()
            .zip(second_buffers.geometry.vertices.iter())
        {
            assert_eq!(left.position, right.position);
            assert_eq!(left.prim_id, right.prim_id);
        }
        assert_eq!(buffers.geometry.indices, second_buffers.geometry.indices);
    }
}

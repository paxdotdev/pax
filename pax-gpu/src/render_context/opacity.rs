use super::*;
use crate::render_backend::capture::CaptureKey;
use crate::render_backend::ScissorRect;

pub(super) enum OpacityPlan {
    Draw(Vec<u32>),
    #[cfg(feature = "3d")]
    SurfaceDraw(Vec<(u32, Option<u32>)>),
    Group {
        key: CaptureKey,
        opacity: f32,
        signature: u64,
        bounds: ScissorRect,
        children: Vec<OpacityPlan>,
    },
}

// CPU draw identities only: no render pass or resource borrow crosses a capture
// boundary. At most one kind is populated; spatial occurrences retain their clips.
#[derive(Default)]
struct OpaqueBatch {
    nodes: Vec<u32>,
    #[cfg(feature = "3d")]
    surfaces: Vec<(u32, Option<u32>)>,
}

pub(super) fn collect_opacity_group_keys(plan: &[OpacityPlan], keys: &mut HashSet<CaptureKey>) {
    for entry in plan {
        if let OpacityPlan::Group { key, children, .. } = entry {
            keys.insert(*key);
            collect_opacity_group_keys(children, keys);
        }
    }
}

impl WgpuRenderer<'_> {
    pub(super) fn opacity_plan(&self) -> Vec<OpacityPlan> {
        self.opacity_plan_at_depth(&self.visible_main_nodes(), 0)
    }

    pub(super) fn visible_main_nodes(&self) -> Vec<u32> {
        let viewport = self.viewport_bounds();
        let visible: Vec<_> = self
            .sorted_nodes
            .iter()
            .filter(|(_, id)| {
                #[cfg(feature = "3d")]
                if self.spatial.hidden.contains(id) {
                    return false;
                }
                self.belongs_to_draw_domain(*id)
            })
            .filter_map(|(_, id)| {
                self.scene
                    .get(id)
                    .filter(|node| match node {
                        RetainedNode::Vector(node) => !node.buffers.geometry.indices.is_empty(),
                        RetainedNode::Image(_) => true,
                        #[cfg(feature = "3d")]
                        RetainedNode::Solid(_) => false,
                        #[cfg(feature = "shaders")]
                        RetainedNode::Effect(node) => node.visible,
                    })
                    .filter(|n| n.intersects_bounds(&viewport))
                    .map(|_| *id)
            })
            .collect();
        visible
    }

    pub(super) fn opacity_plan_at_depth(&self, nodes: &[u32], depth: usize) -> Vec<OpacityPlan> {
        self.opacity_plan_in_domain(nodes, depth, &self.render_backend.globals)
    }

    pub(super) fn opacity_plan_in_domain(
        &self,
        nodes: &[u32],
        depth: usize,
        globals: &crate::render_backend::data::GpuGlobals,
    ) -> Vec<OpacityPlan> {
        let viewport = Box2D::new(
            point(globals.origin[0], globals.origin[1]),
            point(
                globals.origin[0] + globals.resolution[0],
                globals.origin[1] + globals.resolution[1],
            ),
        );
        let visible: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|id| self.scene[id].intersects_bounds(&viewport))
            .collect();
        self.opacity_plan_for_draws_in_domain(&visible, depth, None, globals)
    }

    // Spatial draws are already visibility-filtered. Each occurrence can have a different
    // BSP clip, so repeated node IDs must remain distinct in both the plan and cache key.
    pub(super) fn opacity_plan_for_draws(
        &self,
        nodes: &[u32],
        depth: usize,
        spatial_clips: Option<&[Option<(u64, u32)>]>,
    ) -> Vec<OpacityPlan> {
        self.opacity_plan_for_draws_in_domain(
            nodes,
            depth,
            spatial_clips,
            &self.render_backend.globals,
        )
    }

    fn opacity_plan_for_draws_in_domain(
        &self,
        nodes: &[u32],
        depth: usize,
        spatial_clips: Option<&[Option<(u64, u32)>]>,
        globals: &crate::render_backend::data::GpuGlobals,
    ) -> Vec<OpacityPlan> {
        let viewport = Box2D::new(
            point(globals.origin[0], globals.origin[1]),
            point(
                globals.origin[0] + globals.resolution[0],
                globals.origin[1] + globals.resolution[1],
            ),
        );
        debug_assert!(spatial_clips.is_none_or(|clips| clips.len() == nodes.len()));
        let mut plan = Vec::new();
        let mut start = 0;
        while start < nodes.len() {
            let scope = self
                .opacity_scopes
                .get(&nodes[start])
                .and_then(|s| s.get(depth));
            let mut end = start + 1;
            while end < nodes.len()
                && self
                    .opacity_scopes
                    .get(&nodes[end])
                    .and_then(|s| s.get(depth))
                    .map(|s| s.node_id)
                    == scope.map(|s| s.node_id)
            {
                end += 1;
            }
            let run = &nodes[start..end];
            let run_clips = spatial_clips.map(|clips| &clips[start..end]);
            if let Some(scope) = scope {
                let children =
                    self.opacity_plan_for_draws_in_domain(run, depth + 1, run_clips, globals);
                let mut hash = DefaultHasher::new();
                // Lighting, resolution, and clip transforms change cached pixels. The boundary's
                // own alpha intentionally does not: it is applied only when sampling the cache.
                bytemuck::bytes_of(globals).hash(&mut hash);
                #[cfg(feature = "3d")]
                self.spatial
                    .region
                    .map(|(_, signature, _)| signature)
                    .hash(&mut hash);
                // A fragment's clip can move while its retained node stays unchanged.
                // Distinguish the BSP plan from an ordinary unsplit opacity capture too.
                run_clips.is_some().hash(&mut hash);
                if let Some(clips) = run_clips {
                    for clip in clips {
                        clip.map(|(signature, _)| signature).hash(&mut hash);
                    }
                }
                for id in run {
                    self.hash_opacity_content(*id, &mut hash);
                    if let Some(scopes) = self.opacity_scopes.get(id) {
                        for child in scopes.iter().skip(depth + 1) {
                            child.node_id.hash(&mut hash);
                            child.opacity.to_bits().hash(&mut hash);
                        }
                    }
                }
                let bounds = run
                    .iter()
                    .map(|id| self.scene[id].bounds())
                    .reduce(|a, b| a.union(&b))
                    .unwrap();
                // Include edge antialiasing, but never allocate outside this physical tile.
                let bounds = ScissorRect {
                    min_x: (bounds.min.x - 1.0).max(viewport.min.x),
                    min_y: (bounds.min.y - 1.0).max(viewport.min.y),
                    max_x: (bounds.max.x + 1.0).min(viewport.max.x),
                    max_y: (bounds.max.y + 1.0).min(viewport.max.y),
                };
                plan.push(OpacityPlan::Group {
                    // Separate runs preserve ordering for content which escapes its parent's
                    // normal render position (including unclippable descendants).
                    #[cfg(not(feature = "3d"))]
                    key: CaptureKey::Opacity(scope.node_id, run[0]),
                    #[cfg(feature = "3d")]
                    key: if run_clips.is_some() && scope.opacity != 1.0 {
                        // This scope is proven contiguous. Its first BSP fragment can
                        // change on every pose without changing the capture's identity.
                        CaptureKey::SpatialGroup(self.spatial.namespace, scope.node_id)
                    } else {
                        self.spatial.region.map_or(
                            CaptureKey::Opacity(scope.node_id, run[0]),
                            |(region, _, _)| {
                                CaptureKey::SpatialOpacity(
                                    self.spatial.namespace,
                                    scope.node_id,
                                    run[0],
                                    region,
                                )
                            },
                        )
                    },
                    opacity: scope.opacity,
                    signature: hash.finish(),
                    bounds,
                    children,
                });
            } else {
                #[cfg(not(feature = "3d"))]
                plan.push(OpacityPlan::Draw(run.to_vec()));
                #[cfg(feature = "3d")]
                plan.push(if let Some(clips) = run_clips {
                    OpacityPlan::SurfaceDraw(
                        run.iter()
                            .zip(clips)
                            .map(|(id, clip)| (*id, clip.map(|(_, id)| id)))
                            .collect(),
                    )
                } else {
                    OpacityPlan::Draw(run.to_vec())
                });
            }
            start = end;
        }
        plan
    }

    pub(super) fn hash_opacity_content(&self, id: u32, hash: &mut DefaultHasher) {
        id.hash(hash);
        let node = &self.scene[&id];
        // Callers hash draws in paint order. Absolute z values can change when
        // unrelated nodes are inserted without changing this capture's pixels.
        match node {
            #[cfg(feature = "3d")]
            RetainedNode::Solid(n) => {
                n.hash_content(hash);
                LightingDependencies {
                    direct: n.light_mask,
                    lit: !n.material.unlit,
                }
                .signature(&self.scene_lighting)
                .hash(hash);
            }
            #[cfg(feature = "shaders")]
            RetainedNode::Effect(n) => {
                n.visible.hash(hash);
                self.effect_captures[&n.id].signature.hash(hash);
            }
            RetainedNode::Vector(n) => {
                n.lighting_dependencies
                    .signature(&self.scene_lighting)
                    .hash(hash);
                n.resource_key.hash(hash);
                n.transform_signature.hash(hash);
            }
            RetainedNode::Image(n) => {
                n.signature.hash(hash);
                // Another retained draw can refresh a shared image without replaying this node.
                self.cached_images
                    .get(&n.draw.resource.image_key)
                    .map(|image| image.version)
                    .hash(hash);
            }
        }
        #[cfg(feature = "3d")]
        let clips = node
            .clip_stack()
            .iter()
            .chain(self.spatial.solid_clips.get(&id).into_iter().flatten());
        #[cfg(not(feature = "3d"))]
        let clips = node.clip_stack().iter();
        for clip in clips {
            match clip {
                ClipReference::Alpha { owner } => {
                    0u8.hash(hash);
                    self.render_backend
                        .alpha_masks
                        .signature(Some(*owner))
                        .hash(hash);
                }
                ClipReference::Stencil { clip_id } => {
                    1u8.hash(hash);
                    self.clip_arena.entries[clip_id]
                        .geometry_signature
                        .hash(hash);
                    bytemuck::bytes_of(&self.clip_arena.slots[*clip_id as usize]).hash(hash);
                }
                ClipReference::Scissor(rect) => {
                    2u8.hash(hash);
                    for value in [rect.min_x, rect.min_y, rect.max_x, rect.max_y] {
                        value.to_bits().hash(hash);
                    }
                }
            }
        }
    }

    pub(super) fn draw_opacity_plan(&mut self, plan: &[OpacityPlan]) {
        let mut batch = OpaqueBatch::default();
        self.append_opacity_plan(plan, &mut batch);
        self.flush_opaque_batch(&mut batch);
    }

    fn flush_opaque_batch(&mut self, batch: &mut OpaqueBatch) {
        if !batch.nodes.is_empty() {
            #[cfg(test)]
            {
                self.opaque_batch_flushes += 1;
            }
            self.draw_retained_nodes(&batch.nodes);
            batch.nodes.clear();
        }
        #[cfg(feature = "3d")]
        if !batch.surfaces.is_empty() {
            #[cfg(test)]
            {
                self.opaque_batch_flushes += 1;
            }
            self.draw_surface_fragments(
                batch
                    .surfaces
                    .iter()
                    .map(|(id, clip)| (std::slice::from_ref(id), *clip)),
            );
            batch.surfaces.clear();
        }
    }

    fn append_opacity_plan(&mut self, plan: &[OpacityPlan], batch: &mut OpaqueBatch) {
        for entry in plan {
            match entry {
                OpacityPlan::Draw(nodes) => {
                    #[cfg(feature = "3d")]
                    if !batch.surfaces.is_empty() {
                        self.flush_opaque_batch(batch);
                    }
                    batch.nodes.extend_from_slice(nodes);
                }
                #[cfg(feature = "3d")]
                OpacityPlan::SurfaceDraw(draws) => {
                    if !batch.nodes.is_empty() {
                        self.flush_opaque_batch(batch);
                    }
                    batch.surfaces.extend_from_slice(draws);
                }
                OpacityPlan::Group {
                    key,
                    opacity,
                    signature,
                    bounds,
                    children,
                } => {
                    let cached = self.render_backend.capture_is_cached(*key, *signature);
                    // Preserve a useful cached fade at alpha one, but do not
                    // recapture moving opaque content merely because it once faded.
                    if *opacity == 1.0 && !cached {
                        // A no-op scope must not split an otherwise compatible color pass.
                        // Clip changes remain ordered segments inside the retained batch.
                        self.append_opacity_plan(children, batch);
                        continue;
                    }
                    // Flush before entering scratch, even on a cache hit: composition
                    // must stay between the preceding and following sibling draws.
                    self.flush_opaque_batch(batch);
                    if !cached {
                        if !self.render_backend.begin_bounded_capture(*bounds) {
                            continue;
                        }
                        self.draw_opacity_plan(children);
                        self.render_backend.end_capture(*key, *signature, *bounds);
                        self.resource_churn_stats.opacity_group_renders += 1;
                        self.observe_work(
                            match key {
                                CaptureKey::Opacity(owner, _) => Some(*owner),
                                _ => None,
                            },
                            "opacity raster",
                            1,
                            0,
                        );
                    } else {
                        self.resource_churn_stats.opacity_group_cache_hits += 1;
                        self.observe_work(
                            match key {
                                CaptureKey::Opacity(owner, _) => Some(*owner),
                                _ => None,
                            },
                            "opacity reuse",
                            1,
                            0,
                        );
                    }
                    self.render_backend.draw_capture(*key, *opacity);
                }
            }
            // Preserve the former split-pass path as a pixel-equivalence oracle.
            #[cfg(test)]
            if self.split_opaque_batches {
                self.flush_opaque_batch(batch);
            }
        }
    }
}

impl RetainedNode {
    pub(super) fn bounds(&self) -> Box2D {
        match self {
            #[cfg(feature = "3d")]
            Self::Solid(node) => node.bounds,
            Self::Vector(node) => node.bounds,
            Self::Image(node) => node.bounds,
            #[cfg(feature = "shaders")]
            Self::Effect(node) => node.bounds,
        }
    }
}

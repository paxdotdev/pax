use super::*;
use crate::render_backend::capture::CaptureKey;
use crate::render_backend::ScissorRect;

pub(super) enum OpacityPlan {
    Draw(Vec<u32>),
    Group {
        key: CaptureKey,
        opacity: f32,
        signature: u64,
        bounds: ScissorRect,
        children: Vec<OpacityPlan>,
    },
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
        let viewport = self.viewport_bounds();
        let visible: Vec<_> = self
            .sorted_nodes
            .iter()
            .filter(|(_, id)| !self.source_nodes.contains_key(id))
            .filter_map(|(_, id)| {
                self.scene
                    .get(id)
                    .filter(|n| n.intersects_bounds(&viewport))
                    .map(|_| *id)
            })
            .collect();
        self.opacity_plan_at_depth(&visible, 0)
    }

    pub(super) fn opacity_plan_at_depth(&self, nodes: &[u32], depth: usize) -> Vec<OpacityPlan> {
        let visible: Vec<_> = nodes
            .iter()
            .copied()
            .filter(|id| self.scene[id].intersects_bounds(&self.viewport_bounds()))
            .collect();
        let nodes = visible.as_slice();
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
            if let Some(scope) = scope {
                let children = self.opacity_plan_at_depth(run, depth + 1);
                let mut hash = DefaultHasher::new();
                // Lighting, resolution, and clip transforms change cached pixels. The boundary's
                // own alpha intentionally does not: it is applied only when sampling the cache.
                bytemuck::bytes_of(&self.render_backend.globals).hash(&mut hash);
                self.lighting_signature.hash(&mut hash);
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
                let viewport = self.viewport_bounds();
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
                    key: CaptureKey::Opacity(scope.node_id, run[0]),
                    opacity: scope.opacity,
                    signature: hash.finish(),
                    bounds,
                    children,
                });
            } else {
                plan.push(OpacityPlan::Draw(run.to_vec()));
            }
            start = end;
        }
        plan
    }

    pub(super) fn hash_opacity_content(&self, id: u32, hash: &mut DefaultHasher) {
        id.hash(hash);
        let node = &self.scene[&id];
        node.z_index().hash(hash);
        match node {
            RetainedNode::Vector(n) => {
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
        for clip in node.clip_stack() {
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
        for entry in plan {
            match entry {
                OpacityPlan::Draw(nodes) => self.draw_retained_nodes(nodes),
                OpacityPlan::Group {
                    key,
                    opacity,
                    signature,
                    bounds,
                    children,
                } => {
                    // Fully opaque scopes are equivalent to ordinary source-over on this
                    // canvas. Once a fade has allocated a cache, keep it through alpha one.
                    if *opacity == 1.0 && !self.render_backend.capture_is_resident(*key) {
                        self.draw_opacity_plan(children);
                        continue;
                    }
                    if !self.render_backend.capture_is_cached(*key, *signature) {
                        self.render_backend.begin_capture();
                        self.draw_opacity_plan(children);
                        self.render_backend.end_capture(*key, *signature, *bounds);
                        self.resource_churn_stats.opacity_group_renders += 1;
                    } else {
                        self.resource_churn_stats.opacity_group_cache_hits += 1;
                    }
                    self.render_backend.draw_capture(*key, *opacity);
                }
            }
        }
    }
}

impl RetainedNode {
    pub(super) fn bounds(&self) -> Box2D {
        match self {
            Self::Vector(node) => node.bounds,
            Self::Image(node) => node.bounds,
        }
    }
}

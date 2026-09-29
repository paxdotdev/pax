//! Detached vector draws share the ordinary retained renderer and capture surfaces. Their ownership
//! is separate from visible ordering; only alpha-mask references keep a source alive.
use super::*;
use crate::render_backend::capture::CaptureKey;

pub(super) struct SourceCapture {
    nodes: HashSet<u32>,
    feather: f32,
    parent: Option<u32>,
}

pub(super) struct SourceRecording {
    pub owner: u32,
    pub nodes: HashSet<u32>,
    feather: f32,
    transforms: Vec<Transform2D>,
    clips: Vec<ClipReference>,
    saves: Vec<SceneStateSave>,
    consumer: PendingNode,
}

impl WgpuRenderer<'_> {
    /// Records a detached source in the current mask owner's surface coordinates. Source clips and
    /// opacity are independent of the consuming content; its enclosing mask is applied afterward.
    pub fn begin_alpha_source(&mut self, mapping: Transform2D, feather: f32) {
        let consumer = self
            .current_node
            .take()
            .expect("alpha source needs a mask owner");
        let transform = mapping.then(&self.current_transform());
        self.source_recordings.push(SourceRecording {
            owner: consumer.id,
            nodes: HashSet::new(),
            feather: if feather.is_finite() {
                feather.max(0.0)
            } else {
                0.0
            },
            transforms: std::mem::replace(&mut self.transform_stack, vec![transform]),
            clips: std::mem::take(&mut self.clip_stack),
            saves: std::mem::take(&mut self.saves),
            consumer,
        });
    }

    /// Restores the consumer and installs a reference to the source's retained alpha texture.
    pub fn end_alpha_source(&mut self) {
        let recording = self
            .source_recordings
            .pop()
            .expect("balanced source capture");
        assert!(
            self.current_node.is_none(),
            "source leaf must finish recording"
        );
        if let Some(previous) = self.source_captures.remove(&recording.owner) {
            for id in previous.nodes.difference(&recording.nodes) {
                self.remove_node(*id);
            }
        }
        self.transform_stack = recording.transforms;
        self.clip_stack = recording.clips;
        self.saves = recording.saves;
        self.current_node = Some(recording.consumer);
        self.source_captures.insert(
            recording.owner,
            SourceCapture {
                nodes: recording.nodes,
                feather: recording.feather,
                parent: alpha_for_clip_stack(&self.clip_stack),
            },
        );
        self.clip_stack.push(ClipReference::Alpha {
            owner: recording.owner,
        });
        self.scene_dirty = true;
    }

    fn source_dependencies(&self, owner: u32) -> Vec<u32> {
        let Some(source) = self.source_captures.get(&owner) else {
            return Vec::new();
        };
        source
            .parent
            .into_iter()
            .chain(
                source
                    .nodes
                    .iter()
                    .filter_map(|id| self.scene.get(id))
                    .flat_map(|node| node.clip_stack())
                    .filter_map(|clip| match clip {
                        ClipReference::Alpha { owner } => Some(*owner),
                        _ => None,
                    }),
            )
            .collect()
    }

    pub(super) fn retain_reachable_sources(&mut self) -> HashSet<u32> {
        let mut pending: Vec<_> = self
            .scene
            .iter()
            .filter(|(id, _)| !self.source_nodes.contains_key(id))
            .flat_map(|(_, node)| node.clip_stack())
            .filter_map(|clip| match clip {
                ClipReference::Alpha { owner } => Some(*owner),
                _ => None,
            })
            .collect();
        let mut active = HashSet::new();
        while let Some(owner) = pending.pop() {
            if active.insert(owner) {
                pending.extend(self.source_dependencies(owner));
            }
        }
        let removed: Vec<_> = self
            .source_captures
            .keys()
            .filter(|id| !active.contains(id))
            .copied()
            .collect();
        for owner in removed {
            if let Some(source) = self.source_captures.remove(&owner) {
                for id in source.nodes {
                    self.remove_node(id);
                }
            }
        }
        active
    }

    pub(super) fn render_alpha_sources(
        &mut self,
        active_masks: &HashSet<u32>,
    ) -> HashSet<CaptureKey> {
        let mut active = HashSet::new();
        if self.source_captures.is_empty() {
            return active;
        }
        // A common surface-local domain lets nested alpha results compose without resampling.
        // Sum feather supports along dependencies, including source-side nested masks.
        fn margin(renderer: &WgpuRenderer<'_>, owner: u32, visiting: &mut HashSet<u32>) -> f32 {
            if !visiting.insert(owner) {
                return 0.0;
            }
            let own = renderer
                .source_captures
                .get(&owner)
                .map_or(0.0, |s| 3.0 * s.feather);
            let child = renderer
                .source_dependencies(owner)
                .into_iter()
                .map(|id| margin(renderer, id, visiting))
                .fold(0.0, f32::max);
            visiting.remove(&owner);
            own + child
        }
        let mut visited = HashSet::new();
        let mut gutter = 0.0f32;
        for owner in active_masks {
            if !self.source_captures.contains_key(owner) {
                continue;
            }
            let required = margin(self, *owner, &mut HashSet::new());
            if self.render_backend.source_domain_fits(required) {
                gutter = gutter.max(required);
            } else {
                log::error!("alpha source {owner} exceeds GPU texture limits; hiding its content");
                self.render_backend
                    .render_alpha_mask(*owner, 0, &[], 0.0, None);
                visited.insert(*owner);
            }
        }
        if visited.len() == self.source_captures.len() {
            return active;
        }
        assert!(self.render_backend.begin_source_domain(gutter));
        self.ensure_vector_resources_for_immediate_scene();
        let mut visiting = HashSet::new();
        for owner in active_masks {
            self.render_alpha_source(*owner, &mut visited, &mut visiting, &mut active);
        }
        self.render_backend.end_source_domain();
        active
    }

    fn render_alpha_source(
        &mut self,
        owner: u32,
        visited: &mut HashSet<u32>,
        visiting: &mut HashSet<u32>,
        active: &mut HashSet<CaptureKey>,
    ) {
        if visited.contains(&owner) || !self.source_captures.contains_key(&owner) {
            return;
        }
        assert!(visiting.insert(owner), "alpha-source dependency cycle");
        for dependency in self.source_dependencies(owner) {
            self.render_alpha_source(dependency, visited, visiting, active);
        }
        visiting.remove(&owner);
        let source = &self.source_captures[&owner];
        let feather = source.feather;
        let parent = source.parent;
        let mut nodes: Vec<_> = source
            .nodes
            .iter()
            .filter_map(|id| self.scene.get(id).map(|n| (n.z_index(), *id)))
            .collect();
        nodes.sort_unstable();
        let nodes: Vec<_> = nodes.into_iter().map(|(_, id)| id).collect();
        let plan = self.opacity_plan_at_depth(&nodes, 0);
        collect_opacity_group_keys(&plan, active);
        let mut hash = DefaultHasher::new();
        bytemuck::bytes_of(&self.render_backend.globals).hash(&mut hash);
        self.lighting_signature.hash(&mut hash);
        for id in &nodes {
            self.hash_opacity_content(*id, &mut hash);
            for scope in self.opacity_scopes.get(id).into_iter().flatten() {
                scope.node_id.hash(&mut hash);
                scope.opacity.to_bits().hash(&mut hash);
            }
        }
        let content_signature = hash.finish();
        let key = CaptureKey::Alpha(owner);
        active.insert(key);
        if !self
            .render_backend
            .capture_is_cached(key, content_signature)
        {
            self.render_backend.begin_capture();
            self.draw_opacity_plan(&plan);
            let domain = self.viewport_bounds();
            let bounds = nodes
                .iter()
                .map(|id| self.scene[id].bounds())
                .filter(|bounds| boxes_intersect(bounds, &domain))
                .reduce(|a, b| a.union(&b));
            let bounds = bounds.map_or(
                ScissorRect {
                    min_x: domain.min.x,
                    min_y: domain.min.y,
                    max_x: domain.min.x + 1.0 / self.render_backend.globals.dpr[0],
                    max_y: domain.min.y + 1.0 / self.render_backend.globals.dpr[1],
                },
                |bounds| ScissorRect {
                    min_x: (bounds.min.x - 1.0).max(domain.min.x),
                    min_y: (bounds.min.y - 1.0).max(domain.min.y),
                    max_x: (bounds.max.x + 1.0).min(domain.max.x),
                    max_y: (bounds.max.y + 1.0).min(domain.max.y),
                },
            );
            self.render_backend
                .end_capture(key, content_signature, bounds);
            self.resource_churn_stats.alpha_source_renders += 1;
        } else {
            self.resource_churn_stats.alpha_source_cache_hits += 1;
        }
        feather.to_bits().hash(&mut hash);
        self.render_backend
            .alpha_masks
            .signature(parent)
            .hash(&mut hash);
        self.render_backend
            .render_captured_alpha(owner, hash.finish(), key, feather, parent);
        visited.insert(owner);
    }
}

#[cfg(test)]
impl WgpuRenderer<'_> {
    pub(crate) fn alpha_source_resource_counts(&self) -> (usize, usize, usize) {
        (
            self.source_captures.len(),
            self.source_nodes.len(),
            self.render_backend.alpha_capture_count(),
        )
    }
}

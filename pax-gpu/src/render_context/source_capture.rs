//! Detached vector and opaque mesh draws share the ordinary retained renderer and capture surfaces. Their ownership
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
    projection: Option<pax_runtime_api::math::ProjectedPlane>,
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
            projection: self.projection.take(),
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
        self.projection = recording.projection;
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
        fn request_margin(
            renderer: &WgpuRenderer<'_>,
            owner: u32,
            consumer_margin: f32,
            margins: &mut HashMap<u32, f32>,
            visiting: &mut HashSet<u32>,
        ) {
            let Some(source) = renderer.source_captures.get(&owner) else {
                return;
            };
            assert!(visiting.insert(owner), "alpha-source dependency cycle");
            let required = consumer_margin + 3.0 * source.feather;
            if margins.get(&owner).is_none_or(|old| *old < required) {
                margins.insert(owner, required);
                for dependency in renderer.source_dependencies(owner) {
                    request_margin(renderer, dependency, required, margins, visiting);
                }
            }
            visiting.remove(&owner);
        }
        #[allow(unused_mut)] // Shader consumers can extend the mask's sampling domain.
        let mut consumers: HashMap<u32, f32> = active_masks.iter().map(|id| (*id, 0.)).collect();
        #[cfg(feature = "shaders")]
        {
            let effects = self.effect_margins();
            // Only consumers inside an effect need that effect's sampling halo.
            // Detached sources receive demand through the dependency walk below.
            for (id, node) in &self.scene {
                let Some(effect) = self.effect_nodes.get(id) else {
                    continue;
                };
                for clip in node.clip_stack() {
                    if let ClipReference::Alpha { owner } = clip {
                        let margin = consumers.entry(*owner).or_default();
                        *margin = (*margin).max(effects[effect]);
                    }
                }
            }
        }
        let mut margins = HashMap::new();
        for (owner, required) in consumers {
            request_margin(self, owner, required, &mut margins, &mut HashSet::new());
        }
        let mut visited = HashSet::new();
        let mut visiting = HashSet::new();
        for owner in active_masks {
            self.render_alpha_source(*owner, &margins, &mut visited, &mut visiting, &mut active);
        }
        active
    }

    fn render_alpha_source(
        &mut self,
        owner: u32,
        margins: &HashMap<u32, f32>,
        visited: &mut HashSet<u32>,
        visiting: &mut HashSet<u32>,
        active: &mut HashSet<CaptureKey>,
    ) {
        if visited.contains(&owner) || !self.source_captures.contains_key(&owner) {
            return;
        }
        assert!(visiting.insert(owner), "alpha-source dependency cycle");
        for dependency in self.source_dependencies(owner) {
            self.render_alpha_source(dependency, margins, visited, visiting, active);
        }
        visiting.remove(&owner);
        let source = &self.source_captures[&owner];
        let feather = source.feather;
        let parent = source.parent;
        let margin = margins[&owner];
        let Some((globals, _)) = self.render_backend.source_domain_geometry(margin) else {
            log::error!("alpha source {owner} exceeds GPU texture limits; hiding its content");
            self.render_backend.alpha_masks.set_empty(owner);
            visited.insert(owner);
            return;
        };
        let domain = Box2D::new(
            point(globals.origin[0], globals.origin[1]),
            point(
                globals.origin[0] + globals.resolution[0],
                globals.origin[1] + globals.resolution[1],
            ),
        );
        let bounds = source
            .nodes
            .iter()
            .filter_map(|id| self.scene.get(id))
            .map(|node| node.bounds())
            .filter(|bounds| boxes_intersect(bounds, &domain))
            .reduce(|a, b| a.union(&b));
        let Some(bounds) = bounds else {
            self.render_backend.alpha_masks.set_empty(owner);
            visited.insert(owner);
            return;
        };
        let mut nodes: Vec<_> = source
            .nodes
            .iter()
            .filter_map(|id| self.scene.get(id).map(|n| (n.z_index(), *id)))
            .collect();
        nodes.sort_unstable();
        let nodes: Vec<_> = nodes.into_iter().map(|(_, id)| id).collect();
        let mut hash = DefaultHasher::new();
        bytemuck::bytes_of(&globals).hash(&mut hash);
        for id in &nodes {
            self.hash_capture_content(*id, &mut hash);
            for scope in self.opacity_scopes.get(id).into_iter().flatten() {
                scope.node_id.hash(&mut hash);
                scope.opacity.to_bits().hash(&mut hash);
            }
        }
        let content_signature = hash.finish();
        feather.to_bits().hash(&mut hash);
        self.render_backend
            .alpha_masks
            .signature(parent)
            .hash(&mut hash);
        let output_signature = hash.finish();
        let key = CaptureKey::Alpha(owner);
        active.insert(key);
        let cached = self
            .render_backend
            .capture_is_cached(key, content_signature);
        if cached && self.render_backend.alpha_masks.signature(Some(owner)) == output_signature {
            self.retain_capture_scene(key, &nodes, active);
            self.resource_churn_stats.alpha_source_cache_hits += 1;
            self.observe_work(Some(owner), "alpha source reuse", 1, 0);
            visited.insert(owner);
            return;
        }
        assert!(self.render_backend.begin_source_domain(margin));
        self.ensure_vector_resources_for_immediate_scene();
        if !cached {
            let bounds = ScissorRect {
                min_x: (bounds.min.x - 1.0).max(domain.min.x),
                min_y: (bounds.min.y - 1.0).max(domain.min.y),
                max_x: (bounds.max.x + 1.0).min(domain.max.x),
                max_y: (bounds.max.y + 1.0).min(domain.max.y),
            };
            if !self.render_backend.begin_bounded_capture(bounds) {
                self.render_backend.alpha_masks.set_empty(owner);
                self.render_backend.end_source_domain();
                visited.insert(owner);
                return;
            }
            self.draw_capture_scene(key, &nodes, active);
            self.render_backend
                .end_capture(key, content_signature, bounds);
            self.resource_churn_stats.alpha_source_renders += 1;
            self.observe_work(Some(owner), "alpha source raster", 1, 0);
        } else {
            self.retain_capture_scene(key, &nodes, active);
            self.resource_churn_stats.alpha_source_cache_hits += 1;
            self.observe_work(Some(owner), "alpha source reuse", 1, 0);
        }
        self.render_backend
            .render_captured_alpha(owner, output_signature, key, feather, parent);
        self.render_backend.end_source_domain();
        visited.insert(owner);
    }
}

#[cfg(test)]
impl WgpuRenderer<'_> {
    pub(crate) fn has_capture_resources_for_test(&self) -> bool {
        self.render_backend.has_capture_resources_for_test()
    }

    pub(crate) fn alpha_target_size_for_test(&self, id: u32) -> [u32; 2] {
        self.render_backend.alpha_masks.size_for_test(id)
    }

    pub(crate) fn capture_scratch_sizes_for_test(&self) -> Vec<[u32; 2]> {
        self.render_backend.capture_scratch_sizes_for_test()
    }

    pub(crate) fn alpha_source_has_working_storage(&self) -> bool {
        self.render_backend.alpha_source_has_working_storage()
    }

    pub(crate) fn alpha_source_resource_counts(&self) -> (usize, usize, usize) {
        (
            self.source_captures.len(),
            self.source_nodes.len(),
            self.render_backend.alpha_capture_count(),
        )
    }
}

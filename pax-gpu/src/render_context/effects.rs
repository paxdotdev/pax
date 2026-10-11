//! Effect boundaries retain the original scene draws for source caching and native geometry.
use super::*;
use crate::render_backend::capture::CaptureKey;
use pax_runtime_api::ShaderEffect;

pub(super) struct EffectCapture {
    effect: ShaderEffect,
    transform: Transform2D,
    layout: Box2D,
    children: Vec<u32>,
    pub(super) signature: u64,
}
pub(super) struct EffectRecording {
    clips: Vec<ClipReference>,
    saves: Vec<SceneStateSave>,
}
pub(super) struct EffectNode {
    pub(super) id: u32,
    pub(super) visible: bool,
    pub(super) z_index: i32,
    pub(super) bounds: Box2D,
    pub(super) clip_stack: Vec<ClipReference>,
}

impl WgpuRenderer<'_> {
    /// Start a GPU-only filter boundary. A true return requests an initial source replay.
    pub fn begin_shader_effect(
        &mut self,
        id: u32,
        z: i32,
        effect: &ShaderEffect,
        transform: Transform2D,
        size: (f64, f64),
        scopes: &[OpacityScope],
        children: &[u32],
    ) -> bool {
        effect
            .encode_parameters()
            .unwrap_or_else(|e| panic!("shader effect {}: {e}", effect.program.label));
        assert!(
            self.current_node.is_none() && self.source_recordings.is_empty(),
            "shader effects cannot be used as detached mask sources"
        );
        let transform = transform.then(&self.current_transform());
        assert!(
            transform.to_array().iter().all(|v| v.is_finite()),
            "shader effect transform must be finite"
        );
        let layout = transform.outer_transformed_box(&Box2D::new(
            point(0., 0.),
            point(size.0 as f32, size.1 as f32),
        ));
        let replay = self
            .effect_captures
            .get(&id)
            .is_none_or(|old| old.children != children || old.transform != transform);
        let signature = self.effect_captures.get(&id).map_or(0, |e| e.signature);
        self.effect_nodes.retain(|_, owner| *owner != id);
        self.effect_nodes
            .extend(children.iter().map(|child| (*child, id)));
        self.effect_captures.insert(
            id,
            EffectCapture {
                effect: effect.clone(),
                transform,
                layout,
                children: children.to_vec(),
                signature,
            },
        );
        let clips = std::mem::take(&mut self.clip_stack);
        self.scene.insert(
            id,
            RetainedNode::Effect(EffectNode {
                id,
                visible: false,
                z_index: z,
                bounds: layout,
                clip_stack: clips.clone(),
            }),
        );
        self.effect_recordings.push(EffectRecording {
            clips,
            saves: std::mem::take(&mut self.saves),
        });
        self.set_node_opacity_scopes(id, scopes);
        self.order_dirty = true;
        self.scene_dirty = true;
        replay
    }

    pub fn end_shader_effect(&mut self) {
        assert!(
            self.current_node.is_none(),
            "effect descendants must finish their draw"
        );
        let saved = self
            .effect_recordings
            .pop()
            .expect("balanced shader effect");
        self.clip_stack = saved.clips;
        self.saves = saved.saves;
    }

    pub(super) fn effect_margin(&self) -> f32 {
        fn margin(renderer: &WgpuRenderer<'_>, id: u32, visiting: &mut HashSet<u32>) -> f32 {
            assert!(visiting.insert(id), "shader effect dependency cycle");
            let effect = &renderer.effect_captures[&id];
            let t = effect.transform;
            let own = effect.effect.outset as f32
                * (t.m11.abs() + t.m21.abs()).max(t.m12.abs() + t.m22.abs());
            let child = effect
                .children
                .iter()
                .filter(|id| renderer.effect_captures.contains_key(id))
                .map(|id| margin(renderer, *id, visiting))
                .fold(0., f32::max);
            visiting.remove(&id);
            own + child
        }
        let support = self
            .effect_captures
            .keys()
            .map(|id| margin(self, *id, &mut HashSet::new()))
            .fold(0., f32::max);
        if self.effect_captures.is_empty() {
            0.0
        } else {
            support
                + 16.0
                    / self
                        .render_backend
                        .globals
                        .dpr
                        .into_iter()
                        .fold(f32::INFINITY, f32::min)
        }
    }

    pub(super) fn render_effects(&mut self) -> HashSet<CaptureKey> {
        let mut active = HashSet::new();
        if self.effect_captures.is_empty() {
            return active;
        }
        // Reserve a complete coarse texel beyond filter support, then align each capture to
        // a shared physical grid. Adjacent tiles must not independently stretch a coarse pass.
        let margin = self.effect_margin();
        assert!(
            self.render_backend.source_domain_fits(margin),
            "shader effect sampling support exceeds GPU texture limits"
        );
        assert!(self.render_backend.begin_source_domain(margin));
        self.ensure_vector_resources_for_immediate_scene();
        let mut visited = HashSet::new();
        let ids: Vec<_> = self.effect_captures.keys().copied().collect();
        for id in ids {
            self.render_effect(id, &mut visited, &mut active);
        }
        self.render_backend.end_source_domain();
        active
    }

    fn render_effect(
        &mut self,
        id: u32,
        visited: &mut HashSet<u32>,
        active: &mut HashSet<CaptureKey>,
    ) {
        if !visited.insert(id) {
            return;
        }
        let children = self.effect_captures[&id].children.clone();
        for child in &children {
            if self.effect_captures.contains_key(child) {
                self.render_effect(*child, visited, active);
            }
        }
        let capture = &self.effect_captures[&id];
        let effect = capture.effect.clone();
        let transform = capture.transform;
        if transform.inverse().is_none() {
            if let Some(RetainedNode::Effect(node)) = self.scene.get_mut(&id) {
                node.visible = false;
            }
            self.effect_captures.get_mut(&id).unwrap().signature = 0;
            return;
        }
        let mut nodes: Vec<_> = children
            .iter()
            .filter_map(|id| self.scene.get(id).map(|n| (n.z_index(), *id)))
            .collect();
        nodes.sort_unstable();
        let nodes: Vec<_> = nodes.into_iter().map(|(_, id)| id).collect();
        let t = transform;
        let dx = effect.outset as f32 * (t.m11.abs() + t.m21.abs());
        let dy = effect.outset as f32 * (t.m12.abs() + t.m22.abs());
        let mut bounds = nodes
            .iter()
            .map(|id| self.scene[id].bounds())
            .fold(capture.layout, |a, b| a.union(&b));
        bounds.min -= Vector2D::new(dx, dy);
        bounds.max += Vector2D::new(dx, dy);
        if let Some(RetainedNode::Effect(node)) = self.scene.get_mut(&id) {
            node.bounds = bounds;
            node.visible = false;
        }
        let viewport = self.viewport_bounds();
        if !boxes_intersect(&bounds, &viewport) {
            return;
        }
        let divisor = effect
            .program
            .passes
            .iter()
            .map(|p| p.downsample)
            .max()
            .unwrap() as f32;
        let dpr = self.render_backend.globals.dpr;
        let align = |low: f32, high: f32, vlow: f32, vhigh: f32, origin: f32, density: f32| {
            let step = divisor / density;
            let origin = (origin * density).floor() / density;
            let min = ((low - origin) / step)
                .floor()
                .max(((vlow - origin) / step).ceil())
                * step
                + origin;
            let max = ((high - origin) / step)
                .ceil()
                .min(((vhigh - origin) / step).floor())
                * step
                + origin;
            (min, max)
        };
        let (min_x, max_x) = align(
            bounds.min.x,
            bounds.max.x,
            viewport.min.x,
            viewport.max.x,
            transform.m31,
            dpr[0],
        );
        let (min_y, max_y) = align(
            bounds.min.y,
            bounds.max.y,
            viewport.min.y,
            viewport.max.y,
            transform.m32,
            dpr[1],
        );
        if max_x <= min_x || max_y <= min_y {
            return;
        }
        let bounds = ScissorRect {
            min_x,
            min_y,
            max_x,
            max_y,
        };
        let mut hash = DefaultHasher::new();
        bytemuck::bytes_of(&self.render_backend.globals).hash(&mut hash);
        self.lighting_signature.hash(&mut hash);
        for v in [bounds.min_x, bounds.min_y, bounds.max_x, bounds.max_y] {
            v.to_bits().hash(&mut hash);
        }
        for child in &nodes {
            self.hash_opacity_content(*child, &mut hash);
            if let Some(scopes) = self.opacity_scopes.get(child) {
                for scope in scopes {
                    scope.node_id.hash(&mut hash);
                    scope.opacity.to_bits().hash(&mut hash);
                }
            }
        }
        let source_signature = hash.finish();
        let key = CaptureKey::EffectSource(id);
        active.insert(key);
        let plan = self.opacity_plan_at_depth(&nodes, 0);
        collect_opacity_group_keys(&plan, active);
        if !self.render_backend.capture_is_cached(key, source_signature) {
            self.render_backend.begin_capture();
            self.draw_opacity_plan(&plan);
            let origin = self.render_backend.globals.origin;
            let x = ((bounds.min_x - origin[0]) * dpr[0]).round().max(0.0) as u32;
            let y = ((bounds.min_y - origin[1]) * dpr[1]).round().max(0.0) as u32;
            let width = ((bounds.max_x - bounds.min_x) * dpr[0]).round() as u32;
            let height = ((bounds.max_y - bounds.min_y) * dpr[1]).round() as u32;
            self.render_backend
                .end_capture_rect(key, source_signature, (x, y, width, height));
            self.resource_churn_stats.effect_source_renders += 1;
        } else {
            self.resource_churn_stats.effect_source_cache_hits += 1;
        }
        effect.program.hash(&mut hash);
        effect.encode_parameters().unwrap().hash(&mut hash);
        transform.to_array().map(f32::to_bits).hash(&mut hash);
        let signature = hash.finish();
        self.render_backend.render_effect(
            id,
            &effect,
            transform,
            signature,
            &mut self.resource_churn_stats,
        );
        self.effect_captures.get_mut(&id).unwrap().signature = signature;
        if let Some(RetainedNode::Effect(node)) = self.scene.get_mut(&id) {
            node.visible = true;
        }
    }
}

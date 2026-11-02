//! Exact, bounded group fades across geometric interleaving.
//!
//! A scope's opacity weights the scene with that entire scope present/absent.
//! Nested absent scopes prune their descendants. Disjoint projected components
//! are independent, so their view counts add instead of multiplying. Captures
//! contain the backdrop too: they replace their rectangle with a weighted sum.
use super::*;
use crate::render_backend::capture::CaptureKey;

const MAX_VIEWS: usize = 8;

#[derive(Default)]
pub(super) struct SpatialFades {
    scopes: HashSet<u32>,
    captures: HashMap<u64, Vec<CaptureKey>>,
    warned: bool,
}

#[derive(Clone)]
struct Scope {
    id: u32,
    opacity: f64,
    ancestors: Vec<u32>,
    nodes: Vec<u32>,
    bounds: Box2D,
}

struct State {
    hidden: Vec<u32>,
    weight: f64,
}

// Visit parents first. A child's fade is conditional on its parent existing;
// multiplying an absent parent's weight by all its child choices wastes views.
fn states(scopes: &[Scope]) -> Option<Vec<State>> {
    fn visit(
        scopes: &[Scope],
        index: usize,
        absent: &mut HashSet<u32>,
        weight: f64,
        result: &mut Vec<State>,
    ) -> bool {
        if index == scopes.len() {
            let mut hidden: Vec<_> = scopes
                .iter()
                .filter(|s| absent.contains(&s.id))
                .flat_map(|s| s.nodes.iter().copied())
                .collect();
            hidden.sort_unstable();
            hidden.dedup();
            if let Some(old) = result.iter_mut().find(|s| s.hidden == hidden) {
                old.weight += weight;
            } else {
                result.push(State { hidden, weight });
            }
            return result.len() <= MAX_VIEWS;
        }
        let scope = &scopes[index];
        if scope.ancestors.iter().any(|id| absent.contains(id)) {
            absent.insert(scope.id);
            let ok = visit(scopes, index + 1, absent, weight, result);
            absent.remove(&scope.id);
            return ok;
        }
        if scope.opacity < 1. {
            absent.insert(scope.id);
            let ok = visit(
                scopes,
                index + 1,
                absent,
                weight * (1. - scope.opacity),
                result,
            );
            absent.remove(&scope.id);
            if !ok {
                return false;
            }
        }
        scope.opacity == 0. || visit(scopes, index + 1, absent, weight * scope.opacity, result)
    }
    let mut result = Vec::new();
    visit(scopes, 0, &mut HashSet::new(), 1., &mut result).then_some(result)
}

fn components(mut scopes: Vec<Scope>) -> Vec<Vec<Scope>> {
    scopes.sort_by_key(|s| s.id);
    let mut result: Vec<Vec<Scope>> = Vec::new();
    for scope in scopes {
        let mut component = vec![scope];
        let mut index = 0;
        while index < result.len() {
            // Each final component is presented as a rectangular capture. Merge
            // bounding rectangles too, so those replacement quads cannot overlap.
            let bounds = component
                .iter()
                .map(|s| s.bounds)
                .reduce(|a, b| a.union(&b))
                .unwrap();
            let other = result[index]
                .iter()
                .map(|s| s.bounds)
                .reduce(|a, b| a.union(&b))
                .unwrap();
            if boxes_intersect(&bounds, &other) {
                component.extend(result.remove(index));
                index = 0;
            } else {
                index += 1;
            }
        }
        component.sort_by_key(|s| (s.ancestors.len(), s.id));
        result.push(component);
    }
    result.sort_by_key(|c| c.iter().map(|s| s.id).min().unwrap());
    result
}

impl WgpuRenderer<'_> {
    #[cfg(test)]
    pub(crate) fn spatial_fade_capture_count(&self) -> usize {
        self.render_backend.spatial_fade_capture_count()
    }

    pub(super) fn draw_spatial_fades(&mut self, active: &mut HashSet<CaptureKey>) -> bool {
        if self.depth_planes.is_empty()
            && !self.sorted_nodes.iter().any(|(_, id)|
                self.belongs_to_draw_domain(*id) && matches!(&self.scene[id], RetainedNode::Solid(_)))
        {
            self.spatial.fades = Default::default();
            return false;
        }
        if self.spatial.fades.scopes.is_empty() {
            if !self
                .opacity_scopes
                .values()
                .flatten()
                .any(|s| s.opacity != 1.)
            {
                return false;
            }
            let faded_mesh = self.sorted_nodes.iter().any(|(_, id)| {
                self.belongs_to_draw_domain(*id) && matches!(&self.scene[id], RetainedNode::Solid(_))
                    && self
                        .opacity_scopes
                        .get(id)
                        .is_some_and(|s| s.iter().any(|s| s.opacity != 1.))
            });
            if !faded_mesh && self.planar_opacity_is_contiguous() {
                return false;
            }
        }
        let mut nodes = self.visible_main_nodes();
        nodes.extend(self.sorted_nodes.iter().filter(|(_,id)| self.belongs_to_draw_domain(*id)).filter_map(|(_,id)| {
            matches!(&self.scene[id], RetainedNode::Solid(s) if boxes_intersect(&s.bounds, &self.viewport_bounds()))
                .then_some(*id)
        }));
        nodes.sort_unstable();
        let mut fades = std::mem::take(&mut self.spatial.fades);
        let current: HashSet<_> = nodes
            .iter()
            .flat_map(|id| self.opacity_scopes.get(id).into_iter().flatten())
            .map(|s| s.node_id)
            .collect();
        fades.scopes.retain(|id| current.contains(id));
        for scope in nodes
            .iter()
            .flat_map(|id| self.opacity_scopes.get(id).into_iter().flatten())
        {
            if scope.opacity != 1. {
                fades.scopes.insert(scope.node_id);
            }
        }
        if fades.scopes.is_empty() {
            fades.captures.clear();
            self.spatial.fades = fades;
            return false;
        }
        let mut scopes: HashMap<u32, Scope> = HashMap::new();
        for id in &nodes {
            let mut ancestors = Vec::new();
            for scope in self.opacity_scopes.get(id).into_iter().flatten() {
                if !fades.scopes.contains(&scope.node_id) {
                    continue;
                }
                let bounds = self.scene[id].bounds();
                let bounds = Box2D::new(
                    bounds.min - Vector2D::new(1., 1.),
                    bounds.max + Vector2D::new(1., 1.),
                );
                let entry = scopes.entry(scope.node_id).or_insert_with(|| Scope {
                    id: scope.node_id,
                    opacity: scope.opacity as f64,
                    ancestors: ancestors.clone(),
                    nodes: Vec::new(),
                    bounds,
                });
                entry.nodes.push(*id);
                entry.bounds = entry.bounds.union(&bounds);
                ancestors.push(scope.node_id);
            }
        }
        let mut planned = Vec::new();
        let mut base_hidden = HashSet::new();
        let mut live_components = HashSet::new();
        let mut views = 0;
        for component in components(scopes.into_values().collect()) {
            let mut ids: Vec<_> = component.iter().map(|s| s.id).collect();
            ids.sort_unstable();
            let mut hash = DefaultHasher::new();
            ids.hash(&mut hash);
            let component_id = hash.finish();
            live_components.insert(component_id);
            let states = states(&component);
            if let Some(states) = states.as_ref().filter(|s| s.len() == 1) {
                base_hidden.extend(states[0].hidden.iter().copied());
                continue;
            }
            if states.as_ref().is_none_or(|s| views + s.len() > MAX_VIEWS) {
                // An explicit capability boundary, never a per-fragment alpha
                // approximation. Endpoints still work; intermediate GPU fades
                // become opaque in this component. Native opacity stays native.
                self.resource_churn_stats.spatial_fade_overflows += 1;
                if !fades.warned {
                    log::warn!("Spatial group fade exceeds the 8 scene-view budget for this composition domain in the tile; overflowing GPU assemblies remain opaque (zero-opacity scopes remain hidden)");
                    fades.warned = true;
                }
                for scope in component.iter().filter(|s| s.opacity == 0.) {
                    base_hidden.extend(scope.nodes.iter().copied());
                }
                continue;
            }
            let states = states.unwrap();
            views += states.len();
            let members: HashSet<_> = component
                .iter()
                .flat_map(|s| s.nodes.iter().copied())
                .collect();
            base_hidden.extend(members.iter().copied());
            let bounds = component
                .iter()
                .map(|s| s.bounds)
                .reduce(|a, b| a.union(&b))
                .unwrap();
            planned.push((component_id, bounds, members, states));
        }
        fades.captures.retain(|id, _| live_components.contains(id));
        // These scene states carry all scope opacity in their weights. Retained
        // paint alpha stays untouched, including translucent planar fills.
        let original_scopes = std::mem::take(&mut self.opacity_scopes);
        let old_spatial = std::mem::take(&mut self.spatial.clip_keys);
        let old_solid = std::mem::take(&mut self.spatial.solid_clip_keys);
        let mut used_spatial = Vec::new();
        let mut used_solid = Vec::new();
        let mut used_captures = Vec::new();
        self.spatial.hidden = base_hidden.clone();
        let base_namespace = self.spatial.namespace;
        self.draw_spatial_scene_unfaded(active);
        used_spatial.append(&mut self.spatial.clip_keys);
        used_solid.append(&mut self.spatial.solid_clip_keys);
        for (component_id, bounds, members, states) in planned {
            let viewport = self.viewport_bounds();
            let bounds = ScissorRect {
                min_x: bounds.min.x.max(viewport.min.x),
                min_y: bounds.min.y.max(viewport.min.y),
                max_x: bounds.max.x.min(viewport.max.x),
                max_y: bounds.max.y.min(viewport.max.y),
            };
            for (index, state) in states.iter().enumerate() {
                self.spatial.hidden = base_hidden.clone();
                for id in &members {
                    self.spatial.hidden.remove(id);
                }
                self.spatial.hidden.extend(state.hidden.iter().copied());
                let mut hash = DefaultHasher::new();
                state.hidden.hash(&mut hash);
                let key = CaptureKey::SpatialVisibility(base_namespace, component_id, hash.finish());
                used_captures.push(key);
                let history = fades.captures.entry(component_id).or_default();
                history.retain(|old| *old != key);
                history.push(key);
                if history.len() > MAX_VIEWS {
                    history.remove(0);
                }
                self.spatial.solid_clips.clear();
                let mut hash = DefaultHasher::new();
                bytemuck::bytes_of(&self.render_backend.globals).hash(&mut hash);
                for value in [bounds.min_x, bounds.min_y, bounds.max_x, bounds.max_y] {
                    value.to_bits().hash(&mut hash);
                }
                for id in &nodes {
                    if self.spatial.hidden.contains(id) {
                        continue;
                    }
                    self.hash_opacity_content(*id, &mut hash);
                    if let Some(plane) = self.depth_planes.get(id) {
                        for c in plane.coefficients {
                            c.to_bits().hash(&mut hash);
                        }
                        plane.backface.hash(&mut hash);
                    }
                }
                let signature = hash.finish();
                if !self.render_backend.capture_is_cached(key, signature) {
                    let mut namespace = DefaultHasher::new();
                    key.hash(&mut namespace);
                    self.spatial.namespace = namespace.finish();
                    self.render_backend.begin_capture();
                    self.draw_spatial_scene_unfaded(active);
                    self.render_backend.end_capture(key, signature, bounds);
                    self.resource_churn_stats.spatial_fade_renders += 1;
                    used_spatial.append(&mut self.spatial.clip_keys);
                    used_solid.append(&mut self.spatial.solid_clip_keys);
                } else {
                    self.resource_churn_stats.spatial_fade_cache_hits += 1;
                }
                self.render_backend
                    .draw_capture_weighted(key, state.weight as f32, index == 0);
            }
        }
        self.opacity_scopes = original_scopes;
        self.spatial.hidden.clear();
        self.spatial.namespace = base_namespace;
        // Never recycle clip slots between views queued in the same submission.
        let mut unique = HashSet::new();
        used_spatial.retain(|k| unique.insert(*k));
        unique.clear();
        used_solid.retain(|k| unique.insert(*k));
        let expired: Vec<_> = old_spatial
            .into_iter()
            .chain(old_solid)
            .filter(|k| !used_spatial.contains(k) && !used_solid.contains(k))
            .collect();
        self.clip_arena.release_keys(&expired);
        self.spatial.clip_keys = used_spatial;
        self.spatial.solid_clip_keys = used_solid;
        // Cache residency shares the same per-domain tile budget as rendering. Keep
        // current views first, then a few endpoint/previous-state views for reuse.
        let mut resident: HashSet<_> = used_captures.into_iter().collect();
        let mut component_ids: Vec<_> = fades.captures.keys().copied().collect();
        component_ids.sort_unstable();
        for id in component_ids {
            fades.captures.get_mut(&id).unwrap().retain(|key| {
                if resident.contains(key) {
                    true
                } else if resident.len() < MAX_VIEWS {
                    resident.insert(*key);
                    true
                } else {
                    false
                }
            });
        }
        active.extend(resident);
        self.spatial.fades = fades;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scope(id: u32, ancestors: Vec<u32>, nodes: Vec<u32>, x: f32) -> Scope {
        Scope {
            id,
            opacity: 0.5,
            ancestors,
            nodes,
            bounds: Box2D::new(point(x, 0.), point(x + 10., 10.)),
        }
    }
    #[test]
    fn nested_presence_is_conditional_and_equivalent_states_merge() {
        let scopes = [
            scope(1, vec![], vec![10, 20], 0.),
            scope(2, vec![1], vec![20], 0.),
        ];
        let states = states(&scopes).unwrap();
        assert_eq!(states.len(), 3);
        assert_eq!(states.iter().map(|s| s.weight).sum::<f64>(), 1.);
        assert_eq!(states[0].hidden, vec![10, 20]);
        assert_eq!(states[0].weight, 0.5);
        let same = [
            scope(1, vec![], vec![10], 0.),
            scope(2, vec![1], vec![10], 0.),
        ];
        assert_eq!(states_len(&same), 2);
    }
    fn states_len(scopes: &[Scope]) -> usize {
        states(scopes).unwrap().len()
    }
    #[test]
    fn independent_fades_have_an_explicit_state_budget_and_disjoint_components() {
        let scopes: Vec<_> = (0..4)
            .map(|id| scope(id, vec![], vec![id + 10], id as f32 * 20.))
            .collect();
        assert!(states(&scopes).is_none());
        let components = components(scopes);
        assert_eq!(components.len(), 4);
        assert_eq!(components.iter().map(|c| states_len(c)).sum::<usize>(), 8);
    }
}

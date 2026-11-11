//! Reactive facade for input-only scene records. Preparation invokes no event
//! handlers and never drains effects; unsettled queries prepare shapes on demand.
use super::*;
use crate::scene_geometry::{
    input::{InputPlacement, PreparedHit},
    CoverageBounds, InputQueryStats,
};
use kurbo::{Point, Rect};

pub(super) struct InputQuery {
    pub candidates: HashSet<u32>,
    pub unknown: HashSet<u32>,
    pub children: HashMap<u32, Vec<Rc<ExpandedNode>>>,
}

#[cfg(test)]
mod hit_testing_tests;
#[cfg(test)]
mod tests;

impl RuntimeContext {
    /// Invalidate conservative input coverage after non-reactive shape changes.
    /// This does not invalidate paint or run input/event callbacks.
    pub fn mark_node_input_dirty(&self, id: u32) {
        self.scene_geometry.borrow_mut().input.invalidate(id);
    }

    /// Work counters for indexed, unbounded-shape and pending-state queries.
    pub fn input_query_stats(&self) -> InputQueryStats {
        self.scene_geometry.borrow().input.stats
    }

    pub(super) fn prepare_input_query(
        &self,
        root: &Rc<ExpandedNode>,
        point: Point,
    ) -> Option<InputQuery> {
        #[cfg(test)]
        if self.input_query_unindexed.get() {
            return None;
        }
        if self.startup.borrow().is_active() {
            self.scene_geometry.borrow_mut().input.stats.startup_queries += 1;
            return None;
        }
        if self.occlusion_dirty.get()
            || !self.scene_geometry.borrow().is_settled()
            || pax_runtime_api::properties::has_pending_effects()
        {
            self.scene_geometry.borrow_mut().input.stats.pending_queries += 1;
            return None;
        }
        if root.attached.get() == 0 || !point.x.is_finite() || !point.y.is_finite() {
            return None;
        }
        let key = (root.id.to_u32(), root.mount_epoch.get());
        let renderer_is_gpu = self.globals().renderer_is_gpu;
        let rebuild = {
            let scene = self.scene_geometry.borrow();
            scene.input.root != Some(key)
                || scene.input.topology_dirty
                || scene.input.renderer_is_gpu != renderer_is_gpu
        };
        if rebuild {
            self.scene_geometry
                .borrow_mut()
                .input
                .reset(key, renderer_is_gpu);
            let mut stack = vec![(root.clone(), None, None, false)];
            while let Some((node, parent, scroll_parent, exit)) = stack.pop() {
                let id = node.id.to_u32();
                if exit {
                    let mut scene = self.scene_geometry.borrow_mut();
                    let end = scene.input.order.len();
                    scene.input.records.get_mut(&id)?.end = end;
                    continue;
                }
                if node.is_render_source() {
                    continue;
                }
                node.compute_flattened_projected_children();
                let rank = self.scene_geometry.borrow().input.order.len();
                let (local_owner, coverage, hit) = self.prepare_retained_node_hit(&node);
                let scrolls = borrow!(node.instance_node).scrolls_content(&node);
                {
                    let mut scene = self.scene_geometry.borrow_mut();
                    scene.input.order.push(id);
                    if scrolls {
                        scene.input.scrollers.push(id);
                    }
                    scene.input.publish(
                        id,
                        InputPlacement {
                            epoch: node.mount_epoch.get(),
                            parent,
                            rank,
                            end: rank + 1,
                            scroll_parent,
                            local_owner,
                            scrolls,
                            window_space: node.has_perspective_camera(),
                            coverage,
                            hit,
                            unknown: coverage == CoverageBounds::Unbounded,
                        },
                    );
                }
                stack.push((node.clone(), parent, scroll_parent, true));
                let child_scroll_parent = if scrolls { Some(id) } else { scroll_parent };
                stack.extend(
                    node.presentation_children(self)
                        .into_iter()
                        .rev()
                        .map(|child| (child, Some(id), child_scroll_parent, false)),
                );
            }
        } else {
            let mut pending = {
                let mut scene = self.scene_geometry.borrow_mut();
                std::mem::take(&mut scene.input.dirty)
                    .into_iter()
                    .filter_map(|id| scene.input.records.get(&id).map(|r| (r.rank, id)))
                    .collect::<std::collections::BTreeMap<_, _>>()
            };
            while let Some((_, id)) = pending.pop_first() {
                let old = self.scene_geometry.borrow().input.records.get(&id)?.clone();
                let node = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id))?;
                if old.epoch != node.mount_epoch.get() {
                    self.scene_geometry.borrow_mut().input.topology_dirty = true;
                    return None;
                }
                if old.scrolls != borrow!(node.instance_node).scrolls_content(&node) {
                    self.scene_geometry.borrow_mut().input.topology_dirty = true;
                    return None;
                }
                let (local_owner, coverage, hit) = self.prepare_retained_node_hit(&node);
                let mut scene = self.scene_geometry.borrow_mut();
                scene.input.publish(
                    id,
                    InputPlacement {
                        local_owner,
                        window_space: node.has_perspective_camera(),
                        coverage,
                        hit,
                        unknown: coverage == CoverageBounds::Unbounded,
                        ..old
                    },
                );
            }
        }
        // Compose one transform per scroll chart, not one footprint per descendant.
        // The topology order guarantees that an enclosing chart is available first.
        let scrollers = self.scene_geometry.borrow().input.scrollers.clone();
        let mut scrolls = HashMap::new();
        let mut hosts = HashMap::new();
        // A scoped query can begin inside a chart whose owner is outside the
        // queried subtree. Preserve the existing scoped-query convention:
        // world placement is included, but ancestor scroll offsets are not.
        if let Some((owner, _)) = root.retained_native_placement() {
            let owner_node = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(owner))?;
            hosts.insert(
                owner,
                Affine::from(owner_node.presentation_transform_and_bounds.get().transform),
            );
        }
        for id in scrollers {
            let parent = self.scene_geometry.borrow().input.records[&id].scroll_parent;
            let inherited = parent.map_or(Affine::IDENTITY, |id| scrolls[&id]);
            let node = self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id))?;
            scrolls.insert(
                id,
                inherited * self.scroller_content_presentation_transform(&node),
            );
            hosts.insert(
                id,
                scrolls[&id] * Affine::from(node.presentation_transform_and_bounds.get().transform),
            );
        }
        // Reading lazily computed slot/layout properties can enqueue work. Keep
        // the baseline query's settlement behavior, rather than painting a frame.
        if pax_runtime_api::properties::has_pending_effects() || self.occlusion_dirty.get() {
            self.scene_geometry.borrow_mut().input.stats.pending_queries += 1;
            return None;
        }
        let (candidates, unknown, children) = {
            let mut scene = self.scene_geometry.borrow_mut();
            let input = &mut scene.input;
            let candidates = input.candidates(point, &scrolls, &hosts);
            let unknown: HashSet<_> = candidates
                .iter()
                .copied()
                .filter(|id| input.records[id].unknown)
                .collect();
            input.stats.indexed_queries += 1;
            let children = {
                let mut visited = candidates.clone();
                for &id in &candidates {
                    let mut ancestor = input.records[&id].parent;
                    while let Some(id) = ancestor {
                        if !visited.insert(id) {
                            break;
                        }
                        ancestor = input.records[&id].parent;
                    }
                }
                let mut ordered: Vec<_> = visited.into_iter().collect();
                ordered.sort_unstable_by_key(|id| input.records[id].rank);
                let mut children = HashMap::<u32, Vec<u32>>::new();
                for id in ordered {
                    if let Some(parent) = input.records[&id].parent {
                        children.entry(parent).or_default().push(id);
                    }
                }
                children
            };
            (candidates, unknown, children)
        };
        let children = children
            .into_iter()
            .map(|(parent, ids)| {
                Some((
                    parent,
                    ids.into_iter()
                        .map(|id| self.get_expanded_node_by_eid(ExpandedNodeIdentifier(id)))
                        .collect::<Option<Vec<_>>>()?,
                ))
            })
            .collect::<Option<HashMap<_, _>>>()?;
        Some(InputQuery {
            candidates,
            unknown,
            children,
        })
    }

    fn prepare_retained_node_hit(
        &self,
        node: &ExpandedNode,
    ) -> (Option<u32>, CoverageBounds, PreparedHit) {
        let local = borrow!(node.instance_node)
            .geometry_is_scroller_local()
            .then(|| node.retained_native_placement())
            .flatten();
        let (bounds, hit) = self.prepare_node_hit_in_chart(
            node,
            Affine::IDENTITY,
            local.map(|(_, tab)| Affine::from(tab.transform)),
        );
        (local.map(|(id, _)| id), bounds, hit)
    }

    pub(super) fn prepare_node_hit(
        &self,
        node: &ExpandedNode,
        inherited_scroll: Affine,
    ) -> (CoverageBounds, PreparedHit) {
        self.prepare_node_hit_in_chart(node, inherited_scroll, None)
    }

    fn prepare_node_hit_in_chart(
        &self,
        node: &ExpandedNode,
        inherited_scroll: Affine,
        local_transform: Option<Affine>,
    ) -> (CoverageBounds, PreparedHit) {
        let shape = borrow!(node.instance_node).prepare_hit_shape(node);
        #[cfg(feature = "3d")]
        if let pax_runtime_api::HitShape::Mesh(mesh) = &shape {
            let Some(projection) = self.presented_mesh_projection_for_node(node, mesh.transform)
            else {
                return (CoverageBounds::Empty, PreparedHit::Empty);
            };
            let solid = mesh.projected(projection);
            let scroll = if node.has_perspective_camera() {
                Affine::IDENTITY
            } else {
                inherited_scroll
            };
            let bounds = solid
                .coverage_bounds()
                .map(|bounds| scroll.transform_rect_bbox(bounds));
            return (
                CoverageBounds::conservative(bounds),
                PreparedHit::Mesh {
                    solid,
                    inverse: scroll.inverse(),
                },
            );
        }
        // A collapsed orthographic plane has no area even when floating point
        // rotation leaves an almost-invertible affine matrix.
        #[cfg(feature = "3d")]
        if !node.has_perspective_camera()
            && node.presentation_geometry.get().is_spatial()
            && node
                .presentation_geometry
                .get()
                .plane()
                .is_none_or(|plane| plane.normal().z.abs() <= 1e-12)
        {
            return (CoverageBounds::Empty, PreparedHit::Empty);
        }
        let local = shape.local_bounds().unwrap_or(Rect::ZERO);
        if matches!(shape, pax_runtime_api::HitShape::Empty) {
            return (CoverageBounds::Empty, PreparedHit::Empty);
        }
        #[cfg(feature = "3d")]
        if node.has_perspective_camera() {
            let presentation = self.prepared_presentation(node);
            let Some(projection) = presentation.projection() else {
                return (CoverageBounds::Empty, PreparedHit::Empty);
            };
            let corners = [
                local.origin(),
                Point::new(local.x1, local.y0),
                Point::new(local.x1, local.y1),
                Point::new(local.x0, local.y1),
            ];
            let unbounded = corners.iter().any(|p| {
                !p.x.is_finite()
                    || !p.y.is_finite()
                    || projection.mapping().homogeneous(*p)[2] <= 0.
            });
            let coverage = if unbounded {
                CoverageBounds::Unbounded
            } else {
                projection
                    .coverage(local, None)
                    .map(CoverageBounds::Finite)
                    .unwrap_or(CoverageBounds::Empty)
            };
            return (
                coverage,
                PreparedHit::Projected {
                    shape,
                    presentation,
                },
            );
        }
        {
            let world = local_transform.unwrap_or_else(|| {
                inherited_scroll
                    * Affine::from(node.presentation_transform_and_bounds.get().transform)
            });
            let coverage = if [local.x0, local.y0, local.x1, local.y1]
                .iter()
                .all(|v| v.is_finite())
            {
                CoverageBounds::conservative(Some(world.transform_rect_bbox(local)))
            } else {
                CoverageBounds::Unbounded
            };
            (
                coverage,
                PreparedHit::Affine {
                    shape,
                    inverse: world.inverse(),
                },
            )
        }
    }
}

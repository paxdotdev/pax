//! The depth buffer can replace geometric clipping only when every visible
//! participant in a composition band is opaque. Mixed bands keep their ordered
//! painter, including its transparency, capture, and effect semantics.
use super::*;
use crate::render_backend::capture::CaptureKey;

impl WgpuRenderer<'_> {
    pub(super) fn draw_separated_depth_bands(&mut self, active: &mut HashSet<CaptureKey>) -> bool {
        #[cfg(test)]
        if self.force_ordered_spatial {
            return false;
        }
        // An admitted fade retains its scene views through the opaque endpoint,
        // so reversing an opacity-only animation continues to reuse them.
        if self.spatial.capture
            || !self.spatial.hidden.is_empty()
            || self.spatial.fades.has_admitted_scopes(&self.opacity_scopes)
        {
            return false;
        }
        let viewport = self.viewport_bounds();
        let mut nodes = self.visible_main_nodes();
        nodes.extend(self.sorted_nodes.iter().filter_map(|(_, id)| {
            (self.belongs_to_draw_domain(*id)
                && matches!(&self.scene[id], RetainedNode::Solid(s) if boxes_intersect(&s.bounds, &viewport)))
                .then_some(*id)
        }));
        // Splitting a shared fade across bands could expose hidden members or
        // change its blend. Let the existing group compositor own that case.
        if nodes.iter().any(|id| {
            self.opacity_scopes
                .get(id)
                .is_some_and(|scopes| scopes.iter().any(|s| s.opacity != 1.))
        }) {
            return false;
        }
        let mut intervals = Vec::with_capacity(nodes.len());
        for id in nodes {
            let node = &self.scene[&id];
            let range = match node {
                RetainedNode::Solid(s) => s.pose.depth_bounds(),
                _ => planar_depth_range(node, self.depth_planes.get(&id).copied()),
            };
            let Some([low, high]) = range.filter(|r| r.iter().all(|v| v.is_finite())) else {
                return false;
            };
            intervals.push((low, high, id));
        }
        intervals.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.2.cmp(&b.2)));
        let mut bands: Vec<(f64, Vec<u32>)> = Vec::new();
        for (low, high, id) in intervals {
            if let Some((end, ids)) = bands.last_mut().filter(|(end, _)| low <= *end + 1e-7) {
                *end = end.max(high);
                ids.push(id);
            } else {
                bands.push((high, vec![id]));
            }
        }
        if bands.len() < 2
            || !bands.iter().any(|(_, ids)| {
                ids.iter().all(|id| {
                    let node = &self.scene[id];
                    !node
                        .clip_stack()
                        .iter()
                        .any(|c| matches!(c, ClipReference::Alpha { .. }))
                        && match node {
                            RetainedNode::Vector(v) => opaque_vector(&v.buffers),
                            RetainedNode::Solid(_) => true,
                            _ => false,
                        }
                })
            })
        {
            return false;
        }
        self.release_spatial_depth_clips();
        // These are composition contexts, not raster captures. Their strictly
        // separated depth intervals prove global back-to-front order. Mixed
        // bands keep the painter; an opaque band clears and owns hardware depth.
        for (_, mut ids) in bands {
            ids.sort_by_key(|id| (self.scene[id].z_index(), *id));
            let key = CaptureKey::DepthBand(self.spatial.namespace, *ids.iter().min().unwrap());
            active.insert(key);
            self.draw_capture_scene(key, &ids, active);
        }
        true
    }

    fn release_spatial_depth_clips(&mut self) {
        self.clip_arena
            .release_keys(&std::mem::take(&mut self.spatial.clip_keys));
        self.clip_arena
            .release_keys(&std::mem::take(&mut self.spatial.solid_clip_keys));
        self.spatial.solid_clip_plans.clear();
        self.spatial.solid_clips.clear();
        self.spatial.region = None;
        self.spatial.fades = Default::default();
    }

    pub(super) fn try_draw_opaque_spatial_scene(&mut self) -> bool {
        #[cfg(test)]
        if self.force_ordered_spatial {
            return false;
        }
        // An admitted fade retains its scene views through the opaque endpoint,
        // so reversing an opacity-only animation continues to reuse them.
        if self.spatial.capture
            || !self.spatial.hidden.is_empty()
            || self.spatial.fades.has_admitted_scopes(&self.opacity_scopes)
        {
            return false;
        }
        let mut nodes = self.visible_main_nodes();
        let viewport = self.viewport_bounds();
        let solids: Vec<_> = self.sorted_nodes.iter().filter_map(|(_, id)| {
            (self.belongs_to_draw_domain(*id)
                && matches!(&self.scene[id], RetainedNode::Solid(s) if boxes_intersect(&s.bounds, &viewport)))
                .then_some(*id)
        }).collect();
        if solids.is_empty() && !nodes.iter().any(|id| self.depth_planes.contains_key(id)) {
            return false;
        }
        for id in nodes.iter().chain(&solids) {
            let node = &self.scene[id];
            let reason = if node
                .clip_stack()
                .iter()
                .any(|c| matches!(c, ClipReference::Alpha { .. }))
            {
                Some("ordered depth: alpha mask")
            } else if self
                .opacity_scopes
                .get(id)
                .is_some_and(|scopes| scopes.iter().any(|s| s.opacity != 1.))
            {
                Some("ordered depth: opacity scope")
            } else {
                match node {
                    RetainedNode::Vector(v) if opaque_vector(&v.buffers) => None,
                    RetainedNode::Solid(_) => None,
                    RetainedNode::Vector(_) => Some("ordered depth: paint"),
                    _ => Some("ordered depth: image or effect"),
                }
            };
            if let Some(reason) = reason {
                self.observe_work(Some(*id), reason, 1, 0);
                return false;
            }
        }

        let node_planes: Vec<_> = nodes
            .iter()
            .map(|id| self.depth_planes.get(id).copied().unwrap_or_default())
            .collect();
        // Use the painter's definition of coplanarity, including tiny numerical
        // differences in separately projected faces of the same authored plane.
        let (planes, membership) =
            spatial::unique_planes(&node_planes, &spatial::box_polygon(viewport));
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for (i, id) in nodes.iter().enumerate() {
            let Some([low, high]) = planar_depth_range(
                &self.scene[id],
                self.depth_planes.get(id).map(|_| planes[membership[i]]),
            ) else {
                return false;
            };
            min = min.min(low);
            max = max.max(high);
        }
        for id in &solids {
            if let RetainedNode::Solid(s) = &self.scene[id] {
                if let Some([low, high]) = s.pose.depth_bounds() {
                    min = min.min(low);
                    max = max.max(high);
                }
            }
        }
        min -= 1.;
        max += 1.;
        if !(min as f32).is_finite() || !(max as f32).is_finite() || max <= min {
            return false;
        }
        // Match the mesh shader's f32 range arithmetic, so intersections agree.
        let inverse_span = (1. / (max - min)) as f32;
        let parameters: Vec<[[f32; 4]; 2]> = membership
            .iter()
            .enumerate()
            .map(|(node, i)| {
                let [a, b, c] = planes[*i].coefficients;
                let homogeneous = if self.depth_planes.contains_key(&nodes[node])
                    && matches!(&self.scene[&nodes[node]], RetainedNode::Vector(v) if v.projected_depth_range.is_some()) {
                    // h.z already carries local geometry's homogeneous depth.
                    // Only apply the small coplanarity adjustment, never recover
                    // depth by subtracting huge screen-space plane terms.
                    let [own_a, own_b, own_c] = node_planes[node].coefficients;
                    [
                        -(a - own_a) as f32 * inverse_span,
                        -(b - own_b) as f32 * inverse_span,
                        (max as f32 - (c - own_c) as f32) * inverse_span,
                        -inverse_span,
                    ]
                } else {
                    [
                        -a as f32 * inverse_span,
                        -b as f32 * inverse_span,
                        (max as f32 - c as f32) * inverse_span,
                        0.,
                    ]
                };
                // Recenter in f64 before upload. Raster sample coordinates are
                // tile-local, so a distant scroller origin cannot consume f32
                // precision. Every coplanar member gets identical coefficients.
                let sample_plane = [
                    (-a * inverse_span as f64) as f32,
                    (-b * inverse_span as f64) as f32,
                    ((max - planes[*i].depth_at(viewport.min.x as f64, viewport.min.y as f64))
                        * inverse_span as f64) as f32,
                    0.,
                ];
                [homogeneous, sample_plane]
            })
            .collect();
        if parameters
            .iter()
            .flatten()
            .flatten()
            .any(|v| !v.is_finite())
        {
            return false;
        }

        self.release_spatial_depth_clips();
        self.prepare_solid_resources(&solids, min, max);
        let visible: HashSet<_> = nodes.iter().copied().collect();
        self.spatial
            .opaque_depth
            .retain(|id, _| visible.contains(id));
        for (id, plane) in nodes.iter().zip(parameters) {
            let RetainedNode::Vector(vector) = &self.scene[id] else {
                unreachable!()
            };
            let Some(vector) = &vector.resource else {
                continue;
            };
            let mut resource = self.spatial.opaque_depth.remove(id);
            self.resource_churn_stats.opaque_depth_upload_bytes += self
                .render_backend
                .prepare_opaque_depth(vector, &mut resource, plane);
            self.spatial.opaque_depth.insert(*id, resource.unwrap());
        }
        // LessEqual preserves the painter's front/back authored tie ordering.
        // Solids precede planar paint at equal depth, as in the ordered path.
        nodes.sort_by_key(|id| {
            self.depth_planes
                .get(id)
                .copied()
                .unwrap_or_default()
                .coplanar_order(self.scene[id].z_index() as i64)
        });
        let mut draws = solids;
        draws.extend(nodes);
        self.render_backend.clear_solid_depth();
        self.spatial.opaque_depth_active = true;
        self.draw_retained_nodes(&draws);
        self.spatial.opaque_depth_active = false;
        true
    }
}

fn opaque_vector(buffers: &CpuBuffers) -> bool {
    buffers.primitives.iter().all(|p| {
        buffers
            .transforms
            .get(p.transform_id as usize)
            .is_some_and(|t| t.opacity == 1.)
            && p.fill_type_flag == 0
            && buffers
                .colors
                .get(p.fill_id as usize)
                .is_some_and(|c| c.color[3] == 1.)
    })
}

// A screen-aligned box extrapolates near-edge-on planes to false depths.
// Retained projected coverage encloses their actual sample support instead.
fn planar_depth_range(
    node: &RetainedNode,
    plane: Option<pax_runtime_api::DepthPlane>,
) -> Option<[f64; 2]> {
    // A projection alone is also used by painter-ordered images/mask sources.
    // Only an explicit depth participant may change their composition order.
    if plane.is_some() {
        if let RetainedNode::Vector(v) = node {
            if let Some(range) = v.projected_depth_range {
                return Some(range);
            }
        }
    }
    let plane = plane.unwrap_or_default();
    let fallback;
    let points = match node {
        RetainedNode::Vector(v) => &v.spatial_coverage,
        _ => {
            fallback = spatial::box_polygon(node.bounds());
            &fallback
        }
    };
    let mut range = [f64::INFINITY, f64::NEG_INFINITY];
    for [x, y] in points {
        let depth = plane.depth_at(*x, *y);
        range[0] = range[0].min(depth);
        range[1] = range[1].max(depth);
    }
    range.iter().all(|v| v.is_finite()).then_some(range)
}

// Evaluate Z/W at clipped local corners before any screen-space inverse or f32
// placement rounding. Edge-on XY mappings still have a finite, useful Z range.
pub(super) fn projected_depth_range(ops: &[PendingVectorOp]) -> Option<[f64; 2]> {
    let mut range = [f64::INFINITY, f64::NEG_INFINITY];
    for op in ops {
        let plane = op.projection?;
        let Some(mut bounds) = path_control_bounds(&op.path) else {
            continue;
        };
        if let PendingVectorOpKind::Stroke(width, _, _) = op.kind {
            bounds = expand_box(bounds, width * 0.5);
        }
        let points = spatial::box_polygon(bounds)
            .into_iter()
            .map(|[x, y]| kurbo::Point::new(x, y))
            .collect();
        let m = plane.matrix().coeffs();
        for point in plane.clip_polygon(points, None) {
            let depth = (m[2] * point.x + m[6] * point.y + m[14])
                / (m[3] * point.x + m[7] * point.y + m[15]);
            if !depth.is_finite() {
                return None;
            }
            range[0] = range[0].min(depth);
            range[1] = range[1].max(depth);
        }
    }
    range.iter().all(|v| v.is_finite()).then_some(range)
}

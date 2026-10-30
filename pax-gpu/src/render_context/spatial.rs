//! Orthographic flat-surface composition. A projected intersection can change paint order
//! within one primitive, so sorting whole nodes (or writing opaque depth) is insufficient.
//! Split surfaces at depth-equality lines, then paint fragments in source-over order.
//! Translucent group scopes retain the region compositor's contiguous-run semantics.
use super::*;
use crate::render_backend::capture::CaptureKey;
use crate::render_backend::RetainedBatchSegment;
use pax_runtime_api::DepthPlane;

const PLANE_EPSILON: f64 = 1.0e-7;
type Polygon = Vec<[f64; 2]>;

impl WgpuRenderer<'_> {
    /// Updates an orthographic plane independently of retained paint resources.
    pub fn set_node_depth_plane(&mut self, node_id: u32, plane: Option<DepthPlane>) {
        // The runtime supplies canvas-content coordinates; physical tiles can add a base
        // transform before retaining geometry. Put the depth equation in that same space.
        let plane = plane.and_then(|mut plane| {
            let inverse = self.transform_stack[0].inverse()?.to_array();
            let [a, b, c] = plane.coefficients;
            plane.coefficients = [
                a * inverse[0] as f64 + b * inverse[1] as f64,
                a * inverse[2] as f64 + b * inverse[3] as f64,
                a * inverse[4] as f64 + b * inverse[5] as f64 + c,
            ];
            plane
                .coefficients
                .iter()
                .all(|v| v.is_finite())
                .then_some(plane)
        });
        if self.depth_planes.get(&node_id).copied() != plane {
            if let Some(plane) = plane {
                self.depth_planes.insert(node_id, plane);
            } else {
                self.depth_planes.remove(&node_id);
            }
            self.scene_dirty = true;
        }
    }

    pub(super) fn draw_spatial_scene(&mut self, active: &mut HashSet<CaptureKey>) {
        if self.depth_planes.is_empty() {
            self.clip_arena
                .release_keys(&std::mem::take(&mut self.spatial_clip_keys));
            let plan = self.opacity_plan();
            collect_opacity_group_keys(&plan, active);
            self.draw_opacity_plan(&plan);
            return;
        }
        let nodes = self.visible_main_nodes();
        let viewport = self.viewport_bounds();
        let polygon = vec![
            [viewport.min.x as f64, viewport.min.y as f64],
            [viewport.max.x as f64, viewport.min.y as f64],
            [viewport.max.x as f64, viewport.max.y as f64],
            [viewport.min.x as f64, viewport.max.y as f64],
        ];
        let node_planes: Vec<_> = nodes
            .iter()
            .map(|id| self.depth_planes.get(id).copied().unwrap_or_default())
            .collect();
        let (planes, membership) = unique_planes(&node_planes, &polygon);
        let bounds: Vec<_> = nodes
            .iter()
            .map(|node| expand_box(self.scene[node].bounds(), 1.0))
            .collect();
        // Coplanar 2D content shares a coverage bound rather than making one
        // partition for every ordinary rectangle in a large UI subtree.
        let mut plane_bounds: Vec<Option<Box2D>> = vec![None; planes.len()];
        for (index, bound) in bounds.iter().enumerate() {
            let combined = &mut plane_bounds[membership[index]];
            *combined = Some(combined.map(|old| old.union(bound)).unwrap_or(*bound));
        }
        let plane_bounds: Vec<_> = plane_bounds.into_iter().map(Option::unwrap).collect();
        // At unit group opacity, a BSP painter can split each surface instead
        // of replaying every overlapping surface in each ordering region.
        // Keep the region path for group alpha: its contiguous-run semantics
        // are defined per projected region, not by a global painter traversal.
        let surface_painter = self
            .opacity_scopes
            .values()
            .flatten()
            .all(|scope| scope.opacity == 1.0);
        let groups = if surface_painter {
            surface_paint_groups(
                &planes,
                &plane_bounds,
                &nodes,
                &membership,
                &bounds,
                viewport,
                |i| node_planes[i].coplanar_order(self.scene[&nodes[i]].z_index() as i64),
            )
        } else {
            let cells = coverage_cells(viewport, &plane_bounds);
            // Disjoint cells with the same visible paint order can share a single
            // stencil union. Creating a GPU clip for every mathematical cell made
            // even a few nested solids allocate thousands of tiny buffers per frame.
            let mut groups: Vec<PaintGroup> = Vec::new();
            let mut group_indices: HashMap<Vec<u32>, usize> = HashMap::new();
            for (cell, candidates) in cells {
                if candidates.is_empty() {
                    continue;
                }
                let local_planes: Vec<_> = candidates.iter().map(|&i| planes[i]).collect();
                for polygon in partition_overlaps(&local_planes, box_polygon(cell), |_, _| true) {
                    let center = centroid(&polygon);
                    let region_bounds = polygon_bounds(&polygon);
                    let mut order: Vec<_> = (0..nodes.len())
                        .filter(|&i| {
                            candidates.contains(&membership[i])
                                && boxes_intersect(&bounds[i], &region_bounds)
                        })
                        .collect();
                    order.sort_by(|&a, &b| {
                        node_planes[a].presentation_layer.cmp(&node_planes[b].presentation_layer)
                            .then_with(|| planes[membership[a]].depth_at(center[0], center[1]).total_cmp(&planes[membership[b]].depth_at(center[0], center[1])))
                            .then_with(|| {
                                let order = |i: usize| {
                                    node_planes[i]
                                        .coplanar_order(self.scene[&nodes[i]].z_index() as i64)
                                };
                                order(a).cmp(&order(b))
                            })
                            .then_with(|| nodes[a].cmp(&nodes[b]))
                    });
                    let ordered: Vec<_> = order.into_iter().map(|i| nodes[i]).collect();
                    if ordered.is_empty() {
                        continue;
                    }
                    // Bound each union well below the stencil tessellator's u16 indices.
                    let group = group_indices
                        .get(&ordered)
                        .copied()
                        .filter(|&index| groups[index].vertices + polygon.len() <= 8192)
                        .unwrap_or_else(|| {
                            let index = groups.len();
                            group_indices.insert(ordered.clone(), index);
                            groups.push(PaintGroup {
                                nodes: ordered,
                                polygons: Vec::new(),
                                vertices: 0,
                                needs_clip: true,
                            });
                            index
                        });
                    groups[group].vertices += polygon.len();
                    groups[group].polygons.push(polygon);
                }
            }
            groups
        };
        self.resource_churn_stats.spatial_fragments += groups.len() as u64;
        self.resource_churn_stats.spatial_clip_fragments +=
            groups.iter().filter(|group| group.needs_clip).count() as u64;
        let mut clips = Vec::with_capacity(groups.len());
        // Upload all clip transforms before encoding any draws: changing a uniform slot
        // partway through the frame would also change earlier queued draws using that slot.
        for (index, group) in groups.iter().enumerate() {
            if !group.needs_clip {
                clips.push(None);
                continue;
            }
            let polygons = &group.polygons;
            let mut path = Path::builder();
            for polygon in polygons {
                path.begin(point(polygon[0][0] as f32, polygon[0][1] as f32));
                for p in &polygon[1..] {
                    path.line_to(point(p[0] as f32, p[1] as f32));
                }
                path.close();
            }
            let geometry = tessellate_clip_geometry(&path.build(), self.tolerance);
            let signature = hash_clip_geometry(&geometry);
            let (key, id) = self
                .clip_arena
                .sync_key(
                    ClipArenaKey::SpatialRegion(index),
                    signature,
                    geometry,
                    Transform2D::identity(),
                )
                .unwrap();
            clips.push(Some((key, signature, id)));
        }
        let expired: Vec<_> = self
            .spatial_clip_keys
            .iter()
            .copied()
            .filter(|key| !clips.iter().flatten().any(|(new, _, _)| new == key))
            .collect();
        self.clip_arena.release_keys(&expired);
        self.spatial_clip_keys = clips.iter().flatten().map(|(key, _, _)| *key).collect();
        self.clip_arena.flush_to_gpu(&mut self.render_backend);
    fn coverage_cells_preserve_area_and_exact_candidate_membership() {
        let viewport = Box2D::new(point(0.0, 0.0), point(100.0, 100.0));
        let bounds = [
            Box2D::new(point(-10.0, -10.0), point(120.0, 120.0)),
            Box2D::new(point(10.0, 20.0), point(45.0, 70.0)),
            Box2D::new(point(30.0, 40.0), point(80.0, 95.0)),
            Box2D::new(point(90.0, 0.0), point(110.0, 10.0)),
        ];
        let cells = coverage_cells(viewport, &bounds);
        let mut area = 0.0;
        for (cell, candidates) in &cells {
            area += cell.width() * cell.height();
            for dx in [0.01, 0.5, 0.99] {
                for dy in [0.01, 0.5, 0.99] {
                    let sample = point(
                        cell.min.x + dx * cell.width(),
                        cell.min.y + dy * cell.height(),
                    );
                    let expected: Vec<_> = bounds
                        .iter()
                        .enumerate()
                        .filter_map(|(i, b)| b.contains(sample).then_some(i))
                        .collect();
                    assert_eq!(*candidates, expected);
                }
            }
        }
        assert_eq!(area, 10000.0);
        for (index, (a, _)) in cells.iter().enumerate() {
            for (b, _) in &cells[..index] {
                assert!(a.intersection(b).is_none());
            }
        }

//! Orthographic flat-surface composition. A projected intersection can change paint order
//! within one primitive, so sorting whole nodes (or writing opaque depth) is insufficient.
//! Split surfaces at depth-equality lines, then paint fragments in source-over order.
//! Uninterrupted translucent groups share one capture; interleaved scopes retain the
//! region compositor's contiguous-run semantics.
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
        self.prepare_and_draw_solids();
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
        let mut plane_coverage = vec![Vec::new(); planes.len()];
        for (index, bound) in bounds.iter().enumerate() {
            let combined = &mut plane_bounds[membership[index]];
            *combined = Some(combined.map(|old| old.union(bound)).unwrap_or(*bound));
            let coverage = &mut plane_coverage[membership[index]];
            match &self.scene[&nodes[index]] {
                RetainedNode::Vector(node) => coverage.extend_from_slice(&node.spatial_coverage),
                _ => coverage.extend(box_polygon(*bound)),
            }
        }
        let plane_bounds: Vec<_> = plane_bounds.into_iter().map(Option::unwrap).collect();
        let plane_coverage: Vec<_> = plane_coverage.into_iter().map(convex_hull).collect();
        let surface_groups = surface_paint_groups(
            &planes,
            &plane_bounds,
            &plane_coverage,
            &nodes,
            &membership,
            &bounds,
            viewport,
            |i| node_planes[i].coplanar_order(self.scene[&nodes[i]].z_index() as i64),
        );
        // Contiguity is a conservative proof that capturing a group cannot move it
        // across an outside object. Interleaved groups still need per-region runs.
        let surface_painter = opacity_scopes_are_contiguous(
            surface_groups
                .iter()
                .flat_map(|group| group.nodes.iter().copied()),
            &self.opacity_scopes,
        );
        // A BSP splits against infinite planes. A distant, non-overlapping panel
        // can therefore appear between two fragments of the same opacity scope.
        // Preserve every overlapping pair's order, but allow disjoint fragments
        // to commute before deciding that a scope genuinely needs region captures.
        let regrouped = (!surface_painter)
            .then(|| regroup_disjoint_scopes(&surface_groups, &self.opacity_scopes))
            .flatten();
        let surface_painter = surface_painter || regrouped.is_some();
        let surface_groups = regrouped.unwrap_or(surface_groups);
        let groups = if surface_painter {
            surface_groups
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
                        node_planes[a]
                            .presentation_layer
                            .cmp(&node_planes[b].presentation_layer)
                            .then_with(|| {
                                planes[membership[a]]
                                    .depth_at(center[0], center[1])
                                    .total_cmp(
                                        &planes[membership[b]].depth_at(center[0], center[1]),
                                    )
                            })
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
        if surface_painter {
            if nodes.iter().all(|id| {
                self.opacity_scopes
                    .get(id)
                    .is_none_or(|scopes| scopes.iter().all(|scope| scope.opacity == 1.0))
            }) {
                self.draw_surface_fragments(groups.iter().enumerate().map(|(index, group)| {
                    (group.nodes.as_slice(), clips[index].map(|(_, _, id)| id))
                }));
            } else {
                let (ordered, spatial_clips): (Vec<_>, Vec<_>) = groups
                    .iter()
                    .enumerate()
                    .flat_map(|(index, group)| {
                        let clip = clips[index].map(|(_, signature, id)| (signature, id));
                        group.nodes.iter().map(move |id| (*id, clip))
                    })
                    .unzip();
                let plan = self.opacity_plan_for_draws(&ordered, 0, Some(&spatial_clips));
                collect_opacity_group_keys(&plan, active);
                self.draw_opacity_plan(&plan);
            }
        } else {
            for (index, group) in groups.iter().enumerate() {
                self.spatial_region = clips[index].map(|(_, signature, id)| (index, signature, id));
                let plan = self.opacity_plan_at_depth(&group.nodes, 0);
                collect_opacity_group_keys(&plan, active);
                self.draw_opacity_plan(&plan);
            }
        }
        self.spatial_region = None;
    }

    pub(super) fn draw_surface_fragments<'a>(
        &mut self,
        groups: impl Iterator<Item = (&'a [u32], Option<u32>)>,
    ) {
        let mut current_clips = Vec::new();
        let mut segments = Vec::new();
        for (nodes, spatial_clip) in groups {
            for id in nodes {
                let node = &self.scene[id];
                let draw = match node {
                    RetainedNode::Solid(_) => continue,
                    RetainedNode::Vector(node) => {
                        let Some(resource) = node.resource.as_ref().filter(|r| r.is_drawable())
                        else {
                            continue;
                        };
                        RetainedDraw::Vector(resource)
                    }
                    RetainedNode::Image(node) => {
                        let Some(texture) = self.cached_images.get(&node.draw.resource.image_key)
                        else {
                            continue;
                        };
                        RetainedDraw::Image {
                            texture: &texture.texture,
                            draw: &node.draw,
                        }
                    }
                    #[cfg(feature = "shaders")]
                    RetainedNode::Effect(node) => RetainedDraw::Effect(node.id),
                };
                let mut clips = node.clip_stack().to_vec();
                clips.extend(self.solid_clips.get(id).into_iter().flatten().copied());
                if let Some(clip_id) = spatial_clip {
                    clips.push(ClipReference::Stencil { clip_id });
                }
                let (prefix, pushes) = clip_stack_sync(&current_clips, &clips, &self.clip_arena);
                update_current_clip_stack(&mut current_clips, prefix, &pushes);
                let draws = vec![draw];
                self.resource_churn_stats.retained_nodes_visible += 1;
                record_retained_batch_stats(&mut self.resource_churn_stats, &draws);
                segments.push(RetainedBatchSegment {
                    run: RetainedBatchRun {
                        alpha_mask: alpha_for_clip_stack(&clips),
                        stencil_index: current_clips.len() as u32,
                        scissor: scissor_for_clip_stack(&clips),
                        draws,
                    },
                    stencil_depth: prefix as u32,
                    clips: pushes,
                });
            }
        }
        self.resource_churn_stats.spatial_paint_passes +=
            self.render_backend.draw_retained_segments(segments);
    }
}

fn opacity_scopes_are_contiguous(
    nodes: impl Iterator<Item = u32>,
    scopes: &HashMap<u32, Vec<OpacityScope>>,
) -> bool {
    let mut last_seen = HashMap::new();
    for (index, node) in nodes.enumerate() {
        for scope in scopes.get(&node).into_iter().flatten() {
            if scope.opacity != 1.0 {
                if let Some(previous) = last_seen.insert(scope.node_id, index) {
                    if previous + 1 != index {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// Find an order with contiguous opacity scopes without changing any overlapping
/// fragments' painter order. Constraints are lifted to the scopes' common parent:
/// a cycle means an outside object really must remain between group members (or
/// conservative coverage cannot prove otherwise), so the region path is retained.
fn regroup_disjoint_scopes(
    groups: &[PaintGroup],
    scopes: &HashMap<u32, Vec<OpacityScope>>,
) -> Option<Vec<PaintGroup>> {
    #[derive(Default)]
    struct OrderNode {
        children: Vec<usize>,
        edges: HashSet<(usize, usize)>,
        leaf: Option<(usize, u32)>,
    }
    let mut tree = vec![OrderNode::default()];
    let mut scope_nodes = HashMap::new();
    let mut leaves = Vec::new();
    for (group_index, group) in groups.iter().enumerate() {
        let coverage = group
            .polygons
            .iter()
            .map(|p| polygon_bounds(p))
            .reduce(|a, b| a.union(&b))?;
        for &node in &group.nodes {
            let mut path = vec![0];
            for scope in scopes
                .get(&node)
                .into_iter()
                .flatten()
                .filter(|s| s.opacity != 1.0)
            {
                let parent = *path.last().unwrap();
                let (scope_parent, index) =
                    *scope_nodes.entry(scope.node_id).or_insert_with(|| {
                        let index = tree.len();
                        tree.push(OrderNode::default());
                        tree[parent].children.push(index);
                        (parent, index)
                    });
                if scope_parent != parent {
                    return None;
                }
                path.push(index);
            }
            let parent = *path.last().unwrap();
            let index = tree.len();
            tree.push(OrderNode {
                leaf: Some((group_index, node)),
                ..Default::default()
            });
            tree[parent].children.push(index);
            path.push(index);
            leaves.push((path, expand_box(coverage, 1.0)));
        }
    }
    // Bounding boxes deliberately overestimate coverage. False dependencies may
    // retain the slow path; omitting a real overlap would change rendered pixels.
    for (i, (a, a_bounds)) in leaves.iter().enumerate() {
        for (b, b_bounds) in &leaves[i + 1..] {
            if !boxes_intersect(a_bounds, b_bounds) {
                continue;
            }
            let split = a.iter().zip(b).position(|(a, b)| a != b).unwrap();
            tree[a[split - 1]].edges.insert((a[split], b[split]));
        }
    }
    fn emit(index: usize, tree: &[OrderNode], out: &mut Vec<(usize, u32)>) -> Option<()> {
        let node = &tree[index];
        if let Some(leaf) = node.leaf {
            out.push(leaf);
            return Some(());
        }
        let mut incoming: HashMap<_, usize> = node.children.iter().map(|&id| (id, 0)).collect();
        let mut successors: HashMap<usize, Vec<usize>> = HashMap::new();
        for &(from, to) in &node.edges {
            *incoming.get_mut(&to).unwrap() += 1;
            successors.entry(from).or_default().push(to);
        }
        // IDs follow first occurrence, so the smallest ready child preserves
        // original order whenever no overlap constraint requires a change.
        let mut ready: std::collections::BTreeSet<_> = incoming
            .iter()
            .filter_map(|(&id, &count)| (count == 0).then_some(id))
            .collect();
        let mut emitted = 0;
        while let Some(child) = ready.pop_first() {
            emit(child, tree, out)?;
            emitted += 1;
            for &next in successors.get(&child).into_iter().flatten() {
                let count = incoming.get_mut(&next).unwrap();
                *count -= 1;
                if *count == 0 {
                    ready.insert(next);
                }
            }
        }
        if emitted != node.children.len() {
            return None;
        }
        Some(())
    }
    let mut ordered = Vec::new();
    emit(0, &tree, &mut ordered)?;
    let mut result: Vec<PaintGroup> = Vec::new();
    let mut previous = None;
    for (group, node) in ordered {
        if previous == Some(group) {
            result.last_mut().unwrap().nodes.push(node);
        } else {
            let mut next = groups[group].clone();
            next.nodes = vec![node];
            result.push(next);
        }
        previous = Some(group);
    }
    Some(result)
}

#[derive(Clone)]
struct PaintGroup {
    nodes: Vec<u32>,
    polygons: Vec<Polygon>,
    vertices: usize,
    needs_clip: bool,
}

struct SurfaceFragment {
    plane: usize,
    polygon: Polygon,
    needs_clip: bool,
    behind: Option<usize>,
    ahead: Option<usize>,
}

// Orthographic BSP: negative depth is behind a splitting plane. Traversing
// behind / plane / ahead gives source-over order even for translucent paint.
// Bounds are conservative; the retained geometry and its original clips still
// determine coverage. Only genuinely split fragments need an extra GPU clip.
fn surface_paint_groups(
    planes: &[DepthPlane],
    plane_bounds: &[Box2D],
    plane_coverage: &[Polygon],
    nodes: &[u32],
    membership: &[usize],
    bounds: &[Box2D],
    viewport: Box2D,
    coplanar_order: impl Fn(usize) -> (bool, i64),
) -> Vec<PaintGroup> {
    let mut tree: Vec<SurfaceFragment> = Vec::new();
    let mut insertion: Vec<_> = (0..planes.len()).collect();
    // Large background planes first keep small solids from splitting a full
    // viewport background into many pieces.
    insertion.sort_by(|&a, &b| plane_bounds[b].area().total_cmp(&plane_bounds[a].area()));
    for plane in insertion {
        if !boxes_intersect(&plane_bounds[plane], &viewport) {
            continue;
        }
        let mut polygon = plane_coverage[plane].clone();
        for line in [
            [1., 0., -(viewport.min.x as f64)],
            [-1., 0., viewport.max.x as f64],
            [0., 1., -(viewport.min.y as f64)],
            [0., -1., viewport.max.y as f64],
        ] {
            polygon = split_polygon(polygon, line).1.unwrap_or_default();
        }
        if polygon.len() < 3 {
            continue;
        }
        if tree.is_empty() {
            tree.push(SurfaceFragment {
                plane,
                polygon,
                needs_clip: false,
                behind: None,
                ahead: None,
            });
            continue;
        }
        let mut pending = vec![(0, polygon, false)];
        while let Some((parent, polygon, was_split)) = pending.pop() {
            let split = planes[tree[parent].plane];
            let own = planes[plane];
            let line = std::array::from_fn(|i| own.coefficients[i] - split.coefficients[i]);
            let (behind, ahead) = match own.presentation_layer.cmp(&split.presentation_layer) {
                std::cmp::Ordering::Less => (Some(polygon), None),
                std::cmp::Ordering::Greater => (None, Some(polygon)),
                std::cmp::Ordering::Equal => split_polygon(polygon, line),
            };
            let needs_clip = was_split || (behind.is_some() && ahead.is_some());
            for (ahead_side, polygon) in [(false, behind), (true, ahead)] {
                let Some(polygon) = polygon else {
                    continue;
                };
                let child = if ahead_side {
                    tree[parent].ahead
                } else {
                    tree[parent].behind
                };
                if let Some(child) = child {
                    pending.push((child, polygon, needs_clip));
                } else {
                    let child = tree.len();
                    tree.push(SurfaceFragment {
                        plane,
                        polygon,
                        needs_clip,
                        behind: None,
                        ahead: None,
                    });
                    if ahead_side {
                        tree[parent].ahead = Some(child);
                    } else {
                        tree[parent].behind = Some(child);
                    }
                }
            }
        }
    }
    let mut groups = Vec::new();
    let mut pending = if tree.is_empty() {
        Vec::new()
    } else {
        vec![(0, false)]
    };
    while let Some((index, paint)) = pending.pop() {
        let fragment = &tree[index];
        if !paint {
            if let Some(ahead) = fragment.ahead {
                pending.push((ahead, false));
            }
            pending.push((index, true));
            if let Some(behind) = fragment.behind {
                pending.push((behind, false));
            }
            continue;
        }
        let region = polygon_bounds(&fragment.polygon);
        let mut order: Vec<_> = (0..nodes.len())
            .filter(|&i| membership[i] == fragment.plane && boxes_intersect(&bounds[i], &region))
            .collect();
        order.sort_by_key(|&i| (coplanar_order(i), nodes[i]));
        if !order.is_empty() {
            groups.push(PaintGroup {
                nodes: order.into_iter().map(|i| nodes[i]).collect(),
                polygons: vec![fragment.polygon.clone()],
                vertices: fragment.polygon.len(),
                needs_clip: fragment.needs_clip,
            });
        }
    }
    groups
}

pub(super) fn convex_hull(mut points: Polygon) -> Polygon {
    points.retain(|p| p.iter().all(|v| v.is_finite()));
    points.sort_by(|a, b| a[0].total_cmp(&b[0]).then(a[1].total_cmp(&b[1])));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let half = |points: Vec<[f64; 2]>| {
        let mut hull: Polygon = Vec::new();
        for p in points {
            while hull.len() >= 2 {
                let a = hull[hull.len() - 2];
                let b = hull[hull.len() - 1];
                if (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) > 0. {
                    break;
                }
                hull.pop();
            }
            hull.push(p);
        }
        hull.pop();
        hull
    };
    let mut hull = half(points.clone());
    hull.extend(half(points.into_iter().rev().collect()));
    hull
}

fn split_polygon(polygon: Polygon, mut line: [f64; 3]) -> (Option<Polygon>, Option<Polygon>) {
    let scale = line[0].abs().max(line[1].abs());
    if scale == 0.0 {
        return if line[2] < 0.0 {
            (Some(polygon), None)
        } else {
            (None, Some(polygon))
        };
    }
    for c in &mut line {
        *c /= scale;
    }
    let distances: Vec<_> = polygon
        .iter()
        .map(|p| {
            let d = line[0] * p[0] + line[1] * p[1] + line[2];
            if d.abs() <= PLANE_EPSILON {
                0.0
            } else {
                d
            }
        })
        .collect();
    if !distances.iter().any(|d| *d > 0.0) {
        return (Some(polygon), None);
    }
    if !distances.iter().any(|d| *d < 0.0) {
        return (None, Some(polygon));
    }
    let half = |sign: f64| {
        let mut result = Vec::new();
        for i in 0..polygon.len() {
            let j = (i + 1) % polygon.len();
            let d = sign * distances[i];
            let e = sign * distances[j];
            if d >= 0.0 {
                result.push(polygon[i]);
            }
            if (d > 0.0 && e < 0.0) || (d < 0.0 && e > 0.0) {
                let t = d / (d - e);
                result.push([
                    polygon[i][0] + t * (polygon[j][0] - polygon[i][0]),
                    polygon[i][1] + t * (polygon[j][1] - polygon[i][1]),
                ]);
            }
        }
        result
    };
    (Some(half(-1.0)), Some(half(1.0)))
}

fn unique_planes(planes: &[DepthPlane], viewport: &Polygon) -> (Vec<DepthPlane>, Vec<usize>) {
    let mut unique: Vec<DepthPlane> = Vec::new();
    let membership = planes
        .iter()
        .map(|plane| {
            if let Some(index) = unique.iter().position(|other| {
                plane.presentation_layer == other.presentation_layer
                    && viewport.iter().all(|p| {
                        (plane.depth_at(p[0], p[1]) - other.depth_at(p[0], p[1])).abs()
                            <= PLANE_EPSILON
                    })
            }) {
                index
            } else {
                unique.push(*plane);
                unique.len() - 1
            }
        })
        .collect();
    (unique, membership)
}

fn centroid(polygon: &Polygon) -> [f64; 2] {
    let n = polygon.len() as f64;
    polygon
        .iter()
        .fold([0.0; 2], |sum, p| [sum[0] + p[0] / n, sum[1] + p[1] / n])
}

fn polygon_bounds(polygon: &Polygon) -> Box2D {
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in polygon {
        for axis in 0..2 {
            min[axis] = min[axis].min(p[axis]);
            max[axis] = max[axis].max(p[axis]);
        }
    }
    Box2D::new(
        point(min[0] as f32, min[1] as f32),
        point(max[0] as f32, max[1] as f32),
    )
}

pub(super) fn box_polygon(bounds: Box2D) -> Polygon {
    vec![
        [bounds.min.x as f64, bounds.min.y as f64],
        [bounds.max.x as f64, bounds.min.y as f64],
        [bounds.max.x as f64, bounds.max.y as f64],
        [bounds.min.x as f64, bounds.max.y as f64],
    ]
}

// First isolate projected coverage. Depth-equality lines only matter where
// both surfaces paint; extending them across the viewport multiplies stencil
// regions and scene replays for otherwise independent solids.
fn coverage_cells(viewport: Box2D, bounds: &[Box2D]) -> Vec<(Box2D, Vec<usize>)> {
    let mut cells = vec![(viewport, Vec::new())];
    for (index, bounds) in bounds.iter().enumerate() {
        let mut next = Vec::new();
        for (cell, candidates) in cells {
            let min = point(cell.min.x.max(bounds.min.x), cell.min.y.max(bounds.min.y));
            let max = point(cell.max.x.min(bounds.max.x), cell.max.y.min(bounds.max.y));
            if min.x >= max.x || min.y >= max.y {
                next.push((cell, candidates));
                continue;
            }
            // Four disjoint strips plus the intersection exactly cover the cell.
            for outside in [
                Box2D::new(cell.min, point(cell.max.x, min.y)),
                Box2D::new(point(cell.min.x, max.y), cell.max),
                Box2D::new(point(cell.min.x, min.y), point(min.x, max.y)),
                Box2D::new(point(max.x, min.y), point(cell.max.x, max.y)),
            ] {
                if outside.min.x < outside.max.x && outside.min.y < outside.max.y {
                    next.push((outside, candidates.clone()));
                }
            }
            let mut inside = candidates;
            inside.push(index);
            next.push((Box2D::new(min, max), inside));
        }
        cells = next;
    }
    cells
}

#[cfg(test)]
fn partition(planes: &[DepthPlane], viewport: Polygon) -> Vec<Polygon> {
    partition_overlaps(planes, viewport, |_, _| true)
}

fn partition_overlaps(
    planes: &[DepthPlane],
    viewport: Polygon,
    overlaps: impl Fn(usize, usize) -> bool,
) -> Vec<Polygon> {
    let mut regions = vec![viewport];
    for (index, a) in planes.iter().enumerate() {
        for (other, b) in planes[..index].iter().enumerate() {
            // Disjoint projected bounds cannot exchange visible paint order.
            // Keep the full line for overlapping planes: a cell's centroid can
            // lie outside their overlap, but must still give its correct order.
            if a.presentation_layer != b.presentation_layer || !overlaps(index, other) {
                continue;
            }
            let mut line = [
                a.coefficients[0] - b.coefficients[0],
                a.coefficients[1] - b.coefficients[1],
                a.coefficients[2] - b.coefficients[2],
            ];
            if line[0] == 0.0 && line[1] == 0.0 {
                continue;
            }
            // Near edge-on planes have arbitrarily large depth slopes. Compare
            // distances in projected pixels so rounding at a previously cut edge
            // cannot repeatedly split that edge into numerical slivers.
            let scale = line[0].abs().max(line[1].abs());
            for coefficient in &mut line {
                *coefficient /= scale;
            }
            regions = regions
                .into_iter()
                .flat_map(|polygon| {
                    let distances: Vec<_> = polygon
                        .iter()
                        .map(|p| {
                            let distance = line[0] * p[0] + line[1] * p[1] + line[2];
                            if distance.abs() <= PLANE_EPSILON {
                                0.0
                            } else {
                                distance
                            }
                        })
                        .collect();
                    if !distances.iter().any(|d| *d > PLANE_EPSILON)
                        || !distances.iter().any(|d| *d < -PLANE_EPSILON)
                    {
                        return vec![polygon];
                    }
                    [1.0, -1.0]
                        .into_iter()
                        .map(|sign| {
                            let mut clipped = Vec::new();
                            for i in 0..polygon.len() {
                                let j = (i + 1) % polygon.len();
                                let d = distances[i] * sign;
                                let e = distances[j] * sign;
                                if d >= 0.0 {
                                    clipped.push(polygon[i]);
                                }
                                if (d > 0.0 && e < 0.0) || (d < 0.0 && e > 0.0) {
                                    let t = d / (d - e);
                                    clipped.push([
                                        polygon[i][0] + t * (polygon[j][0] - polygon[i][0]),
                                        polygon[i][1] + t * (polygon[j][1] - polygon[i][1]),
                                    ]);
                                }
                            }
                            clipped
                        })
                        .collect()
                })
                .collect();
        }
    }
    regions
}

#[cfg(test)]
mod tests {
    use super::*;
    fn viewport() -> Polygon {
        vec![[0.0, 0.0], [100.0, 0.0], [100.0, 100.0], [0.0, 100.0]]
    }
    fn plane(a: f64, b: f64, c: f64) -> DepthPlane {
        DepthPlane {
            presentation_layer: 0,
            coefficients: [a, b, c],
            backface: false,
        }
    }

    #[test]
    fn independent_presentation_domains_do_not_partition_each_other() {
        let world = plane(1.0, 0.0, -50.0);
        let overlay = DepthPlane {
            presentation_layer: 1,
            ..plane(0.0, 1.0, -50.0)
        };
        assert_eq!(partition(&[world, overlay], viewport()).len(), 1);
        let same_depth_overlay = DepthPlane {
            presentation_layer: 1,
            ..world
        };
        assert_eq!(
            unique_planes(&[world, same_depth_overlay], &viewport())
                .0
                .len(),
            2
        );
    }

    #[test]
    fn only_uninterrupted_translucent_scopes_use_the_surface_painter() {
        let scope = |node_id, opacity| OpacityScope { node_id, opacity };
        let scopes = HashMap::from([
            (1, vec![scope(10, 0.5)]),
            (2, vec![scope(10, 0.5), scope(20, 0.5)]),
            (3, vec![scope(10, 0.5)]),
            (4, vec![scope(30, 1.0)]),
        ]);
        let contiguous =
            |nodes: &[u32]| opacity_scopes_are_contiguous(nodes.iter().copied(), &scopes);
        assert!(contiguous(&[4, 1, 2, 2, 3, 4]));
        assert!(!contiguous(&[1, 4, 2, 3]));
        assert!(!contiguous(&[1, 2, 3, 2]));
        assert!(contiguous(&[1, 2, 3]));
    }

    #[test]
    fn disjoint_fragments_can_commute_but_real_opacity_interleaving_cannot() {
        let group = |id, y| PaintGroup {
            nodes: vec![id],
            polygons: vec![vec![[0., y], [20., y], [20., y + 10.], [0., y + 10.]]],
            vertices: 4,
            needs_clip: true,
        };
        let scope = |node_id| OpacityScope {
            node_id,
            opacity: 0.5,
        };
        let scopes = HashMap::from([
            (1, vec![scope(10)]),
            (2, vec![scope(10), scope(20)]),
            (3, vec![scope(10), scope(20)]),
        ]);
        // An unrelated panel and a repeated clipped node interrupt both nested
        // scopes in BSP order. Disjoint coverage permits one capture per scope.
        let groups = [
            group(1, 0.),
            group(2, 0.),
            group(4, 50.),
            group(3, 0.),
            group(2, 0.),
        ];
        let reordered = regroup_disjoint_scopes(&groups, &scopes).unwrap();
        let order: Vec<_> = reordered
            .iter()
            .flat_map(|g| g.nodes.iter().copied())
            .collect();
        assert_eq!(order, [1, 2, 3, 2, 4]);
        assert!(opacity_scopes_are_contiguous(order.into_iter(), &scopes));
        // The outside surface overlaps both sides of the group. Moving it in
        // either direction would alter source-over order, creating a cycle.
        assert!(
            regroup_disjoint_scopes(&[group(1, 0.), group(4, 0.), group(2, 0.)], &scopes).is_none()
        );
        // Nested scopes need their own constraints, even if the outer is contiguous.
        assert!(
            regroup_disjoint_scopes(&[group(2, 0.), group(1, 0.), group(3, 0.)], &scopes).is_none()
        );
    }

    #[test]
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
    }

    #[test]
    fn crossing_planes_have_consistent_order_inside_each_region() {
        let planes = [
            plane(1.0, 0.0, -50.0),
            plane(0.0, 1.0, -50.0),
            plane(0.0, 0.0, 0.0),
        ];
        let regions = partition(&planes, viewport());
        assert_eq!(regions.len(), 6);
        let mut area = 0.0;
        for polygon in regions {
            let center = centroid(&polygon);
            for a in &planes {
                for b in &planes {
                    let delta = a.depth_at(center[0], center[1]) - b.depth_at(center[0], center[1]);
                    for p in &polygon {
                        let vertex_delta = a.depth_at(p[0], p[1]) - b.depth_at(p[0], p[1]);
                        assert!(vertex_delta * delta >= -1e-7);
                    }
                }
            }
            area += (0..polygon.len())
                .map(|i| {
                    let a = polygon[i];
                    let b = polygon[(i + 1) % polygon.len()];
                    a[0] * b[1] - a[1] * b[0]
                })
                .sum::<f64>()
                / 2.0;
        }
        assert!((area - 10000.0).abs() < 1e-7);
    }

    #[test]
    fn parallel_and_coplanar_planes_do_not_split() {
        let source = [
            plane(0.0, 0.0, 0.0),
            plane(0.0, 0.0, 30.0),
            plane(1e-12, 0.0, 30.0),
        ];
        let (planes, membership) = unique_planes(&source, &viewport());
        assert_eq!(membership, vec![0, 1, 1]);
        assert_eq!(partition(&planes, viewport()).len(), 1);
    }

    #[test]
    fn near_edge_on_planes_do_not_multiply_numerical_slivers() {
        // All equality lines meet at one point. At most two sectors per pair
        // can exist; depth-unit clipping formerly produced hundreds of slivers.
        let planes: Vec<_> = (0..10)
            .map(|i| {
                let angle = i as f64 * 0.77;
                let a = angle.cos() * 1e16;
                let b = angle.sin() * 1e16;
                plane(a, b, -a * 37.3 - b * 51.7)
            })
            .collect();
        let regions = partition(&planes, viewport());
        assert!(regions.len() <= 90, "{} regions", regions.len());
        let mut total_area = 0.0;
        for region in &regions {
            let area = (0..region.len())
                .map(|i| {
                    let a = region[i];
                    let b = region[(i + 1) % region.len()];
                    a[0] * b[1] - a[1] * b[0]
                })
                .sum::<f64>()
                / 2.0;
            assert!(area > 1e-8, "degenerate region: {region:?}");
            total_area += area;
        }
        assert!((total_area - 10000.0).abs() < 1e-7);
    }
}

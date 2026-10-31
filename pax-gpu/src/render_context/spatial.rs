//! Orthographic flat-surface composition. A projected intersection can change paint order
//! within one primitive, so sorting whole nodes (or writing opaque depth) is insufficient.
//! Partition at depth-equality lines, then use ordinary source-over inside each region.
use super::*;
use crate::render_backend::capture::CaptureKey;
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
        let regions = partition(&planes, polygon);
        let mut clips = Vec::with_capacity(regions.len());
        // Upload all clip transforms before encoding any draws: changing a uniform slot
        // partway through the frame would also change earlier queued draws using that slot.
        for (index, polygon) in regions.iter().enumerate() {
            let mut path = Path::builder();
            path.begin(point(polygon[0][0] as f32, polygon[0][1] as f32));
            for p in &polygon[1..] {
                path.line_to(point(p[0] as f32, p[1] as f32));
            }
            path.close();
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
            clips.push((key, signature, id));
        }
        let expired: Vec<_> = self
            .spatial_clip_keys
            .iter()
            .copied()
            .filter(|key| !clips.iter().any(|(new, _, _)| new == key))
            .collect();
        self.clip_arena.release_keys(&expired);
        self.spatial_clip_keys = clips.iter().map(|(key, _, _)| *key).collect();
        self.clip_arena.flush_to_gpu(&mut self.render_backend);
        for (index, polygon) in regions.iter().enumerate() {
            let center = centroid(polygon);
            let mut order: Vec<_> = (0..nodes.len()).collect();
            order.sort_by(|&a, &b| {
                node_planes[a]
                    .presentation_layer
                    .cmp(&node_planes[b].presentation_layer)
                    .then_with(|| {
                        planes[membership[a]]
                            .depth_at(center[0], center[1])
                            .total_cmp(&planes[membership[b]].depth_at(center[0], center[1]))
                    })
                    .then_with(|| {
                        let order = |i: usize| {
                            node_planes[i].coplanar_order(self.scene[&nodes[i]].z_index() as i64)
                        };
                        order(a).cmp(&order(b))
                    })
                    .then_with(|| nodes[a].cmp(&nodes[b]))
            });
            let ordered: Vec<_> = order.into_iter().map(|i| nodes[i]).collect();
            self.spatial_region = Some((index, clips[index].1, clips[index].2));
            let plan = self.opacity_plan_at_depth(&ordered, 0);
            collect_opacity_group_keys(&plan, active);
            self.draw_opacity_plan(&plan);
        }
        self.spatial_region = None;
    }
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

fn partition(planes: &[DepthPlane], viewport: Polygon) -> Vec<Polygon> {
    let mut regions = vec![viewport];
    for (index, a) in planes.iter().enumerate() {
        for b in &planes[..index] {
            if a.presentation_layer != b.presentation_layer {
                continue;
            }
            let line = [
                a.coefficients[0] - b.coefficients[0],
                a.coefficients[1] - b.coefficients[1],
                a.coefficients[2] - b.coefficients[2],
            ];
            if line[0] == 0.0 && line[1] == 0.0 {
                continue;
            }
            regions = regions
                .into_iter()
                .flat_map(|polygon| {
                    let distances: Vec<_> = polygon
                        .iter()
                        .map(|p| line[0] * p[0] + line[1] * p[1] + line[2])
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
}

//! Opaque convex volumes resolve mutual depth on GPU. Planar paint keeps its
//! ordered compositor and receives the complement of each solid's front coverage.
use super::*;
use crate::render_backend::solid;
use pax_runtime_api::math::{Generic, Transform3};
use pax_runtime_api::mesh3::{OpaqueMeshDraw, PosedConvexMesh};

type Matrix = Transform3<Generic>;
type Polygon = Vec<[f64; 2]>;

pub(super) struct RetainedSolid {
    pub pose: Rc<PosedConvexMesh>,
    world: [f64; 16],
    color: Color,
    pub material: Material,
    pub light_mask: u32,
    pub clip_stack: Vec<ClipReference>,
    pub z_index: i32,
    pub bounds: Box2D,
    pub resource: Option<solid::Resource>,
    pub last_uniform: Vec<u8>,
}

impl RetainedSolid {
    pub(super) fn hash_content(&self, hash: &mut DefaultHasher) {
        (Rc::as_ptr(&self.pose.mesh) as usize).hash(hash);
        for value in self.world.iter().chain(self.pose.transform.iter()) {
            value.to_bits().hash(hash);
        }
        bytemuck::bytes_of(&self.color.rgba).hash(hash);
        bytemuck::bytes_of(&to_gpu_material(self.material)).hash(hash);
        self.light_mask.hash(hash);
    }
}

impl WgpuRenderer<'_> {
    /// Retains immutable topology; pose/material changes update only uniforms.
    pub fn draw_convex_mesh(&mut self, draw: &OpaqueMeshDraw, color: Color, material: Material) {
        let Some(pending) = &mut self.current_node else {
            return;
        };
        // Surface transparency remains separate from whole-assembly fading.
        if self
            .clip_stack
            .iter()
            .any(|c| matches!(c, ClipReference::Alpha { .. }))
            || !self.source_recordings.is_empty()
        {
            return;
        }
        let t = self.transform_stack[0].to_array();
        let base = Matrix::new([
            t[0] as f64,
            t[1] as f64,
            0.,
            0.,
            t[2] as f64,
            t[3] as f64,
            0.,
            0.,
            0.,
            0.,
            1.,
            0.,
            t[4] as f64,
            t[5] as f64,
            0.,
            1.,
        ]);
        let transform = (base * Matrix::new(draw.surface)).coeffs();
        // Paint/opacity updates do not change coverage. Preserve the pose's
        // bounded plane-contour cache across retained-node replacement.
        let retained_pose = self.scene.get(&pending.id).and_then(|node| match node {
            RetainedNode::Solid(s) if Rc::ptr_eq(&s.pose.mesh, &draw.mesh)
                && s.pose.transform == transform => Some(s.pose.clone()),
            _ => None,
        });
        let Some(pose) = retained_pose.or_else(|| draw.mesh.posed(transform)) else {
            return;
        };
        let bounds = bounds_of(&pose.silhouette());
        pending.kind = PendingNodeKind::Solid(RetainedSolid {
            pose,
            world: draw.world,
            color,
            material,
            light_mask: pending.light_mask,
            clip_stack: self.clip_stack.clone(),
            z_index: pending.z_index,
            bounds,
            resource: None,
            last_uniform: Vec::new(),
        });
    }

    pub(super) fn prepare_and_draw_solids(&mut self) {
        self.solid_clips.clear();
        let solids: Vec<_> = self
            .sorted_nodes
            .iter()
            .filter_map(|(_, id)| matches!(&self.scene[id], RetainedNode::Solid(_)).then_some(*id))
            .collect();
        let mut min = f64::INFINITY;
        let mut max = f64::NEG_INFINITY;
        for id in &solids {
            let RetainedNode::Solid(s) = &self.scene[id] else {
                unreachable!()
            };
            for p in s.pose.vertices() {
                min = min.min(p[2]);
                max = max.max(p[2]);
            }
        }
        // One frame-wide mapping keeps the hardware depth comparison meaningful
        // between separately retained meshes; padding avoids near/far clipping.
        min -= 1.;
        max += 1.;
        for id in &solids {
            let RetainedNode::Solid(s) = self.scene.get_mut(id).unwrap() else {
                unreachable!()
            };
            let inverse = Matrix::new(s.world).try_inverse().unwrap().coeffs();
            let uniform = solid::Uniform {
                world: std::array::from_fn(|c| std::array::from_fn(|r| s.world[c * 4 + r] as f32)),
                surface: std::array::from_fn(|c| {
                    std::array::from_fn(|r| s.pose.transform[c * 4 + r] as f32)
                }),
                normal: std::array::from_fn(|c| {
                    std::array::from_fn(|r| if r < 3 { inverse[r * 4 + c] as f32 } else { 0. })
                }),
                color: s.color.rgba,
                material: to_gpu_material(s.material),
                depth_range: [min as f32, max as f32, (1. / (max - min)) as f32, 0.],
                lights: [s.light_mask, 0, 0, 0],
            };
            let bytes = bytemuck::bytes_of(&uniform);
            if let Some(resource) = &s.resource {
                if s.last_uniform != bytes {
                    self.render_backend.update_solid(resource, &uniform);
                    self.resource_churn_stats.solid_uniform_updates += 1;
                }
            } else {
                let (resource, bytes) = self.render_backend.create_solid(&s.pose.mesh, &uniform);
                self.resource_churn_stats.solid_geometry_upload_bytes += bytes as u64;
                s.resource = Some(resource);
            }
            s.last_uniform = bytes.to_vec();
        }
        // All states in a fade use the same world-depth mapping and uniforms.
        let solids: Vec<_> = solids
            .into_iter()
            .filter(|id| !self.spatial_hidden.contains(id))
            .collect();
        let mut keys = Vec::new();
        let viewport = self.viewport_bounds();
        let planar = self.visible_main_nodes();
        for id in planar {
            let plane = self.depth_planes.get(&id).copied().unwrap_or_default();
            for solid_id in &solids {
                let RetainedNode::Solid(s) = &self.scene[solid_id] else {
                    unreachable!()
                };
                if !boxes_intersect(&self.scene[&id].bounds(), &s.bounds) {
                    continue;
                }
                let front = s.pose.coverage_in_front(plane);
                if front.len() < 3 {
                    continue;
                }
                let pieces = clipped_coverage(front, &s.clip_stack, &self.clip_arena);
                if pieces.is_empty() {
                    continue;
                }
                let mut path = Path::builder();
                let outer = [
                    [viewport.min.x as f64, viewport.min.y as f64],
                    [viewport.max.x as f64, viewport.min.y as f64],
                    [viewport.max.x as f64, viewport.max.y as f64],
                    [viewport.min.x as f64, viewport.max.y as f64],
                ];
                append_polygon(&mut path, &outer);
                for mut p in pieces {
                    p.reverse();
                    append_polygon(&mut path, &p);
                }
                let path = path.build();

                let key = ClipArenaKey::SolidOcclusion(id, *solid_id);
                let geometry = tessellate_clip_geometry(&path, self.tolerance);
                let signature = hash_clip_geometry(&geometry);
                if let Some((key, clip_id)) =
                    self.clip_arena
                        .sync_key(key, signature, geometry, Transform2D::identity())
                {
                    keys.push(key);
                    self.solid_clips
                        .entry(id)
                        .or_default()
                        .push(ClipReference::Stencil { clip_id });
                }
            }
        }
        let stale: Vec<_> = self
            .solid_clip_keys
            .iter()
            .filter(|k| !keys.contains(k))
            .copied()
            .collect();
        self.clip_arena.release_keys(&stale);
        self.solid_clip_keys = keys;
        self.clip_arena.flush_to_gpu(&mut self.render_backend);
        if !solids.is_empty() {
            self.render_backend.clear_solid_depth();
            self.draw_retained_nodes(&solids);
        }
    }
}

fn bounds_of(p: &[[f64; 2]]) -> Box2D {
    let mut min = [f64::INFINITY; 2];
    let mut max = [f64::NEG_INFINITY; 2];
    for p in p {
        for i in 0..2 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    Box2D::new(
        point(min[0] as f32, min[1] as f32),
        point(max[0] as f32, max[1] as f32),
    )
}
fn append_polygon(builder: &mut lyon::path::path::Builder, p: &[[f64; 2]]) {
    if p.len() < 3 {
        return;
    }
    builder.begin(point(p[0][0] as f32, p[0][1] as f32));
    for p in &p[1..] {
        builder.line_to(point(p[0] as f32, p[1] as f32));
    }
    builder.close();
}
// Stencil tessellation is a disjoint triangle covering, including concave paths
// and holes. Intersecting each piece preserves that covering without convexifying
// the mask, and therefore produces valid non-overlapping inverse-clip holes.
fn clipped_coverage(front: Polygon, clips: &[ClipReference], arena: &ClipArena) -> Vec<Polygon> {
    let mut pieces = vec![front];
    for clip in clips {
        let cutters: Vec<Polygon> = match clip {
            ClipReference::Alpha { .. } => return Vec::new(),
            ClipReference::Scissor(r) => vec![vec![
                [r.min_x as f64, r.min_y as f64],
                [r.max_x as f64, r.min_y as f64],
                [r.max_x as f64, r.max_y as f64],
                [r.min_x as f64, r.max_y as f64],
            ]],
            ClipReference::Stencil { clip_id } => {
                let entry = &arena.entries[clip_id];
                let t = arena.slots[*clip_id as usize].transform;
                entry
                    .geometry
                    .indices
                    .chunks_exact(3)
                    .map(|tri| {
                        let mut p: Polygon = tri
                            .iter()
                            .map(|&i| {
                                let p = entry.geometry.vertices[i as usize].position;
                                [
                                    (t[0][0] * p[0] + t[1][0] * p[1] + t[2][0]) as f64,
                                    (t[0][1] * p[0] + t[1][1] * p[1] + t[2][1]) as f64,
                                ]
                            })
                            .collect();
                        if cross(p[0], p[1], p[2]) < 0. {
                            p.reverse();
                        }
                        p
                    })
                    .collect()
            }
        };
        pieces = pieces
            .into_iter()
            .flat_map(|p| {
                cutters.iter().filter_map(move |c| {
                    let p = intersect(&p, c);
                    (p.len() >= 3).then_some(p)
                })
            })
            .collect();
    }
    pieces
}
fn cross(a: [f64; 2], b: [f64; 2], p: [f64; 2]) -> f64 {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0])
}
fn intersect(p: &Polygon, cutter: &Polygon) -> Polygon {
    let mut p = p.clone();
    for i in 0..cutter.len() {
        let (a, b) = (cutter[i], cutter[(i + 1) % cutter.len()]);
        let mut out = Vec::new();
        for j in 0..p.len() {
            let (u, v) = (p[j], p[(j + 1) % p.len()]);
            let du = cross(a, b, u);
            let dv = cross(a, b, v);
            if du >= 0. {
                out.push(u);
            }
            if (du >= 0.) != (dv >= 0.) {
                let t = du / (du - dv);
                out.push([u[0] + t * (v[0] - u[0]), u[1] + t * (v[1] - u[1])]);
            }
        }
        p = out;
    }
    p
}
